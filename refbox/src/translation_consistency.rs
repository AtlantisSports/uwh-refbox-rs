//! Consistency checks over the Fluent translation files.
//!
//! `build.rs` already reports keys missing from a locale, but only as a
//! `cargo:warning=` in debug builds, which cannot fail CI, and it compares each
//! file against the union of all files rather than against the reference
//! language. These tests are the enforcing version, and they also cover what
//! the build script cannot see at all: whether a translation's `{ $variables }`
//! still match the English original.
//!
//! `en-US` is the reference, matching `fallback_language` in `i18n.toml`.

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};
use fluent_syntax::parser::parse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The language every other language is checked against.
const REFERENCE: &str = "en-US";

fn translations_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("translations")
}

/// Every locale directory name, sorted, e.g. `["de-DE", "en-US", ...]`.
fn locales() -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(translations_dir())
        .expect("translations directory should exist")
        .map(|e| e.expect("readable dir entry"))
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

/// What one `.ftl` file declares.
struct Catalog {
    /// message id -> the set of `{ $variable }` names it uses, at any depth
    messages: BTreeMap<String, BTreeSet<String>>,
    /// ids referenced from inside other entries: messages plain, terms with a
    /// leading `-`, e.g. `penalty-kind` and `-dark-team-name`
    references: BTreeSet<String>,
}

fn load_catalog(locale: &str) -> Catalog {
    let path = translations_dir().join(locale).join("refbox.ftl");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));

    // Unlike build.rs, a syntax error is a hard failure rather than a silently
    // empty catalog -- an unparseable file must not look like a passing one.
    let resource = match parse(content.as_str()) {
        Ok(r) => r,
        Err((_, errors)) => panic!(
            "{} has {} Fluent syntax error(s); first: {:?}",
            path.display(),
            errors.len(),
            errors.first()
        ),
    };

    let mut messages = BTreeMap::new();
    let mut references = BTreeSet::new();

    for entry in &resource.body {
        match entry {
            Entry::Message(message) => {
                let mut vars = BTreeSet::new();
                if let Some(pattern) = &message.value {
                    walk_pattern(pattern, &mut vars, &mut references);
                }
                for attribute in &message.attributes {
                    walk_pattern(&attribute.value, &mut vars, &mut references);
                }
                messages.insert(message.id.name.to_string(), vars);
            }
            Entry::Term(term) => {
                // A term's own variables are not compared across locales (terms
                // are referenced, never called from code), but the references
                // inside one still count as usage.
                let mut vars = BTreeSet::new();
                walk_pattern(&term.value, &mut vars, &mut references);
            }
            _ => {}
        }
    }

    Catalog {
        messages,
        references,
    }
}

fn walk_pattern(pattern: &Pattern<&str>, vars: &mut BTreeSet<String>, refs: &mut BTreeSet<String>) {
    for element in &pattern.elements {
        if let PatternElement::Placeable { expression } = element {
            walk_expression(expression, vars, refs);
        }
    }
}

/// Descends into select expressions -- both the selector and every variant.
/// `penalty` nests selects inside selects, so a shallow walk misses most of
/// its variables.
fn walk_expression(
    expression: &Expression<&str>,
    vars: &mut BTreeSet<String>,
    refs: &mut BTreeSet<String>,
) {
    match expression {
        Expression::Inline(inline) => walk_inline(inline, vars, refs),
        Expression::Select { selector, variants } => {
            walk_inline(selector, vars, refs);
            for variant in variants {
                walk_pattern(&variant.value, vars, refs);
            }
        }
    }
}

fn walk_inline(
    inline: &InlineExpression<&str>,
    vars: &mut BTreeSet<String>,
    refs: &mut BTreeSet<String>,
) {
    match inline {
        InlineExpression::VariableReference { id } => {
            vars.insert(id.name.to_string());
        }
        InlineExpression::MessageReference { id, .. } => {
            refs.insert(id.name.to_string());
        }
        InlineExpression::TermReference { id, arguments, .. } => {
            refs.insert(format!("-{}", id.name));
            if let Some(arguments) = arguments {
                for positional in &arguments.positional {
                    walk_inline(positional, vars, refs);
                }
                for named in &arguments.named {
                    walk_inline(&named.value, vars, refs);
                }
            }
        }
        InlineExpression::FunctionReference { arguments, .. } => {
            for positional in &arguments.positional {
                walk_inline(positional, vars, refs);
            }
            for named in &arguments.named {
                walk_inline(&named.value, vars, refs);
            }
        }
        InlineExpression::Placeable { expression } => walk_expression(expression, vars, refs),
        InlineExpression::StringLiteral { .. } | InlineExpression::NumberLiteral { .. } => {}
    }
}

/// Every `.rs` file under `src/`, concatenated.
fn all_source() -> String {
    fn visit(dir: &Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).expect("readable source dir") {
            let path = entry.expect("readable dir entry").path();
            if path.is_dir() {
                visit(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push_str(&std::fs::read_to_string(&path).expect("readable source file"));
            }
        }
    }
    let mut out = String::new();
    visit(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut out);
    out
}

#[test]
fn every_reference_key_exists_in_every_locale() {
    let reference = load_catalog(REFERENCE);
    let mut problems: Vec<String> = Vec::new();

    for locale in locales() {
        if locale == REFERENCE {
            continue;
        }
        let catalog = load_catalog(&locale);
        let missing: Vec<&str> = reference
            .messages
            .keys()
            .filter(|key| !catalog.messages.contains_key(*key))
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            problems.push(format!("  {locale}: missing {}", missing.join(", ")));
        }
    }

    assert!(
        problems.is_empty(),
        "these locales are missing keys that {REFERENCE} defines:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_key_uses_the_same_variables_as_the_reference() {
    let reference = load_catalog(REFERENCE);
    let mut problems: Vec<String> = Vec::new();

    for locale in locales() {
        if locale == REFERENCE {
            continue;
        }
        let catalog = load_catalog(&locale);
        for (key, expected) in &reference.messages {
            let Some(actual) = catalog.messages.get(key) else {
                continue; // absence is the other test's job to report
            };
            if actual != expected {
                problems.push(format!(
                    "  {locale}  {key}\n      {REFERENCE} uses: {expected:?}\n      {locale} uses: {actual:?}"
                ));
            }
        }
    }

    assert!(
        problems.is_empty(),
        "these translations do not use the same variables as {REFERENCE}.\n\
         A missing variable renders a sentence with a hole in it; a misspelled \
         one renders the placeholder literally.\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_reference_key_is_used() {
    let reference = load_catalog(REFERENCE);
    let source = all_source();

    // Every call site passes a string literal -- the only non-literal
    // occurrences of `fl!(` are the macro's own definition in main.rs. If a
    // dynamically-keyed call is ever added, this test becomes unsound and the
    // key it builds must be allowed for explicitly.
    let unused: Vec<&str> = reference
        .messages
        .keys()
        .filter(|key| !source.contains(&format!("\"{key}\"")))
        .filter(|key| !reference.references.contains(*key))
        .map(String::as_str)
        .collect();

    assert!(
        unused.is_empty(),
        "these {REFERENCE} keys are not used by any fl!() call and are not \
         referenced from another message, so all 15 locales are carrying dead \
         text:\n  {}",
        unused.join("\n  ")
    );
}

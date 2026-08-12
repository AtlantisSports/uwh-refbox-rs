# Translation Consistency Test Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A test that fails CI when a translation loses a key, changes a key's `{ $variables }`, or when a key exists that nothing uses.

**Architecture:** One test-only module in `refbox/src/`, parsing the `.ftl` files with `fluent-syntax` and `refbox/src/**/*.rs` with a literal `fl!("…")` scan. `en-US` is the reference. Nothing ships in the binary; nothing about the app changes.

**Tech Stack:** Rust 2024, `fluent-syntax` 0.12 (already a build-dependency, added here as a dev-dependency), Fluent `.ftl` files.

**Spec:** `docs/superpowers/specs/2026-08-12-translation-consistency-test-design.md`

**Branch:** `chore/refbox/translation-consistency-test`, off `origin/master` at `15341229`. The spec is committed at `5504f756`.

## Global Constraints

- **`build.rs` is not touched.** Its `cargo:rerun-if-changed` lines share a directory walk with its flawed comparison and may be load-bearing for release builds noticing edited translations.
- **No new crate.** `fluent-syntax = "0.12.0"` goes under `[dev-dependencies]` at the version already in `[build-dependencies]`.
- **No translation text changes.** The only translation edit anywhere in this branch is deleting the dead key `using-portal`.
- **`en-US` is the reference**, matching `fallback_language = "en-US"` in `refbox/i18n.toml`.
- **MSRV 1.85, Rust 2024, `-D warnings`, `cargo fmt`.** Enforced by `just check` and the pre-commit hook.
- **Terms (`-name`) are never required to have an `fl!()` call.** They exist to be referenced from `.ftl`.

## Survey of the current state — do not re-derive

Measured on 2026-08-12 against `15341229`:

- **335 keys, 15 locales.** No missing keys, no extra keys, **no variable mismatches**. The test goes green on arrival except for assertion 3.
- **Exactly one unused key: `using-portal`** (`USING { $portal }PORTAL:`), present in all 15 files, referenced only in historical docs. Its last code user went with `7923b316`.
- **No dynamic `fl!()` call sites.** The only non-literal occurrences are the macro definition at `refbox/src/main.rs:105` and `:109`, plus a doc comment.
- **Select expressions exist and nest** — `penalty`, `brightness`, `foul`, `penalty-kind`. A regex cannot collect their variables; the AST walk must recurse.
- **2 terms** (`-dark-team-name`, `-light-team-name`), referenced as `{ -dark-team-name }` from `gi-team-dark` and similar.

---

### Task 1: The module, the dependency, and the coverage assertion

**Files:**
- Modify: `refbox/Cargo.toml` (`[dev-dependencies]`)
- Modify: `refbox/src/main.rs` (one `mod` line)
- Create: `refbox/src/translation_consistency.rs`

**Interfaces:**
- Produces: `Catalog`, `load_catalog(locale: &str) -> Catalog`, and `locales() -> Vec<String>`, used by Tasks 2 and 3.

- [ ] **Step 1: Add the dev-dependency**

In `refbox/Cargo.toml`, under the existing `[dev-dependencies]` block (which currently holds `tempfile` and `tokio`), add:

```toml
fluent-syntax = "0.12.0"
```

- [ ] **Step 2: Declare the module**

In `refbox/src/main.rs`, beside the other `mod` declarations, add:

```rust
#[cfg(test)]
mod translation_consistency;
```

- [ ] **Step 3: Write the module with the coverage assertion**

Create `refbox/src/translation_consistency.rs`:

```rust
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

fn walk_pattern(
    pattern: &Pattern<&str>,
    vars: &mut BTreeSet<String>,
    refs: &mut BTreeSet<String>,
) {
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
```

- [ ] **Step 4: Run it and watch it pass**

Run: `cargo test -p refbox translation_consistency`
Expected: `every_reference_key_exists_in_every_locale ... ok`. The survey says no key is missing today, so a failure here means the AST walk is wrong, not the translations.

- [ ] **Step 5: Prove the assertion actually bites**

Temporarily delete the `back = ` line from `refbox/translations/de-DE/refbox.ftl`, re-run the test, and confirm it FAILS naming `de-DE` and `back`. Then restore the line with `git checkout -- refbox/translations/de-DE/refbox.ftl` and confirm it passes again.

A guard nobody has watched fail is not a guard. Do not skip this.

- [ ] **Step 6: Commit**

```bash
git add refbox/Cargo.toml refbox/src/main.rs refbox/src/translation_consistency.rs
git commit -m "test(refbox): assert every locale carries every en-US key"
```

---

### Task 2: The variable-consistency assertion

This is the assertion the branch exists for — the one nothing in the repo covers.

**Files:**
- Modify: `refbox/src/translation_consistency.rs`

**Interfaces:**
- Consumes: `Catalog`, `load_catalog`, `locales`, `REFERENCE` from Task 1.

- [ ] **Step 1: Add the test**

Append to `refbox/src/translation_consistency.rs`:

```rust
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
                    "  {locale}  {key}\n      {REFERENCE} uses: {:?}\n      {locale} uses: {:?}",
                    expected, actual
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
```

- [ ] **Step 2: Run it and watch it pass**

Run: `cargo test -p refbox translation_consistency`
Expected: both tests pass. The survey found no mismatches today.

- [ ] **Step 3: Prove it bites**

In `refbox/translations/fr/refbox.ftl`, change `sound = SON : { $sound_text }` so the variable reads `{ $sound_txt }`. Re-run and confirm it FAILS naming `fr`, `sound`, and both variable sets. Restore with `git checkout -- refbox/translations/fr/refbox.ftl`.

Then do it once more inside a nested select: in the same file change one `{$kind}` inside `penalty-kind` to `{$knid}`, confirm the failure, and restore. **This is the case a regex-based check would miss**, so it is the one worth seeing fail.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/translation_consistency.rs
git commit -m "test(refbox): assert translations use the reference's variables"
```

---

### Task 3: The usage assertion, and deleting the dead key

**Files:**
- Modify: `refbox/src/translation_consistency.rs`
- Modify: all 15 `refbox/translations/*/refbox.ftl` (delete one line each)

- [ ] **Step 1: Add the test**

Append to `refbox/src/translation_consistency.rs`:

```rust
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
fn every_reference_key_is_used() {
    let reference = load_catalog(REFERENCE);
    let source = all_source();

    // Every call site passes a string literal -- the only non-literal
    // occurrences of fl!( are the macro's own definition in main.rs. If a
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
```

- [ ] **Step 2: Run it and watch it FAIL**

Run: `cargo test -p refbox translation_consistency`
Expected: `every_reference_key_is_used` FAILS, naming exactly `using-portal`. This is the assertion working, not a defect — the key has been dead since `7923b316`.

If it names anything besides `using-portal`, stop and report: either the usage rule is too narrow or a second key died unnoticed.

- [ ] **Step 3: Delete the dead key from all 15 files**

Remove the `using-portal = …` line from each of the 15 `refbox/translations/*/refbox.ftl`. It is one line per file with no continuation lines. Verify with:

```bash
grep -c "using-portal" refbox/translations/*/refbox.ftl
```

Expected: every file reports `0`.

- [ ] **Step 4: Run the whole suite**

Run: `just check`
Expected: exit 0, all three translation tests green, and the existing suite unaffected.

- [ ] **Step 5: Prove the usage assertion bites on a NEW key**

Add `zzz-unused-probe = probe` to `refbox/translations/en-US/refbox.ftl`, re-run, and confirm it FAILS naming `zzz-unused-probe`. Remove it and confirm green again.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/translation_consistency.rs refbox/translations/
git commit -m "test(refbox): assert every key is used, and drop the dead using-portal"
```

---

### Task 4: Correct the merged plan document

**Files:**
- Modify: `docs/superpowers/plans/2026-08-12-source-neutral-health-wording.md`

The plan document of the branch merged as #2219 states the repository has no translation-coverage check. That is wrong, and it reached master — the correcting commit missed the merge queue's snapshot.

- [ ] **Step 1: Replace the false bullet**

In the section `## No test is written for this change — deliberately`, replace this line:

```
- The repository has **no translation-coverage test at all** — nothing asserts that a key exists in every locale. This was checked, not assumed.
```

with:

```
- **Key coverage was already checked, in `refbox/build.rs`** — it parses every `.ftl` with `fluent-syntax` and reports keys missing from a file. (An earlier revision of this plan claimed no check existed; that was wrong, and came from grepping `src/` for test functions rather than reading the build script.) What it does *not* check is whether a translation still uses the same `{ $variables }` as the English original — the gap that produced `docs/superpowers/specs/2026-08-12-translation-consistency-test-design.md`.
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/plans/2026-08-12-source-neutral-health-wording.md
git commit -m "docs(refbox): correct the merged plan's translation-coverage claim"
```

---

## Final verification, before the PR

- [ ] `just check` exit 0.
- [ ] `git diff --stat origin/master` shows only: `refbox/Cargo.toml`, `refbox/src/main.rs`, `refbox/src/translation_consistency.rs`, the 15 `.ftl` files, and the two docs. Nothing else.
- [ ] `cargo build -p refbox` still succeeds — deleting `using-portal` must not break a call site (nothing references it, but prove it).
- [ ] All three assertions have been **seen to fail** and then restored, per the steps above. State this plainly in the PR.

## Deviations

_Record anything that diverged from this plan here, rather than in standalone commits._

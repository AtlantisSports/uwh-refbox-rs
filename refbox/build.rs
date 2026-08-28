use fluent_syntax::parser::parse;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;

fn extract_message_ids(content: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Ok(ast) = parse(content) {
        for entry in ast.body {
            if let fluent_syntax::ast::Entry::Message(message) = entry {
                ids.insert(message.id.name.to_string());
            }
        }
    }
    ids
}

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12");
    }

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("resources/AppIcon.ico", "refbox_icon");
        res.compile().unwrap();
    }

    // Path to your localization files
    let l10n_dir = "translations";
    // The directory itself, not just the files in it: without this, adding a
    // whole new language does not re-run the build script, and the check that
    // every translations directory is accounted for never gets to run.
    println!("cargo:rerun-if-changed={l10n_dir}");
    let mut file_message_map: HashMap<String, HashSet<String>> = HashMap::new();

    // Load all .ftl files from subdirectories and extract message IDs
    for entry in fs::read_dir(l10n_dir).expect("Could not read directory") {
        let entry = entry.expect("Could not read directory entry");
        let path = entry.path();
        if path.is_dir() {
            for file_entry in fs::read_dir(&path).expect("Could not read subdirectory") {
                let file_entry = file_entry.expect("Could not read file entry");
                let file_path = file_entry.path();
                if file_path.extension().and_then(|ext| ext.to_str()) == Some("ftl") {
                    println!("cargo:rerun-if-changed={}", file_path.display());
                    let content = fs::read_to_string(&file_path).expect("Could not read file");
                    let message_ids = extract_message_ids(&content);
                    file_message_map.insert(file_path.display().to_string(), message_ids);
                }
            }
        }
    }

    // Compare sets of message IDs
    let all_keys: HashSet<_> = file_message_map
        .values()
        .flat_map(|set| set.iter().cloned())
        .collect();
    let mut missing_keys = HashMap::new();

    for (file, ids) in &file_message_map {
        let missing_in_file: Vec<_> = all_keys.difference(ids).cloned().collect();
        if !missing_in_file.is_empty() {
            missing_keys.insert(file, missing_in_file);
        }
    }

    let msg_type = if cfg!(debug_assertions) {
        "warning"
    } else {
        "error"
    };

    if missing_keys != HashMap::new() {
        for (file, missing) in &missing_keys {
            // Two colons, not one. `cargo:error=` is not a recognised
            // directive: cargo silently ignores it, so the release branch above
            // reported nothing at all and this check was warning-only in every
            // build. `cargo::error=` does fail the build, and both spellings
            // work for `warning`. Verified on rustc 1.85, the MSRV.
            println!(
                "cargo::{}=Missing keys in {}: {}",
                msg_type,
                file,
                missing.join(", ")
            );
        }
    }

    // Unlike a missing translation key, these are errors in every profile.
    // `just check` -- the gate this project tells you to run before a PR --
    // builds in debug, so a warning here would let a font gap through the one
    // check anyone runs locally and surface it only in CI's release job. The
    // burden falls exactly on whoever changed the text: re-cutting a subset is
    // one `just` command, named in the message.
    for finding in font_coverage_findings() {
        println!("cargo::error={finding}");
    }
}

// ---------------------------------------------------------------------------
// Bundled-font coverage
//
// refbox ships three font subsets and picks one per language. A character the
// chosen font does not carry is drawn blank on the scoreboard PC, whose
// software renderer does not fall back to another font the way a desktop
// silently does -- which is why a gap here is invisible during development and
// visible only at the pool. The subsets are cut by `scripts/regen-cjk-font.py`
// and `scripts/regen-thai-font.py` from the translation files, so any UI text
// added outside those files, or added without re-cutting, can open a gap.
// ---------------------------------------------------------------------------

const ROBOTO: &str = "resources/Roboto-Medium.ttf";

/// Where the fallback language is configured.
const I18N_CONFIG: &str = "i18n.toml";

/// The language whose text is drawn when a message is missing. Its characters
/// can therefore appear in ANY language's font, not just Roboto's.
///
/// Read from `i18n.toml` rather than assumed: hardcoding it would leave every
/// font checked against the wrong file, silently, the day it changed.
fn fallback_locale() -> String {
    println!("cargo:rerun-if-changed={I18N_CONFIG}");
    let config = fs::read_to_string(I18N_CONFIG)
        .unwrap_or_else(|e| panic!("{I18N_CONFIG}: could not read: {e}"));
    let Some((_, value)) = config
        .lines()
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim() == "fallback_language")
    else {
        panic!(
            "{I18N_CONFIG}: no `fallback_language` found. It decides which language's text every \
             font has to be able to draw, so this check cannot be run without it."
        )
    };
    // Saying "not found" about a key that is right there sends the reader
    // looking for the wrong problem.
    value.split('"').nth(1).map(str::to_string).unwrap_or_else(|| {
        panic!(
            "{I18N_CONFIG}: `fallback_language` is set to `{}`, which this check cannot read -- \
             it expects a double-quoted value.",
            value.trim()
        )
    })
}
const CJK_FONT: &str = "resources/WqyZenHei-Subset.ttf";
const THAI_FONT: &str = "resources/NotoSansThai-Subset.ttf";

/// Every shipped language: its `translations/` directory, its `Language`
/// variant, and the bundled font that renders it.
///
/// This mirrors `default_font_for` in `src/main.rs` (which chooses the font)
/// and `Language::as_lang_id` in `src/app/languages.rs` (which names the
/// locale). Adding a language means adding a row here: a `translations/`
/// directory with no row fails this check rather than being skipped, so the
/// font question cannot be forgotten.
const FONTS_BY_LANGUAGE: &[(&str, &str, &str)] = &[
    ("de-DE", "German", ROBOTO),
    ("en-US", "English", ROBOTO),
    ("es", "Spanish", ROBOTO),
    ("fr", "French", ROBOTO),
    ("id-ID", "Indonesian", ROBOTO),
    ("it-IT", "Italian", ROBOTO),
    ("ja-JP", "Japanese", CJK_FONT),
    ("ko-KR", "Korean", CJK_FONT),
    ("ms-MY", "Malay", ROBOTO),
    ("nl-NL", "Dutch", ROBOTO),
    ("pt-PT", "Portuguese", ROBOTO),
    ("th-TH", "Thai", THAI_FONT),
    ("tl-PH", "Tagalog", ROBOTO),
    ("tr-TR", "Turkish", ROBOTO),
    ("zh-CN", "Mandarin", CJK_FONT),
];

/// What to do about a character a font cannot draw. Only the two subsets are
/// cut by a script; Roboto ships whole, so telling someone to re-cut it would
/// send them to a command that does not touch it.
fn remedy_for(font: &str) -> &'static str {
    match font {
        CJK_FONT => "Re-cut the subset with `just regen-cjk-font`.",
        THAI_FONT => "Re-cut the subset with `just regen-thai-font`.",
        _ => {
            "Roboto ships whole and is not cut by any script, so either the text has to change \
             or the bundled font does."
        }
    }
}

/// Read a big-endian `u16` at `at`, or panic naming the font -- a truncated
/// read means the file is not the font we think it is, which must stop the
/// build rather than quietly reduce what gets checked.
fn be_u16(data: &[u8], at: usize, font: &str) -> u16 {
    match data.get(at..at + 2) {
        Some(b) => u16::from_be_bytes([b[0], b[1]]),
        None => panic!("{font}: truncated while reading two bytes at {at}"),
    }
}

/// Every Unicode code point the font actually maps to a glyph.
///
/// Only the Windows BMP `cmap` format 4 subtable is read, because that is what
/// all three bundled fonts use and every character the UI draws is inside the
/// BMP. A font arriving in any other shape panics instead of returning a
/// partial answer: a coverage check that silently reads nothing would pass
/// forever while proving nothing.
fn font_coverage(path: &str) -> HashSet<u32> {
    let data = fs::read(path).unwrap_or_else(|e| panic!("{path}: could not read font: {e}"));

    let table_count = be_u16(&data, 4, path) as usize;
    let cmap = (0..table_count)
        .map(|i| 12 + 16 * i)
        .find(|&rec| data.get(rec..rec + 4) == Some(b"cmap"))
        .map(|rec| {
            let b = &data[rec + 8..rec + 12];
            u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize
        })
        .unwrap_or_else(|| panic!("{path}: no cmap table"));

    // Prefer the Windows BMP subtable (platform 3, encoding 1); it is the one
    // every renderer consults first.
    let subtable_count = be_u16(&data, cmap + 2, path) as usize;
    let subtable = (0..subtable_count)
        .filter_map(|i| {
            let rec = cmap + 4 + 8 * i;
            let platform = be_u16(&data, rec, path);
            let encoding = be_u16(&data, rec + 2, path);
            let b = &data[rec + 4..rec + 8];
            let offset = cmap + u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;
            (platform == 3 && encoding == 1 && be_u16(&data, offset, path) == 4).then_some(offset)
        })
        .next()
        .unwrap_or_else(|| {
            panic!("{path}: no Windows BMP format-4 cmap subtable -- this check cannot read it")
        });

    // Format 4 lays out four parallel arrays of `segment_count` entries each,
    // after a seven-word header, with a padding word between the first two.
    let segment_count = be_u16(&data, subtable + 6, path) as usize / 2;
    let ends = subtable + 14;
    let starts = ends + segment_count * 2 + 2;
    let deltas = starts + segment_count * 2;
    let range_offsets = deltas + segment_count * 2;

    let mut covered = HashSet::new();
    for seg in 0..segment_count {
        let start = be_u16(&data, starts + seg * 2, path);
        let end = be_u16(&data, ends + seg * 2, path);
        // The final segment is the required 0xFFFF terminator, not real coverage.
        if start == 0xFFFF {
            continue;
        }
        let delta = be_u16(&data, deltas + seg * 2, path);
        let range_offset = be_u16(&data, range_offsets + seg * 2, path) as usize;

        for code in start..=end {
            // A code point inside a segment is still uncovered if it resolves
            // to glyph 0 (.notdef), so the glyph id has to be resolved rather
            // than the segment range trusted.
            let glyph = if range_offset == 0 {
                code.wrapping_add(delta)
            } else {
                let at = range_offsets + seg * 2 + range_offset + (code - start) as usize * 2;
                match be_u16(&data, at, path) {
                    0 => 0,
                    id => id.wrapping_add(delta),
                }
            };
            if glyph != 0 {
                covered.insert(code as u32);
            }
        }
    }
    covered
}

/// The per-language button labels written as Rust literals in
/// `src/app/languages.rs` -- CANCEL, BACK, APPLY and the restart prompt.
///
/// These are on-screen text that the regeneration scripts never see: they read
/// only the `.ftl` files. Returns `(Language variant, literal)` pairs.
fn language_literals(source: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    // Stop at the test module so its `assert_eq!` expectations are not mistaken
    // for UI text.
    let body = source.split("mod tests").next().unwrap_or(source);
    // `Self::A | Self::B => "..."` gives one literal to two languages, so a
    // variant with no `=>` of its own is held until the arm it belongs to.
    let mut pending: Vec<String> = Vec::new();
    for piece in body.split("Self::").skip(1) {
        let leading: String = piece
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let Some((variant, rest)) = piece.split_once(" => ") else {
            if !leading.is_empty() && piece.contains('|') {
                pending.push(leading);
            }
            continue;
        };
        // Only a bare string literal is UI text; `Self::X => LanguageIdentifier`
        // and friends are skipped by requiring the quote immediately.
        let text = rest.strip_prefix('"').and_then(|r| r.split('"').next());
        match text {
            Some(text)
                if !variant.is_empty() && variant.chars().all(|c| c.is_ascii_alphabetic()) =>
            {
                for alternate in pending.drain(..) {
                    found.push((alternate, text.to_string()));
                }
                found.push((variant.to_string(), text.to_string()));
            }
            _ => pending.clear(),
        }
    }
    found
}

/// The lines of a `.ftl` file that are actually shown. A line starting `#` is
/// a note to translators -- never drawn, and often written in the target
/// script, so counting it would demand glyphs for text the app cannot display.
///
/// Only a `#` in column zero starts a Fluent comment: an indented `#` is part
/// of a message's value, and skipping those lines would drop real UI text.
fn drawn_text(ftl: &str) -> String {
    ftl.lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The drawn text of every `.ftl` file in one language's directory.
fn ftl_text_in(dir: &str) -> Result<String, std::io::Error> {
    let mut text = String::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("ftl") {
            text.push_str(&drawn_text(&fs::read_to_string(&path)?));
            text.push('\n');
        }
    }
    Ok(text)
}

/// Characters in `text` that `covered` does not carry, ASCII excluded (the
/// Latin range is present in every bundled font and is checked separately).
fn uncovered(text: &str, covered: &HashSet<u32>) -> BTreeSet<char> {
    text.chars()
        .filter(|c| !c.is_ascii() && !covered.contains(&(*c as u32)))
        .collect()
}

fn describe(chars: &BTreeSet<char>) -> String {
    chars
        .iter()
        .map(|c| format!("'{c}' (U+{:04X})", *c as u32))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every character the UI can draw that the font for its language does not
/// carry. An empty result means every shipped language is fully renderable.
fn font_coverage_findings() -> Vec<String> {
    // Before anything else: the table the rest of this depends on has to still
    // match the app.
    let mut findings = font_selector_findings();

    // Keyed on the bundled fonts themselves rather than on whichever ones the
    // table happens to mention, so dropping a language cannot turn a later
    // lookup into a bare index panic with nothing to explain it.
    let mut coverage: HashMap<&str, HashSet<u32>> = HashMap::new();
    for font in [ROBOTO, CJK_FONT, THAI_FONT] {
        println!("cargo:rerun-if-changed={font}");
        coverage.entry(font).or_insert_with(|| font_coverage(font));
    }

    // A translations directory with no row would otherwise be checked against
    // no font at all -- the silent pass this check exists to prevent.
    let listed: HashSet<&str> = FONTS_BY_LANGUAGE.iter().map(|(dir, _, _)| *dir).collect();
    for entry in fs::read_dir("translations").expect("Could not read translations directory") {
        let path = entry.expect("Could not read translations entry").path();
        if path.is_dir() {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if !listed.contains(name.as_str()) {
                findings.push(format!(
                    "translations/{name} has no row in FONTS_BY_LANGUAGE, so no font was checked \
                     for it -- add one naming the font that language renders in"
                ));
            }
        }
    }

    for (dir, _, font) in FONTS_BY_LANGUAGE {
        // Every .ftl in the directory, not just refbox.ftl: the key-consistency
        // check above walks them all, and a second domain file would be
        // on-screen text no font check had ever seen.
        let path = format!("translations/{dir}");
        let text = match ftl_text_in(&path) {
            Ok(text) => text,
            Err(e) => {
                findings.push(format!(
                    "{path}: listed in FONTS_BY_LANGUAGE but unreadable: {e}"
                ));
                continue;
            }
        };
        let missing = uncovered(&text, &coverage[font]);
        if !missing.is_empty() {
            findings.push(format!(
                "{path} needs {} character(s) that {font} does not carry: {}. {}",
                missing.len(),
                describe(&missing),
                remedy_for(font)
            ));
        }
    }

    // The button labels written as Rust literals rather than translation keys.
    let source_path = "src/app/languages.rs";
    println!("cargo:rerun-if-changed={source_path}");
    let source = fs::read_to_string(source_path).expect("Could not read src/app/languages.rs");
    let literals = language_literals(&source);
    // Finding *some* literals is not enough, and neither is finding one per
    // language: every one of these functions has an arm for every language, so
    // a single arm rewritten in a shape the scan misses still leaves that
    // language present via the others.
    //
    // Comparing the languages against each other is not enough either -- drift
    // that hits every arm equally (a whole function written `Language::x =>`
    // rather than `Self::x =>`) leaves them all agreeing and all wrong. So the
    // expectation comes from the file itself: one label per `*_text` function
    // per language. Adding a label function keeps that true without anyone
    // updating a count here; failing to scan one does not.
    // Counted over the same region `language_literals` reads -- counting the
    // whole file would let a test-only helper raise the expectation and abort
    // the build with a false alarm.
    let label_fns = source
        .split("mod tests")
        .next()
        .unwrap_or(&source)
        .matches("_text(self)")
        .count();
    let mut label_counts: BTreeMap<&str, usize> = FONTS_BY_LANGUAGE
        .iter()
        .map(|(_, variant, _)| (*variant, 0))
        .collect();
    for (variant, _) in &literals {
        if let Some(count) = label_counts.get_mut(variant.as_str()) {
            *count += 1;
        }
    }
    let short: BTreeMap<&str, usize> = label_counts
        .iter()
        .filter(|(_, count)| **count != label_fns)
        .map(|(variant, count)| (*variant, *count))
        .collect();
    assert!(
        label_fns > 0 && short.is_empty(),
        "{source_path}: the file defines {label_fns} function(s) ending `_text(self)`, so every \
         language should have that many labels, but the scan found {short:?}. Either an arm is \
         written in a shape `language_literals` cannot read, or a `_text(self)` method was added \
         that is not a per-language label. Whichever it is, some labels would go unchecked -- \
         teach the scan about it rather than deleting the check."
    );
    let font_for: HashMap<&str, &str> = FONTS_BY_LANGUAGE
        .iter()
        .map(|(_, variant, font)| (*variant, *font))
        .collect();
    for (variant, text) in &literals {
        let Some(font) = font_for.get(variant.as_str()) else {
            findings.push(format!(
                "{source_path}: Language::{variant} has no row in FONTS_BY_LANGUAGE, so its \
                 labels were checked against no font"
            ));
            continue;
        };
        let missing = uncovered(text, &coverage[font]);
        if !missing.is_empty() {
            findings.push(format!(
                "{source_path}: the Language::{variant} label \"{text}\" needs {}, which {font} \
                 does not carry",
                describe(&missing)
            ));
        }
    }

    // A message missing from a translation is drawn in the fallback language's
    // words but the current language's font, so every bundled font has to be
    // able to draw the fallback text -- not just Roboto.
    let fallback_locale = fallback_locale();
    let fallback_path = format!("translations/{fallback_locale}");
    let fallback_text = ftl_text_in(&fallback_path)
        .unwrap_or_else(|e| panic!("{fallback_path}: could not read the fallback language: {e}"));
    let fonts: Vec<&str> = coverage.keys().copied().collect();
    for font in fonts {
        let missing = uncovered(&fallback_text, &coverage[font]);
        if !missing.is_empty() {
            findings.push(format!(
                "{font} cannot draw {} from {fallback_path}. Any message missing from a \
                 translation is shown in {fallback_locale} while keeping the current language's \
                 font, so every bundled font must carry it. {}",
                describe(&missing),
                remedy_for(font)
            ));
        }
    }

    // The language picker names every language in its own script and says
    // which font to draw each entry in. None of that text is in a translation
    // file, so cutting a subset from the .ftl files alone silently drops it.
    println!("cargo:rerun-if-changed={PICKER_SOURCE}");
    let picker_source = fs::read_to_string(PICKER_SOURCE)
        .unwrap_or_else(|e| panic!("{PICKER_SOURCE}: could not read: {e}"));
    let entries = picker_entries(&picker_source);
    // As above, and the two function definitions also match the scan, so
    // "found something" would pass with every call site deleted. The picker
    // offers each language exactly once, so any other count means the scan has
    // stopped reading part of it.
    let mut entry_counts: BTreeMap<&str, usize> = FONTS_BY_LANGUAGE
        .iter()
        .map(|(_, variant, _)| (*variant, 0))
        .collect();
    for (variant, _, _) in &entries {
        if let Some(count) = variant
            .as_deref()
            .and_then(|variant| entry_counts.get_mut(variant))
        {
            *count += 1;
        }
    }
    let miscounted: BTreeMap<&str, usize> = entry_counts
        .iter()
        .filter(|(_, count)| **count != 1)
        .map(|(variant, count)| (*variant, *count))
        .collect();
    assert!(
        miscounted.is_empty(),
        "{PICKER_SOURCE}: the scan expected one picker entry per language but found \
         {miscounted:?}. It no longer matches this file, so those labels would go unchecked -- \
         fix `picker_entries`, do not delete it."
    );
    for (_, labels, font) in &entries {
        // An entry naming no font is drawn in the app default, which follows
        // the language the operator is currently in -- so it has to render in
        // all three.
        let fonts: Vec<&str> = match font {
            Some(font) => vec![font],
            None => vec![CJK_FONT, THAI_FONT, ROBOTO],
        };
        for font in fonts {
            for label in labels {
                let missing = uncovered(label, &coverage[font]);
                if !missing.is_empty() {
                    findings.push(format!(
                        "{PICKER_SOURCE}: the language-picker label \"{label}\" needs {}, which \
                         {font} does not carry",
                        describe(&missing)
                    ));
                }
            }
        }
    }

    // Latin-1: refbox picks one font for the whole UI, so in an Asian-language
    // session these fonts also draw every Latin name arriving from the portal.
    // A team called "Cafe" with an acute accent would otherwise render with a
    // hole, and no translation file would ever have warned us. U+00A0 is
    // excluded: it is a space, so a missing glyph is invisible.
    for font in [CJK_FONT, THAI_FONT] {
        let missing: BTreeSet<char> = (0xA1..=0xFF)
            .filter(|c| !coverage[font].contains(c))
            .filter_map(char::from_u32)
            .collect();
        if !missing.is_empty() {
            findings.push(format!(
                "{font} is missing {} Latin-1 character(s), which it has to draw for portal team \
                 and player names while an Asian language is selected: {}. {}",
                missing.len(),
                describe(&missing),
                remedy_for(font)
            ));
        }
    }

    // Digits, Latin letters and punctuation are generated at runtime (clock,
    // scores, cap numbers) and often appear in no translation file, so both
    // subsets force-include printable ASCII. Losing it renders those blank.
    for font in [CJK_FONT, THAI_FONT] {
        let missing: BTreeSet<char> = (0x20..=0x7E)
            .filter(|c| !coverage[font].contains(c))
            .filter_map(char::from_u32)
            .collect();
        if !missing.is_empty() {
            findings.push(format!(
                "{font} is missing printable ASCII, which is drawn at runtime and force-included \
                 by the subset scripts: {}",
                describe(&missing)
            ));
        }
    }

    findings
}

/// Where the app actually chooses a font, which `FONTS_BY_LANGUAGE` copies.
const FONT_SELECTOR_SOURCE: &str = "src/main.rs";

/// The font families `default_font_for` can return, and the file each is
/// bundled from.
const FAMILY_FILES: &[(&str, &str)] = &[
    ("WenQuanYi Zen Hei", CJK_FONT),
    ("Noto Sans Thai", THAI_FONT),
    ("Roboto", ROBOTO),
];

/// Report any language where `FONTS_BY_LANGUAGE` disagrees with
/// `default_font_for`.
///
/// The table is a copy of a decision made in `src/main.rs`, and a copy is only
/// worth keeping if it cannot drift: move a language between those match arms
/// and every check in this file would go on validating it against the font it
/// used to use, stay green, and let the app draw blanks.
fn font_selector_findings() -> Vec<String> {
    println!("cargo:rerun-if-changed={FONT_SELECTOR_SOURCE}");
    let source = fs::read_to_string(FONT_SELECTOR_SOURCE)
        .unwrap_or_else(|e| panic!("{FONT_SELECTOR_SOURCE}: could not read: {e}"));
    let body = source
        .split("fn default_font_for")
        .nth(1)
        .unwrap_or_else(|| {
            panic!(
                "{FONT_SELECTOR_SOURCE}: `default_font_for` not found. It is where the app picks \
                 a font, so without it FONTS_BY_LANGUAGE cannot be held to anything -- point \
                 this at wherever the choice moved to."
            )
        });
    // Column-zero brace: the arms' own blocks are indented.
    let body = body.split("\n}").next().unwrap_or(body);
    // Strip line comments before reading arms. A comment naming `Language::X`
    // sits inside whichever arm's pattern precedes it, and -- first arm winning
    // -- would beat the real one, failing the build over a claim that is not
    // true and whose suggested fix would put the wrong font in the table.
    let body: String = body
        .lines()
        .map(|line| line.split("//").next().unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    let body = body.as_str();

    let mut findings = Vec::new();
    let mut by_variant: BTreeMap<&str, &str> = BTreeMap::new();
    let mut catch_all = None;
    // Arms are found by line, not by splitting on "=>": an arm's body can hold
    // anything, and a `debug_assert!(lang == Language::Thai)` inside one would
    // otherwise be read as the next arm's pattern and blame the wrong font.
    let lines: Vec<&str> = body.lines().collect();
    let arm_starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            let line = line.trim_start();
            line.starts_with("Language::") || line.starts_with('_')
        })
        .map(|(i, _)| i)
        .collect();
    for (nth, &start) in arm_starts.iter().enumerate() {
        let end = arm_starts.get(nth + 1).copied().unwrap_or(lines.len());
        let arm = lines[start..end].join("\n");
        let pattern = lines[start];
        // The family is the arm's first string literal.
        let Some(family) = arm.split('"').nth(1) else {
            continue;
        };
        let Some((_, file)) = FAMILY_FILES.iter().find(|(name, _)| *name == family) else {
            findings.push(format!(
                "{FONT_SELECTOR_SOURCE}: `default_font_for` returns the font family {family:?}, \
                 which is not one of the bundled fonts -- add it to FAMILY_FILES with the file \
                 it ships from"
            ));
            continue;
        };
        let variants: Vec<&str> = pattern
            .split("Language::")
            .skip(1)
            .map(|rest| {
                rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or_default()
            })
            .collect();
        if variants.is_empty() {
            catch_all = Some(*file);
        } else {
            for variant in variants {
                // First arm wins, as `match` does. Inserting unconditionally
                // would let a later arm mask an earlier one and report the
                // font the app does not actually use.
                by_variant.entry(variant).or_insert(file);
            }
        }
    }
    assert!(
        !by_variant.is_empty(),
        "{FONT_SELECTOR_SOURCE}: no `Language::` arms were read out of `default_font_for`, so \
         FONTS_BY_LANGUAGE would be checked against nothing -- fix `font_selector_findings`, do \
         not delete it."
    );

    for (dir, variant, font) in FONTS_BY_LANGUAGE {
        let selected = by_variant.get(variant).copied().or(catch_all);
        match selected {
            Some(selected) if selected == *font => {}
            Some(selected) => findings.push(format!(
                "{FONT_SELECTOR_SOURCE}: `default_font_for` draws {variant} ({dir}) in \
                 {selected}, but FONTS_BY_LANGUAGE in build.rs says {font}. The check would \
                 validate that language against the wrong font."
            )),
            None => findings.push(format!(
                "{FONT_SELECTOR_SOURCE}: `default_font_for` names no font for {variant} ({dir}) \
                 and has no catch-all arm, so what the app would draw it in is unknown."
            )),
        }
    }
    findings
}

/// The file holding the language picker, whose entries name each language in
/// its own script.
const PICKER_SOURCE: &str = "src/app/view_builders/shared_elements.rs";

/// The byte index of the `)` closing the `(` at `open`, ignoring parentheses
/// inside string literals -- the picker's own notes are written "(未検証)", so
/// counting them would end the call in the wrong place.
fn closing_paren(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, c) in source[open..].char_indices() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// The string literals inside one call.
/// Decode the escapes that can hide a character from a plain scan. Only
/// `\u{...}` matters: read literally it becomes the ASCII `u{65E5}`, so the
/// real character would be neither cut into a subset nor reported missing.
fn decode_unicode_escape(chars: &mut std::str::Chars<'_>) -> Option<char> {
    if chars.clone().next() != Some('{') {
        return None;
    }
    chars.next();
    let mut hex = String::new();
    for c in chars.by_ref() {
        match c {
            '}' => break,
            _ => hex.push(c),
        }
    }
    u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
}

fn literals_in(call: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = call;
    while let Some(start) = rest.find('"') {
        let inner = &rest[start + 1..];
        let mut chars = inner.chars();
        let mut text = String::new();
        let mut consumed = 0;
        let mut closed = false;
        while let Some(c) = chars.next() {
            consumed += c.len_utf8();
            match c {
                '"' => {
                    closed = true;
                    break;
                }
                '\\' => {
                    let before = chars.as_str().len();
                    match chars.next() {
                        Some('u') => {
                            if let Some(decoded) = decode_unicode_escape(&mut chars) {
                                text.push(decoded);
                            }
                        }
                        Some(other) => text.push(other),
                        None => {}
                    }
                    consumed += before - chars.as_str().len();
                }
                _ => text.push(c),
            }
        }
        if !closed {
            break;
        }
        found.push(text);
        rest = &inner[consumed..];
    }
    found
}

/// Every language-picker entry: the `Language` variant it is for, its labels,
/// and the font it names -- `None` when the entry takes the app default, which
/// follows whichever language the operator is currently in and so could be any
/// of the three.
fn picker_entries(source: &str) -> Vec<(Option<String>, Vec<String>, Option<&'static str>)> {
    let body = source.split("#[cfg(test)]").next().unwrap_or(source);
    let mut entries = Vec::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("lang_btn") {
        let start = at + found;
        let after = &body[start + "lang_btn".len()..];
        let open = if after.starts_with('(') {
            start + "lang_btn".len()
        } else if after.starts_with("_note(") {
            start + "lang_btn_note".len()
        } else {
            at = start + "lang_btn".len();
            continue;
        };
        let Some(close) = closing_paren(body, open) else {
            break;
        };
        let call = &body[open..close];
        let font = if call.contains("CJK_FONT") {
            Some(CJK_FONT)
        } else if call.contains("THAI_FONT") {
            Some(THAI_FONT)
        } else if call.contains("LATIN_FONT") {
            Some(ROBOTO)
        } else {
            None
        };
        let variant = call.split("Language::").nth(1).map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect::<String>()
        });
        entries.push((variant, literals_in(call), font));
        at = close;
    }
    entries
}

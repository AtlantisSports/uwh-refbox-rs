# Turkish Language & "(UNVERIFIED)" Label — Design Spec

**Date:** 2026-04-17
**Scope:** `refbox` crate only
**Branch:** `feat/workspace/desktop-build` (v0.4.0 integration branch — accumulates work)
**Split:** Two commits on the same branch

---

## Overview

Two related changes:

1. **Add Turkish (`tr-TR`) as a selectable language**, following the same pattern as the
   previously added Latin-script languages (Dutch, Italian, German, Portuguese, Indonesian,
   Malay, Tagalog). No special font bundling — Turkish uses Latin letters plus diacritics
   (ç, ğ, ı, İ, ö, ş, ü) which the existing Roboto font covers.

2. **Add an "(UNVERIFIED)" note** in small text below the language name on every language
   button except English, Spanish, and French. This is honest signalling to operators and
   end users that a native speaker has not yet reviewed those translations. The note is
   translated into each language (e.g. the Turkish button shows `TÜRKÇE` + `(DOĞRULANMAMIŞ)`).
   Spanish and French are considered verified and do not get the note.

Both changes are scoped to `refbox`. The `uwh-common` crate and the wireless-remote firmware
are not touched.

---

## Commit 1 — Add Turkish Language

### Files

| File | Change |
|------|--------|
| `refbox/src/app/languages.rs` | Add `Turkish` variant; wire up `tr-TR`, `tr` matching, short strings, cycle position |
| `refbox/translations/tr-TR/refbox.ftl` | **New** — 356 keys translated (matching other Latin locales) |
| `refbox/src/app/view_builders/configuration.rs` | Add `TÜRKÇE` button; update alphabetical comment |

### Enum and identifier wiring (`languages.rs`)

- `enum Language` gains `Turkish` (placed after `Thai`, before the closing `}`)
- `as_lang_id`: `Self::Turkish => LanguageIdentifier::from_bytes(b"tr-TR").unwrap()`
- `from_lang_id`: add an `else if` branch matching `"tr"` to `Self::Turkish`
- `cancel_text`: `Self::Turkish => "İPTAL"`
- `done_text`: `Self::Turkish => "TAMAM"` *(placeholder — may be tuned during glossary review)*
- `restart_text`: `Self::Turkish => "UYGULAMAK İÇİN YENİDEN BAŞLAT"` *(placeholder — may be tuned during glossary review)*
- `Cyclable::next`: reorder last two arms so `Thai → Turkish → English`

### Translation file (`tr-TR/refbox.ftl`)

Start from `en-US/refbox.ftl` as the key template. Every key present in `en-US` must be
present in `tr-TR`. Length target: ~356 lines (same as other Latin locales).

Translation sourcing process:

1. Research Turkish UWH resources — prioritise TSSF (Türkiye Sualtı Sporları Federasyonu)
   "Sualtı Hokeyi" rulebook and any Turkish UWH club publications.
2. Produce a glossary of ~25–30 UWH-specific terms (penalty kinds, match phases, foul
   categories, score confirmation, team colours, etc.) with a `source` tag per term:
   `[FED-RULEBOOK]`, `[WEB-UWH-CLUB]`, `[BEST-GUESS]`, etc.
3. Human reviews the glossary. Only after sign-off does the full `.ftl` get generated.
4. Native speaker review of the full file happens after landing, out-of-scope here. The
   "(UNVERIFIED)" label (Commit 2) communicates that status to users until then.

### Language-select grid (`configuration.rs`)

Grid layout after the change (row × col, 4×4):

```
Row 1: BAHASA INDONESIA | BAHASA MELAYU | DEUTSCH        | ENGLISH
Row 2: ESPAÑOL          | FILIPINO      | FRANÇAIS       | 한국어
Row 3: ITALIANO         | NEDERLANDS    | 日本語           | PORTUGUÊS
Row 4: ภาษาไทย            | TÜRKÇE        | 中文            | (empty)
```

Placement rationale: Turkish alphabetises between Thai and Mandarin (`Thai < Türkçe < Zhōngwén`).

The alphabetical-order comment at the top of `make_language_select_page` must be updated to
include `Türkçe(T)` between `Thai/ภาษาไทย(T)` and `Zhōngwén/中文(Z)`.

---

## Commit 2 — "(UNVERIFIED)" Label on Language Buttons

### What the user sees

Every language button except English, Spanish, and French gains a small extra line below the
native language name reading `(UNVERIFIED)` in that language's script.

For most buttons that adds one line (native name on line 1, note on line 2):
- German button: `DEUTSCH` / `(NICHT VERIFIZIERT)`
- Mandarin button: `中文` / `(未验证)`
- Turkish button: `TÜRKÇE` / `(DOĞRULANMAMIŞ)`

The two Bahasa buttons are already two lines today, so they become three lines total:
- `BAHASA` / `INDONESIA` / `(BELUM DIVERIFIKASI)`
- `BAHASA` / `MELAYU` / `(BELUM DISAHKAN)`

Three languages remain unchanged (no note): English, Spanish, French.

### Files

| File | Change |
|------|--------|
| `refbox/src/app/view_builders/shared_elements.rs` | Add helper for the "native name + small note" button shape |
| `refbox/src/app/view_builders/configuration.rs` | Use the new helper for the 12 unverified language buttons, with the note text hardcoded in each button's target language |

**No FTL files are touched in Commit 2.**

### Why the notes are hardcoded, not sourced via `fl!()`

The `fl!()` macro (defined in `refbox/src/main.rs:84`) uses a **single global language
loader** bound to whatever language the operator currently has the app set to. It cannot
render a key in an arbitrary target language. For the language-select grid, where every
button must display text in its own target language regardless of the operator's current
locale, sourcing via `fl!()` would render every note in the operator's current language —
defeating the purpose.

This is why the existing native language names on the buttons (`DEUTSCH`, `中文`,
`ภาษาไทย`, etc.) are already hardcoded inline in `configuration.rs` rather than sourced
through `fl!()`. The unverified note follows the same pattern.

### Candidate "(UNVERIFIED)" translations (first-pass)

Hardcoded inline in `configuration.rs` next to each button's native name. All subject to
native-speaker review later.

| Target language | Candidate note |
|-----------------|----------------|
| German          | `(NICHT VERIFIZIERT)` |
| Italian         | `(NON VERIFICATO)` |
| Dutch           | `(NIET GEVERIFIEERD)` |
| Portuguese      | `(NÃO VERIFICADO)` |
| Indonesian      | `(BELUM DIVERIFIKASI)` |
| Malay           | `(BELUM DISAHKAN)` |
| Tagalog         | `(HINDI PA NA-VERIFY)` |
| Korean          | `(검증되지 않음)` |
| Japanese        | `(未検証)` |
| Mandarin        | `(未验证)` |
| Thai            | `(ยังไม่ได้ตรวจสอบ)` |
| Turkish         | `(DOĞRULANMAMIŞ)` |

English, Spanish, and French are considered verified and do not receive a note.

### Button helpers

The grid ends up with three button shapes:

1. **Single-line, no note** — English, Spanish, French (unchanged from today)
2. **Single-line native name + small unverified note** — 10 languages (German, Italian,
   Dutch, Portuguese, Tagalog, Korean, Japanese, Mandarin, Thai, Turkish)
3. **Two-line native name + small unverified note** — the two Bahasa buttons (Indonesian, Malay)

No 3-line-button precedent exists; the existing `make_multi_label_button` only handles
2 labels at the same size.

**Plan:** add one new helper in `shared_elements.rs` that builds the "native name (1 or 2
lines) + small note" column. Shape (illustrative — the final signature is an implementation
detail):

```rust
make_lang_button_with_note<'a, ...>(
    main: NameLines,             // enum: OneLine(T) | TwoLine(T, T)
    note: T,                     // the "(UNVERIFIED)" text
) -> Button<'a, Message>
```

The note is rendered via `.size(SOME_SMALL_TEXT_CONSTANT)` — the specific constant is
chosen from the existing `refbox/src/app/theme/` text-size constants and confirmed
visually. The existing `make_multi_label_button` is left untouched.

### Button-construction pattern in `configuration.rs`

- Verified languages (English, Spanish, French): continue using the existing `lang_btn`
  closure, no change.
- Single-line unverified languages: use the new helper with the native name and the
  hardcoded unverified note for that language.
- Bahasa Indonesia / Bahasa Melayu: use the new helper with a two-line native name
  (`"BAHASA"` / `"INDONESIA"` or `"MELAYU"`) and the hardcoded unverified note for that
  language.

The note text for each button is a literal string in that button's target language, sitting
next to the button's native name label in the `make_language_select_page` function.

---

## Testing and Verification

After each commit, run `just check` (fmt, clippy, tests, audit). No new unit tests required —
the existing codebase has no "all languages round-trip" test, and this change is
structurally equivalent to the prior language-addition commits.

Manual verification:
- Launch the app, open the language-select screen, confirm the grid layout and
  "(UNVERIFIED)" rendering in each language's script.
- Select Turkish, confirm the UI text loads from `tr-TR/refbox.ftl`.
- Confirm Cancel/Done/Restart text on the language-select screen uses the Turkish short
  strings from `languages.rs`.
- Confirm no regression in the other 14 languages.

---

## Explicit Non-Scope

- The wireless-remote firmware is **not** touched.
- No font files are added or regenerated. The Thai-specific font-regen scripts and
  `.ttf` bundling are untouched.
- No changes to `uwh-common` or any other crate.
- No refactoring of surrounding code in `configuration.rs` or `shared_elements.rs` beyond
  what is strictly necessary to add the new helper and wire up the note.
- The native-speaker review of each translation is a future, out-of-band activity. The
  "(UNVERIFIED)" label exists to make that pending review visible in the meantime.

---

## Plain-Language Summary (for commit messages and PR body)

**Commit 1:** Adds Turkish as a selectable language. A new translation file covers all the
refbox text in Turkish, sourced with a mix of the Turkish UWH federation rulebook and
best-effort translation for generic UI terms. A "TÜRKÇE" button appears on the
language-select screen between the Thai and Mandarin buttons.

**Commit 2:** Adds a small "(UNVERIFIED)" note under every language button except English,
Spanish, and French. This tells operators that those translations have not yet been
reviewed by a native speaker, so they should be treated with appropriate caution until
that review happens.

# Beep-Test Edit Levels: ADD LEVEL + 10-Level Cap + Drop Wrapping — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 6 at `4df81a1`)
**Chunk:** 7 of the beep-test redesign follow-on series
**Process gate:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change).

---

## Goal

Three operator-driven updates to the BeepTest Edit Levels page, plus a
translation cleanup:

1. **Rename** the button currently labeled `+ NEW` to **ADD LEVEL**, making it the verb-noun counterpart to the existing **REMOVE LEVEL** button. **Translate the new ADD LEVEL value into all 15 locales (no English fallbacks). Also translate the existing REMOVE LEVEL value in the 13 locales that currently use English fallback**, so the management row's verb-noun pair is operator-visible-symmetric in every supported language.
2. **Cap the number of levels at 10.** The ADD LEVEL button is disabled when the staged level count reaches 10. A defense-in-depth guard in the handler refuses to append a new level when the count is already 10.
3. **Drop the row-wrapping ("band") logic** in both the Edit Levels page and the main BeepTest view. With a 10-level cap and existing `BAND_WIDTH = 10` / `EDIT_BAND_WIDTH = 10`, the wrapping branch is dead code; remove it for clarity.

---

## Motivation

Operator surfaced all three asks 2026-05-21 after the Chunk 6 walkthrough:

- The `+ NEW` label doesn't read as the obvious counterpart to `REMOVE LEVEL`. **ADD LEVEL** does, and the parallel naming aids operator-visible symmetry on the edit panel's management row.
- The current code allows operators to keep appending levels without limit. The operator wants a hard cap of 10, both to bound the table layout (10 columns is the maximum that fits cleanly in one row at the current screen width) and to deprecate the multi-row "band" wrapping plan, which was an earlier design idea that was never operator-validated.
- The `chunks(BAND_WIDTH)` band-wrapping logic in `beep_test.rs` and `beep_test_settings.rs` was added in anticipation of >10-level layouts. With the cap landing at exactly 10, those branches will never execute. Leaving them in place is dead code that future maintainers would need to reason about.

---

## Scope

### Files touched

- `refbox/src/app/view_builders/beep_test_settings.rs` — strip the band loop in `build_editor_levels_table` (lines 332–395), remove the `EDIT_BAND_WIDTH = 10` constant (line 261), and add the `add_disabled = staged.len() >= 10` gate around both ADD LEVEL call sites in `build_edit_panel` (the fallback at line 479 and the management-row at line 492). Keep `EDIT_TABLE_CELL_HEIGHT`, `EDIT_TABLE_CELL_SPACING`, and the rest of the file unchanged.
- `refbox/src/app/view_builders/beep_test.rs` — strip the band loop in the main view's table builder (lines 187–268) and remove the `BAND_WIDTH = 10` constant (line 37). Update the doc comment at lines 177–178 to reflect the new "single row of columns" rendering.
- `refbox/src/app/mod.rs` — in the `Message::BeepTestEditAddLevel` handler (line 3580), guard the `levels.push(...)` call with `if levels.len() < 10`. This mirrors Chunk 1's defense-in-depth pattern for the per-level count.
- `refbox/translations/en-US/refbox.ftl` — change line 377's `beep-test-edit-new` value from `+ NEW` to `ADD LEVEL`. The existing `beep-test-edit-remove = REMOVE LEVEL` is unchanged (already English-correct).
- `refbox/translations/fr/refbox.ftl` — change line 370 `beep-test-edit-new` to `AJOUTER NIVEAU`. `beep-test-edit-remove = SUPPRIMER NIVEAU` is unchanged (already correctly translated).
- `refbox/translations/es/refbox.ftl` — change line 379 `beep-test-edit-new` to `AÑADIR NIVEL`. `beep-test-edit-remove = ELIMINAR NIVEL` is unchanged (already correctly translated).
- `refbox/translations/de-DE/refbox.ftl`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`, `tr-TR`, `zh-CN`/`refbox.ftl` — **change BOTH `beep-test-edit-new` AND `beep-test-edit-remove`** to the locale-appropriate translations listed in the Design section's translation table. Each of these 12 locales currently has English fallback for both keys; this chunk replaces both fallbacks with proper translations so the management row's verb-noun pair is operator-visible-symmetric per-language.

### Not touched

- The translation key name `beep-test-edit-new` — only its values change. Renaming the key is out of scope (Approach B/C from brainstorming, intentionally not chosen).
- Chunk 1's per-level lap-count cap (`level.count >= 5`) at `beep_test_settings.rs:582` and `mod.rs:3521-3527`. That cap is separate and stays as-is.
- The `Message` enum.
- `uwh-common`, `wireless-remote`, the LED panel, the overlay.
- Other locales' values for `beep-test-edit-remove` (Chunk 7 only touches the `-edit-new` key value).
- Behavior for legacy configs that already contain >10 levels — see "Out of scope" below.

---

## Design

### Item 1 — rename the button value + full per-locale translation

The view code in `beep_test_settings.rs` references the button via `fl!("beep-test-edit-new")`. The reference stays the same; only the **values** in the `.ftl` files change. Additionally, the existing `beep-test-edit-remove` value is also updated in the 12 currently-English-fallback locales so the management row's verb-noun pair stays operator-visible-symmetric per-language.

**Full translation table (both keys, all 15 locales):**

| Locale | `beep-test-edit-new` (old → new) | `beep-test-edit-remove` (old → new) |
|---|---|---|
| en-US | `+ NEW` → **`ADD LEVEL`** | `REMOVE LEVEL` → unchanged |
| fr | `+ NOUVEAU` → **`AJOUTER NIVEAU`** | `SUPPRIMER NIVEAU` → unchanged |
| es | `+ NUEVO` → **`AÑADIR NIVEL`** | `ELIMINAR NIVEL` → unchanged |
| de-DE | `+ NEW` → **`STUFE HINZUFÜGEN`** | `REMOVE LEVEL` → **`STUFE ENTFERNEN`** |
| id-ID | `+ NEW` → **`TAMBAH LEVEL`** | `REMOVE LEVEL` → **`HAPUS LEVEL`** |
| it-IT | `+ NEW` → **`AGGIUNGI LIVELLO`** | `REMOVE LEVEL` → **`RIMUOVI LIVELLO`** |
| ja-JP | `+ NEW` → **`レベル追加`** | `REMOVE LEVEL` → **`レベル削除`** |
| ko-KR | `+ NEW` → **`레벨 추가`** | `REMOVE LEVEL` → **`레벨 제거`** |
| ms-MY | `+ NEW` → **`TAMBAH TAHAP`** | `REMOVE LEVEL` → **`BUANG TAHAP`** |
| nl-NL | `+ NEW` → **`NIVEAU TOEVOEGEN`** | `REMOVE LEVEL` → **`NIVEAU VERWIJDEREN`** |
| pt-PT | `+ NEW` → **`ADICIONAR NÍVEL`** | `REMOVE LEVEL` → **`REMOVER NÍVEL`** |
| th-TH | `+ NEW` → **`เพิ่มระดับ`** | `REMOVE LEVEL` → **`ลบระดับ`** |
| tl-PH | `+ NEW` → **`MAGDAGDAG NG ANTAS`** | `REMOVE LEVEL` → **`ALISIN ANG ANTAS`** |
| tr-TR | `+ NEW` → **`SEVİYE EKLE`** | `REMOVE LEVEL` → **`SEVİYE KALDIR`** |
| zh-CN | `+ NEW` → **`添加级别`** | `REMOVE LEVEL` → **`删除级别`** |

**Translation rationale.** Each row follows the verb-noun (or noun-verb in Asian languages) pattern already established by the French (`AJOUTER NIVEAU` / `SUPPRIMER NIVEAU`) and Spanish (`AÑADIR NIVEL` / `ELIMINAR NIVEL`) entries. Verb choices:

- **de-DE** `HINZUFÜGEN` / `ENTFERNEN` — standard German "add" / "remove" pair.
- **id-ID** `TAMBAH` / `HAPUS` — common Indonesian UI verbs; "level" kept as loanword (consistent with much Indonesian software UI).
- **it-IT** `AGGIUNGI` / `RIMUOVI` — standard Italian imperatives, `LIVELLO` for level.
- **ja-JP** `追加` (tsuika, "add") / `削除` (sakujo, "delete") — compound-form labels are the natural Japanese UI convention; `レベル` is the standard loanword for "level".
- **ko-KR** `추가` (chuga, "add") / `제거` (jeogeo, "remove") — standard Korean UI verbs; `레벨` is the standard loanword.
- **ms-MY** `TAMBAH` / `BUANG` — Malay UI conventions; `TAHAP` for level.
- **nl-NL** `TOEVOEGEN` / `VERWIJDEREN` — standard Dutch.
- **pt-PT** `ADICIONAR` / `REMOVER` — Portuguese; `NÍVEL` for level.
- **th-TH** `เพิ่ม` (phoem, "add") / `ลบ` (lop, "delete") — common Thai UI verbs; `ระดับ` for level.
- **tl-PH** `MAGDAGDAG NG` / `ALISIN ANG` — natural Tagalog imperatives, `ANTAS` for level.
- **tr-TR** `EKLE` / `KALDIR` — standard Turkish imperatives; `SEVİYE` for level.
- **zh-CN** `添加` (tiānjiā, "add") / `删除` (shānchú, "delete") — standard Simplified Chinese UI verbs; `级别` for level.

If the operator (or a native speaker) prefers a different verb or word-order, the spec can be amended before implementation. None of these terms are present in the existing `docs/superpowers/specs/2026-04-17-{lang}-glossary.md` files (which cover UWH domain terms — fouls, penalties, period names — not generic UI verbs), so all 12 translations are best-effort based on standard UI conventions in each language.

### Item 2 — cap at 10

**View side (`beep_test_settings.rs::build_edit_panel`).** Before constructing the management row, compute:

```rust
let add_disabled = levels.len() >= 10;
```

Apply this gate to both ADD LEVEL call sites (the fallback at line 479 when no level is selected, and the management-row at line 492). When `add_disabled`, the button gets the `gray_button` style and does not receive an `on_press` (mirroring the existing `count_inc_disabled` pattern at line 582 and the `remove_disabled` pattern at line 496).

**Handler side (`mod.rs::Message::BeepTestEditAddLevel`).** Wrap the `levels.push(...)` and selection update in `if levels.len() < 10`:

```rust
Message::BeepTestEditAddLevel => {
    if let Some(ref mut edited) = self.edited_settings {
        if let Some(ref mut levels) = edited.beep_test_levels {
            if levels.len() < 10 {
                levels.push(crate::config::Level {
                    count: 4,
                    duration: std::time::Duration::from_secs(20),
                });
                edited.selected_level = levels.len() - 1;
            }
        }
    }
    Task::none()
}
```

The cap value `10` is inline-literal in both spots, matching Chunk 1's pattern of inline-literal `5` for the per-level count cap. No named constant is introduced. See Out of scope for the rationale.

### Item 3 — drop the band-wrapping logic

**`build_editor_levels_table` (`beep_test_settings.rs:329-402`).** Replace:

```rust
for (band_idx, band_levels) in levels.chunks(EDIT_BAND_WIDTH).enumerate() {
    let level_index_offset = band_idx * EDIT_BAND_WIDTH;
    // ...header row + cell rows + padding + odd-layer blank-row padding...
}
```

with a single straight pass over `levels` that builds one header row and one stack of cell rows (using `max_count = levels.iter().map(|l| l.count as usize).max().unwrap_or(0)` directly). No band-padding, no offset arithmetic, no odd-layer blank row. The existing odd-layer blank-row trick at lines 387–394 was specifically for keeping multiple bands' rendered heights aligned; with a single band that's no longer needed.

Remove the constant `const EDIT_BAND_WIDTH: usize = 10;` at line 261.

**Main view's table builder (`beep_test.rs:175-269`).** Same simplification: drop the `chunks(BAND_WIDTH)` loop, render one header row and one column of cell rows. Remove the constant `const BAND_WIDTH: usize = 10;` at line 37. Update the doc comment at lines 177–178 from "wrap onto additional rows when there are more user levels than `BAND_WIDTH`" to a sentence describing the single-row layout.

The two view builders implement the same conceptual layout — keeping them in sync is the point of removing the wrap from both.

### Pattern reference (per `.claude/rules/patterns.md`)

The `add_disabled` predicate and styling match the existing `remove_disabled` pattern in the same file (`build_edit_panel`, lines 496–503). The handler-side guard matches Chunk 1's `if level.count < 5` pattern at `mod.rs:3521-3527`. The English label `ADD LEVEL` matches the existing English value pattern of `REMOVE LEVEL` (all caps, verb-noun, no plus sign).

---

## Testing

No new unit test. Lean process applies (refbox UI work, no state-machine change). Verification is via walkthrough.

**Walkthrough scenarios:**

1. **ADD LEVEL label visible.** From a fresh Edit Levels page (start with the default config), confirm the upper-left button reads **ADD LEVEL** in English (and the parallel translated text in French/Spanish if the operator has those locales available).
2. **Add works under cap.** With fewer than 10 levels staged, tap ADD LEVEL and confirm a new level appends to the right of the table; the new column is selected; the per-level edit panel updates accordingly.
3. **Cap reached at 10.** Keep tapping ADD LEVEL until 10 levels exist. The button must visibly **gray out** (disabled style) and become non-responsive on tap. The handler also rejects any direct trigger; visually no 11th column should ever appear.
4. **Removing re-enables Add.** With 10 levels, tap REMOVE LEVEL once. ADD LEVEL must re-enable (green style + responsive).
5. **Layout sanity at boundary counts.** Confirm the table renders cleanly at 1, 5, and 10 levels — single row of columns, no visual artifacts where the band code used to live.

`just check` is green (fmt, clippy, full test suite, audit).

---

## Acceptance criteria

The five walkthrough scenarios above all pass on the running refbox; `just check` is green.

---

## Out of scope (intentionally deferred)

- **Renaming the translation key `beep-test-edit-new`** to something semantic like `beep-test-edit-add-level`. The key name will drift from its rendered value, which is a real (small) future-readability cost — accepted per Approach A in brainstorming. A focused rename branch can address it later if it becomes annoying.
- **Introducing a named `MAX_LEVELS` constant.** Chunk 1's per-level cap used inline-literal `5`; this chunk uses inline-literal `10`. Consistency with the established pattern over local code-quality polish. A future refactor could extract both caps into named constants if motivated.
- **Behavior for legacy configs with >10 levels.** If an operator loads a config file with more than 10 levels (e.g., a config saved before this cap landed), the table will render in a single overflowing row and the ADD LEVEL button will be disabled. REMOVE LEVEL still works, so the operator can decrement to 10 or below. No data is auto-truncated; no migration is performed. This mirrors Chunk 1's "existing levels with count > 5 are not modified" semantics. If the operator's config files in practice always have ≤ 10 levels, this case is unreachable.
- **Translations for the rest of the `beep-test-edit-*` family.** This chunk fixes the English fallbacks for `beep-test-edit-new` and `beep-test-edit-remove` only. Other keys in the family (`-time`, `-count`, `-selected`, `-levels`, etc.) keep their current values (some English-fallback in some locales). A future `chore/translations/*` branch can clean up the full family per locale.
- **Raising or changing Chunk 1's per-level lap-count cap of 5.** Not touched.
- **The Chunk 6 LED panel work** — separate concern, already landed.

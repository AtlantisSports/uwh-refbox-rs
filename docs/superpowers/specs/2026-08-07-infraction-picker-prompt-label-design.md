# Infraction picker: prompt instead of "Unknown"

**Date:** 2026-08-07
**Branch:** `fix/refbox/infraction-picker-prompt-label`
**Crate:** `refbox` only

## Problem

On the **Add Foul** and **Add Warning** pages, the blue bar above the infraction icon grid
reads `Infraction: Unknown` before the operator has picked anything.

"Unknown" reads as a legitimate choice, but it is not one. Both pages refuse to save until a
real infraction is selected — the save button stays greyed out
(`foul_add_can_commit`, `warning_add_can_commit`). The bar is showing an empty state as if it
were a value.

## Decision

The bar reads **`Infraction: Make selection`** while nothing is selected. As soon as any icon
is tapped it shows that infraction's name, exactly as today. Tapping `?` returns it to
`Infraction: Make selection`.

The `Infraction:` prefix is retained (rather than showing a bare "Make selection") so the bar
always identifies which field it belongs to and the text does not shift position as the
operator picks.

## Scope boundary

Explicitly unchanged:

- **The `?` icon itself** and the icon grid layout.
- **The penalty editor.** It renders the same grid via `make_penalty_dropdown(infraction,
  false)` — the `false` suppresses the name bar — so it never displayed this text.
- **Saved list rows.** The existing `unknown = Unknown` string stays as-is. A penalty can
  legitimately be saved with no infraction when "track fouls and warnings" is off, and a row
  in the list is reporting a recorded fact, not prompting for input. Renaming the shared key
  would have made those rows read "Make selection", which is wrong.
- **Enforcement.** No change to when saving is allowed; this is a label change only.

## Implementation

New Fluent key `select-infraction`, added to all 15 locales next to the existing `unknown`
key.

`shared_elements.rs` gains a small helper:

```rust
fn infraction_bar_label(infraction: Infraction) -> String {
    let value = if infraction == Infraction::Unknown {
        fl!("select-infraction")
    } else {
        inf_short_name(infraction)
    };
    fl!("infraction", infraction = value)
}
```

`make_penalty_dropdown` calls it in place of the inline `fl!("infraction", ...)`.

The prompt is substituted into the **existing** `infraction = Infraction: {$infraction}`
template rather than being a new full-sentence string. All 15 locales already translate that
template, so each language's prefix (`Verstoß:`, `反則:`, `犯规类型：`, …) stays automatically
in sync between the empty and populated states, and translators only see one short phrase.

### Translations

| Locale | Value |
|---|---|
| de-DE | Bitte auswählen |
| en-US | Make selection |
| es | Seleccione una opción |
| fr | Faire une sélection |
| id-ID | Pilih salah satu |
| it-IT | Effettua una scelta |
| ja-JP | 選択してください |
| ko-KR | 선택하세요 |
| ms-MY | Sila pilih |
| nl-NL | Maak een keuze |
| pt-PT | Selecione uma opção |
| th-TH | กรุณาเลือก |
| tl-PH | Pumili |
| tr-TR | Seçim yapın |
| zh-CN | 请选择 |

## Acceptance criteria

1. Add Foul with nothing selected → bar reads `Infraction: Make selection`.
2. Add Warning with nothing selected → same.
3. Pick any infraction → bar reads that infraction's name.
4. Tap `?` again → bar returns to `Infraction: Make selection`.
5. With "track fouls and warnings" off, save a penalty with no infraction → its list row
   still reads "Unknown".
6. `just check` clean.

## Tests

`infraction_bar_prompts_until_a_selection_is_made` and
`infraction_bar_names_a_chosen_infraction` in `shared_elements.rs`. Both compare against
`fl!` keys rather than literal English so they hold in any locale.

Mutation-checked: reverting the helper to the old behaviour fails the first test with
`left: "Infraction: Unknown"` / `right: "Infraction: Make selection"`.

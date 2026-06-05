# Design: Surface "2 Halves / 1 Period" in the Half Length editor

Date: 2026-06-05
Crate: `refbox` (only)
Status: approved (brainstorm)

## Problem

The refbox supports single-period games via a `single_half` setting on the shared game
config, but **there is no operator-facing control to turn it on or off.** Today `single_half`
can only be set by editing a config file or by a portal schedule pushing it down. A code
comment (`refbox/src/app/message.rs`, the `BoolGameParameter::SingleHalf` variant marked
`#[expect(dead_code)]`) records the original intent: "Single Half toggle was removed from Game
Options in ADR-009 Task 14; TODO #1 will surface it from inside the Half Length parameter
editor." This change implements that TODO.

## Current state (what already works)

`single_half` lives on `Game` in `uwh-common/src/config.rs` (default `false`). When it is
`true`, the following already happen with no further work:

- The Game Options page button relabels from **Half Length** to **Game Length**
  (`refbox/src/app/view_builders/configuration.rs`, Half Length button ~687-696).
- The **Half-Time Length** button is disabled.
- The game flow skips half-time and the second half
  (`refbox/src/tournament_manager/mod.rs` `end_first_half()` ~1329-1354).

A toggle handler also already exists (`refbox/src/app/mod.rs` ~2596-2597) but is unreachable
because no UI emits the message.

What is missing is purely the **operator control**, which we add to the Half Length parameter
editor (`build_game_parameter_editor()` for `LengthParameter::Half`, configuration.rs
~1267-1332). That editor currently shows a title, a time keypad, help text, and Cancel/Done.

## Desired behaviour

### On the Half Length edit screen

- Add a **segmented selector** at the top of the editor: two buttons, **2 HALVES** and
  **1 PERIOD**, with the active one highlighted. Below it remain the existing time keypad and
  Cancel / Done buttons.
- Reuse existing theme button styles only: active segment uses the app's standard
  "selected/confirm" style, inactive uses the standard grey button style. No new styling
  approach is introduced.
- Tapping **1 PERIOD** stages `single_half = true`; tapping **2 HALVES** stages
  `single_half = false`.
- The screen **title** reflects the staged choice: **HALF LENGTH** (2 Halves) /
  **GAME LENGTH** (1 Period). The **help text** updates with it.
- The keypad always edits the **same single duration**. It is "length of a half" in 2-Halves
  mode and "length of the whole game" in 1-Period mode. Switching the selector never clears
  the entered value.
- The format choice **and** the time are **staged together** and committed only on **Done**;
  **Cancel** discards both.

### Back on the Game Options page (already built, now reachable)

- 1-Period mode: Half Length button reads **Game Length**; Half-Time button is disabled.
- The half-time length value is **retained** (disabled, not erased) so switching back to
  2 Halves restores it.
- Overtime and sudden-death are unaffected — they keep working off their own toggles.

## Wording (literal)

- Selector buttons: **"2 Halves"** and **"1 Period"**.
- Help text — 2 Halves: **"Length of each half during regular play"**; 1 Period:
  **"Length of the game during regular play"**.
- Main button label: **"Half Length"** (2 Halves) / **"Game Length"** (1 Period) — already
  implemented via existing keys.

## Architecture notes

- The parameter editor currently stages only the `Duration` being edited
  (`AppState::ParameterEditor(LengthParameter, Duration)`). It will be extended to also carry
  the staged `single_half` choice for `LengthParameter::Half`, so both commit on Done / discard
  on Cancel. Exact state shape is an implementation decision for the plan.
- Reuse the existing `single_half` field, the existing `BoolGameParameter::SingleHalf` handler,
  and remove the `#[expect(dead_code)]` now that the variant is reachable.

## Translations

New translatable strings: **"2 Halves"**, **"1 Period"**, and the 1-Period help text. Added to
`refbox/translations/en-US/refbox.ftl` now. The other locales (`es`, `fr`, `de-DE`, `it-IT`,
`pt-PT`, …) start with English placeholders until real translations are supplied — flagged so
coverage isn't silently lost. (Per the translation-coverage check, placeholder entries must be
recorded, not assumed complete.)

## Scope boundary

- Only `refbox`. No `uwh-common` change (the field already exists), no wire-format change, no
  portal change, no `wireless-remote`.
- Not changing overtime / sudden-death behaviour.
- Not changing the game-flow logic for single-period games (already implemented).
- Not touching Change 2 (Game Block) — separate branch/spec.

## Acceptance criteria (operator-observable)

1. On the Game Options page, tap **Half Length**. The editor shows a **2 Halves / 1 Period**
   selector with 2 Halves active by default (for a default config).
2. Tap **1 Period**: title changes to **Game Length**, help text updates, the highlight moves;
   the entered time is unchanged.
3. Tap **Done**: back on Game Options, the button now reads **Game Length** and **Half-Time**
   is greyed out.
4. Re-open the editor, tap **2 Halves**, **Done**: button reads **Half Length** again and the
   previously set Half-Time length is intact.
5. **Cancel** in the editor after changing the selector leaves the format unchanged.

## Process

Lean process (`.claude/rules/plan-execution.md`): `refbox` UI, low blast radius. Compilation +
`just check` + manual observation. New translation keys are mechanical. The game-flow logic for
single-period games is unchanged, so no tournament-manager re-verification is required.

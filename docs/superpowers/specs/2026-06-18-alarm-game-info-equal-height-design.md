# Spec: Game-info table + equal-height Alarm Button on Main screen

**Date:** 2026-06-18
**Crate:** `refbox` (lean process — UI + translations only, no `uwh-common` / state-machine impact)
**Status:** Approved design, implemented on `feat/refbox/equal-alarm-game-info-height`

## Problem

On the Main screen, when the **Alarm Button** is enabled and **Track Fouls and Warnings** is
disabled, the centre column showed the game clock, then a small fixed "GAME INFO" label button,
then the Alarm Button filling all remaining space (thin label, huge button). The operator wants:

1. the two to be roughly equal height, AND
2. the game-info **table** shown instead of the plain "GAME INFO" label (the same table already
   shown when Alarm is off), AND
3. the label text spelled out / simplified to just **INFORMATION** (dropping "Game").

## Goal

- **Alarm on + Track Fouls off:** show the game-info table (tappable → Game Options) and the Alarm
  Button each at ~half height below the clock (even 50/50), no header.
- **Alarm on + Track Fouls on:** unchanged shape — the game-info label bar on top, Alarm +
  Warnings split below — but the label now reads **INFORMATION**.
- **Alarm off:** unchanged.
- Rename the `game-info` label to just the word "information" in all 15 locales.

## Scope

**In scope:**
- `refbox/src/app/view_builders/main_view.rs` — `build_main_view()`, the `manual_alarm_enabled`
  block.
- `refbox/translations/<15 locales>/refbox.ftl` — the `game-info` key value.

**Explicitly out of scope:**
- The table's contents/columns (reuse `game_info_rows` / `render_game_info_table` verbatim).
- The Alarm-off layout, the game clock, timeout/start-now buttons, scores, penalties.
- Behaviour of the buttons (Game info area → `ShowGameDetails`; Alarm → buzzer).
- No `uwh-common`, `Message`, or dependency changes; no new translation keys.

## Design

### Layout (`main_view.rs`, `if manual_alarm_enabled { ... }`)

The game-info element is built inside the `track_fouls_and_warnings` branches rather than once
above them:

- `track_fouls_and_warnings == true`: push `make_button(fl!("game-info"))` (default fixed height)
  as the top bar, then `row![alarm_face, warnings_zone]` (unchanged).
- `track_fouls_and_warnings == false`: push a `button` wrapping
  `render_game_info_table(game_info_rows(snapshot, game_config, using_uwhportal, schedule, teams,
  last_game_scores))` at `Length::Fill` (tappable → `ShowGameDetails`), then `alarm_face` (already
  `Length::Fill`). Two `Fill` siblings under the clock → roughly equal height.

The table construction mirrors the existing alarm-disabled branch (`align_y(Top)`, `Fill` width and
height, `light_gray_button`, `padding(PADDING)`).

### Rename (translations)

The `game-info` key value becomes just the word for "information" in every locale. The 14
non-English values are best-guess and **must be flagged for native-speaker review**:

| Locale | New | Locale | New | Locale | New |
|---|---|---|---|---|---|
| en-US | INFORMATION | id-ID | INFORMASI | ms-MY | MAKLUMAT |
| de-DE | INFORMATIONEN | it-IT | INFORMAZIONI | th-TH | ข้อมูล |
| es | INFORMACIÓN | nl-NL | INFORMATIE | tl-PH | IMPORMASYON |
| fr | INFORMATIONS | pt-PT | INFORMAÇÕES | tr-TR | BİLGİ |
| | | ja-JP | 情報 | ko-KR | 정보 |
| | | zh-CN | 信息 | | |

## Acceptance criteria

1. Alarm on + Track Fouls off → game-info table + Alarm Button, roughly equal height, no header,
   table tappable to Game Options.
2. Alarm on + Track Fouls on → "INFORMATION" label bar on top, Alarm + Warnings split below
   (unchanged layout).
3. Alarm off → unchanged.
4. The renamed label/value reads "INFORMATION" (or locale equivalent) wherever it appears.
5. `just check` passes (fmt, clippy `-D warnings`, tests).

## Verification

Launch refbox; toggle Alarm Button (Sound Options) and Track Fouls and Warnings to exercise the
three cases above.

## Follow-up

The 14 non-English `game-info` values are best-guess translations and need native-speaker review.

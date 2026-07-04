# Game-info table + equal-height Alarm Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** On the Main screen, when the Alarm Button is enabled and Track Fouls and Warnings is disabled, show the game-info table (tappable → Game Options) and the Alarm Button at roughly equal height; keep the both-enabled layout but rename its top label to just "INFORMATION" across all 15 locales.

**Architecture:** Restructure the `manual_alarm_enabled` block in `build_main_view()` so the game-info element is built inside each `track_fouls_and_warnings` branch: a fixed-height label bar (Track Fouls on) or the `Length::Fill` game-info table (Track Fouls off, paired with the `Fill` alarm). Reuse the existing `game_info_rows` / `render_game_info_table` helpers. Rename the `game-info` translation value in all 15 locales. No `uwh-common`, `Message`, or dependency changes.

**Tech Stack:** Rust 2024, iced 0.13, Fluent translations.

## Global Constraints

- MSRV Rust 1.85; clippy `-D warnings`; no new `unwrap()`/`expect()`.
- No new dependencies, no new translation keys, no `uwh-common`/`Message` changes.
- Non-English translations are best-guess → flag for native-speaker review.
- Lean process (refbox UI): code review once at the end.

---

### Task 1: Restructure the alarm block + add the table (Track Fouls off)

**Files:**
- Modify: `refbox/src/app/view_builders/main_view.rs` — `if manual_alarm_enabled { ... }` block.

- [x] Remove the pre-branch game-info label push (and the `game_info_height` conditional from the
  first iteration).
- [x] In the `track_fouls_and_warnings == true` branch: push `make_button(fl!("game-info"))`
  (default fixed height) as the top bar before the `row![alarm_face, warnings_zone]`.
- [x] In the `else` branch: push a `button` wrapping
  `render_game_info_table(game_info_rows(snapshot, game_config, using_uwhportal, schedule, teams,
  last_game_scores))` at `Length::Fill` (container `align_y(Top)`, `padding(PADDING)`,
  `light_gray_button`, `on_press(Message::ShowGameDetails)`), then push `alarm_face`.
- [x] `cargo build -p refbox` clean.

### Task 2: Rename `game-info` → "information" in all 15 locales

**Files:**
- Modify: `refbox/translations/{en-US,de-DE,es,fr,id-ID,it-IT,nl-NL,pt-PT,ja-JP,ko-KR,ms-MY,th-TH,tl-PH,tr-TR,zh-CN}/refbox.ftl`

- [x] Set `game-info` to: en-US INFORMATION, de-DE INFORMATIONEN, es INFORMACIÓN, fr INFORMATIONS,
  id-ID INFORMASI, it-IT INFORMAZIONI, nl-NL INFORMATIE, pt-PT INFORMAÇÕES, ja-JP 情報, ko-KR 정보,
  ms-MY MAKLUMAT, th-TH ข้อมูล, tl-PH IMPORMASYON, tr-TR BİLGİ, zh-CN 信息.

### Task 3: Verify

- [x] `just check` (fmt, clippy `-D warnings`, tests) — EXIT 0.
- [ ] Visual walkthrough (manual): Case 1 (Alarm on + Track Fouls off → table + alarm equal
  height); Case 2 (Alarm on + Track Fouls on → "INFORMATION" bar + alarm/warnings, unchanged
  layout); Case 3 (Alarm off → unchanged).
- [ ] Commit (after user approval).

## Status

Implemented on `feat/refbox/equal-alarm-game-info-height` (worktree). `just check` green. Awaiting
visual walkthrough + commit approval. Follow-up: native review of the 14 best-guess translations.

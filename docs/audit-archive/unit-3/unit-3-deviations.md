## Out of scope (tracked elsewhere)

- **View Mode button** in User Options — ADR 010. The hidden spacer in Task 6 is the placeholder.
- **Live preview** (sound / starting-sides / brightness take effect while editing) — ADR 014. Tasks 10–12 must retain non-live semantics; do not push to subsystems from inside view builders.
- **Cold-restart state recovery** — ADR 013. Do not touch the settings-done trigger path; ADR 013 will add a separate entry point.

---

## Deviations log

### Task 5

- **`ConfigPage::Game` early-returns from `Message::ApplyConfigPage`.** The plan listed `ConfigPage::Game => self.apply_game_options()` but `apply_game_options()` does not exist yet (Task 3 deferred it to Task 8). The `Game` arm was bucketed alongside `Language | Main | User` for an early `return Task::none()`. Task 8 will move `Game` out of that arm when it adds `apply_game_options()`.
- **Stale `#[expect(dead_code)]` removals.** The wiring in this task made `apply_remote_options`, `capture_snapshot_for`, `revert_from_snapshot`, and `ConfigPage::User` live, so their `#[expect(dead_code)]` attributes became unfulfilled and had to be removed.

### Task 6

- **Plan-snippet `button::primary` / `button::secondary` styles do not exist** in this codebase. Used `light_gray_button` for tiles (matching Main's pre-Task-7 tile style) and `gray_button` for the back button.
- **`MIN_BUTTON_SIZE` is `f32`**, so `.height(MIN_BUTTON_SIZE)` was wrapped as `.height(Length::Fixed(MIN_BUTTON_SIZE))`.
- **Sound tile uses `fl!("sound-options")`, not `fl!("sound")`.** The `sound` key is parameterized for a status-display widget (`sound = SOUND: { $sound_text }`); `sound-options` is the page-tile label.
- **`make_back_button` reuses `shared_elements::make_button`** rather than constructing a button manually — matches the two existing back-button call sites in the codebase (`game_info.rs:63`, `warnings_fouls_summary.rs:91`).
- **Layout pattern was wrong on first ship and corrected during Task 7 walkthrough.** The User Options page tiles initially used `LARGE_TEXT` + manual `button(text(...))` construction instead of the established `make_button(fl!("...")).style(light_gray_button).on_press(...)` pattern. The bottom row also used `[h_space, h_space, back]` with `.height(Length::Fill)`, both wrong. Final form (corrected in Task 7's commit, since it's the same file):
  - Tiles use `make_button(fl!("...")).style(light_gray_button).on_press(...)` with no height override.
  - `tiles` row has no `.height(Length::Fill)` — buttons hug their natural `MIN_BUTTON_SIZE` and the column's `vertical_space()` absorbs leftover.
  - Bottom row is `[back, h_space, h_space]` (BACK on the left, Cancel position) with no `.height(Length::Fill)`.
  - `make_back_button` style was changed from `gray_button` to `red_button` to match the codebase's back-button convention (the two existing back buttons in `game_info.rs` and `warnings_fouls_summary.rs` both use `red_button`).

### Task 7

- **Plan-snippet `button::primary` / `tile = |label, dest|` closure** were swapped for inlined `make_button(fl!("...")).style(light_gray_button).on_press(...)` per the established pre-Task-7 Main pattern. `fl!` requires literal arguments so the closure form was incompatible.
- **`make_game_time_button` 5-arg form** `(snapshot, false, false, mode, clock_running)` matches every other call site in `configuration.rs` (the plan snippet's 4-arg form was incorrect).
- **Picker placement** — moved into the existing blank `make_button("")` spacer slot in the middle column of the first parameter row of `make_event_config_page` (between `single-half` and `using-uwh-portal`), rather than added as a new full-width row at the top of Game Options. The user identified this during walkthrough.
- **Picker label-size flag** changed from `(true, game_large_text)` to `(false, game_large_text)` to match the surrounding parameter buttons (`single-half`, `using-uwh-portal` both use `(false, true)`). The conditional value-size logic (`game_large_text`) was kept to handle long "none-selected" / "loading" labels.
- **`Message::LanguageSelectComplete` route** changed from `ConfigPage::App` to `ConfigPage::Main` (`refbox/src/app/mod.rs:1960`). With Task 7 making LANGUAGE reachable directly from Main's grid, the post-Language navigation now returns to Main. Until Task 9 removes App Options' inner Language button, users entering Language from that path will return to Main instead of App Options — a transitional inconsistency that resolves when Task 9 ships.
- **User Options page layout corrections** (see Task 6 deviations) shipped in the Task 7 commit because they share the same file; the Task 6 commit at `0686efa` is left as-is and the corrections are part of `4ba2753`.
- **Post-picker navigation routing** — first discovery (during Task 8 smoke test): `Message::ParameterEditComplete` at `refbox/src/app/mod.rs:1959` and `:1963` routed back to `ConfigPage::Main` after `KeypadPage::GameNumber` and `ListableParameter::Game`. With Task 7 moving the picker into Game Options, these should return to `ConfigPage::Game`. Both were changed in the Task 8 commit. Second discovery (during ADR-009 expansion-work smoke test): `Message::ParameterSelected` at `refbox/src/app/mod.rs:2038` had its own `ConfigPage::Main` routing for `ListableParameter::Game` — separate code path from `ParameterEditComplete`, missed on the first fix pass. Also changed in the Task 8 commit.

### Task 8

- **Plan-snippet `button::primary` / `button::secondary` styles** — same lesson as Tasks 6/7. Used `red_button` for Cancel (matching `make_back_button`'s style) and `green_button` for Apply (matching the existing Done button's style in this and other pages).
- **Footer geometry** — plan called for `row![cancel, apply]` (50/50 full-width). Used `row![cancel, horizontal_space(), apply]` to match the page's 3-column rhythm (every other row in `make_event_config_page` is 3-cell). Discussed and approved before implementation.
- **`apply_game_options()` body — Option B (apply with gates).** The plan specified the per-page Apply for Game but left the body undefined. Three options were considered: (A) naive slice-apply ignoring gates, (B) mirror `ConfigEditComplete`'s gate logic, (C) defer the Apply body. Option B was selected to preserve mid-game safety. New `ConfirmationKind` variants — `GameConfigChangedFromApply(GameConfig)`, `GameNumberChangedFromApply`, `UwhPortalIncompleteFromApply` — share UI with their global-Done counterparts but commit only the Game slice and route back to settings (not out to `MainPage`). New helper `apply_game_confirmation` handles the post-confirmation actions.
- **`PageEntrySnapshot::Game` expansion (4 added fields).** Surfaced during code review (issue I-2): Game Options in portal mode edits App-slice fields (`using_uwhportal`, `current_event_id`, `current_court`, `schedule`) but the page-entry snapshot only covered `config` + `game_number`. Cancel didn't revert those edits, and per-page Apply silently lost them across the next settings entry (which rebuilds `edited_settings` from `self.X`). Fixed by extending the snapshot variant; `capture_snapshot_for(Game)`, `revert_from_snapshot`, `page_has_changes`, and `apply_game_options` updated to match. Documented in side plan `docs/superpowers/plans/2026-04-29-game-options-snapshot-expansion.md`.
- **`#[allow(clippy::large_enum_variant)]` on `PageEntrySnapshot`.** Added with justification comment that `PageEntrySnapshot` is a singleton — `RefBoxApp.page_entry_snapshot` holds at most one variant at a time, so the variant-size disparity from the inline `Schedule` doesn't compound. Box-the-fields fix would cascade through capture/revert/page_has_changes/apply for no real benefit. Discussed and approved.
- **Auto-clear of dependent fields when picker selections change.** In `Message::ParameterSelected`: picking a new event clears `current_court`, `game_number`, and `schedule` (the new schedule is async-fetched); picking a new court clears `game_number`. Prevents the stale-state path where `game_number` retains a value from a previous event's filtered list whose court no longer exists in the new event's schedule. Eliminates the secondary `uwhportal_incomplete` gate-paths through cross-change rather than through invalid initial pick.
- **`uwhportal_incomplete()` extracted as a method on `EditableSettings`.** Used by `apply_game_options` (gate check) and `make_cancel_apply_footer` (Apply enable). Lives in `configuration.rs` alongside the type. Keeps gate and disable in sync.
- **Apply button disabled when `uwhportal_incomplete()` returns true on Game Options.** New behaviour beyond the plan: pressing Apply when portal state is incomplete would only fire a confirmation dialog with no actionable choice. Disabling Apply skips that wasteful round-trip. Mid-game gates (`GameConfigChangedFromApply`, `GameNumberChangedFromApply`) still fire as confirmation dialogs because those have real action choices (Keep / End / Discard).
- **Stale `#[expect(dead_code)]` removals.** Task 8 wired up `Message::ApplyConfigPage(Game)` and `Message::CancelConfigPage(Game)` for the first time in production code, fulfilling the previously-unfulfilled `#[expect(dead_code)]` attributes on those variants. Removed.
- **Out-of-scope discoveries filed as new ADRs.** Two issues surfaced during smoke testing but were not in Task 8 scope:
  - "Unknown vs Unknown" placeholders in the game picker when teams data is still in flight — captured in ADR 017 (Portal data lifecycle), which also covers the wasteful startup event-list fetch when `using_uwhportal=false`.
  - Event picker hides events at the top of the reversed display when `current_event_id` has a non-smallest BTreeMap index — captured in ADR 018 (Event picker sort order), which also covers the operator request to sort events by date proximity rather than EventId.
  - Both ADRs are filed as `proposed` with `Behavior definition required` — the operator owes design input before planning/implementation.

# ADR 009 — Settings Navigation and Layout — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework the Rust refbox's settings UI to match the web refbox's 2×2 grid → User Options → leaf-page navigation, with per-page Cancel/Apply commits and Back-only chrome on navigation-only pages. The game-number picker moves from Main to Game Options. Live preview of sound and starting-side changes is out of scope (ADR 014).

**Architecture:**

- **Navigation:** Add `ConfigPage::User` between Main and Display/Sound. Main becomes a 2×2 grid (Game | App, User | Language); User Options shows Display, a hidden View-Mode spacer (for ADR 010), and Sound.
- **Save model:** Each editing page commits its own slice to `self.config` on Apply. A page-entry snapshot drives both Cancel-revert and Apply-enabled detection. Today's single global `apply_settings_change()` splits into per-page slice-apply functions.
- **Chrome:** Main and User Options carry `BACK` only. Game, App, Display, Sound, Manage Remotes, and Language each carry `CANCEL` + `APPLY`. `APPLY` is disabled until a change from page-entry is detected; `CANCEL` is always enabled with fixed label.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13 (Elm-like), Fluent (`translations/`), `confy` for persistence.

**Scope boundary:** All work is inside the `refbox` crate. No changes to `uwh-common`, `overlay`, `schedule-processor`, or `wireless-remote`.

**Branch discipline:** This plan will be executed on a dedicated worktree off `master`, branch `refactor/refbox/settings-navigation`. **Do not create this branch or any commits without explicit user approval each time.** The plan's commits are written as one-liners; every commit gate pauses for approval.

---

## File Structure

Files to touch:

- **Modify:** `refbox/src/app/message.rs` — add `ConfigPage::User` variant; add per-page `Message::ApplyConfigPage(ConfigPage)` and `Message::CancelConfigPage(ConfigPage)` variants; retire or repurpose the global `Message::ConfigEditComplete { canceled }` path for the pages that now commit per-page.
- **Modify:** `refbox/src/app/view_builders/configuration.rs` — rework `make_main_config_page` to a 2×2 grid with Back-only chrome; add `make_user_config_page`; add Cancel/Apply footer helpers to each editing-page builder; move game-number picker into `make_event_config_page`; remove the inner Language button from `make_app_config_page`; extend the dispatcher for `ConfigPage::User`.
- **Modify:** `refbox/src/app/mod.rs` — split `apply_settings_change()` into one slice-apply per editing page; add `page_entry_snapshot: Option<PageEntrySnapshot>` to `RefBoxApp`; capture snapshot on `ChangeConfigPage`; wire `ApplyConfigPage` and `CancelConfigPage` handlers; compute an Apply-enabled flag per page from buffer-vs-snapshot diff.
- **Modify:** `refbox/translations/*/refbox.ftl` (all 15 locales: `de-DE`, `en-US`, `es`, `fr`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`, `tr-TR`, `zh-CN`) — add `apply` and `user-options` keys.
- **Create (tests):** a `#[cfg(test)] mod tests` block in `refbox/src/app/view_builders/configuration.rs` for snapshot-diff unit tests (pure logic; no iced rendering).

Shape of the new snapshot type (defined inside `refbox/src/app/mod.rs`):

```rust
#[derive(Debug, Clone, PartialEq)]
enum PageEntrySnapshot {
    Game {
        config: uwh_common::config::Game,
        game_number: uwh_common::uwhportal::schedule::GameNumber,
    },
    App {
        using_uwhportal: bool,
        current_event_id: Option<uwh_common::uwhportal::schedule::EventId>,
        current_court: Option<String>,
        schedule: Option<uwh_common::uwhportal::schedule::Schedule>,
        mode: crate::config::Mode,
        collect_scorer_cap_num: bool,
        track_fouls_and_warnings: bool,
        confirm_score: bool,
    },
    Display {
        white_on_right: bool,
        brightness: matrix_drawing::transmitted_data::Brightness,
        hide_time: bool,
    },
    Sound {
        sound: crate::sound_controller::SoundSettings,
    },
    Remotes {
        remotes: Vec<crate::sound_controller::RemoteInfo>,
    },
    Language {
        original_language: Option<crate::app::languages::Language>,
        pending_language: Option<crate::app::languages::Language>,
    },
}
```

---

## Phase 0 — Preconditions

### Task 0: Create the worktree and confirm clean state

**Files:** none (shell only)

- [ ] **Step 1: Confirm approval to create the branch.**

Do not proceed without explicit user approval for the branch name `refactor/refbox/settings-navigation` off `master`.

- [ ] **Step 2: Create a worktree off master.**

Run:
```bash
git fetch origin
git worktree add -b refactor/refbox/settings-navigation ../refbox-settings-worktree master
cd ../refbox-settings-worktree
```

Expected: fresh worktree created; `git status` clean.

- [ ] **Step 3: Verify baseline passes.**

Run: `just check`

Expected: PASS. If it does not pass on `master`, stop and escalate — the refactor must start from a green baseline.

---

## Phase 1 — Translation scaffolding

### Task 1: Add `apply` and `user-options` translation keys to all 15 locales

**Files:**

- Modify: `refbox/translations/de-DE/refbox.ftl`
- Modify: `refbox/translations/en-US/refbox.ftl`
- Modify: `refbox/translations/es/refbox.ftl`
- Modify: `refbox/translations/fr/refbox.ftl`
- Modify: `refbox/translations/id-ID/refbox.ftl`
- Modify: `refbox/translations/it-IT/refbox.ftl`
- Modify: `refbox/translations/ja-JP/refbox.ftl`
- Modify: `refbox/translations/ko-KR/refbox.ftl`
- Modify: `refbox/translations/ms-MY/refbox.ftl`
- Modify: `refbox/translations/nl-NL/refbox.ftl`
- Modify: `refbox/translations/pt-PT/refbox.ftl`
- Modify: `refbox/translations/th-TH/refbox.ftl`
- Modify: `refbox/translations/tl-PH/refbox.ftl`
- Modify: `refbox/translations/tr-TR/refbox.ftl`
- Modify: `refbox/translations/zh-CN/refbox.ftl`

- [ ] **Step 1: Add the two new keys in English.**

In `en-US/refbox.ftl`, add after the existing `back = BACK` line in the `# Multipage` block:

```
apply = APPLY
user-options = USER OPTIONS
```

- [ ] **Step 2: Add each key to every other locale.**

For each of the remaining 14 locales, add `apply` and `user-options` with localised uppercase values. Reasonable defaults (an operator who reads the language will want to verify these; the author is the accepted translator for now — escalate to the user for any they want a native speaker to validate):

| locale | apply | user-options |
|--------|-------|--------------|
| de-DE  | ANWENDEN | BENUTZEROPTIONEN |
| es     | APLICAR | OPCIONES DE USUARIO |
| fr     | APPLIQUER | OPTIONS UTILISATEUR |
| id-ID  | TERAPKAN | OPSI PENGGUNA |
| it-IT  | APPLICA | OPZIONI UTENTE |
| ja-JP  | 適用 | ユーザー設定 |
| ko-KR  | 적용 | 사용자 옵션 |
| ms-MY  | GUNA | PILIHAN PENGGUNA |
| nl-NL  | TOEPASSEN | GEBRUIKERSOPTIES |
| pt-PT  | APLICAR | OPÇÕES DO UTILIZADOR |
| th-TH  | ใช้ | ตัวเลือกผู้ใช้ |
| tl-PH  | ILAPAT | MGA OPSYON NG USER |
| tr-TR  | UYGULA | KULLANICI SEÇENEKLERİ |
| zh-CN  | 应用 | 用户选项 |

- [ ] **Step 3: Format-check.**

Run: `just fmt-check && just lint`

Expected: PASS. Translation files are not lint-checked, but this confirms no unrelated breakage.

- [ ] **Step 4: Commit.**

Pause for user approval, then:
```bash
git add refbox/translations/
git commit -m "chore(refbox): add apply and user-options translation keys"
```

---

## Phase 2 — Message enum and routing scaffolding

### Task 2: Add `ConfigPage::User` variant + placeholder routing

**Files:**

- Modify: `refbox/src/app/message.rs:409`
- Modify: `refbox/src/app/view_builders/configuration.rs:110-134` (dispatcher)
- Modify: `refbox/src/app/mod.rs` (any exhaustive `match ConfigPage { … }` sites)

- [ ] **Step 1: Add the variant.**

In `refbox/src/app/message.rs:409`, change:

```rust
pub enum ConfigPage {
    Main,
    Game,
    Sound,
    Display,
    App,
    Remotes(usize, bool),
    Language,
}
```

to:

```rust
pub enum ConfigPage {
    Main,
    Game,
    Sound,
    Display,
    App,
    User,
    Remotes(usize, bool),
    Language,
}
```

- [ ] **Step 2: Build to surface every non-exhaustive `match` on `ConfigPage`.**

Run: `cargo check -p refbox --all-features 2>&1 | grep -E "non-exhaustive|pattern|match" | head -40`

Expected: a small list of `match ConfigPage { ... }` sites (dispatcher, update handler). Note each location.

- [ ] **Step 3: Wire a placeholder in the view dispatcher.**

In `refbox/src/app/view_builders/configuration.rs:123-133`, add a `ConfigPage::User` arm that temporarily routes to `make_main_config_page`:

```rust
    match page {
        ConfigPage::Main => make_main_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Game => make_event_config_page(snapshot, settings, events, mode, clock_running),
        ConfigPage::Sound => make_sound_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Display => make_display_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::App => make_app_config_page(mode, snapshot, settings, clock_running),
        ConfigPage::User => make_main_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Remotes(index, listening) => {
            make_remote_config_page(snapshot, settings, index, listening, mode, clock_running)
        }
        ConfigPage::Language => make_language_select_page(snapshot, settings, mode, clock_running),
    }
```

This is a temporary stub — Task 6 replaces it with the real builder.

- [ ] **Step 4: Patch any other exhaustive matches on `ConfigPage`.**

In `refbox/src/app/mod.rs`, search for `match … ConfigPage` and match `ConfigPage::User` identically to `ConfigPage::Main` where reasonable (both are navigation-only at this stage). Specifically expect a site inside the `ChangeConfigPage` handler around `refbox/src/app/mod.rs:1314-1328` if it branches per-page.

- [ ] **Step 5: Run `cargo check`.**

Run: `cargo check -p refbox --all-features`

Expected: PASS with zero warnings.

- [ ] **Step 6: Commit.**

Pause for user approval, then:
```bash
git add refbox/src/app/message.rs refbox/src/app/view_builders/configuration.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): add ConfigPage::User variant with placeholder routing"
```

---

## Phase 3 — Save model infrastructure

### Task 3: Split `apply_settings_change()` into per-page slice-apply functions

**Files:**

- Modify: `refbox/src/app/mod.rs:440-481`

Today, `apply_settings_change()` is one function that writes every editable slice to `self.config` and fires side effects (`self.update_sender.set_hide_time(...)` and `self.sound.update_settings(...)`). We split it into five slice-apply methods on `RefBoxApp`. Language already writes per-page today; it is not refactored here.

- [ ] **Step 1: Write the four new private methods, plus rewrite the existing one.**

Replace the current body of `apply_settings_change` with five private methods on `impl RefBoxApp`. Each reads the corresponding slice from `self.edited_settings` and writes it to `self.config`, firing only the side effects owned by that page.

```rust
fn apply_game_options(&mut self) {
    let Some(edited) = self.edited_settings.as_ref() else { return };
    self.config.game = edited.config.clone();
    // game_number lives outside config; applied by the caller context when
    // the user is in Game Options.
    if let Some(game_number) = edited.game_number_for_apply() {
        if let Ok(mut tm) = self.tm.lock() {
            tm.set_next_game_number(game_number);
        }
    }
}

fn apply_app_options(&mut self) {
    let Some(edited) = self.edited_settings.as_ref() else { return };
    self.config.using_uwhportal = edited.using_uwhportal;
    self.config.current_event_id = edited.current_event_id.clone();
    self.config.current_court = edited.current_court.clone();
    self.config.schedule = edited.schedule.clone();
    self.config.mode = edited.mode;
    self.config.collect_scorer_cap_num = edited.collect_scorer_cap_num;
    self.config.track_fouls_and_warnings = edited.track_fouls_and_warnings;
    self.config.confirm_score = edited.confirm_score;
}

fn apply_display_options(&mut self) {
    let Some(edited) = self.edited_settings.as_ref() else { return };
    self.config.white_on_right = edited.white_on_right;
    self.config.brightness = edited.brightness;
    self.config.hide_time = edited.hide_time;
    self.update_sender.set_hide_time(edited.hide_time);
}

fn apply_sound_options(&mut self) {
    let Some(edited) = self.edited_settings.as_ref() else { return };
    self.config.sound = edited.sound.clone();
    self.sound.update_settings(edited.sound.clone());
}

fn apply_remote_options(&mut self) {
    // Remotes are stored inside self.config.sound.remotes; Sound Options
    // and Manage Remotes both commit via self.sound.update_settings.
    let Some(edited) = self.edited_settings.as_ref() else { return };
    self.config.sound.remotes = edited.sound.remotes.clone();
    self.sound.update_settings(self.config.sound.clone());
}
```

If `game_number_for_apply` does not already exist, add it to `EditableSettings`:

```rust
impl EditableSettings {
    pub(in super::super) fn game_number_for_apply(&self) -> Option<uwh_common::uwhportal::schedule::GameNumber> {
        Some(self.game_number.clone())
    }
}
```

Remove the old `apply_settings_change` function body, or keep it as a thin wrapper that calls all five in order (useful only for the transitional `ConfigEditComplete { canceled: false }` path until every page has moved over). Recommended: keep `apply_settings_change` for one transitional commit so the old `ConfigEditComplete` path does not break, then delete it in Task 13.

- [ ] **Step 2: Persist config inside each slice-apply method.**

Confirm `apply_settings_change` today ends with `confy::store(APP_NAME, CONFIG_FILE, &self.config).unwrap()` or similar. Move that same call to the end of each of the five slice-apply methods so each page's Apply writes to disk.

If that store call lives in the caller, leave the caller alone for now and move the call in Task 5 when we wire the new message variants.

- [ ] **Step 3: `cargo check`.**

Run: `cargo check -p refbox --all-features`

Expected: PASS.

- [ ] **Step 4: Commit.**

Pause for approval, then:
```bash
git add refbox/src/app/mod.rs
git commit -m "refactor(refbox): split apply_settings_change into per-page slice functions"
```

### Task 4: Add `PageEntrySnapshot` storage and helper methods

**Files:**

- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Add the field to `RefBoxApp`.**

Find the `struct RefBoxApp { … }` declaration. Add:

```rust
page_entry_snapshot: Option<PageEntrySnapshot>,
```

Initialise it to `None` wherever `RefBoxApp` is constructed (typically in `new()` / `update()` boot paths).

- [ ] **Step 2: Add the enum.**

Paste the enum from the File Structure section at the top of the same file (below imports, above `impl RefBoxApp`).

- [ ] **Step 3: Add `capture_snapshot_for` and `revert_from_snapshot` helpers.**

```rust
impl RefBoxApp {
    fn capture_snapshot_for(&mut self, page: ConfigPage) {
        let Some(edited) = self.edited_settings.as_ref() else { return };
        let snapshot = match page {
            ConfigPage::Game => PageEntrySnapshot::Game {
                config: edited.config.clone(),
                game_number: edited.game_number.clone(),
            },
            ConfigPage::App => PageEntrySnapshot::App {
                using_uwhportal: edited.using_uwhportal,
                current_event_id: edited.current_event_id.clone(),
                current_court: edited.current_court.clone(),
                schedule: edited.schedule.clone(),
                mode: edited.mode,
                collect_scorer_cap_num: edited.collect_scorer_cap_num,
                track_fouls_and_warnings: edited.track_fouls_and_warnings,
                confirm_score: edited.confirm_score,
            },
            ConfigPage::Display => PageEntrySnapshot::Display {
                white_on_right: edited.white_on_right,
                brightness: edited.brightness,
                hide_time: edited.hide_time,
            },
            ConfigPage::Sound => PageEntrySnapshot::Sound {
                sound: edited.sound.clone(),
            },
            ConfigPage::Remotes(_, _) => PageEntrySnapshot::Remotes {
                remotes: edited.sound.remotes.clone(),
            },
            ConfigPage::Language => PageEntrySnapshot::Language {
                original_language: edited.original_language,
                pending_language: edited.pending_language,
            },
            ConfigPage::Main | ConfigPage::User => return, // navigation-only
        };
        self.page_entry_snapshot = Some(snapshot);
    }

    fn revert_from_snapshot(&mut self) {
        let (Some(edited), Some(snapshot)) =
            (self.edited_settings.as_mut(), self.page_entry_snapshot.take())
        else { return };
        match snapshot {
            PageEntrySnapshot::Game { config, game_number } => {
                edited.config = config;
                edited.game_number = game_number;
            }
            PageEntrySnapshot::App {
                using_uwhportal, current_event_id, current_court, schedule,
                mode, collect_scorer_cap_num, track_fouls_and_warnings, confirm_score,
            } => {
                edited.using_uwhportal = using_uwhportal;
                edited.current_event_id = current_event_id;
                edited.current_court = current_court;
                edited.schedule = schedule;
                edited.mode = mode;
                edited.collect_scorer_cap_num = collect_scorer_cap_num;
                edited.track_fouls_and_warnings = track_fouls_and_warnings;
                edited.confirm_score = confirm_score;
            }
            PageEntrySnapshot::Display { white_on_right, brightness, hide_time } => {
                edited.white_on_right = white_on_right;
                edited.brightness = brightness;
                edited.hide_time = hide_time;
            }
            PageEntrySnapshot::Sound { sound } => {
                edited.sound = sound;
            }
            PageEntrySnapshot::Remotes { remotes } => {
                edited.sound.remotes = remotes;
            }
            PageEntrySnapshot::Language { original_language, pending_language } => {
                edited.original_language = original_language;
                edited.pending_language = pending_language;
            }
        }
    }
}
```

- [ ] **Step 4: Add a pure `has_changes(page, edited, snapshot) -> bool` helper.**

Add this as a free function in `refbox/src/app/view_builders/configuration.rs` (it reads only `EditableSettings` and the snapshot; keeping it near the view builders lets them import it without touching `mod.rs`):

```rust
pub(in super::super) fn page_has_changes(
    page: ConfigPage,
    edited: &EditableSettings,
    snapshot: Option<&PageEntrySnapshot>,
) -> bool {
    let Some(snapshot) = snapshot else { return false };
    match (page, snapshot) {
        (ConfigPage::Game, PageEntrySnapshot::Game { config, game_number }) => {
            edited.config != *config || edited.game_number != *game_number
        }
        (ConfigPage::App, PageEntrySnapshot::App {
            using_uwhportal, current_event_id, current_court, schedule,
            mode, collect_scorer_cap_num, track_fouls_and_warnings, confirm_score,
        }) => {
            edited.using_uwhportal != *using_uwhportal
                || edited.current_event_id != *current_event_id
                || edited.current_court != *current_court
                || edited.schedule != *schedule
                || edited.mode != *mode
                || edited.collect_scorer_cap_num != *collect_scorer_cap_num
                || edited.track_fouls_and_warnings != *track_fouls_and_warnings
                || edited.confirm_score != *confirm_score
        }
        (ConfigPage::Display, PageEntrySnapshot::Display { white_on_right, brightness, hide_time }) => {
            edited.white_on_right != *white_on_right
                || edited.brightness != *brightness
                || edited.hide_time != *hide_time
        }
        (ConfigPage::Sound, PageEntrySnapshot::Sound { sound }) => {
            edited.sound != *sound
        }
        (ConfigPage::Remotes(_, _), PageEntrySnapshot::Remotes { remotes }) => {
            edited.sound.remotes != *remotes
        }
        (ConfigPage::Language, PageEntrySnapshot::Language { original_language, pending_language }) => {
            edited.original_language != *original_language || edited.pending_language != *pending_language
        }
        _ => false,
    }
}
```

`PageEntrySnapshot` must be re-exported into the `view_builders::configuration` scope — add `pub(in super::super)` to the enum in `mod.rs` or re-export via a `use` in `configuration.rs`.

- [ ] **Step 5: Add a unit test for `page_has_changes`.**

At the bottom of `configuration.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PageEntrySnapshot;
    use matrix_drawing::transmitted_data::Brightness;

    #[test]
    fn display_no_changes_when_buffer_equals_snapshot() {
        let edited = EditableSettings {
            white_on_right: false,
            brightness: Brightness::Medium,
            hide_time: false,
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Display {
            white_on_right: false,
            brightness: Brightness::Medium,
            hide_time: false,
        };
        assert!(!page_has_changes(ConfigPage::Display, &edited, Some(&snap)));
    }

    #[test]
    fn display_detects_brightness_change() {
        let edited = EditableSettings {
            white_on_right: false,
            brightness: Brightness::High,
            hide_time: false,
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Display {
            white_on_right: false,
            brightness: Brightness::Medium,
            hide_time: false,
        };
        assert!(page_has_changes(ConfigPage::Display, &edited, Some(&snap)));
    }

    #[test]
    fn page_without_snapshot_reports_no_changes() {
        let edited = EditableSettings::default();
        assert!(!page_has_changes(ConfigPage::Display, &edited, None));
    }
}
```

- [ ] **Step 6: Run tests and check.**

Run: `cargo test -p refbox --lib configuration::tests`

Expected: 3 tests PASS.

Run: `just check`

Expected: PASS.

- [ ] **Step 7: Commit.**

Pause for approval, then:
```bash
git add refbox/src/app/mod.rs refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): add per-page entry snapshot and change detection"
```

### Task 5: Add per-page Apply/Cancel message variants

**Files:**

- Modify: `refbox/src/app/message.rs`
- Modify: `refbox/src/app/mod.rs` (add handlers; wire existing ChangeConfigPage to capture snapshot)

- [ ] **Step 1: Add the two new message variants.**

In the `Message` enum (search for `ConfigEditComplete`), add near that variant:

```rust
ApplyConfigPage(ConfigPage),
CancelConfigPage(ConfigPage),
```

- [ ] **Step 2: Add update handlers.**

In `update()`, add:

```rust
Message::ApplyConfigPage(page) => {
    match page {
        ConfigPage::Game => self.apply_game_options(),
        ConfigPage::App => self.apply_app_options(),
        ConfigPage::Display => self.apply_display_options(),
        ConfigPage::Sound => self.apply_sound_options(),
        ConfigPage::Remotes(_, _) => self.apply_remote_options(),
        ConfigPage::Language | ConfigPage::Main | ConfigPage::User => {
            // Language uses its own LanguageSelectComplete path; Main and
            // User are navigation-only and should never receive Apply.
            return Task::none();
        }
    }
    self.page_entry_snapshot = None;
    self.persist_config();
    self.navigate_to_parent(page);
    Task::none()
}
Message::CancelConfigPage(page) => {
    self.revert_from_snapshot();
    self.navigate_to_parent(page);
    Task::none()
}
```

Add `persist_config` and `navigate_to_parent` helpers:

```rust
impl RefBoxApp {
    fn persist_config(&self) {
        // Match whatever the existing ConfigEditComplete handler uses today
        // to write self.config to disk. If that logic is inline, extract it.
        let _ = confy::store(APP_NAME, CONFIG_FILE, &self.config);
    }

    fn navigate_to_parent(&mut self, page: ConfigPage) {
        let parent = match page {
            ConfigPage::Game | ConfigPage::App | ConfigPage::User | ConfigPage::Language => ConfigPage::Main,
            ConfigPage::Display | ConfigPage::Sound => ConfigPage::User,
            ConfigPage::Remotes(_, _) => ConfigPage::Sound,
            ConfigPage::Main => ConfigPage::Main, // no-op
        };
        self.app_state = AppState::EditGameConfig(parent);
    }
}
```

The exact `APP_NAME` / `CONFIG_FILE` constants should be copied from wherever `ConfigEditComplete` calls `confy::store` today. If the existing call has surrounding logic (e.g. error handling, restart-on-font-change), mirror it — do not simplify.

- [ ] **Step 3: Capture snapshot when a page is entered.**

In the `Message::ChangeConfigPage(page)` handler at `refbox/src/app/mod.rs:1314-1328`, after the existing body, add:

```rust
self.capture_snapshot_for(page);
```

This replaces the Language-specific `original_language` bookkeeping with a unified approach. The Language special-case can stay for now (defensive); Task 12 removes it.

- [ ] **Step 4: `just check`.**

Run: `just check`

Expected: PASS.

- [ ] **Step 5: Commit.**

Pause for approval, then:
```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): add per-page Apply/Cancel messages and handlers"
```

---

## Phase 4 — Page-by-page chrome rework

Each task in this phase:

1. Reworks one page's button footer to `Cancel + Apply`.
2. Wires the footer buttons to the new per-page messages.
3. Disables `Apply` when `page_has_changes(page, edited, snapshot)` is false.

### Task 6: Build the User Options grouping page

**Files:**

- Modify: `refbox/src/app/view_builders/configuration.rs` — add `make_user_config_page`; update dispatcher.

- [ ] **Step 1: Add the back-button helper.**

Near the other footer helpers in `configuration.rs`, add:

```rust
fn make_back_button<'a>(destination: Message) -> Element<'a, Message> {
    button(text(fl!("back")).size(LARGE_TEXT).align_x(Horizontal::Center))
        .width(Length::Fill)
        .height(MIN_BUTTON_SIZE)
        .style(button::secondary)
        .on_press(destination)
        .into()
}
```

Main uses `make_back_button(Message::ConfigEditComplete { canceled: true })` to exit settings; User Options uses `make_back_button(Message::ChangeConfigPage(ConfigPage::Main))` to return to the grid.

- [ ] **Step 2: Add the User Options builder.**

Below the Main page builder, add:

```rust
fn make_user_config_page<'a>(
    snapshot: &GameSnapshot,
    _settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    // Three tiles: Display | View Mode (hidden spacer per ADR 010) | Sound.
    let display_btn = button(
        text(fl!("display-options"))
            .size(LARGE_TEXT)
            .align_x(Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::FillPortion(2))
    .style(button::primary)
    .on_press(Message::ChangeConfigPage(ConfigPage::Display));

    // Hidden spacer — View Mode slot per ADR 010.
    let view_mode_spacer = horizontal_space().width(Length::Fill);

    let sound_btn = button(
        text(fl!("sound"))
            .size(LARGE_TEXT)
            .align_x(Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::FillPortion(2))
    .style(button::primary)
    .on_press(Message::ChangeConfigPage(ConfigPage::Sound));

    let tiles = row![display_btn, view_mode_spacer, sound_btn]
        .spacing(SPACING)
        .height(Length::Fill);

    column![
        make_game_time_button(snapshot, false, mode, clock_running),
        tiles,
        make_back_button(Message::ChangeConfigPage(ConfigPage::Main)),
    ]
    .spacing(SPACING)
    .into()
}
```

Translation keys `display-options` and `sound` — verify they exist in `en-US/refbox.ftl` before assuming. If either is missing, add it now to all 15 locales (mirror Task 1's pattern).

- [ ] **Step 3: Update the dispatcher.**

In `build_game_config_edit_page` (lines 110-134), replace the placeholder `ConfigPage::User` arm with:

```rust
ConfigPage::User => make_user_config_page(snapshot, settings, mode, clock_running),
```

- [ ] **Step 4: `cargo check`.**

Run: `cargo check -p refbox --all-features`

Expected: PASS.

- [ ] **Step 5: Commit.**

Pause for approval, then:
```bash
git add refbox/src/app/view_builders/configuration.rs refbox/translations/
git commit -m "feat(refbox): add User Options page"
```

### Task 7: Move game-number picker to Game Options, then rebuild Main as a 2×2 grid

**Files:**

- Modify: `refbox/src/app/view_builders/configuration.rs:136-239` (Main) and the top of `make_event_config_page` (around line 242).

**Ordering note:** The picker must land on Game Options *before* Main is rebuilt. Both changes ship in one commit so the dev-app is never in a state without the picker visible somewhere.

- [ ] **Step 1: Copy the game-number picker into Game Options.**

Locate the game-number picker in `make_main_config_page` — it reads `settings.game_number` and emits `Message::GameNumberUpdated(i64)` (or similar — grep `game_number` within the file to find the exact widget row). Move that widget row (label + `-` / `+` buttons) into `make_event_config_page`, placing it as the first editable row above the existing game-parameter fields.

Keep the picker logic identical — only its container changes. After this step, the picker exists on both Main and Game Options; the next step removes it from Main.

- [ ] **Step 2: Rebuild Main as a 2×2 grid.**

Replace the body of `make_main_config_page` with:

```rust
fn make_main_config_page<'a>(
    snapshot: &GameSnapshot,
    _settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let tile = |label: &'static str, dest: ConfigPage| -> Element<'a, Message> {
        button(text(fl!(label)).size(LARGE_TEXT).align_x(Horizontal::Center))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(button::primary)
            .on_press(Message::ChangeConfigPage(dest))
            .into()
    };

    let row_top = row![
        tile("game-options", ConfigPage::Game),
        tile("app-options", ConfigPage::App),
    ]
    .spacing(SPACING)
    .height(Length::FillPortion(1));

    let row_bottom = row![
        tile("user-options", ConfigPage::User),
        tile("language", ConfigPage::Language),
    ]
    .spacing(SPACING)
    .height(Length::FillPortion(1));

    column![
        make_game_time_button(snapshot, false, mode, clock_running),
        row_top,
        row_bottom,
        make_back_button(Message::ConfigEditComplete { canceled: true }),
    ]
    .spacing(SPACING)
    .into()
}
```

After this step, the picker is only on Game Options (where it belongs per ADR 009). The game_number update message continues to write `edited_settings.game_number` as before.

- [ ] **Step 3: Confirm translation keys exist.**

Grep the English locale for `game-options`, `app-options`, `language`. If any are missing, add them in all 15 locales (mirror the Task 1 translation table pattern).

- [ ] **Step 4: `cargo check` + `just lint`.**

Run: `just check`

Expected: PASS.

- [ ] **Step 5: Manual sanity check.**

Pause — ask the user to launch the dev app (`just run` or the Wayland-sandbox incantation) and confirm:
- Main settings shows a 2×2 grid.
- All four tiles navigate somewhere sensible (Game and App to working pages, User to the new page, Language to the language list).
- Game Options now shows the game-number picker; incrementing and decrementing work as before.
- Back on Main exits settings.

- [ ] **Step 6: Commit.**

Pause for approval, then:
```bash
git add refbox/src/app/view_builders/configuration.rs refbox/translations/
git commit -m "feat(refbox): move game-number picker to Game Options, restructure Main as 2x2 grid"
```

### Task 8: Add Cancel/Apply chrome to Game Options

**Files:**

- Modify: `refbox/src/app/view_builders/configuration.rs` — `make_event_config_page` (around line 242).
- Modify: `refbox/src/app/mod.rs` — thread `page_entry_snapshot` into the dispatcher.

- [ ] **Step 1: Add a Cancel/Apply footer helper.**

```rust
fn make_cancel_apply_footer<'a>(
    page: ConfigPage,
    edited: &EditableSettings,
    snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    let apply_enabled = page_has_changes(page, edited, snapshot);

    let cancel = button(text(fl!("cancel")).size(LARGE_TEXT).align_x(Horizontal::Center))
        .width(Length::Fill)
        .height(MIN_BUTTON_SIZE)
        .style(button::secondary)
        .on_press(Message::CancelConfigPage(page));

    let mut apply = button(text(fl!("apply")).size(LARGE_TEXT).align_x(Horizontal::Center))
        .width(Length::Fill)
        .height(MIN_BUTTON_SIZE)
        .style(button::primary);
    if apply_enabled {
        apply = apply.on_press(Message::ApplyConfigPage(page));
    }

    row![cancel, apply].spacing(SPACING).into()
}
```

Note that `make_event_config_page`, `make_app_config_page`, `make_display_config_page`, `make_sound_config_page`, `make_remote_config_page` must now receive `snapshot: Option<&PageEntrySnapshot>` as an argument. Update their signatures and the dispatcher in the same commit.

- [ ] **Step 2: Thread `snapshot` through the dispatcher.**

Update `build_game_config_edit_page` to receive and pass `page_entry_snapshot: Option<&PageEntrySnapshot>` to each editing page builder. The caller in `view()` (`refbox/src/app/mod.rs`) passes `self.page_entry_snapshot.as_ref()`.

- [ ] **Step 3: Replace the existing Done/Cancel footer in `make_event_config_page`.**

Replace the page's final `column![…]` footer with `make_cancel_apply_footer(ConfigPage::Game, settings, snapshot)`.

- [ ] **Step 4: `just check`.**

Run: `just check`

Expected: PASS.

- [ ] **Step 5: Manual test.**

Launch the dev app. On Game Options: change the half length, verify Apply becomes enabled; Cancel reverts; Apply commits. Verify the game-number picker works as before.

- [ ] **Step 6: Commit.**

```bash
git add refbox/src/app/view_builders/configuration.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): Game Options gains Cancel/Apply chrome"
```

### Task 9: App Options — remove inner Language button + Cancel/Apply chrome

**Files:**

- Modify: `refbox/src/app/view_builders/configuration.rs` — `make_app_config_page` (line 537+), specifically lines 591-596 for the inner Language button.

- [ ] **Step 1: Remove the inner Language button.**

Delete the widget at lines 591-596 that routes to `ConfigPage::Language`. Language is reached from the Main grid now.

- [ ] **Step 2: Replace the footer.**

Replace whatever bottom-row footer exists with `make_cancel_apply_footer(ConfigPage::App, settings, snapshot)`.

- [ ] **Step 3: `just check`.**

Expected: PASS.

- [ ] **Step 4: Manual test.**

Launch dev app. App Options: toggle `collect_scorer_cap_num`. Apply enables; Cancel reverts; Apply commits.

- [ ] **Step 5: Commit.**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): App Options drops inner Language button, adds Cancel/Apply"
```

### Task 10: Display Options — Cancel/Apply chrome

**Files:**

- Modify: `refbox/src/app/view_builders/configuration.rs` — `make_display_config_page` (line 618+).

- [ ] **Step 1: Replace the footer with `make_cancel_apply_footer(ConfigPage::Display, settings, snapshot)`.**
- [ ] **Step 2: `just check`** — Expected: PASS.
- [ ] **Step 3: Manual test** — toggle `white_on_right`; Apply enables; Cancel reverts; Apply commits and still fires `update_sender.set_hide_time(...)` / side-effects.
- [ ] **Step 4: Commit.**
```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): Display Options carries Cancel/Apply chrome"
```

### Task 11: Sound Options — Cancel/Apply chrome

**Files:**

- Modify: `refbox/src/app/view_builders/configuration.rs` — `make_sound_config_page` (line 699+). The `manage-remotes` button at line 728 stays put.

- [ ] **Step 1: Replace the footer with `make_cancel_apply_footer(ConfigPage::Sound, settings, snapshot)`.**
- [ ] **Step 2: `just check`** — Expected: PASS.
- [ ] **Step 3: Manual test** — change a volume cycle. Apply enables; Cancel reverts the volume (no live preview — that is ADR 014); Apply commits and pushes sound settings to the controller.
- [ ] **Step 4: Commit.**
```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): Sound Options carries Cancel/Apply chrome"
```

### Task 12: Manage Remotes — Cancel/Apply chrome + hardware push

**Files:**

- Modify: `refbox/src/app/view_builders/configuration.rs` — `make_remote_config_page` (line 838+).

- [ ] **Step 1: Replace the footer with `make_cancel_apply_footer(ConfigPage::Remotes(index, listening), settings, snapshot)`.**

Note the payload: the per-page snapshot uses `ConfigPage::Remotes(_, _)` for the match; the message carries the current `(index, listening)` so the handler returns to the same place after navigation if needed. In practice Remotes Apply returns to Sound Options via `navigate_to_parent`.

- [ ] **Step 2: Confirm `apply_remote_options` pushes to the sound controller.**

Verify Task 3's `apply_remote_options` calls `self.sound.update_settings(self.config.sound.clone())`. Without this, the LoRa radio link to the physical remote does not reconfigure.

- [ ] **Step 3: `just check`** — Expected: PASS.
- [ ] **Step 4: Manual test** — add/remove a remote entry; Apply enables; Cancel reverts the list to its page-entry state; Apply commits and re-pushes sound settings (including the remotes slice) to the sound controller.
- [ ] **Step 5: Commit.**
```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): Manage Remotes carries Cancel/Apply chrome"
```

### Task 13: Language screen — align chrome + retire global `apply_settings_change`

**Files:**

- Modify: `refbox/src/app/view_builders/configuration.rs` — `make_language_select_page` (line 1017+).
- Modify: `refbox/src/app/mod.rs` — Language message path; delete the transitional `apply_settings_change` wrapper.

- [ ] **Step 1: Ensure Language's footer uses the same Cancel/Apply pattern.**

Today Language uses its own `LanguageSelectComplete { canceled }` message. Decide the minimal-churn path: keep `LanguageSelectComplete` as the internal mechanism but make the view render the standard `make_cancel_apply_footer`, translating button presses to `LanguageSelectComplete { canceled: false }` on Apply and `LanguageSelectComplete { canceled: true }` on Cancel. Alternative: route through `ApplyConfigPage(ConfigPage::Language)` / `CancelConfigPage(ConfigPage::Language)` and handle them specially (they must still call `confy::store`, check font-family change, handle restart message).

Recommendation: **keep `LanguageSelectComplete` as-is, adapt the view layout only.** Language has special restart-on-font-family semantics that should stay in one handler.

```rust
fn make_language_select_footer<'a>(
    edited: &EditableSettings,
    snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    let apply_enabled = page_has_changes(ConfigPage::Language, edited, snapshot);

    let cancel = button(text(fl!("cancel")).size(LARGE_TEXT).align_x(Horizontal::Center))
        .width(Length::Fill)
        .height(MIN_BUTTON_SIZE)
        .style(button::secondary)
        .on_press(Message::LanguageSelectComplete { canceled: true });

    let mut apply = button(text(fl!("apply")).size(LARGE_TEXT).align_x(Horizontal::Center))
        .width(Length::Fill)
        .height(MIN_BUTTON_SIZE)
        .style(button::primary);
    if apply_enabled {
        apply = apply.on_press(Message::LanguageSelectComplete { canceled: false });
    }

    row![cancel, apply].spacing(SPACING).into()
}
```

Replace whatever Language's current footer is with this helper.

- [ ] **Step 2: Delete `apply_settings_change` and its caller.**

The `ConfigEditComplete { canceled: false }` path in `update()` (around line 1330+) used to validate uwhportal completeness and route to ConfirmationPage. That validation logic still matters — it gates exit from settings when the portal session is mid-configuration. Keep the validation + ConfirmationPage routing; remove only the call to `apply_settings_change()`. Each page has already persisted itself by this point.

If any required per-page commit was previously only happening via the global path (e.g., a field that no page owns), escalate — do not silently drop it.

- [ ] **Step 3: `just check`** — Expected: PASS.
- [ ] **Step 4: Manual test** — select a different language; Apply enables; Cancel reverts the selection; Apply persists and, if the font family changed, surfaces the restart prompt as before.
- [ ] **Step 5: Commit.**
```bash
git add refbox/src/app/view_builders/configuration.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): align Language chrome and retire global settings commit"
```

---

## Phase 5 — Validation

### Task 14: Full regression walkthrough

**Files:** none

- [ ] **Step 1: Clean rebuild and run `just check`.**

Run: `cargo clean -p refbox && just check`

Expected: PASS across fmt, lint, tests, audit.

- [ ] **Step 2: Manual walkthrough — match every ADR 009 chrome row.**

| Page | Back visible? | Cancel visible? | Apply visible? | Apply disabled until change? |
|------|:-:|:-:|:-:|:-:|
| Main settings | yes | no | no | n/a |
| Game Options | no | yes | yes | yes |
| App Options | no | yes | yes | yes |
| User Options | yes | no | no | n/a |
| Display Options | no | yes | yes | yes |
| Sound Options | no | yes | yes | yes |
| Manage Remotes | no | yes | yes | yes |
| Language | no | yes | yes | yes |

Walk every row. For rows with Apply: enter the page, confirm Apply is disabled; make one change, confirm Apply enables; press Cancel, confirm the change reverts and navigation returns to the parent; re-enter, make one change, press Apply, confirm the change persists (restart the app, confirm it survives).

- [ ] **Step 3: Regression check — non-UI behaviour.**

- Start a game (simulator mode). Enter settings mid-game. Verify the uwhportal-mid-session confirmation still fires where it did before.
- Trigger a hardware side effect: toggle `hide_time`, Apply on Display Options, confirm `update_sender.set_hide_time(...)` fires.
- Toggle a sound volume, Apply on Sound Options, confirm the sound controller was updated.

- [ ] **Step 4: Document the manual test in the PR description.**

Draft the PR body (template in `.claude/rules/pr-review.md`):

- **What changed:** one paragraph in plain English.
- **Why:** reference ADR 009.
- **Scope:** `refbox` crate only.
- **How to verify:** the walkthrough from Step 2.

Do not open the PR without approval.

- [ ] **Step 5: Final commit (if any cleanup).**

Any remaining formatting/lint fixes:
```bash
git add -p
git commit -m "chore(refbox): post-implementation cleanup for ADR 009"
```

---

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

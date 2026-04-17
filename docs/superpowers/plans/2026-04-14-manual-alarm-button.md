# Manual Alarm Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in on-screen alarm button and spacebar shortcut that lets the referee operator manually trigger the buzzer during live play, with a hold-to-test mode during the Between Games period.

**Architecture:** A new boolean setting (`manual_alarm_enabled`) gates the entire feature. When on, the main screen's center column replaces the game info area with a compact "GAME INFO" button plus an alarm zone (split with the warnings panel when fouls/warnings tracking is active).

A **uniform hold model** applies: every press schedules a fire timer (250ms during active play with no timeout, 1 second everywhere else). When the timer fires, the alarm starts in a continuous mode via `SoundController::start_manual_buzzer()`. Release stops further queueing via `stop_manual_buzzer()`, letting the currently playing tone finish its natural cycle. A generation counter on `RefBoxApp` invalidates pending timers on early release without needing task cancellation.

The button visual state depends on the game state: red with "Alarm / Or press Spacebar" during active play (no timeout), blue with "Hold to Test / Or hold Spacebar" in every other state. The button is never greyed out while the feature is enabled.

Spacebar events are added to the existing subscription.

**Tech Stack:** Rust 2024, iced 0.13 (Elm-like UI), tokio async runtime, Fluent (`.ftl`) translations

---

## File Map

| Action | File | What changes |
|--------|------|-------------|
| Modify | `refbox/src/sound_controller/mod.rs` | Add `manual_alarm_enabled` field and migration |
| Modify | `refbox/src/app/message.rs` | Add `AlarmPressed`, `AlarmReleased`, `AlarmFired(u32)`, `BoolGameParameter::ManualAlarmEnabled` |
| Verify | `refbox/translations/*/refbox.ftl` | All required keys (`alarm-button`, `alarm`, `hold-to-test`, `or-press-spacebar`, `or-hold-spacebar`, `game-info`) already exist across all 14 languages — no changes needed |
| Modify | `refbox/src/app/view_builders/configuration.rs` | Add Alarm Button toggle row to sound settings page |
| Modify | `refbox/src/app/mod.rs` | Add `alarm_hold_generation`, handle new messages, extend subscription, pass new param to `build_main_view` |
| Modify | `refbox/src/app/view_builders/main_view.rs` | New layout with GAME INFO button and state-dependent alarm button (red/blue) |

---

### Task 1: Add `manual_alarm_enabled` to `SoundSettings`

**Files:**
- Modify: `refbox/src/sound_controller/mod.rs`

- [ ] **Step 1: Add the field to `SoundSettings`**

In `refbox/src/sound_controller/mod.rs`, add the new field to the `SoundSettings` struct. It goes after `auto_sound_stop_play` and before `remotes`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SoundSettings {
    #[derivative(Default(value = "true"))]
    pub sound_enabled: bool,
    #[derivative(Default(value = "true"))]
    pub whistle_enabled: bool,
    pub buzzer_sound: BuzzerSound,
    #[derivative(Default(value = "Volume::Medium"))]
    pub whistle_vol: Volume,
    pub above_water_vol: Volume,
    pub under_water_vol: Volume,
    #[derivative(Default(value = "true"))]
    pub auto_sound_start_play: bool,
    #[derivative(Default(value = "true"))]
    pub auto_sound_stop_play: bool,
    pub manual_alarm_enabled: bool,   // <-- new field (defaults false)
    pub remotes: Vec<RemoteInfo>,
}
```

- [ ] **Step 2: Add migration support**

In the `SoundSettings::migrate()` method, add handling for the new field after the `auto_sound_stop_play` block. Follow the exact same pattern as the existing fields:

```rust
// Add this block after the auto_sound_stop_play block:
if let Some(old_manual_alarm_enabled) = old.get("manual_alarm_enabled") {
    if let Some(old_manual_alarm_enabled) = old_manual_alarm_enabled.as_bool() {
        manual_alarm_enabled = old_manual_alarm_enabled;
    }
}
```

Also add `mut manual_alarm_enabled` to the destructuring at the top of `migrate()`:

```rust
let Self {
    mut sound_enabled,
    mut whistle_enabled,
    mut buzzer_sound,
    mut whistle_vol,
    mut above_water_vol,
    mut under_water_vol,
    mut auto_sound_start_play,
    mut auto_sound_stop_play,
    mut manual_alarm_enabled,   // <-- add this
    mut remotes,
} = Default::default();
```

And include it in the final `Self { ... }` construction:

```rust
Self {
    sound_enabled,
    whistle_enabled,
    buzzer_sound,
    whistle_vol,
    above_water_vol,
    under_water_vol,
    auto_sound_start_play,
    auto_sound_stop_play,
    manual_alarm_enabled,   // <-- add this
    remotes,
}
```

- [ ] **Step 3: Update the existing serialization test**

In the `#[cfg(test)]` block at the bottom of `refbox/src/sound_controller/mod.rs`, the `test_ser_sound_settings` test exercises the default `SoundSettings`. It will still pass because the new field defaults to `false` and round-trips correctly — but run it now to confirm:

```bash
cargo test -p refbox test_ser_sound_settings -- --nocapture
```

Expected: `test test_ser_sound_settings ... ok`

- [ ] **Step 4: Update the migration test**

The `test_migrate_sound_settings` test in the same file does not include the new field in the `old` table, which is correct (it tests migration from an old config without the field). Verify it still passes:

```bash
cargo test -p refbox test_migrate_sound_settings -- --nocapture
```

Expected: `test test_migrate_sound_settings ... ok`

- [ ] **Step 5: Commit**

```bash
git add refbox/src/sound_controller/mod.rs
git commit -m "feat(refbox): add manual_alarm_enabled field to SoundSettings"
```

---

### Task 2: Add new `Message` variants

**Files:**
- Modify: `refbox/src/app/message.rs`

- [ ] **Step 1: Add the three new `Message` variants**

In `refbox/src/app/message.rs`, add to the `Message` enum after `StartClock`:

```rust
AlarmPressed,
AlarmReleased,
AlarmFired(u32),
```

- [ ] **Step 2: Add `ManualAlarmEnabled` to `BoolGameParameter`**

In the same file, add to the `BoolGameParameter` enum after `ConfirmScore`:

```rust
ManualAlarmEnabled,
```

- [ ] **Step 3: Update `is_repeatable()`**

`AlarmPressed`, `AlarmReleased`, and `AlarmFired` are not repeatable. Add them to the `false` arm of `is_repeatable()` alongside `StopClock` and `StartClock`:

```rust
| Self::StopClock
| Self::StartClock
| Self::AlarmPressed
| Self::AlarmReleased
| Self::AlarmFired(_) => false,
```

- [ ] **Step 4: Update `PartialEq`**

In the hand-written `PartialEq` impl, add equality cases. In the `true` arm of the match, add (alongside `StopClock` and `StartClock`):

```rust
(Self::AlarmPressed, Self::AlarmPressed)
| (Self::AlarmReleased, Self::AlarmReleased) => true,
```

Add a parametric case for `AlarmFired` (put it alongside the other parametric `true` cases such as `ChangeScore`):

```rust
(Self::AlarmFired(a), Self::AlarmFired(b)) => a == b,
```

Add the three variants to the catch-all `false` arm at the bottom (alongside `StartClock`):

```rust
| (Self::AlarmPressed, _)
| (Self::AlarmReleased, _)
| (Self::AlarmFired(_), _) => false,
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p refbox
```

Expected: no errors. The new variants will produce `non-exhaustive patterns` warnings in the `update()` function — that is expected and will be resolved in Task 5.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/message.rs
git commit -m "feat(refbox): add AlarmPressed/Released/Fired messages and ManualAlarmEnabled parameter"
```

---

### Task 3: Verify translation strings already exist

All six translation keys used by this feature (`alarm-button`, `alarm`, `hold-to-test`, `or-press-spacebar`, `or-hold-spacebar`, `game-info`) are already present in every supported language file. No edits are needed — just a verification pass.

- [ ] **Step 1: Confirm all keys exist in all languages**

```bash
for lang in refbox/translations/*/refbox.ftl; do
  for key in alarm-button alarm hold-to-test or-press-spacebar or-hold-spacebar game-info; do
    grep -q "^${key}\s*=" "$lang" || echo "MISSING: $key in $lang"
  done
done
```

Expected: no output (every key is present in every file). If any key is reported missing, add it before proceeding.

- [ ] **Step 2: No commit needed for this task**

---

### Task 4: Add the Alarm Button toggle to the Sound settings page

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`

- [ ] **Step 1: Add a new row to `make_sound_config_page`**

In `refbox/src/app/view_builders/configuration.rs`, find the `make_sound_config_page` function. After the third `row![ ... ].spacing(SPACING),` block (the one with `buzzer-sound`, `underwater-volume`, `auto-sound-stop-play`), add a new row and a divider before the remotes section. The existing remotes section starts with `make_scroll_list(...)` or similar — add the new row just before it:

```rust
row![
    make_value_button(
        fl!("alarm-button"),
        bool_string(sound.manual_alarm_enabled),
        (false, true),
        if sound.sound_enabled {
            Some(Message::ToggleBoolParameter(
                BoolGameParameter::ManualAlarmEnabled,
            ))
        } else {
            None
        },
    ),
]
.spacing(SPACING),
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check -p refbox
```

Expected: `ManualAlarmEnabled` will still produce a non-exhaustive warning in `update()` — that is resolved in the next task.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): add Alarm Button toggle to sound settings page"
```

---

### Task 5: Handle new messages in `app/mod.rs`

**Files:**
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Add `alarm_hold_generation` to `RefBoxApp`**

In the `RefBoxApp` struct definition (around line 55), add the new field after `list_all_events`:

```rust
pub struct RefBoxApp {
    tm: Arc<Mutex<TournamentManager>>,
    config: Config,
    edited_settings: Option<EditableSettings>,
    snapshot: GameSnapshot,
    pen_edit: ListEditor<Penalty, Color>,
    warn_edit: ListEditor<InfractionDetails, Color>,
    foul_edit: ListEditor<InfractionDetails, Option<Color>>,
    app_state: AppState,
    last_app_state: AppState,
    last_message: Message,
    update_sender: UpdateSender,
    uwhportal_client: Option<UwhPortalClient>,
    using_uwhportal: bool,
    events: Option<BTreeMap<EventId, Event>>,
    schedule: Option<Schedule>,
    current_event_id: Option<EventId>,
    current_court: Option<String>,
    sound: SoundController,
    sim_child: Option<Child>,
    list_all_events: bool,
    alarm_hold_generation: u32,   // <-- new field
}
```

- [ ] **Step 2: Initialize the new field**

Find where `RefBoxApp` is constructed (the `new()` or equivalent function — search for `list_all_events:` to find the construction site). Add:

```rust
alarm_hold_generation: 0,
```

- [ ] **Step 3: Handle `BoolGameParameter::ManualAlarmEnabled` in `update()`**

Find the `ToggleBoolParameter` match arm in `update()` (around line 1640). Add the new case after `ConfirmScore`:

```rust
BoolGameParameter::ManualAlarmEnabled => {
    edited_settings.sound.manual_alarm_enabled ^= true
}
```

- [ ] **Step 4: Handle `AlarmPressed`, `AlarmReleased`, and `AlarmFired`**

Under the uniform hold model: every press schedules a timer (250ms for active play with no timeout, 1 second everywhere else). When the timer fires, start the continuous manual buzzer. On release, invalidate any pending timer and stop the buzzer (which lets the currently playing tone finish its natural cycle before silence).

Add handling for the three new message variants alongside `StopClock` and `StartClock` near the end of the message match:

```rust
Message::AlarmPressed => {
    if self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled {
        self.alarm_hold_generation = self.alarm_hold_generation.wrapping_add(1);
        let gen = self.alarm_hold_generation;
        let hold_duration = match (self.snapshot.current_period, self.snapshot.timeout) {
            (
                GamePeriod::FirstHalf
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::SuddenDeath,
                None,
            ) => Duration::from_millis(250),
            _ => Duration::from_secs(1),
        };
        return Task::future(async move {
            tokio::time::sleep(hold_duration).await;
            Message::AlarmFired(gen)
        });
    }
    Task::none()
}
Message::AlarmFired(gen) => {
    if gen == self.alarm_hold_generation
        && self.config.sound.sound_enabled
        && self.config.sound.manual_alarm_enabled
    {
        self.sound.start_manual_buzzer();
    }
    Task::none()
}
Message::AlarmReleased => {
    self.alarm_hold_generation = self.alarm_hold_generation.wrapping_add(1);
    self.sound.stop_manual_buzzer();
    Task::none()
}
```

Notes:
- `start_manual_buzzer()` and `stop_manual_buzzer()` already exist in `SoundController`. `start_manual_buzzer()` pushes `ManualAlarm` to the front of the sound queue (continuous playback); `stop_manual_buzzer()` removes it from the queue, allowing the currently playing tone to finish naturally.
- `stop_manual_buzzer()` is idempotent — it's safe to call on every `AlarmReleased` even if no alarm is playing.
- No state check in `AlarmFired` beyond the generation counter and the feature flags. If the game state changes between press and fire, the operator's expressed intent (the hold) is still honoured.

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p refbox
```

Expected: no errors or warnings.

- [ ] **Step 6: Run tests**

```bash
just test
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): handle alarm messages and ManualAlarmEnabled toggle in app update"
```

---

### Task 6: Extend `subscription()` for spacebar events

**Files:**
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Add keyboard imports**

At the top of `refbox/src/app/mod.rs`, the existing iced import is:

```rust
use iced::{Element, Subscription, Task, Theme, application::Appearance, widget::column, window};
```

Extend it to include keyboard support:

```rust
use iced::{
    Element, Subscription, Task, Theme,
    application::Appearance,
    keyboard::{self, Key, key::Named},
    widget::column,
    window,
};
```

- [ ] **Step 2: Replace `subscription()`**

Find the existing `subscription()` method (around line 2251):

```rust
pub(super) fn subscription(&self) -> Subscription<Message> {
    Subscription::run(time_updater)
}
```

Replace it with:

```rust
pub(super) fn subscription(&self) -> Subscription<Message> {
    let time_sub = Subscription::run(time_updater);

    if self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled {
        let key_press = keyboard::on_key_press(|key, _modifiers| {
            if matches!(key, Key::Named(Named::Space)) {
                Some(Message::AlarmPressed)
            } else {
                None
            }
        });
        let key_release = keyboard::on_key_release(|key, _modifiers| {
            if matches!(key, Key::Named(Named::Space)) {
                Some(Message::AlarmReleased)
            } else {
                None
            }
        });
        Subscription::batch([time_sub, key_press, key_release])
    } else {
        time_sub
    }
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p refbox
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): subscribe to spacebar events for manual alarm when enabled"
```

---

### Task 7: Update the main view layout

**Files:**
- Modify: `refbox/src/app/view_builders/main_view.rs`

- [ ] **Step 1: Add `mouse_area`, `container`, and `Padding` to imports**

At the top of `refbox/src/app/view_builders/main_view.rs`, the existing iced import includes `button`, `column`, `row`, `text`. Add `mouse_area`, `container`, and `Padding`:

```rust
use iced::{
    Alignment, Element, Length, Padding,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, container, mouse_area, row, text},
};
```

- [ ] **Step 2: Add `manual_alarm_enabled` parameter to `build_main_view`**

Change the function signature from:

```rust
pub(in super::super) fn build_main_view<'a>(
    data: ViewData<'_, '_>,
    game_config: &GameConfig,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    track_fouls_and_warnings: bool,
) -> Element<'a, Message>
```

To:

```rust
pub(in super::super) fn build_main_view<'a>(
    data: ViewData<'_, '_>,
    game_config: &GameConfig,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    track_fouls_and_warnings: bool,
    manual_alarm_enabled: bool,
) -> Element<'a, Message>
```

- [ ] **Step 3: Replace the game info / warnings section**

Find the section in `build_main_view` that pushes the game info button and, conditionally, the warnings panel. It currently looks like:

```rust
center_col = center_col.push(if max_num_warns < 4 {
    button(text(config_string(...))...)
        .on_press(Message::ShowGameDetails)
} else {
    button(text(config_string_game_num(...))...)
        .on_press(Message::ShowGameDetails)
});

if track_fouls_and_warnings {
    center_col = center_col.push( /* warnings panel */ );
}
```

Replace that entire block with the following:

```rust
if manual_alarm_enabled {
    // Compact GAME INFO button
    center_col = center_col.push(
        button(
            text(fl!("game-info"))
                .size(SMALL_TEXT)
                .align_y(Vertical::Center)
                .align_x(Horizontal::Center),
        )
        .padding(PADDING)
        .style(light_gray_button)
        .width(Length::Fill)
        .on_press(Message::ShowGameDetails),
    );

    // Determine whether we're in active play with no timeout.
    // This drives both the colour scheme (red vs blue) and the button label.
    // The button is always interactive while the feature is enabled — the
    // minimum hold duration (handled in update()) gates whether a press fires.
    let is_active_play = matches!(
        (snapshot.current_period, snapshot.timeout),
        (
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath,
            None,
        )
    );

    // Build the alarm button (visual only — mouse_area handles press/release)
    let alarm_btn = if is_active_play {
        button(
            column![
                text(fl!("alarm")).size(MEDIUM_TEXT),
                text(fl!("or-press-spacebar")).size(SMALL_TEXT),
            ]
            .align_x(Alignment::Center),
        )
        .style(red_button)
    } else {
        button(
            column![
                text(fl!("hold-to-test")).size(MEDIUM_TEXT),
                text(fl!("or-hold-spacebar")).size(SMALL_TEXT),
            ]
            .align_x(Alignment::Center),
        )
        .style(blue_button)
    }
    .width(Length::Fill)
    .on_press(Message::NoAction);

    // Alarm zone: container with padding so the button doesn't fill edge-to-edge.
    // mouse_area always binds press/release — the feature is always interactive
    // when enabled; state-specific behaviour is in the update() handler.
    let alarm_zone = mouse_area(
        container(alarm_btn)
            .style(light_gray_container)
            .padding(Padding::from([PADDING * 3.0, PADDING * 3.0]))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
    )
    .on_press(Message::AlarmPressed)
    .on_release(Message::AlarmReleased);

    if track_fouls_and_warnings {
        // Split the lower area: alarm left, warnings right
        let warnings_zone = button(
            column![
                text(fl!("warnings"))
                    .align_y(Vertical::Top)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill),
                row(snapshot.warnings.iter().map(|(color, warns)| column(
                    warns
                        .iter()
                        .rev()
                        .take(10)
                        .map(|warning| make_warning_container(warning, Some(color)).into())
                )
                .spacing(1)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()))
                .spacing(SPACING),
            ]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(light_gray_button)
        .on_press(Message::ShowWarnings);

        center_col = center_col.push(
            row![alarm_zone, warnings_zone]
                .spacing(SPACING)
                .height(Length::Fill),
        );
    } else {
        center_col = center_col.push(alarm_zone);
    }
} else {
    // Original behavior: game info area + optional warnings panel
    center_col = center_col.push(if max_num_warns < 4 {
        button(
            text(config_string(
                snapshot,
                game_config,
                using_uwhportal,
                schedule,
                teams,
            ))
            .size(SMALL_TEXT)
            .align_y(Vertical::Center)
            .align_x(Horizontal::Left),
        )
        .padding(PADDING)
        .style(light_gray_button)
        .height(Length::FillPortion(2))
        .width(Length::Fill)
        .on_press(Message::ShowGameDetails)
    } else {
        button(
            text(config_string_game_num(snapshot, using_uwhportal, schedule.map(|s| &s.games)).0)
                .size(SMALL_TEXT)
                .align_y(Vertical::Center)
                .align_x(Horizontal::Left),
        )
        .padding(PADDING)
        .style(light_gray_button)
        .width(Length::Fill)
        .on_press(Message::ShowGameDetails)
    });

    if track_fouls_and_warnings {
        center_col = center_col.push(
            button(
                column![
                    text(fl!("warnings"))
                        .align_y(Vertical::Top)
                        .align_x(Horizontal::Center)
                        .width(Length::Fill),
                    row(snapshot.warnings.iter().map(|(color, warns)| column(
                        warns
                            .iter()
                            .rev()
                            .take(10)
                            .map(|warning| make_warning_container(warning, Some(color)).into())
                    )
                    .spacing(1)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()))
                    .spacing(SPACING),
                ]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::NoAction)
            .style(light_gray_button)
            .on_press(Message::ShowWarnings),
        )
    }
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check -p refbox
```

Expected: one error — the call site in `app/mod.rs` does not yet pass the new parameter. Fix that in the next task.

---

### Task 8: Pass `manual_alarm_enabled` at the call site

**Files:**
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Update the `build_main_view` call**

Find the call to `build_main_view` at around line 2171:

```rust
build_main_view(
    data,
    game_config,
    self.using_uwhportal,
    self.schedule.as_ref(),
    self.config.track_fouls_and_warnings,
)
```

Add the new argument:

```rust
build_main_view(
    data,
    game_config,
    self.using_uwhportal,
    self.schedule.as_ref(),
    self.config.track_fouls_and_warnings,
    self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled,
)
```

- [ ] **Step 2: Full compilation check**

```bash
cargo check -p refbox
```

Expected: no errors.

- [ ] **Step 3: Run all tests**

```bash
just test
```

Expected: all tests pass.

- [ ] **Step 4: Run full validation**

```bash
just check
```

Expected: formatting clean, linter clean, tests pass, audit clean.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/main_view.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): add manual alarm button to main view with spacebar support"
```

---

## Manual Verification Checklist

After `just check` passes, verify the following by running the app (`cargo run -p refbox` or the simulator):

1. **Default state:** Open Sound settings — "ALARM BUTTON" toggle is present, shows "OFF", is greyed out if Sound is disabled.
2. **Enable the feature:** Turn on Sound + Alarm Button. Return to main screen — game info area is replaced by "GAME INFO" + alarm zone.
3. **GAME INFO button:** Tapping it opens the game details screen.
4. **During live play (no timeout):** Alarm button is **red**, shows "Alarm / Or press Spacebar". Hold button (or spacebar) for 250ms → buzzer fires and continues while held. Release before 250ms → nothing fires. Release while playing → current tone finishes, then silence.
5. **During a timeout in a play period:** Alarm button is **blue**, shows "Hold to Test / Or hold Spacebar". Hold button (or spacebar) for 1 second → buzzer fires and continues while held. Release before 1 second → nothing fires.
6. **During Between Games:** Alarm button is **blue**, shows "Hold to Test / Or hold Spacebar". Same 1-second hold behaviour as item 5.
7. **During Half Time / Pre-Overtime / Overtime Half Time / Pre-Sudden Death:** Alarm button is **blue**, shows "Hold to Test / Or hold Spacebar". Same 1-second hold behaviour as items 5 and 6.
8. **Fouls & Warnings ON:** The lower area splits — alarm on the left, warnings panel on the right. Both work as expected.
9. **Fouls & Warnings OFF:** Alarm zone takes full width.
10. **Feature disabled:** Turn off Alarm Button in settings — main screen reverts to original game info layout with no alarm button.
11. **On other screens:** Pressing spacebar on the configuration, penalties, or score edit screen does nothing.

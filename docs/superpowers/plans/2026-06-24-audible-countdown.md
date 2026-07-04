# Visual & Audible 10-Second Countdown Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the visual "hide last-15-seconds" setting to the App Options page (relabelled "SHOW COUNTDOWN FOR LAST 10 SECONDS", inverted, threshold 15→10), and add an "AUDIBLE COUNTDOWN FOR LAST 10 SECONDS" setting that beeps once per second for the final 10 seconds before each playing period.

**Architecture:** All changes live in the `refbox` crate. The visual setting keeps its internal `Config.hide_time` flag (no migration) and is inverted only at the button. The audible feature adds a new `Config.audible_countdown` flag, a dedicated embedded beep clip (mirroring the whistle — NOT a selectable buzzer), and a one-shot trigger inside `maybe_play_sound` that reads the *raw* game snapshot (independent of the visual setting).

**Tech Stack:** Rust 2024, iced 0.13, web-audio-api, Fluent (`.ftl`) translations, Python 3 (stdlib only) for the sound generator.

## Global Constraints

- MSRV Rust 1.85; edition 2024. No APIs newer than 1.85.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass (zero warnings, all platforms). No new `#[allow]`.
- No `unwrap()`/`expect()` in non-test code without a justifying comment (existing sound code uses `.unwrap()` on `msg_tx.send` with the established rationale — mirror it).
- All user-facing text goes through `translations/`; every new key must exist in **all 15 locales** (`de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN`). No English placeholders — best-guess translations, native review later.
- Literal label strings, exactly: `SHOW COUNTDOWN FOR LAST 10 SECONDS` and `AUDIBLE COUNTDOWN FOR LAST 10 SECONDS`.
- Do NOT touch `uwh-common`, the `tournament_manager` state machine, the wire format, or `wireless-remote`.
- Run `just check` before the final commit.

---

### Task 1: Add the `audible_countdown` config flag

**Files:**
- Modify: `refbox/src/config.rs` (struct ~213-235, `migrate` ~238-317, tests ~518+)

**Interfaces:**
- Produces: `Config.audible_countdown: bool` (default `false`), read later by `maybe_play_sound` and the UI.

- [ ] **Step 1: Write failing migration tests** — add to the `#[cfg(test)] mod tests` block in `config.rs` (next to `test_migrate_show_behind_schedule_time_*`):

```rust
    #[test]
    fn test_migrate_audible_countdown_defaults_false_when_absent() {
        let old: Table = Default::default();
        let config = Config::migrate(&old);
        assert!(!config.audible_countdown);
    }

    #[test]
    fn test_migrate_audible_countdown_respects_present_true() {
        let mut old: Table = Default::default();
        old.insert("audible_countdown".to_string(), toml::Value::Boolean(true));
        let config = Config::migrate(&old);
        assert!(config.audible_countdown);
    }
```

- [ ] **Step 2: Run, verify it fails to compile** — `cargo test -p refbox config:: 2>&1 | head` → Expected: error, no field `audible_countdown`.

- [ ] **Step 3: Add the struct field** — in `Config` (after `confirm_score`, ~line 224):

```rust
    #[derivative(Default(value = "true"))]
    pub confirm_score: bool,
    #[serde(default)]
    pub audible_countdown: bool,
    pub game: Game,
```

(`#[serde(default)]` matches the newer-field convention so loading an older config that lacks the key still deserializes.)

- [ ] **Step 4: Wire it into `migrate`** — add `mut audible_countdown` to the `Default::default()` destructure (near `confirm_score`), a read call after the `show_behind_schedule_time` read, and the field in the returned `Self`:

```rust
        // in the destructure:
        confirm_score,
        mut audible_countdown,
        mut game,
        // ... after get_boolean_value(... "show_behind_schedule_time" ...):
        get_boolean_value(old, "audible_countdown", &mut audible_countdown);
        // ... in the returned Self { ... }:
        confirm_score,
        audible_countdown,
        game,
```

- [ ] **Step 5: Run tests** — `cargo test -p refbox config::` → Expected: PASS (both new tests + existing).

- [ ] **Step 6: Commit**

```bash
git add refbox/src/config.rs
git commit -m "feat(refbox): add audible_countdown config flag"
```

---

### Task 2: Edit/apply plumbing for `audible_countdown`

**Files:**
- Modify: `refbox/src/app/message.rs` (`BoolGameParameter` ~795-810)
- Modify: `refbox/src/app/mod.rs` (handler ~3478; apply block ~926-930; EditableSettings builds)
- Modify: `refbox/src/app/view_builders/configuration.rs` (`EditableSettings` struct ~26-53)

**Interfaces:**
- Consumes: `Config.audible_countdown` (Task 1).
- Produces: `BoolGameParameter::AudibleCountdown`; `EditableSettings.audible_countdown: bool`.

- [ ] **Step 1: Add the message variant** — in `message.rs`, after `ConfirmScore` in `BoolGameParameter`:

```rust
    ConfirmScore,
    AudibleCountdown,
    ManualAlarmEnabled,
```

- [ ] **Step 2: Add the toggle handler** — in `mod.rs`, after the `BoolGameParameter::ConfirmScore` arm (~3493):

```rust
                            BoolGameParameter::AudibleCountdown => {
                                edited_settings.audible_countdown ^= true
                            }
```

- [ ] **Step 3: Add the `EditableSettings` field** — in `configuration.rs` (~line 43, after `confirm_score`):

```rust
    pub confirm_score: bool,
    pub audible_countdown: bool,
```

- [ ] **Step 4: Wire every `EditableSettings { ... }` construction** — add `audible_countdown: self.config.audible_countdown,` immediately after each `confirm_score: self.config.confirm_score,`. Sites in `mod.rs`: ~1503, ~4304, ~4352, ~4437, ~4494. (Search `confirm_score: self.config.confirm_score` and add the sibling line at each.)

- [ ] **Step 5: Wire the Apply assignment** — in `mod.rs` apply block (~930), after `self.config.confirm_score = confirm_score;`:

```rust
        self.config.confirm_score = confirm_score;
        self.config.audible_countdown = audible_countdown;
```

Then add `audible_countdown` to that function's destructured parameter list (the same destructure that yields `confirm_score`, ~lines 920-925) — `audible_countdown,` alongside `confirm_score,`. (The compiler will point to the exact destructure if missed.)

- [ ] **Step 6: Build** — `cargo build -p refbox` → Expected: compiles. Fix any "missing field `audible_countdown`" errors by adding the sibling line next to `confirm_score` at that site.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): wire audible_countdown through edit/apply settings"
```

---

### Task 3: Move `hide_time` Apply-tracking Display→App; add `audible_countdown` to App snapshot

This is the Apply-enable machinery. `hide_time` is leaving the Display page, so its page-entry tracking must move to App, and the new flag must be tracked on App — or the Apply button will misbehave.

**Files:**
- Modify: `refbox/src/app/mod.rs` (`PageEntrySnapshot` enum ~362-389; `revert_into` ~412-443; `capture_snapshot_for` ~1409-1425)
- Modify: `refbox/src/app/view_builders/configuration.rs` (dirty-check ~221-258; tests ~2285-2460)

- [ ] **Step 1: Enum** — in `PageEntrySnapshot::App { ... }` add `hide_time: bool,` and `audible_countdown: bool,` (after `confirm_score`). In `PageEntrySnapshot::Display { ... }` REMOVE `hide_time: bool,`.

- [ ] **Step 2: `revert_into`** — in the `PageEntrySnapshot::App { .. }` arm add `hide_time,` and `audible_countdown,` to the pattern and:

```rust
                edited.confirm_score = confirm_score;
                edited.hide_time = hide_time;
                edited.audible_countdown = audible_countdown;
```

In the `PageEntrySnapshot::Display { .. }` arm REMOVE `hide_time,` from the pattern and the `edited.hide_time = hide_time;` line.

- [ ] **Step 3: `capture_snapshot_for`** — in `ConfigPage::App => PageEntrySnapshot::App { ... }` (~1409) add:

```rust
                confirm_score: edited.confirm_score,
                hide_time: edited.hide_time,
                audible_countdown: edited.audible_countdown,
```

In `ConfigPage::Display => PageEntrySnapshot::Display { ... }` (~1420) REMOVE `hide_time: edited.hide_time,`.

- [ ] **Step 4: Dirty-check** — in `configuration.rs`, the `(ConfigPage::App, PageEntrySnapshot::App { ... })` arm (~221): add `hide_time,` and `audible_countdown,` to the destructure and:

```rust
                || edited.confirm_score != *confirm_score
                || edited.hide_time != *hide_time
                || edited.audible_countdown != *audible_countdown
```

In the `(ConfigPage::Display, PageEntrySnapshot::Display { ... })` arm (~245): remove `hide_time,` from the destructure and the `|| edited.hide_time != *hide_time` line.

- [ ] **Step 5: Fix the existing snapshot tests** — `configuration.rs` ~2285-2460 builds `PageEntrySnapshot::Display { ... hide_time: ... }` (3 sites) and `PageEntrySnapshot::App { ... }` (1 site). Remove `hide_time` from the Display constructions; add `hide_time: <existing-value-or-false>, audible_countdown: false,` to the App construction. If a Display test specifically asserted the hide_time dirty behaviour, move that assertion to the App test (toggle `edited.hide_time` and assert the App page is dirty).

- [ ] **Step 6: Build + test** — `cargo build -p refbox && cargo test -p refbox configuration::` → Expected: compiles, tests pass.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/mod.rs refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): move hide_time apply-tracking to App page, track audible_countdown"
```

---

### Task 4: Generate and embed the countdown beep clip

**Files:**
- Create: `refbox/resources/sounds/regen-countdown-beep.py`
- Create: `refbox/resources/sounds/countdown.raw` (generated)
- Modify: `refbox/src/sound_controller/sounds.rs`

**Interfaces:**
- Produces: `SoundLibrary::countdown(&self) -> &AudioBuffer`.

- [ ] **Step 1: Write the generator script** — `refbox/resources/sounds/regen-countdown-beep.py` (stdlib only, no numpy):

```python
#!/usr/bin/env python3
"""Regenerate countdown.raw: a single short "pip" played once per second during
the final 10 seconds before a playing period. Mono, 32-bit float LE, 44,100 Hz —
same format as buzz.raw / whistle.raw (raw samples, no header). Re-run after
tweaking FREQ/DUR to retune the tone, then commit countdown.raw."""
import math, struct, os

SR = 44100
FREQ = 1000.0   # Hz — clear mid-high pip
DUR = 0.15      # seconds
FADE = 0.005    # 5 ms fade in/out to avoid clicks
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "countdown.raw")

n = int(round(DUR * SR))
fade_n = max(1, int(round(FADE * SR)))
with open(OUT, "wb") as f:
    for i in range(n):
        x = math.sin(2 * math.pi * FREQ * i / SR)
        if i < fade_n:
            x *= i / fade_n
        elif i >= n - fade_n:
            x *= (n - i) / fade_n
        f.write(struct.pack("<f", x * 0.9))
print(f"wrote {OUT}: {n} samples ({DUR * 1000:.0f} ms @ {SR} Hz)")
```

- [ ] **Step 2: Generate the clip** — `python3 refbox/resources/sounds/regen-countdown-beep.py` → Expected: prints `wrote .../countdown.raw: 6615 samples (150 ms @ 44100 Hz)`. Verify: `ls -l refbox/resources/sounds/countdown.raw` shows 26460 bytes.

- [ ] **Step 3: Embed it** — in `sounds.rs`, after the `TWO_TONE` block (~line 47):

```rust
const COUNTDOWN_LEN: usize = include_bytes!("../../resources/sounds/countdown.raw").len() / 4;
static COUNTDOWN: [f32; COUNTDOWN_LEN] =
    process_array(include_bytes!("../../resources/sounds/countdown.raw"));
```

In `struct SoundLibrary` add `countdown: AudioBuffer,` (after `whistle`). In `SoundLibrary::new`, before `Self {`:

```rust
        let mut countdown = context.create_buffer(1, COUNTDOWN_LEN, SAMPLE_RATE);
        countdown.copy_to_channel(&COUNTDOWN, 0);
```

Add `countdown,` to the returned `Self { ... }`. Add the accessor after `whistle()`:

```rust
    pub(super) fn countdown(&self) -> &AudioBuffer {
        &self.countdown
    }
```

- [ ] **Step 4: Build** — `cargo build -p refbox` → Expected: compiles (the `const`-evaluated array length resolves from the committed `.raw`).

- [ ] **Step 5: Commit**

```bash
git add refbox/resources/sounds/regen-countdown-beep.py refbox/resources/sounds/countdown.raw refbox/src/sound_controller/sounds.rs
git commit -m "feat(refbox): add synthesized countdown beep clip"
```

---

### Task 5: Sound-controller plumbing for the beep

**Files:**
- Modify: `refbox/src/sound_controller/button_handler/mod.rs` (`SoundMessage` ~19-32)
- Modify: `refbox/src/sound_controller/mod.rs` (`SoundId` ~222-231; msg match ~341; `start_sound` match ~448-528; public method ~599)

**Interfaces:**
- Consumes: `SoundLibrary::countdown()` (Task 4).
- Produces: `SoundController::trigger_countdown_beep(&self)`.

- [ ] **Step 1: Add the message** — in `button_handler/mod.rs` `SoundMessage` (NOT linux-gated), after `TriggerWhistle`:

```rust
    TriggerWhistle,
    TriggerCountdownBeep,
```

- [ ] **Step 2: Add the SoundId** — in `mod.rs` `enum SoundId`, after `Whistle`:

```rust
    Whistle,
    CountdownBeep,
```

- [ ] **Step 3: Queue the beep** — in the `msg` match, after the `SoundMessage::TriggerWhistle` arm:

```rust
                                    SoundMessage::TriggerCountdownBeep => {
                                        if !sound_queue.contains(&SoundId::CountdownBeep) {
                                            sound_queue.push_back(SoundId::CountdownBeep);
                                        }
                                    }
```

- [ ] **Step 4: Play it one-shot** — in the `start_sound` closure's match (after the `SoundId::Whistle` arm, ~473):

```rust
                        SoundId::CountdownBeep => {
                            info!("Playing countdown beep once");
                            let volumes = ChannelVolumes::new(&settings, false);
                            Sound::new(
                                context.clone(),
                                volumes,
                                library.countdown().clone(),
                                false,
                                false,
                            )
                        }
```

(`(repeat=false, timed=false)` = play once for the clip's own duration, then auto-removed from the queue — identical lifecycle to the whistle. `ChannelVolumes::new(&settings, false)` uses the buzzer above/under-water volumes and is silenced when `sound_enabled` is false.)

- [ ] **Step 5: Public trigger** — in `impl SoundController`, after `trigger_whistle`:

```rust
    pub fn trigger_countdown_beep(&self) {
        self.msg_tx
            .send(SoundMessage::TriggerCountdownBeep)
            .unwrap()
    }
```

- [ ] **Step 6: Build** — `cargo build -p refbox` → Expected: compiles. (`SoundId` derives `Ord` for the `BTreeMap`; the new variant is fine.)

- [ ] **Step 7: Commit**

```bash
git add refbox/src/sound_controller/
git commit -m "feat(refbox): add countdown beep to sound controller"
```

---

### Task 6: Countdown trigger logic in `maybe_play_sound`

**Files:**
- Modify: `refbox/src/app/mod.rs` (`maybe_play_sound` ~498-558; new helper + tests)

**Interfaces:**
- Consumes: `Config.audible_countdown`, `SoundController::trigger_countdown_beep`.
- Produces: `fn should_play_countdown_beep(period, new_secs, old_secs, audible_countdown) -> bool`.

- [ ] **Step 1: Write the failing test** — add a `#[cfg(test)]` module near `maybe_play_sound` (or in the existing app tests module):

```rust
#[cfg(test)]
mod countdown_beep_tests {
    use super::should_play_countdown_beep;
    use uwh_common::game_snapshot::GamePeriod;

    #[test]
    fn fires_each_second_10_down_to_1_in_each_break() {
        for p in [
            GamePeriod::BetweenGames,
            GamePeriod::HalfTime,
            GamePeriod::PreOvertime,
            GamePeriod::OvertimeHalfTime,
            GamePeriod::PreSuddenDeath,
        ] {
            for s in 1..=10u32 {
                assert!(should_play_countdown_beep(p, s, s + 1, true), "{p:?} @ {s}");
            }
        }
    }

    #[test]
    fn silent_outside_window_when_disabled_or_unchanged() {
        assert!(!should_play_countdown_beep(GamePeriod::HalfTime, 11, 12, true));
        assert!(!should_play_countdown_beep(GamePeriod::HalfTime, 0, 1, true));
        assert!(!should_play_countdown_beep(GamePeriod::HalfTime, 10, 11, false));
        assert!(!should_play_countdown_beep(GamePeriod::HalfTime, 10, 10, true));
    }

    #[test]
    fn never_fires_during_playing_periods() {
        for p in [
            GamePeriod::FirstHalf,
            GamePeriod::SecondHalf,
            GamePeriod::OvertimeFirstHalf,
            GamePeriod::OvertimeSecondHalf,
            GamePeriod::SuddenDeath,
        ] {
            assert!(!should_play_countdown_beep(p, 5, 6, true), "{p:?}");
        }
    }
}
```

- [ ] **Step 2: Run, verify failure** — `cargo test -p refbox countdown_beep_tests 2>&1 | head` → Expected: `should_play_countdown_beep` not found.

- [ ] **Step 3: Implement the helper** — add as a free function in `mod.rs` (near `maybe_play_sound`):

```rust
/// One countdown beep this tick when: the audible-countdown setting is on, we are
/// in a break that precedes a playing period, the whole-second value just changed,
/// and the new value is in the final 10..=1 window. Reads the RAW snapshot, so it
/// is independent of the visual "show countdown" (`hide_time`) setting.
fn should_play_countdown_beep(
    period: GamePeriod,
    new_secs: u32,
    old_secs: u32,
    audible_countdown: bool,
) -> bool {
    let is_break_before_play = matches!(
        period,
        GamePeriod::BetweenGames
            | GamePeriod::HalfTime
            | GamePeriod::PreOvertime
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::PreSuddenDeath
    );
    audible_countdown && is_break_before_play && new_secs != old_secs && (1..=10).contains(&new_secs)
}
```

- [ ] **Step 4: Wire it into `maybe_play_sound`** — after the `match new_snapshot.timeout { ... }` block that yields `(play_whistle, play_buzzer)` (i.e. just before `if play_whistle {`), add:

```rust
        let play_countdown = new_snapshot.timeout.is_none()
            && should_play_countdown_beep(
                new_snapshot.current_period,
                new_snapshot.secs_in_period,
                self.snapshot.secs_in_period,
                self.config.audible_countdown,
            );
```

Then after the existing `if play_whistle { ... } else if play_buzzer { ... }` block, add an independent check (the beep can never coincide with the 30 s whistle or the 0 s buzzer):

```rust
        if play_countdown {
            info!("Triggering countdown beep");
            self.sound.trigger_countdown_beep();
        }
```

- [ ] **Step 5: Run tests** — `cargo test -p refbox countdown_beep_tests` → Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): play countdown beep in final 10s before each playing period"
```

---

### Task 7: Visual threshold 15 → 10 seconds

**Files:**
- Modify: `refbox/src/app/update_sender.rs` (~510 and ~515)

- [ ] **Step 1: Change both thresholds** — in the `if self.hide_time { match ... }` block, change `if self.snapshot.secs_in_period < 15 {` to `< 10` in BOTH the `BetweenGames | HalfTime | OvertimeHalfTime | PreOvertime` arm (~510) and the `PreSuddenDeath` arm (~515).

- [ ] **Step 2: Build** — `cargo build -p refbox` → Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/update_sender.rs
git commit -m "feat(refbox): visual countdown hide threshold 15s -> 10s"
```

---

### Task 8: UI — App page rows + Display page removal

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` (`make_app_config_page` ~925-1017; `make_display_config_page` ~1049-1197)

- [ ] **Step 1: App page — destructure** — in `make_app_config_page`, extend the `EditableSettings { ... }` destructure to include `hide_time,` and `audible_countdown,`:

```rust
    let EditableSettings {
        collect_scorer_cap_num,
        track_fouls_and_warnings,
        show_behind_schedule_time,
        confirm_score,
        hide_time,
        audible_countdown,
        ..
    } = settings;
```

- [ ] **Step 2: App page — replace the show-behind row + the empty spacer row** — replace the block from `row![ make_value_button( fl!("show-behind-schedule-time"), ... horizontal_space(), ] ... .height(Length::Fill),` PLUS the following `row![horizontal_space()].height(Length::Fill),` (current lines ~993-1006) with:

```rust
        row![
            make_value_button(
                // Internally still `hide_time`; the button shows the INVERSE:
                // YES = the final 10-second countdown IS shown on the scoreboard.
                fl!("show-countdown-for-last-10-seconds"),
                bool_string(!*hide_time),
                (false, true),
                Some(Message::ToggleBoolParameter(BoolGameParameter::HideTime)),
            ),
            make_value_button(
                fl!("audible-countdown-for-last-10-seconds"),
                bool_string(*audible_countdown),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::AudibleCountdown,
                )),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("show-behind-schedule-time"),
                bool_string(*show_behind_schedule_time),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::ShowBehindScheduleTime,
                )),
            ),
            horizontal_space(),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
```

- [ ] **Step 3: Display page — drop `hide_time` from the destructure** — in `make_display_config_page`, remove `hide_time,` from the `EditableSettings { ... }` destructure (it is no longer used here).

- [ ] **Step 4: Display page — remove the `hide_time_btn` binding** — delete the `let hide_time_btn = make_value_button(fl!("hide-time-for-last-15-seconds"), ...);` block (~1129-1134).

- [ ] **Step 5: Display page — Layout button keeps its place** — change `row![hide_time_btn, layout_btn]` (~1181) to:

```rust
        row![layout_btn, horizontal_space()]
            .spacing(SPACING)
            .height(Length::Fill),
```

- [ ] **Step 6: Build** — `cargo build -p refbox` → Expected: compiles (`fl!("hide-time-for-last-15-seconds")` is no longer referenced).

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): move show-countdown to App page, add audible countdown button"
```

---

### Task 9: Translations (remove old key, add two new keys ×15 locales)

**Files:**
- Modify: all 15 `refbox/translations/<locale>/refbox.ftl`

- [ ] **Step 1: Remove the obsolete key** — delete the `hide-time-for-last-15-seconds = ...` line from every locale file (it is no longer referenced; leaving it risks an unused-key check).

- [ ] **Step 2: Add the two new keys** — in each locale file (place near `show-behind-schedule-time`), add the lines from this table (best-guess, flagged for native review):

```
en-US: show-countdown-for-last-10-seconds = SHOW COUNTDOWN FOR LAST 10 SECONDS
       audible-countdown-for-last-10-seconds = AUDIBLE COUNTDOWN FOR LAST 10 SECONDS
de-DE: show-countdown-for-last-10-seconds = COUNTDOWN DER LETZTEN 10 SEKUNDEN
       audible-countdown-for-last-10-seconds = AKUSTISCHER COUNTDOWN 10 SEKUNDEN
es:    show-countdown-for-last-10-seconds = MOSTRAR CUENTA ATRÁS ÚLTIMOS 10 S
       audible-countdown-for-last-10-seconds = CUENTA ATRÁS SONORA ÚLTIMOS 10 S
fr:    show-countdown-for-last-10-seconds = AFFICHER COMPTE À REBOURS 10 S
       audible-countdown-for-last-10-seconds = COMPTE À REBOURS SONORE 10 S
id-ID: show-countdown-for-last-10-seconds = TAMPILKAN HITUNG MUNDUR 10 DETIK
       audible-countdown-for-last-10-seconds = HITUNG MUNDUR SUARA 10 DETIK
it-IT: show-countdown-for-last-10-seconds = MOSTRA CONTO ALLA ROVESCIA 10 S
       audible-countdown-for-last-10-seconds = CONTO ALLA ROVESCIA SONORO 10 S
ja-JP: show-countdown-for-last-10-seconds = 残り10秒のカウントダウン表示
       audible-countdown-for-last-10-seconds = 残り10秒の音声カウントダウン
ko-KR: show-countdown-for-last-10-seconds = 마지막 10초 카운트다운 표시
       audible-countdown-for-last-10-seconds = 마지막 10초 음성 카운트다운
ms-MY: show-countdown-for-last-10-seconds = TUNJUK KIRA DETIK 10 SAAT
       audible-countdown-for-last-10-seconds = KIRA DETIK BUNYI 10 SAAT
nl-NL: show-countdown-for-last-10-seconds = AFTELLING LAATSTE 10 SECONDEN
       audible-countdown-for-last-10-seconds = HOORBARE AFTELLING 10 SECONDEN
pt-PT: show-countdown-for-last-10-seconds = MOSTRAR CONTAGEM DECRESCENTE 10 S
       audible-countdown-for-last-10-seconds = CONTAGEM DECRESCENTE SONORA 10 S
th-TH: show-countdown-for-last-10-seconds = แสดงนับถอยหลัง 10 วินาทีสุดท้าย
       audible-countdown-for-last-10-seconds = นับถอยหลังด้วยเสียง 10 วินาที
tl-PH: show-countdown-for-last-10-seconds = IPAKITA COUNTDOWN HULING 10 SEGUNDO
       audible-countdown-for-last-10-seconds = COUNTDOWN NA TUNOG HULING 10 SEGUNDO
tr-TR: show-countdown-for-last-10-seconds = SON 10 SANİYE GERİ SAYIMI GÖSTER
       audible-countdown-for-last-10-seconds = SESLİ GERİ SAYIM SON 10 SANİYE
zh-CN: show-countdown-for-last-10-seconds = 显示最后10秒倒计时
       audible-countdown-for-last-10-seconds = 最后10秒声音倒计时
```

- [ ] **Step 3: Verify coverage** — confirm each new key exists in all 15 files and the old key is gone:

```bash
for k in show-countdown-for-last-10-seconds audible-countdown-for-last-10-seconds hide-time-for-last-15-seconds; do echo "$k:"; grep -rl "$k" refbox/translations | wc -l; done
```

Expected: `15`, `15`, `0`.

- [ ] **Step 4: Commit**

```bash
git add refbox/translations/
git commit -m "feat(refbox): translations for show/audible countdown labels (15 locales)"
```

---

### Task 10: Full verification

- [ ] **Step 1:** `just check` → Expected: fmt, clippy (-D warnings), tests, audit all clean.
- [ ] **Step 2: Walkthrough build** — `cargo build -p refbox` then launch (see project run convention: `WAYLAND_DISPLAY= cargo run -p refbox`, background + `dangerouslyDisableSandbox`). Verify:
  - App Options shows SHOW COUNTDOWN (left) · AUDIBLE COUNTDOWN (right), then SHOW BEHIND TIME/DELAY on the next row; labels wrap acceptably.
  - Display Options: Layout button in place, empty space beside it, no countdown button.
  - With AUDIBLE COUNTDOWN = YES, hear 10 beeps in the final 10 s of a break before a half; retune `regen-countdown-beep.py` (FREQ/DUR) if desired and re-commit `countdown.raw`.
  - SHOW COUNTDOWN = YES shows the last-10-second break countdown; = NO hides it.

## Self-Review notes

- Spec coverage: visual relabel+invert (T8), threshold 15→10 (T7), move Display→App incl. Apply tracking (T3, T8), new flag (T1,T2), beep sound (T4,T5), trigger (T6), 15-locale labels (T9). ✔
- Type consistency: `should_play_countdown_beep` uses `u32` (matches `GameSnapshot.secs_in_period: u32`). Beep playback uses `(repeat=false, timed=false)` exactly like the whistle. ✔
- The audible trigger reads the raw snapshot in `maybe_play_sound` (verified independent of `update_sender`'s `hide_time` mutation). ✔

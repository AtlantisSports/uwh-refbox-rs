# Half Length: 2 Halves / 1 Period Selector — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an operator-facing "2 Halves / 1 Period" segmented selector to the Half Length parameter editor, staging the `single_half` choice in the editor and committing it on Done.

**Architecture:** The parameter editor already stages the edited `Duration` inside `AppState::ParameterEditor`. We extend that variant to also carry a staged `single_half` bool, render a two-button segmented selector (reusing the existing `blue_selected_button` / `light_gray_button` styles from the language picker), reuse the existing `Message::ToggleBoolParameter(BoolGameParameter::SingleHalf)` to flip the staged bool, and write it into `edited_settings.config.single_half` on Done. The game-flow effects of `single_half` (label change, Half-Time disabled, no second half) already exist.

**Tech Stack:** Rust 2024, `iced` 0.13, `i18n-embed-fl` Fluent translations, project `just` commands.

**Spec:** `docs/superpowers/specs/2026-06-05-half-length-two-halves-one-period-design.md`

---

## Process note (testing)

Per `.claude/rules/plan-execution.md` (lean process), this is low-blast-radius `refbox` UI
work. The iced view/update path has no clean unit-test seam (view builders return an opaque
`Element`; the update loop is driven by the iced runtime), and the project rule explicitly
allows compilation + `just check` + manual observation for this class of change. The
game-flow logic for single-period games is unchanged. So this plan verifies by build + lint +
`just check` + manual checks against the spec's acceptance criteria — no brittle synthetic
test is added.

## File structure

- `refbox/translations/*/refbox.ftl` — 4 new translation keys (Task 1).
- `refbox/src/app/mod.rs` — extend `AppState::ParameterEditor` to a 3-tuple; update all match
  sites; capture staged `single_half` on `EditParameter`; commit it on `ParameterEditComplete`;
  re-route the `SingleHalf` toggle to mutate the editor's staged bool (Task 2).
- `refbox/src/app/message.rs` — drop the `#[expect(dead_code)]` on
  `BoolGameParameter::SingleHalf` (Task 2).
- `refbox/src/app/view_builders/configuration.rs` — render the selector + dynamic title/hint in
  `build_game_parameter_editor`; pass `single_half` from the view dispatch (Task 3).

---

### Task 1: Add translation keys

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl`
- Modify (English placeholders, per the codebase's "keep all 15 locales in sync" convention):
  `de-DE`, `es`, `fr`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`,
  `tl-PH`, `tr-TR`, `zh-CN` (each `…/refbox.ftl`)

Note: `i18n.toml` sets `fallback_language = "en-US"`, so missing keys fall back to English
automatically. We still add explicit English placeholders to every locale to match the existing
convention (see the comment in `refbox/src/app/view_builders/beep_test_settings.rs:208`) and so
translators have a visible entry to fill in later.

- [ ] **Step 1: Add the four keys to `en-US`**

In `refbox/translations/en-US/refbox.ftl`, near the other Half Length keys (the
`half-length` / `length-of-half-during-regular-play` block around line 133), add:

```ftl
two-halves = 2 HALVES
one-period = 1 PERIOD
game-len = GAME LEN
length-of-game-during-regular-play = The length of the game during regular play
```

- [ ] **Step 2: Add the same four lines (English placeholder) to each other locale**

Append the identical four lines above to each of the other 14 locale files
(`de-DE`, `es`, `fr`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`,
`tl-PH`, `tr-TR`, `zh-CN` — each `refbox/translations/<locale>/refbox.ftl`). They remain
English until a native translation is supplied; this is the intended placeholder behaviour.

- [ ] **Step 3: Build to confirm the Fluent assets compile**

Run: `cargo build -p refbox`
Expected: builds successfully (new keys are not yet referenced; this just confirms valid .ftl).

- [ ] **Step 4: Commit**

```bash
git add refbox/translations
git commit -m "feat(refbox): add 2 halves / 1 period translation keys"
```

---

### Task 2: Carry staged `single_half` in the parameter editor state

**Files:**
- Modify: `refbox/src/app/mod.rs` (AppState def ~192; keypad mut ~1408; `EditParameter` ~2340;
  `ParameterEditComplete` ~2385; `next_state` ~2466; view dispatch ~3786; `ToggleBoolParameter`
  ~2555)
- Modify: `refbox/src/app/message.rs` (`BoolGameParameter::SingleHalf` attribute)

- [ ] **Step 1: Extend the AppState variant**

In `refbox/src/app/mod.rs` change the variant definition (line ~192) from:

```rust
    ParameterEditor(LengthParameter, Duration),
```

to:

```rust
    // 3rd field: staged `single_half` choice, only meaningful for
    // LengthParameter::Half (the 2 Halves / 1 Period selector). Carried here so
    // it commits on Done and is discarded on Cancel, like the edited Duration.
    ParameterEditor(LengthParameter, Duration, bool),
```

- [ ] **Step 2: Update the keypad duration-mutation match arm**

At line ~1408, change:

```rust
                    AppState::ParameterEditor(_, ref mut dur) => (dur, false),
```

to:

```rust
                    AppState::ParameterEditor(_, ref mut dur, _) => (dur, false),
```

- [ ] **Step 3: Capture staged `single_half` when opening the editor**

In the `Message::EditParameter(param)` handler (~2340), the state is built as
`AppState::ParameterEditor(param, <dur match>)`. Capture the current staged `single_half`
(from the edited settings the Game Options page is showing, falling back to the live config)
and pass it as the third field. Replace the assignment so it reads:

```rust
            Message::EditParameter(param) => {
                let single_half = self
                    .edited_settings
                    .as_ref()
                    .map(|s| s.config.single_half)
                    .unwrap_or(self.config.game.single_half);
                self.app_state = AppState::ParameterEditor(
                    param,
                    match param {
                        LengthParameter::Half => self.config.game.half_play_duration,
                        LengthParameter::HalfTime => self.config.game.half_time_duration,
                        LengthParameter::NominalBetweenGame => self.config.game.nominal_break,
                        LengthParameter::MinimumBetweenGame => self.config.game.minimum_break,
                        LengthParameter::PreOvertime => self.config.game.pre_overtime_break,
                        LengthParameter::OvertimeHalf => self.config.game.ot_half_play_duration,
                        LengthParameter::OvertimeHalfTime => self.config.game.ot_half_time_duration,
                        LengthParameter::PreSuddenDeath => {
                            self.config.game.pre_sudden_death_duration
                        }
                    },
                    single_half,
                );
                trace!("AppState changed to {:?}", self.app_state);
                Task::none()
            }
```

- [ ] **Step 4: Commit the staged `single_half` on Done**

In `Message::ParameterEditComplete { canceled }` (~2385), change the match arm header from:

```rust
                        AppState::ParameterEditor(param, dur) => {
```

to:

```rust
                        AppState::ParameterEditor(param, dur, single_half) => {
```

and inside that arm, change the `LengthParameter::Half` case to also write the staged
`single_half`:

```rust
                                LengthParameter::Half => {
                                    edited_settings.config.half_play_duration = dur;
                                    edited_settings.config.single_half = single_half;
                                }
```

(The other `LengthParameter` cases are unchanged.)

- [ ] **Step 5: Update the `next_state` match arm**

At line ~2466, change:

```rust
                    AppState::ParameterEditor(_, _) => AppState::EditGameConfig(ConfigPage::Game),
```

to:

```rust
                    AppState::ParameterEditor(_, _, _) => {
                        AppState::EditGameConfig(ConfigPage::Game)
                    }
```

- [ ] **Step 6: Re-route the `SingleHalf` toggle to the editor's staged bool**

In `Message::ToggleBoolParameter(param)` (~2555), `BoolGameParameter::SingleHalf` currently
lives inside the inner `_ =>` block and does `edited_settings.config.single_half ^= true`.
Remove it from there and add a dedicated top-level arm next to `TimeoutsCountedPerHalf`
(which already mutates `AppState` in place):

Add this arm (alongside `TeamWarning` / `TimeoutsCountedPerHalf`):

```rust
                    BoolGameParameter::SingleHalf => {
                        if let AppState::ParameterEditor(_, _, ref mut single_half) =
                            self.app_state
                        {
                            *single_half ^= true
                        } else {
                            unreachable!()
                        }
                        trace!("AppState changed to {:?}", self.app_state)
                    }
```

And delete the old inner arm:

```rust
                            BoolGameParameter::SingleHalf => {
                                edited_settings.config.single_half ^= true
                            }
```

- [ ] **Step 7: Update the view dispatch**

At line ~3786, change:

```rust
            AppState::ParameterEditor(param, dur) => build_game_parameter_editor(data, param, dur),
```

to:

```rust
            AppState::ParameterEditor(param, dur, single_half) => {
                build_game_parameter_editor(data, param, dur, single_half)
            }
```

- [ ] **Step 8: Make the `SingleHalf` message variant non-dead**

In `refbox/src/app/message.rs`, remove the `#[expect(dead_code)]` attribute (and its
now-stale TODO comment) on the `BoolGameParameter::SingleHalf` variant, since it is now
emitted by the selector. (Leave the variant itself.)

- [ ] **Step 9: Build — let the compiler flag any remaining match site**

Run: `cargo build -p refbox`
Expected: FAILS only in `build_game_parameter_editor` (Task 3 not done yet) — i.e. an
arity/arg error at the `build_game_parameter_editor` definition/call. All `AppState::ParameterEditor`
match sites should now compile. If the compiler reports any *other* unupdated
`ParameterEditor(...)` match arm, update it to the 3-field form. Do NOT proceed until the only
remaining errors are about `build_game_parameter_editor`'s signature.

- [ ] **Step 10: Commit**

```bash
git add refbox/src/app/mod.rs refbox/src/app/message.rs
git commit -m "feat(refbox): stage single_half in the parameter editor state"
```

---

### Task 3: Render the segmented selector and dynamic title/hint

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` (`build_game_parameter_editor`
  ~1267-1332)

- [ ] **Step 1: Add the `single_half` parameter to the function signature**

Change the signature (line ~1267) from:

```rust
pub(in super::super) fn build_game_parameter_editor<'a>(
    data: ViewData<'_, '_>,
    param: LengthParameter,
    length: Duration,
) -> Element<'a, Message> {
```

to:

```rust
pub(in super::super) fn build_game_parameter_editor<'a>(
    data: ViewData<'_, '_>,
    param: LengthParameter,
    length: Duration,
    single_half: bool,
) -> Element<'a, Message> {
```

- [ ] **Step 2: Make the Half title/hint dynamic**

In the `(title, hint)` match (line ~1280), change the `LengthParameter::Half` arm from:

```rust
        LengthParameter::Half => (
            fl!("half-length"),
            fl!("length-of-half-during-regular-play"),
        ),
```

to:

```rust
        LengthParameter::Half => (
            if single_half {
                fl!("game-len")
            } else {
                fl!("half-length")
            },
            if single_half {
                fl!("length-of-game-during-regular-play")
            } else {
                fl!("length-of-half-during-regular-play")
            },
        ),
```

- [ ] **Step 3: Build the segmented selector (Half only) and insert it above the time editor**

Replace the final `column![ ... ].into()` block (lines ~1298-1331) with the version below.
It builds a two-button selector for `LengthParameter::Half` (active = `blue_selected_button`
and not pressable; inactive = `light_gray_button` and pressable, emitting the existing
`SingleHalf` toggle), and inserts it between the top game-time button and the time editor.
For all other parameters the selector is absent and the layout is exactly as before.

```rust
    let format_selector: Option<Element<'a, Message>> = if matches!(param, LengthParameter::Half)
    {
        let two_halves = {
            let b = make_button(fl!("two-halves")).width(Length::Fill).style(
                if single_half {
                    light_gray_button
                } else {
                    blue_selected_button
                },
            );
            if single_half {
                b.on_press(Message::ToggleBoolParameter(BoolGameParameter::SingleHalf))
            } else {
                b
            }
        };
        let one_period = {
            let b = make_button(fl!("one-period")).width(Length::Fill).style(
                if single_half {
                    blue_selected_button
                } else {
                    light_gray_button
                },
            );
            if single_half {
                b
            } else {
                b.on_press(Message::ToggleBoolParameter(BoolGameParameter::SingleHalf))
            }
        };
        Some(row![two_halves, one_period].spacing(SPACING).into())
    } else {
        None
    };

    let mut col = column![make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator
    )]
    .spacing(SPACING)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill);

    if let Some(selector) = format_selector {
        col = col.push(selector);
    }

    col.push(vertical_space())
        .push(make_time_editor(title, length, false))
        .push(vertical_space())
        .push(
            text(fl!("help") + &hint)
                .size(SMALL_TEXT)
                .align_x(Horizontal::Center),
        )
        .push(vertical_space())
        .push(
            row![
                make_button(fl!("cancel"))
                    .style(red_button)
                    .width(Length::Fill)
                    .on_press(Message::ParameterEditComplete { canceled: true }),
                horizontal_space(),
                make_button(fl!("done"))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press(Message::ParameterEditComplete { canceled: false }),
            ]
            .spacing(SPACING),
        )
        .into()
}
```

- [ ] **Step 4: Build the crate**

Run: `cargo build -p refbox`
Expected: builds successfully, no errors.

- [ ] **Step 5: Lint (mirrors CI / `just lint` for this bin crate)**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: no warnings, no errors.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): add 2 halves / 1 period selector to Half Length editor"
```

---

### Task 4: Full check and manual verification

- [ ] **Step 1: Full project check**

Run: `just check`
Expected: fmt, lint, tests, audit all pass (the 5 pre-existing allowed audit advisories are
not failures).

- [ ] **Step 2: Manual verification against the spec's acceptance criteria**

Launch (no panel needed): `WAYLAND_DISPLAY= cargo run -p refbox`

1. Game Options → tap **Half Length** → the editor shows a **2 HALVES / 1 PERIOD** selector,
   with **2 HALVES** highlighted by default.
2. Tap **1 PERIOD** → title shows **GAME LEN**, help text updates to the game-length wording,
   highlight moves to **1 PERIOD**; the entered time is unchanged.
3. Tap **DONE** → back on Game Options the button reads **GAME LENGTH:** and **Half-Time
   Length** is greyed out.
4. Re-open the editor, tap **2 HALVES**, **DONE** → button reads **HALF LENGTH:** again and the
   previously-set Half-Time length is intact.
5. Re-open, change the selector, tap **CANCEL** → the format is unchanged from before.

- [ ] **Step 3: Final commit (if `just fmt` reformatted anything)**

```bash
git add -A
git commit -m "style(refbox): fmt after half-length selector" || true
```

---

## Self-Review

- **Spec coverage:**
  - Segmented selector with both options, active highlighted → Task 3 Step 3.
  - 1 Period → title "GAME LEN", Half-Time disabled, no 2nd half → Task 3 Step 2 (title) + existing
    behaviour (Half-Time disable already keyed on `single_half`; game flow already implemented).
  - Same single value across modes, not wiped → the keypad mutates the one staged `Duration`;
    switching format only flips the bool (Task 2 Step 6), never touches `dur`.
  - Staged together, committed on Done, discarded on Cancel → Task 2 Steps 3/4/6.
  - Half-time length retained when in 1 Period → we never clear `half_time_duration`; only the
    edit button is disabled (existing behaviour) — covered, no code needed.
  - Overtime/sudden-death untouched → not modified.
  - Wording/translations → Task 1.
  - refbox-only scope → no `uwh-common`/portal/wire changes in any task.
- **Placeholder scan:** none — every code step shows the full edited code. Task 2 Step 9's
  "compiler flags remaining sites" enumerates the known sites first and uses the compiler as a
  deterministic backstop, not as a vague instruction.
- **Type consistency:** `ParameterEditor(LengthParameter, Duration, bool)` is used identically
  in the definition (Task 2 Step 1), all match arms (Steps 2/4/5/7), and the dispatch call
  (Step 7); `build_game_parameter_editor(data, param, length, single_half)` signature (Task 3
  Step 1) matches its only call site (Task 2 Step 7). The reused message is
  `Message::ToggleBoolParameter(BoolGameParameter::SingleHalf)` throughout. New ftl keys
  `two-halves`, `one-period`, `game-len`, `length-of-game-during-regular-play` are defined in
  Task 1 and referenced in Task 3.

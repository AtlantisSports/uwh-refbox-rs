# Front Display Layouts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add selectable full-screen front-display layouts (Default, Classic, Big Time, Corners, Scores Only) to the refbox's player-facing display window, chosen via a cycling button on the Display Options page, gated off when the physical LED panel is connected, updating any open display live.

**Architecture:** A new `FrontDisplayLayout` enum is persisted in the refbox `Config` and cycled by a `Message::CycleFrontDisplayLayout` (immediate-commit, like the existing `CycleDisplayMode`). The choice reaches the display window by wrapping the existing binary/TCP frame with a one-byte layout selector (`SimFrame`) — **only** on the display path; the serial/hardware encoding (`TransmittedData`) is left byte-for-byte identical. The simulator window decodes the layout per frame and, for the four new layouts, draws a full-screen scoreboard on its existing `iced` `Canvas` (rectangles + text), scaled to the window. `Default` keeps calling `draw_panels` exactly as today.

**Tech Stack:** Rust 2024, `iced` 0.13 canvas, `matrix-drawing` (`TransmittedData`, `secs_to_time_string`), `confy` config, Fluent (`fl!`) translations.

**Scope guard (from the approved spec — do not exceed):**
- `refbox` crate only. **Do NOT** modify `matrix-drawing::transmitted_data::TransmittedData`, `uwh-common`, `overlay`, or anything on the serial/wireless path.
- Teams stay **WHITE/BLACK**. No team names/colours.
- New layouts show scores, clock, period, and timeout state only — **no** penalties/fouls/warnings.
- This is lean-process work: the deterministic plumbing tasks below carry exact code + tests; the four visual draw routines (Task 6) carry one fully-worked layout plus precise element specs for the rest — exact pixel positions are tuned visually, with final browser confirmation in Task 10. Treat coordinates in Task 6 as a starting point, not gospel.

**Pre-flight:** Work is on branch `feat/refbox/front-display-layouts` (already created off updated master; the approved spec is committed there). Do not commit the user's unrelated working-tree files (modified `CLAUDE.md`, the other untracked `docs/superpowers/*` and `refbox-translations.*` files). Stage only files you create/modify per each task.

---

## File Structure

**Create:**
- `refbox/src/sim_frame.rs` — `FrontDisplayLayout` enum + `SimFrame` (layout + `TransmittedData`) encode/decode. The shared, testable seam between the refbox server and the display window.
- `refbox/src/sim_app/scoreboard.rs` — the four full-screen draw routines + shared colour/text helpers.

**Modify:**
- `refbox/src/main.rs` — declare `mod sim_frame;`; register the bundled Roboto font in the simulator branch.
- `refbox/src/config.rs` — add `front_display_layout` field.
- `refbox/src/app/update_sender.rs` — `Server` carries a layout; binary path emits `SimFrame`; `ServerMessage::SetLayout`; `UpdateSender::set_layout`; `Server::new` takes an initial layout.
- `refbox/src/app/message.rs` — `CycleFrontDisplayLayout` variant.
- `refbox/src/app/mod.rs` — handle `CycleFrontDisplayLayout`; pass initial layout when constructing the server.
- `refbox/src/sim_app/mod.rs` — read `SimFrame`; store layout + latest snapshot; dispatch draw by layout.
- `refbox/src/app/view_builders/configuration.rs` — add the "Display Layout" value button on the Display page with hardware gating.
- `refbox/translations/{en-US,es,fr}/refbox.ftl` — new strings.

---

## Task 1: `FrontDisplayLayout` enum + config field

**Files:**
- Create: `refbox/src/sim_frame.rs`
- Modify: `refbox/src/main.rs` (add `mod sim_frame;`), `refbox/src/config.rs`

- [ ] **Step 1: Write the failing test** (in `refbox/src/sim_frame.rs`)

```rust
use matrix_drawing::transmitted_data::TransmittedData;
use serde::{Deserialize, Serialize};

/// Which front-display layout the player-facing display window renders.
/// `Default` is the existing 256x64 LED-panel mirror; the others are
/// full-screen scoreboards usable only when no physical panel is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FrontDisplayLayout {
    #[default]
    Default,
    Classic,
    BigTime,
    Corners,
    ScoresOnly,
}

impl FrontDisplayLayout {
    /// Cycle order for the picker (wraps).
    pub const fn next(self) -> Self {
        match self {
            Self::Default => Self::Classic,
            Self::Classic => Self::BigTime,
            Self::BigTime => Self::Corners,
            Self::Corners => Self::ScoresOnly,
            Self::ScoresOnly => Self::Default,
        }
    }

    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Default => 0,
            Self::Classic => 1,
            Self::BigTime => 2,
            Self::Corners => 3,
            Self::ScoresOnly => 4,
        }
    }

    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Classic,
            2 => Self::BigTime,
            3 => Self::Corners,
            4 => Self::ScoresOnly,
            _ => Self::Default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_order_wraps() {
        let mut l = FrontDisplayLayout::Default;
        let mut seen = vec![l];
        for _ in 0..5 {
            l = l.next();
            seen.push(l);
        }
        assert_eq!(
            seen,
            vec![
                FrontDisplayLayout::Default,
                FrontDisplayLayout::Classic,
                FrontDisplayLayout::BigTime,
                FrontDisplayLayout::Corners,
                FrontDisplayLayout::ScoresOnly,
                FrontDisplayLayout::Default,
            ]
        );
    }

    #[test]
    fn u8_round_trips() {
        for l in [
            FrontDisplayLayout::Default,
            FrontDisplayLayout::Classic,
            FrontDisplayLayout::BigTime,
            FrontDisplayLayout::Corners,
            FrontDisplayLayout::ScoresOnly,
        ] {
            assert_eq!(FrontDisplayLayout::from_u8(l.to_u8()), l);
        }
    }

    #[test]
    fn unknown_u8_falls_back_to_default() {
        assert_eq!(FrontDisplayLayout::from_u8(99), FrontDisplayLayout::Default);
    }
}
```

- [ ] **Step 2: Wire the module in**

In `refbox/src/main.rs`, add alongside the other top-level `mod` declarations (near `mod sim_app;`):

```rust
mod sim_frame;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p refbox sim_frame`
Expected: the three `sim_frame::tests` PASS.

- [ ] **Step 4: Add the config field**

In `refbox/src/config.rs`, in the `Config` struct (next to `pub display_mode: crate::app::theme::DisplayMode,` ~line 230), add:

```rust
    pub front_display_layout: crate::sim_frame::FrontDisplayLayout,
```

`#[derive(Default)]` on `Config` (or its `Default` impl) makes this default to `FrontDisplayLayout::Default`. If `Config` has a hand-written `Default`, add `front_display_layout: FrontDisplayLayout::Default,` there. Confirm `confy` (serde) serialises it — it derives `Serialize`/`Deserialize`, and missing-in-old-file is fine because the field defaults.

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p refbox`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/sim_frame.rs refbox/src/main.rs refbox/src/config.rs
git commit -m "feat(refbox): add FrontDisplayLayout enum and config field"
```

---

## Task 2: `SimFrame` — the display-only frame wrapper

**Files:**
- Modify: `refbox/src/sim_frame.rs`

This adds the one-byte layout prefix to the display frame **without touching `TransmittedData`** (so the serial/hardware encoding is unchanged).

- [ ] **Step 1: Write the failing test** (append to `refbox/src/sim_frame.rs`)

```rust
/// A display-feed frame: the existing panel payload plus a one-byte layout
/// selector. Used ONLY on the binary/TCP path to the display window. The
/// serial/hardware path keeps sending bare `TransmittedData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimFrame {
    pub layout: FrontDisplayLayout,
    pub data: TransmittedData,
}

impl SimFrame {
    pub const ENCODED_LEN: usize = TransmittedData::ENCODED_LEN + 1;

    pub fn encode(
        &self,
    ) -> Result<[u8; Self::ENCODED_LEN], uwh_common::game_snapshot::EncodingError> {
        let mut out = [0u8; Self::ENCODED_LEN];
        out[0] = self.layout.to_u8();
        out[1..].copy_from_slice(&self.data.encode()?);
        Ok(out)
    }

    pub fn decode(
        bytes: &[u8; Self::ENCODED_LEN],
    ) -> Result<Self, uwh_common::game_snapshot::DecodingError> {
        use arrayref::array_ref;
        Ok(Self {
            layout: FrontDisplayLayout::from_u8(bytes[0]),
            data: TransmittedData::decode(array_ref![bytes, 1, TransmittedData::ENCODED_LEN])?,
        })
    }
}
```

Add a test in the existing `tests` module:

```rust
    #[test]
    fn sim_frame_round_trips_and_is_one_byte_longer() {
        // Proves the serial/hardware format is untouched: SimFrame is exactly
        // one byte longer than TransmittedData, and that byte is the layout.
        assert_eq!(SimFrame::ENCODED_LEN, TransmittedData::ENCODED_LEN + 1);

        let frame = SimFrame {
            layout: FrontDisplayLayout::Corners,
            data: TransmittedData::default(),
        };
        let bytes = frame.encode().unwrap();
        assert_eq!(bytes[0], FrontDisplayLayout::Corners.to_u8());
        assert_eq!(SimFrame::decode(&bytes).unwrap(), frame);
    }
```

If `TransmittedData` does not derive `Default`, build a minimal value instead (e.g. `TransmittedData { white_on_right: false, flash: false, beep_test: false, brightness: matrix_drawing::transmitted_data::Brightness::Low, snapshot: Default::default() }`). Verify which by checking the struct; `GameSnapshotNoHeap` derives `Default`.

- [ ] **Step 2: Ensure `arrayref` is available to `refbox`**

Run: `cargo tree -p refbox -i arrayref 2>/dev/null | head` — if `arrayref` is not already a dependency of `refbox`, replace the `array_ref!` use in `decode` with a plain slice→array conversion:

```rust
            data: {
                let mut buf = [0u8; TransmittedData::ENCODED_LEN];
                buf.copy_from_slice(&bytes[1..]);
                TransmittedData::decode(&buf)?
            },
```

(Prefer the no-extra-dependency form unless `arrayref` is already present.)

- [ ] **Step 3: Run tests**

Run: `cargo test -p refbox sim_frame`
Expected: all `sim_frame::tests` PASS, including `sim_frame_round_trips_and_is_one_byte_longer`.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/sim_frame.rs
git commit -m "feat(refbox): add SimFrame display-feed wrapper (layout + payload)"
```

---

## Task 3: Server carries the layout; binary path emits `SimFrame`

**Files:**
- Modify: `refbox/src/app/update_sender.rs`

Reference patterns to mirror (verbatim, already in the file): `ServerMessage::SetHideTime(bool)`, the `Server` struct fields (`hide_time`, `beep_test`, `binary`, …), `Server::new(...)`, and `encode_flash()` (~lines 446-462).

- [ ] **Step 1: Add the field + message + setter**

Add to the `Server` struct:

```rust
    layout: crate::sim_frame::FrontDisplayLayout,
```

Add to `ServerMessage` (next to `SetHideTime(bool)`):

```rust
    SetLayout(crate::sim_frame::FrontDisplayLayout),
```

Add an `initial_layout` parameter to `Server::new` and initialise the field (place the param after `beep_test`):

```rust
    pub fn new(
        rx: mpsc::Receiver<ServerMessage>,
        initial: Vec<SerialStream>,
        hide_time: bool,
        beep_test: bool,
        initial_layout: crate::sim_frame::FrontDisplayLayout,
    ) -> Self {
        let mut server = Server {
            // ...existing fields...
            hide_time,
            beep_test,
            layout: initial_layout,
        };
        // ...existing serial-sender loop + return...
    }
```

- [ ] **Step 2: Handle `SetLayout` in the server message loop**

Find where `ServerMessage::SetHideTime` is handled in the server's `run`/recv loop and add, right beside it:

```rust
            ServerMessage::SetLayout(layout) => {
                self.layout = layout;
                // Re-encode so the next send to display clients reflects the
                // new layout immediately (live update).
                self.encode_flash();
                self.send_to_workers(true);
            }
```

(Match the surrounding style — if `SetHideTime` does not call `send_to_workers`, mirror exactly what it does; the important part is `self.layout = layout;` followed by `self.encode_flash();`. The next regular snapshot tick will also carry it.)

- [ ] **Step 3: Emit `SimFrame` on the binary path only**

In `encode_flash()`, replace the `TransmittedData { ... }.encode()` that fills `self.binary` with a `SimFrame` wrapping it:

```rust
    fn encode_flash(&mut self) {
        self.binary = if self.has_binary {
            Vec::from(
                crate::sim_frame::SimFrame {
                    layout: self.layout,
                    data: TransmittedData {
                        white_on_right: self.white_on_right,
                        flash: self.flash,
                        beep_test: self.beep_test,
                        brightness: self.brightness,
                        snapshot: self.snapshot.clone(),
                    },
                }
                .encode()
                .unwrap(),
            )
        } else {
            Vec::new()
        };
    }
```

**Do NOT change the serial worker** (`serial_worker_loop`) — it keeps building and sending bare `TransmittedData`. This is the whole point: hardware bytes are unchanged.

- [ ] **Step 4: Add `UpdateSender::set_layout`**

Mirror whatever method sends `SetHideTime` (e.g. a `set_hide_time`). Add:

```rust
    pub fn set_layout(&self, layout: crate::sim_frame::FrontDisplayLayout) {
        let _ = self.tx.try_send(ServerMessage::SetLayout(layout));
    }
```

(Use the same channel/handle and error-handling idiom as the existing `set_hide_time`.)

- [ ] **Step 5: Compile**

Run: `cargo check -p refbox`
Expected: fails only at the `Server::new(...)` call site (fixed in Task 7) — i.e. an arity error there. Everything in `update_sender.rs` itself compiles.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/update_sender.rs
git commit -m "feat(refbox): carry layout in display feed, leave serial format unchanged"
```

---

## Task 4: Simulator reads `SimFrame`; stores layout + latest snapshot

**Files:**
- Modify: `refbox/src/sim_app/mod.rs`

- [ ] **Step 1: Decode `SimFrame` in the listener**

In `snapshot_listener`, change the read buffer length and decode:

```rust
                let mut buffer = [0u8; crate::sim_frame::SimFrame::ENCODED_LEN];
```

and replace the `TransmittedData::decode(&buffer)` call with:

```rust
                let frame = match crate::sim_frame::SimFrame::decode(&buffer) {
                    Ok(val) => val,
                    Err(e) => {
                        warn!("Sim: Decoding error: {e:?}");
                        continue;
                    }
                };
                msg_tx.send(Message::NewSnapshot(frame)).await.unwrap();
```

- [ ] **Step 2: Change the `Message` payload + app state**

Change the message variant:

```rust
pub enum Message {
    NewSnapshot(crate::sim_frame::SimFrame),
    Stop,
}
```

Add fields to `SimRefBoxApp` to retain the chosen layout and the most recent payload (needed because the full-screen layouts render directly in `draw()` from the latest snapshot):

```rust
pub struct SimRefBoxApp {
    buffer: Rc<Mutex<DisplaySim>>,
    cache: Cache,
    layout: Rc<Mutex<crate::sim_frame::FrontDisplayLayout>>,
    latest: Rc<Mutex<Option<TransmittedData>>>,
}
```

Initialise both in `new()` (`layout: Rc::new(Mutex::new(FrontDisplayLayout::Default))`, `latest: Rc::new(Mutex::new(None))`).

- [ ] **Step 3: Update `update()`**

```rust
            Message::NewSnapshot(frame) => {
                let crate::sim_frame::SimFrame { layout, data } = frame;
                *self.layout.lock().unwrap() = layout;
                *self.latest.lock().unwrap() = Some(data.clone());

                let mut buffer = self.buffer.lock().unwrap();
                match *buffer {
                    DisplaySim::Sunlight(ref mut state) => {
                        (*state, _) = DisplayState::from_transmitted_data(&data);
                    }
                    DisplaySim::Matrix(ref mut buffer) => {
                        // Only the Default layout uses the matrix buffer; the
                        // full-screen layouts render from `latest` in draw().
                        if layout == crate::sim_frame::FrontDisplayLayout::Default {
                            buffer.clear_buffer();
                            draw_panels::<DisplayBuffer<WIDTH, HEIGHT>>(
                                &mut *buffer,
                                data.snapshot,
                                data.white_on_right,
                                data.flash,
                                data.beep_test,
                            )
                            .unwrap();
                        }
                    }
                }
                self.cache.clear();
                Task::none()
            }
```

- [ ] **Step 4: Dispatch in `draw()`**

In `Program::draw`, inside the `self.cache.draw(...)` closure, branch by layout for the non-sunlight case. Sunlight stays exactly as today. For the `Matrix`/non-sunlight branch:

```rust
                DisplaySim::Matrix(ref buffer) => {
                    let layout = *self.layout.lock().unwrap();
                    if layout == crate::sim_frame::FrontDisplayLayout::Default {
                        // ...existing matrix cell-painting loop, unchanged...
                    } else if let Some(ref data) = *self.latest.lock().unwrap() {
                        use crate::sim_app::scoreboard;
                        let bounds = frame.size();
                        match layout {
                            crate::sim_frame::FrontDisplayLayout::Classic => {
                                scoreboard::draw_classic(frame, bounds, data)
                            }
                            crate::sim_frame::FrontDisplayLayout::BigTime => {
                                scoreboard::draw_big_time(frame, bounds, data)
                            }
                            crate::sim_frame::FrontDisplayLayout::Corners => {
                                scoreboard::draw_corners(frame, bounds, data)
                            }
                            crate::sim_frame::FrontDisplayLayout::ScoresOnly => {
                                scoreboard::draw_scores_only(frame, bounds, data)
                            }
                            crate::sim_frame::FrontDisplayLayout::Default => unreachable!(),
                        }
                    }
                }
```

Add `mod scoreboard;` near the other `mod` lines in `sim_app/mod.rs` (the file is created in Task 6; until then this won't compile — acceptable mid-task, fully wired by Task 6).

- [ ] **Step 5: Commit** (after Task 6 makes it compile, or stub `scoreboard` now)

To keep this task independently compilable, create a stub `refbox/src/sim_app/scoreboard.rs` with empty bodies first:

```rust
use iced::{Size, widget::canvas::Frame};
use matrix_drawing::transmitted_data::TransmittedData;

pub fn draw_classic(_f: &mut Frame, _b: Size, _d: &TransmittedData) {}
pub fn draw_big_time(_f: &mut Frame, _b: Size, _d: &TransmittedData) {}
pub fn draw_corners(_f: &mut Frame, _b: Size, _d: &TransmittedData) {}
pub fn draw_scores_only(_f: &mut Frame, _b: Size, _d: &TransmittedData) {}
```

Run: `cargo check -p refbox` (still expect the `Server::new` arity error from Task 3 until Task 7).

```bash
git add refbox/src/sim_app/mod.rs refbox/src/sim_app/scoreboard.rs
git commit -m "feat(refbox): simulator decodes SimFrame and dispatches by layout"
```

---

## Task 5: Register the bundled Roboto font in the simulator process

**Files:**
- Modify: `refbox/src/main.rs`

The simulator is a separate process that returns early (the `if args.is_simulator { return iced::application(...) }` block ~lines 316-344) **before** the main app registers fonts. So register Roboto on the simulator's application builder.

- [ ] **Step 1: Add `.font(...)` to the simulator application builder**

In the `is_simulator` branch, add a `.font(...)` call to the builder chain before `.run_with(...)`:

```rust
        return iced::application(
            "Panel Simulator",
            sim_app::SimRefBoxApp::update,
            sim_app::SimRefBoxApp::view,
        )
        .subscription(sim_app::SimRefBoxApp::subscription)
        .font(include_bytes!("../resources/Roboto-Medium.ttf").as_slice())
        .window(window_settings)
        .style(sim_app::SimRefBoxApp::application_style)
        .run_with(|| sim_app::SimRefBoxApp::new(flags))
        .map_err(|e| e.into());
```

(The path `../resources/Roboto-Medium.ttf` is the same asset the main app bundles — confirm it exists; it is referenced at `refbox/src/main.rs` ~line 473.)

- [ ] **Step 2: Compile**

Run: `cargo check -p refbox` (still expect the Task 3 arity error).

- [ ] **Step 3: Commit**

```bash
git add refbox/src/main.rs
git commit -m "feat(refbox): register Roboto font in the simulator process"
```

---

## Task 6: Implement the four full-screen draw routines

**Files:**
- Modify: `refbox/src/sim_app/scoreboard.rs` (replace the Task-4 stub)

**Drawing model:** Each routine receives `frame: &mut Frame`, `bounds: Size` (the window size), and `data: &TransmittedData`. Position every element as a fraction of `bounds.width` / `bounds.height` so it scales to any window/fullscreen. Use `frame.fill_rectangle(Point, Size, Fill)` for boxes and `frame.fill_text(Text)` for text (see `sunlight_display::static_text` for the `Text {...}` construction pattern: set `font: Font::with_name("Roboto")`, `size`, `color`, `position`, and `horizontal_alignment`/`vertical_alignment`).

**Data mapping (all layouts):**
- Scores: `data.snapshot.scores.black`, `data.snapshot.scores.white` (`u8`).
- Clock: `matrix_drawing::secs_to_time_string(data.snapshot.secs_in_period)` → `MM:SS`. During a timeout, use the timeout's seconds instead (see below).
- Period name: `data.snapshot.current_period` → `to_string().to_uppercase()` (via `GamePeriod`'s `Display`). Example: `Second Half` → `SECOND HALF`.
- Period colour: green for `FirstHalf | SecondHalf | OvertimeFirstHalf | OvertimeSecondHalf`; red for `SuddenDeath`; yellow for the rest (breaks/pre-game).
- Sides: `data.white_on_right == true` ⇒ WHITE on the right, BLACK on the left; else the reverse.
- Timeout: `match data.snapshot.timeout { Some(t) => ... }`. `TimeoutSnapshot::White(s)|Black(s)|Ref(s)|PenaltyShot(s)` — `s` is the remaining seconds; show `secs_to_time_string(s)` in the clock area and the type label (`WHITE T/O` / `BLACK T/O` / `REF T/O` / `PENALTY SHOT`) in the period-pill slot. `None` ⇒ show the period name + game clock.

- [ ] **Step 1: Shared helpers + colours**

```rust
use iced::{
    Color, Point, Size,
    alignment::{Horizontal, Vertical},
    widget::canvas::{Fill, Frame, Text},
};
use iced::widget::text::{LineHeight, Shaping};
use iced::Font;
use matrix_drawing::{secs_to_time_string, transmitted_data::TransmittedData};
use uwh_common::game_snapshot::{GamePeriod, TimeoutSnapshot};

const BG: Color = Color::BLACK;
const CLOCK_YELLOW: Color = Color::from_rgb(1.0, 1.0, 0.0);
const WHITE_BOX: Color = Color::WHITE;
const BLACK_BOX: Color = Color::from_rgb(0.05, 0.05, 0.05);
const BLACK_BOX_OUTLINE: Color = Color::from_rgb(0.5, 0.5, 0.5);
const GREEN: Color = Color::from_rgb(0.0, 0.6, 0.0);
const YELLOW: Color = Color::from_rgb(0.8, 0.7, 0.0);
const RED: Color = Color::from_rgb(0.8, 0.0, 0.0);

fn period_color(p: GamePeriod) -> Color {
    match p {
        GamePeriod::FirstHalf
        | GamePeriod::SecondHalf
        | GamePeriod::OvertimeFirstHalf
        | GamePeriod::OvertimeSecondHalf => GREEN,
        GamePeriod::SuddenDeath => RED,
        _ => YELLOW,
    }
}

/// (badge text, badge colour) for the centre pill, accounting for timeouts.
fn badge(data: &TransmittedData) -> (String, Color) {
    match data.snapshot.timeout {
        Some(TimeoutSnapshot::White(_)) => ("WHITE T/O".to_string(), YELLOW),
        Some(TimeoutSnapshot::Black(_)) => ("BLACK T/O".to_string(), YELLOW),
        Some(TimeoutSnapshot::Ref(_)) => ("REF T/O".to_string(), YELLOW),
        Some(TimeoutSnapshot::PenaltyShot(_)) => ("PENALTY SHOT".to_string(), RED),
        None => (
            data.snapshot.current_period.to_string().to_uppercase(),
            period_color(data.snapshot.current_period),
        ),
    }
}

/// The clock string to show: timeout countdown if a timeout is running,
/// otherwise the period clock.
fn clock_string(data: &TransmittedData) -> String {
    let secs = match data.snapshot.timeout {
        Some(TimeoutSnapshot::White(s))
        | Some(TimeoutSnapshot::Black(s))
        | Some(TimeoutSnapshot::Ref(s))
        | Some(TimeoutSnapshot::PenaltyShot(s)) => s,
        None => data.snapshot.secs_in_period,
    };
    secs_to_time_string(secs).trim().to_string()
}

fn text(content: String, x: f32, y: f32, size: f32, color: Color, h: Horizontal) -> Text {
    Text {
        content,
        position: Point::new(x, y),
        color,
        size: size.into(),
        line_height: LineHeight::Relative(1.0),
        font: Font::with_name("Roboto"),
        horizontal_alignment: h,
        vertical_alignment: Vertical::Center,
        shaping: Shaping::Basic,
    }
}
```

- [ ] **Step 2: Implement `draw_classic` (the fully-worked reference layout)**

```rust
pub fn draw_classic(frame: &mut Frame, bounds: Size, data: &TransmittedData) {
    let (w, h) = (bounds.width, bounds.height);
    frame.fill_rectangle(Point::ORIGIN, bounds, Fill::from(BG));

    let white_left = !data.white_on_right;
    let box_w = w * 0.22;
    let box_h = h * 0.55;
    let box_y = h * 0.28;
    let left_x = w * 0.04;
    let right_x = w - box_w - w * 0.04;

    let draw_team = |frame: &mut Frame, x: f32, label: &str, score: u8, is_white: bool| {
        // label
        frame.fill_text(text(
            label.to_string(),
            x + box_w / 2.0,
            box_y - h * 0.06,
            h * 0.07,
            Color::WHITE,
            Horizontal::Center,
        ));
        // box
        let fill = if is_white { WHITE_BOX } else { BLACK_BOX };
        frame.fill_rectangle(Point::new(x, box_y), Size::new(box_w, box_h), Fill::from(fill));
        if !is_white {
            // thin outline so the black box reads on the black background
            let t = 3.0;
            frame.fill_rectangle(Point::new(x, box_y), Size::new(box_w, t), Fill::from(BLACK_BOX_OUTLINE));
            frame.fill_rectangle(Point::new(x, box_y + box_h - t), Size::new(box_w, t), Fill::from(BLACK_BOX_OUTLINE));
            frame.fill_rectangle(Point::new(x, box_y), Size::new(t, box_h), Fill::from(BLACK_BOX_OUTLINE));
            frame.fill_rectangle(Point::new(x + box_w - t, box_y), Size::new(t, box_h), Fill::from(BLACK_BOX_OUTLINE));
        }
        // score
        let score_color = if is_white { Color::BLACK } else { Color::WHITE };
        frame.fill_text(text(
            score.to_string(),
            x + box_w / 2.0,
            box_y + box_h / 2.0,
            box_h * 0.8,
            score_color,
            Horizontal::Center,
        ));
    };

    if white_left {
        draw_team(frame, left_x, "WHITE", data.snapshot.scores.white, true);
        draw_team(frame, right_x, "BLACK", data.snapshot.scores.black, false);
    } else {
        draw_team(frame, left_x, "BLACK", data.snapshot.scores.black, false);
        draw_team(frame, right_x, "WHITE", data.snapshot.scores.white, true);
    }

    // centre: badge then clock
    let (badge_text, badge_color) = badge(data);
    let cx = w / 2.0;
    let badge_w = w * 0.30;
    let badge_h = h * 0.12;
    frame.fill_rectangle(
        Point::new(cx - badge_w / 2.0, box_y - h * 0.02),
        Size::new(badge_w, badge_h),
        Fill::from(badge_color),
    );
    frame.fill_text(text(
        badge_text,
        cx,
        box_y - h * 0.02 + badge_h / 2.0,
        badge_h * 0.5,
        Color::WHITE,
        Horizontal::Center,
    ));
    frame.fill_text(text(
        clock_string(data),
        cx,
        h * 0.62,
        h * 0.34,
        CLOCK_YELLOW,
        Horizontal::Center,
    ));
}
```

- [ ] **Step 3: Implement the other three (same helpers, different element specs)**

- `draw_big_time`: fill background; draw only `clock_string(data)` centred at `(w/2, h/2)`, size ≈ `h * 0.6`, colour `CLOCK_YELLOW`. If a timeout is running, draw the badge text (small, ≈ `h*0.08`) centred at `(w/2, h*0.18)` in `badge` colour above the clock.
- `draw_corners`: fill background; small team blocks (label + score box, box ≈ `w*0.14` × `h*0.30`) in the top-left (`x=w*0.03`, `y=h*0.06`) and top-right (`x=w-box_w-w*0.03`); badge pill centred at ≈ `(w/2, h*0.30)`; clock centred at ≈ `(w/2, h*0.66)`, size ≈ `h*0.5`. Sides follow `white_on_right`.
- `draw_scores_only`: fill background; two large team blocks (label + box ≈ `w*0.26` × `h*0.5`, vertically centred), left at `x=w*0.08`, right at `x=w-box_w-w*0.08`; no badge, no clock. Sides follow `white_on_right`.

Each reuses `draw_team`-style logic; factor the team-block drawing into a shared `fn draw_team_block(frame, x, y, box_w, box_h, label, score, is_white, label_size, score_size)` to keep it DRY across Classic/Corners/Scores Only.

- [ ] **Step 4: Compile + lint**

Run: `cargo check -p refbox` (still expect Task 3 arity error until Task 7) then, after Task 7, `cargo clippy -p refbox -- -D warnings`.
Expected: no warnings in `scoreboard.rs`.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/sim_app/scoreboard.rs
git commit -m "feat(refbox): draw Classic/BigTime/Corners/ScoresOnly scoreboards"
```

---

## Task 7: Message, handler, and initial-layout wiring

**Files:**
- Modify: `refbox/src/app/message.rs`, `refbox/src/app/mod.rs`

- [ ] **Step 1: Add the message variant** (`message.rs`, next to `CycleDisplayMode`)

```rust
    /// Advance the front-display layout (Default → Classic → … ). Commits
    /// immediately, like CycleDisplayMode; not part of the Apply round-trip.
    CycleFrontDisplayLayout,
```

- [ ] **Step 2: Handle it in `update()`** (`mod.rs`, next to the `CycleDisplayMode` arm)

```rust
            Message::CycleFrontDisplayLayout => {
                let next = self.config.front_display_layout.next();
                self.config.front_display_layout = next;
                self.update_sender.set_layout(next);
                self.persist_config();
                Task::none()
            }
```

- [ ] **Step 3: Pass the initial layout when constructing the server**

Find where `Server::new(...)` (or the `UpdateSender`/server setup) is constructed in `mod.rs` (the call that already passes `hide_time` and `beep_test`). Add the initial layout as the new final argument:

```rust
            if has_led_panel {
                crate::sim_frame::FrontDisplayLayout::Default
            } else {
                config.front_display_layout
            },
```

This forces `Default` when a physical panel is connected (matching the greyed button), and otherwise honours the saved choice.

- [ ] **Step 4: Compile**

Run: `cargo check -p refbox`
Expected: clean now (the Task 3 arity error is resolved).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): cycle front-display layout and seed initial layout"
```

---

## Task 8: Display Options button + hardware gating

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`

`make_display_config_page` already receives `has_led_panel: bool`. Add the layout value button to the row that holds `open-new-display`.

- [ ] **Step 1: Build the layout label + button**

Inside `make_display_config_page`, before the final `column![...]`, compute the label and gating:

```rust
    use crate::sim_frame::FrontDisplayLayout;
    let effective_layout = if has_led_panel {
        FrontDisplayLayout::Default
    } else {
        settings_front_display_layout // see Step 2 for where this comes from
    };
    let layout_label = match effective_layout {
        FrontDisplayLayout::Default => fl!("layout-default"),
        FrontDisplayLayout::Classic => fl!("layout-classic"),
        FrontDisplayLayout::BigTime => fl!("layout-big-time"),
        FrontDisplayLayout::Corners => fl!("layout-corners"),
        FrontDisplayLayout::ScoresOnly => fl!("layout-scores-only"),
    };
    let layout_button = make_value_button(
        fl!("front-display-layout"),
        layout_label,
        (false, true),
        if has_led_panel {
            None // greyed out: locked to Default while a real panel is connected
        } else {
            Some(Message::CycleFrontDisplayLayout)
        },
    );
```

- [ ] **Step 2: Source the current layout**

The layout lives in `Config`, not `EditableSettings` (it commits immediately, like display mode). The view builder reads the live config value. Mirror exactly how `display_mode()` is read in `make_user_config_page` (it calls the free function `display_mode()`): if the page has access to the app config, read `config.front_display_layout`; otherwise add a small accessor the same way display mode does. The cleanest match: pass the current `FrontDisplayLayout` into `make_display_config_page` from the caller (the caller already has `&self.config`), as a new parameter `front_display_layout: FrontDisplayLayout`, and use it for `settings_front_display_layout` above. Update the one call site of `make_display_config_page` accordingly.

- [ ] **Step 3: Place the button**

Add `layout_button` into the existing `row![ ... open-new-display ... ]` (replace the trailing `horizontal_space()` with the button, or add a dedicated row above it):

```rust
        row![
            layout_button,
            {
                let btn = make_button(fl!("open-new-display")).style(light_gray_button);
                if has_led_panel { btn } else { btn.on_press(Message::OpenNewDisplay) }
            },
        ]
        .spacing(SPACING)
        .height(Length::Fill),
```

- [ ] **Step 4: Compile + lint**

Run: `cargo check -p refbox && cargo clippy -p refbox -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): add Display Layout picker button with panel gating"
```

---

## Task 9: Translations

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl`, `refbox/translations/es/refbox.ftl`, `refbox/translations/fr/refbox.ftl`

- [ ] **Step 1: Add the keys to en-US** (near the `view-mode` / `display-mode-*` block)

```ftl
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
```

- [ ] **Step 2: Add the same keys to es and fr**

Add the identical six keys to `es/refbox.ftl` and `fr/refbox.ftl`. Per the existing pattern, the `display-mode-*` strings are currently English placeholders in es/fr; mirror that here (English values) so the keys resolve and the build's translation-completeness check passes. Flag in the PR description that proper es/fr wording is a follow-up, consistent with the existing display-mode strings.

- [ ] **Step 3: Verify no missing-key panics**

Run: `cargo test -p refbox`
Expected: PASS (the Fluent loader resolves all `fl!` keys used in Task 8).

- [ ] **Step 4: Commit**

```bash
git add refbox/translations/en-US/refbox.ftl refbox/translations/es/refbox.ftl refbox/translations/fr/refbox.ftl
git commit -m "feat(refbox): add translations for front-display layout picker"
```

---

## Task 10: Full verification + visual confirmation

**Files:** none (verification only)

- [ ] **Step 1: Tests + lint + format**

Run: `cargo test -p refbox`
Run: `just check`
Expected: all green (fmt, clippy `-D warnings`, tests, audit).

- [ ] **Step 2: Manual walkthrough (no LED panel)**

Launch (per project run convention — background, `dangerouslyDisableSandbox`, `WAYLAND_DISPLAY=` unset on WSL):
`WAYLAND_DISPLAY= cargo run -p refbox`

Verify against the spec's acceptance criteria:
- Display Options shows **DISPLAY LAYOUT** cycling `DEFAULT → CLASSIC → BIG TIME → CORNERS → SCORES ONLY → DEFAULT`.
- Selecting each changes the open display window **immediately**.
- Scores/clock/period update live; WHITE/BLACK sides follow the white-on-right setting.
- Start a timeout → Classic/Corners/Big Time show the countdown + type label; Scores Only unchanged.
- Restart the app → the last-selected layout is restored.
- `DEFAULT` is identical to today's panel mirror.

- [ ] **Step 3: Manual walkthrough (LED panel connected)**

Launch with a serial port (`--serial-port <port>`). Verify the DISPLAY LAYOUT button is greyed and shows DEFAULT, and the panel renders exactly as before.

- [ ] **Step 4: Browser visual confirmation**

With the user's explicit permission, use the visual companion for a final side-by-side of Classic/Big Time/Corners/Scores Only against the web reference, and tune coordinates in `scoreboard.rs` if needed.

- [ ] **Step 5: Code review + PR**

Run `superpowers:requesting-code-review` once, then prepare the PR (do not open without the user's go-ahead). PR body per `.claude/rules/pr-review.md` (What changed / Why / Scope / How to verify), noting: refbox-only, serial/hardware format unchanged, es/fr strings are English placeholders pending translation.

---

## Self-Review

**Spec coverage:**
- Display Layout button + 5 choices → Tasks 1, 8. ✓
- Live update via display feed, serial untouched → Tasks 2, 3, 4. ✓
- Hardware gating (greyed + forced Default) → Tasks 7 (seed), 8 (button). ✓
- Persistence + default Default → Task 1. ✓
- Four layouts + colours/labels → Task 6. ✓
- Timeout behaviour → Task 6 (`badge`/`clock_string`). ✓
- WHITE/BLACK only, sides follow white-on-right → Task 6. ✓
- Translations → Task 9. ✓
- Tests (frame round-trip, serial-unchanged proof, cycle order, u8 round-trip) → Tasks 1, 2. ✓
- `cargo test -p refbox` / `just check` → Task 10. ✓

**Placeholder scan:** Task 6 intentionally fully-works one layout (Classic) and gives precise element specs for the other three, because exact pixel positions are visual/iterative (justified by the project's lean-plan rule and confirmed via Task 10 Step 4). The Task-4 `scoreboard` stub is replaced in Task 6. The `Server::new` call-site arity error is expected between Tasks 3 and 7 and explicitly resolved in Task 7 Step 3. No unresolved TBDs.

**Type consistency:** `FrontDisplayLayout` (`next`, `to_u8`, `from_u8`), `SimFrame` (`ENCODED_LEN`, `encode`, `decode`), `ServerMessage::SetLayout`, `UpdateSender::set_layout`, `Message::CycleFrontDisplayLayout`, `config.front_display_layout`, and `scoreboard::draw_*` signatures are used consistently across tasks.

## Deviations

_(none yet)_

//! Tracks the single cursor position that iced exposes to widgets.
//!
//! iced writes both mouse movements and touch positions into one cursor, so the
//! two input methods can overwrite each other. This type owns that decision so
//! it can be reasoned about and tested on its own.

use winit::dpi::PhysicalPosition;
use winit::event::{Touch, WindowEvent};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CursorTracker {
    position: Option<PhysicalPosition<f64>>,
    /// Whether touch, rather than the mouse, last owned the cursor.
    touch_is_current: bool,
    /// Whether the next `moved`, if any, is the synthetic one that winit emits
    /// directly after a pointer enter, and must therefore be ignored. Nothing
    /// else clears this once armed — not even a later `touched()`: an
    /// `entered()` with no following `moved()` leaves it armed, and the first
    /// move of a later, unrelated mouse interaction is then dropped. That is
    /// reachable on Windows, where winit emits `CursorEntered` but skips the
    /// following `CursorMoved` when the position has not changed. It is kept
    /// this way deliberately: the cost is one dropped mouse move and the move
    /// after it lands normally, so it self-heals. Pinned by
    /// `a_later_touch_does_not_disarm_the_suppression`.
    suppress_next_moved: bool,
}

impl CursorTracker {
    /// The position widgets are evaluated against, if the cursor is available.
    pub(crate) fn position(&self) -> Option<PhysicalPosition<f64>> {
        self.position
    }

    /// Applies a window event to the cursor, ignoring any event that does not
    /// concern it.
    ///
    /// This mapping lives here, rather than in `state.rs`'s event `match`, so
    /// that it is covered by the tests below. It is the fix's wiring, and it is
    /// easy to lose: upstream `iced_winit` handles `CursorMoved` and `Touch` in
    /// ONE shared arm and has no `CursorEntered` arm at all, so a re-vendor that
    /// reinstates upstream's arm shape would leave every method below intact and
    /// the fix inert.
    pub(crate) fn handle(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.moved(*position);
            }
            WindowEvent::Touch(Touch { location, .. }) => {
                self.touched(*location);
            }
            WindowEvent::CursorEntered { .. } => {
                self.entered();
            }
            WindowEvent::CursorLeft { .. } => {
                self.left();
            }
            _ => {}
        }
    }

    /// A finger landed, moved, or lifted at `location`.
    pub(crate) fn touched(&mut self, location: PhysicalPosition<f64>) {
        self.position = Some(location);
        self.touch_is_current = true;
    }

    /// The mouse pointer entered the window.
    ///
    /// Wayland compositors hide the cursor while a finger is on the screen and
    /// restore it the moment the finger lifts. winit turns that restore into a
    /// `CursorEntered` immediately followed by a `CursorMoved` at wherever the
    /// physical pointer was parked. Unfiltered, that move overwrites the touch
    /// position before the widget tree sees the finger lift, and the tap is
    /// discarded — so suppress exactly that one move.
    pub(crate) fn entered(&mut self) {
        self.suppress_next_moved = self.touch_is_current;
    }

    /// The mouse pointer moved to `position`.
    pub(crate) fn moved(&mut self, position: PhysicalPosition<f64>) {
        if self.suppress_next_moved {
            self.suppress_next_moved = false;
        } else {
            self.position = Some(position);
            self.touch_is_current = false;
        }
    }

    /// The mouse pointer left the window.
    ///
    /// Ignored while touch owns the cursor: a compositor hiding the pointer
    /// around a touch must not blank a position a finger just set.
    pub(crate) fn left(&mut self) {
        if !self.touch_is_current {
            self.position = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use winit::event::{DeviceId, TouchPhase};

    fn at(x: f64, y: f64) -> PhysicalPosition<f64> {
        PhysicalPosition::new(x, y)
    }

    // `DeviceId::dummy()` is public and documented by winit for exactly this
    // purpose, so every event the tracker cares about is constructible here.
    fn moved_event(x: f64, y: f64) -> WindowEvent {
        WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: at(x, y),
        }
    }

    fn touch_event(phase: TouchPhase, x: f64, y: f64) -> WindowEvent {
        WindowEvent::Touch(Touch {
            device_id: DeviceId::dummy(),
            phase,
            location: at(x, y),
            force: None,
            id: 0,
        })
    }

    fn entered_event() -> WindowEvent {
        WindowEvent::CursorEntered {
            device_id: DeviceId::dummy(),
        }
    }

    fn left_event() -> WindowEvent {
        WindowEvent::CursorLeft {
            device_id: DeviceId::dummy(),
        }
    }

    #[test]
    fn starts_with_no_position() {
        assert_eq!(CursorTracker::default().position(), None);
    }

    #[test]
    fn a_touch_sets_the_position_to_the_finger() {
        let mut c = CursorTracker::default();
        c.touched(at(530.0, 190.0));
        assert_eq!(c.position(), Some(at(530.0, 190.0)));
    }

    #[test]
    fn a_mouse_move_sets_the_position() {
        let mut c = CursorTracker::default();
        c.moved(at(12.0, 34.0));
        assert_eq!(c.position(), Some(at(12.0, 34.0)));
    }

    #[test]
    fn a_pointer_leave_clears_the_position() {
        let mut c = CursorTracker::default();
        c.moved(at(12.0, 34.0));
        c.left();
        assert_eq!(c.position(), None);
    }

    // ---- Regression tests --------------------------------------------------
    //
    // These pin down the fix for the dropped-tap bug: a mouse pointer restored
    // by the compositor after a finger lifts must not be able to overwrite or
    // blank the touch position it left behind.

    /// Replays the exact Wayland sequence captured on field Pi uwh-refbox-006
    /// on 2026-08-26 for a single tap at (530, 190). The capture carries two
    /// `wl_pointer` objects (`#23` and `#53`); Sway hides the cursor when the
    /// finger lands and restores it 0.04 ms after the finger lifts, sending a
    /// pointer enter for EACH of the two objects at the physical pointer's
    /// parked position of (100, 100) — two back-to-back `entered`/`moved`
    /// pairs, both replayed below. Neither restore must become the cursor, or
    /// the widget tree evaluates the finger-lift against (100, 100) and
    /// silently discards the tap.
    #[test]
    fn a_restored_pointer_cannot_overwrite_a_touch() {
        let mut c = CursorTracker::default();

        c.left();                    // wl_pointer.leave, cursor hidden for touch
        c.touched(at(530.0, 190.125));   // wl_touch.down
        c.touched(at(529.0, 190.125));   // wl_touch.motion
        c.touched(at(529.0, 191.625));   // wl_touch.motion
        c.touched(at(529.0, 191.625));   // wl_touch.up
        c.entered();                 // wl_pointer#23.enter -> CursorEntered
        c.moved(at(100.0, 100.0));   // ...immediately followed by CursorMoved
        c.entered();                 // wl_pointer#53.enter -> CursorEntered
        c.moved(at(100.0, 100.0));   // ...and its CursorMoved

        assert_eq!(
            c.position(),
            Some(at(529.0, 191.625)),
            "the restored pointer overwrote the touch position"
        );
    }

    #[test]
    fn only_the_first_move_after_an_enter_is_suppressed() {
        let mut c = CursorTracker::default();

        c.touched(at(530.0, 190.0));
        c.entered();
        c.moved(at(100.0, 100.0)); // synthetic, suppressed
        c.moved(at(101.0, 102.0)); // the user really is moving the mouse now

        assert_eq!(c.position(), Some(at(101.0, 102.0)));
    }

    #[test]
    fn a_pointer_enter_with_no_preceding_touch_is_honoured() {
        let mut c = CursorTracker::default();

        c.entered();
        c.moved(at(100.0, 100.0));

        assert_eq!(c.position(), Some(at(100.0, 100.0)));
    }

    #[test]
    fn a_pointer_leave_cannot_blank_a_touch_position() {
        let mut c = CursorTracker::default();

        c.touched(at(530.0, 190.0));
        c.left();

        assert_eq!(c.position(), Some(at(530.0, 190.0)));
    }

    #[test]
    fn a_mouse_move_reclaims_the_cursor_from_touch() {
        let mut c = CursorTracker::default();

        c.touched(at(530.0, 190.0));
        c.moved(at(10.0, 10.0)); // no enter first: a genuine mouse move
        c.left();

        assert_eq!(c.position(), None, "mouse should own the cursor again");
    }

    #[test]
    fn a_pointer_enter_alone_never_moves_the_cursor() {
        let mut c = CursorTracker::default();

        c.touched(at(530.0, 190.0));
        c.entered();

        assert_eq!(c.position(), Some(at(530.0, 190.0)));
    }

    /// `touched()` deliberately does NOT clear an armed suppression, so a tap
    /// arriving between a pointer enter and its synthetic move leaves the flag
    /// armed. Kept that way on purpose — see the field's doc comment — and
    /// pinned here so the choice stays deliberate rather than accidental.
    #[test]
    fn a_later_touch_does_not_disarm_the_suppression() {
        let mut c = CursorTracker::default();

        c.touched(at(530.0, 190.0));
        c.entered();                 // arms the suppression
        c.touched(at(531.0, 191.0)); // a fresh touch leaves it armed...
        c.moved(at(100.0, 100.0));   // ...so this move is still swallowed

        assert_eq!(c.position(), Some(at(531.0, 191.0)));
    }

    // ---- Event mapping -----------------------------------------------------
    //
    // Everything above drives the tracker's methods directly. These drive it
    // through `handle`, the mapping from the `WindowEvent`s winit actually
    // delivers, because that mapping is as load-bearing as the logic and much
    // easier to lose: upstream `iced_winit` handles `CursorMoved` and `Touch`
    // in one shared arm and has no `CursorEntered` arm at all. Without these
    // tests, a re-vendor that restored upstream's arm shape would leave the
    // whole fix inert with a fully green suite.

    #[test]
    fn a_cursor_moved_event_reaches_moved() {
        let mut c = CursorTracker::default();

        c.handle(&moved_event(12.0, 34.0));

        assert_eq!(c.position(), Some(at(12.0, 34.0)));
    }

    #[test]
    fn a_touch_event_reaches_touched() {
        let mut c = CursorTracker::default();

        c.handle(&touch_event(TouchPhase::Started, 530.0, 190.0));
        // A leave that follows must be ignored, which only happens if the
        // event reached `touched()` rather than `moved()`.
        c.handle(&left_event());

        assert_eq!(c.position(), Some(at(530.0, 190.0)));
    }

    #[test]
    fn a_cursor_entered_event_reaches_entered() {
        let mut c = CursorTracker::default();

        c.handle(&touch_event(TouchPhase::Ended, 530.0, 190.0));
        c.handle(&entered_event());
        c.handle(&moved_event(100.0, 100.0));

        assert_eq!(
            c.position(),
            Some(at(530.0, 190.0)),
            "CursorEntered never reached entered(), so the restored pointer won"
        );
    }

    #[test]
    fn a_cursor_left_event_reaches_left() {
        let mut c = CursorTracker::default();

        c.handle(&moved_event(12.0, 34.0));
        c.handle(&left_event());

        assert_eq!(c.position(), None);
    }

    #[test]
    fn an_event_the_cursor_does_not_care_about_changes_nothing() {
        let mut c = CursorTracker::default();
        c.handle(&moved_event(12.0, 34.0));
        let before = c;

        c.handle(&WindowEvent::Focused(true));

        assert_eq!(c, before);
    }

    /// The same captured tap as `a_restored_pointer_cannot_overwrite_a_touch`,
    /// replayed as the `WindowEvent`s winit delivers instead of as direct method
    /// calls, so the end-to-end path is pinned and not just the helpers. Both
    /// pointer leaves and both pointer restores from the capture are included.
    #[test]
    fn the_captured_tap_survives_when_replayed_as_window_events() {
        let mut c = CursorTracker::default();

        for event in [
            left_event(),                                     // #53.leave
            left_event(),                                     // #23.leave
            touch_event(TouchPhase::Started, 530.0, 190.125), // wl_touch.down
            touch_event(TouchPhase::Moved, 529.0, 190.125),   // wl_touch.motion
            touch_event(TouchPhase::Moved, 529.0, 191.625),   // wl_touch.motion
            touch_event(TouchPhase::Ended, 529.0, 191.625),   // wl_touch.up
            entered_event(),                                  // #23.enter
            moved_event(100.0, 100.0),                        // ...and its move
            entered_event(),                                  // #53.enter
            moved_event(100.0, 100.0),                        // ...and its move
        ] {
            c.handle(&event);
        }

        assert_eq!(
            c.position(),
            Some(at(529.0, 191.625)),
            "the restored pointer overwrote the touch position"
        );
    }
}

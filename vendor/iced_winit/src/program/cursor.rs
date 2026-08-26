//! Tracks the single cursor position that iced exposes to widgets.
//!
//! iced writes both mouse movements and touch positions into one cursor, so the
//! two input methods can overwrite each other. This type owns that decision so
//! it can be reasoned about and tested on its own.

use winit::dpi::PhysicalPosition;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CursorTracker {
    position: Option<PhysicalPosition<f64>>,
    /// Whether touch, rather than the mouse, last owned the cursor.
    touch_is_current: bool,
    /// Whether the next `moved` is the synthetic one that winit emits directly
    /// after a pointer enter, and must therefore be ignored.
    suppress_next_moved: bool,
}

impl CursorTracker {
    /// The position widgets are evaluated against, if the cursor is available.
    pub(crate) fn position(&self) -> Option<PhysicalPosition<f64>> {
        self.position
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

    fn at(x: f64, y: f64) -> PhysicalPosition<f64> {
        PhysicalPosition::new(x, y)
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
    /// on 2026-08-26 for a single tap at (530, 190). Sway hides the cursor when
    /// the finger lands and restores it 0.04 ms after the finger lifts, sending
    /// a pointer enter at the physical pointer's parked position of (100, 100).
    /// That must NOT become the cursor, or the widget tree evaluates the
    /// finger-lift against (100, 100) and silently discards the tap.
    #[test]
    fn a_restored_pointer_cannot_overwrite_a_touch() {
        let mut c = CursorTracker::default();

        c.left();                    // wl_pointer.leave, cursor hidden for touch
        c.touched(at(530.0, 190.125));   // wl_touch.down
        c.touched(at(529.0, 190.125));   // wl_touch.motion
        c.touched(at(529.0, 191.625));   // wl_touch.motion
        c.touched(at(529.0, 191.625));   // wl_touch.up
        c.entered();                 // wl_pointer.enter -> CursorEntered
        c.moved(at(100.0, 100.0));   // ...immediately followed by CursorMoved

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
}

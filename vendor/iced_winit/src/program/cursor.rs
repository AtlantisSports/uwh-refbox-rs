//! Tracks the single cursor position that iced exposes to widgets.
//!
//! iced writes both mouse movements and touch positions into one cursor, so the
//! two input methods can overwrite each other. This type owns that decision so
//! it can be reasoned about and tested on its own.

use winit::dpi::PhysicalPosition;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CursorTracker {
    position: Option<PhysicalPosition<f64>>,
}

impl CursorTracker {
    /// The position widgets are evaluated against, if the cursor is available.
    pub(crate) fn position(&self) -> Option<PhysicalPosition<f64>> {
        self.position
    }

    /// A finger landed, moved, or lifted at `location`.
    pub(crate) fn touched(&mut self, location: PhysicalPosition<f64>) {
        self.position = Some(location);
    }

    /// The mouse pointer moved to `position`.
    pub(crate) fn moved(&mut self, position: PhysicalPosition<f64>) {
        self.position = Some(position);
    }

    /// The mouse pointer entered the window.
    pub(crate) fn entered(&mut self) {}

    /// The mouse pointer left the window.
    pub(crate) fn left(&mut self) {
        self.position = None;
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
}

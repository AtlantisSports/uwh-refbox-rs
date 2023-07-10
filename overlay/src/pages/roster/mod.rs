use super::{draw_texture_both, fit_text, Interpolate, Justify, PageRenderer};
use crate::State;
mod list;
mod picture;

impl PageRenderer {
    pub fn roster(&mut self, state: &State) {
        if state.snapshot.secs_in_period >= 169 {
            list::draw(self, state);
        } else if state.snapshot.secs_in_period <= 168 {
            picture::draw(self, state);
        }
    }
}

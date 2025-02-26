use super::{Interpolate, Justify, PageRenderer, draw_texture_both, fit_text};
use crate::State;
mod list;
mod picture;

impl PageRenderer {
    pub fn roster(&mut self, state: &State) {
        if state.snapshot.secs_in_period >= 169 {
            list::draw(self, state);
        } else if state.snapshot.secs_in_period < 169 {
            picture::draw(self, state);
        }
    }
}

use super::PageRenderer;
use macroquad::prelude::*;
impl PageRenderer {
    /// Display during overtime. Has no animations
    pub fn overtime_display(&mut self) {
        draw_texture(*self.textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(
            *self.textures.time_and_game_state_graphic(),
            0f32,
            0f32,
            WHITE,
        );
    }

    // Shown every time a goal is made for five seconds. A second each for fade in and out.
    // Must use a secondary animation counter because this is called along with other draw functions
    // pub fn show_goal_graphic(&mut self) {
    //     //animate fade for the first second
    //     let offset = if self.animation_counter < 1f32 {
    //         self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
    //         (0f32, 255f32).interpolate_linear(self.animation_counter)
    //     } else if self.animation_counter < 4f32 {
    //         self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period

    //         (0f32, 255f32).interpolate_linear(1f32)
    //     } else if self.animation_counter < 5f32 {
    //         //animate fade out in the last one second
    //         self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
    //         (0f32, 255f32).interpolate_linear(5f32 - self.animation_counter)
    //     } else {
    //         self.animation_counter = 0f32;
    //         0f32
    //     } as u8;
    //     draw_texture(
    //         *self.textures.team_white_graphic(),
    //         25f32,
    //         150f32,
    //         Color::from_rgba(255, 255, 255, offset),
    //     );
    // }
}

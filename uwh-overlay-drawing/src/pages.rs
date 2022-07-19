use crate::load_images::TexturesUWH;
use glium::uniform;
use glium::uniforms::{EmptyUniforms, UniformsStorage};

type UniformList<'a> = Vec<
    UniformsStorage<
        'a,
        &'a glium::texture::SrgbTexture2d,
        UniformsStorage<'a, [[f32; 4]; 4], EmptyUniforms>,
    >,
>;

pub fn roster(textures: &dyn TexturesUWH) -> UniformList {
    vec![
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.atlantis_logo_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.bottom_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 1.1, 0.0, 1.0f32],
            ],
            tex: textures.team_information_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.65, -0.5, 0.0, 1.0f32],
            ],
            tex: textures.team_black_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.65, -0.6, 0.0, 1.0f32],
            ],
            tex: textures.team_black_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-0.35, -0.5, 0.0, 1.0f32],
            ],
            tex: textures.team_white_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-0.35, -0.6, 0.0, 1.0f32],
            ],
            tex: textures.team_white_graphic(),
        },
    ]
}

pub fn next_game(textures: &dyn TexturesUWH) -> UniformList {
    vec![
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.atlantis_logo_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.bottom_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, -0.01, 0.0, 1.0f32],
            ],
            tex: textures.team_information_graphic(),
        },
    ]
}

pub fn final_scores(textures: &dyn TexturesUWH) -> UniformList {
    vec![
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.final_score_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.atlantis_logo_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, -0.01, 0.0, 1.0f32],
            ],
            tex: textures.team_information_graphic(),
        },
    ]
}

pub fn pre_game_display(textures: &dyn TexturesUWH) -> UniformList {
    vec![
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.atlantis_logo_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.bottom_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.team_bar_graphic(),
        },
        uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ -0.5, 0.0, 0.0, 1.0f32],
            ],
            tex: textures.time_and_game_state_graphic(),
        },
    ]
}

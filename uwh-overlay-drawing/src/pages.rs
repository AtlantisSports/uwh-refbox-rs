use glium::uniform;
use glium::uniforms::{EmptyUniforms, UniformsStorage};

type UniformList<'a> = Vec<
    UniformsStorage<
        'a,
        &'a glium::texture::SrgbTexture2d,
        UniformsStorage<'a, [[f32; 4]; 4], EmptyUniforms>,
    >,
>;

fn get_input() -> f32 {
    let mut buffer = String::new();
    println!(" Enter size: ");
    std::io::stdin().read_line(&mut buffer).expect("Failed");
    buffer.trim().parse::<f32>().unwrap()
}

/// stores all texture data
pub trait TexturesUWH {
    fn atlantis_logo_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn bottom_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn team_information_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn team_black_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn team_white_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn team_bar_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn time_and_game_state_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn final_score_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn get_text_color(&self) -> (f32, f32, f32, f32);
}

/// contains all the changing information (actual text, text color, text size and position) for drawing text
pub struct TextParams<'a> {
    pub matrix: [[f32; 4]; 4],
    pub text: &'a str,
    pub color: (f32, f32, f32, f32),
}

type TextList<'a> = Vec<TextParams<'a>>;

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

pub fn next_game(textures: &dyn TexturesUWH) -> (UniformList, TextList) {
    let x = get_input();
    (
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
        ],
        vec![TextParams {
            color: (0.0, 0.0, 0.0, 1.0),
            matrix: [
                [2.0 / x, 0.0, 0.0, 0.0],
                [0.0, 2.0 / x, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0 + (x - 1f32) * (1f32 / x), 0.0, 0.0, 1.0],
            ],
            text: "this is a test",
        }],
    )
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

use std::{io::Cursor, time};

pub struct Textures {
    pub atlantis_logo_graphic: glium::texture::SrgbTexture2d,
    pub bottom_graphic: glium::texture::SrgbTexture2d,
    pub team_information_graphic: glium::texture::SrgbTexture2d,
    pub team_black_graphic: glium::texture::SrgbTexture2d,
    pub team_white_graphic: glium::texture::SrgbTexture2d,
    pub team_bar_graphic: glium::texture::SrgbTexture2d,
    pub time_and_game_state_graphic: glium::texture::SrgbTexture2d,
    pub final_score_graphic: glium::texture::SrgbTexture2d,
    pub atlantis_logo_graphic_mask: glium::texture::SrgbTexture2d,
    pub bottom_graphic_mask: glium::texture::SrgbTexture2d,
    pub team_information_graphic_mask: glium::texture::SrgbTexture2d,
    pub team_black_graphic_mask: glium::texture::SrgbTexture2d,
    pub team_white_graphic_mask: glium::texture::SrgbTexture2d,
    pub team_bar_graphic_mask: glium::texture::SrgbTexture2d,
    pub time_and_game_state_graphic_mask: glium::texture::SrgbTexture2d,
    pub final_score_graphic_mask: glium::texture::SrgbTexture2d,
}

macro_rules! load {
    ($file:literal, $display:ident, $textures: ident) => {
        let image = image::load(Cursor::new(&include_bytes!($file)), image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        $textures.push(glium::texture::SrgbTexture2d::new($display, image).unwrap());
    };
}

impl Textures {
    pub fn init(display: &glium::Display) -> Self {
        let start = time::Instant::now();
        let mut textures = Vec::new();
        load!(
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Final Score Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Time and Game State Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Team Black Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Team White Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Team Bars Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Team Information Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Bottom Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Atlantis Logo.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Final Score Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Time and Game State Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Team Black Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Team White Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Team Bars Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Team Information Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Bottom Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Atlantis Logo.png",
            display,
            textures
        );
        println!(
            "Loaded images in: {} seconds",
            start.elapsed().as_secs_f32()
        );
        Self {
            atlantis_logo_graphic: textures.pop().unwrap(),
            bottom_graphic: textures.pop().unwrap(),
            team_information_graphic: textures.pop().unwrap(),
            team_black_graphic: textures.pop().unwrap(),
            team_white_graphic: textures.pop().unwrap(),
            team_bar_graphic: textures.pop().unwrap(),
            time_and_game_state_graphic: textures.pop().unwrap(),
            final_score_graphic: textures.pop().unwrap(),
            atlantis_logo_graphic_mask: textures.pop().unwrap(),
            bottom_graphic_mask: textures.pop().unwrap(),
            team_information_graphic_mask: textures.pop().unwrap(),
            team_black_graphic_mask: textures.pop().unwrap(),
            team_white_graphic_mask: textures.pop().unwrap(),
            team_bar_graphic_mask: textures.pop().unwrap(),
            time_and_game_state_graphic_mask: textures.pop().unwrap(),
            final_score_graphic_mask: textures.pop().unwrap(),
        }
    }
}

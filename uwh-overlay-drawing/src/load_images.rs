use std::io::Cursor;

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

macro_rules! textures_uwh_impl {
    ($structure:ty) => {
        impl TexturesUWH for $structure {
            /// Get a reference to the textures alpha's atlantis logo graphic.
            #[must_use]
            fn atlantis_logo_graphic(&self) -> &glium::texture::SrgbTexture2d {
                &self.atlantis_logo_graphic
            }

            /// Get a reference to the textures alpha's bottom graphic.
            #[must_use]
            fn bottom_graphic(&self) -> &glium::texture::SrgbTexture2d {
                &self.bottom_graphic
            }

            /// Get a reference to the textures alpha's team information graphic.
            #[must_use]
            fn team_information_graphic(&self) -> &glium::texture::SrgbTexture2d {
                &self.team_information_graphic
            }

            /// Get a reference to the textures alpha's team black graphic.
            #[must_use]
            fn team_black_graphic(&self) -> &glium::texture::SrgbTexture2d {
                &self.team_black_graphic
            }

            /// Get a reference to the textures alpha's team white graphic.
            #[must_use]
            fn team_white_graphic(&self) -> &glium::texture::SrgbTexture2d {
                &self.team_white_graphic
            }

            /// Get a reference to the textures alpha's team bar graphic.
            #[must_use]
            fn team_bar_graphic(&self) -> &glium::texture::SrgbTexture2d {
                &self.team_bar_graphic
            }

            /// Get a reference to the textures alpha's time and game state graphic.
            #[must_use]
            fn time_and_game_state_graphic(&self) -> &glium::texture::SrgbTexture2d {
                &self.time_and_game_state_graphic
            }

            /// Get a reference to the textures alpha's final score graphic.
            #[must_use]
            fn final_score_graphic(&self) -> &glium::texture::SrgbTexture2d {
                &self.final_score_graphic
            }
        }
    };
}

pub trait TexturesUWH {
    fn atlantis_logo_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn bottom_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn team_information_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn team_black_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn team_white_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn team_bar_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn time_and_game_state_graphic(&self) -> &glium::texture::SrgbTexture2d;
    fn final_score_graphic(&self) -> &glium::texture::SrgbTexture2d;
}

textures_uwh_impl!(TexturesColor);
pub struct TexturesColor {
    atlantis_logo_graphic: glium::texture::SrgbTexture2d,
    bottom_graphic: glium::texture::SrgbTexture2d,
    team_information_graphic: glium::texture::SrgbTexture2d,
    team_black_graphic: glium::texture::SrgbTexture2d,
    team_white_graphic: glium::texture::SrgbTexture2d,
    team_bar_graphic: glium::texture::SrgbTexture2d,
    time_and_game_state_graphic: glium::texture::SrgbTexture2d,
    final_score_graphic: glium::texture::SrgbTexture2d,
}

impl TexturesColor {
    pub fn init(display: &glium::Display) -> Self {
        let mut textures = Vec::new();
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
            "../../uwh-overlay-drawing/assets/color/1080/[PNG] 8K - Team Bars Graphic.png",
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
        Self {
            atlantis_logo_graphic: textures.pop().unwrap(),
            bottom_graphic: textures.pop().unwrap(),
            team_information_graphic: textures.pop().unwrap(),
            team_black_graphic: textures.pop().unwrap(),
            team_white_graphic: textures.pop().unwrap(),
            team_bar_graphic: textures.pop().unwrap(),
            time_and_game_state_graphic: textures.pop().unwrap(),
            final_score_graphic: textures.pop().unwrap(),
        }
    }
}

textures_uwh_impl!(TexturesAlpha);
pub struct TexturesAlpha {
    atlantis_logo_graphic: glium::texture::SrgbTexture2d,
    bottom_graphic: glium::texture::SrgbTexture2d,
    team_information_graphic: glium::texture::SrgbTexture2d,
    team_black_graphic: glium::texture::SrgbTexture2d,
    team_white_graphic: glium::texture::SrgbTexture2d,
    team_bar_graphic: glium::texture::SrgbTexture2d,
    time_and_game_state_graphic: glium::texture::SrgbTexture2d,
    final_score_graphic: glium::texture::SrgbTexture2d,
}

impl TexturesAlpha {
    pub fn init(display: &glium::Display) -> Self {
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
            "../../uwh-overlay-drawing/assets/alpha/1080/[PNG] 8K - Team Bars Graphic.png",
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
        Self {
            atlantis_logo_graphic: textures.pop().unwrap(),
            bottom_graphic: textures.pop().unwrap(),
            team_information_graphic: textures.pop().unwrap(),
            team_black_graphic: textures.pop().unwrap(),
            team_white_graphic: textures.pop().unwrap(),
            team_bar_graphic: textures.pop().unwrap(),
            time_and_game_state_graphic: textures.pop().unwrap(),
            final_score_graphic: textures.pop().unwrap(),
        }
    }
}

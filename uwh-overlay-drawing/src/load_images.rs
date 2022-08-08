use macroquad::prelude::*;

macro_rules! load {
    ($file:literal) => {
        Texture2D::from_file_with_format(include_bytes!($file), None)
    };
}

pub struct Textures {
    atlantis_logo_graphic: Texture2D,
    bottom_graphic: Texture2D,
    team_information_graphic: Texture2D,
    team_black_graphic: Texture2D,
    team_white_graphic: Texture2D,
    team_bar_graphic: Texture2D,
    time_and_game_state_graphic: Texture2D,
    final_score_graphic: Texture2D,
    font: Font,
}

impl Textures {
    pub fn init_color() -> Self {
        Self {
            final_score_graphic: load!("../assets/color/1080/[PNG] 8K - Final Score Graphic.png"),
            time_and_game_state_graphic: load!(
                "../assets/color/1080/[PNG] 8K - Time and Game State Graphic.png"
            ),
            team_bar_graphic: load!("../assets/color/1080/[PNG] 8K - Team Bars Graphic.png"),
            team_black_graphic: load!("../assets/color/1080/[PNG] 8K - Team Black Graphic.png"),
            team_white_graphic: load!("../assets/color/1080/[PNG] 8K - Team White Graphic.png"),
            team_information_graphic: load!(
                "../assets/color/1080/[PNG] 8K - Team Information Graphic.png"
            ),
            bottom_graphic: load!("../assets/color/1080/[PNG] 8K - Bottom Graphic.png"),
            atlantis_logo_graphic: load!("../assets/color/1080/[PNG] 8K - Atlantis Logo.png"),
            font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF")).unwrap(),
        }
    }

    pub fn init_alpha() -> Self {
        Self {
            final_score_graphic: load!("../assets/alpha/1080/[PNG] 8K - Final Score Graphic.png"),
            time_and_game_state_graphic: load!(
                "../assets/alpha/1080/[PNG] 8K - Time and Game State Graphic.png"
            ),
            team_bar_graphic: load!("../assets/alpha/1080/[PNG] 8K - Team Bars Graphic.png"),
            team_black_graphic: load!("../assets/alpha/1080/[PNG] 8K - Team Black Graphic.png"),
            team_white_graphic: load!("../assets/alpha/1080/[PNG] 8K - Team White Graphic.png"),
            team_information_graphic: load!(
                "../assets/alpha/1080/[PNG] 8K - Team Information Graphic.png"
            ),
            bottom_graphic: load!("../assets/alpha/1080/[PNG] 8K - Bottom Graphic.png"),
            atlantis_logo_graphic: load!("../assets/alpha/1080/[PNG] 8K - Atlantis Logo.png"),
            font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF")).unwrap(),
        }
    }

    /// Get a reference to the textures atlantis logo graphic.
    #[must_use]
    pub fn atlantis_logo_graphic(&self) -> &Texture2D {
        &self.atlantis_logo_graphic
    }

    /// Get a reference to the textures bottom graphic.
    #[must_use]
    pub fn bottom_graphic(&self) -> &Texture2D {
        &self.bottom_graphic
    }

    /// Get a reference to the textures team information graphic.
    #[must_use]
    pub fn team_information_graphic(&self) -> &Texture2D {
        &self.team_information_graphic
    }

    /// Get a reference to the textures team black graphic.
    #[must_use]
    pub fn team_black_graphic(&self) -> &Texture2D {
        &self.team_black_graphic
    }

    /// Get a reference to the textures team white graphic.
    #[must_use]
    pub fn team_white_graphic(&self) -> &Texture2D {
        &self.team_white_graphic
    }

    /// Get a reference to the textures team bar graphic.
    #[must_use]
    pub fn team_bar_graphic(&self) -> &Texture2D {
        &self.team_bar_graphic
    }

    /// Get a reference to the textures time and game state graphic.
    #[must_use]
    pub fn time_and_game_state_graphic(&self) -> &Texture2D {
        &self.time_and_game_state_graphic
    }

    /// Get a reference to the textures final score graphic.
    #[must_use]
    pub fn final_score_graphic(&self) -> &Texture2D {
        &self.final_score_graphic
    }

    /// Get the textures's font.
    #[must_use]
    pub fn font(&self) -> Font {
        self.font
    }
}

use macroquad::prelude::*;

macro_rules! load {
    ($file:literal) => {
        Texture2D::from_file_with_format(include_bytes!($file), None)
    };
}
pub(crate) use load;

pub struct Texture {
    pub alpha: Texture2D,
    pub color: Texture2D,
}

pub struct Textures {
    pub atlantis_logo_graphic: Texture,
    pub bottom_graphic: Texture,
    pub team_information_graphic: Texture,
    pub team_black_graphic: Texture,
    pub team_white_graphic: Texture,
    pub team_bar_graphic: Texture,
    pub time_and_game_state_graphic: Texture,
    pub final_score_graphic: Texture,
    pub in_game_mask: Texture,
    pub penalty_graphic: Texture,
    pub white_timout_graphic: Texture,
    pub black_timout_graphic: Texture,
    pub referee_timout_graphic: Texture,
    pub font: Font,
}

impl Default for Textures {
    fn default() -> Self {
        Self {
            font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF")).unwrap(),
            final_score_graphic: Texture {
                color: load!("../assets/color/1080/Final Score.png"),
                alpha: load!("../assets/alpha/1080/Final Score.png"),
            },
            time_and_game_state_graphic: Texture {
                color: load!("../assets/color/1080/Time and Game State.png"),
                alpha: load!("../assets/alpha/1080/Time and Game State.png"),
            },
            team_bar_graphic: Texture {
                color: load!("../assets/color/1080/Team Bars.png"),
                alpha: load!("../assets/alpha/1080/Team Bars.png"),
            },
            team_black_graphic: Texture {
                color: load!("../assets/color/1080/Team Black.png"),
                alpha: load!("../assets/alpha/1080/Team Black.png"),
            },
            team_white_graphic: Texture {
                color: load!("../assets/color/1080/Team White.png"),
                alpha: load!("../assets/alpha/1080/Team White.png"),
            },
            team_information_graphic: Texture {
                color: load!("../assets/color/1080/Team Information.png"),
                alpha: load!("../assets/alpha/1080/Team Information.png"),
            },
            bottom_graphic: Texture {
                color: load!("../assets/color/1080/Bottom.png"),
                alpha: load!("../assets/alpha/1080/Bottom.png"),
            },
            atlantis_logo_graphic: Texture {
                color: load!("../assets/color/1080/Atlantis Logo.png"),
                alpha: load!("../assets/alpha/1080/Atlantis Logo.png"),
            },
            in_game_mask: Texture {
                color: load!("../assets/alpha/1080/mask.png"),
                alpha: load!("../assets/alpha/1080/mask.png"),
            },
            penalty_graphic: Texture {
                color: load!("../assets/color/1080/Penalty Shot Flag.png"),
                alpha: load!("../assets/alpha/1080/Penalty Shot Flag.png"),
            },
            white_timout_graphic: Texture {
                color: load!("../assets/color/1080/White Timeout Flag.png"),
                alpha: load!("../assets/alpha/1080/White Timeout Flag.png"),
            },
            black_timout_graphic: Texture {
                color: load!("../assets/color/1080/Black Timeout Flag.png"),
                alpha: load!("../assets/alpha/1080/Black Timeout Flag.png"),
            },
            referee_timout_graphic: Texture {
                color: load!("../assets/color/1080/Referee Timeout Flag.png"),
                alpha: load!("../assets/alpha/1080/Referee Timeout Flag.png"),
            },
        }
    }
}

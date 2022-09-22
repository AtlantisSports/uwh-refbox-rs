use macroquad::prelude::*;

macro_rules! load {
    ($file:literal) => {
        Texture2D::from_file_with_format(include_bytes!($file), None)
    };
}
pub(crate) use load;

pub struct Textures {
    pub atlantis_logo_graphic: Texture2D,
    pub bottom_graphic: Texture2D,
    pub team_information_graphic: Texture2D,
    pub team_black_graphic: Texture2D,
    pub team_white_graphic: Texture2D,
    pub team_bar_graphic: Texture2D,
    pub time_and_game_state_graphic: Texture2D,
    pub final_score_graphic: Texture2D,
    pub in_game_mask: Texture2D,
    pub penalty_graphic: Texture2D,
    pub white_timout_graphic: Texture2D,
    pub black_timout_graphic: Texture2D,
    pub referee_timout_graphic: Texture2D,
    pub font: Font,
}

impl Textures {
    pub fn init_color() -> Self {
        Self {
            final_score_graphic: load!("../assets/color/1080/Final Score.png"),
            time_and_game_state_graphic: load!("../assets/color/1080/Time and Game State.png"),
            team_bar_graphic: load!("../assets/color/1080/Team Bars.png"),
            team_black_graphic: load!("../assets/color/1080/Team Black.png"),
            team_white_graphic: load!("../assets/color/1080/Team White.png"),
            team_information_graphic: load!("../assets/color/1080/Team Information.png"),
            bottom_graphic: load!("../assets/color/1080/Bottom.png"),
            atlantis_logo_graphic: load!("../assets/color/1080/Atlantis Logo.png"),
            in_game_mask: load!("../assets/alpha/1080/mask.png"),
            font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF")).unwrap(),
            penalty_graphic: load!("../assets/color/1080/Penalty Shot Flag.png"),
            white_timout_graphic: load!("../assets/color/1080/White Timeout Flag.png"),
            black_timout_graphic: load!("../assets/color/1080/Black Timeout Flag.png"),
            referee_timout_graphic: load!("../assets/color/1080/Referee Timeout Flag.png"),
        }
    }

    pub fn init_alpha() -> Self {
        Self {
            final_score_graphic: load!("../assets/alpha/1080/Final Score.png"),
            time_and_game_state_graphic: load!("../assets/alpha/1080/Time and Game State.png"),
            team_bar_graphic: load!("../assets/alpha/1080/Team Bars.png"),
            team_black_graphic: load!("../assets/alpha/1080/Team Black.png"),
            team_white_graphic: load!("../assets/alpha/1080/Team White.png"),
            team_information_graphic: load!("../assets/alpha/1080/Team Information.png"),
            in_game_mask: load!("../assets/alpha/1080/mask.png"),
            bottom_graphic: load!("../assets/alpha/1080/Bottom.png"),
            atlantis_logo_graphic: load!("../assets/alpha/1080/Atlantis Logo.png"),
            font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF")).unwrap(),
            penalty_graphic: load!("../assets/alpha/1080/Penalty Shot Flag.png"),
            white_timout_graphic: load!("../assets/alpha/1080/White Timeout Flag.png"),
            black_timout_graphic: load!("../assets/alpha/1080/Black Timeout Flag.png"),
            referee_timout_graphic: load!("../assets/alpha/1080/Referee Timeout Flag.png"),
        }
    }
}

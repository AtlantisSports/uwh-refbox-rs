use std::{fs::File, io::Read, path::Path};

use macroquad::prelude::*;

macro_rules! asset_load {
    ($file:literal) => {
        Texture {
            color: Texture2D::from_file_with_format(
                include_bytes!(concat!("../assets/color/1080/", $file)),
                None,
            ),
            alpha: Texture2D::from_file_with_format(
                include_bytes!(concat!("../assets/alpha/1080/", $file)),
                None,
            ),
        }
    };
}
pub(crate) use asset_load;

#[derive(Clone)]
pub struct Texture {
    pub alpha: Texture2D,
    pub color: Texture2D,
}

pub struct RpdTextures {
    pub team_name_bg: Texture,
    pub single_line_name_bg: Texture,
    pub double_line_name_bg: Texture,
    pub triple_line_name_bg: Texture,
    pub team_member_role_bg: Texture,
    pub frame_bg: Texture,
}

pub struct Textures {
    pub atlantis_logo: Texture,
    pub bottom: Texture,
    pub team_information: Texture,
    pub team_black_banner: Texture,
    pub team_white_banner: Texture,
    pub team_bar: Texture,
    pub time_and_game_state: Texture,
    pub final_score: Texture,
    pub penalty: Texture,
    pub white_timout: Texture,
    pub black_timout: Texture,
    pub referee_timout: Texture,
    pub black_rpd: RpdTextures,
    pub white_rpd: RpdTextures,
    pub red_rpd: RpdTextures,
    pub frame_rpd: Texture,
    pub number_bg_rpd: Texture,
    pub tournament_logo: Option<Texture>,
    pub font: Font,
}

impl Default for Textures {
    fn default() -> Self {
        Self {
            font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF")).unwrap(),
            final_score: asset_load!("Final Score.png"),
            time_and_game_state: asset_load!("Time and Game State.png"),
            team_bar: asset_load!("Team Bars.png"),
            team_black_banner: asset_load!("Team Black.png"),
            team_white_banner: asset_load!("Team White.png"),
            team_information: asset_load!("Team Information.png"),
            bottom: asset_load!("Bottom.png"),
            atlantis_logo: asset_load!("Atlantis Logo.png"),
            penalty: asset_load!("Penalty Shot Flag.png"),
            white_timout: asset_load!("White Timeout Flag.png"),
            black_timout: asset_load!("Black Timeout Flag.png"),
            referee_timout: asset_load!("Referee Timeout Flag.png"),
            number_bg_rpd: asset_load!("Number Background.png"),
            frame_rpd: asset_load!("Frame without Number.png"),
            black_rpd: RpdTextures {
                team_name_bg: asset_load!("Black Team Name.png"),
                single_line_name_bg: asset_load!("Black Single Line Name Background.png"),
                double_line_name_bg: asset_load!("Black Double Line Name Background.png"),
                triple_line_name_bg: asset_load!("Black Triple Line Name Background.png"),
                team_member_role_bg: asset_load!("Black Team Member Role Background.png"),
                frame_bg: asset_load!("Black Picture Background.png"),
            },
            white_rpd: RpdTextures {
                team_name_bg: asset_load!("White Team Name.png"),
                single_line_name_bg: asset_load!("White Single Line Name Background.png"),
                double_line_name_bg: asset_load!("White Double Line Name Background.png"),
                triple_line_name_bg: asset_load!("White Triple Line Name Background.png"),
                team_member_role_bg: asset_load!("White Team Member Role Background.png"),
                frame_bg: asset_load!("White Picture Background.png"),
            },
            red_rpd: RpdTextures {
                team_name_bg: asset_load!("Red Team Name.png"),
                single_line_name_bg: asset_load!("Red Single Line Name Background.png"),
                double_line_name_bg: asset_load!("Red Double Line Name Background.png"),
                triple_line_name_bg: asset_load!("Red Triple Line Name Background.png"),
                team_member_role_bg: asset_load!("Red Team Member Role Background.png"),
                frame_bg: asset_load!("Red Picture Background.png"),
            },
            tournament_logo: None,
        }
    }
}

pub fn read_image_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<Texture2D, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(Texture2D::from_file_with_format(&bytes[..], None))
}

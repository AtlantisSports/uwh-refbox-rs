use std::{fs::File, io::Read, path::Path};

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

pub struct RpdTextures {
    pub team_name_bg: Texture,
    pub single_line_name_bg: Texture,
    pub double_line_name_bg: Texture,
    pub triple_line_name_bg: Texture,
    pub frame_with_number: Texture,
    pub frame_without_number: Texture,
    pub team_member_role_bg: Texture,
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
    pub in_game_mask: Texture,
    pub penalty: Texture,
    pub white_timout: Texture,
    pub black_timout: Texture,
    pub referee_timout: Texture,
    pub black_rpd: RpdTextures,
    pub white_rpd: RpdTextures,
    pub red_rpd: RpdTextures,
    pub tournament_logo: Option<Texture>,
    pub font: Font,
}

impl Default for Textures {
    fn default() -> Self {
        Self {
            font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF")).unwrap(),
            final_score: Texture {
                color: load!("../assets/color/1080/Final Score.png"),
                alpha: load!("../assets/alpha/1080/Final Score.png"),
            },
            time_and_game_state: Texture {
                color: load!("../assets/color/1080/Time and Game State.png"),
                alpha: load!("../assets/alpha/1080/Time and Game State.png"),
            },
            team_bar: Texture {
                color: load!("../assets/color/1080/Team Bars.png"),
                alpha: load!("../assets/alpha/1080/Team Bars.png"),
            },
            team_black_banner: Texture {
                color: load!("../assets/color/1080/Team Black.png"),
                alpha: load!("../assets/alpha/1080/Team Black.png"),
            },
            team_white_banner: Texture {
                color: load!("../assets/color/1080/Team White.png"),
                alpha: load!("../assets/alpha/1080/Team White.png"),
            },
            team_information: Texture {
                color: load!("../assets/color/1080/Team Information.png"),
                alpha: load!("../assets/alpha/1080/Team Information.png"),
            },
            bottom: Texture {
                color: load!("../assets/color/1080/Bottom.png"),
                alpha: load!("../assets/alpha/1080/Bottom.png"),
            },
            atlantis_logo: Texture {
                color: load!("../assets/color/1080/Atlantis Logo.png"),
                alpha: load!("../assets/alpha/1080/Atlantis Logo.png"),
            },
            in_game_mask: Texture {
                color: load!("../assets/alpha/1080/mask.png"),
                alpha: load!("../assets/alpha/1080/mask.png"),
            },
            penalty: Texture {
                color: load!("../assets/color/1080/Penalty Shot Flag.png"),
                alpha: load!("../assets/alpha/1080/Penalty Shot Flag.png"),
            },
            white_timout: Texture {
                color: load!("../assets/color/1080/White Timeout Flag.png"),
                alpha: load!("../assets/alpha/1080/White Timeout Flag.png"),
            },
            black_timout: Texture {
                color: load!("../assets/color/1080/Black Timeout Flag.png"),
                alpha: load!("../assets/alpha/1080/Black Timeout Flag.png"),
            },
            referee_timout: Texture {
                color: load!("../assets/color/1080/Referee Timeout Flag.png"),
                alpha: load!("../assets/alpha/1080/Referee Timeout Flag.png"),
            },
            black_rpd: RpdTextures {
                team_name_bg: Texture {
                    color: load!("../assets/color/1080/Black Team Name.png"),
                    alpha: load!("../assets/alpha/1080/Black Team Name.png"),
                },
                single_line_name_bg: Texture {
                    color: load!("../assets/color/1080/Black Single Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/Black Single Line Name Background.png"),
                },
                double_line_name_bg: Texture {
                    color: load!("../assets/color/1080/Black Double Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/Black Double Line Name Background.png"),
                },
                triple_line_name_bg: Texture {
                    color: load!("../assets/color/1080/Black Triple Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/Black Triple Line Name Background.png"),
                },
                frame_with_number: Texture {
                    color: load!("../assets/color/1080/Black Frame with Number.png"),
                    alpha: load!("../assets/alpha/1080/Black Frame with Number.png"),
                },
                frame_without_number: Texture {
                    color: load!("../assets/color/1080/Black Frame without Number.png"),
                    alpha: load!("../assets/alpha/1080/Black Frame without Number.png"),
                },
                team_member_role_bg: Texture {
                    color: load!("../assets/color/1080/Black Team Member Role Background.png"),
                    alpha: load!("../assets/alpha/1080/Black Team Member Role Background.png"),
                },
            },
            white_rpd: RpdTextures {
                team_name_bg: Texture {
                    color: load!("../assets/color/1080/White Team Name.png"),
                    alpha: load!("../assets/alpha/1080/White Team Name.png"),
                },
                single_line_name_bg: Texture {
                    color: load!("../assets/color/1080/White Single Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/White Single Line Name Background.png"),
                },
                double_line_name_bg: Texture {
                    color: load!("../assets/color/1080/White Double Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/White Double Line Name Background.png"),
                },
                triple_line_name_bg: Texture {
                    color: load!("../assets/color/1080/White Triple Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/White Triple Line Name Background.png"),
                },
                frame_with_number: Texture {
                    color: load!("../assets/color/1080/White Frame with Number.png"),
                    alpha: load!("../assets/alpha/1080/White Frame with Number.png"),
                },
                frame_without_number: Texture {
                    color: load!("../assets/color/1080/White Frame without Number.png"),
                    alpha: load!("../assets/alpha/1080/White Frame without Number.png"),
                },
                team_member_role_bg: Texture {
                    color: load!("../assets/color/1080/White Team Member Role Background.png"),
                    alpha: load!("../assets/alpha/1080/White Team Member Role Background.png"),
                },
            },
            red_rpd: RpdTextures {
                team_name_bg: Texture {
                    color: load!("../assets/color/1080/Red Team Name.png"),
                    alpha: load!("../assets/alpha/1080/Red Team Name.png"),
                },
                single_line_name_bg: Texture {
                    color: load!("../assets/color/1080/Red Single Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/Red Single Line Name Background.png"),
                },
                double_line_name_bg: Texture {
                    color: load!("../assets/color/1080/Red Double Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/Red Double Line Name Background.png"),
                },
                triple_line_name_bg: Texture {
                    color: load!("../assets/color/1080/Red Triple Line Name Background.png"),
                    alpha: load!("../assets/alpha/1080/Red Triple Line Name Background.png"),
                },
                frame_with_number: Texture {
                    color: load!("../assets/color/1080/Red Frame with Number.png"),
                    alpha: load!("../assets/alpha/1080/Red Frame with Number.png"),
                },
                frame_without_number: Texture {
                    color: load!("../assets/color/1080/Red Frame without Number.png"),
                    alpha: load!("../assets/alpha/1080/Red Frame without Number.png"),
                },
                team_member_role_bg: Texture {
                    color: load!("../assets/color/1080/Red Team Member Role Background.png"),
                    alpha: load!("../assets/alpha/1080/Red Team Member Role Background.png"),
                },
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

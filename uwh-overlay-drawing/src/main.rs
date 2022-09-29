use std::{str::FromStr, sync::mpsc::channel};

use coarsetime::Instant;
//use uwh_common::game_snapshot::GamePeriod;
use network::{StatePacket, TeamInfo};
use std::net::IpAddr;

use macroquad::prelude::*;
use uwh_common::game_snapshot::{GamePeriod, GameSnapshot, TimeoutSnapshot};
mod flag;
mod load_images;
mod network;
mod pages;

const APP_CONFIG_NAME: &str = "uwh-overlay-drawing";

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Overlay Program"),
        window_width: 3840,
        window_height: 1080,
        window_resizable: false,
        ..Default::default()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    refbox_ip: IpAddr,
    refbox_port: u64,
    uwhscores_url: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            refbox_ip: IpAddr::from_str("127.0.0.1").unwrap(),
            refbox_port: 8000,
            uwhscores_url: String::from("uwhscores.com"),
        }
    }
}

pub struct State {
    snapshot: GameSnapshot,
    black: TeamInfo,
    white: TeamInfo,
    game_id: u32,
    pool: String,
    start_time: String,
    white_flag: Option<Texture2D>,
    black_flag: Option<Texture2D>,
}

#[macroquad::main(window_conf())]
async fn main() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let (tx, rx) = channel::<StatePacket>();

    let config: AppConfig = match confy::load(APP_CONFIG_NAME, None) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read config file, overwriting with default. Error: {e}");
            let config = AppConfig::default();
            confy::store(APP_CONFIG_NAME, None, &config).unwrap();
            config
        }
    };

    let net_worker = std::thread::spawn(|| {
        network::networking_thread(tx, config)
            .expect("Networking error. Does the supplied URL exist and is it live?");
    });

    let textures = load_images::Textures::default();
    let mut local_state: State = State {
        snapshot: Default::default(),
        black: TeamInfo {
            team_name: String::from("BLACK"),
            flag: None,
            players: Vec::new(),
        },
        white: TeamInfo {
            team_name: String::from("WHITE"),
            flag: None,
            players: Vec::new(),
        },
        game_id: 0,
        pool: String::from("0"),
        start_time: String::new(),
        white_flag: None,
        black_flag: None,
    };

    let mut renderer = pages::PageRenderer {
        animation_register1: Instant::now(),
        animation_register2: Instant::now(),
        animation_register3: false,
        textures,
        last_timeout: TimeoutSnapshot::None,
    };
    let mut flag_renderer = flag::FlagRenderer::new();

    loop {
        assert!(!net_worker.is_finished(), "Error in Networking thread!");
        clear_background(BLACK);

        if let Ok(recieved_state) = rx.try_recv() {
            if let Some(team) = recieved_state.black {
                local_state.black = team;
                if let Some(flag_bytes) = local_state.black.flag.clone() {
                    local_state.black_flag =
                        Some(Texture2D::from_file_with_format(&flag_bytes, None));
                }
            }
            if let Some(team) = recieved_state.white {
                local_state.white = team;
                if let Some(flag_bytes) = local_state.white.flag.clone() {
                    local_state.white_flag =
                        Some(Texture2D::from_file_with_format(&flag_bytes, None));
                }
            }
            if let Some(game_id) = recieved_state.game_id {
                local_state.game_id = game_id;
            }
            if let Some(pool) = recieved_state.pool {
                local_state.pool = pool;
            }
            if let Some(start_time) = recieved_state.start_time {
                local_state.start_time = start_time;
            }
            local_state.snapshot = recieved_state.snapshot;

            // sync local penalty list
            flag_renderer.synchronize_flags(&local_state);
        }

        match local_state.snapshot.current_period {
            GamePeriod::BetweenGames => match local_state.snapshot.secs_in_period {
                151..=u32::MAX => {
                    // If an old game just finished, display its scores
                    if local_state.snapshot.is_old_game {
                        renderer.final_scores(&local_state);
                    } else {
                        renderer.next_game(&local_state);
                    }
                }
                30..=150 => {
                    renderer.roster(&local_state);
                }
                _ => {
                    renderer.pre_game_display(&local_state);
                }
            },
            GamePeriod::FirstHalf | GamePeriod::SecondHalf | GamePeriod::HalfTime => {
                renderer.in_game_display(&local_state);
                flag_renderer.draw();
            }
            GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::PreOvertime
            | GamePeriod::PreSuddenDeath
            | GamePeriod::SuddenDeath => {
                renderer.overtime_and_sudden_death_display(&local_state);
                flag_renderer.draw();
            }
        }
        next_frame().await;
    }
}

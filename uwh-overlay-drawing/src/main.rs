use std::{str::FromStr, sync::mpsc::channel};

//use uwh_common::game_snapshot::GamePeriod;
use ipc_channel::ipc;
use network::{StatePacket, TeamInfo};
use std::net::IpAddr;

use macroquad::prelude::*;
use uwh_common::game_snapshot::{GamePeriod, GameSnapshot};
mod flag;
mod load_images;
mod network;
mod pages;

const APP_CONFIG_NAME: &str = "uwh-overlay-drawing";

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
    white_flag: Option<Texture2D>,
    black_flag: Option<Texture2D>,
}

fn main() {
    procspawn::init();
    simple_logger::SimpleLogger::new().init().unwrap();
    let (tx, rx) = channel::<StatePacket>();
    let (tx_a, rx_a) = ipc::channel::<StatePacket>().unwrap();
    let (tx_c, rx_c) = ipc::channel::<StatePacket>().unwrap();

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
    procspawn::spawn(rx_a, |rx| {
        macroquad::Window::new("Alpha Stream", render_process(true, rx));
    });
    procspawn::spawn(rx_c, |rx| {
        macroquad::Window::new("Color Stream", render_process(false, rx));
    });

    loop {
        if let Ok(item) = rx.recv() {
            assert!(
                !(tx_a.send(item.clone()).is_err() & tx_c.send(item).is_err()),
                "Exiting.. Both windows closed!"
            );
        }
        assert!(!net_worker.is_finished(), "Error in Networking thread!");
    }
}

async fn render_process(is_alpha_mode: bool, rx: ipc::IpcReceiver<StatePacket>) {
    let textures = if is_alpha_mode {
        load_images::Textures::init_alpha()
    } else {
        load_images::Textures::init_color()
    };

    let mut local_state: Option<State> = None;

    //keeps track of last recieved value of recent_goal snapshot to detect a toggle into a `Some(_)` value
    let mut last_recent_goal: Option<(uwh_common::game_snapshot::Color, u8)> = None;

    let mut renderer = pages::PageRenderer {
        animation_counter: 0f32,
        textures,
        is_alpha_mode,
        secondary_animation_counter: 0f32,
    };
    let mut flag_renderer = flag::FlagRenderer::new(is_alpha_mode);

    loop {
        clear_background(BLACK);

        if let Ok(recieved_state) = rx.try_recv() {
            // Update state parameters like team names and flags if they are `Some`.
            if let Some(local_state) = &mut local_state {
                if let Some(team) = recieved_state.black {
                    local_state.black = team;
                }
                if let Some(team) = recieved_state.white {
                    local_state.white = team;
                }
                local_state.snapshot = recieved_state.snapshot;
            } else {
                // If `game_state` hasn't been init'd, just copy all the values over. Unwrap because we expect the first snapshot to contain this info always.
                local_state = Some(State {
                    snapshot: recieved_state.snapshot,
                    white_flag: recieved_state.white.as_ref().unwrap().flag_url.clone().map(
                        |url| {
                            Texture2D::from_file_with_format(&network::get_flag(url.as_str()), None)
                        },
                    ),
                    black_flag: recieved_state.black.as_ref().unwrap().flag_url.clone().map(
                        |url| {
                            Texture2D::from_file_with_format(&network::get_flag(url.as_str()), None)
                        },
                    ),
                    white: recieved_state.white.unwrap(),
                    black: recieved_state.black.unwrap(),
                });
            }
            // check if goal has been toggled to a `Some(_)` value; tell the flag renderer about the new goal
            if let Some(goal) = local_state.as_ref().unwrap().snapshot.recent_goal {
                if Some(goal) != last_recent_goal {
                    flag_renderer.add_penalty_flag(
                        flag::Flag::new(String::new(), goal.1, flag::FlagType::Goal(goal.0)),
                        &local_state.as_ref().unwrap(),
                    );
                    last_recent_goal = Some(goal);
                } else {
                    last_recent_goal = local_state.as_ref().unwrap().snapshot.recent_goal;
                }
            } else {
                last_recent_goal = local_state.as_ref().unwrap().snapshot.recent_goal;
            }

            // More concise code works only in nightly, if let chaining not stable yet.
            // if let Some(goal) = game_state.as_ref().unwrap().snapshot.recent_goal && Some(goal) != last_recent_goal  {
            //     flag_renderer.add_flag(flag::Flag::new(String::from("re"), goal.1, flag::FlagType::Goal( goal.0)));
            //     last_recent_goal = Some(goal);
            // } else  {
            //     last_recent_goal = game_state.as_ref().unwrap().snapshot.recent_goal;
            // }

            // sync local penalty list
            flag_renderer.synchronize_penalties(
                uwh_common::game_snapshot::Color::White,
                local_state.as_ref().unwrap(),
            );
            flag_renderer.synchronize_penalties(
                uwh_common::game_snapshot::Color::Black,
                local_state.as_ref().unwrap(),
            );
        }

        if let Some(state) = &local_state {
            match state.snapshot.current_period {
                GamePeriod::BetweenGames => match state.snapshot.secs_in_period {
                    151..=u32::MAX => {
                        // If an old game just finished, display its scores
                        if state.snapshot.is_old_game {
                            renderer.final_scores(state);
                        } else {
                            renderer.next_game(state);
                        }
                    }
                    30..=150 => {
                        renderer.roster(state);
                    }
                    _ => {
                        renderer.pre_game_display(state);
                    }
                },
                GamePeriod::FirstHalf | GamePeriod::SecondHalf | GamePeriod::HalfTime => {
                    renderer.in_game_display(state);
                    flag_renderer.draw();
                }
                GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::PreOvertime
                | GamePeriod::PreSuddenDeath
                | GamePeriod::SuddenDeath => {
                    renderer.overtime_and_sudden_death_display(state);
                    flag_renderer.draw();
                }
            }
        }
        next_frame().await;
    }
}

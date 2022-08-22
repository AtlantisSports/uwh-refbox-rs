use std::{str::FromStr, sync::mpsc::channel};

//use uwh_common::game_snapshot::GamePeriod;
use ipc_channel::ipc;
use network::StatePacket;
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
    black: String,
    white: String,
    w_flag: Option<Texture2D>,
    b_flag: Option<Texture2D>,
}

fn main() {
    procspawn::init();
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
            .expect("Networking error. Does the supplied URL exist and is it live?")
    });
    procspawn::spawn(rx_a, |rx| {
        macroquad::Window::new("Alpha Stream", render_process(true, rx))
    });
    procspawn::spawn(rx_c, |rx| {
        macroquad::Window::new("Color Stream", render_process(false, rx))
    });

    loop {
        if let Ok(item) = rx.recv() {
            if tx_a.send(item.clone()).is_err() & tx_c.send(item).is_err() {
                panic!("Exiting.. Both windows closed!");
            }
        }
        if net_worker.is_finished() {
            panic!("Error in Networking thread!");
        }
    }
}

async fn render_process(is_alpha_mode: bool, rx: ipc::IpcReceiver<StatePacket>) {
    let textures = if !is_alpha_mode {
        load_images::Textures::init_color()
    } else {
        load_images::Textures::init_alpha()
    };

    let mut game_state: Option<State> = None;

    // Should the goal graphic be displayed?
    // let mut show_goal = false;
    //keeps track of old whit and black scores in order to detect a change and show the goal graphic
    // let (mut b_score, mut w_score) = (0, 0);
    let mut renderer = pages::PageRenderer {
        animation_counter: 0f32,
        textures,
        is_alpha_mode,
        secondary_animation_counter: 0f32,
    };

    loop {
        clear_background(BLACK);
        if let Ok(state) = rx.try_recv() {
            // Update state parameters like team names and flags if they are present.
            if let Some(game_state) = &mut game_state {
                game_state.w_flag = if state.w_flag.is_some() {
                    Some(Texture2D::from_file_with_format(
                        state.w_flag.unwrap().as_ref(),
                        None,
                    ))
                } else {
                    game_state.w_flag
                };
                if let Some(team_name) = state.black {
                    game_state.black = team_name;
                }
                if let Some(team_name) = state.white {
                    game_state.white = team_name;
                }
                game_state.snapshot = state.snapshot;
            } else {
                // If `game_state` hasn't been init'd, just copy all the values over.
                game_state = Some(State {
                    white: state.white.unwrap(),
                    black: state.black.unwrap(),
                    w_flag: if let Some(flag) = state.w_flag {
                        Some(Texture2D::from_file_with_format(flag.as_ref(), None))
                    } else {
                        None
                    },
                    b_flag: if let Some(flag) = state.b_flag {
                        Some(Texture2D::from_file_with_format(flag.as_ref(), None))
                    } else {
                        None
                    },
                    snapshot: state.snapshot,
                })
            }
        }
        // if show_goal {
        //     if !is_alpha_mode {
        //         pages_color::show_goal_graphic(
        //             &textures,
        //             &mut secondary_animation_counter,
        //             &mut show_goal,
        //         );
        //     } else {
        //         pages_alpha::show_goal_graphic(
        //             &textures,
        //             &mut secondary_animation_counter,
        //             &mut show_goal,
        //         );
        //     }
        // }

        if let Some(state) = &game_state {
            // if state.snapshot.b_score != b_score || state.snapshot.w_score != w_score {
            //     w_score = state.snapshot.w_score;
            //     b_score = state.snapshot.b_score;
            //     show_goal = true;
            // }
            match state.snapshot.current_period {
                GamePeriod::BetweenGames => match state.snapshot.secs_in_period {
                    151..=u16::MAX => {
                        // If an old game just finished, display its scores for a minute
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
                }
                GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::PreOvertime
                | GamePeriod::PreSuddenDeath
                | GamePeriod::SuddenDeath => {
                    renderer.overtime_and_sudden_death_display(state);
                }
            }
        }
        next_frame().await;
    }
}

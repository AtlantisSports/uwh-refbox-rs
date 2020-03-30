use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    SubCommand,
};
use embedded_graphics::prelude::*;
//use embedded_graphics::{egcircle, egline, fonts::Font, pixelcolor, text_6x8};
use embedded_graphics::{fonts::Font, pixelcolor};
use embedded_graphics_simulator::DisplayBuilder;
//use fonts::fonts::Font6x8 as CustomFont6x8;
use fonts::fonts::{Font11x25, Font16x31, Font22x46, Font32x64, Font6x8, Font8x15};
use gio::prelude::*;
use gtk::prelude::*;
use log::*;
use std::{
    convert::TryInto,
    fmt::Display,
    fs::{File, OpenOptions},
    io::{ErrorKind, Write},
    ops::{Div, Rem},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::{Duration, Instant},
};

mod config;
mod game_snapshot;
mod tournament_manager;
use config::Config;
use game_snapshot::*;
use tournament_manager::*;

const BUTTON_SPACING: i32 = 12;
const BUTTON_MARGIN: i32 = 6;
//const RETURN_BTN_SIZE_X: i32 = 400;
//const RETURN_BTN_SIZE_Y: i32 = 250;
//const BUTTON_STANDARD_HEIGHT: i32 = 70;
//const BUTTON_STANDARD_HEIGHT: config.hardware.screen_y / 6;

//const LABEL_STANDARD_HEIGHT: i32 = 35;
//const KEYPAD_BUTTON_SIZE: i32 = 70;

const STYLE: &str = std::include_str!("style.css");

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // This allows the use of error!(), warn!(), info!(), etc.
    env_logger::init();

    // Parse argumanets given on the command line
    let matches = app_from_crate!()
        .setting(AppSettings::VersionlessSubcommands)
        .arg(
            Arg::with_name("config-file")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("PATH")
                .help("Path to the config file"),
        )
        .subcommand(
            SubCommand::with_name("config")
                .about("Manipulate the config file")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("check").about("Check that the config file is valid"),
                )
                .subcommand(
                    SubCommand::with_name("generate").about("Generate a default config file"),
                ),
        )
        .subcommand(
            SubCommand::with_name("simulate").about("Run the simulation of the display panels"),
        )
        .get_matches();

    let config_path = matches.value_of("config-file").unwrap_or("timeshark.toml");

    // If the `config` command was given, handle that then exit
    if let Some(matches) = matches.subcommand_matches("config") {
        if let Some(_matches) = matches.subcommand_matches("check") {
            match Config::new_from_file(config_path) {
                Ok(_) => println!("Config file {} is valid", config_path),
                Err(_) => println!("Config file {} is not valid :(", config_path),
            }
            return Ok(());
        } else if let Some(_matches) = matches.subcommand_matches("generate") {
            let mut file = match create_new_file(config_path) {
                Ok(f) => f,
                Err(e) => match e.kind() {
                    ErrorKind::AlreadyExists => {
                        let mut backup_path = config_path.to_string();
                        backup_path.push_str(".bak");
                        eprintln!(
                            "Warning: {} already exists, moving it to {}",
                            config_path, &backup_path
                        );
                        let _backup_file = create_new_file(&backup_path)?;
                        std::fs::rename(config_path, &backup_path)?;
                        create_new_file(config_path)?
                    }
                    _ => {
                        eprintln!("Error: could not open {} for writing: {}", config_path, e);
                        return Err(Box::new(e));
                    }
                },
            };

            let config: Config = Default::default();
            let config_str = toml::ser::to_string(&config)?;
            file.write_all(&config_str.into_bytes())?;

            println!("Wrote default config to {}", config_path);
            return Ok(());
        }
    }

    // If the user asks, simulate the display panels instead
    if matches.subcommand_matches("simulate").is_some() {
        // Make a fake game state
        let state = GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 754,                    // 12:34
            timeout: TimeoutState::None, //White (34), //Ref(34), //PenaltyShot(34),
            b_score: 10,
            w_score: 15,
            penalties: vec![
                PenaltySnapshot {
                    color: Color::Black,
                    player_number: 1,
                    time: PenaltyTime::Seconds(23),
                },
                PenaltySnapshot {
                    color: Color::Black,
                    player_number: 4,
                    time: PenaltyTime::Seconds(56),
                },
                PenaltySnapshot {
                    color: Color::Black,
                    player_number: 7,
                    time: PenaltyTime::Seconds(89),
                },
                PenaltySnapshot {
                    color: Color::White,
                    player_number: 10,
                    time: PenaltyTime::Seconds(12),
                },
                PenaltySnapshot {
                    color: Color::White,
                    player_number: 3,
                    time: PenaltyTime::Seconds(45),
                },
                PenaltySnapshot {
                    color: Color::White,
                    player_number: 6,
                    time: PenaltyTime::TotalDismissal,
                },

            ],
        };

        let red = pixelcolor::Rgb888::new(255, 0, 0);
        let yellow = pixelcolor::Rgb888::new(255, 255, 0);
        let green = pixelcolor::Rgb888::new(0, 255, 0);
        let blue = pixelcolor::Rgb888::new(0, 0, 255);
        let white = pixelcolor::Rgb888::new(255, 255, 255);

        let mut display = DisplayBuilder::new()
            .size(256, 64)
            .scale(3)
            .pixel_spacing(1)
            .build_rgb();

        let game_color = match state.timeout {
            TimeoutSnapshot::PenaltyShot(_) => red,
            TimeoutSnapshot::Ref(_) => yellow,
            _ => match state.current_period {
                GamePeriod::FirstHalf | GamePeriod::SecondHalf | GamePeriod::OvertimeFirstHalf | GamePeriod::OvertimeSecondHalf => green,
                GamePeriod::BetweenGames | GamePeriod::HalfTime | GamePeriod::PreOvertime | GamePeriod::OvertimeHalfTime | GamePeriod::PreSuddenDeath => yellow,
                GamePeriod::SuddenDeath => red,
            },
        };

        let timeout_color = match state.timeout {
            TimeoutSnapshot::White(_) => white,
            TimeoutSnapshot::Black(_) => blue,
            TimeoutSnapshot::Ref(_) => yellow,
            _ => red,
        };

        // EVERYTHING TO BE DISPLAYED ON THE CENTER 2 TIME PANELS
        match state.timeout {
            TimeoutSnapshot::None => {
                //No timeout currently
                display.draw(Font22x46::render_str(&secs_to_time_string(state.secs_in_period)).stroke(Some(game_color)).translate(Point::new(74, 18)));

                let (text, x, y) = match state.current_period {
                    GamePeriod::BetweenGames => ("NEXT GAME", 92, 2),
                    GamePeriod::FirstHalf => ("1ST HALF", 100, 2),
                    GamePeriod::HalfTime => ("HALF-TIME", 92, 2),
                    GamePeriod::SecondHalf => ("2ND HALF", 100, 2),
                    GamePeriod::PreOvertime => ("PRE-OVERTIME", 80, 2),
                    GamePeriod::OvertimeFirstHalf => ("O/T 1ST HALF", 80, 2),
                    GamePeriod::OvertimeHalfTime => ("O/T HALF TIME", 76, 2),
                    GamePeriod::OvertimeSecondHalf => ("O/T 2ND HALF", 80, 2),
                    GamePeriod::PreSuddenDeath => ("PRE-SUDDEN DEATH", 64, 2),
                    GamePeriod::SuddenDeath => ("SUDDEN DEATH", 80, 2),
                };

                display.draw(Font8x15::render_str(text).stroke(Some(game_color)).translate(Point::new(x, y)));
            }

            _ => {
                //Some timeout currently
                display.draw(Font16x31::render_str(&secs_to_time_string(state.secs_in_period)).stroke(Some(game_color)).translate(Point::new(108, 33)));

                let (text1, x1, y1, text2, x2, y2, text3, x3, y3) = match state.current_period {
                    GamePeriod::FirstHalf => ("1ST", 72, 33, "", 0, 0, "HALF", 68, 48),
                    GamePeriod::SecondHalf => ("2ND", 72, 33, "", 0, 0, "HALF", 68, 48),
                    GamePeriod::OvertimeFirstHalf => ("OT 1", 64, 33, "ST", 96, 33, "HALF", 68, 48),
                    GamePeriod::OvertimeSecondHalf => ("OT 2", 64, 33, "ND", 96, 33, "HALF", 68, 48),
                    GamePeriod::SuddenDeath => ("", 0, 0, "SUDDEN", 70, 39, "DEATH", 68, 48),
                    _ => ("PERIOD ERROR", 72, 33, "", 0, 0, "", 0, 0),
                };

                display.draw(Font8x15::render_str(text1).stroke(Some(game_color)).translate(Point::new(x1, y1)));
                display.draw(Font6x8::render_str(text2).stroke(Some(game_color)).translate(Point::new(x2, y2)));
                display.draw(Font8x15::render_str(text3).stroke(Some(game_color)).translate(Point::new(x3, y3)));

                match state.timeout {
                    TimeoutState::White(secs) => {
                        display.draw(Font8x15::render_str("WHITE").stroke(Some(timeout_color)).translate(Point::new(76, 2)));
                        display.draw(Font8x15::render_str("TIMEOUT").stroke(Some(timeout_color)).translate(Point::new(68, 17)));
                        display.draw(Font16x31::render_str(&format!(":{}", secs)).stroke(Some(timeout_color)).translate(Point::new(132, 2)));
                    }

                    TimeoutState::Black(secs) => {
                        display.draw(Font8x15::render_str("BLACK").stroke(Some(timeout_color)).translate(Point::new(76, 2)));
                        display.draw(Font8x15::render_str("TIMEOUT").stroke(Some(timeout_color)).translate(Point::new(68, 17)));
                        display.draw(Font16x31::render_str(&format!(":{}", secs)).stroke(Some(timeout_color)).translate(Point::new(132, 2)));
                    }

                    TimeoutState::Ref(_) => display.draw(
                        Font11x25::render_str("REF TIMEOUT").stroke(Some(timeout_color)).translate(Point::new(68, 3))),

                    TimeoutState::PenaltyShot(_) => {
                        display.draw(Font11x25::render_str("PENALTY").stroke(Some(timeout_color)).translate(Point::new(64, 3)));
                        display.draw(Font11x25::render_str("SHOT").stroke(Some(timeout_color)).translate(Point::new(149, 3)));
                    }

                    _ => display.draw(Font8x15::render_str("T/O ERROR").stroke(Some(red)).translate(Point::new(64, 133))),
                };
            }
        };

        // Temporary values for assigning a penalty
        let black_penalties = true;
        let white_penalties = true;

        // Temporary value for assigning colors to a side
        let white_on_right = false;

        // Assigning X-Offsets depending on which sides the teams/colors are
        // [0 - x_offset,
        // 1 - single_digit_score_x_offset,
        // 2 - single_digit_score_with_penalty_x_offset,
        // 3 - small_score_x_offset,
        // 4 - vertical_pen_x_offset,
        // 5 - pen_offset_sign]
        
        let b_x: [i32; 6] = if white_on_right {
            [2, 16, 32, 9, 3, -1]
        } else {
            [194, 16, 0, 9, 34, 1]
        };

        let w_x: [i32; 6] = if white_on_right {
            [194, 16, 0, 9, 34, 1]        
        } else {
            [2, 16, 32, 9, 3, -1]
        };

        // Black Score Panel
        if black_penalties {
            if state.b_score < 10 {
                // Full Size Black Score, Single Digit - Justified Inside (Towards Time Panels)
                display.draw(Font32x64::render_str(&format!("{:<2}", state.b_score)).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[2], 2)));
                // Vertical Penalties (Up to 3) - Justified Outside (Away from Time Panels)
                // Top Penalty
                display.draw(Font6x8::render_str(&format!("{:^4}", "#1")).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[4], 2)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "0:23")).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[4], 10)));
                // Middle Penalty - 
                display.draw(Font6x8::render_str(&format!("{:^4}", "#4")).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[4], 24)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "0:56")).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[4], 32)));
                // Bottom Penalty
                display.draw(Font6x8::render_str(&format!("{:^4}", "#7")).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[4], 47)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "1:29")).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[4], 55)));
            } else {
                // 3/4 Size Black Score (Double Digit - Centered on Score Panel)
                display.draw(Font22x46::render_str(&format!("{:<2}", state.b_score)).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[3], 2)));
                // Horizontal Penalties (Up to 2) - Justified Outside (Away from Time Panels)
                // Outside Penalty
                display.draw(Font6x8::render_str(&format!("{:^4}", "#1")).stroke(Some(blue)).translate(Point::new(b_x[0] + 18 + 14 * b_x[5], 47)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "0:23")).stroke(Some(blue)).translate(Point::new(b_x[0] + 18 + 14 * b_x[5], 55)));
                // Inside Penalty
                display.draw(Font6x8::render_str(&format!("{:^4}", "#4")).stroke(Some(blue)).translate(Point::new(b_x[0] + 18 + 14 * b_x[5] - 29 * b_x[5], 47)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "0:56")).stroke(Some(blue)).translate(Point::new(b_x[0] + 18 + 14 * b_x[5] - 29 * b_x[5], 55)));
            }
        } else if state.b_score < 10 {
            // Full Size Black Score (Single Digit Centered)
            display.draw(Font32x64::render_str(&format!("{:<2}", state.b_score)).stroke(Some(blue)).translate(Point::new(b_x[0] + b_x[1], 2)));
        } else {
            // Full Size Black Score (Double Digit Centered)
            display.draw(Font32x64::render_str(&format!("{:<2}", state.b_score)).stroke(Some(blue)).translate(Point::new(b_x[0], 2)));
        }

        // White Score Panel
        if white_penalties {
            if state.w_score < 10 {
                // Full Size White Score, Single Digit - Justified Inside (Towards Time Panels)
                display.draw(Font32x64::render_str(&format!("{:<2}", state.w_score)).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[2], 2)));
                // Vertical Penalties (Up to 3) - Justified Outside (Away from Time Panels)
                // Top Penalty
                display.draw(Font6x8::render_str(&format!("{:^4}", "#10")).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[4], 2)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "0:12")).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[4], 10)));
                // Middle Penalty - 
                display.draw(Font6x8::render_str(&format!("{:^4}", "#3")).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[4], 24)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "0:45")).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[4], 32)));
                // Bottom Penalty
                display.draw(Font6x8::render_str(&format!("{:^4}", "#6")).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[4], 47)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "T-D")).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[4], 55)));
            } else {
                // 3/4 Size White Score (Double Digit - Centered on Score Panel)
                display.draw(Font22x46::render_str(&format!("{:<2}", state.w_score)).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[3], 2)));
                // Horizontal Penalties (Up to 2) - Justified Outside (Away from Time Panels)
                // Outside Penalty
                display.draw(Font6x8::render_str(&format!("{:^4}", "#10")).stroke(Some(white)).translate(Point::new(w_x[0] + 18 + 14 * w_x[5], 47)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "0:12")).stroke(Some(white)).translate(Point::new(w_x[0] + 18 + 14 * w_x[5], 55)));
                // Inside Penalty
                display.draw(Font6x8::render_str(&format!("{:^4}", "#3")).stroke(Some(white)).translate(Point::new(w_x[0] + 18 + 14 * w_x[5] - 29 * w_x[5], 47)));
                display.draw(Font6x8::render_str(&format!("{:^4}", "0:45")).stroke(Some(white)).translate(Point::new(w_x[0] + 18 + 14 * w_x[5] - 29 * w_x[5], 55)));
            }
        } else if state.w_score < 10 {
            // Full Size White Score (Single Digit Centered)
            display.draw(Font32x64::render_str(&format!("{:<2}", state.w_score)).stroke(Some(white)).translate(Point::new(w_x[0] + w_x[1], 2)));
        } else {
            // Full Size White Score (Double Digit Centered)
            display.draw(Font32x64::render_str(&format!("{:<2}", state.w_score)).stroke(Some(white)).translate(Point::new(w_x[0], 2)));
        }


        loop {
            let end = display.run_once();

            if end {
                break;
            }

            thread::sleep(Duration::from_millis(200))
        }

        return Ok(());
    }

    let config = Config::new_from_file(config_path)?;

    let tm = Arc::new(Mutex::new(TournamentManager::new(config.game.clone())));

    // Setup the application that gets run
    let uiapp = gtk::Application::new(
        Some("org.navisjon.refbox"),
        gio::ApplicationFlags::FLAGS_NONE,
    )
    .expect("Application::new() failed");

    // Some debugging output
    info!("App initialized");

    // Initialize the app
    uiapp.connect_activate(move |app| {
        // Setup the app to use the CSS Style defined at the top of this file
        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(STYLE.as_bytes())
            .expect("Failed to load CSS Style");
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default().expect("Error initializing CSS provider"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Setup the main window
        let win = gtk::ApplicationWindow::new(app);
        win.set_default_size(config.hardware.screen_x, config.hardware.screen_y);
        win.set_title("UWH Refbox");
        win.set_resizable(false);

        // create the channel that sends updates to be drawn
        let (state_send, state_recv) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);

        //
        //
        // Start Page
        //
        let start_layout = gtk::Grid::new();
        start_layout.set_column_homogeneous(true);
        start_layout.set_row_homogeneous(true);
        start_layout.set_margin_top(BUTTON_MARGIN);
        start_layout.set_margin_start(BUTTON_MARGIN);
        start_layout.set_margin_end(BUTTON_MARGIN);
        start_layout.set_margin_bottom(BUTTON_MARGIN);
        start_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        start_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let choose_manual = new_button("MANUAL", &["blue"], None);
        let choose_auto = new_button("AUTOMATIC", &["green"], None);
        let choose_exit = new_button("EXIT TO DESKTOP", &["red"], None);

        let choose_type = new_label("CHOOSE TIMING CONFIGURATION", "header-gray");

        start_layout.attach(&choose_type, 0, 0, 1, 1);
        start_layout.attach(&choose_manual, 0, 1, 1, 3);
        start_layout.attach(&choose_auto, 0, 4, 1, 3);
        start_layout.attach(&choose_exit, 0, 7, 1, 2);

        //
        //
        // Main Page
        //
        let main_layout = gtk::Grid::new();
        main_layout.set_column_homogeneous(true);
        main_layout.set_row_homogeneous(true);
        main_layout.set_margin_top(BUTTON_MARGIN);
        main_layout.set_margin_start(BUTTON_MARGIN);
        main_layout.set_margin_end(BUTTON_MARGIN);
        main_layout.set_margin_bottom(BUTTON_MARGIN);
        main_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        main_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let edit_game_time = new_button("##:##", &["game-time-green"], None);
        let new_penalty_shot = new_button("PENALTY SHOT", &["red"], None);
        let edit_game_information = new_button("GAME INFORMATION", &["gray"], None);
        let edit_game_parameters = new_button("GAME PARAMETERS", &["gray"], None);

        let edit_white_score = new_button("#W", &["white-score"], None);
        let add_white_score = new_button("SCORE\nWHITE", &["white"], None);
        let edit_white_time_penalty = new_button("WHITE\nTIME\nPENALTY\nLIST", &["white"], None);
        let edit_black_score = new_button("#B", &["black-score"], None);
        let add_black_score = new_button("SCORE\nBLACK", &["black"], None);
        let edit_black_time_penalty = new_button("BLACK\nTIME\nPENALTY\nLIST", &["black"], None);
        let main_white_timeout = new_button("WHITE\nTIMEOUT", &["white"], None);
        let main_referee_timeout = new_button("START", &["yellow"], None);
        let main_black_timeout = new_button("BLACK\nTIMEOUT", &["black"], None);

        let game_state_header = new_label("FIRST GAME IN", "header-dark-gray");
        let white_header = new_label("WHITE", "header-white");
        let black_header = new_label("BLACK", "header-black");

        let white_box = gtk::Grid::new();
        white_box.set_column_homogeneous(true);
        white_box.set_row_homogeneous(true);
        white_box.attach(&white_header, 0, 0, 1, 1);
        white_box.attach(&edit_white_score, 0, 1, 1, 2);

        let game_box = gtk::Grid::new();
        game_box.set_column_homogeneous(true);
        game_box.set_row_homogeneous(true);
        game_box.attach(&game_state_header, 0, 0, 1, 1);
        game_box.attach(&edit_game_time, 0, 1, 1, 2);

        let black_box = gtk::Grid::new();
        black_box.set_column_homogeneous(true);
        black_box.set_row_homogeneous(true);
        black_box.attach(&black_header, 0, 0, 1, 1);
        black_box.attach(&edit_black_score, 0, 1, 1, 2);

        main_layout.attach(&white_box, 0, 0, 3, 3);
        main_layout.attach(&game_box, 3, 0, 6, 3);
        main_layout.attach(&black_box, 9, 0, 3, 3);
        main_layout.attach(&add_white_score, 0, 3, 3, 2);
        main_layout.attach(&new_penalty_shot, 3, 3, 6, 2);
        main_layout.attach(&add_black_score, 9, 3, 3, 2);
        main_layout.attach(&edit_white_time_penalty, 0, 5, 3, 4);
        main_layout.attach(&edit_game_information, 3, 5, 6, 1);
        main_layout.attach(&edit_game_parameters, 3, 6, 6, 3);
        main_layout.attach(&edit_black_time_penalty, 9, 5, 3, 4);
        main_layout.attach(&main_white_timeout, 0, 9, 3, 2);
        main_layout.attach(&main_referee_timeout, 3, 9, 6, 2);
        main_layout.attach(&main_black_timeout, 9, 9, 3, 2);

        //
        //
        // New Score Page
        //
        let new_score_layout = gtk::Grid::new();
        new_score_layout.set_column_homogeneous(true);
        new_score_layout.set_row_homogeneous(true);
        new_score_layout.set_margin_top(BUTTON_MARGIN);
        new_score_layout.set_margin_start(BUTTON_MARGIN);
        new_score_layout.set_margin_end(BUTTON_MARGIN);
        new_score_layout.set_margin_bottom(BUTTON_MARGIN);
        new_score_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        new_score_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let score_white_select = new_toggle_button("WHITE", &["white"], None);
        let score_black_select = new_toggle_button("BLACK", &["black"], None);

        let score_cancel = new_button("CANCEL", &["red"], None);
        let score_submit = new_button("SUBMIT", &["green"], None);
        let score_white_timeout = new_button("WHITE\nTIMEOUT", &["white"], None);
        let score_referee_timeout = new_button("REFEREE TIMEOUT", &["yellow"], None);
        let score_black_timeout = new_button("BLACK\nTIMEOUT", &["black"], None);

        let (score_keypad, score_player_number) = new_keypad();

        new_score_layout.attach(&score_keypad, 0, 0, 4, 9);
        new_score_layout.attach(&score_white_select, 4, 0, 4, 3);
        new_score_layout.attach(&score_black_select, 8, 0, 4, 3);
        new_score_layout.attach(&score_cancel, 4, 7, 4, 2);
        new_score_layout.attach(&score_submit, 8, 7, 4, 2);
        new_score_layout.attach(&score_white_timeout, 0, 9, 3, 2);
        new_score_layout.attach(&score_referee_timeout, 3, 9, 6, 2);
        new_score_layout.attach(&score_black_timeout, 9, 9, 3, 2);

        // Setting up the white/black selected buttons
        let score_black_select_ = score_black_select.clone();
        let score_white_select_ = score_white_select.clone();

        score_black_select.connect_clicked(move |b| {
            if b.get_active() {
                score_white_select_.set_active(false);
            } else if !score_white_select_.get_active() {
                b.set_active(true);
            }
        });

        score_white_select.connect_clicked(move |b| {
            if b.get_active() {
                score_black_select_.set_active(false);
            } else if !score_black_select_.get_active() {
                b.set_active(true);
            }
        });

        //
        //
        // Score Edit Page
        //
        let edit_score_layout = gtk::Grid::new();
        edit_score_layout.set_column_homogeneous(true);
        edit_score_layout.set_row_homogeneous(true);
        edit_score_layout.set_margin_top(BUTTON_MARGIN);
        edit_score_layout.set_margin_start(BUTTON_MARGIN);
        edit_score_layout.set_margin_end(BUTTON_MARGIN);
        edit_score_layout.set_margin_bottom(BUTTON_MARGIN);
        edit_score_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        edit_score_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let white_score_plus = new_button("+", &["blue-modifier"], None);
        white_score_plus.set_margin_start(BUTTON_MARGIN);
        white_score_plus.set_margin_top(BUTTON_MARGIN);
        white_score_plus.set_margin_bottom(BUTTON_MARGIN);
        let white_score_minus = new_button("-", &["blue-modifier"], None);
        white_score_minus.set_margin_start(BUTTON_MARGIN);
        white_score_minus.set_margin_top(BUTTON_MARGIN);
        white_score_minus.set_margin_bottom(BUTTON_MARGIN);
        let black_score_plus = new_button("+", &["blue-modifier"], None);
        black_score_plus.set_margin_end(BUTTON_MARGIN);
        black_score_plus.set_margin_top(BUTTON_MARGIN);
        black_score_plus.set_margin_bottom(BUTTON_MARGIN);
        let black_score_minus = new_button("-", &["blue-modifier"], None);
        black_score_minus.set_margin_end(BUTTON_MARGIN);
        black_score_minus.set_margin_top(BUTTON_MARGIN);
        black_score_minus.set_margin_bottom(BUTTON_MARGIN);

        let score_edit_cancel = new_button("CANCEL", &["red"], None);
        let score_edit_submit = new_button("SUBMIT", &["green"], None);
        let score_edit_white_timeout = new_button("WHITE\nTIMEOUT", &["white"], None);
        let score_edit_referee_timeout = new_button("REFEREE TIMEOUT", &["yellow"], None);
        let score_edit_black_timeout = new_button("BLACK\nTIMEOUT", &["black"], None);

        let white_score_header = new_label("WHITE SCORE", "header-white");
        let black_score_header = new_label("BLACK SCORE", "header-black");
        let modified_white_score = new_label("#W", "modified-score-white");
        let modified_black_score = new_label("#B", "modified-score-black");
        let empty_score_edit_label = gtk::Label::new(None);

        let white_score_header_box = gtk::Grid::new();
        white_score_header_box
            .get_style_context()
            .add_class("white");
        white_score_header_box.set_column_homogeneous(true);
        white_score_header_box.set_row_homogeneous(true);
        white_score_header_box.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        white_score_header_box.attach(&white_score_header, 0, 0, 2, 1);
        white_score_header_box.attach(&white_score_plus, 0, 1, 1, 2);
        white_score_header_box.attach(&white_score_minus, 0, 3, 1, 2);
        white_score_header_box.attach(&modified_white_score, 1, 1, 1, 4);

        let black_score_header_box = gtk::Grid::new();
        black_score_header_box
            .get_style_context()
            .add_class("black");
        black_score_header_box.set_column_homogeneous(true);
        black_score_header_box.set_row_homogeneous(true);
        black_score_header_box.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        black_score_header_box.attach(&black_score_header, 0, 0, 2, 1);
        black_score_header_box.attach(&modified_black_score, 0, 1, 1, 4);
        black_score_header_box.attach(&black_score_plus, 1, 1, 1, 2);
        black_score_header_box.attach(&black_score_minus, 1, 3, 1, 2);

        edit_score_layout.attach(&white_score_header_box, 0, 0, 6, 5);
        edit_score_layout.attach(&black_score_header_box, 6, 0, 6, 5);
        edit_score_layout.attach(&empty_score_edit_label, 0, 5, 12, 2);
        edit_score_layout.attach(&score_edit_cancel, 0, 7, 4, 2);
        edit_score_layout.attach(&score_edit_submit, 8, 7, 4, 2);
        edit_score_layout.attach(&score_edit_white_timeout, 0, 9, 3, 2);
        edit_score_layout.attach(&score_edit_referee_timeout, 3, 9, 6, 2);
        edit_score_layout.attach(&score_edit_black_timeout, 9, 9, 3, 2);

        let modified_white_score_ = modified_white_score.clone();
        white_score_plus.connect_clicked(move |_| {
            let old = modified_white_score_
                .get_label()
                .unwrap()
                .as_str()
                .parse::<u8>()
                .unwrap();
            modified_white_score_.set_label(&format!("{}", old.saturating_add(1)));
        });

        let modified_white_score_ = modified_white_score.clone();
        white_score_minus.connect_clicked(move |_| {
            let old = modified_white_score_
                .get_label()
                .unwrap()
                .as_str()
                .parse::<u8>()
                .unwrap();
            modified_white_score_.set_label(&format!("{}", old.saturating_sub(1)));
        });

        let modified_black_score_ = modified_black_score.clone();
        black_score_plus.connect_clicked(move |_| {
            let old = modified_black_score_
                .get_label()
                .unwrap()
                .as_str()
                .parse::<u8>()
                .unwrap();
            modified_black_score_.set_label(&format!("{}", old.saturating_add(1)));
        });

        let modified_black_score_ = modified_black_score.clone();
        black_score_minus.connect_clicked(move |_| {
            let old = modified_black_score_
                .get_label()
                .unwrap()
                .as_str()
                .parse::<u8>()
                .unwrap();
            modified_black_score_.set_label(&format!("{}", old.saturating_sub(1)));
        });

        //
        //
        // Time Penalty Confirmation Page
        //
        let time_penalty_conf_layout = gtk::Grid::new();
        time_penalty_conf_layout.set_column_homogeneous(true);
        time_penalty_conf_layout.set_row_homogeneous(true);
        time_penalty_conf_layout.set_margin_top(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_start(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_end(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_bottom(BUTTON_MARGIN);
        time_penalty_conf_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        time_penalty_conf_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let white_time_list = new_button("WHITE PENALTIES", &["white"], None);
        let black_time_list = new_button("BLACK PENALTIES", &["black"], None);
        let penalty_conf_cancel = new_button("CANCEL", &["red"], None);
        let penalty_conf_new = new_button("NEW", &["blue"], None);
        let penalty_conf_start = new_button("START /\nDONE", &["green"], None);
        let penalty_conf_white_timeout = new_button("WHITE\nTIMEOUT", &["white"], None);
        let penalty_conf_referee_timeout = new_button("REFEREE TIMEOUT", &["yellow"], None);
        let penalty_conf_black_timeout = new_button("BLACK\nTIMEOUT", &["black"], None);

        time_penalty_conf_layout.attach(&white_time_list, 0, 0, 6, 7);
        time_penalty_conf_layout.attach(&black_time_list, 6, 0, 6, 7);
        time_penalty_conf_layout.attach(&penalty_conf_new, 0, 7, 4, 2);
        time_penalty_conf_layout.attach(&penalty_conf_cancel, 4, 7, 4, 2);
        time_penalty_conf_layout.attach(&penalty_conf_start, 8, 7, 4, 2);
        time_penalty_conf_layout.attach(&penalty_conf_white_timeout, 0, 9, 3, 2);
        time_penalty_conf_layout.attach(&penalty_conf_referee_timeout, 3, 9, 6, 2);
        time_penalty_conf_layout.attach(&penalty_conf_black_timeout, 9, 9, 3, 2);

        //
        //
        // Time Penalty Add/Edit Page
        //
        let penalty_add_layout = gtk::Grid::new();
        penalty_add_layout.set_column_homogeneous(true);
        penalty_add_layout.set_row_homogeneous(true);
        penalty_add_layout.set_margin_top(BUTTON_MARGIN);
        penalty_add_layout.set_margin_start(BUTTON_MARGIN);
        penalty_add_layout.set_margin_end(BUTTON_MARGIN);
        penalty_add_layout.set_margin_bottom(BUTTON_MARGIN);
        penalty_add_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        penalty_add_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let penalty_white_select = new_toggle_button("WHITE", &["white"], None);
        let penalty_black_select = new_toggle_button("BLACK", &["black"], None);
        let penalty_1min = new_toggle_button("1 MIN", &["yellow"], None);
        let penalty_2min = new_toggle_button("2 MIN", &["orange"], None);
        let penalty_5min = new_toggle_button("5 MIN", &["red"], None);
        let penalty_dismiss = new_toggle_button("DISMISS", &["blue"], None);

        let penalty_delete = new_button("DELETE", &["red"], None);
        let penalty_add = new_button("ADD", &["green"], None);
        let penalty_white_timeout = new_button("WHITE\nTIMEOUT", &["white"], None);
        let penalty_referee_timeout = new_button("REFEREE TIMEOUT", &["yellow"], None);
        let penalty_black_timeout = new_button("BLACK\nTIMEOUT", &["black"], None);

        let (penalty_keypad, _penalty_player_number) = new_keypad();

        penalty_add_layout.attach(&penalty_keypad, 0, 0, 4, 9);
        penalty_add_layout.attach(&penalty_white_select, 4, 0, 4, 3);
        penalty_add_layout.attach(&penalty_black_select, 8, 0, 4, 3);
        penalty_add_layout.attach(&penalty_1min, 4, 3, 2, 4);
        penalty_add_layout.attach(&penalty_2min, 6, 3, 2, 4);
        penalty_add_layout.attach(&penalty_5min, 8, 3, 2, 4);
        penalty_add_layout.attach(&penalty_dismiss, 10, 3, 2, 4);
        penalty_add_layout.attach(&penalty_delete, 4, 7, 4, 2);
        penalty_add_layout.attach(&penalty_add, 8, 7, 4, 2);
        penalty_add_layout.attach(&penalty_white_timeout, 0, 9, 3, 2);
        penalty_add_layout.attach(&penalty_referee_timeout, 3, 9, 6, 2);
        penalty_add_layout.attach(&penalty_black_timeout, 9, 9, 3, 2);

        // Setting up the white/black selected buttons
        let penalty_black_select_ = penalty_black_select.clone();
        let penalty_white_select_ = penalty_white_select.clone();

        penalty_black_select.connect_clicked(move |b| {
            if b.get_active() {
                penalty_white_select_.set_active(false);
            } else if !penalty_white_select_.get_active() {
                b.set_active(true);
            }
        });

        penalty_white_select.connect_clicked(move |b| {
            if b.get_active() {
                penalty_black_select_.set_active(false);
            } else if !penalty_black_select_.get_active() {
                b.set_active(true);
            }
        });

        // Setting up the time penalty selected buttons
        let penalty_2min_ = penalty_2min.clone();
        let penalty_5min_ = penalty_5min.clone();
        let penalty_dismiss_ = penalty_dismiss.clone();

        penalty_1min.connect_clicked(move |b| {
            if b.get_active() {
                penalty_2min_.set_active(false);
                penalty_5min_.set_active(false);
                penalty_dismiss_.set_active(false);
            } else if !penalty_2min_.get_active()
                && !penalty_5min_.get_active()
                && !penalty_dismiss_.get_active()
            {
                b.set_active(true);
            }
        });

        let penalty_1min_ = penalty_1min.clone();
        let penalty_5min_ = penalty_5min.clone();
        let penalty_dismiss_ = penalty_dismiss.clone();

        penalty_2min.connect_clicked(move |b| {
            if b.get_active() {
                penalty_1min_.set_active(false);
                penalty_5min_.set_active(false);
                penalty_dismiss_.set_active(false);
            } else if !penalty_1min_.get_active()
                && !penalty_5min_.get_active()
                && !penalty_dismiss_.get_active()
            {
                b.set_active(true);
            }
        });

        let penalty_1min_ = penalty_1min.clone();
        let penalty_2min_ = penalty_2min.clone();
        let penalty_dismiss_ = penalty_dismiss.clone();

        penalty_5min.connect_clicked(move |b| {
            if b.get_active() {
                penalty_1min_.set_active(false);
                penalty_2min_.set_active(false);
                penalty_dismiss_.set_active(false);
            } else if !penalty_1min_.get_active()
                && !penalty_2min_.get_active()
                && !penalty_dismiss_.get_active()
            {
                b.set_active(true);
            }
        });

        penalty_dismiss.connect_clicked(move |b| {
            if b.get_active() {
                penalty_1min.set_active(false);
                penalty_2min.set_active(false);
                penalty_5min.set_active(false);
            } else if !penalty_1min.get_active()
                && !penalty_2min.get_active()
                && !penalty_5min.get_active()
            {
                b.set_active(true);
            }
        });

        //
        //
        // Time Edit Page
        //
        let time_edit_layout = gtk::Grid::new();
        time_edit_layout.set_column_homogeneous(true);
        time_edit_layout.set_row_homogeneous(true);
        time_edit_layout.set_margin_top(BUTTON_MARGIN);
        time_edit_layout.set_margin_start(BUTTON_MARGIN);
        time_edit_layout.set_margin_end(BUTTON_MARGIN);
        time_edit_layout.set_margin_bottom(BUTTON_MARGIN);
        time_edit_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        time_edit_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let minute_plus = new_button("+", &["blue-modifier"], None);
        let minute_minus = new_button("-", &["blue-modifier"], None);
        let second_plus = new_button("+", &["blue-modifier"], None);
        let second_minus = new_button("-", &["blue-modifier"], None);
        let time_edit_cancel = new_button("CANCEL", &["red"], None);
        let time_edit_submit = new_button("SUBMIT", &["green"], None);
        let time_edit_white_timeout = new_button("WHITE\nTIMEOUT", &["white"], None);
        let time_edit_referee_timeout = new_button("REFEREE TIMEOUT", &["yellow"], None);
        let time_edit_black_timeout = new_button("BLACK\nTIMEOUT", &["black"], None);

        let minute_header = new_label("MINUTE", "header-gray");
        let second_header = new_label("SECOND", "header-gray");
        let new_time_header = new_label("NEW TIME", "header-gray");
        let modified_game_time = new_label("##:##", "modified-time-gray");
        let empty_time_edit_label = gtk::Label::new(None);

        let minute_header_box = gtk::Grid::new();
        minute_header_box.set_column_homogeneous(true);
        minute_header_box.set_row_homogeneous(true);
        minute_header_box.attach(&minute_header, 0, 0, 1, 1);
        minute_header_box.attach(&minute_plus, 0, 1, 1, 2);

        let new_time_header_box = gtk::Grid::new();
        new_time_header_box.set_column_homogeneous(true);
        new_time_header_box.set_row_homogeneous(true);
        new_time_header_box.attach(&new_time_header, 0, 0, 1, 1);
        new_time_header_box.attach(&modified_game_time, 0, 1, 1, 2);

        let second_header_box = gtk::Grid::new();
        second_header_box.set_column_homogeneous(true);
        second_header_box.set_row_homogeneous(true);
        second_header_box.attach(&second_header, 0, 0, 1, 1);
        second_header_box.attach(&second_plus, 0, 1, 1, 2);

        time_edit_layout.attach(&minute_header_box, 0, 0, 3, 3);
        time_edit_layout.attach(&new_time_header_box, 3, 0, 6, 3);
        time_edit_layout.attach(&second_header_box, 9, 0, 3, 3);
        time_edit_layout.attach(&minute_minus, 0, 3, 3, 2);
        time_edit_layout.attach(&second_minus, 9, 3, 3, 2);
        time_edit_layout.attach(&empty_time_edit_label, 0, 5, 12, 2);
        time_edit_layout.attach(&time_edit_cancel, 0, 7, 4, 2);
        time_edit_layout.attach(&time_edit_submit, 8, 7, 4, 2);
        time_edit_layout.attach(&time_edit_white_timeout, 0, 9, 3, 2);
        time_edit_layout.attach(&time_edit_referee_timeout, 3, 9, 6, 2);
        time_edit_layout.attach(&time_edit_black_timeout, 9, 9, 3, 2);

        let modified_game_time_ = modified_game_time.clone();
        let get_displayed_time = move || {
            let label = modified_game_time_.get_label().unwrap();
            let current: Vec<&str> = label.as_str().split(':').collect();
            assert_eq!(2, current.len());
            current[0].trim().parse::<u64>().unwrap() * 60 + current[1].parse::<u64>().unwrap()
        };

        let modified_game_time_ = modified_game_time.clone();
        let get_displayed_time_ = get_displayed_time.clone();
        minute_plus.connect_clicked(move |_| {
            modified_game_time_.set_label(&secs_to_time_string(
                get_displayed_time_().saturating_add(60),
            ))
        });

        let modified_game_time_ = modified_game_time.clone();
        let get_displayed_time_ = get_displayed_time.clone();
        minute_minus.connect_clicked(move |_| {
            modified_game_time_.set_label(&secs_to_time_string(
                get_displayed_time_().saturating_sub(60),
            ))
        });

        let modified_game_time_ = modified_game_time.clone();
        let get_displayed_time_ = get_displayed_time.clone();
        second_plus.connect_clicked(move |_| {
            modified_game_time_.set_label(&secs_to_time_string(
                get_displayed_time_().saturating_add(1),
            ))
        });

        let modified_game_time_ = modified_game_time.clone();
        let get_displayed_time_ = get_displayed_time.clone();
        second_minus.connect_clicked(move |_| {
            modified_game_time_.set_label(&secs_to_time_string(
                get_displayed_time_().saturating_sub(1),
            ))
        });

        //
        //
        // Game Over Confirmation Page
        //

        //
        //
        // Edit Game Information Page
        //
        let edit_game_information_layout = gtk::Grid::new();
        edit_game_information_layout.set_column_homogeneous(true);
        edit_game_information_layout.set_row_homogeneous(true);
        edit_game_information_layout.set_margin_top(BUTTON_MARGIN);
        edit_game_information_layout.set_margin_start(BUTTON_MARGIN);
        edit_game_information_layout.set_margin_end(BUTTON_MARGIN);
        edit_game_information_layout.set_margin_bottom(BUTTON_MARGIN);
        edit_game_information_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        edit_game_information_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let edit_game_information_submit = new_button("SUBMIT", &["green"], None);
        let edit_uwhscores = new_button("EDIT UWHSCORES", &["blue"], None);

        edit_game_information_layout.attach(&edit_game_information_submit, 0, 0, 1, 1);
        edit_game_information_layout.attach(&edit_uwhscores, 0, 1, 1, 1);

        //
        //
        // Edit Game Parameters Page
        //
        let edit_game_parameters_layout = gtk::Grid::new();
        edit_game_parameters_layout.set_column_homogeneous(true);
        edit_game_parameters_layout.set_row_homogeneous(true);
        edit_game_parameters_layout.set_margin_top(BUTTON_MARGIN);
        edit_game_parameters_layout.set_margin_start(BUTTON_MARGIN);
        edit_game_parameters_layout.set_margin_end(BUTTON_MARGIN);
        edit_game_parameters_layout.set_margin_bottom(BUTTON_MARGIN);
        edit_game_parameters_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        edit_game_parameters_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let edit_game_parameters_ot_yes = new_toggle_button("YES", &["little-green"], None);
        let edit_game_parameters_ot_no = new_toggle_button("NO", &["little-red"], None);
        let edit_game_parameters_sd_yes = new_toggle_button("YES", &["little-green"], None);
        let edit_game_parameters_sd_no = new_toggle_button("NO", &["little-red"], None);
        let edit_game_parameters_type = new_toggle_button("DISABLE", &["blue"], None);

        let edit_game_parameters_cancel = new_button("CANCEL", &["red"], None);
        let edit_game_parameters_submit = new_button("SUBMIT", &["green"], None);

        let edit_half_length_label = new_label("HALF LENGTH", "edit-parameter-header");
        let edit_halftime_length_label = new_label("HALF-TIME", "edit-parameter-header");
        let edit_between_game_length_label = new_label("BETWEEN", "edit-parameter-header");
        let edit_overtime_allow_label = new_label("OVERTIME ALLOWED?", "edit-parameter-header");
        let edit_min_between_game_length_label = new_label("MIN BETWEEN", "edit-parameter-header");
        let edit_pre_overtime_length_label =
            new_label("PRE-OVERTIME BREAK", "edit-parameter-header");
        let edit_overtime_half_length_label =
            new_label("OVERTIME HALF LENGTH", "edit-parameter-header");
        let edit_overtime_halftime_length_label =
            new_label("OVERTIME HALF-TIME LENGTH", "edit-parameter-header");
        let edit_sudden_death_allow_label =
            new_label("SUDDEN DEATH ALLOWED?", "edit-parameter-header");
        let edit_pre_sudden_death_length_label =
            new_label("PRE-SUDDEN DEATH BREAK", "edit-parameter-header");

        let half_length_label = new_label("15:00", "edit-parameter-time");
        let halftime_length_label = new_label("3:00", "edit-parameter-time");
        let between_game_length_label = new_label("8:00", "edit-parameter-time");
        let min_between_game_length_label = new_label("4:00", "edit-parameter-time");
        let pre_overtime_length_label = new_label("3:00", "edit-parameter-time");
        let overtime_half_length_label = new_label("5:00", "edit-parameter-time");
        let overtime_halftime_length_label = new_label("1:00", "edit-parameter-time");
        let pre_sudden_death_length_label = new_label("1:00", "edit-parameter-time");

        let half_length_time_edit = time_edit_ribbon();
        let halftime_length_time_edit = time_edit_ribbon();
        let between_game_length_time_edit = time_edit_ribbon();
        let min_between_game_length_time_edit = time_edit_ribbon();
        let pre_overtime_length_time_edit = time_edit_ribbon();
        let overtime_half_length_time_edit = time_edit_ribbon();
        let overtime_halftime_length_time_edit = time_edit_ribbon();
        let pre_sudden_death_length_time_edit = time_edit_ribbon();

        edit_game_parameters_layout.attach(&edit_half_length_label, 0, 0, 6, 1);
        edit_game_parameters_layout.attach(&edit_halftime_length_label, 0, 1, 6, 1);
        edit_game_parameters_layout.attach(&edit_between_game_length_label, 0, 2, 6, 1);
        edit_game_parameters_layout.attach(&edit_min_between_game_length_label, 0, 3, 6, 1);
        edit_game_parameters_layout.attach(&edit_overtime_allow_label, 0, 4, 6, 1);
        edit_game_parameters_layout.attach(&edit_pre_overtime_length_label, 0, 5, 6, 1);
        edit_game_parameters_layout.attach(&edit_overtime_half_length_label, 0, 6, 6, 1);
        edit_game_parameters_layout.attach(&edit_overtime_halftime_length_label, 0, 7, 6, 1);
        edit_game_parameters_layout.attach(&edit_sudden_death_allow_label, 0, 8, 6, 1);
        edit_game_parameters_layout.attach(&edit_pre_sudden_death_length_label, 0, 9, 6, 1);

        edit_game_parameters_layout.attach(&half_length_label, 6, 0, 2, 1);
        edit_game_parameters_layout.attach(&halftime_length_label, 6, 1, 2, 1);
        edit_game_parameters_layout.attach(&between_game_length_label, 6, 2, 2, 1);
        edit_game_parameters_layout.attach(&min_between_game_length_label, 6, 3, 2, 1);
        edit_game_parameters_layout.attach(&pre_overtime_length_label, 6, 5, 2, 1);
        edit_game_parameters_layout.attach(&overtime_half_length_label, 6, 6, 2, 1);
        edit_game_parameters_layout.attach(&overtime_halftime_length_label, 6, 7, 2, 1);
        edit_game_parameters_layout.attach(&pre_sudden_death_length_label, 6, 9, 2, 1);

        edit_game_parameters_layout.attach(&half_length_time_edit, 8, 0, 4, 1);
        edit_game_parameters_layout.attach(&halftime_length_time_edit, 8, 1, 4, 1);
        edit_game_parameters_layout.attach(&between_game_length_time_edit, 8, 2, 4, 1);
        edit_game_parameters_layout.attach(&min_between_game_length_time_edit, 8, 3, 4, 1);
        edit_game_parameters_layout.attach(&edit_game_parameters_ot_no, 8, 4, 2, 1);
        edit_game_parameters_layout.attach(&edit_game_parameters_ot_yes, 10, 4, 2, 1);
        edit_game_parameters_layout.attach(&pre_overtime_length_time_edit, 8, 5, 4, 1);
        edit_game_parameters_layout.attach(&overtime_half_length_time_edit, 8, 6, 4, 1);
        edit_game_parameters_layout.attach(&overtime_halftime_length_time_edit, 8, 7, 4, 1);
        edit_game_parameters_layout.attach(&edit_game_parameters_sd_no, 8, 8, 2, 1);
        edit_game_parameters_layout.attach(&edit_game_parameters_sd_yes, 10, 8, 2, 1);
        edit_game_parameters_layout.attach(&pre_sudden_death_length_time_edit, 8, 9, 4, 1);

        edit_game_parameters_layout.attach(&edit_game_parameters_cancel, 0, 10, 4, 2);
        edit_game_parameters_layout.attach(&edit_game_parameters_type, 4, 10, 4, 2);
        edit_game_parameters_layout.attach(&edit_game_parameters_submit, 8, 10, 4, 2);

        // Setting initial status
        edit_game_parameters_ot_yes.set_active(true);
        edit_game_parameters_sd_yes.set_active(true);

        // Selecting Overtime buttons
        let mut ot_edit_buttons = pre_overtime_length_time_edit.get_children();
        ot_edit_buttons.append(&mut overtime_half_length_time_edit.get_children());
        ot_edit_buttons.append(&mut overtime_halftime_length_time_edit.get_children());
        ot_edit_buttons.push(edit_pre_overtime_length_label.upcast::<gtk::Widget>());
        ot_edit_buttons.push(pre_overtime_length_label.upcast::<gtk::Widget>());
        ot_edit_buttons.push(edit_overtime_half_length_label.upcast::<gtk::Widget>());
        ot_edit_buttons.push(overtime_half_length_label.upcast::<gtk::Widget>());
        ot_edit_buttons.push(edit_overtime_halftime_length_label.upcast::<gtk::Widget>());
        ot_edit_buttons.push(overtime_halftime_length_label.upcast::<gtk::Widget>());

        let edit_game_parameters_ot_no_ = edit_game_parameters_ot_no.clone();
        let ot_edit_buttons_: Vec<_> = ot_edit_buttons.to_vec();

        edit_game_parameters_ot_yes.connect_clicked(move |b| {
            if b.get_active() {
                edit_game_parameters_ot_no_.set_active(false);
                for button in &ot_edit_buttons_ {
                    button.set_sensitive(true);
                }
            } else if !edit_game_parameters_ot_no_.get_active() {
                b.set_active(true);
            }
        });

        let ot_edit_buttons_: Vec<_> = ot_edit_buttons.to_vec();
        let edit_game_parameters_ot_yes_ = edit_game_parameters_ot_yes.clone();

        edit_game_parameters_ot_no.connect_clicked(move |b| {
            if b.get_active() {
                edit_game_parameters_ot_yes_.set_active(false);
                for button in &ot_edit_buttons_ {
                    button.set_sensitive(false);
                }
            } else if !edit_game_parameters_ot_yes_.get_active() {
                b.set_active(true);
            }
        });

        // Selecting Sudden Death buttons
        let mut sd_edit_buttons = pre_sudden_death_length_time_edit.get_children();
        sd_edit_buttons.push(edit_pre_sudden_death_length_label.upcast::<gtk::Widget>());
        sd_edit_buttons.push(pre_sudden_death_length_label.upcast::<gtk::Widget>());

        let edit_game_parameters_sd_no_ = edit_game_parameters_sd_no.clone();
        let sd_edit_buttons_: Vec<_> = sd_edit_buttons.to_vec();

        edit_game_parameters_sd_yes.connect_clicked(move |b| {
            if b.get_active() {
                edit_game_parameters_sd_no_.set_active(false);
                for button in &sd_edit_buttons_ {
                    button.set_sensitive(true);
                }
            } else if !edit_game_parameters_sd_no_.get_active() {
                b.set_active(true);
            }
        });

        let edit_game_parameters_sd_yes_ = edit_game_parameters_sd_yes.clone();
        let sd_edit_buttons_: Vec<_> = sd_edit_buttons.to_vec();

        edit_game_parameters_sd_no.connect_clicked(move |b| {
            if b.get_active() {
                edit_game_parameters_sd_yes_.set_active(false);
                for button in &sd_edit_buttons_ {
                    button.set_sensitive(false);
                }
            } else if !edit_game_parameters_sd_yes_.get_active() {
                b.set_active(true);
            }
        });

        // Selecting Enable/Disable All button

        let ot_edit_buttons_ = ot_edit_buttons.to_vec();
        let sd_edit_buttons_ = sd_edit_buttons.to_vec();
        let edit_game_parameters_ot_no_ = edit_game_parameters_ot_no.clone();
        let edit_game_parameters_sd_no_ = edit_game_parameters_sd_no.clone();

        let mut all_parameter_widgets = ot_edit_buttons_.to_vec();
        all_parameter_widgets.append(&mut sd_edit_buttons_.to_vec());
        all_parameter_widgets.append(&mut half_length_time_edit.get_children());
        all_parameter_widgets.append(&mut halftime_length_time_edit.get_children());
        all_parameter_widgets.append(&mut between_game_length_time_edit.get_children());
        all_parameter_widgets.append(&mut min_between_game_length_time_edit.get_children());
        all_parameter_widgets.push(edit_game_parameters_ot_no_.upcast::<gtk::Widget>());
        all_parameter_widgets.push(edit_game_parameters_ot_yes.upcast::<gtk::Widget>());
        all_parameter_widgets.push(edit_game_parameters_sd_no_.upcast::<gtk::Widget>());
        all_parameter_widgets.push(edit_game_parameters_sd_yes.upcast::<gtk::Widget>());

        all_parameter_widgets.push(edit_half_length_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(half_length_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(edit_halftime_length_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(halftime_length_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(edit_between_game_length_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(between_game_length_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(edit_min_between_game_length_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(min_between_game_length_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(edit_overtime_allow_label.upcast::<gtk::Widget>());
        all_parameter_widgets.push(edit_sudden_death_allow_label.upcast::<gtk::Widget>());

        let all_parameter_widgets_: Vec<_> = all_parameter_widgets.to_vec();

        //        let ot_edit_buttons_ = ot_edit_buttons.to_vec();
        //        let sd_edit_buttons_ = sd_edit_buttons.to_vec();

        edit_game_parameters_type.connect_clicked(move |b| {
            if b.get_active() {
                for button in &all_parameter_widgets_ {
                    b.get_child()
                        .unwrap()
                        .downcast::<gtk::Label>()
                        .unwrap()
                        .set_label("ENABLE");
                    button.set_sensitive(false);
                }
            } else {
                for button in &all_parameter_widgets_ {
                    b.get_child()
                        .unwrap()
                        .downcast::<gtk::Label>()
                        .unwrap()
                        .set_label("DISABLE");
                    button.set_sensitive(true);
                }
                if edit_game_parameters_ot_no.get_active() {
                    for button1 in &ot_edit_buttons_ {
                        button1.set_sensitive(false);
                    }
                }
                if edit_game_parameters_sd_no.get_active() {
                    for button2 in &sd_edit_buttons_ {
                        button2.set_sensitive(false);
                    }
                }
            }
        });

        //
        //
        // Roster Edit Page
        //

        //
        //
        // UWHSCores Edit Page
        //
        let uwhscores_edit_layout = gtk::Box::new(gtk::Orientation::Vertical, BUTTON_SPACING);
        uwhscores_edit_layout.set_margin_top(BUTTON_MARGIN);
        uwhscores_edit_layout.set_margin_start(BUTTON_MARGIN);
        uwhscores_edit_layout.set_margin_bottom(BUTTON_MARGIN);
        uwhscores_edit_layout.set_margin_end(BUTTON_MARGIN);

        let uwhscores_submit = new_button("SUBMIT", &["green"], None);

        uwhscores_edit_layout.pack_start(&uwhscores_submit, false, false, 0);

        //
        //
        // Build the Stack, which switches between screen layouts
        //
        let layout_stack = gtk::Stack::new();
        layout_stack.add_named(&start_layout, "Start Layout");
        layout_stack.add_named(&main_layout, "Main Layout");
        layout_stack.add_named(&time_edit_layout, "Time Edit Layout");
        layout_stack.add_named(
            &edit_game_information_layout,
            "Edit Game Information Layout",
        );
        layout_stack.add_named(&edit_game_parameters_layout, "Edit Game Parameters");
        layout_stack.add_named(&new_score_layout, "New Score Layout");
        layout_stack.add_named(&penalty_add_layout, "Penalty Add/Edit Layout");
        layout_stack.add_named(
            &time_penalty_conf_layout,
            "Time Penalty Confirmation Layout",
        );
        layout_stack.add_named(&edit_score_layout, "Edit Score Layout");
        layout_stack.add_named(&uwhscores_edit_layout, "UWH Scores Layout");

        // Set up the buttons to switch between layouts
        let clock_was_running = Arc::new(AtomicBool::new(false));

        //
        //
        // Buttons for moving back to the Main Layout
        //

        let main_layout_ = main_layout.clone(); // need this first clone part for all but the last call to that page
        let layout_stack_ = layout_stack.clone();
        let edit_game_parameters_type_ = edit_game_parameters_type.clone();
        choose_auto.connect_clicked(move |_| {
            edit_game_parameters_type_.set_active(true);
            layout_stack_.set_visible_child(&main_layout_); //need the _ at the end for all except the last call to that page
        });

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        uwhscores_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_information_submit
            .connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        let score_player_number_ = score_player_number.clone();
        let score_white_select_ = score_white_select.clone();
        let tm_ = tm.clone();
        let state_send_ = state_send.clone();
        score_submit.connect_clicked(move |_| {
            let player = score_player_number_
                .get_label()
                .unwrap()
                .as_str()
                .lines()
                .last()
                .unwrap()
                .trim()
                .parse::<u8>()
                .unwrap_or(std::u8::MAX);
            let now = Instant::now();
            let mut tm = tm_.lock().unwrap();
            if score_white_select_.get_active() {
                tm.add_w_score(player, now);
            } else {
                tm.add_b_score(player, now);
            }
            state_send_
                .send((tm.generate_snapshot(now).unwrap(), false))
                .unwrap();
            layout_stack_.set_visible_child(&main_layout_)
        });

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        score_cancel.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        let tm_ = tm.clone();
        let modified_white_score_ = modified_white_score.clone();
        let modified_black_score_ = modified_black_score.clone();
        score_edit_cancel.connect_clicked(move |_| {
            let tm = tm_.lock().unwrap();
            modified_white_score_.set_label(&format!("{}", tm.get_w_score()));
            modified_black_score_.set_label(&format!("{}", tm.get_b_score()));
            layout_stack_.set_visible_child(&main_layout_)
        });

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        let tm_ = tm.clone();
        let state_send_ = state_send.clone();
        let modified_white_score_ = modified_white_score.clone();
        let modified_black_score_ = modified_black_score.clone();
        score_edit_submit.connect_clicked(move |_| {
            let w_score = modified_white_score_
                .get_label()
                .unwrap()
                .as_str()
                .parse::<u8>()
                .unwrap();
            let b_score = modified_black_score_
                .get_label()
                .unwrap()
                .as_str()
                .parse::<u8>()
                .unwrap();

            let now = Instant::now();
            let mut tm = tm_.lock().unwrap();
            tm.set_scores(b_score, w_score, now);
            state_send_
                .send((tm.generate_snapshot(now).unwrap(), false))
                .unwrap();
            layout_stack_.set_visible_child(&main_layout_)
        });

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        let tm_ = tm.clone();
        let clock_was_running_ = clock_was_running.clone();
        time_edit_cancel.connect_clicked(move |_| {
            if clock_was_running_.load(Ordering::SeqCst) {
                tm_.lock().unwrap().start_clock(Instant::now());
            }
            layout_stack_.set_visible_child(&main_layout_)
        });

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        let state_send_ = state_send.clone();
        let tm_ = tm.clone();
        let clock_was_running_ = clock_was_running.clone();
        time_edit_submit.connect_clicked(move |_| {
            let mut tm = tm_.lock().unwrap();
            tm.set_game_clock_time(Duration::from_secs(get_displayed_time()))
                .unwrap();
            if clock_was_running_.load(Ordering::SeqCst) {
                tm.start_clock(Instant::now());
            } else {
                state_send_
                    .send((tm.generate_snapshot(Instant::now()).unwrap(), false))
                    .unwrap();
            }
            layout_stack_.set_visible_child(&main_layout_)
        });

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_parameters_cancel
            .connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_parameters_submit
            .connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        penalty_conf_cancel
            .connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let layout_stack_ = layout_stack.clone();
        penalty_conf_start.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout));

        //
        //
        // Buttons for navigating between Layouts that are not Main Layout
        //
        // move to edit_time_layout
        let layout_stack_ = layout_stack.clone();
        let tm_ = tm.clone();
        edit_game_time.connect_clicked(move |_| {
            let mut tm = tm_.lock().unwrap();
            let now = Instant::now();
            tm.update(now);
            clock_was_running.store(tm.clock_is_running(), Ordering::SeqCst);
            tm.stop_clock(now).unwrap();
            modified_game_time.set_label(&secs_to_time_string(
                tm.game_clock_time(now).unwrap().as_secs(),
            ));
            layout_stack_.set_visible_child(&time_edit_layout);
        });

        // move to new_score_layout
        let new_score_layout_ = new_score_layout.clone();
        let layout_stack_ = layout_stack.clone();
        let score_player_number_ = score_player_number.clone();
        add_white_score.connect_clicked(move |_| {
            score_white_select.set_active(true);
            score_player_number_.set_label("Player #:\n");
            layout_stack_.set_visible_child(&new_score_layout_);
        });

        let layout_stack_ = layout_stack.clone();
        add_black_score.connect_clicked(move |_| {
            score_black_select.set_active(true);
            score_player_number.set_label("Player #:\n");
            layout_stack_.set_visible_child(&new_score_layout);
        });

        // move to edit_game_parameters_layout
        let edit_game_parameters_layout_ = edit_game_parameters_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_parameters.connect_clicked(move |_| {
            layout_stack_.set_visible_child(&edit_game_parameters_layout_)
        });

        let layout_stack_ = layout_stack.clone();
        choose_manual.connect_clicked(move |_| {
            edit_game_parameters_type.set_active(false);
            layout_stack_.set_visible_child(&edit_game_parameters_layout)
        });

        // move to edit_score_layout
        let edit_score_layout_ = edit_score_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_white_score
            .connect_clicked(move |_| layout_stack_.set_visible_child(&edit_score_layout_));

        let layout_stack_ = layout_stack.clone();
        edit_black_score
            .connect_clicked(move |_| layout_stack_.set_visible_child(&edit_score_layout));

        // move to edit_game_information_layout
        let layout_stack_ = layout_stack.clone();
        edit_game_information.connect_clicked(move |_| {
            layout_stack_.set_visible_child(&edit_game_information_layout)
        });

        // move to time_penalty_add_layout
        let layout_stack_ = layout_stack.clone();
        penalty_conf_new
            .connect_clicked(move |_| layout_stack_.set_visible_child(&penalty_add_layout));

        // move to time_penalty_conf_layout
        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_white_time_penalty.connect_clicked(move |_| {
            penalty_white_select.set_active(true);
            layout_stack_.set_visible_child(&time_penalty_conf_layout_)
        });

        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_black_time_penalty.connect_clicked(move |_| {
            penalty_black_select.set_active(true);
            layout_stack_.set_visible_child(&time_penalty_conf_layout_)
        });

        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        penalty_delete
            .connect_clicked(move |_| layout_stack_.set_visible_child(&time_penalty_conf_layout_));

        let layout_stack_ = layout_stack.clone();
        penalty_add
            .connect_clicked(move |_| layout_stack_.set_visible_child(&time_penalty_conf_layout));

        // move to uwhscores_edit_layout
        let layout_stack_ = layout_stack.clone();
        edit_uwhscores
            .connect_clicked(move |_| layout_stack_.set_visible_child(&uwhscores_edit_layout));

        //
        //
        // Connect to the backend
        //
        let tm_ = tm.clone();
        let state_send_ = state_send.clone();
        main_referee_timeout.connect_clicked(move |b| {
            let mut tm = tm_.lock().unwrap();
            match b.get_label().unwrap().as_str() {
                "REFEREE TIMEOUT" | "START REFEREE TIMEOUT" => {
                    debug!("Button starting Ref timeout");
                    tm.start_ref_timeout(Instant::now()).unwrap() // TODO: Get rid of unwrap here
                }
                "SWITCH TO REFEREE TIMEOUT" => {
                    debug!("Button switching to Penalty Shot");
                    tm.switch_to_ref_timeout().unwrap()
                }
                "SWITCH TO PENALTY SHOT" => {
                    debug!("Button switching to Penalty Shot");
                    tm.switch_to_penalty_shot().unwrap()
                }
                "START" => {
                    debug!("Button starting clock for first time");
                    tm.start_clock(Instant::now())
                }
                l => panic!("Unknown button label: {}", l),
            }
            state_send_
                .send((tm.generate_snapshot(Instant::now()).unwrap(), false))
                .unwrap();
        });

        let tm_ = tm.clone();
        let state_send_ = state_send.clone();
        new_penalty_shot.connect_clicked(move |b| {
            let mut tm = tm_.lock().unwrap();
            match b.get_label().unwrap().as_str() {
                "RESUME" => {
                    debug!("Button starting clock");
                    tm.end_timeout(Instant::now()).unwrap()
                }
                "PENALTY SHOT" => {
                    debug!("Button starting Penalty Shot");
                    tm.start_penalty_shot(Instant::now()).unwrap()
                }
                l => panic!("Unknown button label: {}", l),
            }
            state_send_
                .send((tm.generate_snapshot(Instant::now()).unwrap(), false))
                .unwrap();
        });

        let tm_ = tm.clone();
        let state_send_ = state_send.clone();
        main_white_timeout.connect_clicked(move |b| {
            let mut tm = tm_.lock().unwrap();
            match b.get_label().unwrap().as_str() {
                "SWITCH TO\nWHITE" => {
                    debug!("Button switching to White timeout");
                    tm.switch_to_w_timeout().unwrap()
                }
                "START\nWHITE T/O" | "WHITE\nTIMEOUT" => {
                    debug!("Button starting a White timeout");
                    tm.start_w_timeout(Instant::now()).unwrap()
                }
                l => panic!("Unknown button label: {}", l),
            }
            state_send_
                .send((tm.generate_snapshot(Instant::now()).unwrap(), false))
                .unwrap();
        });

        let tm_ = tm.clone();
        let state_send_ = state_send.clone();
        main_black_timeout.connect_clicked(move |b| {
            let mut tm = tm_.lock().unwrap();
            match b.get_label().unwrap().as_str() {
                "SWITCH TO\nBLACK" => {
                    debug!("Button switching to Black timeout");
                    tm.switch_to_b_timeout().unwrap()
                }
                "START\nBLACK T/O" | "BLACK\nTIMEOUT" => {
                    debug!("Button starting a White timeout");
                    tm.start_b_timeout(Instant::now()).unwrap()
                }
                l => panic!("Unknown button label: {}", l),
            }
            state_send_
                .send((tm.generate_snapshot(Instant::now()).unwrap(), false))
                .unwrap();
        });

        // start a thread that updates the tm every second and sends the result to the UI
        let (clock_running_send, clock_running_recv) = mpsc::channel();
        clock_running_send.send(false).unwrap();
        tm.lock().unwrap().add_start_stop_sender(clock_running_send);
        let tm_ = tm.clone();
        let mut just_started = false;
        thread::spawn(move || {
            let mut timeout = Duration::from_secs(1);

            let update_and_send_snapshot =
                move |tm: &mut MutexGuard<TournamentManager>, just_started: bool| {
                    let now = Instant::now();
                    tm.update(now);
                    if let Some(snapshot) = tm.generate_snapshot(now) {
                        trace!("Updater: sending snapshot");
                        state_send.send((snapshot, just_started)).unwrap();
                    } else {
                        panic!("Failed to generate snapshot");
                    }
                    now
                };

            loop {
                match clock_running_recv.recv_timeout(timeout) {
                    Ok(false) => loop {
                        trace!("Updater: locking tm");
                        update_and_send_snapshot(&mut tm_.lock().unwrap(), just_started);
                        info!("Updater: Waiting for Clock to start");
                        if clock_running_recv.recv().unwrap() {
                            info!("Updater: Clock has restarted");
                            timeout = Duration::from_secs(0);
                            just_started = true;
                            break;
                        }
                    },
                    Err(RecvTimeoutError::Disconnected) => break,
                    Ok(true) | Err(RecvTimeoutError::Timeout) => {
                        trace!("Updater: locking tm");
                        let mut tm = tm_.lock().unwrap();
                        let now = update_and_send_snapshot(&mut tm, just_started);
                        just_started = false;
                        if let Some(nanos) = tm.nanos_to_update(now) {
                            debug!("Updater: waiting for up to {} ns", nanos);
                            timeout = Duration::from_nanos(nanos.into());
                        } else {
                            panic!("Failed to get nanos to update");
                        }
                    }
                }
            }
        });

        // Update the gui when a snapshot is received
        let mut last_snapshot = tm
            .lock()
            .unwrap()
            .generate_snapshot(Instant::now())
            .unwrap();
        last_snapshot.w_score = std::u8::MAX;
        last_snapshot.b_score = std::u8::MAX;

        let tm_ = tm.clone();
        state_recv.attach(None, move |(snapshot, just_started)| {
            edit_game_time.set_label(&secs_to_time_string(match snapshot.timeout {
                TimeoutSnapshot::White(t)
                | TimeoutSnapshot::Black(t)
                | TimeoutSnapshot::Ref(t)
                | TimeoutSnapshot::PenaltyShot(t) => t,
                TimeoutSnapshot::None => snapshot.secs_in_period,
            }));

            if (snapshot.current_period != last_snapshot.current_period)
                | (snapshot.timeout != last_snapshot.timeout)
                | just_started
            {
                let (game_header, ref_t_o_text, p_s_text, w_t_o_text, b_t_o_text) =
                    match snapshot.timeout {
                        TimeoutSnapshot::Black(_) => (
                            "BLACK TIMEOUT",
                            "START REFEREE TIMEOUT",
                            "RESUME",
                            "SWITCH TO\nWHITE",
                            "SWITCH TO\nBLACK",
                        ),
                        TimeoutSnapshot::White(_) => (
                            "WHITE TIMEOUT",
                            "START REFEREE TIMEOUT",
                            "RESUME",
                            "SWITCH TO\nWHITE",
                            "SWITCH TO\nBLACK",
                        ),
                        TimeoutSnapshot::Ref(_) => (
                            "REFEREE TIMEOUT",
                            "SWITCH TO PENALTY SHOT",
                            "RESUME",
                            "START\nWHITE T/O",
                            "START\nBLACK T/O",
                        ),
                        TimeoutSnapshot::PenaltyShot(_) => (
                            "PENALTY SHOT",
                            "SWITCH TO REFEREE TIMEOUT",
                            "RESUME",
                            "START\nWHITE T/O",
                            "START\nBLACK T/O",
                        ),
                        TimeoutSnapshot::None => match snapshot.current_period {
                            GamePeriod::BetweenGames => (
                                "NEXT GAME IN",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::FirstHalf => (
                                "FIRST HALF",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::HalfTime => (
                                "HALF TIME",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::SecondHalf => (
                                "SECOND HALF",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::PreOvertime => (
                                "PRE OVERTIME BREAK",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::OvertimeFirstHalf => (
                                "OVERTIME FIRST HALF",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::OvertimeHalfTime => (
                                "OVERTIME HALF TIME",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::OvertimeSecondHalf => (
                                "OVERTIME SECOND HALF",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::PreSuddenDeath => (
                                "PRE SUDDEN DEATH BREAK",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                            GamePeriod::SuddenDeath => (
                                "SUDDEN DEATH",
                                "REFEREE TIMEOUT",
                                "PENALTY SHOT",
                                "WHITE\nTIMEOUT",
                                "BLACK\nTIMEOUT",
                            ),
                        },
                    };

                let tm = tm_.lock().unwrap();

                game_state_header.set_label(game_header);
                main_referee_timeout.set_label(ref_t_o_text);
                new_penalty_shot.set_label(p_s_text);
                main_white_timeout.set_label(w_t_o_text);
                main_black_timeout.set_label(b_t_o_text);

                let ref_t_o_en = if let TimeoutSnapshot::Ref(_) = snapshot.timeout {
                    tm.can_start_penalty_shot().is_ok()
                } else {
                    true
                };

                let p_s_en = if let TimeoutSnapshot::None = snapshot.timeout {
                    tm.can_start_penalty_shot().is_ok()
                } else {
                    true
                };

                let (w_t_o_en, b_t_o_en) = if let TimeoutSnapshot::White(_)
                | TimeoutSnapshot::Black(_) = snapshot.timeout
                {
                    (
                        tm.can_switch_to_w_timeout().is_ok(),
                        tm.can_switch_to_b_timeout().is_ok(),
                    )
                } else {
                    (
                        tm.can_start_w_timeout().is_ok(),
                        tm.can_start_b_timeout().is_ok(),
                    )
                };

                main_referee_timeout.set_sensitive(ref_t_o_en);
                new_penalty_shot.set_sensitive(p_s_en);
                main_white_timeout.set_sensitive(w_t_o_en);
                main_black_timeout.set_sensitive(b_t_o_en);
            }

            if snapshot.w_score != last_snapshot.w_score {
                edit_white_score.set_label(&format!("{}", snapshot.w_score));
                modified_white_score.set_label(&format!("{}", snapshot.w_score));
            }

            if snapshot.b_score != last_snapshot.b_score {
                edit_black_score.set_label(&format!("{}", snapshot.b_score));
                modified_black_score.set_label(&format!("{}", snapshot.b_score));
            }

            last_snapshot = snapshot;

            glib::source::Continue(true)
        });

        //
        //
        // Make everything visible
        //
        win.add(&layout_stack);
        win.show_all();

        let size = win.get_size();
        if size != (config.hardware.screen_x, config.hardware.screen_y) {
            error!(
                "Window size is wrong. Current: {:?}, Expected: {:?}",
                size,
                (config.hardware.screen_x, config.hardware.screen_y)
            );
        }
    });

    // Actually run the program
    uiapp.run(&[]);

    // Wait for the simulator to close
    //    if let Some(t) = simulator_thread {
    //        t.join().unwrap();
    //    }

    Ok(())
}

fn create_new_file(path: &str) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
}

fn secs_to_time_string<T>(secs: T) -> String
where
    T: Div<T> + Rem<T> + From<u8> + Copy,
    <T as Div>::Output: Display,
    <T as Rem>::Output: Display,
{
    let min = secs / T::from(60u8);
    let sec = secs % T::from(60u8);
    format!("{:2}:{:02}", min, sec)
}

macro_rules! new_button_func {
    ($type:ty, $name:ident) => {
        fn $name(text: &str, styles: &[&str], size: Option<(i32, i32)>) -> $type {
            let button = <$type>::new_with_label(text);
            button
                .get_child()
                .unwrap()
                .downcast::<gtk::Label>()
                .unwrap()
                .set_justify(gtk::Justification::Center);
            for style in styles {
                button.get_style_context().add_class(style);
            }
            if let Some((x, y)) = size {
                button.set_size_request(x, y);
            }
            button
        }
    };
}

new_button_func!(gtk::Button, new_button);
new_button_func!(gtk::ToggleButton, new_toggle_button);

fn new_keypad_button(text: &str, style: &str, size: Option<(i32, i32)>) -> gtk::Button {
    let keypad_button = gtk::Button::new_with_label(text);
    keypad_button
        .get_child()
        .unwrap()
        .downcast::<gtk::Label>()
        .unwrap()
        .set_justify(gtk::Justification::Center);
    keypad_button.get_style_context().add_class(style);
    keypad_button.set_margin_start(BUTTON_MARGIN);
    keypad_button.set_margin_bottom(BUTTON_MARGIN);
    if let Some((x, y)) = size {
        keypad_button.set_size_request(x, y);
    }
    keypad_button
}

fn new_label(text: &str, style: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_justify(gtk::Justification::Center);
    label.get_style_context().add_class(style);
    label
}

fn new_keypad() -> (gtk::Grid, gtk::Label) {
    let keypad = gtk::Grid::new();
    keypad.set_column_homogeneous(true);
    keypad.set_row_homogeneous(true);
    keypad.get_style_context().add_class("keypad");

    let player_number = new_label("Player #:\n", "player-number-gray");

    let button_backspace = new_keypad_button("<--", "keypad", None);
    button_backspace.set_margin_end(BUTTON_MARGIN);

    let player_number_ = player_number.clone();
    button_backspace.connect_clicked(move |_| {
        let label = player_number_.get_label().unwrap();
        if label.as_str().chars().next_back().unwrap().is_digit(10) {
            let mut updated_label = label.as_str().to_string();
            updated_label.pop();
            player_number_.set_label(&updated_label);
        }
    });

    macro_rules! new_number_button {
        ($name:ident, $text:literal, $value:literal) => {
            let $name = new_keypad_button($text, "keypad", None);
            let player_number_ = player_number.clone();
            $name.connect_clicked(move |_| {
                let mut updated_label = player_number_.get_label().unwrap().as_str().to_string();
                if updated_label.len() < 12 {
                    updated_label.push($value);
                    player_number_.set_label(&updated_label);
                }
            });
        };
    }

    new_number_button!(button_0, "0", '0');
    new_number_button!(button_1, "1", '1');
    new_number_button!(button_2, "2", '2');
    new_number_button!(button_3, "3", '3');
    new_number_button!(button_4, "4", '4');
    new_number_button!(button_5, "5", '5');
    new_number_button!(button_6, "6", '6');
    new_number_button!(button_7, "7", '7');
    new_number_button!(button_8, "8", '8');
    new_number_button!(button_9, "9", '9');
    button_3.set_margin_end(BUTTON_MARGIN);
    button_6.set_margin_end(BUTTON_MARGIN);
    button_9.set_margin_end(BUTTON_MARGIN);

    keypad.attach(&player_number, 0, 0, 3, 2);
    keypad.attach(&button_7, 0, 2, 1, 1);
    keypad.attach(&button_8, 1, 2, 1, 1);
    keypad.attach(&button_9, 2, 2, 1, 1);
    keypad.attach(&button_4, 0, 3, 1, 1);
    keypad.attach(&button_5, 1, 3, 1, 1);
    keypad.attach(&button_6, 2, 3, 1, 1);
    keypad.attach(&button_1, 0, 4, 1, 1);
    keypad.attach(&button_2, 1, 4, 1, 1);
    keypad.attach(&button_3, 2, 4, 1, 1);
    keypad.attach(&button_0, 0, 5, 1, 1);
    keypad.attach(&button_backspace, 1, 5, 2, 1);
    (keypad, player_number)
}

fn time_edit_ribbon() -> gtk::Grid {
    let time_edit = gtk::Grid::new();
    time_edit.set_column_homogeneous(true);
    time_edit.set_row_homogeneous(true);
    time_edit.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
    time_edit.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

    let plus_60 = new_button("+60", &["little-blue"], None);
    let minus_60 = new_button("-60", &["little-blue"], None);
    let plus_15 = new_button("+15", &["little-blue"], None);
    let minus_15 = new_button("-15", &["little-blue"], None);

    time_edit.attach(&plus_60, 0, 0, 1, 1);
    time_edit.attach(&minus_60, 1, 0, 1, 1);
    time_edit.attach(&plus_15, 2, 0, 1, 1);
    time_edit.attach(&minus_15, 3, 0, 1, 1);
    time_edit
}

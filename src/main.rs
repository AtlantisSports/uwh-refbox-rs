use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    SubCommand,
};
use gio::prelude::*;
use gtk::prelude::*;
use log::*;
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};

mod config;
mod css;
use config::Config;

const BUTTON_SPACING: i32 = 12;
const BUTTON_MARGIN: i32 = 6;
//const RETURN_BTN_SIZE_X: i32 = 400;
//const RETURN_BTN_SIZE_Y: i32 = 250;
//const BUTTON_STANDARD_HEIGHT: i32 = 70;
//const BUTTON_STANDARD_HEIGHT: config.hardware.screen_y / 6;

//const LABEL_STANDARD_HEIGHT: i32 = 35;
//const KEYPAD_BUTTON_SIZE: i32 = 70;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // If we get here, no CLI subcommands were given, so run the refbox program

    let config = Config::new_from_file(config_path)?;

    // Setup the application that gets run
    let uiapp = gtk::Application::new("org.navisjon.refbox", gio::ApplicationFlags::FLAGS_NONE)
        .expect("Application::new() failed");

    // Some debugging output
    info!("App initialized");

    // Initialize the app
    uiapp.connect_activate(move |app| {
        // Setup the app to use the CSS Style defined at the top of this file
        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(css::STYLE.as_bytes())
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
        win.set_resizable(true);

        // Main Page
        let main_layout = gtk::Grid::new();
        main_layout.set_column_homogeneous(true);
        main_layout.set_row_homogeneous(true);
        main_layout.set_margin_top(BUTTON_MARGIN);
        main_layout.set_margin_start(BUTTON_MARGIN);
        main_layout.set_margin_end(BUTTON_MARGIN);
        main_layout.set_margin_bottom(BUTTON_MARGIN);
        main_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        main_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let edit_game_time = new_button("##:##", "game-time", None);
        let new_penalty_shot = new_button("PENALTY SHOT", "red", None);
        let edit_game_information = new_button("GAME INFORMATION", "gray", None);
        let edit_white_score = new_button("#W", "white-score", None);
        let add_white_score = new_button("SCORE WHITE", "white", None);
        let edit_white_time_penalty = new_button("WHITE TIME PENALTY", "white", None);
        let edit_black_score = new_button("#B", "black-score", None);
        let add_black_score = new_button("SCORE BLACK", "black", None);
        let edit_black_time_penalty = new_button("BLACK TIME PENALTY", "black", None);
        /*
                let main_white_timeout = new_button("WHITE TIMEOUT", "white", None);
                let main_referee_timeout = new_button("REFEREE TIMEOUT", "yellow", None);
                let main_black_timeout = new_button("BLACK TIMEOUT", "black", None);
        */

        let game_state_header = gtk::Label::new(Some("GAME STATE"));
        game_state_header
            .get_style_context()
            .add_class("game-state-header");

        let white_header = gtk::Label::new(Some("WHITE"));
        white_header.get_style_context().add_class("white-header");

        let black_header = gtk::Label::new(Some("Black"));
        black_header.get_style_context().add_class("black-header");

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

        let main_timeout_ribbon = new_timeout_ribbon();

        main_layout.attach(&white_box, 0, 0, 3, 3);
        main_layout.attach(&game_box, 3, 0, 6, 3);
        main_layout.attach(&black_box, 9, 0, 3, 3);
        main_layout.attach(&add_white_score, 0, 3, 3, 2);
        main_layout.attach(&new_penalty_shot, 3, 3, 6, 2);
        main_layout.attach(&add_black_score, 9, 3, 3, 2);
        main_layout.attach(&edit_white_time_penalty, 0, 5, 3, 4);
        main_layout.attach(&edit_game_information, 3, 5, 6, 4);
        main_layout.attach(&edit_black_time_penalty, 9, 5, 3, 4);
        main_layout.attach(&main_timeout_ribbon, 0, 9, 12, 2);
        /*
                main_layout.attach(&main_white_timeout, 0, 9, 3, 2);
                main_layout.attach(&main_referee_timeout, 3, 9, 6, 2);
                main_layout.attach(&main_black_timeout, 9, 9, 3, 2);
        */

        // New Score Page
        let new_score_layout = gtk::Grid::new();
        new_score_layout.set_column_homogeneous(true);
        new_score_layout.set_row_homogeneous(true);
        new_score_layout.set_margin_top(BUTTON_MARGIN);
        new_score_layout.set_margin_start(BUTTON_MARGIN);
        new_score_layout.set_margin_end(BUTTON_MARGIN);
        new_score_layout.set_margin_bottom(BUTTON_MARGIN);
        new_score_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        new_score_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let score_white_select = new_button("WHITE", "white", None);
        let score_black_select = new_button("BLACK", "black", None);
        let score_cancel = new_button("CANCEL", "red", None);
        let score_submit = new_button("SUBMIT", "green", None);
        /*
                let score_white_timeout = new_button("WHITE TIMEOUT", "white", None);
                let score_referee_timeout = new_button("REFEREE TIMEOUT", "yellow", None);
                let score_black_timeout = new_button("BLACK TIMEOUT", "black", None);
        */
        let score_player_number = gtk::Label::new(Some("#P"));
        score_player_number.get_style_context().add_class("gray");

        let score_keypad = new_keypad();

        let new_score_timeout_ribbon = new_timeout_ribbon();

        new_score_layout.attach(&score_keypad, 0, 0, 4, 7);
        new_score_layout.attach(&score_white_select, 4, 0, 4, 3);
        new_score_layout.attach(&score_black_select, 8, 0, 4, 3);
        new_score_layout.attach(&score_cancel, 0, 7, 4, 2);
        new_score_layout.attach(&score_player_number, 4, 7, 4, 2);
        new_score_layout.attach(&score_submit, 8, 7, 4, 2);
        new_score_layout.attach(&new_score_timeout_ribbon, 0, 9, 12, 2);
        /*
                new_score_layout.attach(&score_white_timeout, 0, 9, 3, 2);
                new_score_layout.attach(&score_referee_timeout, 3, 9, 6, 2);
                new_score_layout.attach(&score_black_timeout, 9, 9, 3, 2);
        */

        // Score Edit Page
        let edit_score_layout = gtk::Grid::new();
        edit_score_layout.set_column_homogeneous(true);
        edit_score_layout.set_row_homogeneous(true);
        edit_score_layout.set_margin_top(BUTTON_MARGIN);
        edit_score_layout.set_margin_start(BUTTON_MARGIN);
        edit_score_layout.set_margin_end(BUTTON_MARGIN);
        edit_score_layout.set_margin_bottom(BUTTON_MARGIN);
        edit_score_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        edit_score_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let white_score_plus = new_button("+", "blue-top", None);
        let white_score_minus = new_button("-", "light-blue", None);
        let black_score_plus = new_button("+", "blue-top", None);
        let black_score_minus = new_button("-", "light-blue", None);
        let score_edit_cancel = new_button("CANCEL", "red", None);
        let score_edit_submit = new_button("SUBMIT", "green", None);

        let white_score_header = gtk::Label::new(Some("WHITE SCORE"));
        white_score_header
            .get_style_context()
            .add_class("white-header");
        let black_score_header = gtk::Label::new(Some("BLACK SCORE"));
        black_score_header
            .get_style_context()
            .add_class("black-header");
        let modified_white_score = gtk::Label::new(Some("##"));
        modified_white_score
            .get_style_context()
            .add_class("modified-score");
        let modified_black_score = gtk::Label::new(Some("##"));
        modified_black_score
            .get_style_context()
            .add_class("modified-score");
        let empty_score_edit_label = gtk::Label::new(None);

        let white_score_header_box = gtk::Grid::new();
        white_score_header_box.set_column_homogeneous(true);
        white_score_header_box.set_row_homogeneous(true);
        white_score_header_box.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        white_score_header_box.attach(&white_score_header, 0, 0, 2, 1);
        white_score_header_box.attach(&white_score_plus, 0, 1, 1, 2);
        white_score_header_box.attach(&modified_white_score, 1, 1, 1, 2);

        let black_score_header_box = gtk::Grid::new();
        black_score_header_box.set_column_homogeneous(true);
        black_score_header_box.set_row_homogeneous(true);
        black_score_header_box.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        black_score_header_box.attach(&black_score_header, 0, 0, 2, 1);
        black_score_header_box.attach(&modified_black_score, 0, 1, 1, 2);
        black_score_header_box.attach(&black_score_plus, 1, 1, 1, 2);

        let score_edit_timeout_ribbon = new_timeout_ribbon();

        edit_score_layout.attach(&white_score_header_box, 0, 0, 6, 3);
        edit_score_layout.attach(&black_score_header_box, 6, 0, 6, 3);
        edit_score_layout.attach(&white_score_minus, 0, 3, 3, 2);
        edit_score_layout.attach(&black_score_minus, 9, 3, 3, 2);
        edit_score_layout.attach(&empty_score_edit_label, 0, 5, 12, 2);
        edit_score_layout.attach(&score_edit_cancel, 0, 7, 4, 2);
        edit_score_layout.attach(&score_edit_submit, 8, 7, 4, 2);
        edit_score_layout.attach(&score_edit_timeout_ribbon, 0, 9, 12, 2);

        // Time Penalty Confirmation Page
        let time_penalty_conf_layout = gtk::Grid::new();
        time_penalty_conf_layout.set_column_homogeneous(true);
        time_penalty_conf_layout.set_row_homogeneous(true);
        time_penalty_conf_layout.set_margin_top(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_start(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_end(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_bottom(BUTTON_MARGIN);
        time_penalty_conf_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        time_penalty_conf_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let white_time_list = new_button("WHITE PENALTIES", "white", None);
        let black_time_list = new_button("BLACK PENALTIES", "black", None);
        let penalty_conf_cancel = new_button("CANCEL", "red", None);
        let penalty_conf_new = new_button("NEW", "blue", None);
        let penalty_conf_start = new_button("START", "green", None);
        let penalty_conf_white_timeout = new_button("WHITE TIMEOUT", "white", None);
        let penalty_conf_referee_timeout = new_button("REFEREE TIMEOUT", "yellow", None);
        let penalty_conf_black_timeout = new_button("BLACK TIMEOUT", "black", None);

        time_penalty_conf_layout.attach(&white_time_list, 0, 0, 6, 7);
        time_penalty_conf_layout.attach(&black_time_list, 6, 0, 6, 7);
        time_penalty_conf_layout.attach(&penalty_conf_cancel, 0, 7, 4, 2);
        time_penalty_conf_layout.attach(&penalty_conf_new, 4, 7, 4, 2);
        time_penalty_conf_layout.attach(&penalty_conf_start, 8, 7, 4, 2);
        time_penalty_conf_layout.attach(&penalty_conf_white_timeout, 0, 9, 3, 2);
        time_penalty_conf_layout.attach(&penalty_conf_referee_timeout, 3, 9, 6, 2);
        time_penalty_conf_layout.attach(&penalty_conf_black_timeout, 9, 9, 3, 2);

        // Time Penalty Add/Edit Page
        let penalty_add_layout = gtk::Grid::new();
        penalty_add_layout.set_column_homogeneous(true);
        penalty_add_layout.set_row_homogeneous(true);
        penalty_add_layout.set_margin_top(BUTTON_MARGIN);
        penalty_add_layout.set_margin_start(BUTTON_MARGIN);
        penalty_add_layout.set_margin_end(BUTTON_MARGIN);
        penalty_add_layout.set_margin_bottom(BUTTON_MARGIN);
        penalty_add_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        penalty_add_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let penalty_white_select = new_button("WHITE", "white", None);
        let penalty_black_select = new_button("BLACK", "black", None);
        let penalty_1min = new_button("1 MIN", "yellow", None);
        let penalty_2min = new_button("2 MIN", "orange", None);
        let penalty_5min = new_button("5 MIN", "red", None);
        let penalty_dismiss = new_button("DISMISS", "lavender", None);
        let penalty_delete = new_button("DELETE", "red", None);
        let penalty_add = new_button("ADD", "green", None);
        let penalty_white_timeout = new_button("WHITE TIMEOUT", "white", None);
        let penalty_referee_timeout = new_button("REFEREE TIMEOUT", "yellow", None);
        let penalty_black_timeout = new_button("BLACK TIMEOUT", "black", None);

        let penalty_player_number = gtk::Label::new(Some("#P"));
        penalty_player_number.get_style_context().add_class("gray");

        let penalty_keypad = new_keypad();

        penalty_add_layout.attach(&penalty_keypad, 0, 0, 4, 7);
        penalty_add_layout.attach(&penalty_white_select, 4, 0, 4, 3);
        penalty_add_layout.attach(&penalty_black_select, 8, 0, 4, 3);
        penalty_add_layout.attach(&penalty_1min, 4, 3, 2, 4);
        penalty_add_layout.attach(&penalty_2min, 6, 3, 2, 4);
        penalty_add_layout.attach(&penalty_5min, 8, 3, 2, 4);
        penalty_add_layout.attach(&penalty_dismiss, 10, 3, 2, 4);
        penalty_add_layout.attach(&penalty_delete, 0, 7, 4, 2);
        penalty_add_layout.attach(&penalty_player_number, 4, 7, 4, 2);
        penalty_add_layout.attach(&penalty_add, 8, 7, 4, 2);
        penalty_add_layout.attach(&penalty_white_timeout, 0, 9, 3, 2);
        penalty_add_layout.attach(&penalty_referee_timeout, 3, 9, 6, 2);
        penalty_add_layout.attach(&penalty_black_timeout, 9, 9, 3, 2);

        // Time Edit Page
        let time_edit_layout = gtk::Grid::new();
        time_edit_layout.set_column_homogeneous(true);
        time_edit_layout.set_row_homogeneous(true);
        time_edit_layout.set_margin_top(BUTTON_MARGIN);
        time_edit_layout.set_margin_start(BUTTON_MARGIN);
        time_edit_layout.set_margin_end(BUTTON_MARGIN);
        time_edit_layout.set_margin_bottom(BUTTON_MARGIN);
        time_edit_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        time_edit_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let minute_plus = new_button("+", "blue", None);
        let minute_minus = new_button("-", "light-blue", None);
        let second_plus = new_button("+", "blue", None);
        let second_minus = new_button("-", "light-blue", None);
        let time_edit_cancel = new_button("CANCEL", "red", None);
        let time_edit_submit = new_button("SUBMIT", "green", None);

        let minute_header = gtk::Label::new(Some("MINUTE"));
        minute_header.get_style_context().add_class("time-mod");
        let second_header = gtk::Label::new(Some("SECOND"));
        second_header.get_style_context().add_class("time-mod");
        let new_time_header = gtk::Label::new(Some("NEW TIME"));
        new_time_header.get_style_context().add_class("time-mod");
        let modified_game_time = gtk::Label::new(Some("##:##"));
        modified_game_time.get_style_context().add_class("time-mod");
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

        let time_edit_timeout_ribbon = new_timeout_ribbon();

        time_edit_layout.attach(&minute_header_box, 0, 0, 3, 3);
        time_edit_layout.attach(&new_time_header_box, 3, 0, 6, 3);
        time_edit_layout.attach(&second_header_box, 9, 0, 3, 3);
        time_edit_layout.attach(&minute_minus, 0, 3, 3, 2);
        time_edit_layout.attach(&second_minus, 9, 3, 3, 2);
        time_edit_layout.attach(&empty_time_edit_label, 0, 5, 12, 2);
        time_edit_layout.attach(&time_edit_cancel, 0, 7, 4, 2);
        time_edit_layout.attach(&time_edit_submit, 8, 7, 4, 2);
        time_edit_layout.attach(&time_edit_timeout_ribbon, 0, 9, 12, 2);

        // Game Over Confirmation Page

        // Game Information Edit Page
        let game_information_edit_layout =
            gtk::Box::new(gtk::Orientation::Vertical, BUTTON_SPACING);
        game_information_edit_layout.set_margin_top(BUTTON_MARGIN);
        game_information_edit_layout.set_margin_start(BUTTON_MARGIN);
        game_information_edit_layout.set_margin_bottom(BUTTON_MARGIN);
        game_information_edit_layout.set_margin_end(BUTTON_MARGIN);

        let game_information_edit_submit = new_button("SUBMIT", "green", None);

        game_information_edit_layout.pack_start(&game_information_edit_submit, false, false, 0);

        // Roster Edit Page

        // Build the Stack, which switches between screen layouts
        let layout_stack = gtk::Stack::new();
        layout_stack.add_named(&main_layout, "Main Layout");
        layout_stack.add_named(&time_edit_layout, "Time Edit Layout");
        layout_stack.add_named(
            &game_information_edit_layout,
            "Game Information Edit Layout",
        );
        layout_stack.add_named(&new_score_layout, "New Score Layout");
        layout_stack.add_named(&penalty_add_layout, "Penalty Add/Edit Layout");
        layout_stack.add_named(
            &time_penalty_conf_layout,
            "Time Penalty Confirmation Layout",
        );
        layout_stack.add_named(&edit_score_layout, "Edit Score Layout");
        //        layout_stack.add_named(&edit_time_layout, "Edit Time Layout");

        // Set up the buttons to switch between layouts

        //Buttons for moving back to the Main Layout
        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        game_information_edit_submit
            .connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        time_edit_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        score_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        score_cancel.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        score_edit_cancel.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        score_edit_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        time_edit_cancel.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        time_edit_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        penalty_conf_cancel
            .connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        //        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        penalty_conf_start.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout));

        //Buttons for moving away from the Main Layout
        //        let time_edit_layout_ = time_edit_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_time.connect_clicked(move |_| layout_stack_.set_visible_child(&time_edit_layout));

        let new_score_layout_ = new_score_layout.clone();
        let layout_stack_ = layout_stack.clone();
        add_white_score
            .connect_clicked(move |_| layout_stack_.set_visible_child(&new_score_layout_));

        //        let new_score_layout_ = new_score_layout.clone();
        let layout_stack_ = layout_stack.clone();
        add_black_score
            .connect_clicked(move |_| layout_stack_.set_visible_child(&new_score_layout));

        let edit_score_layout_ = edit_score_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_white_score
            .connect_clicked(move |_| layout_stack_.set_visible_child(&edit_score_layout_));

        //        let edit_score_layout_ = edit_score_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_black_score
            .connect_clicked(move |_| layout_stack_.set_visible_child(&edit_score_layout));

        //        let game_information_edit_layout_ = game_information_edit_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_information.connect_clicked(move |_| {
            layout_stack_.set_visible_child(&game_information_edit_layout)
        });

        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_white_time_penalty
            .connect_clicked(move |_| layout_stack_.set_visible_child(&time_penalty_conf_layout_));

        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_black_time_penalty
            .connect_clicked(move |_| layout_stack_.set_visible_child(&time_penalty_conf_layout_));

        //Buttons for navigating between Layouts that are not Main Layout

        //        let penalty_add_layout_ = penalty_add_layout.clone();
        let layout_stack_ = layout_stack.clone();
        penalty_conf_new
            .connect_clicked(move |_| layout_stack_.set_visible_child(&penalty_add_layout));

        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        penalty_delete
            .connect_clicked(move |_| layout_stack_.set_visible_child(&time_penalty_conf_layout_));

        //        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        penalty_add
            .connect_clicked(move |_| layout_stack_.set_visible_child(&time_penalty_conf_layout));

        // Make everything visible
        win.add(&layout_stack);
        win.show_all();
    });

    // Actually run the program
    uiapp.run(&[]);

    Ok(())
}

fn create_new_file(path: &str) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
}

fn new_button(text: &str, style: &str, size: Option<(i32, i32)>) -> gtk::Button {
    let button = gtk::Button::new_with_label(text);
    button.get_style_context().add_class(style);
    if let Some((x, y)) = size {
        button.set_size_request(x, y);
    }
    button
}

fn new_keypad() -> gtk::Grid {
    let keypad = gtk::Grid::new();
    keypad.set_column_homogeneous(true);
    keypad.set_row_homogeneous(true);

    let button_backspace = new_button("<--", "keypad", None);
    let button_0 = new_button("0", "keypad", None);
    let button_1 = new_button("1", "keypad", None);
    let button_2 = new_button("2", "keypad", None);
    let button_3 = new_button("3", "keypad", None);
    let button_4 = new_button("4", "keypad", None);
    let button_5 = new_button("5", "keypad", None);
    let button_6 = new_button("6", "keypad", None);
    let button_7 = new_button("7", "keypad", None);
    let button_8 = new_button("8", "keypad", None);
    let button_9 = new_button("9", "keypad", None);

    keypad.attach(&button_7, 0, 0, 1, 1);
    keypad.attach(&button_8, 1, 0, 1, 1);
    keypad.attach(&button_9, 2, 0, 1, 1);
    keypad.attach(&button_4, 0, 1, 1, 1);
    keypad.attach(&button_5, 1, 1, 1, 1);
    keypad.attach(&button_6, 2, 1, 1, 1);
    keypad.attach(&button_1, 0, 2, 1, 1);
    keypad.attach(&button_2, 1, 2, 1, 1);
    keypad.attach(&button_3, 2, 2, 1, 1);
    keypad.attach(&button_0, 0, 3, 1, 1);
    keypad.attach(&button_backspace, 1, 3, 2, 1);
    keypad
}

fn new_timeout_ribbon() -> gtk::Grid {
    let timeout_ribbon = gtk::Grid::new();
    timeout_ribbon.set_column_homogeneous(true);
    timeout_ribbon.set_row_homogeneous(true);
    timeout_ribbon.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
    timeout_ribbon.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

    let new_white_timeout = new_button("WHITE TIMEOUT", "white", None);
    let new_referee_timeout = new_button("REFEREE TIMEOUT", "yellow", None);
    let new_black_timeout = new_button("BLACK TIMEOUT", "black", None);

    timeout_ribbon.attach(&new_white_timeout, 0, 0, 3, 2);
    timeout_ribbon.attach(&new_referee_timeout, 3, 0, 6, 2);
    timeout_ribbon.attach(&new_black_timeout, 9, 0, 3, 2);
    timeout_ribbon
}

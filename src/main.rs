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

const BUTTON_SPACING: i32 = 10;
const BUTTON_MARGIN: i32 = 10;
//const RETURN_BTN_SIZE_X: i32 = 400;
//const RETURN_BTN_SIZE_Y: i32 = 250;
const BUTTON_STANDARD_HEIGHT: i32 = 70;
//const BUTTON_STANDARD_HEIGHT: config.hardware.screen_y / 6;

const LABEL_STANDARD_HEIGHT: i32 = 35;
const KEYPAD_BUTTON_SIZE: i32 = 70;

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

        // Timeout Ribbon
        let timeout_ribbon = gtk::Grid::new();
        timeout_ribbon.set_column_homogeneous(true);
        timeout_ribbon.insert_column(0);
        timeout_ribbon.insert_column(1);
        timeout_ribbon.insert_column(2);
        timeout_ribbon.insert_column(3);
        timeout_ribbon.insert_row(0);

        timeout_ribbon.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        timeout_ribbon.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let new_white_timeout =
            new_button("WHITE TIMEOUT", "white", Some((-1, BUTTON_STANDARD_HEIGHT)));
        let new_referee_timeout = new_button(
            "REFEREE TIMEOUT",
            "yellow",
            Some((-1, BUTTON_STANDARD_HEIGHT)),
        );
        let new_black_timeout =
            new_button("BLACK TIMEOUT", "black", Some((-1, BUTTON_STANDARD_HEIGHT)));

        timeout_ribbon.attach(&new_white_timeout, 0, 0, 1, 1);
        timeout_ribbon.attach(&new_referee_timeout, 1, 0, 2, 1);
        timeout_ribbon.attach(&new_black_timeout, 3, 0, 1, 1);

        // Main Page
        let main_layout = gtk::Grid::new();
        main_layout.set_column_homogeneous(true);
        main_layout.set_margin_top(BUTTON_MARGIN);
        main_layout.set_margin_start(BUTTON_MARGIN);
        main_layout.set_margin_end(BUTTON_MARGIN);
        main_layout.set_margin_bottom(BUTTON_MARGIN);
        main_layout.insert_column(0);
        main_layout.insert_column(1);
        main_layout.insert_column(2);
        main_layout.insert_column(3);
        main_layout.insert_row(0);
        main_layout.insert_row(1);
        main_layout.insert_row(2);
        main_layout.insert_row(3);

        main_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        main_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let edit_game_time = new_button("##:##", "game-time", Some((-1, BUTTON_STANDARD_HEIGHT)));
        let new_penalty_shot =
            new_button("PENALTY SHOT", "red", Some((-1, BUTTON_STANDARD_HEIGHT)));
        let edit_game_information = new_button("GAME INFORMATION", "gray", None);
        let edit_white_score = new_button("#W", "white-score", Some((-1, BUTTON_STANDARD_HEIGHT)));
        let add_white_score =
            new_button("SCORE WHITE", "white", Some((-1, BUTTON_STANDARD_HEIGHT)));
        let edit_white_time_penalty = new_button("WHITE TIME PENALTY", "white", None);
        let edit_black_score = new_button("#B", "black-score", Some((-1, BUTTON_STANDARD_HEIGHT)));
        let add_black_score =
            new_button("SCORE BLACK", "black", Some((-1, BUTTON_STANDARD_HEIGHT)));
        let edit_black_time_penalty = new_button("BLACK TIME PENALTY", "black", None);

        let game_state_header = gtk::Label::new(Some("GAME STATE"));
        game_state_header
            .get_style_context()
            .add_class("game-state-header");
        game_state_header.set_size_request(-1, LABEL_STANDARD_HEIGHT);

        let white_header = gtk::Label::new(Some("WHITE"));
        white_header.get_style_context().add_class("white-header");
        white_header.set_size_request(-1, LABEL_STANDARD_HEIGHT);

        let black_header = gtk::Label::new(Some("Black"));
        black_header.get_style_context().add_class("black-header");
        black_header.set_size_request(-1, LABEL_STANDARD_HEIGHT);

        let white_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        white_box.pack_start(&white_header, false, false, 0);
        white_box.pack_start(&edit_white_score, false, false, 0);

        let game_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        game_box.pack_start(&game_state_header, false, false, 0);
        game_box.pack_start(&edit_game_time, false, false, 0);

        let black_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        black_box.pack_start(&black_header, false, false, 0);
        black_box.pack_start(&edit_black_score, false, false, 0);

        main_layout.attach(&white_box, 0, 0, 1, 1);
        main_layout.attach(&game_box, 1, 0, 2, 1);
        main_layout.attach(&black_box, 3, 0, 1, 1);
        main_layout.attach(&add_white_score, 0, 1, 1, 1);
        main_layout.attach(&new_penalty_shot, 1, 1, 2, 1);
        main_layout.attach(&add_black_score, 3, 1, 1, 1);
        main_layout.attach(&edit_white_time_penalty, 0, 2, 1, 1);
        main_layout.attach(&edit_game_information, 1, 2, 2, 1);
        main_layout.attach(&edit_black_time_penalty, 3, 2, 1, 1);
        main_layout.attach(&timeout_ribbon, 0, 3, 4, 1);

        game_box.set_hexpand(true);
        edit_white_time_penalty.set_vexpand(true);

        // New Score Page
        let new_score_layout = gtk::Box::new(gtk::Orientation::Vertical, BUTTON_SPACING);
        new_score_layout.set_margin_top(BUTTON_MARGIN);
        new_score_layout.set_margin_start(BUTTON_MARGIN);
        new_score_layout.set_margin_end(BUTTON_MARGIN);
        new_score_layout.set_margin_bottom(BUTTON_MARGIN);

        let new_score_upper_layout = gtk::Grid::new();
        new_score_upper_layout.set_column_homogeneous(true);
        new_score_upper_layout.insert_column(0);
        new_score_upper_layout.insert_column(1);
        new_score_upper_layout.insert_column(2);
        new_score_upper_layout.insert_row(0);
        new_score_upper_layout.insert_row(1);
        new_score_upper_layout.insert_row(2);
        new_score_upper_layout.insert_row(3);
        new_score_upper_layout.insert_row(4);

        new_score_upper_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        new_score_upper_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let score_white_select = new_button("WHITE", "white", None);
        let score_black_select = new_button("BLACK", "black", None);
        let score_cancel = new_button("CANCEL", "red", Some((-1, BUTTON_STANDARD_HEIGHT)));
        let score_submit = new_button("SUBMIT", "green", Some((-1, BUTTON_STANDARD_HEIGHT)));

        let score_player_number = gtk::Label::new(Some("#P"));
        score_player_number.get_style_context().add_class("gray");
        score_player_number.set_size_request(-1, LABEL_STANDARD_HEIGHT);

        let score_keypad = new_keypad();

        new_score_upper_layout.attach(&score_keypad, 0, 0, 1, 4);
        new_score_upper_layout.attach(&score_white_select, 1, 0, 1, 2);
        new_score_upper_layout.attach(&score_black_select, 1, 2, 1, 2);
        new_score_upper_layout.attach(&score_cancel, 0, 4, 1, 1);
        new_score_upper_layout.attach(&score_player_number, 1, 4, 1, 1);
        new_score_upper_layout.attach(&score_submit, 2, 4, 1, 1);

        new_score_layout.pack_start(&new_score_upper_layout, false, false, 0);
        //        new_score_layout.pack_start(&timeout_ribbon, false, false, 0);

        // Score Edit Page

        // Time Penalty Confirmation Page
        let time_penalty_conf_layout = gtk::Box::new(gtk::Orientation::Vertical, BUTTON_SPACING);
        time_penalty_conf_layout.set_margin_top(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_start(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_end(BUTTON_MARGIN);
        time_penalty_conf_layout.set_margin_bottom(BUTTON_MARGIN);

        let time_penalty_conf_upper_layout = gtk::Grid::new();
        time_penalty_conf_upper_layout.set_column_homogeneous(true);
        time_penalty_conf_upper_layout.insert_column(0);
        time_penalty_conf_upper_layout.insert_column(1);
        time_penalty_conf_upper_layout.insert_column(2);
        time_penalty_conf_upper_layout.insert_column(3);
        time_penalty_conf_upper_layout.insert_column(4);
        time_penalty_conf_upper_layout.insert_column(5);
        time_penalty_conf_upper_layout.insert_row(0);
        time_penalty_conf_upper_layout.insert_row(1);

        time_penalty_conf_upper_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        time_penalty_conf_upper_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        //let white_time_list = gtk::Button::new_with_label("WHITE PENALTIES");
        //let black_time_list = gtk::Button::new_with_label("BLACK PENALTIES");

        let _cancel_time_penalty_edit = new_button("CANCEL", "red", None);
        let new_time_penalty = new_button("NEW", "blue", None);
        let _start_time_penalty = new_button("START", "green", None);

        //        time_penalty_conf_upper_layout.attach(&white_time_list, 0, 0, 1, 1);
        //        time_penalty_conf_upper_layout.attach(&black_time_list, 0, 1, 1, 1);
        //        time_penalty_conf_upper_layout.attach(&cancel_time_penalty_edit, 0, 1, 2, 1);
        time_penalty_conf_upper_layout.attach(&new_time_penalty, 2, 1, 2, 1);
        //        time_penalty_conf_upper_layout.attach(&start_time_penalty, 4, 1, 2,1);

        time_penalty_conf_layout.pack_start(&time_penalty_conf_upper_layout, false, false, 0);
        //        time_penalty_conf_layout.pack_start(&timeout_ribbon, false, false, 0);

        // Time Penalty Add/Edit Page
        let penalty_add_layout = gtk::Box::new(gtk::Orientation::Vertical, BUTTON_SPACING);
        penalty_add_layout.set_margin_top(BUTTON_MARGIN);
        penalty_add_layout.set_margin_start(BUTTON_MARGIN);
        penalty_add_layout.set_margin_end(BUTTON_MARGIN);
        penalty_add_layout.set_margin_bottom(BUTTON_MARGIN);

        let penalty_add_upper_layout = gtk::Grid::new();
        penalty_add_upper_layout.set_column_homogeneous(true);
        penalty_add_upper_layout.insert_column(0);
        penalty_add_upper_layout.insert_column(1);
        penalty_add_upper_layout.insert_column(2);
        penalty_add_upper_layout.insert_row(0);
        penalty_add_upper_layout.insert_row(1);
        penalty_add_upper_layout.insert_row(2);
        penalty_add_upper_layout.insert_row(3);
        penalty_add_upper_layout.insert_row(4);

        penalty_add_upper_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        penalty_add_upper_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let penalty_white_select = new_button("WHITE", "white", None);
        let penalty_black_select = new_button("BLACK", "black", None);
        let penalty_1min = new_button("1 MIN", "yellow", None);
        let penalty_2min = new_button("2 MIN", "orange", None);
        let penalty_5min = new_button("5 MIN", "red", None);
        let penalty_dismiss = new_button("DISMISS", "lavender", None);
        let delete_time_penalty = new_button("DELETE", "red", None);
        let start_time_penalty = new_button("ADD", "green", None);

        let penalty_player_number = gtk::Label::new(Some("#P"));
        penalty_player_number.get_style_context().add_class("gray");

        let penalty_keypad = new_keypad();

        penalty_add_upper_layout.attach(&penalty_keypad, 0, 0, 1, 4);
        penalty_add_upper_layout.attach(&penalty_white_select, 1, 0, 1, 2);
        penalty_add_upper_layout.attach(&penalty_black_select, 1, 2, 1, 2);
        penalty_add_upper_layout.attach(&delete_time_penalty, 0, 4, 1, 1);
        penalty_add_upper_layout.attach(&penalty_player_number, 1, 4, 1, 1);
        penalty_add_upper_layout.attach(&start_time_penalty, 2, 4, 1, 1);
        penalty_add_upper_layout.attach(&penalty_1min, 2, 0, 1, 1);
        penalty_add_upper_layout.attach(&penalty_2min, 2, 1, 1, 1);
        penalty_add_upper_layout.attach(&penalty_5min, 2, 2, 1, 1);
        penalty_add_upper_layout.attach(&penalty_dismiss, 2, 3, 1, 1);

        penalty_add_layout.pack_start(&penalty_add_upper_layout, false, false, 0);
        //        penalty_add_layout.pack_start(&timeout_ribbon, false, false, 0);

        // Time Edit Page
        let time_edit_layout = gtk::Grid::new();
        time_edit_layout.insert_column(0);
        time_edit_layout.insert_column(1);
        time_edit_layout.insert_column(2);
        time_edit_layout.insert_row(0);
        time_edit_layout.insert_row(1);
        time_edit_layout.insert_row(2);
        time_edit_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        time_edit_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());

        let _minute_plus = new_button("+", "gray", None);
        let _minute_minus = new_button("-", "gray", None);
        let _second_plus = new_button("+", "gray", None);
        let _second_minus = new_button("-", "gray", None);
        let time_edit_cancel = new_button("CANCEL", "red", None);
        let time_edit_submit = new_button("SUBMIT", "green", None);

        time_edit_layout.attach(&time_edit_submit, 1, 1, 1, 1);

        let empty_label_1 = gtk::Label::new(None);
        let empty_label_2 = gtk::Label::new(None);
        time_edit_layout.attach(&empty_label_1, 0, 0, 1, 1);
        time_edit_layout.attach(&empty_label_2, 2, 2, 1, 1);

        empty_label_1.set_hexpand(true);
        empty_label_1.set_vexpand(true);
        empty_label_2.set_hexpand(true);
        empty_label_2.set_vexpand(true);

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

        // Set up the buttons to switch between layouts
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
        time_edit_cancel.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        time_edit_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        delete_time_penalty
            .connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        //        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        start_time_penalty.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout));

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

        //        let game_information_edit_layout_ = game_information_edit_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_information.connect_clicked(move |_| {
            layout_stack_.set_visible_child(&game_information_edit_layout)
        });

        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_white_time_penalty
            .connect_clicked(move |_| layout_stack_.set_visible_child(&time_penalty_conf_layout_));

        //        let time_penalty_conf_layout_ = time_penalty_conf_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_black_time_penalty
            .connect_clicked(move |_| layout_stack_.set_visible_child(&time_penalty_conf_layout));

        //        let penalty_add_layout_ = penalty_add_layout.clone();
        let layout_stack_ = layout_stack.clone();
        new_time_penalty
            .connect_clicked(move |_| layout_stack_.set_visible_child(&penalty_add_layout));

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
    let button_0 = new_button("0", "keypad", Some((-1, KEYPAD_BUTTON_SIZE)));
    let button_1 = new_button("1", "keypad", Some((-1, KEYPAD_BUTTON_SIZE)));
    let button_2 = new_button("2", "keypad", None);
    let button_3 = new_button("3", "keypad", None);
    let button_4 = new_button("4", "keypad", Some((-1, KEYPAD_BUTTON_SIZE)));
    let button_5 = new_button("5", "keypad", None);
    let button_6 = new_button("6", "keypad", None);
    let button_7 = new_button(
        "7",
        "keypad",
        Some((KEYPAD_BUTTON_SIZE, KEYPAD_BUTTON_SIZE)),
    );
    let button_8 = new_button("8", "keypad", Some((KEYPAD_BUTTON_SIZE, -1)));
    let button_9 = new_button("9", "keypad", Some((KEYPAD_BUTTON_SIZE, -1)));

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

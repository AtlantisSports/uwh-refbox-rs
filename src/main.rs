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
const RETURN_BTN_SIZE_X: i32 = 400;
const RETURN_BTN_SIZE_Y: i32 = 250;
const BUTTON_STANDARD_HEIGHT: i32 = 70;
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


        // Buttons for the Time Penalty Confirmation Page
        
        // Buttons for the Time Penalty Add/Edit Page
        
        // Request a specific size for the return button
        //time_edit_submit.set_size_request(RETURN_BTN_SIZE_X, RETURN_BTN_SIZE_Y);





//Set up the Timeout Ribbon
        let timeout_ribbon = gtk::Grid::new();
        timeout_ribbon.set_column_homogeneous(true);
        timeout_ribbon.insert_column(0);
        timeout_ribbon.insert_column(1);
        timeout_ribbon.insert_column(2);
        timeout_ribbon.insert_column(3);
        timeout_ribbon.insert_row(0);

        timeout_ribbon.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        timeout_ribbon.set_row_spacing(BUTTON_SPACING.try_into().unwrap());
  
        let new_white_timeout = gtk::Button::new_with_label("WHITE TIMEOUT");
        new_white_timeout.get_style_context().add_class("white");
        new_white_timeout.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        let new_referee_timeout = gtk::Button::new_with_label("REFEREE TIMEOUT");
        new_referee_timeout.get_style_context().add_class("yellow");
        new_referee_timeout.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        let new_black_timeout = gtk::Button::new_with_label("BLACK TIMEOUT");
        new_black_timeout.get_style_context().add_class("black");
        new_black_timeout.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        timeout_ribbon.attach(&new_white_timeout, 0, 0, 1, 1);
        timeout_ribbon.attach(&new_referee_timeout, 1, 0, 2, 1);
        timeout_ribbon.attach(&new_black_timeout, 3, 0, 1, 1);


// Set up the Keypad
        let keypad = gtk::Grid::new();
        keypad.set_column_homogeneous(true);
        keypad.set_row_homogeneous(true);
        keypad.insert_column(0);
        keypad.insert_column(1);
        keypad.insert_column(2);
        keypad.insert_row(0);
        keypad.insert_row(1);
        keypad.insert_row(2);
        keypad.insert_row(3);

        let keypad_backspace = gtk::Button::new_with_label("<--");
        keypad_backspace.get_style_context().add_class("keypad");

        let keypad_0 = gtk::Button::new_with_label("0");
        keypad_0.get_style_context().add_class("keypad");
        keypad_0.set_size_request(-1, KEYPAD_BUTTON_SIZE);

        let keypad_1 = gtk::Button::new_with_label("1");
        keypad_1.get_style_context().add_class("keypad");
        keypad_1.set_size_request(-1, KEYPAD_BUTTON_SIZE);

        let keypad_2 = gtk::Button::new_with_label("2");
        keypad_2.get_style_context().add_class("keypad");

        let keypad_3 = gtk::Button::new_with_label("3");
        keypad_3.get_style_context().add_class("keypad");

        let keypad_4 = gtk::Button::new_with_label("4");
        keypad_4.get_style_context().add_class("keypad");
        keypad_4.set_size_request(-1, KEYPAD_BUTTON_SIZE);

        let keypad_5 = gtk::Button::new_with_label("5");
        keypad_5.get_style_context().add_class("keypad");

        let keypad_6 = gtk::Button::new_with_label("6");
        keypad_6.get_style_context().add_class("keypad");

        let keypad_7 = gtk::Button::new_with_label("7");
        keypad_7.get_style_context().add_class("keypad");
        keypad_7.set_size_request(KEYPAD_BUTTON_SIZE, KEYPAD_BUTTON_SIZE);

        let keypad_8 = gtk::Button::new_with_label("8");
        keypad_8.get_style_context().add_class("keypad");
        keypad_8.set_size_request(KEYPAD_BUTTON_SIZE, -1);

        let keypad_9 = gtk::Button::new_with_label("9");
        keypad_9.get_style_context().add_class("keypad");
        keypad_9.set_size_request(KEYPAD_BUTTON_SIZE, -1);

        keypad.attach(&keypad_7, 0, 0, 1, 1);
        keypad.attach(&keypad_8, 1, 0, 1, 1);
        keypad.attach(&keypad_9, 2, 0, 1, 1);
        keypad.attach(&keypad_4, 0, 1, 1, 1);
        keypad.attach(&keypad_5, 1, 1, 1, 1);
        keypad.attach(&keypad_6, 2, 1, 1, 1);
        keypad.attach(&keypad_1, 0, 2, 1, 1);
        keypad.attach(&keypad_2, 1, 2, 1, 1);
        keypad.attach(&keypad_3, 2, 2, 1, 1);
        keypad.attach(&keypad_0, 0, 3, 1, 1);
        keypad.attach(&keypad_backspace, 1, 3, 2, 1);




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

        let edit_game_time = gtk::Button::new_with_label("##:##");
        edit_game_time.get_style_context().add_class("game-time");
        edit_game_time.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        let new_penalty_shot = gtk::Button::new_with_label("PENALTY SHOT");
        new_penalty_shot.get_style_context().add_class("red");
        new_penalty_shot.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        let edit_game_information = gtk::Button::new_with_label("GAME INFORMATION");
        edit_game_information.get_style_context().add_class("grayed-out");

        let edit_white_score = gtk::Button::new_with_label("#W");
        edit_white_score.get_style_context().add_class("white-score");
        edit_white_score.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        let add_white_score = gtk::Button::new_with_label("SCORE WHITE");
        add_white_score.get_style_context().add_class("white");
        add_white_score.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        let edit_white_time_penalty = gtk::Button::new_with_label("WHITE TIME PENALTY");
        edit_white_time_penalty.get_style_context().add_class("white");

        let edit_black_score = gtk::Button::new_with_label("#B");
        edit_black_score.get_style_context().add_class("black-score");
        edit_black_score.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        let add_black_score = gtk::Button::new_with_label("SCORE BLACK");
        add_black_score.get_style_context().add_class("black");
        add_black_score.set_size_request(-1, BUTTON_STANDARD_HEIGHT);

        let edit_black_time_penalty = gtk::Button::new_with_label("BLACK TIME PENALTY");
        edit_black_time_penalty.get_style_context().add_class("black");

        let game_state_header = gtk::Label::new(Some("GAME STATE"));
        game_state_header.get_style_context().add_class("game-state-header");
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
    


//New Score Page
        let new_score_layout = gtk::Box::new(gtk::Orientation::Vertical, BUTTON_SPACING);

        let new_score_upper_layout = gtk::Grid::new();
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

        let white_select = gtk::Button::new_with_label("WHITE");
        white_select.get_style_context().add_class("white");
        
        let black_select = gtk::Button::new_with_label("BLACK");
        black_select.get_style_context().add_class("black");

        let score_cancel = gtk::Button::new_with_label("CANCEL");
        score_cancel.get_style_context().add_class("red");

        let score_submit = gtk::Button::new_with_label("SUBMIT");
        score_submit.get_style_context().add_class("green");

        let player_number = gtk::Label::new(Some("#P"));
        player_number.get_style_context().add_class("grayed-out");

        new_score_upper_layout.attach(&keypad, 0, 0, 1, 4);
        new_score_upper_layout.attach(&white_select, 1, 0, 1, 2);
        new_score_upper_layout.attach(&black_select, 1, 2, 1, 2);
        new_score_upper_layout.attach(&score_cancel, 0, 4, 1, 1);
        new_score_upper_layout.attach(&player_number, 1, 4, 1, 1);
        new_score_upper_layout.attach(&score_submit, 2, 4, 1, 1);

        new_score_layout.pack_start(&new_score_upper_layout, false, false, 0);
        new_score_layout.pack_start(&timeout_ribbon, false, false, 0);



// Time Edit Page
      


// Time Penalty Confirmation Page
        let white_time_1 = gtk::Button::new_with_label("W PLYR 1");
        let white_time_2 = gtk::Button::new_with_label("W PLYR 2");
        let white_time_3 = gtk::Button::new_with_label("W PLYR 3");
        let white_time_4 = gtk::Button::new_with_label("W PLYR 4");
        let white_time_5 = gtk::Button::new_with_label("W PLYR 5");
        let black_time_1 = gtk::Button::new_with_label("B PLYR 1");
        let black_time_2 = gtk::Button::new_with_label("B PLYR 2");
        let black_time_3 = gtk::Button::new_with_label("B PLYR 3");
        let black_time_4 = gtk::Button::new_with_label("B PLYR 4");
        let black_time_5 = gtk::Button::new_with_label("B PLYR 5");
        let cancel_time_penalty_edit = gtk::Button::new_with_label("CANCEL");
        let new_time_penalty = gtk::Button::new_with_label("NEW");
        let start_time_penalty = gtk::Button::new_with_label("START");


// Time Penalty Add/Edit Page
        let penalty_1min = gtk::Button::new_with_label("1 MIN");
        let penalty_2min = gtk::Button::new_with_label("2 MIN");
        let penalty_5min = gtk::Button::new_with_label("5 MIN");
        let penalty_dismiss = gtk::Button::new_with_label("DISMISS");
        let delete_time_penalty = gtk::Button::new_with_label("DELETE");
        let start_time_penalty = gtk::Button::new_with_label("START");


// Game Information Edit Page
        let game_information_edit_layout = gtk::Box::new(gtk::Orientation::Vertical, BUTTON_SPACING);
        game_information_edit_layout.set_margin_top(BUTTON_MARGIN);
        game_information_edit_layout.set_margin_start(BUTTON_MARGIN);
        game_information_edit_layout.set_margin_bottom(BUTTON_MARGIN);
        game_information_edit_layout.set_margin_end(BUTTON_MARGIN);

        let game_information_edit_submit = gtk::Button::new_with_label("SUBMIT");
        game_information_edit_submit.get_style_context().add_class("green");

        game_information_edit_layout.pack_start(&game_information_edit_submit, false, false, 0);
        

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

        let minute_plus = gtk::Button::new_with_label("+");     
        let minute_minus = gtk::Button::new_with_label("-");
        let second_plus = gtk::Button::new_with_label("+");
        let second_minus = gtk::Button::new_with_label("-");
        let time_edit_cancel = gtk::Button::new_with_label("CANCEL");
        time_edit_cancel.get_style_context().add_class("red");
        let time_edit_submit = gtk::Button::new_with_label("SUBMIT");
        time_edit_submit.get_style_context().add_class("green");

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



// Roster Edit Page



// Build the Stack, which switches between screen layouts
        let layout_stack = gtk::Stack::new();
        layout_stack.add_named(&main_layout, "Main Layout");
        layout_stack.add_named(&time_edit_layout, "Time Edit Layout");
        layout_stack.add_named(&game_information_edit_layout, "Game Information Edit Layout");
        layout_stack.add_named(&new_score_layout, "New Score Layout");





// Set up the buttons to switch between layouts
        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        game_information_edit_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        time_edit_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        score_submit.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        score_cancel.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

        let time_edit_layout_ = time_edit_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_time.connect_clicked(move |_| layout_stack_.set_visible_child(&time_edit_layout_));

        let new_score_layout_ = new_score_layout.clone();
        let layout_stack_ = layout_stack.clone();
        add_white_score.connect_clicked(move |_| layout_stack_.set_visible_child(&new_score_layout_));

        let new_score_layout_ = new_score_layout.clone();
        let layout_stack_ = layout_stack.clone();
        add_black_score.connect_clicked(move |_| layout_stack_.set_visible_child(&new_score_layout_));
           
        let game_information_edit_layout_ = game_information_edit_layout.clone();
        let layout_stack_ = layout_stack.clone();
        edit_game_information.connect_clicked(move |_| layout_stack_.set_visible_child(&game_information_edit_layout_));

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

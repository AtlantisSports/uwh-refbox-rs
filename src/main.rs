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
const BUTTON_MARGIN: i32 = 100;
const RETURN_BTN_SIZE_X: i32 = 400;
const RETURN_BTN_SIZE_Y: i32 = 250;

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
        win.set_resizable(false);

        // Make some buttons
        let button = gtk::Button::new_with_label("Button");
        let clickable_button = gtk::Button::new_with_label("CLICK ME!!!");
        let named_button = gtk::Button::new_with_label("Named Button");
        let return_button = gtk::Button::new_with_label("Go Back");

        // Apply the "blue" style to the button
        named_button.get_style_context().add_class("blue");

        // Request a specific size for the return button
        return_button.set_size_request(RETURN_BTN_SIZE_X, RETURN_BTN_SIZE_Y);

        // Make a text box (gtk calls it a Label)
        let game_time = gtk::Label::new(Some("98:76"));
        game_time.get_style_context().add_class("game-time");

        // Set up the main layout
        let main_layout = gtk::Box::new(gtk::Orientation::Vertical, BUTTON_SPACING);
        main_layout.set_margin_top(BUTTON_MARGIN);
        main_layout.set_margin_start(BUTTON_MARGIN);
        main_layout.set_margin_end(BUTTON_MARGIN);
        main_layout.set_margin_bottom(BUTTON_MARGIN);
        main_layout.add(&game_time);
        main_layout.add(&button);
        main_layout.add(&clickable_button);
        main_layout.add(&named_button);

        // Setup the second page with a 3x3 grid layout
        let second_layout = gtk::Grid::new();
        second_layout.insert_column(0);
        second_layout.insert_column(1);
        second_layout.insert_column(2);
        second_layout.insert_row(0);
        second_layout.insert_row(1);
        second_layout.insert_row(2);
        second_layout.set_column_spacing(BUTTON_SPACING.try_into().unwrap());
        second_layout.set_row_spacing(BUTTON_SPACING.try_into().unwrap());
        // Add the button to the middle of the grid
        second_layout.attach(&return_button, 1, 1, 1, 1);
        // Add something to the empty columns/rows so GTK doesn't remove them
        let empty_label_1 = gtk::Label::new(None);
        let empty_label_2 = gtk::Label::new(None);
        second_layout.attach(&empty_label_1, 0, 0, 1, 1);
        second_layout.attach(&empty_label_2, 2, 2, 1, 1);
        // Have the empty labels expand to fill space not requested by the `return_button`
        empty_label_1.set_hexpand(true);
        empty_label_1.set_vexpand(true);
        empty_label_2.set_hexpand(true);
        empty_label_2.set_vexpand(true);

        // Build the Stack, which switches between screen layouts
        let layout_stack = gtk::Stack::new();
        layout_stack.add_named(&main_layout, "main-layout");
        layout_stack.add_named(&second_layout, "second-layout");

        // Set up the buttons to switch between layouts
        let second_layout_ = second_layout.clone();
        let layout_stack_ = layout_stack.clone();
        clickable_button.connect_clicked(move |_| layout_stack_.set_visible_child(&second_layout_));

        let main_layout_ = main_layout.clone();
        let layout_stack_ = layout_stack.clone();
        return_button.connect_clicked(move |_| layout_stack_.set_visible_child(&main_layout_));

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

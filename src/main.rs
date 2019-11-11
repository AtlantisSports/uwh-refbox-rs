use gio::prelude::*;
use gtk::prelude::*;
use log::*;
use std::convert::TryInto;

mod css;

const SCREEN_X: i32 = 800;
const SCREEN_Y: i32 = 480;
const BUTTON_SPACING: i32 = 10;
const BUTTON_MARGIN: i32 = 100;
const RETURN_BTN_SIZE_X: i32 = 400;
const RETURN_BTN_SIZE_Y: i32 = 250;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This allows the use of error!(), warn!(), info!(), etc.
    env_logger::init();

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
        win.set_default_size(SCREEN_X, SCREEN_Y);
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

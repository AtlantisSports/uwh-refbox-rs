// Stub module for rustfmt/module resolution.
// The real simulator UI lives under refbox/src/sim_app.
// In this crate, the simulator UI is gated behind the "sim-ui" feature.
// This stub keeps default builds and workspace formatting working cross-platform.

#[allow(dead_code)]
pub struct SimRefBoxAppFlags {
    pub tcp_port: u16,
    pub sunlight_mode: bool,
}

#[allow(dead_code)]
pub struct SimRefBoxApp;

#[allow(dead_code)]
pub fn sunlight_window_size(_scale: f32) -> iced::Size {
    iced::Size::new(800.0, 600.0)
}

#[allow(dead_code)]
pub fn matrix_window_size(_scale: f32, _spacing: f32) -> iced::Size {
    iced::Size::new(800.0, 600.0)
}

impl SimRefBoxApp {
    #[allow(dead_code)]
    pub fn update(&mut self) {}

    #[allow(dead_code)]
    pub fn view(&self) {}

    #[allow(dead_code)]
    pub fn subscription() {}

    #[allow(dead_code)]
    pub fn application_style() {}

    #[allow(dead_code)]
    pub fn new(_flags: SimRefBoxAppFlags) -> Self {
        Self
    }
}


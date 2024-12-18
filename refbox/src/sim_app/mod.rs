use arrayref::array_ref;
use iced::{
    Element, Length, Point, Rectangle, Renderer, Size, Subscription, Task, Theme,
    application::Appearance,
    exit,
    mouse::Cursor,
    widget::canvas::{Cache, Canvas, Fill, Geometry, Program},
};
use led_panel_sim::DisplayState;
use log::*;
use matrix_drawing::{draw_panels, transmitted_data::TransmittedData};
use std::sync::atomic::{AtomicU16, Ordering};
use std::{rc::Rc, sync::Mutex};
use tokio::{
    net::TcpStream,
    time::{self, Duration},
};

mod display_simulator;
use display_simulator::*;

mod sunlight_display;
use sunlight_display::*;

use crate::app::theme::{BLACK, WHITE};

const WIDTH: usize = 256;
const HEIGHT: usize = 64;

static TCP_PORT: AtomicU16 = AtomicU16::new(0);

pub fn matrix_window_size(scale: f32, spacing: f32) -> Size {
    Size::new(
        WIDTH as f32 * scale + ((WIDTH as f32 + 1.0) * spacing),
        HEIGHT as f32 * scale + ((HEIGHT as f32 + 1.0) * spacing),
    )
}

pub fn sunlight_window_size(matrix_scale: f32) -> Size {
    let matrix_height = HEIGHT as f32 * matrix_scale + ((HEIGHT as f32 + 1.0) * matrix_scale / 4.0);
    let scale = matrix_height / PANEL_HEIGHT;
    Size::new(PANEL_WIDTH * scale, PANEL_HEIGHT * scale)
}

#[derive(Clone, Debug)]
pub enum Message {
    NewSnapshot(TransmittedData),
    Stop,
}

#[derive(Debug)]
enum DisplaySim {
    Matrix(Box<DisplayBuffer<WIDTH, HEIGHT>>),
    Sunlight(DisplayState),
}

#[derive(Debug)]
pub struct SimRefBoxApp {
    buffer: Rc<Mutex<DisplaySim>>,
    cache: Cache,
}

#[derive(Clone, Debug)]
pub struct SimRefBoxAppFlags {
    pub tcp_port: u16,
    pub sunlight_mode: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ApplicationTheme {
    #[default]
    Dark,
}

impl SimRefBoxApp {
    pub(super) fn new(flags: SimRefBoxAppFlags) -> (Self, Task<Message>) {
        let SimRefBoxAppFlags {
            tcp_port,
            sunlight_mode,
        } = flags;

        let buffer = if sunlight_mode {
            DisplaySim::Sunlight(DisplayState::OFF)
        } else {
            DisplaySim::Matrix(Default::default())
        };

        TCP_PORT.store(tcp_port, Ordering::SeqCst);

        (
            Self {
                buffer: Rc::new(Mutex::new(buffer)),
                cache: Cache::new(),
            },
            Task::none(),
        )
    }

    pub(super) fn subscription(&self) -> Subscription<Message> {
        Subscription::run(snapshot_listener)
    }

    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        trace!("Handling message: {message:?}");
        match message {
            Message::NewSnapshot(data) => {
                let mut buffer = self.buffer.lock().unwrap();
                match *buffer {
                    DisplaySim::Matrix(ref mut buffer) => {
                        buffer.clear_buffer();
                        draw_panels::<DisplayBuffer<WIDTH, HEIGHT>>(
                            &mut *buffer,
                            data.snapshot,
                            data.white_on_right,
                            data.flash,
                            data.beep_test,
                        )
                        .unwrap();
                    }
                    DisplaySim::Sunlight(ref mut state) => {
                        (*state, _) = DisplayState::from_transmitted_data(&data);
                    }
                }
                self.cache.clear();
                Task::none()
            }
            Message::Stop => exit(),
        }
    }

    pub(super) fn view(&self) -> Element<Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn application_style(&self, _theme: &Theme) -> Appearance {
        Appearance {
            background_color: BLACK,
            text_color: WHITE,
        }
    }
}

impl<Message> Program<Message> for SimRefBoxApp {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let buffer_ = self.buffer.clone();
        let panel = self.cache.draw(renderer, bounds.size(), |frame| {
            let buffer = buffer_.lock().unwrap();

            match *buffer {
                DisplaySim::Matrix(ref buffer) => {
                    let horiz_spacing = frame.width() / ((WIDTH * 5 + 1) as f32);
                    let vert_spacing = frame.height() / ((HEIGHT * 5 + 1) as f32);
                    let spacing = if horiz_spacing > vert_spacing {
                        vert_spacing
                    } else {
                        horiz_spacing
                    };
                    let scale = spacing * 4.0;

                    for (x, y, maybe) in buffer.iter().enumerate().flat_map(|(y, row)| {
                        row.iter().enumerate().map(move |(x, maybe)| (x, y, maybe))
                    }) {
                        if let Some(color) = maybe {
                            let x = spacing + x as f32 * (scale + spacing);
                            let y = spacing + y as f32 * (scale + spacing);
                            frame.fill_rectangle(
                                Point::new(x, y),
                                Size::new(scale, scale),
                                Fill::from(*color),
                            );
                        }
                    }
                }
                DisplaySim::Sunlight(ref state) => {
                    let scale = calculate_scale(frame.width(), frame.height());
                    for (point, size, color) in
                        static_rectangles(scale).chain(led_panel_rectangles(state, scale))
                    {
                        frame.fill_rectangle(point, size, Fill::from(color));
                    }

                    for text in static_text(scale) {
                        frame.fill_text(text);
                    }
                }
            }
        });

        vec![panel]
    }
}

fn snapshot_listener() -> impl futures_lite::Stream<Item = Message> {
    use iced::futures::SinkExt;
    info!("Sim: starting listener");

    iced::stream::channel(100, async move |mut msg_tx| {
        use tokio::io::AsyncReadExt;

        let mut fail_count = 0;
        let port = TCP_PORT.load(Ordering::SeqCst);

        let stream = loop {
            match TcpStream::connect(("localhost", port)).await {
                Ok(conn) => break Some(conn),
                Err(e) => {
                    warn!("Sim: Failed to connect to refbox: {e:?}");
                    fail_count += 1;
                    time::sleep(Duration::from_millis(500)).await;
                    if fail_count > 20 {
                        error!("Failed to connect to refbox too many times. Quitting");
                        msg_tx.send(Message::Stop).await.unwrap();
                        break None;
                    }
                    continue;
                }
            };
        };

        if let Some(mut stream) = stream {
            loop {
                // Make the buffer longer than needed so that we can detect messages that are too long
                let mut buffer = [0u8; TransmittedData::ENCODED_LEN + 1];

                match stream.read(&mut buffer).await {
                    Ok(val) if val == TransmittedData::ENCODED_LEN => {}
                    Ok(0) => {
                        error!("Sim: TCP connection closed, stopping");
                        msg_tx.send(Message::Stop).await.unwrap();
                        break;
                    }
                    Ok(val) => {
                        warn!("Sim: Received message of wrong length: {val}");
                        continue;
                    }
                    Err(e) => {
                        error!("Sim: TCP error: {e:?}");
                        error!("Sim: Stopping");
                        msg_tx.send(Message::Stop).await.unwrap();
                        break;
                    }
                }

                let data = match TransmittedData::decode(array_ref![
                    buffer,
                    0,
                    TransmittedData::ENCODED_LEN
                ]) {
                    Ok(val) => val,
                    Err(e) => {
                        warn!("Sim: Decoding error: {e:?}");
                        continue;
                    }
                };

                msg_tx.send(Message::NewSnapshot(data)).await.unwrap();
            }
        }
    })
}

use arrayref::array_ref;
use iced::{
    application, executor,
    mouse::Cursor,
    widget::canvas::{Cache, Canvas, Fill, Geometry, Program},
    Application, Color, Command, Length, Point, Rectangle, Size, Subscription,
};
use iced_futures::{
    futures::{
        future::{pending, Pending},
        stream::{self, BoxStream},
    },
    subscription::{EventStream, Recipe},
};
use iced_runtime::{command, window};
use log::*;
use matrix_drawing::{draw_panels, transmitted_data::TransmittedData};
use std::{rc::Rc, sync::Mutex};
use tokio::{
    net::TcpStream,
    time::{self, Duration},
};

mod display_simulator;
use display_simulator::*;

const WIDTH: usize = 256;
const HEIGHT: usize = 64;

pub fn window_size(scale: f32, spacing: f32) -> (u32, u32) {
    (
        (WIDTH as f32 * scale + ((WIDTH as f32 + 1.0) * spacing)).ceil() as u32,
        (HEIGHT as f32 * scale + ((HEIGHT as f32 + 1.0) * spacing)).ceil() as u32,
    )
}

#[derive(Clone, Debug)]
pub enum Message {
    NewSnapshot(TransmittedData),
    Stop,
    NoAction,
}

#[derive(Debug)]
pub struct SimRefBoxApp {
    buffer: Rc<Mutex<DisplayBuffer<WIDTH, HEIGHT>>>,
    cache: Cache,
    listener: SnapshotListener,
}

#[derive(Clone, Debug)]
pub struct SimRefBoxAppFlags {
    pub tcp_port: u16,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ApplicationTheme {
    #[default]
    Dark,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ApplicationStyle {}

impl application::StyleSheet for ApplicationTheme {
    type Style = ApplicationStyle;

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: Color::BLACK,
            text_color: Color::BLACK,
        }
    }
}

type Renderer = iced_renderer::Renderer<ApplicationTheme>;
type Element<'a, Message> = iced::Element<'a, Message, Renderer>;

impl Application for SimRefBoxApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = ApplicationTheme;
    type Flags = SimRefBoxAppFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let Self::Flags { tcp_port } = flags;

        (
            Self {
                buffer: Rc::new(Mutex::new(Default::default())),
                cache: Cache::new(),
                listener: SnapshotListener { port: tcp_port },
            },
            Command::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::from_recipe(self.listener.clone())
    }

    fn title(&self) -> String {
        "Panel Simulator".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        trace!("Handling message: {message:?}");
        match message {
            Message::NewSnapshot(data) => {
                let mut buffer = self.buffer.lock().unwrap();
                buffer.clear_buffer();
                draw_panels(
                    &mut *buffer,
                    data.snapshot,
                    data.white_on_right,
                    data.flash,
                    data.beep_test,
                )
                .unwrap();
                self.cache.clear();
                Command::none()
            }
            Message::Stop => Command::single(command::Action::Window(window::Action::Close)),
            Message::NoAction => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> Program<Message, Renderer> for SimRefBoxApp {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &ApplicationTheme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let buffer_ = self.buffer.clone();
        let panel =
            self.cache.draw(renderer, bounds.size(), |frame| {
                let buffer = buffer_.lock().unwrap();

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
            });

        vec![panel]
    }
}

#[derive(Clone, Debug)]
struct SnapshotListener {
    port: u16,
}

impl Recipe for SnapshotListener {
    type Output = Message;

    fn hash(&self, state: &mut iced_core::Hasher) {
        use std::hash::Hash;

        "SnapshotListener".hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        info!("Sim: starting listener");

        #[derive(Debug)]
        struct State {
            stream: Option<TcpStream>,
            stop: bool,
            fail_count: u8,
        }

        let state = State {
            stream: None,
            stop: false,
            fail_count: 0,
        };

        let port = self.port;

        Box::pin(stream::unfold(state, move |mut state| async move {
            use tokio::io::AsyncReadExt;

            if state.stop {
                let pend: Pending<()> = pending();
                // Won't ever return
                pend.await;
            }

            if state.stream.is_none() {
                match TcpStream::connect(("localhost", port)).await {
                    Ok(conn) => state.stream = Some(conn),
                    Err(e) => {
                        warn!("Sim: Failed to connect to refbox: {e:?}");
                        state.fail_count += 1;
                        time::sleep(Duration::from_millis(500)).await;
                        if state.fail_count > 20 {
                            state.stop = true;
                            error!("Failed to connect to refbox too many times. Quitting");
                            return Some((Message::Stop, state));
                        }
                        return Some((Message::NoAction, state));
                    }
                };
            }

            // Make the buffer longer than needed so that we can detect messages that are too long
            let mut buffer = [0u8; TransmittedData::ENCODED_LEN + 1];

            match state.stream.as_mut().unwrap().read(&mut buffer).await {
                Ok(val) if val == TransmittedData::ENCODED_LEN => {}
                Ok(0) => {
                    error!("Sim: TCP connection closed, stopping");
                    state.stop = true;
                    return Some((Message::Stop, state));
                }
                Ok(val) => {
                    warn!("Sim: Received message of wrong length: {val}");
                    return Some((Message::NoAction, state));
                }
                Err(e) => {
                    error!("Sim: TCP error: {e:?}");
                    error!("Sim: Stopping");
                    state.stop = true;
                    return Some((Message::Stop, state));
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
                    return Some((Message::NoAction, state));
                }
            };

            Some((Message::NewSnapshot(data), state))
        }))
    }
}

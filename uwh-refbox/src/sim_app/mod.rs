use arrayref::array_ref;
use iced::{
    canvas::{Cache, Cursor, Fill, Geometry, Program},
    executor, Application, Canvas, Color, Command, Element, Length, Point, Rectangle, Size,
    Subscription,
};
use iced_futures::{
    futures::{
        future::{pending, Pending},
        stream::{self, BoxStream},
    },
    subscription::Recipe,
};
use log::*;
use std::{hash::Hasher, rc::Rc, sync::Mutex};
use tokio::{
    net::TcpStream,
    time::{self, Duration},
};
use uwh_matrix_drawing::{draw_panels, transmitted_data::TransmittedData};

mod display_simulator;
use display_simulator::*;

const WINDOW_BACKGROUND: Color = Color::BLACK;
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
    scale: f32,
    spacing: f32,
    cache: Cache,
    listener: SnapshotListener,
    should_stop: bool,
}

#[derive(Clone, Debug)]
pub struct SimRefBoxAppFlags {
    pub tcp_port: u16,
    pub scale: f32,
    pub spacing: f32,
}

impl Application for SimRefBoxApp {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = SimRefBoxAppFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let Self::Flags {
            tcp_port,
            scale,
            spacing,
        } = flags;

        (
            Self {
                buffer: Rc::new(Mutex::new(Default::default())),
                scale,
                spacing,
                cache: Cache::new(),
                listener: SnapshotListener { port: tcp_port },
                should_stop: false,
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

    fn background_color(&self) -> iced::Color {
        WINDOW_BACKGROUND
    }

    fn should_exit(&self) -> bool {
        self.should_stop
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        trace!("Handling message: {message:?}");
        match message {
            Message::NewSnapshot(data) => {
                let mut buffer = self.buffer.lock().unwrap();
                buffer.clear_buffer();
                draw_panels(&mut *buffer, data.snapshot, data.white_on_right).unwrap();
                self.cache.clear();
            }
            Message::Stop => self.should_stop = true,
            Message::NoAction => {}
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> Program<Message> for SimRefBoxApp {
    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
        let buffer_ = self.buffer.clone();
        let panel =
            self.cache.draw(bounds.size(), |frame| {
                let buffer = buffer_.lock().unwrap();

                for (x, y, maybe) in buffer.iter().enumerate().flat_map(|(y, row)| {
                    row.iter().enumerate().map(move |(x, maybe)| (x, y, maybe))
                }) {
                    if let Some(color) = maybe {
                        let x = self.spacing + x as f32 * (self.scale + self.spacing);
                        let y = self.spacing + y as f32 * (self.scale + self.spacing);
                        frame.fill_rectangle(
                            Point::new(x, y),
                            Size::new(self.scale, self.scale),
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

impl<H: Hasher, I> Recipe<H, I> for SnapshotListener {
    type Output = Message;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        "SnapshotListener".hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
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
                Ok(val) if val == 0 => {
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

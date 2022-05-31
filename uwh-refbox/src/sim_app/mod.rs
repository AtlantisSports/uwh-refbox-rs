use arrayref::array_ref;
use embedded_graphics::pixelcolor::Rgb888;
use iced::{
    executor,
    pure::{Application, Element},
    Color, Command, Subscription,
};
use iced_futures::{
    futures::{
        future::{pending, Pending},
        stream::{self, BoxStream},
    },
    subscription::Recipe,
};
use log::*;
use std::hash::Hasher;
use tokio::net::TcpStream;
use uwh_matrix_drawing::{draw_panels, transmitted_data::TransmittedData};

mod display_simulator;
use display_simulator::*;

const WINDOW_BACKGROUND: Color = Color::BLACK;
const WIDTH: usize = 256;
const HEIGHT: usize = 64;

pub const fn window_size(scale: u16, spacing: u16) -> (u32, u32) {
    (
        WIDTH as u32 * (scale as u32) + ((WIDTH as u32 + 1) * (spacing as u32)),
        HEIGHT as u32 * (scale as u32) + ((HEIGHT as u32 + 1) * (spacing as u32)),
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
    simulator: DisplaySimulator<WIDTH, HEIGHT, Rgb888>,
    listener: SnapshotListener,
    has_subscribed: bool,
    should_stop: bool,
}

#[derive(Clone, Debug)]
pub struct SimRefBoxAppFlags {
    pub tcp_port: u16,
    pub scale: u16,
    pub spacing: u16,
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
                simulator: DisplaySimulator::new(scale, spacing),
                listener: SnapshotListener { port: tcp_port },
                has_subscribed: false,
                should_stop: false,
            },
            Command::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        if !self.has_subscribed {
            Subscription::from_recipe(self.listener.clone())
        } else {
            Subscription::none()
        }
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
                self.simulator.clear_buffer();
                draw_panels(&mut self.simulator, data.snapshot, data.white_on_right).unwrap();
            }
            Message::Stop => self.should_stop = true,
            Message::NoAction => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        self.simulator.view().into()
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
        }

        let state = State {
            stream: None,
            stop: false,
        };

        let port = self.port;

        Box::pin(stream::unfold(state, move |mut state| async move {
            use tokio::io::AsyncReadExt;

            if state.stop {
                let pend: Pending<()> = pending();
                // Won't ever return
                pend.await;
            }

            if matches!(state.stream, None) {
                match TcpStream::connect(("localhost", port)).await {
                    Ok(conn) => state.stream = Some(conn),
                    Err(e) => {
                        error!("Sim: Failed to connect to refbox: {e:?}");
                        state.stop = true;
                        return Some((Message::Stop, state));
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

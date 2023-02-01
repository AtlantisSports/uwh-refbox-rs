use futures_lite::future::FutureExt;
use log::*;
use matrix_drawing::transmitted_data::TransmittedData;
use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};
use thiserror::Error;
use tokio::{
    io::{self, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    select,
    sync::mpsc::{self, error::TrySendError},
    task::{self, JoinHandle},
    time::{sleep_until, timeout, Duration, Instant},
};
use tokio_serial::{SerialPortBuilder, SerialPortBuilderExt, SerialStream};
use uwh_common::game_snapshot::{EncodingError, GameSnapshot, GameSnapshotNoHeap};

const TIMEOUT: Duration = Duration::from_millis(500);
const SERIAL_SEND_SPACING: Duration = Duration::from_millis(100);
const WORKER_CHANNEL_LEN: usize = 4;
const FLASH_DURATION: Duration = Duration::from_millis(300); // Used by the simulator
const FLASH_LENGTH: u8 = 3; // Number of transmit cycles to flash for (each cycle is 100ms)

#[derive(Debug)]
pub struct UpdateSender {
    tx: mpsc::Sender<ServerMessage>,
    server_join: JoinHandle<()>,
    listener_join: JoinHandle<()>,
}

impl UpdateSender {
    pub fn new(initial: Vec<SerialPortBuilder>, binary_port: u16, json_port: u16) -> Self {
        let (tx, rx) = mpsc::channel(8);

        let initial = initial
            .into_iter()
            .map(|builder| builder.open_native_async().unwrap())
            .collect();

        let server_join = task::spawn(Server::new(rx, initial).run_loop());

        let listener_join = task::spawn(listener_loop(tx.clone(), binary_port, json_port));

        Self {
            tx,
            server_join,
            listener_join,
        }
    }

    pub fn send_snapshot(
        &self,
        snapshot: GameSnapshot,
        white_on_right: bool,
    ) -> Result<(), TrySendError<GameSnapshot>> {
        self.tx
            .try_send(ServerMessage::NewSnapshot(snapshot, white_on_right))
            .map_err(|e| match e {
                TrySendError::Full(ServerMessage::NewSnapshot(snapshot, _)) => {
                    TrySendError::Full(snapshot)
                }
                TrySendError::Closed(ServerMessage::NewSnapshot(snapshot, _)) => {
                    TrySendError::Closed(snapshot)
                }
                _ => unreachable!(),
            })
    }

    pub fn get_trigger_flash_fn(
        &self,
    ) -> impl Send + Fn() -> Result<(), TrySendError<ServerMessage>> {
        let tx = self.tx.clone();
        move || tx.try_send(ServerMessage::TriggerFlash)
    }
}

impl Drop for UpdateSender {
    fn drop(&mut self) {
        if self.tx.try_send(ServerMessage::Stop).is_err() {
            self.server_join.abort();
        }
        self.listener_join.abort();
    }
}

#[derive(Debug, Error)]
enum WorkerError {
    #[error("The sender closed the channel")]
    ChannelClosed,
    #[error("The sender sent an illegal first message")]
    IllegalMessage,
    #[error(transparent)]
    EncodingError(#[from] EncodingError),
    #[error(transparent)]
    IoError(#[from] io::Error),
}

async fn worker_loop<T: AsyncWrite + Debug + Unpin + Send>(
    mut rx: mpsc::Receiver<Vec<u8>>,
    mut write: T,
) -> Result<(), WorkerError> {
    loop {
        let data = rx.recv().await.ok_or(WorkerError::ChannelClosed)?;

        match timeout(TIMEOUT, write.write_all(&data[..])).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                error!("Send to {:?} failed: {e:?}", write);
                Err(e)?;
            }
            Err(_) => {
                warn!("Send to {:?} timed out", write);
            }
        }
    }
}

#[derive(Debug)]
enum SerialWorkerMessage {
    NewSnapshot(GameSnapshotNoHeap, bool),
    TriggerFlash,
}

async fn serial_worker_loop(
    mut rx: mpsc::Receiver<SerialWorkerMessage>,
    mut write: SerialStream,
) -> Result<(), WorkerError> {
    let msg = rx.recv().await.ok_or(WorkerError::ChannelClosed)?;
    let (snapshot, white_on_right) = match msg {
        SerialWorkerMessage::NewSnapshot(snapshot, white_on_right) => (snapshot, white_on_right),
        SerialWorkerMessage::TriggerFlash => {
            return Err(WorkerError::IllegalMessage);
        }
    };

    let mut data = TransmittedData {
        snapshot,
        flash: false,
        white_on_right,
    };
    let mut bytes = data.encode()?;

    let mut next_send = Instant::now() + SERIAL_SEND_SPACING;
    let mut counter = 0u8;

    loop {
        select! {
            _ = sleep_until(next_send) => {
                match write.try_write(&bytes[..]) {
                    Ok(bytes_written) if bytes_written == bytes.len() => {},
                    Ok(bytes_written) => warn!("An incorrect number of bytes was written to the serial port: {bytes_written}"),
                    Err(e) => error!("Error writing to serial port: {e:?}"),
                }
                next_send += SERIAL_SEND_SPACING;
                if data.flash {
                    counter += 1;
                    if counter >= FLASH_LENGTH {
                        data.flash = false;
                        bytes = data.encode()?;
                    }
                } else {
                    counter = 0;
                }
            }
            recv = rx.recv() => {
                match recv {
                    Some(SerialWorkerMessage::NewSnapshot(snapshot, white_on_right)) => {
                        data.snapshot = snapshot;
                        data.white_on_right = white_on_right;
                        bytes = data.encode()?;
                    }
                    Some(SerialWorkerMessage::TriggerFlash) => {
                        data.flash = true;
                        bytes = data.encode()?;
                    }
                    None => return Err(WorkerError::ChannelClosed),
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SendType {
    Binary,
    Json,
}

#[derive(Debug)]
enum WorkerTx {
    Binary(mpsc::Sender<Vec<u8>>),
    Json(mpsc::Sender<Vec<u8>>),
    Serial(mpsc::Sender<SerialWorkerMessage>),
}

#[derive(Debug)]
struct WorkerHandle {
    tx: WorkerTx,
    join: JoinHandle<Result<(), WorkerError>>,
}

impl WorkerHandle {
    fn new_binary(tx: mpsc::Sender<Vec<u8>>, join: JoinHandle<Result<(), WorkerError>>) -> Self {
        WorkerHandle {
            tx: WorkerTx::Binary(tx),
            join,
        }
    }

    fn new_json(tx: mpsc::Sender<Vec<u8>>, join: JoinHandle<Result<(), WorkerError>>) -> Self {
        WorkerHandle {
            tx: WorkerTx::Json(tx),
            join,
        }
    }

    fn new_serial(
        tx: mpsc::Sender<SerialWorkerMessage>,
        join: JoinHandle<Result<(), WorkerError>>,
    ) -> Self {
        WorkerHandle {
            tx: WorkerTx::Serial(tx),
            join,
        }
    }

    fn is_binary(&self) -> bool {
        matches!(self.tx, WorkerTx::Binary(_))
    }

    fn is_json(&self) -> bool {
        matches!(self.tx, WorkerTx::Json(_))
    }

    fn is_serial(&self) -> bool {
        matches!(self.tx, WorkerTx::Serial(_))
    }

    fn send(
        &self,
        binary: &[u8],
        json: &[u8],
        snapshot: &GameSnapshotNoHeap,
        white_on_right: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.tx {
            WorkerTx::Binary(ref tx) => tx.try_send(Vec::from(binary))?,
            WorkerTx::Json(ref tx) => tx.try_send(Vec::from(json))?,
            WorkerTx::Serial(ref tx) => tx.try_send(SerialWorkerMessage::NewSnapshot(
                snapshot.clone(),
                white_on_right,
            ))?,
        };
        Ok(())
    }
}

#[derive(Debug)]
pub enum ServerMessage {
    NewConnection(SendType, TcpStream),
    NewSnapshot(GameSnapshot, bool),
    TriggerFlash,
    Stop,
}

#[derive(Debug)]
struct Server {
    next_id: usize,
    senders: HashMap<usize, WorkerHandle>,
    rx: mpsc::Receiver<ServerMessage>,
    has_binary: bool,
    has_json: bool,
    snapshot: GameSnapshotNoHeap,
    white_on_right: bool,
    flash: bool,
    binary: Vec<u8>,
    json: Vec<u8>,
}

impl Server {
    pub fn new(rx: mpsc::Receiver<ServerMessage>, initial: Vec<SerialStream>) -> Self {
        let mut server = Server {
            next_id: 0,
            senders: HashMap::new(),
            rx,
            has_binary: false,
            has_json: false,
            snapshot: Default::default(),
            white_on_right: false,
            flash: false,
            binary: Vec::new(),
            json: Vec::new(),
        };

        for stream in initial {
            server.add_serial_sender(stream);
        }

        server
    }

    fn add_sender<T: 'static + AsyncWrite + Debug + Unpin + Send>(
        &mut self,
        send_type: SendType,
        sender: T,
    ) {
        let (tx, rx) = mpsc::channel(WORKER_CHANNEL_LEN);
        let join = task::spawn(worker_loop(rx, sender));

        self.senders.insert(
            self.next_id,
            match send_type {
                SendType::Binary => WorkerHandle::new_binary(tx, join),
                SendType::Json => WorkerHandle::new_json(tx, join),
            },
        );
        self.next_id += 1;

        match send_type {
            SendType::Binary => self.has_binary = true,
            SendType::Json => self.has_json = true,
        };
    }

    fn add_serial_sender(&mut self, sender: SerialStream) {
        let (tx, rx) = mpsc::channel(WORKER_CHANNEL_LEN);
        let join = task::spawn(serial_worker_loop(rx, sender));

        self.senders
            .insert(self.next_id, WorkerHandle::new_serial(tx, join));
        self.next_id += 1;

        self.has_binary = true;
    }

    fn check_types(&mut self) {
        self.has_binary = self.senders.iter().any(|(_, handle)| handle.is_binary());
        self.has_json = self.senders.iter().any(|(_, handle)| handle.is_json());
    }

    fn encode(&mut self, new_snapshot: GameSnapshot) {
        self.json = if self.has_json {
            (serde_json::to_string(&new_snapshot).unwrap() + "\n").into_bytes()
        } else {
            Vec::new()
        };

        self.snapshot = new_snapshot.into();
        self.encode_flash();
    }

    fn encode_flash(&mut self) {
        self.binary = if self.has_binary {
            Vec::from(
                TransmittedData {
                    white_on_right: self.white_on_right,
                    flash: self.flash,
                    snapshot: self.snapshot.clone(),
                }
                .encode()
                .unwrap(),
            )
        } else {
            Vec::new()
        };
    }

    fn send_to_workers(&mut self, only_binary: bool) {
        let filter = |(_, handle): &(_, &WorkerHandle)| {
            if only_binary {
                handle.is_binary()
            } else {
                true
            }
        };

        for (_, handle) in self.senders.iter().filter(filter) {
            if let Err(e) = handle.send(
                &self.binary,
                &self.json,
                &self.snapshot,
                self.white_on_right,
            ) {
                error!("Error sending to worker: {e:?}");
            }
        }
    }

    pub async fn run_loop(mut self) {
        let mut flash_ends = None;

        loop {
            let flash_end = if let Some(time) = flash_ends {
                FlashEnd::Time(Box::pin(sleep_until(time)))
            } else {
                FlashEnd::Never(core::future::pending())
            };

            select! {
                _ = flash_end => {
                    self.flash = false;
                }
                msg = self.rx.recv() => {
                    match msg {
                        Some(ServerMessage::NewConnection(send_type, stream)) => {
                            self.add_sender(send_type, stream);
                            self.check_types();
                        }
                        Some(ServerMessage::NewSnapshot(snapshot, white_on_right)) => {
                            self.white_on_right = white_on_right;
                            self.encode(snapshot);
                            self.send_to_workers(false);
                        }
                        Some(ServerMessage::TriggerFlash) => {
                            self.flash = true;
                            flash_ends = Some(Instant::now() + FLASH_DURATION);
                            self.encode_flash();
                            self.send_to_workers(true);  // Send to the binary listeners
                            for (_, handle) in self.senders.iter().filter(|(_, handle)| handle.is_serial()) {
                                if let WorkerTx::Serial(tx) = &handle.tx {
                                    if let Err(e) = tx.try_send(SerialWorkerMessage::TriggerFlash) {
                                        error!("Error sending to worker: {e:?}");
                                    }
                                }
                            }
                        }
                        Some(ServerMessage::Stop) => {
                            break;
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        for (_, handle) in self.senders.iter() {
            handle.join.abort();
        }
    }
}

async fn listener_loop(tx: mpsc::Sender<ServerMessage>, binary_port: u16, json_port: u16) {
    info!("Starting Listeners for JSON (port {json_port}) and binary (port {binary_port})");
    let binary_listener_v6 = TcpListener::bind(("::", binary_port)).await.unwrap();
    let json_listener_v6 = TcpListener::bind(("::", json_port)).await.unwrap();

    // On some OSs, we must separately listen on IPv4, but on other OSs that
    // that isn't allowed, so we just try to listen on IPv4
    let binary_listener_v4 = TcpListener::bind(("0.0.0.0", binary_port)).await.ok();
    let json_listener_v4 = TcpListener::bind(("0.0.0.0", json_port)).await.ok();

    info!("Listeners started");

    loop {
        type ListenResult = std::io::Result<(TcpStream, SocketAddr)>;
        let binary_v4_future: Pin<Box<dyn Future<Output = ListenResult> + Send>> =
            if let Some(listener) = binary_listener_v4.as_ref() {
                Box::pin(listener.accept())
            } else {
                Box::pin(iced::futures::future::pending())
            };
        let json_v4_future: Pin<Box<dyn Future<Output = ListenResult> + Send>> =
            if let Some(listener) = json_listener_v4.as_ref() {
                Box::pin(listener.accept())
            } else {
                Box::pin(iced::futures::future::pending())
            };

        select! {
            conn = binary_v4_future => {
                match conn {
                    Ok((stream, addr)) => {
                        info!("New Binary connection from {addr:?}");
                        tx.send(ServerMessage::NewConnection(SendType::Binary, stream))
                            .await
                            .unwrap();
                    }
                    Err(addr) => error!("New binary connection to {addr:?} failed"),
                }
            }
            conn = json_v4_future => {
                match conn {
                    Ok((stream, addr)) => {
                        info!("New JSON connection from {addr:?}");
                        tx.send(ServerMessage::NewConnection(SendType::Json, stream))
                            .await
                            .unwrap();
                    }
                    Err(addr) => error!("New JSON connection to {addr:?} failed"),
                }
            }
            conn = binary_listener_v6.accept() => {
                match conn {
                    Ok((stream, addr)) => {
                        info!("New Binary connection from {addr:?}");
                        tx.send(ServerMessage::NewConnection(SendType::Binary, stream))
                            .await
                            .unwrap();
                    }
                    Err(addr) => error!("New binary connection to {addr:?} failed"),
                }
            }
            conn = json_listener_v6.accept() => {
                match conn {
                    Ok((stream, addr)) => {
                        info!("New JSON connection from {addr:?}");
                        tx.send(ServerMessage::NewConnection(SendType::Json, stream))
                            .await
                            .unwrap();
                    }
                    Err(addr) => error!("New JSON connection to {addr:?} failed"),
                }
            }
        }
    }
}

enum FlashEnd {
    Never(core::future::Pending<()>),
    Time(Pin<Box<tokio::time::Sleep>>),
}

impl Future for FlashEnd {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match *self {
            Self::Never(ref mut pend) => pend.poll(cx),
            Self::Time(ref mut slp) => slp.poll(cx),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use more_asserts::*;
    use std::io::ErrorKind;
    use tokio::io::AsyncReadExt;
    use uwh_common::game_snapshot::{GamePeriod, PenaltySnapshot, PenaltyTime, TimeoutSnapshot};

    const BINARY_PORT: u16 = 12345;
    const JSON_PORT: u16 = 12346;
    const MAX_CONN_FAILS: usize = 20;

    #[tokio::test]
    async fn test_update_sender() {
        let update_sender = UpdateSender::new(vec![], BINARY_PORT, JSON_PORT);

        let mut binary_conn;
        let mut fail_count = 0;
        loop {
            match TcpStream::connect(("localhost", BINARY_PORT)).await {
                Ok(stream) => {
                    binary_conn = stream;
                    break;
                }
                Err(e) => {
                    if e.kind() == ErrorKind::ConnectionRefused {
                        assert_le!(fail_count, MAX_CONN_FAILS);
                        fail_count += 1;
                    } else {
                        panic!("Unexpected connection error: {e:?}");
                    }
                }
            };
        }

        let mut json_conn;
        let mut fail_count = 0;
        loop {
            match TcpStream::connect(("localhost", JSON_PORT)).await {
                Ok(stream) => {
                    json_conn = stream;
                    break;
                }
                Err(e) => {
                    if e.kind() == ErrorKind::ConnectionRefused {
                        assert_le!(fail_count, MAX_CONN_FAILS);
                        fail_count += 1;
                    } else {
                        panic!("Unexpected connection error: {e:?}");
                    }
                }
            };
        }

        // Make a third connection to the binary port to ensure that the server has processed the first two
        println!("Connecting to server on binary port");
        let mut fail_count = 0;
        loop {
            match TcpStream::connect(("localhost", BINARY_PORT)).await {
                Ok(_) => {
                    break;
                }
                Err(e) => {
                    if e.kind() == ErrorKind::ConnectionRefused {
                        assert_le!(fail_count, MAX_CONN_FAILS);
                        fail_count += 1;
                    } else {
                        panic!("Unexpected connection error: {e:?}");
                    }
                }
            };
        }

        let white_on_right = false;
        let flash = false;
        let snapshot = GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 897,
            timeout: TimeoutSnapshot::None,
            b_score: 2,
            w_score: 3,
            b_penalties: vec![
                PenaltySnapshot {
                    time: PenaltyTime::Seconds(57),
                    player_number: 3,
                },
                PenaltySnapshot {
                    time: PenaltyTime::Seconds(117),
                    player_number: 6,
                },
            ],
            w_penalties: vec![
                PenaltySnapshot {
                    time: PenaltyTime::Seconds(297),
                    player_number: 12,
                },
                PenaltySnapshot {
                    time: PenaltyTime::TotalDismissal,
                    player_number: 15,
                },
            ],
            is_old_game: true,
            game_number: 26,
            next_game_number: 28,
            tournament_id: 1,
            recent_goal: None,
            next_period_len_secs: Some(180),
        };

        let json_expected = serde_json::to_string(&snapshot).unwrap().into_bytes();

        let binary_expected = Vec::from(
            TransmittedData {
                white_on_right,
                flash,
                snapshot: snapshot.clone().into(),
            }
            .encode()
            .unwrap(),
        );

        update_sender
            .send_snapshot(snapshot, white_on_right)
            .unwrap();

        let expected_binary_bytes = binary_expected.len();
        let mut binary_result = vec![0u8; expected_binary_bytes];
        let mut binary_read_so_far = 0;

        let expected_json_bytes = json_expected.len();
        let mut json_result = vec![0u8; expected_json_bytes];
        let mut json_read_so_far = 0;

        while json_read_so_far < expected_json_bytes || binary_read_so_far < expected_binary_bytes {
            select! {
                bytes = binary_conn.read(&mut binary_result[binary_read_so_far..]) => {
                    binary_read_so_far += bytes.unwrap();
                }
                bytes = json_conn.read(&mut json_result[json_read_so_far..]) => {
                    json_read_so_far += bytes.unwrap();
                }
            }
        }

        assert_eq!(expected_json_bytes, json_read_so_far);
        assert_eq!(json_expected, json_result);

        assert_eq!(expected_binary_bytes, binary_read_so_far);
        assert_eq!(binary_expected, binary_result);
    }
}

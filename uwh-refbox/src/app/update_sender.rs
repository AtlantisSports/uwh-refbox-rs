use log::*;
use std::{collections::HashMap, fmt::Debug};
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
use uwh_common::game_snapshot::GameSnapshot;
use uwh_matrix_drawing::transmitted_data::TransmittedData;

const TIMEOUT: Duration = Duration::from_millis(500);
const SERIAL_SEND_SPACING: Duration = Duration::from_millis(100);
const WORKER_CHANNEL_LEN: usize = 4;

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

async fn serial_worker_loop(
    mut rx: mpsc::Receiver<Vec<u8>>,
    mut write: SerialStream,
) -> Result<(), WorkerError> {
    let mut data = rx.recv().await.ok_or(WorkerError::ChannelClosed)?;
    let mut next_send = Instant::now() + SERIAL_SEND_SPACING;

    loop {
        select! {
            _ = sleep_until(next_send) => {
                match write.try_write(&data[..]) {
                    Ok(bytes_written) if bytes_written == data.len() => {},
                    Ok(bytes_written) => warn!("An incorrect number of bytes was writeen to the serial port: {bytes_written}"),
                    Err(e) => error!("Error writing to serial port: {e:?}"),
                }
                next_send += SERIAL_SEND_SPACING;
            }
            recv = rx.recv() => {
                data = recv.ok_or(WorkerError::ChannelClosed)?;
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
struct WorkerHandle {
    send_type: SendType,
    tx: mpsc::Sender<Vec<u8>>,
    join: JoinHandle<Result<(), WorkerError>>,
}

#[derive(Debug)]
pub enum ServerMessage {
    NewConnection(SendType, TcpStream),
    NewSnapshot(GameSnapshot, bool),
    Stop,
}

#[derive(Debug)]
struct Server {
    next_id: usize,
    senders: HashMap<usize, WorkerHandle>,
    rx: mpsc::Receiver<ServerMessage>,
    has_binary: bool,
    has_json: bool,
}

impl Server {
    pub fn new(rx: mpsc::Receiver<ServerMessage>, initial: Vec<SerialStream>) -> Self {
        let mut server = Server {
            next_id: 0,
            senders: HashMap::new(),
            rx,
            has_binary: false,
            has_json: false,
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
            WorkerHandle {
                send_type,
                tx,
                join,
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

        self.senders.insert(
            self.next_id,
            WorkerHandle {
                send_type: SendType::Binary,
                tx,
                join,
            },
        );
        self.next_id += 1;

        self.has_binary = true;
    }

    fn check_types(&mut self) {
        self.has_binary = self
            .senders
            .iter()
            .any(|(_, handle)| handle.send_type == SendType::Binary);
        self.has_json = self
            .senders
            .iter()
            .any(|(_, handle)| handle.send_type == SendType::Json);
    }

    pub async fn run_loop(mut self) {
        loop {
            if let Some(msg) = self.rx.recv().await {
                match msg {
                    ServerMessage::NewConnection(send_type, stream) => {
                        self.add_sender(send_type, stream)
                    }
                    ServerMessage::NewSnapshot(snapshot, white_on_right) => {
                        //info!("Server received snapshot: {snapshot:?}");
                        let json = if self.has_json {
                            (serde_json::to_string(&snapshot).unwrap() + "\n").into_bytes()
                        } else {
                            vec![]
                        };
                        let binary = if self.has_binary {
                            Vec::from(
                                TransmittedData {
                                    white_on_right,
                                    snapshot: snapshot.into(),
                                }
                                .encode()
                                .unwrap(),
                            )
                        } else {
                            vec![]
                        };

                        let mut remove = vec![];

                        for (id, handle) in self.senders.iter_mut() {
                            let to_send = match handle.send_type {
                                SendType::Binary => binary.clone(),
                                SendType::Json => json.clone(),
                            };

                            //info!("Sending to worker: {to_send:?}");
                            match handle.tx.send(to_send).await {
                                Ok(()) => {}
                                Err(_) => remove.push(*id),
                            };
                        }

                        let removing = !remove.is_empty();
                        for id in remove {
                            self.senders.remove(&id);
                        }

                        if removing {
                            self.check_types();
                        }
                    }
                    ServerMessage::Stop => break,
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
    info!("Strating Listeners for JSON (port {json_port}) and binary (port {binary_port})");
    let binary_listener = TcpListener::bind(("::", binary_port)).await.unwrap();
    let json_listener = TcpListener::bind(("::", json_port)).await.unwrap();
    info!("Listeners started");

    loop {
        select! {
            conn = binary_listener.accept() => {
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
            conn = json_listener.accept() => {
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

        let white_on_right = false;
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
        };

        let json_expected = serde_json::to_string(&snapshot).unwrap().into_bytes();

        let binary_expected = Vec::from(
            TransmittedData {
                white_on_right,
                snapshot: snapshot.clone().into(),
            }
            .encode()
            .unwrap(),
        );

        update_sender
            .send_snapshot(snapshot, white_on_right)
            .unwrap();

        let expected_json_bytes = json_expected.len();
        let mut json_result = vec![0u8; expected_json_bytes];
        let json_bytes = json_conn
            .read_exact(&mut json_result[..expected_json_bytes])
            .await
            .unwrap();

        let expected_binary_bytes = binary_expected.len();
        let mut binary_result = vec![0u8; expected_binary_bytes];
        let binary_bytes = binary_conn
            .read_exact(&mut binary_result[..expected_binary_bytes])
            .await
            .unwrap();

        assert_eq!(expected_json_bytes, json_bytes);
        assert_eq!(json_expected, json_result);

        assert_eq!(expected_binary_bytes, binary_bytes);
        assert_eq!(binary_expected, binary_result);
    }
}

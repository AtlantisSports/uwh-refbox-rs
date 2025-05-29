use embedded_hal::spi::{ErrorType, Operation};
use embedded_hal_async::spi::SpiDevice;
use rppal::spi::{Bus, Error, Mode, Segment, SlaveSelect, Spi};
use std::{
    sync::mpsc::{Sender, channel},
    thread::{self, JoinHandle},
    vec,
};
use tokio::sync::oneshot::{Sender as OneshotSender, channel as oneshot_channel};

type Result<T> = std::result::Result<T, Error>;

enum SendableOperation {
    Write {
        data: Vec<u8>,
        delay: u16,
    },
    Read {
        count: usize,
        index: usize,
        delay: u16,
    },
    Transaction {
        data: Vec<u8>,
        count: usize,
        index: usize,
        delay: u16,
    },
}

impl SendableOperation {
    fn delay(&self) -> u16 {
        match self {
            SendableOperation::Write { delay, .. } => *delay,
            SendableOperation::Read { delay, .. } => *delay,
            SendableOperation::Transaction { delay, .. } => *delay,
        }
    }

    fn set_delay(&mut self, delay: u16) {
        match self {
            SendableOperation::Write { delay: d, .. } => *d = delay,
            SendableOperation::Read { delay: d, .. } => *d = delay,
            SendableOperation::Transaction { delay: d, .. } => *d = delay,
        }
    }
}

type SenderType = Sender<(
    Vec<SendableOperation>,
    OneshotSender<Result<Vec<(usize, Vec<u8>)>>>,
)>;

pub struct AsyncSpi {
    tx: SenderType,
    _thread: JoinHandle<()>,
}

impl AsyncSpi {
    pub fn new(bus: Bus, ss: SlaveSelect, clock_speed: u32, mode: Mode) -> Self {
        let (tx, rx): (SenderType, _) = channel();
        let spi = Spi::new(bus, ss, clock_speed, mode).unwrap();
        let thread = thread::spawn(move || {
            while let Ok((operations, tx)) = rx.recv() {
                let mut reply = vec![None; operations.len()];

                let segments = operations
                    .iter()
                    .zip(reply.iter_mut())
                    .map(|(op, r)| match op {
                        SendableOperation::Write { data, delay } => {
                            let mut seg = Segment::with_write(&data[..]);
                            seg.set_delay(*delay);
                            seg
                        }
                        SendableOperation::Read {
                            count,
                            index,
                            delay,
                            ..
                        } => {
                            *r = Some((*index, vec![0; *count]));
                            let mut seg = Segment::with_read(r.as_mut().unwrap().1.as_mut());
                            seg.set_delay(*delay);
                            seg
                        }
                        SendableOperation::Transaction {
                            data,
                            count,
                            index,
                            delay,
                            ..
                        } => {
                            *r = Some((*index, vec![0; *count]));
                            let mut seg = Segment::new(r.as_mut().unwrap().1.as_mut(), &data[..]);
                            seg.set_delay(*delay);
                            seg
                        }
                    })
                    .collect::<Vec<_>>();
                let res = spi.transfer_segments(&segments);
                let reply = reply.into_iter().flatten().collect();
                match res {
                    Ok(_) => {
                        tx.send(Ok(reply)).unwrap();
                    }
                    Err(e) => {
                        tx.send(Err(e)).unwrap();
                    }
                }
            }
        });
        Self {
            tx,
            _thread: thread,
        }
    }
}

impl ErrorType for AsyncSpi {
    type Error = Error;
}

impl SpiDevice for AsyncSpi {
    async fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<()> {
        if operations.is_empty() {
            return Ok(());
        }

        let mut sendable_ops = vec![];

        if matches!(operations[0], Operation::DelayNs(_)) {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "First operation cannot be a delay",
            )));
        }

        for (i, op) in operations.iter().enumerate() {
            match op {
                Operation::Write(data) => {
                    sendable_ops.push(SendableOperation::Write {
                        data: data.to_vec(),
                        delay: 0,
                    });
                }
                Operation::Read(buffer) => {
                    sendable_ops.push(SendableOperation::Read {
                        count: (*buffer).len(),
                        index: i,
                        delay: 0,
                    });
                }
                Operation::Transfer(buffer, data) => {
                    sendable_ops.push(SendableOperation::Transaction {
                        data: data.to_vec(),
                        count: (*buffer).len(),
                        index: i,
                        delay: 0,
                    });
                }
                Operation::TransferInPlace(data) => {
                    sendable_ops.push(SendableOperation::Transaction {
                        data: data.to_vec(),
                        count: (*data).len(),
                        index: i,
                        delay: 0,
                    });
                }
                Operation::DelayNs(delay_ns) => {
                    let delay_us = (*delay_ns as f64 / 1_000.0).ceil();
                    let delay_us = if delay_us >= u16::MIN as f64 && delay_us <= u16::MAX as f64 {
                        delay_us as u16
                    } else {
                        return Err(Error::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "Delay is too long",
                        )));
                    };
                    let last_op = sendable_ops.last_mut().unwrap();
                    let delay = last_op.delay().checked_add(delay_us).ok_or(Error::Io(
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Delay is too long"),
                    ))?;
                    last_op.set_delay(delay);
                }
            }
        }

        let (tx, rx) = oneshot_channel();
        self.tx.send((sendable_ops, tx)).unwrap();
        let reads = rx.await.unwrap()?;

        for (i, data) in reads {
            match &mut operations[i] {
                Operation::Read(buffer)
                | Operation::Transfer(buffer, _)
                | Operation::TransferInPlace(buffer) => {
                    buffer.copy_from_slice(&data);
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}

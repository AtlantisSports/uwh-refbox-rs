use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct RemoteId(u32);

impl Display for RemoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(self.0.to_be_bytes()).into_string())
    }
}

impl From<u32> for RemoteId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SoundMessage {
    TriggerBuzzer,
    TriggerWhistle,
    #[cfg(target_os = "linux")]
    StartWiredBuzzer,
    #[cfg(target_os = "linux")]
    StopWiredBuzzer,
    #[cfg(target_os = "linux")]
    WirelessRemoteReceived(RemoteId),
}

// The only systems that have the hardware for buttons are Raspberry Pis
#[cfg(target_os = "linux")]
mod button_handler_rpi {
    use super::{RemoteId, SoundMessage};
    use crc::{Crc, NoTable};
    use log::{debug, info, warn};
    use lora_phy::{
        LoRa,
        iv::GenericSx127xInterfaceVariant,
        sx127x::{Config, Sx127x, Sx1276},
    };
    use rppal::{
        gpio::{Gpio, InputPin, Trigger},
        spi::{Bus, Mode, SlaveSelect},
    };
    use tokio::{
        sync::{mpsc::UnboundedSender, watch::Sender},
        task::{self, JoinHandle},
        time::{Duration, sleep},
    };
    use wireless_modes::WirelessMode;

    mod async_spi;
    use async_spi::AsyncSpi;

    mod wait_pin;
    use wait_pin::WaitPin;

    const WIRED_BUTTON_PIN: u8 = 12;
    const RESET_PIN: u8 = 24;
    const IRQ_PIN: u8 = 25;

    const ROTARY_SWITCH_PIN_1: u8 = 6;
    const ROTARY_SWITCH_PIN_2: u8 = 20;
    const ROTARY_SWITCH_PIN_4: u8 = 16;
    const ROTARY_SWITCH_PIN_8: u8 = 26;

    struct Delay();

    impl embedded_hal_async::delay::DelayNs for Delay {
        async fn delay_ns(&mut self, ns: u32) {
            sleep(Duration::from_nanos(ns as u64)).await;
        }
    }

    pub(in super::super) struct ButtonHandler {
        _wired_pin: InputPin,
        _wireless_button_handle: Option<JoinHandle<()>>,
    }

    impl ButtonHandler {
        pub fn new(
            msg_tx: UnboundedSender<SoundMessage>,
            remote_id_tx: Sender<RemoteId>,
        ) -> Option<Self> {
            if let Ok(sys_info) = rppal::system::DeviceInfo::new() {
                info!("Detected a Raspberry Pi system: {sys_info:?}, starting GPIO processes");

                let gpio = Gpio::new().unwrap();

                let mut wired_pin = gpio.get(WIRED_BUTTON_PIN).unwrap().into_input_pullup();
                let msg_tx_ = msg_tx.clone();
                wired_pin
                    .set_async_interrupt(Trigger::Both, None, move |event| {
                        msg_tx_
                            .send(match event.trigger {
                                Trigger::RisingEdge => SoundMessage::StartWiredBuzzer,
                                Trigger::FallingEdge => SoundMessage::StopWiredBuzzer,
                                _ => unreachable!(),
                            })
                            .unwrap();
                    })
                    .unwrap();

                let one_pin = gpio.get(ROTARY_SWITCH_PIN_1).unwrap().into_input_pullup();
                let two_pin = gpio.get(ROTARY_SWITCH_PIN_2).unwrap().into_input_pullup();
                let four_pin = gpio.get(ROTARY_SWITCH_PIN_4).unwrap().into_input_pullup();
                let eight_pin = gpio.get(ROTARY_SWITCH_PIN_8).unwrap().into_input_pullup();

                let mut pins = [
                    (one_pin, false, false, 0),
                    (two_pin, false, false, 0),
                    (four_pin, false, false, 0),
                    (eight_pin, false, false, 0),
                ];

                // Wait for the rotary switch pins to stabilize, require 10 consecutive stable readings
                // spaced out by 10ms
                while pins.iter().any(|(_, stable, _, _)| !stable) {
                    for (pin, stable, last, stable_count) in &mut pins {
                        let current = pin.is_low();
                        if current == *last {
                            *stable_count += 1;
                            if *stable_count >= 10 {
                                *stable = true;
                            }
                        } else {
                            *stable_count = 0;
                            *stable = false;
                        }
                        *last = current;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                let one = pins[0].2;
                let two = pins[1].2;
                let four = pins[2].2;
                let eight = pins[3].2;
                debug!("Rotary switch pins: 1: {one}, 2: {two}, 4: {four}, 8: {eight}");

                let wireless_mode = WirelessMode::from_gray_code(one, two, four, eight);

                info!("Wireless mode selected: {wireless_mode:?}");

                let wireless_button_handle = if let Some(wireless_mode) = wireless_mode {
                    Some(task::spawn(async move {
                        let spi =
                            AsyncSpi::new(Bus::Spi0, SlaveSelect::Ss0, 2_000_000, Mode::Mode0);
                        let reset = gpio.get(RESET_PIN).unwrap().into_output();
                        let irq = gpio.get(IRQ_PIN).unwrap().into_input();
                        let irq = WaitPin::new(irq);

                        debug!("Initialized pins and SPI");

                        let iv =
                            GenericSx127xInterfaceVariant::new(reset, irq, None, None).unwrap();
                        let config = Config {
                            chip: Sx1276,
                            tcxo_used: false,
                            tx_boost: false,
                            rx_boost: true,
                        };
                        let mut lora = LoRa::new(Sx127x::new(spi, iv, config), true, Delay())
                            .await
                            .unwrap();

                        debug!("Instantiated LoRa");

                        lora.init().await.unwrap();

                        debug!("Initialized LoRa");

                        let mdltn_params = lora
                            .create_modulation_params(
                                wireless_mode.spreading_factor(),
                                wireless_mode.bandwidth(),
                                wireless_mode.coding_rate(),
                                wireless_mode.frequency(),
                            )
                            .unwrap();

                        debug!("Created modulation parameters");

                        let rx_pkt_params = lora
                            .create_rx_packet_params(4, false, 100, false, false, &mdltn_params)
                            .unwrap();

                        debug!("Created RX packet parameters");

                        lora.prepare_for_rx(
                            lora_phy::RxMode::Continuous,
                            &mdltn_params,
                            &rx_pkt_params,
                        )
                        .await
                        .unwrap();

                        debug!("Prepared for RX");

                        let mut buffer = [0; 256];

                        info!("Starting listening loop");

                        loop {
                            match lora.rx(&rx_pkt_params, &mut buffer).await {
                                Ok((size, stats)) => {
                                    if size == 5 {
                                        let crc = Crc::<u8, NoTable>::new(&crc::CRC_8_SMBUS);
                                        let expected_crc = crc.checksum(&buffer[0..4]);
                                        if buffer[4] == expected_crc {
                                            debug!(
                                                "Received packet: {:?}, CRC: {}, RSSI: {}, SNR: {}",
                                                &buffer[..(size as usize)],
                                                expected_crc == buffer[4],
                                                stats.rssi,
                                                stats.snr
                                            );
                                            let id = u32::from_be_bytes(*arrayref::array_ref![
                                                buffer, 0, 4
                                            ])
                                            .into();
                                            msg_tx
                                                .send(SoundMessage::WirelessRemoteReceived(id))
                                                .unwrap();
                                            remote_id_tx.send(id).unwrap();
                                        } else {
                                            debug!(
                                                "CRC mismatch: expected {expected_crc}, got {}",
                                                buffer[4]
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Error receiving lora packet: {e:?}");
                                }
                            }
                        }
                    }))
                } else {
                    warn!("Invalid wireless mode selected");
                    None
                };

                Some(Self {
                    _wired_pin: wired_pin,
                    _wireless_button_handle: wireless_button_handle,
                })
            } else {
                None
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub(super) use button_handler_rpi::ButtonHandler;

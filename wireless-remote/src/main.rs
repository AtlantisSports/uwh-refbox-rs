#![no_std]
#![no_main]

use crc::{Crc, NoTable};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    clocks::dormant_sleep,
    flash::{Blocking, Flash},
    gpio::{DormantWakeConfig, Input, Level, Output, Pull},
    spi::{Config, Spi},
};
use embassy_time::{Delay, Duration, Instant};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::{
    LoRa,
    iv::GenericSx127xInterfaceVariant,
    sx127x::{self, Sx127x, Sx1276},
};
use wireless_modes::WirelessMode;
use {defmt_rtt as _, panic_probe as _};

const FLASH_SIZE: usize = 2 * 1024 * 1024;

const TRANSMIT_SPACING: Duration = Duration::from_millis(250);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    info!("Program Started!");

    // add some delay to give an attached debug probe time to parse the
    // defmt RTT header. Reading that header might touch flash memory, which
    // interferes with flash write operations.
    // https://github.com/knurling-rs/defmt/pull/683
    Delay.delay_ms(10).await;

    info!("Generating ID");

    let mut flash = Flash::<_, Blocking, FLASH_SIZE>::new_blocking(p.FLASH);

    let mut buffer = [0u8; 8];
    defmt::unwrap!(flash.blocking_unique_id(&mut buffer));

    let crc = Crc::<u32, NoTable>::new(&crc::CRC_32_ISO_HDLC);
    let id = crc.checksum(&buffer);
    info!("ID: {:#08x}", id);

    let miso = p.PIN_8;
    let mosi = p.PIN_15;
    let clk = p.PIN_14;

    let mut led = Output::new(p.PIN_13, Level::Low);
    led.set_high();

    let one_pin = Input::new(p.PIN_9, Pull::Up);
    let two_pin = Input::new(p.PIN_10, Pull::Up);
    let four_pin = Input::new(p.PIN_11, Pull::Up);
    let eight_pin = Input::new(p.PIN_12, Pull::Up);

    let mut pins = [
        (one_pin, false, false, 0),
        (two_pin, false, false, 0),
        (four_pin, false, false, 0),
        (eight_pin, false, false, 0),
    ];

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
        Delay.delay_ms(1).await;
    }

    let wireless_mode = WirelessMode::from_gray_code(pins[0].2, pins[1].2, pins[2].2, pins[3].2);

    let wireless_mode = match wireless_mode {
        Some(wireless_mode) => wireless_mode,
        None => {
            error!(
                "Invalid wireless mode. Gray code was 1: {}, 2: {}, 4: {}, 8: {}",
                pins[0].2, pins[1].2, pins[2].2, pins[3].2
            );
            return;
        }
    };

    let num_flashes = match wireless_mode {
        WirelessMode::UnitedStates => 1,
        WirelessMode::Europe => 2,
        WirelessMode::Australia => 3,
    };

    for _ in 0..num_flashes {
        led.set_low();
        Delay.delay_ms(500).await;
        led.set_high();
        Delay.delay_ms(500).await;
    }

    let button = Input::new(p.PIN_5, Pull::Up);
    let mut button_wake = Input::new(p.PIN_6, Pull::None);

    let mut config = Config::default();
    config.frequency = 2_000_000;

    let spi = Spi::new(p.SPI1, clk, mosi, miso, p.DMA_CH0, p.DMA_CH1, config);

    let nss = Output::new(p.PIN_16, Level::High);
    let spi = ExclusiveDevice::new(spi, nss, Delay).unwrap();

    let config = sx127x::Config {
        chip: Sx1276,
        tcxo_used: false,
        tx_boost: true,
        rx_boost: false,
    };

    let reset = Output::new(p.PIN_17, Level::High);
    let irq = Input::new(p.PIN_21, Pull::Up);
    let iv = GenericSx127xInterfaceVariant::new(reset, irq, None, None).unwrap();
    let mut lora = LoRa::new(Sx127x::new(spi, iv, config), true, Delay)
        .await
        .unwrap();

    if let Err(err) = lora.init().await {
        error!("Radio init error = {}", err);
        return;
    }

    let mdltn_params = {
        match lora.create_modulation_params(
            wireless_mode.spreading_factor(),
            wireless_mode.bandwidth(),
            wireless_mode.coding_rate(),
            wireless_mode.frequency(),
        ) {
            Ok(mp) => {
                info!("Modulation params created");
                mp
            }
            Err(err) => {
                error!("Radio create mod params error = {}", err);
                return;
            }
        }
    };

    let mut tx_pkt_params = {
        match lora.create_tx_packet_params(4, false, true, false, &mdltn_params) {
            Ok(pp) => {
                info!("TX pkt params created");
                pp
            }
            Err(err) => {
                error!("Radio tx pkt params error = {}", err);
                return;
            }
        }
    };

    let wake_config = DormantWakeConfig {
        edge_high: false,
        edge_low: true,
        level_high: false,
        level_low: false,
    };
    let _wake = button_wake.dormant_wake(wake_config);

    let msg = id.to_be_bytes();
    let crc = Crc::<u8, NoTable>::new(&crc::CRC_8_SMBUS);
    let crc = crc.checksum(&msg);
    let msg = [msg[0], msg[1], msg[2], msg[3], crc];

    match lora.sleep(false).await {
        Ok(()) => {
            info!("Radio sleep done");
        }
        Err(err) => {
            error!("Radio sleep error = {}", err);
            return;
        }
    }

    led.set_low();

    dormant_sleep();

    let mut last_send = None;
    let mut first_tx = true;

    loop {
        if button.is_low() {
            if first_tx {
                info!("Button pressed, starting transmission");
                led.set_high();
                first_tx = false;
            }

            'inner: loop {
                match lora.prepare_for_cad(&mdltn_params).await {
                    Ok(()) => {
                        info!("CAD PREP DONE");
                    }
                    Err(err) => {
                        error!("Radio prep for cad error = {}", err);
                        led.set_high();
                        return;
                    }
                };

                match lora.cad(&mdltn_params).await {
                    Ok(false) => {
                        break 'inner;
                    }
                    Ok(true) => {
                        Delay.delay_ms(2).await;
                        continue 'inner;
                    }
                    Err(err) => {
                        error!("Radio cad error = {}", err);
                        led.set_high();
                        return;
                    }
                };
            }

            match lora
                .prepare_for_tx(
                    &mdltn_params,
                    &mut tx_pkt_params,
                    wireless_mode.tx_power(),
                    &msg,
                )
                .await
            {
                Ok(()) => {
                    info!("TX PREP DONE");
                }
                Err(err) => {
                    error!("Radio prep for tx error = {}", err);
                    led.set_high();
                    return;
                }
            };

            match lora.tx().await {
                Ok(()) => {
                    info!("TX DONE");
                }
                Err(err) => {
                    error!("Radio tx error = {}", err);
                    led.set_high();
                    return;
                }
            };

            last_send = if let Some(last) = last_send {
                Some(last + TRANSMIT_SPACING)
            } else {
                Some(Instant::now())
            };

            match lora.sleep(false).await {
                Ok(()) => {
                    info!("Radio sleep done");
                }
                Err(err) => {
                    error!("Radio sleep error = {}", err);
                    led.set_high();
                    return;
                }
            }

            Delay
                .delay_us(
                    (last_send.unwrap() + TRANSMIT_SPACING)
                        .duration_since(Instant::now())
                        .as_micros() as u32,
                )
                .await;

            led.set_low();
        } else {
            last_send = None;
            first_tx = true;
            dormant_sleep();
        }
    }
}

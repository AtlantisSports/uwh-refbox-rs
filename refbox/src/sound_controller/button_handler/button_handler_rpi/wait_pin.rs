use std::convert::Infallible;

use embedded_hal::digital::ErrorType;
use embedded_hal_async::digital::Wait;
use rppal::gpio::InputPin;
use tokio::sync::oneshot::channel;

pub struct WaitPin {
    pin: InputPin,
}

impl WaitPin {
    pub fn new(pin: InputPin) -> Self {
        Self { pin }
    }
}

impl ErrorType for WaitPin {
    type Error = Infallible;
}

impl Wait for WaitPin {
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        if self.pin.is_high() {
            return Ok(());
        }
        let (tx, rx) = channel();
        let mut tx = Some(tx);
        self.pin
            .set_async_interrupt(rppal::gpio::Trigger::RisingEdge, None, move |_| {
                if let Some(tx) = tx.take() {
                    tx.send(()).unwrap();
                }
            })
            .unwrap();
        rx.await.unwrap();
        Ok(())
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        if self.pin.is_low() {
            return Ok(());
        }
        let (tx, rx) = channel();
        let mut tx = Some(tx);
        self.pin
            .set_async_interrupt(rppal::gpio::Trigger::FallingEdge, None, move |_| {
                if let Some(tx) = tx.take() {
                    tx.send(()).unwrap();
                }
            })
            .unwrap();
        rx.await.unwrap();
        Ok(())
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        let (tx, rx) = channel();
        let mut tx = Some(tx);
        self.pin
            .set_async_interrupt(rppal::gpio::Trigger::RisingEdge, None, move |_| {
                if let Some(tx) = tx.take() {
                    tx.send(()).unwrap();
                }
            })
            .unwrap();
        rx.await.unwrap();
        Ok(())
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        let (tx, rx) = channel();
        let mut tx = Some(tx);
        self.pin
            .set_async_interrupt(rppal::gpio::Trigger::FallingEdge, None, move |_| {
                if let Some(tx) = tx.take() {
                    tx.send(()).unwrap();
                }
            })
            .unwrap();
        rx.await.unwrap();
        Ok(())
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        let (tx, rx) = channel();
        let mut tx = Some(tx);
        self.pin
            .set_async_interrupt(rppal::gpio::Trigger::Both, None, move |_| {
                if let Some(tx) = tx.take() {
                    tx.send(()).unwrap();
                }
            })
            .unwrap();
        rx.await.unwrap();
        Ok(())
    }
}

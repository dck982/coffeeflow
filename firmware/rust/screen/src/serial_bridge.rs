//! Pont USB UART0, GPIO43 TX / GPIO44 RX, 115200 bauds.
//! Les FLASH adressés à l'écran restent locaux ; aucune donnée d'image écran
//! ne doit atteindre sensors. Le décodeur COBS recadre après un bruit série.

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use esp_hal::{
    Async,
    uart::{UartRx, UartTx},
};
use protocol::{Frame, MAX_FRAMED_SIZE, StreamDecoder, encode_framed};

/// Réception OTA locale, consommée par le service flash sur le cœur 1.
pub static LOCAL_OTA: Channel<CriticalSectionRawMutex, Frame, 64> = Channel::new();

#[embassy_executor::task]
pub async fn receive(mut rx: UartRx<'static, Async>) -> ! {
    let mut decoder = StreamDecoder::new();
    let mut bytes = [0; 64];
    loop {
        match rx.read_async(&mut bytes).await {
            Ok(n) => {
                for byte in &bytes[..n] {
                    if let Some(frame) = decoder.push_byte(*byte) {
                        if crate::ota_proxy::is_local_ota(&frame) {
                            LOCAL_OTA.send(frame).await;
                        } else {
                            crate::can::BRIDGE_TX.send(frame).await;
                        }
                    }
                }
            }
            Err(_) => embassy_time::Timer::after_millis(20).await,
        }
    }
}

#[embassy_executor::task]
pub async fn transmit(mut tx: UartTx<'static, Async>) -> ! {
    let mut bytes = [0; MAX_FRAMED_SIZE];
    loop {
        let frame = crate::can::SERIAL.receive().await;
        if let Some(n) = encode_framed(&frame, &mut bytes) {
            let mut written = 0;
            while written < n {
                match tx.write_async(&bytes[written..n]).await {
                    Ok(0) | Err(_) => break,
                    Ok(count) => written += count,
                }
            }
        }
    }
}

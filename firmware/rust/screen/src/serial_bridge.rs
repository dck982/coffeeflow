//! Pont USB UART0, GPIO43 TX / GPIO44 RX, 115200 bauds.
//! Les FLASH adressés à l'écran restent locaux ; aucune donnée d'image écran
//! ne doit atteindre sensors. Le décodeur COBS recadre après un bruit série.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use esp_hal::{
    Async,
    uart::{UartRx, UartTx},
};
use protocol::{Frame, MAX_FRAMED_SIZE, StreamDecoder, encode_framed};

/// Réception OTA locale, consommée par le service flash sur le cœur 1.
pub static LOCAL_OTA: Channel<CriticalSectionRawMutex, Frame, 64> = Channel::new();

static FRAMED_TX_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Un client du pont (`coffeetool --usb`) a envoyé une trame valide. Avant,
/// l'UART ne porte que du texte : sur le bureau, la console reste lisible au
/// lieu d'être mêlée aux trames que le cœur émet même sans bus CAN.
static BRIDGE_CLIENT: AtomicBool = AtomicBool::new(false);
static LOGS: Channel<CriticalSectionRawMutex, heapless::Vec<u8, 256>, 16> = Channel::new();
pub static DROPPED_LOGS: AtomicU32 = AtomicU32::new(0);
struct Logger;
static LOGGER: Logger = Logger;
impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if !FRAMED_TX_ACTIVE.load(Relaxed) {
            // Avant le lancement de TX il n'existe encore aucune trame à couper.
            esp_println::println!(
                "[{}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        } else {
            let line =
                crate::serial_log::record(record.level().as_str(), record.target(), *record.args());
            if LOGS.try_send(line).is_err() {
                DROPPED_LOGS.fetch_add(1, Relaxed);
            }
        }
    }
    fn flush(&self) {}
}

/// Installe le journal avant les premiers messages, sans allocation en tas.
/// ESP_LOG peut régler le niveau global au build ; le défaut est INFO.
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER)?;
    let level = option_env!("ESP_LOG")
        .and_then(|s| s.parse().ok())
        .unwrap_or(log::LevelFilter::Info);
    log::set_max_level(level);
    Ok(())
}

#[embassy_executor::task]
pub async fn receive(mut rx: UartRx<'static, Async>) -> ! {
    let mut decoder = StreamDecoder::new();
    let mut bytes = [0; 64];
    loop {
        match rx.read_async(&mut bytes).await {
            Ok(n) => {
                for byte in &bytes[..n] {
                    if let Some(frame) = decoder.push_byte(*byte) {
                        BRIDGE_CLIENT.store(true, Relaxed);
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
    FRAMED_TX_ACTIVE.store(true, Relaxed);
    // Recadrer après les derniers journaux de boot avant le premier PDU.
    let _ = write_all(&mut tx, &[0]).await;
    let mut bytes = [0; MAX_FRAMED_SIZE];
    loop {
        match select(crate::can::SERIAL.receive(), LOGS.receive()).await {
            Either::First(frame) => {
                if !BRIDGE_CLIENT.load(Relaxed) {
                    continue;
                }
                if let Some(n) = encode_framed(&frame, &mut bytes) {
                    let _ = write_all(&mut tx, &bytes[..n]).await;
                }
            }
            Either::Second(line) => {
                let _ = write_all(&mut tx, line.as_slice()).await;
            }
        }
    }
}

async fn write_all(tx: &mut UartTx<'static, Async>, bytes: &[u8]) -> bool {
    let mut written = 0;
    while written < bytes.len() {
        match tx.write_async(&bytes[written..]).await {
            Ok(0) | Err(_) => return false,
            Ok(count) => written += count,
        }
    }
    true
}

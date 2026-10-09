//! TWAI à 500 kbit/s sur GPIO 7 (TX) et 8 (RX), via le module CAN Pal.
//!
//! Une tâche possède le contrôleur. Elle reçoit dans la file de la logique,
//! émet ce que la logique a mis en file (50 ms au plus par trame, une trame
//! non partie est perdue, comme `twai_transmit` avec son délai), relève les
//! compteurs d'erreur et relance le contrôleur s'il passe bus-off.

use core::sync::atomic::{AtomicU8, Ordering::Relaxed};

use embassy_futures::select::{Either3, select3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Timer, with_timeout};
use esp_hal::{Async, Blocking};
use esp_hal::twai::{EspTwaiFrame, Id, StandardId, Twai};
use log::warn;
use protocol::messages::LogPayload;
use protocol::{Dest, Frame, LogCode, LogSeverity, Message, Node};

/// Trames reçues, vers la logique. Les anciennes sont gardées : la file
/// d'esp-hal (32 trames) absorbe déjà les rafales.
pub static RX: Channel<CriticalSectionRawMutex, Frame, 64> = Channel::new();
/// Trames à émettre, depuis la logique.
pub static TX: Channel<CriticalSectionRawMutex, Frame, 32> = Channel::new();

pub static RX_ERRORS: AtomicU8 = AtomicU8::new(0);
pub static TX_ERRORS: AtomicU8 = AtomicU8::new(0);

fn to_frame(frame: &EspTwaiFrame) -> Option<Frame> {
    // Les trames étendues et à distance n'existent pas dans le protocole.
    if frame.is_remote_request() {
        return None;
    }
    let Id::Standard(id) = frame.identifier() else {
        return None;
    };
    Frame::new(id.as_raw(), frame.data())
}

fn to_esp(frame: &Frame) -> Option<EspTwaiFrame> {
    EspTwaiFrame::new(StandardId::new(frame.id)?, frame.payload())
}

/// Repère de `arg32` dans les jalons : « RUST » en ASCII.
const MILESTONE_MARK: u32 = 0x5255_5354;

pub fn milestone_frame(step: u16) -> Frame {
    let payload = LogPayload {
        code: LogCode::Boot as u8,
        severity: LogSeverity::Info as u8,
        arg16: step,
        arg32: MILESTONE_MARK,
    };
    Message::Log(payload).encode_routed(Node::Sensors, Dest::Broadcast)
}

/// Jalon du démarrage : `LOG BOOT`, `arg16` = étape, émis en attente active
/// avant que l'exécuteur ne tourne. Sans console sur la machine, le dernier
/// jalon vu par l'écran situe un blocage du démarrage. 20 ms au plus : sans
/// acquittement sur le bus, la trame précédente occupe encore le tampon et
/// celle-ci est perdue, sans retarder le démarrage.
pub fn milestone(twai: &mut Twai<'static, Blocking>, step: u16) {
    let Some(esp) = to_esp(&milestone_frame(step)) else { return };
    let start = esp_hal::time::Instant::now();
    while twai.transmit(&esp).is_err() {
        if start.elapsed() > esp_hal::time::Duration::from_millis(20) {
            return;
        }
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "clippy compte l'état de la future (trames, délais), qui vit dans l'arène statique de l'exécuteur et non sur la pile"
)]
#[embassy_executor::task]
pub async fn task(mut twai: Twai<'static, Async>) -> ! {
    // Échéance fixe : sur un bus chargé, un délai relancé à chaque trame ne
    // lirait jamais les compteurs.
    let mut next_poll = Instant::now() + Duration::from_millis(100);
    loop {
        match select3(twai.receive_async(), TX.receive(), Timer::at(next_poll)).await {
            Either3::First(Ok(frame)) => {
                if let Some(frame) = to_frame(&frame) {
                    // File pleine : la plus récente est perdue.
                    let _ = RX.try_send(frame);
                }
            }
            Either3::First(Err(err)) => {
                warn!("twai rx: {err:?}");
                // Une erreur persistante (bus-off) ne doit pas faire tourner la
                // boucle à vide.
                Timer::after(Duration::from_millis(10)).await;
            }
            Either3::Second(frame) => {
                if let Some(esp) = to_esp(&frame) {
                    match with_timeout(Duration::from_millis(50), twai.transmit_async(&esp)).await {
                        Ok(Ok(())) => {}
                        Ok(Err(err)) => warn!("twai tx: {err:?}"),
                        Err(_) => warn!("twai tx: délai de 50 ms"),
                    }
                }
            }
            Either3::Third(()) => {
                next_poll = Instant::now() + Duration::from_millis(100);
                RX_ERRORS.store(twai.receive_error_count(), Relaxed);
                TX_ERRORS.store(twai.transmit_error_count(), Relaxed);
                if twai.is_bus_off() {
                    twai.initiate_recovery();
                }
            }
        }
    }
}

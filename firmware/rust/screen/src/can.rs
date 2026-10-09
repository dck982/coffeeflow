//! TWAI à 500 kbit/s sur GPIO19 (TX) et GPIO20 (RX), CAN_SEL haut.
//!
//! Une tâche possède le contrôleur. Elle reçoit dans la file de la logique,
//! émet ce que la logique a mis en file (50 ms au plus par trame, une trame
//! non partie est perdue, comme `twai_transmit` avec son délai), relève les
//! compteurs d'erreur et relance le contrôleur s'il passe bus-off.

use core::sync::atomic::{AtomicU8, Ordering::Relaxed};

use embassy_futures::select::{Either, Either3, select, select3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Timer, with_timeout};
use esp_hal::Async;
use esp_hal::twai::{EspTwaiFrame, Id, StandardId, Twai};

use protocol::Frame;

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

#[allow(
    clippy::large_stack_frames,
    reason = "clippy compte l'état de la future (trames, délais), qui vit dans l'arène statique de l'exécuteur et non sur la pile"
)]
#[embassy_executor::task]
pub async fn task(mut twai: Twai<'static, Async>) -> ! {
    // Échéance fixe : sur un bus chargé, un délai relancé à chaque trame ne
    // lirait jamais les compteurs.
    let mut next_poll = Instant::now() + Duration::from_millis(100);
    let mut recovering = false;
    loop {
        match select3(
            twai.receive_async(),
            select(TX.receive(), BRIDGE_TX.receive()),
            Timer::at(next_poll),
        )
        .await
        {
            Either3::First(Ok(frame)) => {
                if let Some(frame) = to_frame(&frame) {
                    // File pleine : la plus récente est perdue.
                    let _ = RX.try_send(frame);
                    mirror(frame);
                    crate::proxy_service::on_frame(frame);
                }
            }
            Either3::First(Err(err)) => {
                let _ = err;
                // Une erreur persistante (bus-off) ne doit pas faire tourner la
                // boucle à vide.
                Timer::after(Duration::from_millis(10)).await;
            }
            Either3::Second(request) => {
                let (frame, serial_origin) = match request {
                    Either::First(f) => (f, false),
                    Either::Second(f) => (f, true),
                };
                if let Some(esp) = to_esp(&frame) {
                    let transmitted = matches!(
                        with_timeout(Duration::from_millis(50), twai.transmit_async(&esp)).await,
                        Ok(Ok(()))
                    );
                    // Le cœur publie aussi ses tentatives échouées, comme le
                    // firmware archivé ; le pont ne publie qu'une émission réussie.
                    if !serial_origin {
                        mirror(frame);
                    } else if transmitted {
                        let _ = WS.try_send(frame);
                    }
                }
            }
            Either3::Third(()) => {}
        }
        // Même sous trafic continu, ne pas affamer la surveillance bus-off.
        if Instant::now() >= next_poll {
            next_poll = Instant::now() + Duration::from_millis(100);
            RX_ERRORS.store(twai.receive_error_count(), Relaxed);
            TX_ERRORS.store(twai.transmit_error_count(), Relaxed);
            let bus_off = twai.is_bus_off();
            if bus_off && !recovering {
                // Un nouveau basculement reset relancerait le comptage des
                // 128 séquences récessives si la récupération est déjà partie.
                twai.initiate_recovery();
            }
            recovering = bus_off;
        }
    }
}

/// Miroir WebSocket : une file indépendante évite que l'USB vole ses trames.
pub static WS: Channel<CriticalSectionRawMutex, Frame, 128> = Channel::new();
/// Miroir série : une tâche possède seule TX UART0 pour garder chaque PDU entier.
pub static SERIAL: Channel<CriticalSectionRawMutex, Frame, 128> = Channel::new();

fn mirror(frame: Frame) {
    let _ = WS.try_send(frame);
    let _ = SERIAL.try_send(frame);
}

/// Trames venant du pont USB : publiées en WS, sans écho vers USB.
pub static BRIDGE_TX: Channel<CriticalSectionRawMutex, Frame, 32> = Channel::new();

//! Flash d'une image des capteurs par FLASH_CTRL / FLASH_DATA.
//!
//! Séquence :
//!  1. FLASH_CTRL `BEGIN` porte la taille ; les capteurs effacent leur
//!     partition puis acquittent le bloc 0.
//!  2. Pour chaque bloc de 2 Ko : FLASH_CTRL `BLOCK_START` (numéro et CRC16),
//!     puis 256 FLASH_DATA de 8 octets purs.
//!  3. Les capteurs répondent `BLOCK_ACK` avec le numéro et le CRC16 : c'est
//!     le contrôle de flux.
//!  4. Bloc faux : on rejoue le bloc, pas l'image entière.
//!  5. FLASH_CTRL `END` porte le CRC32 global.

use std::time::{Duration, Instant};

use protocol::messages::{FlashCtrlPayload, FlashDataPayload};
use protocol::{Dest, Frame, Message, MessageType, Node, crc16_ccitt, crc32_ieee, split_can_id};

use crate::transport::Transport;

pub const BLOCK_SIZE: usize = 2048;
pub const MAX_RETRIES_PER_BLOCK: u32 = 3;
pub const ACK_TIMEOUT: Duration = Duration::from_secs(5);
/// Pause entre deux FLASH_DATA. Sans elle, le pont série vers CAN de l'écran
/// ne suit pas 256 trames d'affilée : sa file TWAI déborde, des FLASH_DATA se
/// perdent et le bloc reconstruit échoue au CRC16 (constaté sur le vrai bus).
pub const DATA_PACE: Duration = Duration::from_millis(2);

#[derive(Debug, PartialEq, Eq)]
pub struct FlashError(pub String);

impl std::fmt::Display for FlashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FlashError {}

#[derive(Debug, Clone, Copy)]
pub struct FlashProgress {
    pub block_number: usize,
    pub block_count: usize,
    pub bytes_sent: usize,
    pub image_size: usize,
}

pub struct FlashOptions {
    pub src: Node,
    pub dest: Dest,
    pub ack_timeout: Duration,
    pub data_pace: Duration,
}

impl Default for FlashOptions {
    fn default() -> Self {
        Self {
            src: Node::Screen,
            dest: Dest::Sensors,
            ack_timeout: ACK_TIMEOUT,
            data_pace: DATA_PACE,
        }
    }
}

fn fail<T>(message: impl Into<String>) -> Result<T, FlashError> {
    Err(FlashError(message.into()))
}

fn send(
    transport: &mut dyn Transport,
    options: &FlashOptions,
    message: Message,
) -> Result<(), FlashError> {
    let frame = message.encode_routed(options.src, options.dest);
    transport
        .send(&frame)
        .map_err(|e| FlashError(format!("envoi impossible : {e:#}")))
}

/// Attend un FLASH_CTRL en ignorant les autres trames (LOG, PING...) qui
/// transitent entre-temps : le firmware émet par exemple un LOG FLASH_BEGIN
/// juste avant le BLOCK_ACK. Le délai vaut pour l'ensemble, pas par trame
/// ignorée.
fn wait_flash_ctrl(
    transport: &mut dyn Transport,
    timeout: Duration,
) -> Result<Option<FlashCtrlPayload>, FlashError> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        let received = transport
            .recv(remaining)
            .map_err(|e| FlashError(format!("réception impossible : {e:#}")))?;
        let Some((_, frame)) = received else {
            return Ok(None);
        };
        if is_flash_ctrl(&frame)
            && let Ok(ctrl) = FlashCtrlPayload::unpack(frame.payload())
        {
            return Ok(Some(ctrl));
        }
    }
}

fn is_flash_ctrl(frame: &Frame) -> bool {
    split_can_id(frame.id).0 == MessageType::FlashCtrl as u8
}

/// Envoie `image`. Échoue si un bloc est refusé après
/// [`MAX_RETRIES_PER_BLOCK`] essais, ou si l'acquittement final n'arrive pas.
pub fn flash(
    transport: &mut dyn Transport,
    image: &[u8],
    options: &FlashOptions,
    mut on_progress: impl FnMut(FlashProgress),
) -> Result<(), FlashError> {
    let begin = FlashCtrlPayload::Begin {
        image_size: image.len() as u32,
    };
    send(transport, options, Message::FlashCtrl(begin))?;

    match wait_flash_ctrl(transport, options.ack_timeout)? {
        Some(FlashCtrlPayload::BlockAck {
            block_number: 0, ..
        }) => {}
        _ => return fail("pas d'acquittement de BEGIN (effacement de partition)"),
    }

    let block_count = image.len().div_ceil(BLOCK_SIZE);
    for (index, block) in image.chunks(BLOCK_SIZE).enumerate() {
        let expected_crc = crc16_ccitt(block);
        let number = (index + 1) as u16;
        let mut attempt = 1;
        loop {
            let start = FlashCtrlPayload::BlockStart {
                block_number: number,
                block_crc16: expected_crc,
            };
            send(transport, options, Message::FlashCtrl(start))?;
            for chunk in block.chunks(8) {
                let data = FlashDataPayload::new(chunk).expect("morceau de 1 à 8 octets");
                send(transport, options, Message::FlashData(data))?;
                std::thread::sleep(options.data_pace);
            }
            let ack = wait_flash_ctrl(transport, options.ack_timeout)?;
            if ack
                == Some(FlashCtrlPayload::BlockAck {
                    block_number: number,
                    block_crc16: expected_crc,
                })
            {
                break;
            }
            if attempt == MAX_RETRIES_PER_BLOCK {
                return fail(format!("bloc {index} refusé après {attempt} essais"));
            }
            attempt += 1;
        }
        on_progress(FlashProgress {
            block_number: index + 1,
            block_count,
            bytes_sent: ((index + 1) * BLOCK_SIZE).min(image.len()),
            image_size: image.len(),
        });
    }

    let end = FlashCtrlPayload::End {
        image_crc32: crc32_ieee(image),
    };
    send(transport, options, Message::FlashCtrl(end))?;
    match wait_flash_ctrl(transport, options.ack_timeout)? {
        Some(FlashCtrlPayload::End { .. }) => Ok(()),
        _ => fail("pas de confirmation finale — surveiller les LOG (FLASH_DONE / FLASH_FAILED)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::collections::VecDeque;

    /// Capteurs simulés sur le chemin nominal : accusent chaque bloc complet
    /// avec son CRC16 réel. `drop_once` abîme le premier essai du bloc
    /// donné, pour exercer la reprise d'un bloc.
    struct FakeReceiver {
        received: Vec<u8>,
        block_data: Vec<u8>,
        image_size: usize,
        pending: VecDeque<(f64, Frame)>,
        block_starts: Vec<FlashCtrlPayload>,
        sent_frames: usize,
        drop_once: Option<u16>,
        current_block: u16,
        ack_end: bool,
        ack_begin: bool,
    }

    impl FakeReceiver {
        fn new() -> Self {
            Self {
                received: Vec::new(),
                block_data: Vec::new(),
                image_size: 0,
                pending: VecDeque::new(),
                block_starts: Vec::new(),
                sent_frames: 0,
                drop_once: None,
                current_block: 0,
                ack_end: true,
                ack_begin: true,
            }
        }

        fn ack(&mut self, payload: FlashCtrlPayload) {
            let frame = Message::FlashCtrl(payload).encode_routed(Node::Sensors, Dest::Screen);
            self.pending.push_back((1.0, frame));
        }

        fn log_noise(&mut self) {
            let log = Message::Log(protocol::messages::LogPayload::default())
                .encode_routed(Node::Sensors, Dest::Broadcast);
            self.pending.push_back((1.0, log));
        }
    }

    impl Transport for FakeReceiver {
        fn send(&mut self, frame: &Frame) -> Result<()> {
            self.sent_frames += 1;
            let message_type = split_can_id(frame.id).0;
            if message_type == MessageType::FlashCtrl as u8 {
                match FlashCtrlPayload::unpack(frame.payload()).unwrap() {
                    FlashCtrlPayload::Begin { image_size } => {
                        self.image_size = image_size as usize;
                        self.received.clear();
                        if self.ack_begin {
                            self.log_noise();
                            self.ack(FlashCtrlPayload::BlockAck {
                                block_number: 0,
                                block_crc16: 0,
                            });
                        }
                    }
                    FlashCtrlPayload::End { image_crc32 } if self.ack_end => {
                        self.ack(FlashCtrlPayload::End { image_crc32 });
                    }
                    start @ FlashCtrlPayload::BlockStart { block_number, .. } => {
                        self.block_starts.push(start);
                        self.current_block = block_number;
                        self.block_data.clear();
                    }
                    _ => {}
                }
            } else if message_type == MessageType::FlashData as u8 {
                self.block_data.extend_from_slice(frame.payload());
                let total = self.received.len() + self.block_data.len();
                if self.block_data.len() == BLOCK_SIZE || total == self.image_size {
                    let mut crc = crc16_ccitt(&self.block_data);
                    if self.drop_once == Some(self.current_block) {
                        // Bloc abîmé en route : le CRC reçu ne colle pas.
                        self.drop_once = None;
                        crc ^= 1;
                    } else {
                        self.received.extend_from_slice(&self.block_data);
                    }
                    let block_number = self.current_block;
                    self.ack(FlashCtrlPayload::BlockAck {
                        block_number,
                        block_crc16: crc,
                    });
                }
            }
            Ok(())
        }

        fn recv(&mut self, _timeout: Duration) -> Result<Option<(f64, Frame)>> {
            Ok(self.pending.pop_front())
        }
    }

    fn fast() -> FlashOptions {
        FlashOptions {
            ack_timeout: Duration::from_millis(5),
            data_pace: Duration::ZERO,
            ..FlashOptions::default()
        }
    }

    #[test]
    fn small_image_succeeds() {
        let mut receiver = FakeReceiver::new();
        let image: Vec<u8> = (0..=255u8).cycle().take(1024).collect();
        flash(&mut receiver, &image, &fast(), |_| {}).unwrap();
        assert_eq!(receiver.received, image);
        assert_eq!(receiver.block_starts.len(), 1);
        assert_eq!(
            receiver.block_starts[0],
            FlashCtrlPayload::BlockStart {
                block_number: 1,
                block_crc16: crc16_ccitt(&image),
            }
        );
    }

    #[test]
    fn multi_block_image_reports_progress() {
        let mut receiver = FakeReceiver::new();
        let image: Vec<u8> = (0..5000u32).map(|i| (i * 7) as u8).collect();
        let mut progress = Vec::new();
        flash(&mut receiver, &image, &fast(), |p| progress.push(p)).unwrap();
        assert_eq!(receiver.received, image);
        assert_eq!(progress.len(), 3);
        assert_eq!(progress[2].bytes_sent, 5000);
        assert_eq!(progress[0].block_count, 3);
        // 1 BEGIN + 3 BLOCK_START + 625 FLASH_DATA (2048/8*2 + 904/8 arrondi) + END.
        assert_eq!(receiver.sent_frames, 1 + 3 + 256 + 256 + 113 + 1);
    }

    #[test]
    fn uses_the_exact_dlc_of_the_c_firmware() {
        struct Recorder(Vec<Frame>, FakeReceiver);
        impl Transport for Recorder {
            fn send(&mut self, frame: &Frame) -> Result<()> {
                self.0.push(*frame);
                self.1.send(frame)
            }
            fn recv(&mut self, t: Duration) -> Result<Option<(f64, Frame)>> {
                self.1.recv(t)
            }
        }
        let mut transport = Recorder(Vec::new(), FakeReceiver::new());
        flash(&mut transport, &[1, 2, 3], &fast(), |_| {}).unwrap();
        let lens: Vec<u8> = transport.0.iter().map(|f| f.len).collect();
        // BEGIN 8, BLOCK_START 5, FLASH_DATA 3, END 8.
        assert_eq!(lens, vec![8, 5, 3, 8]);
        // écran → capteurs.
        assert_eq!(transport.0[0].id, 0x38 << 5 | 2 << 3 | 1);
        assert_eq!(transport.0[2].id, 0x39 << 5 | 2 << 3 | 1);
    }

    #[test]
    fn a_bad_block_is_replayed_not_the_whole_image() {
        let mut receiver = FakeReceiver::new();
        receiver.drop_once = Some(2);
        let image: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        flash(&mut receiver, &image, &fast(), |_| {}).unwrap();
        assert_eq!(receiver.received, image);
        let starts: Vec<u16> = receiver
            .block_starts
            .iter()
            .map(|s| match s {
                FlashCtrlPayload::BlockStart { block_number, .. } => *block_number,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(starts, vec![1, 2, 2]);
    }

    #[test]
    fn gives_up_without_begin_ack() {
        let mut receiver = FakeReceiver::new();
        receiver.ack_begin = false;
        let error = flash(&mut receiver, &[0; 16], &fast(), |_| {}).unwrap_err();
        assert!(error.0.contains("BEGIN"), "{error}");
    }

    #[test]
    fn gives_up_without_final_confirmation() {
        let mut receiver = FakeReceiver::new();
        receiver.ack_end = false;
        let error = flash(&mut receiver, &[0; 16], &fast(), |_| {}).unwrap_err();
        assert!(error.0.contains("confirmation finale"), "{error}");
    }

    #[test]
    fn a_block_refused_three_times_is_an_error() {
        struct Refuser(FakeReceiver);
        impl Transport for Refuser {
            fn send(&mut self, frame: &Frame) -> Result<()> {
                self.0.send(frame)?;
                // Remplace chaque acquittement de bloc par un CRC faux.
                for (_, f) in self.0.pending.iter_mut() {
                    if let Ok(FlashCtrlPayload::BlockAck {
                        block_number,
                        block_crc16,
                    }) = FlashCtrlPayload::unpack(f.payload())
                        && block_number != 0
                    {
                        *f = Message::FlashCtrl(FlashCtrlPayload::BlockAck {
                            block_number,
                            block_crc16: block_crc16 ^ 1,
                        })
                        .encode_routed(Node::Sensors, Dest::Screen);
                    }
                }
                Ok(())
            }
            fn recv(&mut self, t: Duration) -> Result<Option<(f64, Frame)>> {
                self.0.recv(t)
            }
        }
        let mut transport = Refuser(FakeReceiver::new());
        let error = flash(&mut transport, &[7; 100], &fast(), |_| {}).unwrap_err();
        assert_eq!(error.0, "bloc 0 refusé après 3 essais");
        assert_eq!(transport.0.block_starts.len(), 3);
    }
}

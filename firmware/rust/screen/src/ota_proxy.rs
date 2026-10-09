//! Format staging et acquittements OTA, indépendants du matériel.
//! Le premier secteur est réservé à un en-tête marqué valide seulement après
//! réception complète. Les capteurs acquittent chaque bloc de 2048 octets.

use protocol::messages::FlashCtrlPayload;
use protocol::{Dest, Frame, Message, MessageType, Node, decode_can_id};

pub const HEADER_BYTES: u32 = 4096;
pub const BLOCK_BYTES: usize = 2048;
pub const ACK_TIMEOUT_US: u64 = 5_000_000;
pub const MAX_IMAGE_BYTES: u32 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Image {
    pub size: u32,
    pub crc32: u32,
}

impl Image {
    pub fn header(self) -> [u8; 16] {
        let mut bytes = [0; 16];
        for (chunk, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip([
            0x4f54_4153u32,
            self.size,
            self.crc32,
            0xa55a_a55a,
        ]) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }
    pub fn from_header(bytes: &[u8; 16]) -> Option<Self> {
        let word = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        (word(0) == 0x4f54_4153
            && word(12) == 0xa55a_a55a
            && word(4) > 0
            && word(4) <= MAX_IMAGE_BYTES)
            .then(|| Self {
                size: word(4),
                crc32: word(8),
            })
    }
}

/// FLASH pour l'écran reste local au pont, quelle que soit l'identité source.
pub fn is_local_ota(frame: &Frame) -> bool {
    decode_can_id(frame.id).is_ok_and(|id| {
        id.dest == Dest::Screen
            && matches!(
                id.message_type,
                MessageType::FlashCtrl | MessageType::FlashData
            )
    })
}

/// Contrôle OTA reçu uniquement depuis sensors et adressé à screen/broadcast.
pub fn acknowledgement(frame: &Frame) -> Option<FlashCtrlPayload> {
    let id = decode_can_id(frame.id).ok()?;
    if id.src != Node::Sensors
        || !matches!(id.dest, Dest::Screen | Dest::Broadcast)
        || id.message_type != MessageType::FlashCtrl
    {
        return None;
    }
    FlashCtrlPayload::unpack(frame.payload()).ok()
}

/// CRC de BEGIN ignoré comme le récepteur historique ; CRC des blocs obligatoire.
pub fn matches_ack(payload: FlashCtrlPayload, number: u16, crc: u16) -> bool {
    matches!(payload, FlashCtrlPayload::BlockAck { block_number, block_crc16 }
        if block_number == number && (number == 0 || block_crc16 == crc))
}

pub fn ctrl(payload: FlashCtrlPayload) -> Frame {
    Message::FlashCtrl(payload).encode_routed(Node::Screen, Dest::Sensors)
}

/// Une échéance fixe : des ACK obsolètes ne prolongent jamais les cinq secondes.
#[derive(Clone, Copy)]
pub enum ExpectedAck {
    Block { number: u16, crc: u16 },
    End,
}
pub struct AckWait {
    pub deadline_us: u64,
    expected: ExpectedAck,
}
#[derive(Debug, PartialEq, Eq)]
pub enum AckResult {
    Pending,
    Accepted,
    Failed,
}
impl AckWait {
    pub fn new(now_us: u64, expected: ExpectedAck) -> Self {
        Self {
            deadline_us: now_us.saturating_add(ACK_TIMEOUT_US),
            expected,
        }
    }
    pub fn check(&self, now_us: u64, received: Option<FlashCtrlPayload>) -> AckResult {
        if now_us >= self.deadline_us || matches!(received, Some(FlashCtrlPayload::Abort)) {
            return AckResult::Failed;
        }
        let matched = match (self.expected, received) {
            (ExpectedAck::Block { number, crc }, Some(p)) => matches_ack(p, number, crc),
            (ExpectedAck::End, Some(FlashCtrlPayload::End { .. })) => true,
            _ => false,
        };
        if matched {
            AckResult::Accepted
        } else {
            AckResult::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{CanId, encode_can_id};
    #[test]
    fn stale_ack_does_not_extend_deadline_and_abort_fails_immediately() {
        let wait = AckWait::new(100, ExpectedAck::Block { number: 2, crc: 4 });
        let stale = Some(FlashCtrlPayload::BlockAck {
            block_number: 1,
            block_crc16: 4,
        });
        assert_eq!(
            wait.check(100 + ACK_TIMEOUT_US - 1, stale),
            AckResult::Pending
        );
        let valid = Some(FlashCtrlPayload::BlockAck {
            block_number: 2,
            block_crc16: 4,
        });
        assert_eq!(wait.check(100 + ACK_TIMEOUT_US, valid), AckResult::Failed);
        assert_eq!(
            wait.check(101, Some(FlashCtrlPayload::Abort)),
            AckResult::Failed
        );
        assert_eq!(wait.check(101, valid), AckResult::Accepted);
        let end = AckWait::new(100, ExpectedAck::End);
        assert_eq!(end.check(101, valid), AckResult::Pending);
        assert_eq!(
            end.check(101, Some(FlashCtrlPayload::End { image_crc32: 0 })),
            AckResult::Accepted
        );
    }
    #[test]
    fn staging_header_matches_cpp_and_rejects_partial_commit() {
        let image = Image {
            size: 2057,
            crc32: 0x12345678,
        };
        let mut header = image.header();
        assert_eq!(&header[..4], b"SATO");
        assert_eq!(Image::from_header(&header), Some(image));
        header[12..].fill(0xff);
        assert_eq!(Image::from_header(&header), None);
    }
    #[test]
    fn begin_ignores_crc_but_blocks_require_number_and_crc() {
        let ack = |n, c| FlashCtrlPayload::BlockAck {
            block_number: n,
            block_crc16: c,
        };
        assert!(matches_ack(ack(0, 123), 0, 0));
        assert!(matches_ack(ack(2, 123), 2, 123));
        assert!(!matches_ack(ack(2, 124), 2, 123));
        assert!(!matches_ack(ack(1, 123), 2, 123));
    }
    #[test]
    fn only_local_flash_intercepted_and_only_sensors_acks_accepted() {
        let id = |dest, src| {
            encode_can_id(CanId {
                message_type: MessageType::FlashCtrl,
                dest,
                src,
            })
        };
        let payload = FlashCtrlPayload::BlockAck {
            block_number: 3,
            block_crc16: 12,
        };
        let mut frame = Frame::new(id(Dest::Screen, Node::Sensors), &payload.pack()).unwrap();
        assert!(is_local_ota(&frame));
        assert_eq!(acknowledgement(&frame), Some(payload));
        frame.id = id(Dest::Screen, Node::Screen);
        assert_eq!(acknowledgement(&frame), None);
        frame.id = id(Dest::Sensors, Node::Screen);
        assert!(!is_local_ota(&frame));
    }
}

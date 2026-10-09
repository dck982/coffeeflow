//! WebSocket en miroir CAN : handshake RFC6455 et trames binaires émises.
//! Aucun paquet reçu du client n'est converti en commande CAN.
use alloc::string::String;
use base64::{Engine, engine::general_purpose::STANDARD};
use sha1::{Digest, Sha1};

/// Accepte uniquement une clé de handshake qui encode seize octets.
pub fn accept_key(key: &str) -> Option<String> {
    if STANDARD.decode(key).ok()?.len() != 16 {
        return None;
    }
    let mut hash = Sha1::new();
    hash.update(key.as_bytes());
    hash.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    Some(STANDARD.encode(hash.finalize()))
}

/// En-tête d'une trame serveur binaire complète, sans masque.
pub fn binary_header(len: usize) -> ([u8; 10], usize) {
    let mut out = [0; 10];
    out[0] = 0x82;
    if len < 126 {
        out[1] = len as u8;
        (out, 2)
    } else if len <= u16::MAX as usize {
        out[1] = 126;
        out[2..4].copy_from_slice(&(len as u16).to_be_bytes());
        (out, 4)
    } else {
        out[1] = 127;
        out[2..10].copy_from_slice(&(len as u64).to_be_bytes());
        (out, 10)
    }
}

/// Lecture bornée d'une trame client masquée. Le payload reste en mémoire
/// locale et ne devient jamais une commande CAN.
#[derive(Debug, PartialEq, Eq)]
pub enum ClientError {
    Incomplete,
    Invalid,
}
pub struct ClientFrame {
    pub opcode: u8,
    pub payload: [u8; 125],
    pub length: usize,
    pub consumed: usize,
}
impl ClientFrame {
    pub fn parse(bytes: &[u8]) -> Result<Self, ClientError> {
        if bytes.len() < 2 {
            return Err(ClientError::Incomplete);
        }
        let opcode = bytes[0] & 15;
        if bytes[0] & 0xf0 != 0x80 || bytes[1] & 0x80 == 0 || !matches!(opcode, 1 | 2 | 8 | 9 | 10)
        {
            return Err(ClientError::Invalid);
        }
        let length = usize::from(bytes[1] & 127);
        if length > 125 || (opcode < 8 && length > protocol::MAX_PDU_SIZE) {
            return Err(ClientError::Invalid);
        }
        let consumed = length + 6;
        if bytes.len() < consumed {
            return Err(ClientError::Incomplete);
        }
        let mut payload = [0; 125];
        for (i, byte) in payload[..length].iter_mut().enumerate() {
            *byte = bytes[6 + i] ^ bytes[2 + i % 4];
        }
        Ok(Self {
            opcode,
            payload,
            length,
            consumed,
        })
    }
}

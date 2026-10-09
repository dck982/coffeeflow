//! Cadrage série (USB CDC). Le WebSocket transporte le PDU tel quel, un
//! message = une trame ; seul le fil série a besoin d'un cadrage, parce
//! qu'un flux d'octets n'a pas de bordure.
//!
//! PDU (identique sur les deux transports), petit-boutiste :
//!
//! ```text
//! [0..1]  id     u16 : bits 10..0 = identifiant CAN, bits 15..11 = 0
//! [2]     dlc    0..8
//! [3..]   data   dlc octets
//! [fin]   crc16  u16, CRC-16-CCITT (poly 0x1021, init 0xFFFF) sur id..data
//! ```
//!
//! Sur le fil série : COBS(PDU) puis un octet 0x00 de fin de trame. COBS
//! élimine tous les zéros du PDU, donc une image OTA pleine de zéros ne pose
//! pas de problème de resynchronisation ; le premier 0x00 rencontré recadre
//! après un câble mal branché ou un octet perdu.

use crate::crc::crc16_ccitt;
use crate::frame::Frame;

/// id (2) + dlc (1) + données (8) + crc16 (2).
pub const MAX_PDU_SIZE: usize = 2 + 1 + 8 + 2;
pub const MAX_COBS_SIZE: usize = MAX_PDU_SIZE + (MAX_PDU_SIZE / 254) + 1;
/// COBS + l'octet 0x00 de fin de trame.
pub const MAX_FRAMED_SIZE: usize = MAX_COBS_SIZE + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramingError {
    /// Trame hors gabarit à l'encodage (id > 0x7FF ou DLC > 8), ou longueur
    /// du PDU incohérente avec le DLC qu'il contient au décodage.
    Length,
    /// CRC16 du PDU erroné.
    Crc,
    /// Identifiant au-delà de 11 bits.
    Id,
}

impl core::fmt::Display for FramingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Length => "longueur de PDU incohérente",
            Self::Crc => "CRC16 du PDU erroné",
            Self::Id => "identifiant CAN au-delà de 11 bits",
        })
    }
}

impl core::error::Error for FramingError {}

/// Sérialise `frame` en PDU (id, dlc, données, crc16) et renvoie la longueur
/// écrite. Refuse un gabarit hors norme (`id > 0x7FF`, `len > 8`).
pub fn encode_pdu(frame: &Frame, out: &mut [u8; MAX_PDU_SIZE]) -> Result<usize, FramingError> {
    if frame.len > 8 {
        return Err(FramingError::Length);
    }
    if frame.id > 0x7FF {
        return Err(FramingError::Id);
    }
    let dlc = usize::from(frame.len);
    out[0] = (frame.id & 0xFF) as u8;
    out[1] = (frame.id >> 8) as u8;
    out[2] = frame.len;
    out[3..3 + dlc].copy_from_slice(&frame.data[..dlc]);
    let body_len = 3 + dlc;
    let crc = crc16_ccitt(&out[..body_len]);
    out[body_len] = (crc & 0xFF) as u8;
    out[body_len + 1] = (crc >> 8) as u8;
    Ok(body_len + 2)
}

/// Décode un PDU de longueur exacte. Refuse une longueur incohérente avec le
/// DLC contenu, un CRC erroné ou un identifiant au-delà de 11 bits.
pub fn decode_pdu(input: &[u8]) -> Result<Frame, FramingError> {
    // id (2) + dlc (1) + crc16 (2), DLC 0 au plancher.
    if input.len() < 5 {
        return Err(FramingError::Length);
    }
    let dlc = usize::from(input[2]);
    if dlc > 8 || input.len() != 3 + dlc + 2 {
        return Err(FramingError::Length);
    }
    let crc = crc16_ccitt(&input[..3 + dlc]);
    let got = u16::from_le_bytes([input[3 + dlc], input[3 + dlc + 1]]);
    if crc != got {
        return Err(FramingError::Crc);
    }
    let id = u16::from_le_bytes([input[0], input[1]]);
    if id > 0x7FF {
        return Err(FramingError::Id);
    }
    let mut data = [0u8; 8];
    data[..dlc].copy_from_slice(&input[3..3 + dlc]);
    Ok(Frame {
        id,
        len: dlc as u8,
        data,
    })
}

/// COBS (Cheshire & Baker, « Consistent Overhead Byte Stuffing »). Écrit
/// dans `out` et renvoie la longueur ; `None` si `out` est trop petit
/// (`input.len() + input.len() / 254 + 1` octets suffisent). N'ajoute pas le
/// 0x00 de fin de trame.
pub fn cobs_encode(input: &[u8], out: &mut [u8]) -> Option<usize> {
    if out.is_empty() {
        return None;
    }
    let mut write_index = 1;
    let mut code_index = 0;
    let mut code: u8 = 1;
    for &byte in input {
        if byte == 0 {
            out[code_index] = code;
            code = 1;
            code_index = write_index;
            write_index += 1;
        } else {
            *out.get_mut(write_index)? = byte;
            write_index += 1;
            code += 1;
            if code == 0xFF {
                out[code_index] = code;
                code = 1;
                code_index = write_index;
                write_index += 1;
            }
        }
        // La case du prochain code doit exister, même si l'entrée s'arrête.
        if write_index > out.len() {
            return None;
        }
    }
    out[code_index] = code;
    Some(write_index)
}

/// COBS inverse. `None` si l'entrée est mal formée (zéro dans le flux, bloc
/// qui dépasse) ou si `out` est trop petit. Une entrée vide donne `Some(0)` :
/// le C++ renvoie 0 dans les deux cas, et aucun PDU valide n'a une longueur
/// nulle (le plus court, STOP, fait 5 octets).
pub fn cobs_decode(input: &[u8], out: &mut [u8]) -> Option<usize> {
    let len = input.len();
    let mut read_index = 0;
    let mut write_index = 0;
    while read_index < len {
        let code = input[read_index];
        if code == 0 {
            // Un 0x00 ne peut apparaître qu'en fin de trame.
            return None;
        }
        if read_index + usize::from(code) > len && code != 1 {
            return None;
        }
        read_index += 1;
        for _ in 1..code {
            if read_index >= len {
                return None;
            }
            *out.get_mut(write_index)? = input[read_index];
            write_index += 1;
            read_index += 1;
        }
        if code != 0xFF && read_index != len {
            *out.get_mut(write_index)? = 0;
            write_index += 1;
        }
    }
    Some(write_index)
}

/// Assemble le cadrage complet COBS(PDU(frame)) + 0x00 et renvoie la longueur
/// écrite. `None` si `frame` est hors gabarit ; rien n'est écrit alors.
pub fn encode_framed(frame: &Frame, out: &mut [u8; MAX_FRAMED_SIZE]) -> Option<usize> {
    let mut pdu = [0u8; MAX_PDU_SIZE];
    let pdu_len = encode_pdu(frame, &mut pdu).ok()?;
    let cobs_len = cobs_encode(&pdu[..pdu_len], &mut out[..MAX_COBS_SIZE])?;
    out[cobs_len] = 0x00;
    Some(cobs_len + 1)
}

/// Décodeur incrémental pour un flux d'octets sans bordure (USB CDC). On lui
/// pousse les octets reçus au fil de l'eau ; il rend chaque trame valide
/// trouvée entre deux 0x00 et compte dans `dropped_count` tout ce qui ne
/// décode pas. Un octet perdu ne bloque jamais le flux, il coûte seulement
/// la trame en cours.
#[derive(Debug, Clone)]
pub struct StreamDecoder {
    buffer: [u8; MAX_COBS_SIZE],
    len: usize,
    pub dropped_count: u32,
}

impl StreamDecoder {
    pub const fn new() -> Self {
        Self {
            buffer: [0; MAX_COBS_SIZE],
            len: 0,
            dropped_count: 0,
        }
    }

    pub fn push_byte(&mut self, byte: u8) -> Option<Frame> {
        if byte == 0x00 {
            if self.len == 0 {
                // 0x00 double, keepalive optionnel : rien à faire.
                return None;
            }
            let mut pdu = [0u8; MAX_PDU_SIZE];
            let decoded = cobs_decode(&self.buffer[..self.len], &mut pdu)
                .filter(|&n| n > 0)
                .and_then(|n| decode_pdu(&pdu[..n]).ok());
            self.len = 0;
            if decoded.is_none() {
                self.dropped_count = self.dropped_count.wrapping_add(1);
            }
            return decoded;
        }
        if self.len >= self.buffer.len() {
            // Trame plus longue que MAX_COBS_SIZE avant un 0x00 : forcément
            // corrompue ou hors gabarit. On l'abandonne et on se resynchronise
            // au prochain 0x00.
            self.dropped_count = self.dropped_count.wrapping_add(1);
            self.len = 0;
            return None;
        }
        self.buffer[self.len] = byte;
        self.len += 1;
        None
    }
}

impl Default for StreamDecoder {
    fn default() -> Self {
        Self::new()
    }
}

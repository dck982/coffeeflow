//! Trame CAN brute, telle qu'elle circule sur le bus, le WebSocket et le
//! cadrage série.

use crate::id::{CanId, CanIdError, decode_can_id};

/// Trame CAN : identifiant 11 bits, DLC et 8 octets de données (les octets
/// au-delà du DLC valent zéro).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frame {
    /// Bits 10..0 : identifiant CAN ; les autres sont à zéro.
    pub id: u16,
    /// DLC, 0 à 8.
    pub len: u8,
    pub data: [u8; 8],
}

impl Frame {
    /// Construit une trame à partir des octets utiles. `None` si plus de 8
    /// octets.
    pub fn new(id: u16, payload: &[u8]) -> Option<Self> {
        if payload.len() > 8 {
            return None;
        }
        let mut data = [0u8; 8];
        data[..payload.len()].copy_from_slice(payload);
        Some(Self {
            id,
            len: payload.len() as u8,
            data,
        })
    }

    /// Les `len` octets utiles (au plus 8, même si `len` est incohérent).
    pub fn payload(&self) -> &[u8] {
        &self.data[..usize::from(self.len).min(8)]
    }

    /// Identifiant décomposé ; erreur si un champ n'a pas de valeur connue.
    pub const fn can_id(&self) -> Result<CanId, CanIdError> {
        decode_can_id(self.id)
    }
}

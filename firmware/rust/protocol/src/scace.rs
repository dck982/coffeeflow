//! Service BLE de la sonde SCACE : le Core2 est périphérique GATT et notifie
//! une trame par mesure, à 10 Hz. Lu par l'écran et par les outils Mac, qui
//! reprennent ces UUID et ce format.

pub const DEVICE_NAME: &str = "SCACE";
pub const SERVICE_UUID: &str = "5017120a-7fa7-46d8-a22e-61b24b1201fe";
pub const FRAME_UUID: &str = "84fef574-1171-4c9a-b221-6f17aea79a38";

/// Mêmes UUID, octets dans l'ordre de l'air (petit-boutiste).
pub const SERVICE_UUID_LE: [u8; 16] = [
    0xfe, 0x01, 0x12, 0x4b, 0xb2, 0x61, 0x2e, 0xa2, 0xd8, 0x46, 0xa7, 0x7f, 0x0a, 0x12, 0x17, 0x50,
];
pub const FRAME_UUID_LE: [u8; 16] = [
    0x38, 0x9a, 0xa7, 0xae, 0x17, 0x6f, 0x21, 0xb2, 0x9a, 0x4c, 0x71, 0x11, 0x74, 0xf5, 0xfe, 0x84,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Status {
    /// Pont hors tension, ADS1115 introuvable ou erreur I2C. Envoyé une fois
    /// par seconde, codes à 0.
    #[default]
    NoMeasurement = 0,
    Ok = 1,
    Open = 2,
    Shorted = 3,
}

impl Status {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::NoMeasurement),
            1 => Some(Self::Ok),
            2 => Some(Self::Open),
            3 => Some(Self::Shorted),
            _ => None,
        }
    }
}

/// Température absente (NTC ouverte, court-circuit, pas de mesure).
pub const NO_TEMPERATURE: i16 = i16::MIN;

/// Taille de la trame : elle tient dans le MTU par défaut de 23 octets
/// (20 octets de valeur).
pub const FRAME_SIZE: usize = 12;

/// Trame de 12 octets, petit-boutiste.
///
/// ```text
///  0  u16  numéro de séquence, incrémenté à chaque trame
///  2  u16  temps sonde en ms modulo 65 536, à la fin de la conversion A1
///  4  i16  code A0 (rail)
///  6  i16  code A1 (point milieu)
///  8  i16  température en centièmes de °C, constantes de la sonde
/// 10  u8   état
/// 11  u8   réservé, 0
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frame {
    pub seq: u16,
    pub ms: u16,
    pub a0: i16,
    pub a1: i16,
    pub centi_c: i16,
    pub status: Status,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            seq: 0,
            ms: 0,
            a0: 0,
            a1: 0,
            centi_c: NO_TEMPERATURE,
            status: Status::NoMeasurement,
        }
    }
}

impl Frame {
    pub fn encode(&self) -> [u8; FRAME_SIZE] {
        let mut out = [0u8; FRAME_SIZE];
        out[0..2].copy_from_slice(&self.seq.to_le_bytes());
        out[2..4].copy_from_slice(&self.ms.to_le_bytes());
        out[4..6].copy_from_slice(&self.a0.to_le_bytes());
        out[6..8].copy_from_slice(&self.a1.to_le_bytes());
        out[8..10].copy_from_slice(&self.centi_c.to_le_bytes());
        out[10] = self.status as u8;
        out
    }

    /// Refuse une trame trop courte ou un état inconnu. Les octets au-delà
    /// de 12 sont ignorés, pour qu'une trame allongée reste lisible.
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < FRAME_SIZE {
            return None;
        }
        let status = Status::from_u8(data[10])?;
        let get16 = |at: usize| [data[at], data[at + 1]];
        Some(Self {
            seq: u16::from_le_bytes(get16(0)),
            ms: u16::from_le_bytes(get16(2)),
            a0: i16::from_le_bytes(get16(4)),
            a1: i16::from_le_bytes(get16(6)),
            centi_c: i16::from_le_bytes(get16(8)),
            status,
        })
    }
}

//! Sonde SCACE (BLE, 10 Hz) : filtrage des annonces, lecture d'une trame et
//! règle de fraîcheur. Le format de la trame est dans `protocol::scace`.

use protocol::scace::{DEVICE_NAME, FRAME_SIZE, NO_TEMPERATURE, SERVICE_UUID_LE, Status};

/// La sonde notifie à 10 Hz : trois trames manquées la rendent invalide.
pub const FRESH_MS: i64 = 300;

/// Mesure d'une notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Centièmes de °C ; significatif seulement si `ok`.
    pub centi_c: i16,
    /// Faux pour une trame sans température (pont hors tension, NTC ouverte
    /// ou en court-circuit).
    pub ok: bool,
}

/// La sonde se reconnaît à son service ou, à défaut, à son nom exact.
pub fn matches(uuids128_le: &[[u8; 16]], name: Option<&[u8]>) -> bool {
    if uuids128_le.contains(&SERVICE_UUID_LE) {
        return true;
    }
    name.is_some_and(|n| n == DEVICE_NAME.as_bytes())
}

/// Décode une notification. Les octets au-delà de la trame sont ignorés : une
/// sonde plus récente qui l'allonge reste lisible.
pub fn parse_notification(data: &[u8]) -> Option<Sample> {
    if data.len() < FRAME_SIZE {
        return None;
    }
    let frame = protocol::scace::Frame::decode(data)?;
    Some(Sample {
        centi_c: frame.centi_c,
        ok: frame.status == Status::Ok && frame.centi_c != NO_TEMPERATURE,
    })
}

/// Connectée et mesure correcte reçue depuis moins de 300 ms.
pub fn is_valid(connected: bool, received_us: i64, now_us: i64) -> bool {
    connected && received_us != 0 && now_us - received_us <= FRESH_MS * 1000
}

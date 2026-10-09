//! Requête SNTP v4 et validation des réponses, avec cookie d'origine.
//! L'heure reçue ne change jamais l'horloge monotone du cœur.
const UNIX_OFFSET: u64 = 2_208_988_800;

pub fn request(cookie: [u8; 8]) -> [u8; 48] {
    let mut out = [0; 48];
    out[0] = 0x23;
    out[40..48].copy_from_slice(&cookie);
    out
}

/// Heure Unix en secondes. Les réponses non synchronisées, les KoD et les
/// paquets d'une autre requête sont ignorés. La seconde 2036 appartient à l'ère 1.
pub fn response(packet: &[u8], cookie: [u8; 8]) -> Option<i64> {
    if packet.len() < 48 || packet[0] >> 6 == 3 || packet[0] & 7 != 4
        || !matches!((packet[0] >> 3) & 7, 3 | 4) || !(1..=15).contains(&packet[1])
        || packet[24..32] != cookie { return None; }
    let seconds = u32::from_be_bytes(packet[40..44].try_into().ok()?) as u64;
    if packet[40..48].iter().all(|b| *b == 0) { return None; }
    let seconds = if seconds < UNIX_OFFSET { seconds + (1u64 << 32) } else { seconds };
    Some((seconds - UNIX_OFFSET) as i64)
}

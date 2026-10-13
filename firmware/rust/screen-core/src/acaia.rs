//! Protocole BLE de la balance Acaia Lunar : trames émises, décodage des
//! notifications (poids), filtrage des annonces et cadence de publication.
//! Pas de pile BLE ici : la tâche BLE de l'écran transmet les octets reçus et
//! écrit ceux qu'on lui rend.

use alloc::vec::Vec;

/// Intervalle entre deux trames de maintien de connexion.
pub const HEARTBEAT_MS: i64 = 2500;
/// Au repos, un poids par seconde suffit.
pub const IDLE_PUBLISH_MS: i64 = 1000;
const RX_CAPACITY: usize = 128;

// UUID, octets dans l'ordre de l'air (petit-boutiste).
// Lunar récentes : canaux d'écriture et de notification distincts.
pub const MODERN_SERVICE_UUID_LE: [u8; 16] = [
    0x55, 0xE4, 0x05, 0xD2, 0xAF, 0x9F, 0xA9, 0x8F, 0xE5, 0x4A, 0x7D, 0xFE, 0x43, 0x53, 0x53, 0x49,
];
pub const MODERN_WRITE_UUID_LE: [u8; 16] = [
    0xB3, 0x9B, 0x72, 0x34, 0xBE, 0xEC, 0xD4, 0xA8, 0xF4, 0x43, 0x41, 0x88, 0x43, 0x53, 0x53, 0x49,
];
pub const MODERN_NOTIFY_UUID_LE: [u8; 16] = [
    0x16, 0x96, 0x24, 0x47, 0xC6, 0x23, 0x61, 0xBA, 0xD9, 0x4B, 0x4D, 0x1E, 0x43, 0x53, 0x53, 0x49,
];
/// Lunar « legacy », confirmée sur ACAIAL-C1EF4C9 le 2026-09-11.
pub const LEGACY_SERVICE_UUID16: u16 = 0x1820;
pub const LEGACY_CHARACTERISTIC_UUID16: u16 = 0x2A80;

/// Annonce BLE reçue : seuls les champs utiles au filtrage.
#[derive(Debug, Clone, Copy, Default)]
pub struct Advertisement<'a> {
    pub uuids128_le: &'a [[u8; 16]],
    pub uuids16: &'a [u16],
    pub name: Option<&'a [u8]>,
}

/// La Lunar annonce normalement ce service. Le nom est un repli utile pour
/// les versions qui ne mettent l'UUID que dans la scan response.
pub fn is_acaia_advertisement(adv: &Advertisement<'_>) -> bool {
    if adv.uuids128_le.contains(&MODERN_SERVICE_UUID_LE) {
        return true;
    }
    if adv.uuids16.contains(&LEGACY_SERVICE_UUID16) {
        return true;
    }
    let Some(name) = adv.name else {
        return false;
    };
    // 32 octets au plus, comme le tampon du C++.
    let name = &name[..name.len().min(32)];
    let upper: Vec<u8> = name.iter().map(u8::to_ascii_uppercase).collect();
    contains(&upper, b"ACAIA") || contains(&upper, b"LUNAR")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Trame d'écriture : `EF DD cmd [len+1] payload even odd`. `None` si elle
/// dépasse 64 octets.
pub fn encode_frame(cmd: u8, payload: &[u8], length_prefix: bool) -> Option<Vec<u8>> {
    if payload.len() + if length_prefix { 6 } else { 5 } > 64 {
        return None;
    }
    let mut frame: Vec<u8> = Vec::with_capacity(payload.len() + 6);
    frame.extend_from_slice(&[0xEF, 0xDD, cmd]);
    if length_prefix {
        frame.push((payload.len() + 1) as u8);
    }
    frame.extend_from_slice(payload);
    let (mut even, mut odd) = (0u8, 0u8);
    for (i, &b) in frame.iter().enumerate().skip(3) {
        if (i - 3) % 2 == 0 {
            even = even.wrapping_add(b);
        } else {
            odd = odd.wrapping_add(b);
        }
    }
    frame.push(even);
    frame.push(odd);
    Some(frame)
}

/// Les trois trames de l'abonnement et du maintien : identification,
/// heartbeat, demande de notifications de poids.
pub fn subscribe_frames() -> [Vec<u8>; 3] {
    const ID: [u8; 15] = [
        0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34,
    ];
    const HEARTBEAT: [u8; 1] = [0];
    const REQUEST: [u8; 8] = [0x00, 0x01, 0x01, 0x02, 0x02, 0x05, 0x03, 0x04];
    [
        encode_frame(0x0B, &ID, false).unwrap_or_default(),
        encode_frame(0x00, &HEARTBEAT, true).unwrap_or_default(),
        encode_frame(0x0C, &REQUEST, true).unwrap_or_default(),
    ]
}

/// Commande TARE.
pub fn tare_frame() -> Vec<u8> {
    encode_frame(0x0F, &[0], true).unwrap_or_default()
}

/// Poids d'une sous-trame de type 5 (6 octets : entier 32 bits, puissance de
/// dix, signe).
///
/// # Panics
///
/// Le tampon doit contenir au moins six octets. [`Session`] vérifie cette
/// longueur avant tout décodage de notification.
pub fn decode_weight(data: &[u8]) -> f32 {
    let raw = u32::from(data[0])
        | (u32::from(data[1]) << 8)
        | (u32::from(data[2]) << 16)
        | (u32::from(data[3]) << 24);
    let mut weight = raw as f32;
    match data[4] {
        1 => weight /= 10.0,
        2 => weight /= 100.0,
        3 => weight /= 1000.0,
        4 => weight /= 10000.0,
        _ => {}
    }
    // Bit de signe : la Lunar réelle affiche bien un poids négatif quand on
    // retire la tasse après une tare.
    if data[5] & 0x02 != 0 { -weight } else { weight }
}

/// Ce qu'une notification apporte au cœur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RxEvent {
    /// Poids publié (cadence limitée au repos).
    Weight(f32),
    /// La balance annonce sa déconnexion (commande 0x20).
    Disconnected,
}

/// Session avec une balance : réassemblage des trames reçues, cadence de
/// publication et de maintien.
#[derive(Debug, Clone)]
pub struct Session {
    rx: [u8; RX_CAPACITY],
    rx_len: usize,
    /// Pendant un cycle, chaque pesée est publiée.
    active: bool,
    last_publish_us: i64,
    last_heartbeat_us: i64,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub const fn new() -> Self {
        Self {
            rx: [0; RX_CAPACITY],
            rx_len: 0,
            active: false,
            last_publish_us: 0,
            last_heartbeat_us: 0,
        }
    }

    /// Publie chaque pesée (cycle en cours) plutôt qu'une par seconde.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Nouvelle connexion ou connexion perdue : repart d'un tampon vide.
    pub fn clear_connection(&mut self) {
        self.rx_len = 0;
    }

    /// Pile BLE réinitialisée ou arrêtée.
    pub fn reset(&mut self) {
        self.active = false;
        self.clear_connection();
    }

    /// Trames à écrire si le maintien est dû. `ready` : abonnement actif ; le
    /// minuteur avance même sinon, comme le C++.
    pub fn heartbeat(&mut self, now_us: i64, ready: bool) -> Vec<Vec<u8>> {
        if now_us - self.last_heartbeat_us < HEARTBEAT_MS * 1000 {
            return Vec::new();
        }
        self.last_heartbeat_us = now_us;
        if ready {
            subscribe_frames().into()
        } else {
            Vec::new()
        }
    }

    /// Octets d'une notification GATT.
    pub fn on_notification(&mut self, data: &[u8], now_us: i64) -> Vec<RxEvent> {
        let mut events = Vec::new();
        let length = data.len();
        if length > RX_CAPACITY {
            return events;
        }
        if self.rx_len + length > RX_CAPACITY {
            self.rx_len = 0;
        }
        self.rx[self.rx_len..self.rx_len + length].copy_from_slice(data);
        self.rx_len += length;
        while self.rx_len >= 3 {
            if self.rx[0] != 0xEF || self.rx[1] != 0xDD {
                self.rx.copy_within(1..self.rx_len, 0);
                self.rx_len -= 1;
                continue;
            }
            if self.rx[2] == 0x20 {
                self.clear_connection();
                events.push(RxEvent::Disconnected);
                return events;
            }
            if self.rx_len < 6 {
                return events;
            }
            let payload_len = usize::from(self.rx[3]);
            let frame_len = payload_len + 5;
            if payload_len == 0 || frame_len > RX_CAPACITY {
                self.rx_len = 0;
                return events;
            }
            if self.rx_len < frame_len {
                return events;
            }
            if self.rx[2] == 12 {
                let payload: Vec<u8> = self.rx[4..4 + payload_len].to_vec();
                self.process_notification(&payload, now_us, &mut events);
            }
            self.rx.copy_within(frame_len..self.rx_len, 0);
            self.rx_len -= frame_len;
        }
        events
    }

    fn process_notification(&mut self, payload: &[u8], now_us: i64, events: &mut Vec<RxEvent>) {
        let length = payload.len();
        let mut offset = 0;
        while offset < length {
            let kind = payload[offset];
            offset += 1;
            if kind == 5 && offset + 6 <= length {
                let weight = decode_weight(&payload[offset..]);
                if self.active || now_us - self.last_publish_us >= IDLE_PUBLISH_MS * 1000 {
                    events.push(RxEvent::Weight(weight));
                    self.last_publish_us = now_us;
                }
                offset += 6;
            } else if kind == 7 && offset + 3 <= length {
                offset += 3; // chrono balance : hors algorithme CoffeeFlow
            } else if kind == 8 && offset < length {
                offset += 1; // boutons bruts : pas encore une action d'interface
            } else {
                break; // sous-message inconnu, taille non encodée : garder le cadrage
            }
        }
    }
}

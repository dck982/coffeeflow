//! Côté logique de la liaison CAN de l'écran : filtrage des trames reçues,
//! présence du module capteurs et sondes PING, confirmation de l'image OTA des
//! capteurs. Aucun accès au contrôleur TWAI : les trames à émettre sortent
//! par [`crate::core::Output::Can`].

use protocol::messages::NodeIdentityPayload;
use protocol::{Dest, Frame, Message, MessageType, Node, Version, heating_protocol_supported};

/// Après ce silence, les sondes PING partent.
pub const PRESENCE_SILENCE_BEFORE_CHECK_US: i64 = 1500 * 1000;
pub const PRESENCE_PROBE_INTERVAL_US: i64 = 500 * 1000;
/// Sans réponse à ce délai après la première sonde, la présence est perdue.
pub const PRESENCE_CHECK_BAIL_US: i64 = 1500 * 1000;
/// Nombre de sondes PING avant de déclarer la perte.
pub const PRESENCE_PROBE_COUNT: u8 = 3;
/// Période de renvoi de `CONFIRM_SENSORS_OTA` tant que les conditions sont
/// réunies.
pub const OTA_CONFIRM_PERIOD_US: i64 = 1000 * 1000;

/// Ce qu'une trame reçue demande au cœur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Inbound {
    /// Pas pour nous (autre destinataire ou autre émetteur que les capteurs) :
    /// ni présence ni traitement.
    Ignored,
    /// Trame des capteurs dont le type est inconnu de cette image : elle
    /// maintient la présence, rien d'autre.
    PresenceOnly,
    /// Trame des capteurs dont la charge utile ne se décode pas.
    Malformed { message_type: MessageType },
    /// Trame décodée des capteurs.
    Message(Message),
}

/// Attribue une trame reçue : seules comptent celles adressées à l'écran ou en
/// broadcast, émises par les capteurs.
pub fn classify(frame: &Frame) -> Inbound {
    let (type_bits, dest_bits, src_bits) = protocol::split_can_id(frame.id);
    if dest_bits != Dest::Broadcast as u8 && dest_bits != Dest::Screen as u8 {
        return Inbound::Ignored;
    }
    if src_bits != Node::Sensors as u8 {
        return Inbound::Ignored;
    }
    if !protocol::is_known_message_type(type_bits) {
        return Inbound::PresenceOnly;
    }
    match Message::decode(frame) {
        Ok(message) => Inbound::Message(message),
        Err(_) => Inbound::Malformed {
            message_type: MessageType::from_u8(type_bits).unwrap_or(MessageType::Stop),
        },
    }
}

/// Action demandée à l'appelant après un pas de [`Presence::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PresenceTick {
    /// Émettre un PING vers les capteurs.
    pub send_ping: bool,
    /// La présence vient d'être déclarée perdue.
    pub lost: bool,
}

/// Machine d'état de présence : tout trafic valide du pair l'entretient ;
/// après 1,5 s de silence, trois PING sont envoyés à 500 ms d'intervalle
/// avant de déclarer la perte. La validation OTA reste plus stricte et exige
/// un PONG.
#[derive(Debug, Clone)]
pub struct Presence {
    last_rx_us: i64,
    check_started_us: i64,
    last_probe_us: i64,
    probe_count: u8,
    lost: bool,
    peer_roundtrip_confirmed: bool,
}

impl Presence {
    /// Au boot la présence est « perdue » jusqu'à la première trame ; le
    /// compteur démarre à `now_us`.
    pub const fn new(now_us: i64) -> Self {
        Self {
            last_rx_us: now_us,
            check_started_us: 0,
            last_probe_us: now_us,
            probe_count: 0,
            lost: true,
            peer_roundtrip_confirmed: false,
        }
    }

    pub const fn lost(&self) -> bool {
        self.lost
    }

    /// Un PONG des capteurs (DLC 8, nœud capteurs) a été reçu.
    pub const fn peer_roundtrip_confirmed(&self) -> bool {
        self.peer_roundtrip_confirmed
    }

    pub fn note_roundtrip(&mut self) {
        self.peer_roundtrip_confirmed = true;
    }

    /// Trame valide du pair. Renvoie `true` si la présence vient d'être
    /// rétablie.
    pub fn mark(&mut self, now_us: i64) -> bool {
        self.last_rx_us = now_us;
        self.check_started_us = 0;
        self.probe_count = 0;
        if self.lost {
            self.lost = false;
            return true;
        }
        false
    }

    /// À appeler à chaque tour de la tâche de télémétrie.
    pub fn tick(&mut self, now_us: i64) -> PresenceTick {
        let mut out = PresenceTick::default();
        if now_us - self.last_rx_us < PRESENCE_SILENCE_BEFORE_CHECK_US {
            return out;
        }
        if self.check_started_us == 0 {
            self.check_started_us = now_us;
            self.last_probe_us = 0;
            self.probe_count = 0;
        }
        if self.probe_count < PRESENCE_PROBE_COUNT
            && (self.probe_count == 0 || now_us - self.last_probe_us >= PRESENCE_PROBE_INTERVAL_US)
        {
            out.send_ping = true;
            self.last_probe_us = now_us;
            self.probe_count += 1;
        }
        if !self.lost && now_us - self.check_started_us >= PRESENCE_CHECK_BAIL_US {
            self.lost = true;
            out.lost = true;
        }
        out
    }
}

/// PING ou PONG de l'écran (8 octets, identité de l'écran).
pub fn identity(version: Version, uptime_s: u32) -> NodeIdentityPayload {
    NodeIdentityPayload::new(Node::Screen, version, uptime_s)
}

/// Identité d'un PING/PONG reçu, si elle vient bien des capteurs.
pub fn sensors_identity(payload: &NodeIdentityPayload) -> Option<Version> {
    (payload.node() == Some(Node::Sensors)).then_some(payload.version)
}

/// Conditions de confirmation de l'image OTA des capteurs (reprises du C++) :
/// capteurs vivants, interface chauffage supportée (version ≥ 0.2.63),
/// acquittements d'actionneurs et de chauffage frais, vanne fermée, pompe à
/// l'arrêt, et chaudière éteinte ou pilotée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtaConfirmState {
    pub sensors_alive: bool,
    pub sensors_version: Version,
    pub actuators_fresh: bool,
    pub heating_fresh: bool,
    pub valve_open: bool,
    pub pump_pct: u8,
    pub heater_on: bool,
    pub heating_power_capable: bool,
    pub heating_power_accepted_permille_nonzero: bool,
    pub heating_lease_remaining_ms: u16,
}

impl OtaConfirmState {
    pub fn ready(&self) -> bool {
        let controlled_heating = self.heating_power_capable
            && self.heating_power_accepted_permille_nonzero
            && self.heating_lease_remaining_ms > 0;
        self.sensors_alive
            && heating_protocol_supported(
                self.sensors_version.major,
                self.sensors_version.minor,
                self.sensors_version.patch,
            )
            && self.actuators_fresh
            && self.heating_fresh
            && !self.valve_open
            && self.pump_pct == 0
            && (!self.heater_on || controlled_heating)
    }
}

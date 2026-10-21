//! Identifiant CAN 11 bits.
//!
//! ```text
//! bits 10..5   type   (64 valeurs) : valeur basse = priorité bus haute
//! bits  4..3   dest   (0 broadcast, 1 écran, 2 capteurs)
//! bits  2..0   src    (1 écran, 2 capteurs)
//! ```
//!
//! Le type EST la priorité : pas de champ séparé.

/// Nœud émetteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Node {
    Screen = 1,
    Sensors = 2,
}

impl Node {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Screen),
            2 => Some(Self::Sensors),
            _ => None,
        }
    }

    /// L'autre nœud du bus.
    pub const fn peer(self) -> Self {
        match self {
            Self::Screen => Self::Sensors,
            Self::Sensors => Self::Screen,
        }
    }
}

/// Destinataire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Dest {
    Broadcast = 0,
    Screen = 1,
    Sensors = 2,
}

impl Dest {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Broadcast),
            1 => Some(Self::Screen),
            2 => Some(Self::Sensors),
            _ => None,
        }
    }

    pub const fn from_node(node: Node) -> Self {
        match node {
            Node::Screen => Self::Screen,
            Node::Sensors => Self::Sensors,
        }
    }
}

impl From<Node> for Dest {
    fn from(node: Node) -> Self {
        Self::from_node(node)
    }
}

/// Type de message (6 bits de l'identifiant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum MessageType {
    Stop = 0x00,
    Set = 0x01,
    Reset = 0x02,
    /// Maintenance de l'esclave DimmerLink, écran vers capteurs. Distinct de
    /// `Reset`, qui redémarre tout le module capteurs.
    DimmerCommand = 0x03,
    SetHeating = 0x04,
    ConfirmSensorsOta = 0x05,
    SetHeatingPower = 0x06,
    /// Maintenance : vanne seule, pompe arrêtée (vidange de la chaudière).
    /// Jamais émise par le cycle d'infusion, qui passe uniquement par `Set`.
    MaintenanceValve = 0x07,
    Ping = 0x08,
    Pong = 0x09,
    ReqStatus = 0x10,
    StatusPressure = 0x20,
    StatusFlow = 0x21,
    StatusActuators = 0x22,
    StatusHeating = 0x23,
    StatusEnclosure = 0x24,
    Log = 0x30,
    FlashCtrl = 0x38,
    FlashData = 0x39,
}

impl MessageType {
    /// `None` si la valeur n'est pas un type connu : une trame de type
    /// inconnu se compte comme présence du pair, mais s'ignore sinon.
    pub const fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x00 => Self::Stop,
            0x01 => Self::Set,
            0x02 => Self::Reset,
            0x03 => Self::DimmerCommand,
            0x04 => Self::SetHeating,
            0x05 => Self::ConfirmSensorsOta,
            0x06 => Self::SetHeatingPower,
            0x07 => Self::MaintenanceValve,
            0x08 => Self::Ping,
            0x09 => Self::Pong,
            0x10 => Self::ReqStatus,
            0x20 => Self::StatusPressure,
            0x21 => Self::StatusFlow,
            0x22 => Self::StatusActuators,
            0x23 => Self::StatusHeating,
            0x24 => Self::StatusEnclosure,
            0x30 => Self::Log,
            0x38 => Self::FlashCtrl,
            0x39 => Self::FlashData,
            _ => return None,
        })
    }
}

/// `true` si `value` est un type de message connu.
pub const fn is_known_message_type(value: u8) -> bool {
    MessageType::from_u8(value).is_some()
}

/// Identifiant CAN décomposé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CanId {
    pub message_type: MessageType,
    pub dest: Dest,
    pub src: Node,
}

/// Champ d'un identifiant qui ne correspond à aucune valeur connue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanIdError {
    UnknownType(u8),
    UnknownDest(u8),
    UnknownSrc(u8),
}

impl core::fmt::Display for CanIdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownType(v) => write!(f, "type de message inconnu: {v:#04x}"),
            Self::UnknownDest(v) => write!(f, "destinataire inconnu: {v}"),
            Self::UnknownSrc(v) => write!(f, "émetteur inconnu: {v}"),
        }
    }
}

impl core::error::Error for CanIdError {}

/// Encode un [`CanId`] en identifiant 11 bits.
pub const fn encode_can_id(id: CanId) -> u16 {
    let type_bits = (id.message_type as u16) & 0x3F;
    let dest_bits = (id.dest as u16) & 0x03;
    let src_bits = (id.src as u16) & 0x07;
    (type_bits << 5) | (dest_bits << 3) | src_bits
}

/// Découpe un identifiant brut en `(type, dest, src)`, sans rien valider.
pub const fn split_can_id(raw: u16) -> (u8, u8, u8) {
    (
        ((raw >> 5) & 0x3F) as u8,
        ((raw >> 3) & 0x03) as u8,
        (raw & 0x07) as u8,
    )
}

/// Décode un identifiant 11 bits. Les bits au-dessus du 11e sont ignorés,
/// comme côté C++ ; c'est le cadrage série qui refuse `id > 0x7FF`.
pub const fn decode_can_id(raw: u16) -> Result<CanId, CanIdError> {
    let (type_bits, dest_bits, src_bits) = split_can_id(raw);
    let Some(message_type) = MessageType::from_u8(type_bits) else {
        return Err(CanIdError::UnknownType(type_bits));
    };
    let Some(dest) = Dest::from_u8(dest_bits) else {
        return Err(CanIdError::UnknownDest(dest_bits));
    };
    let Some(src) = Node::from_u8(src_bits) else {
        return Err(CanIdError::UnknownSrc(src_bits));
    };
    Ok(CanId {
        message_type,
        dest,
        src,
    })
}

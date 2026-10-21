//! Charges utiles du protocole et énumération [`Message`].
//!
//! Chaque message tient dans une trame CAN de 8 octets au plus ; STOP et
//! RESET sont vides. Tout est petit-boutiste. Chaque charge utile a :
//!
//! - `pack()` : les 8 octets de la trame, octets inutilisés à zéro ;
//! - `dlc()` : le DLC réellement émis par les firmwares C++ archivés ;
//! - `unpack(&[u8])` : lit `len` octets (le DLC reçu) et refuse ce que le C++
//!   refuse. Les octets au-delà du minimum sont ignorés, sauf pour
//!   `SET_HEATING_POWER`, dont la longueur choisit le format.

use crate::frame::Frame;
use crate::id::{Dest, MessageType, Node, split_can_id};
use crate::version::Version;

/// Pourquoi une trame ne se décode pas en [`Message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Type de message hors de la table (le pair reste présent, mais la
    /// trame s'ignore).
    UnknownType(u8),
    /// Identifiant au-delà de 11 bits.
    InvalidId,
    /// DLC trop court pour ce message, ou supérieur à 8.
    BadLength,
    /// Une valeur est hors de son domaine (booléen à 2, sous-commande
    /// inconnue, puissance au-delà de 100 %, etc.).
    InvalidValue,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownType(t) => write!(f, "type de message inconnu: {t:#04x}"),
            Self::InvalidId => f.write_str("identifiant CAN au-delà de 11 bits"),
            Self::BadLength => f.write_str("DLC incohérent avec le message"),
            Self::InvalidValue => f.write_str("valeur hors domaine"),
        }
    }
}

impl core::error::Error for DecodeError {}

fn get_u16(p: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([p[at], p[at + 1]])
}

fn get_u32(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]])
}

fn put_u16(f: &mut [u8; 8], at: usize, v: u16) {
    f[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32(f: &mut [u8; 8], at: usize, v: u32) {
    f[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

const fn need(p: &[u8], n: usize) -> Result<(), DecodeError> {
    if p.len() < n {
        Err(DecodeError::BadLength)
    } else {
        Ok(())
    }
}

/// SET (0x01), écran vers capteurs. Le niveau de pompe demandé est l'unique
/// intention exposée par le protocole ; les capteurs séquencent vanne et
/// pompe localement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetPayload {
    /// 0..=100
    pub pump_pct: u8,
    /// 0 = défaut (500 ms)
    pub ttl_ms: u16,
}

impl SetPayload {
    pub const DLC: u8 = 3;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = self.pump_pct;
        put_u16(&mut f, 1, self.ttl_ms);
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 3)?;
        Ok(Self {
            pump_pct: p[0],
            ttl_ms: get_u16(p, 1),
        })
    }
}

/// Durée maximale d'une ouverture de la vanne de maintenance.
pub const MAINTENANCE_VALVE_MAX_MS: u16 = 30000;

/// MAINTENANCE_VALVE (0x07), écran vers capteurs. Ouvre la vanne seule,
/// pompe forcée à 0, pour une durée bornée par les capteurs eux-mêmes ;
/// jamais renouvelée automatiquement. `open == false` ferme la vanne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaintenanceValvePayload {
    pub open: bool,
    /// `1..=MAINTENANCE_VALVE_MAX_MS` si `open`, ignoré (et émis à 0) sinon.
    pub duration_ms: u16,
}

impl MaintenanceValvePayload {
    pub const DLC: u8 = 3;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = u8::from(self.open);
        put_u16(&mut f, 1, if self.open { self.duration_ms } else { 0 });
        f
    }

    /// Une ouverture sans durée ou au-delà du plafond est refusée, pas
    /// bornée en silence.
    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 3)?;
        if p[0] > 1 {
            return Err(DecodeError::InvalidValue);
        }
        let duration = get_u16(p, 1);
        let open = p[0] != 0;
        if open && (duration == 0 || duration > MAINTENANCE_VALVE_MAX_MS) {
            return Err(DecodeError::InvalidValue);
        }
        Ok(Self {
            open,
            duration_ms: if open { duration } else { 0 },
        })
    }
}

/// SET_HEATING (0x04) : commande chaudière indépendante du SET infusion.
/// Bail non renouvelé par l'écran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetHeatingPayload {
    pub on: bool,
    pub duration_ms: u16,
}

impl SetHeatingPayload {
    pub const DLC: u8 = 3;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = u8::from(self.on);
        put_u16(&mut f, 1, self.duration_ms);
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 3)?;
        if p[0] > 1 {
            return Err(DecodeError::InvalidValue);
        }
        Ok(Self {
            on: p[0] != 0,
            duration_ms: get_u16(p, 1),
        })
    }
}

/// SET_HEATING_POWER (0x06) : régulation continue. Le bail est renouvelé
/// même lorsque la puissance est nulle.
///
/// Trois formats en réception, choisis par le DLC :
/// - 3 octets (écran 0.2.68) : puissance en pourcents sur 1 octet (0..=100),
///   puis le bail ;
/// - 4 octets : puissance en dixièmes de pourcent (0..=1000), puis le bail ;
/// - 5 octets : idem, plus un octet de drapeaux dont le bit 0 est
///   `restart_window`.
///
/// Toute autre longueur est refusée, de même qu'un drapeau inconnu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetHeatingPowerPayload {
    /// 0..=1000 = 0,0..100,0 %
    pub power_permille: u16,
    pub lease_ms: u16,
    /// Ouvre une nouvelle période PWM à la réception, au lieu d'attendre la
    /// fin de la période en cours. L'écran ne l'envoie (DLC 5) qu'à un module
    /// qui annonce `window_restart_capable` ; sinon la trame reste en DLC 4.
    pub restart_window: bool,
}

impl SetHeatingPowerPayload {
    pub const FLAG_RESTART_WINDOW: u8 = 0x01;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        put_u16(&mut f, 0, self.power_permille);
        put_u16(&mut f, 2, self.lease_ms);
        f[4] = if self.restart_window {
            Self::FLAG_RESTART_WINDOW
        } else {
            0
        };
        f
    }

    pub const fn dlc(&self) -> u8 {
        if self.restart_window { 5 } else { 4 }
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        if p.len() == 3 && p[0] <= 100 {
            return Ok(Self {
                power_permille: u16::from(p[0]) * 10,
                lease_ms: get_u16(p, 1),
                restart_window: false,
            });
        }
        if p.len() != 4 && p.len() != 5 {
            return Err(DecodeError::BadLength);
        }
        if get_u16(p, 0) > 1000 {
            return Err(DecodeError::InvalidValue);
        }
        if p.len() == 5 && (p[4] & !Self::FLAG_RESTART_WINDOW) != 0 {
            return Err(DecodeError::InvalidValue);
        }
        Ok(Self {
            power_permille: get_u16(p, 0),
            lease_ms: get_u16(p, 2),
            restart_window: p.len() == 5 && (p[4] & Self::FLAG_RESTART_WINDOW) != 0,
        })
    }
}

/// STATUS_HEATING (0x23). Les capacités s'annoncent dans l'octet 3 : bit 0
/// toujours 1 (capteur valide), bit 1 puissance, bit 2 résolution 0,1 %,
/// bit 3 `restart_window`. Un récepteur ancien envoie 4 octets, un récepteur
/// à puissance entière 5, un à résolution fine 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusHeatingPayload {
    pub heater_on: bool,
    pub lease_remaining_ms: u16,
    pub power_capable: bool,
    pub fine_power_capable: bool,
    /// Accepte [`SetHeatingPowerPayload::restart_window`].
    pub window_restart_capable: bool,
    pub power_permille: u16,
}

impl StatusHeatingPayload {
    pub const DLC: u8 = 6;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = u8::from(self.heater_on);
        put_u16(&mut f, 1, self.lease_remaining_ms);
        f[3] = 1
            | if self.power_capable { 2 } else { 0 }
            | if self.fine_power_capable { 4 } else { 0 }
            | if self.window_restart_capable { 8 } else { 0 };
        f[4] = (self.power_permille / 10) as u8;
        f[5] = (self.power_permille % 10) as u8;
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 4)?;
        if p[0] > 1 || (p[3] & 1) == 0 {
            return Err(DecodeError::InvalidValue);
        }
        let power_capable = (p[3] & 2) != 0 && p.len() >= 5;
        if power_capable && p[4] > 100 {
            return Err(DecodeError::InvalidValue);
        }
        if (p[3] & 4) != 0 {
            need(p, 6)?;
            if p[5] > 9 || (p[4] == 100 && p[5] != 0) {
                return Err(DecodeError::InvalidValue);
            }
        }
        let fine_power_capable = (p[3] & 4) != 0 && power_capable;
        let window_restart_capable = (p[3] & 8) != 0 && fine_power_capable;
        let power_permille = if power_capable {
            u16::from(p[4]) * 10
                + if fine_power_capable {
                    u16::from(p[5])
                } else {
                    0
                }
        } else {
            0
        };
        Ok(Self {
            heater_on: p[0] != 0,
            lease_remaining_ms: get_u16(p, 1),
            power_capable,
            fine_power_capable,
            window_restart_capable,
            power_permille,
        })
    }
}

/// Identité transportée par PING (0x08) et PONG (0x09), dans les deux sens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeIdentityPayload {
    /// Octet brut, tel que reçu : un récepteur compare à [`Node`] lui-même
    /// (l'écran n'accepte un PONG que si c'est `Sensors`).
    pub node: u8,
    pub version: Version,
    pub uptime_s: u32,
}

impl NodeIdentityPayload {
    pub const DLC: u8 = 8;

    pub const fn new(node: Node, version: Version, uptime_s: u32) -> Self {
        Self {
            node: node as u8,
            version,
            uptime_s,
        }
    }

    /// `None` si l'octet n'est ni l'écran ni les capteurs.
    pub const fn node(&self) -> Option<Node> {
        Node::from_u8(self.node)
    }

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = self.node;
        f[1] = self.version.major;
        f[2] = self.version.minor;
        f[3] = self.version.patch;
        put_u32(&mut f, 4, self.uptime_s);
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 8)?;
        Ok(Self {
            node: p[0],
            version: Version::new(p[1], p[2], p[3]),
            uptime_s: get_u32(p, 4),
        })
    }
}

/// Commande DimmerLink de DIMMER_COMMAND (0x03). Les capteurs sont le seul
/// nœud autorisé à écrire le bus I2C du DimmerLink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum DimmerCommand {
    Reset = 0x01,
    #[default]
    Recalibrate = 0x02,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DimmerCommandPayload {
    pub command: DimmerCommand,
}

impl DimmerCommandPayload {
    pub const DLC: u8 = 1;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = self.command as u8;
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 1)?;
        let command = match p[0] {
            0x01 => DimmerCommand::Reset,
            0x02 => DimmerCommand::Recalibrate,
            _ => return Err(DecodeError::InvalidValue),
        };
        Ok(Self { command })
    }
}

/// CONFIRM_SENSORS_OTA (0x05) : l'écran confirme la nouvelle image des
/// capteurs. Le récepteur exige DLC 1 et le jeton
/// [`HEATING_PROTOCOL_CONFIRMATION_TOKEN`](crate::version::HEATING_PROTOCOL_CONFIRMATION_TOKEN) ;
/// ce contrôle de politique reste au récepteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConfirmSensorsOtaPayload {
    pub protocol_patch: u8,
}

impl Default for ConfirmSensorsOtaPayload {
    fn default() -> Self {
        Self {
            protocol_patch: crate::version::HEATING_PROTOCOL_CONFIRMATION_TOKEN,
        }
    }
}

impl ConfirmSensorsOtaPayload {
    pub const DLC: u8 = 1;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = self.protocol_patch;
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 1)?;
        Ok(Self {
            protocol_patch: p[0],
        })
    }
}

/// REQSTATUS (0x10), écran vers capteurs : période d'émission d'un STATUS_*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReqStatusPayload {
    /// Octet brut : 0x20 à 0x24 en pratique.
    pub target_type: u8,
    /// 0 = arrêt
    pub period_ms: u16,
}

impl ReqStatusPayload {
    pub const DLC: u8 = 3;

    pub const fn new(target: MessageType, period_ms: u16) -> Self {
        Self {
            target_type: target as u8,
            period_ms,
        }
    }

    /// `None` si l'octet n'est pas un type connu.
    pub const fn target(&self) -> Option<MessageType> {
        MessageType::from_u8(self.target_type)
    }

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = self.target_type;
        put_u16(&mut f, 1, self.period_ms);
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 3)?;
        Ok(Self {
            target_type: p[0],
            period_ms: get_u16(p, 1),
        })
    }
}

impl Default for ReqStatusPayload {
    fn default() -> Self {
        Self::new(MessageType::StatusPressure, 0)
    }
}

/// STATUS_PRESSURE (0x20) : recopie du registre 0x06 du XDB401.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusPressurePayload {
    /// 24 bits utiles ; les bits de poids fort ne sont pas émis.
    pub pressure_raw: u32,
    pub temperature_raw: u16,
    /// 16 bits bas
    pub timestamp_ms: u16,
    /// Bit 0 capteur valide (détection réelle : le XDB401 répond sur I2C),
    /// bit 1 timeout de conversion. Même bit 0, même sens, sur tous les
    /// STATUS_* qui en ont un.
    pub flags: u8,
}

impl StatusPressurePayload {
    pub const DLC: u8 = 8;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0..3].copy_from_slice(&self.pressure_raw.to_le_bytes()[..3]);
        put_u16(&mut f, 3, self.temperature_raw);
        put_u16(&mut f, 5, self.timestamp_ms);
        f[7] = self.flags;
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 8)?;
        Ok(Self {
            pressure_raw: u32::from(p[0]) | (u32::from(p[1]) << 8) | (u32::from(p[2]) << 16),
            temperature_raw: get_u16(p, 3),
            timestamp_ms: get_u16(p, 5),
            flags: p[7],
        })
    }
}

/// STATUS_FLOW (0x21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusFlowPayload {
    /// Cumulé depuis le reset
    pub pulse_count: u32,
    /// 16 bits bas
    pub last_edge_ms: u16,
    /// Bit 0 capteur valide, toujours 1 ici : une simple entrée GPIO ne
    /// permet pas de détecter l'absence du débitmètre, seule l'absence
    /// d'impulsions attendues le laisse deviner (LOG `FLOWMETER_SILENT`).
    pub flags: u8,
}

impl StatusFlowPayload {
    pub const DLC: u8 = 7;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        put_u32(&mut f, 0, self.pulse_count);
        put_u16(&mut f, 4, self.last_edge_ms);
        f[6] = self.flags;
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 7)?;
        Ok(Self {
            pulse_count: get_u32(p, 0),
            last_edge_ms: get_u16(p, 4),
            flags: p[6],
        })
    }
}

/// STATUS_ACTUATORS (0x22) : accusé de réception d'un SET.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusActuatorsPayload {
    pub valve_open: bool,
    pub pump_pct: u8,
    pub lease_remaining_ms: u16,
    pub continuous_on_ms: u16,
    /// Bit 0 verrou actif, bit 1 DimmerLink prêt, bit 2 DimmerLink valide
    /// (détection I2C), bit 3 erreur DimmerLink, bit 4 vanne ouverte en
    /// maintenance, bit 5 MAINTENANCE_VALVE pris en charge.
    pub flags: u8,
}

impl StatusActuatorsPayload {
    pub const DLC: u8 = 7;
    pub const FLAG_LOCKOUT: u8 = 0x01;
    pub const FLAG_DIMMER_READY: u8 = 0x02;
    pub const FLAG_DIMMER_PRESENT: u8 = 0x04;
    pub const FLAG_DIMMER_ERROR: u8 = 0x08;
    pub const FLAG_MAINTENANCE_VALVE_OPEN: u8 = 0x10;
    pub const FLAG_MAINTENANCE_VALVE_CAPABLE: u8 = 0x20;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = u8::from(self.valve_open);
        f[1] = self.pump_pct;
        put_u16(&mut f, 2, self.lease_remaining_ms);
        put_u16(&mut f, 4, self.continuous_on_ms);
        f[6] = self.flags;
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 7)?;
        Ok(Self {
            valve_open: p[0] != 0,
            pump_pct: p[1],
            lease_remaining_ms: get_u16(p, 2),
            continuous_on_ms: get_u16(p, 4),
            flags: p[6],
        })
    }
}

/// STATUS_ENCLOSURE (0x24) : température du boîtier des capteurs, lue sur
/// le TMP102 de la carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusEnclosurePayload {
    /// 1/16 °C (résolution 12 bits du TMP102), signé.
    pub temperature_raw: i16,
    /// 16 bits bas
    pub timestamp_ms: u16,
    /// Bit 0 capteur valide (le TMP102 a répondu à cette lecture).
    pub flags: u8,
}

impl StatusEnclosurePayload {
    pub const DLC: u8 = 5;
    pub const FLAG_VALID: u8 = 0x01;

    pub fn temperature_c(&self) -> f32 {
        f32::from(self.temperature_raw) / 16.0
    }

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0..2].copy_from_slice(&self.temperature_raw.to_le_bytes());
        put_u16(&mut f, 2, self.timestamp_ms);
        f[4] = self.flags;
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 5)?;
        Ok(Self {
            temperature_raw: get_u16(p, 0) as i16,
            timestamp_ms: get_u16(p, 2),
            flags: p[4],
        })
    }
}

/// LOG (0x30) : pas de texte sur le bus, seulement un code de la table
/// [`LOG_CODES`](crate::LOG_CODES).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LogPayload {
    pub code: u8,
    pub severity: u8,
    pub arg16: u16,
    pub arg32: u32,
}

impl LogPayload {
    pub const DLC: u8 = 8;

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = self.code;
        f[1] = self.severity;
        put_u16(&mut f, 2, self.arg16);
        put_u32(&mut f, 4, self.arg32);
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 8)?;
        Ok(Self {
            code: p[0],
            severity: p[1],
            arg16: get_u16(p, 2),
            arg32: get_u32(p, 4),
        })
    }
}

/// FLASH_CTRL (0x38) : sous-commande de la mise à jour OTA des capteurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlashCtrlPayload {
    Begin {
        image_size: u32,
    },
    BlockAck {
        block_number: u16,
        block_crc16: u16,
    },
    End {
        image_crc32: u32,
    },
    Abort,
    /// Numéro et CRC du bloc annoncé avant ses FLASH_DATA.
    BlockStart {
        block_number: u16,
        block_crc16: u16,
    },
}

impl FlashCtrlPayload {
    pub const SUBCMD_BEGIN: u8 = 0;
    pub const SUBCMD_BLOCK_ACK: u8 = 1;
    pub const SUBCMD_END: u8 = 2;
    pub const SUBCMD_ABORT: u8 = 3;
    pub const SUBCMD_BLOCK_START: u8 = 4;

    pub const fn subcmd(&self) -> u8 {
        match self {
            Self::Begin { .. } => Self::SUBCMD_BEGIN,
            Self::BlockAck { .. } => Self::SUBCMD_BLOCK_ACK,
            Self::End { .. } => Self::SUBCMD_END,
            Self::Abort => Self::SUBCMD_ABORT,
            Self::BlockStart { .. } => Self::SUBCMD_BLOCK_START,
        }
    }

    /// 8 pour tout sauf `BlockStart` (5), comme les émetteurs C++.
    pub const fn dlc(&self) -> u8 {
        match self {
            Self::BlockStart { .. } => 5,
            _ => 8,
        }
    }

    pub fn pack(&self) -> [u8; 8] {
        let mut f = [0u8; 8];
        f[0] = self.subcmd();
        match *self {
            Self::Begin { image_size } => put_u32(&mut f, 1, image_size),
            Self::BlockAck {
                block_number,
                block_crc16,
            }
            | Self::BlockStart {
                block_number,
                block_crc16,
            } => {
                put_u16(&mut f, 1, block_number);
                put_u16(&mut f, 3, block_crc16);
            }
            Self::End { image_crc32 } => put_u32(&mut f, 1, image_crc32),
            Self::Abort => {}
        }
        f
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        need(p, 1)?;
        match p[0] {
            Self::SUBCMD_BEGIN => {
                need(p, 5)?;
                Ok(Self::Begin {
                    image_size: get_u32(p, 1),
                })
            }
            Self::SUBCMD_BLOCK_ACK => {
                need(p, 5)?;
                Ok(Self::BlockAck {
                    block_number: get_u16(p, 1),
                    block_crc16: get_u16(p, 3),
                })
            }
            Self::SUBCMD_END => {
                need(p, 5)?;
                Ok(Self::End {
                    image_crc32: get_u32(p, 1),
                })
            }
            Self::SUBCMD_ABORT => Ok(Self::Abort),
            Self::SUBCMD_BLOCK_START => {
                need(p, 5)?;
                Ok(Self::BlockStart {
                    block_number: get_u16(p, 1),
                    block_crc16: get_u16(p, 3),
                })
            }
            _ => Err(DecodeError::InvalidValue),
        }
    }
}

/// FLASH_DATA (0x39) : 1 à 8 octets bruts, aucun en-tête. La trame CAN
/// elle-même EST la charge utile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FlashDataPayload {
    len: u8,
    data: [u8; 8],
}

impl FlashDataPayload {
    /// `None` si `bytes` est vide ou fait plus de 8 octets (un FLASH_DATA
    /// vide est ignoré par le récepteur).
    pub fn new(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() || bytes.len() > 8 {
            return None;
        }
        let mut data = [0u8; 8];
        data[..bytes.len()].copy_from_slice(bytes);
        Some(Self {
            len: bytes.len() as u8,
            data,
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.data[..usize::from(self.len)]
    }

    pub const fn dlc(&self) -> u8 {
        self.len
    }

    pub const fn pack(&self) -> [u8; 8] {
        self.data
    }

    pub fn unpack(p: &[u8]) -> Result<Self, DecodeError> {
        Self::new(p).ok_or(DecodeError::BadLength)
    }
}

/// Un message du protocole, avec sa charge utile décodée.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Message {
    Stop,
    Set(SetPayload),
    Reset,
    DimmerCommand(DimmerCommandPayload),
    SetHeating(SetHeatingPayload),
    ConfirmSensorsOta(ConfirmSensorsOtaPayload),
    SetHeatingPower(SetHeatingPowerPayload),
    MaintenanceValve(MaintenanceValvePayload),
    Ping(NodeIdentityPayload),
    /// PING DLC 0, format des images antérieures à v0.2.30 : toujours accepté
    /// en réception.
    PingLegacy,
    Pong(NodeIdentityPayload),
    ReqStatus(ReqStatusPayload),
    StatusPressure(StatusPressurePayload),
    StatusFlow(StatusFlowPayload),
    StatusActuators(StatusActuatorsPayload),
    StatusHeating(StatusHeatingPayload),
    StatusEnclosure(StatusEnclosurePayload),
    Log(LogPayload),
    FlashCtrl(FlashCtrlPayload),
    FlashData(FlashDataPayload),
}

impl Message {
    pub const fn message_type(&self) -> MessageType {
        match self {
            Self::Stop => MessageType::Stop,
            Self::Set(_) => MessageType::Set,
            Self::Reset => MessageType::Reset,
            Self::DimmerCommand(_) => MessageType::DimmerCommand,
            Self::SetHeating(_) => MessageType::SetHeating,
            Self::ConfirmSensorsOta(_) => MessageType::ConfirmSensorsOta,
            Self::SetHeatingPower(_) => MessageType::SetHeatingPower,
            Self::MaintenanceValve(_) => MessageType::MaintenanceValve,
            Self::Ping(_) | Self::PingLegacy => MessageType::Ping,
            Self::Pong(_) => MessageType::Pong,
            Self::ReqStatus(_) => MessageType::ReqStatus,
            Self::StatusPressure(_) => MessageType::StatusPressure,
            Self::StatusFlow(_) => MessageType::StatusFlow,
            Self::StatusActuators(_) => MessageType::StatusActuators,
            Self::StatusHeating(_) => MessageType::StatusHeating,
            Self::StatusEnclosure(_) => MessageType::StatusEnclosure,
            Self::Log(_) => MessageType::Log,
            Self::FlashCtrl(_) => MessageType::FlashCtrl,
            Self::FlashData(_) => MessageType::FlashData,
        }
    }

    /// DLC émis par les firmwares C++ pour ce message.
    pub const fn dlc(&self) -> u8 {
        match self {
            Self::Stop | Self::Reset | Self::PingLegacy => 0,
            Self::Set(_) => SetPayload::DLC,
            Self::DimmerCommand(_) => DimmerCommandPayload::DLC,
            Self::SetHeating(_) => SetHeatingPayload::DLC,
            Self::ConfirmSensorsOta(_) => ConfirmSensorsOtaPayload::DLC,
            Self::SetHeatingPower(p) => p.dlc(),
            Self::MaintenanceValve(_) => MaintenanceValvePayload::DLC,
            Self::Ping(_) | Self::Pong(_) => NodeIdentityPayload::DLC,
            Self::ReqStatus(_) => ReqStatusPayload::DLC,
            Self::StatusPressure(_) => StatusPressurePayload::DLC,
            Self::StatusFlow(_) => StatusFlowPayload::DLC,
            Self::StatusActuators(_) => StatusActuatorsPayload::DLC,
            Self::StatusHeating(_) => StatusHeatingPayload::DLC,
            Self::StatusEnclosure(_) => StatusEnclosurePayload::DLC,
            Self::Log(_) => LogPayload::DLC,
            Self::FlashCtrl(p) => p.dlc(),
            Self::FlashData(p) => p.dlc(),
        }
    }

    /// Émetteur et destinataire d'un message envoyé par le firmware C++ :
    /// ordres et FLASH_* de l'écran vers les capteurs, STATUS_* des capteurs
    /// vers l'écran, LOG des capteurs en broadcast, PING/PONG vers l'autre
    /// nœud (PING DLC 0 de l'écran en broadcast), accusés `BlockAck` des
    /// capteurs vers l'écran. L'écran émet aussi des LOG et les capteurs un
    /// accusé `End` : voir [`Message::encode_routed`].
    pub const fn default_route(&self) -> (Node, Dest) {
        match self {
            Self::Stop
            | Self::Set(_)
            | Self::Reset
            | Self::DimmerCommand(_)
            | Self::SetHeating(_)
            | Self::ConfirmSensorsOta(_)
            | Self::SetHeatingPower(_)
            | Self::MaintenanceValve(_)
            | Self::ReqStatus(_)
            | Self::FlashData(_) => (Node::Screen, Dest::Sensors),
            Self::Ping(id) | Self::Pong(id) => match id.node() {
                Some(node) => (node, Dest::from_node(node.peer())),
                None => (Node::Screen, Dest::Broadcast),
            },
            Self::PingLegacy => (Node::Screen, Dest::Broadcast),
            Self::StatusPressure(_)
            | Self::StatusFlow(_)
            | Self::StatusActuators(_)
            | Self::StatusHeating(_)
            | Self::StatusEnclosure(_) => (Node::Sensors, Dest::Screen),
            Self::Log(_) => (Node::Sensors, Dest::Broadcast),
            Self::FlashCtrl(FlashCtrlPayload::BlockAck { .. }) => (Node::Sensors, Dest::Screen),
            Self::FlashCtrl(_) => (Node::Screen, Dest::Sensors),
        }
    }

    /// Trame du message avec l'acheminement par défaut.
    pub fn encode(&self) -> Frame {
        let (src, dest) = self.default_route();
        self.encode_routed(src, dest)
    }

    /// Trame du message émise par `src` vers `dest`.
    pub fn encode_routed(&self, src: Node, dest: Dest) -> Frame {
        let data = match self {
            Self::Stop | Self::Reset | Self::PingLegacy => [0u8; 8],
            Self::Set(p) => p.pack(),
            Self::DimmerCommand(p) => p.pack(),
            Self::SetHeating(p) => p.pack(),
            Self::ConfirmSensorsOta(p) => p.pack(),
            Self::SetHeatingPower(p) => p.pack(),
            Self::MaintenanceValve(p) => p.pack(),
            Self::Ping(p) | Self::Pong(p) => p.pack(),
            Self::ReqStatus(p) => p.pack(),
            Self::StatusPressure(p) => p.pack(),
            Self::StatusFlow(p) => p.pack(),
            Self::StatusActuators(p) => p.pack(),
            Self::StatusHeating(p) => p.pack(),
            Self::StatusEnclosure(p) => p.pack(),
            Self::Log(p) => p.pack(),
            Self::FlashCtrl(p) => p.pack(),
            Self::FlashData(p) => p.pack(),
        };
        Frame {
            id: crate::id::encode_can_id(crate::id::CanId {
                message_type: self.message_type(),
                dest,
                src,
            }),
            len: self.dlc(),
            data,
        }
    }

    /// Décode la charge utile selon le type de l'identifiant. L'émetteur et
    /// le destinataire ne sont pas contrôlés : voir [`Frame::can_id`].
    pub fn decode(frame: &Frame) -> Result<Self, DecodeError> {
        if frame.len > 8 {
            return Err(DecodeError::BadLength);
        }
        if frame.id > 0x7FF {
            return Err(DecodeError::InvalidId);
        }
        let (type_bits, _, _) = split_can_id(frame.id);
        let message_type =
            MessageType::from_u8(type_bits).ok_or(DecodeError::UnknownType(type_bits))?;
        let p = frame.payload();
        Ok(match message_type {
            MessageType::Stop => Self::Stop,
            MessageType::Set => Self::Set(SetPayload::unpack(p)?),
            MessageType::Reset => Self::Reset,
            MessageType::DimmerCommand => Self::DimmerCommand(DimmerCommandPayload::unpack(p)?),
            MessageType::SetHeating => Self::SetHeating(SetHeatingPayload::unpack(p)?),
            MessageType::ConfirmSensorsOta => {
                Self::ConfirmSensorsOta(ConfirmSensorsOtaPayload::unpack(p)?)
            }
            MessageType::SetHeatingPower => {
                Self::SetHeatingPower(SetHeatingPowerPayload::unpack(p)?)
            }
            MessageType::MaintenanceValve => {
                Self::MaintenanceValve(MaintenanceValvePayload::unpack(p)?)
            }
            MessageType::Ping if p.is_empty() => Self::PingLegacy,
            MessageType::Ping => Self::Ping(NodeIdentityPayload::unpack(p)?),
            MessageType::Pong => Self::Pong(NodeIdentityPayload::unpack(p)?),
            MessageType::ReqStatus => Self::ReqStatus(ReqStatusPayload::unpack(p)?),
            MessageType::StatusPressure => Self::StatusPressure(StatusPressurePayload::unpack(p)?),
            MessageType::StatusFlow => Self::StatusFlow(StatusFlowPayload::unpack(p)?),
            MessageType::StatusActuators => {
                Self::StatusActuators(StatusActuatorsPayload::unpack(p)?)
            }
            MessageType::StatusHeating => Self::StatusHeating(StatusHeatingPayload::unpack(p)?),
            MessageType::StatusEnclosure => {
                Self::StatusEnclosure(StatusEnclosurePayload::unpack(p)?)
            }
            MessageType::Log => Self::Log(LogPayload::unpack(p)?),
            MessageType::FlashCtrl => Self::FlashCtrl(FlashCtrlPayload::unpack(p)?),
            MessageType::FlashData => Self::FlashData(FlashDataPayload::unpack(p)?),
        })
    }
}

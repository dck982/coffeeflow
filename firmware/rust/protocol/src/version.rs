//! Version de l'image, remontée dans PING/PONG, et compatibilité de
//! l'interface chauffage.

/// Numéro de version `major.minor.patch`, trois octets dans PING/PONG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl Version {
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Voir [`heating_protocol_supported`].
    pub const fn heating_protocol_supported(self) -> bool {
        heating_protocol_supported(self.major, self.minor, self.patch)
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Version des deux firmwares Rust (`screen` et `sensors` avancent ensemble).
/// À incrémenter à chaque image flashée, dans les deux projets.
pub const FIRMWARE_VERSION: Version = Version::new(0, 4, 25);

/// Version minimale de l'interface chauffage, comparée sur les trois nombres.
pub const HEATING_PROTOCOL_MIN: Version = Version::new(0, 2, 63);

/// Jeton CAN de `CONFIRM_SENSORS_OTA`. Il reste celui de l'interface 0.2.63,
/// et non la version courante, pour que les anciennes images le reconnaissent.
pub const HEATING_PROTOCOL_CONFIRMATION_TOKEN: u8 = 63;

/// Le pair parle-t-il une interface chauffage que ce firmware comprend ?
/// Même majeure 0 et au moins 0.2.63.
pub const fn heating_protocol_supported(major: u8, minor: u8, patch: u8) -> bool {
    major == HEATING_PROTOCOL_MIN.major
        && (minor > HEATING_PROTOCOL_MIN.minor
            || (minor == HEATING_PROTOCOL_MIN.minor && patch >= HEATING_PROTOCOL_MIN.patch))
}

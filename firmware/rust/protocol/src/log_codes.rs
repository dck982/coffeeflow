//! Table des codes LOG. Le message LOG ne porte pas de texte sur le bus,
//! seulement un code ; l'énumération et la table viennent de
//! `log_codes.yaml`, généré par `build.rs`.

/// Une ligne de la table : le code, son nom, sa gravité et sa description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogCodeInfo {
    pub code: LogCode,
    pub name: &'static str,
    pub severity: LogSeverity,
    /// Dit aussi ce que valent `arg16` et `arg32` pour ce code.
    pub description: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/log_codes.rs"));

impl LogCode {
    pub const fn from_u8(value: u8) -> Option<Self> {
        if (value as usize) < LOG_CODE_COUNT {
            Some(LOG_CODES[value as usize].code)
        } else {
            None
        }
    }

    pub const fn info(self) -> &'static LogCodeInfo {
        &LOG_CODES[self as usize]
    }

    /// Nom du code, tel qu'écrit dans le YAML (`RUNTIME_LOCKOUT_TRIGGERED`).
    pub const fn name(self) -> &'static str {
        self.info().name
    }

    pub const fn severity(self) -> LogSeverity {
        self.info().severity
    }

    pub const fn description(self) -> &'static str {
        self.info().description
    }
}

impl LogSeverity {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Debug),
            1 => Some(Self::Info),
            2 => Some(Self::Warn),
            3 => Some(Self::Error),
            _ => None,
        }
    }

    /// Nom en minuscules, comme `LOG_SEVERITIES` côté Python.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// Nom d'un code reçu du bus, `"UNKNOWN"` s'il n'est pas dans la table
/// (comme `log_code_name` côté C++).
pub const fn log_code_name(code: u8) -> &'static str {
    match LogCode::from_u8(code) {
        Some(code) => code.name(),
        None => "UNKNOWN",
    }
}

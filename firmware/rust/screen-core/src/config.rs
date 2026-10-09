//! Configuration utilisateur persistante : valeurs par défaut, validation,
//! bornes, blob NVS (schéma 10) et migrations des schémas 1 à 9.
//!
//! Les calibrations machine ne passent ni par NVS ni par `/config` : elles
//! sont compilées dans [`crate::calibration`].
//!
//! Le stockage est abstrait par [`ConfigStore`] : deux emplacements
//! transactionnels `a` et `b` (blobs d'une centaine d'octets, espace de noms
//! NVS `ui`), écrits en alternance. L'écriture est synchrone parce que
//! `/config` doit pouvoir répondre `500 storage` sans avoir rien modifié.

use alloc::vec;
use alloc::vec::Vec;
use core::ops::BitOr;

pub const CONFIG_SCHEMA_VERSION: u16 = 10;
pub const MINIMUM_BREW_TEMPERATURE_C: f32 = 50.0;
pub const MAXIMUM_BREW_TEMPERATURE_C: f32 = 100.0;
pub const MAXIMUM_BREW_PREHEAT_TIME_S: f32 = 15.0;
pub const BREW_TEMPERATURE_TOLERANCE_C: f32 = 1.0;
/// Sous cette puissance, la pompe ne maintient plus une pression d'infusion
/// sûre. Cette borne est distincte des puissances de remplissage et de purge.
pub const MINIMUM_BREW_PUMP_PCT: u8 = 50;

/// Masque des bits de pré-infusion encore valides. Le bit 1 désignait la
/// sortie par pression (schéma 7 et antérieurs) ; il est retiré, mais les
/// bits suivants gardent leur valeur pour que l'encodage NVS reste lisible.
pub const PREINFUSION_MODE_MASK: u8 = 0x05;

/// Sorties de pré-infusion : ensemble de bits (temps, poids).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreinfusionMode(pub u8);

impl PreinfusionMode {
    pub const NONE: Self = Self(0);
    pub const TIME: Self = Self(1 << 0);
    pub const WEIGHT: Self = Self(1 << 2);

    /// `self` contient-il tous les bits de `mode` ? Faux si `mode` est vide,
    /// comme `(modes & mode) != kNone`.
    pub const fn has(self, mode: Self) -> bool {
        (self.0 & mode.0) != 0
    }
}

impl BitOr for PreinfusionMode {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Fin d'infusion progressive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RampdownMode {
    #[default]
    None = 0,
    Time = 1,
    Weight = 2,
    PressureDrop = 3,
}

impl RampdownMode {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Time),
            2 => Some(Self::Weight),
            3 => Some(Self::PressureDrop),
            _ => None,
        }
    }

    /// Nom dans le JSON de `/config`.
    pub const fn text(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Time => "time",
            Self::Weight => "weight",
            Self::PressureDrop => "pressure_drop",
        }
    }
}

/// Bornes et pas de la validation, exposés pour que l'UI les partage.
pub mod bounds {
    pub const TARGET_WEIGHT_G: (f32, f32, f32) = (10.0, 100.0, 0.5);
    pub const TARGET_TIME_S: (u16, u16) = (5, 60);
    pub const TARGET_PRESSURE_BAR: (f32, f32, f32) = (6.0, 12.0, 0.1);
    pub const BREW_TEMPERATURE_C: (f32, f32, f32) = (
        super::MINIMUM_BREW_TEMPERATURE_C,
        super::MAXIMUM_BREW_TEMPERATURE_C,
        0.5,
    );
    pub const BREW_PREHEAT_TIME_S: (f32, f32, f32) = (0.0, super::MAXIMUM_BREW_PREHEAT_TIME_S, 0.5);
    pub const FILLING_TIME_S: (u16, u16) = (1, 20);
    pub const FILLING_PRESSURE_BAR: (f32, f32, f32) = (0.3, 2.0, 0.1);
    /// (min, max, pas) en pourcents.
    pub const FILLING_PUMP_PCT: (u8, u8, u8) = (20, 100, 5);
    pub const PREINFUSION_TIME_S_MAX: u16 = 20;
    pub const PREINFUSION_PUMP_PCT_MAX: u8 = 100;
    pub const RAMPDOWN_LEAD_TIME_S: (f32, f32, f32) = (0.0, 15.0, 0.5);
    pub const RAMPDOWN_LEAD_WEIGHT_G: (f32, f32, f32) = (0.0, 20.0, 0.5);
    pub const RAMPDOWN_PRESSURE_DROP_BAR: (f32, f32, f32) = (0.5, 4.0, 0.5);
    pub const BREW_PUMP_PCT: (u8, u8, u8) = (super::MINIMUM_BREW_PUMP_PCT, 100, 5);
    pub const PURGE_PUMP_PCT: (u8, u8, u8) = (20, 100, 5);
    pub const PURGE_MAX_S: (u16, u16, u16) = (5, 60, 5);
    pub const DIM_AFTER_S: (u16, u16, u16) = (60, 1800, 60);
    pub const STANDBY_AFTER_S: (u16, u16, u16) = (300, 3600, 300);
}

/// Configuration utilisateur (schéma 10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    pub version: u16,
    pub revision: u32,
    pub target_weight_g: f32,
    pub target_time_s: u16,
    pub target_pressure_bar: f32,
    pub brew_temperature_c: f32,
    pub heating_enabled: bool,
    pub brew_preheat_time_s: f32,
    /// Secours : à 75 % de pompe, 10 s admettent ≈ 34 ml, au-delà du volume
    /// à 1 bar de toutes les captures (23,5 à 30,7 ml).
    pub filling_time_s: u16,
    /// Pression absolue qui termine le remplissage : headspace plein et
    /// galette mouillée.
    pub filling_pressure_bar: f32,
    pub filling_pump_pct: u8,
    pub preinfusion_mode: PreinfusionMode,
    pub preinfusion_time_s: u16,
    pub preinfusion_pump_pct: u8,
    pub rampdown_mode: RampdownMode,
    pub rampdown_lead_time_s: f32,
    pub rampdown_lead_weight_g: f32,
    pub rampdown_pressure_drop_bar: f32,
    pub brew_pump_pct: u8,
    pub purge_pump_pct: u8,
    pub purge_max_s: u16,
    pub dim_after_s: u16,
    pub standby_after_s: u16,
    /// Recherche et connexion BLE de la sonde SCACE. Désactivée par défaut :
    /// la sonde ne sert qu'aux campagnes de mesure.
    pub scace_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_SCHEMA_VERSION,
            revision: 0,
            target_weight_g: 36.0,
            target_time_s: 28,
            target_pressure_bar: 9.0,
            brew_temperature_c: 90.0,
            heating_enabled: true,
            brew_preheat_time_s: 2.5,
            filling_time_s: 10,
            filling_pressure_bar: 1.0,
            filling_pump_pct: 100,
            preinfusion_mode: PreinfusionMode::TIME,
            preinfusion_time_s: 4,
            preinfusion_pump_pct: 35,
            rampdown_mode: RampdownMode::None,
            rampdown_lead_time_s: 3.0,
            rampdown_lead_weight_g: 4.0,
            rampdown_pressure_drop_bar: 1.0,
            brew_pump_pct: 100,
            purge_pump_pct: 100,
            purge_max_s: 20,
            dim_after_s: 240,
            standby_after_s: 1800,
            scace_enabled: false,
        }
    }
}

/// Les profils ne sont délibérément pas encodés dans le blob.
pub const PROFILES_SUPPORTED: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigStatus {
    Ok,
    VersionMismatch,
    StaleRevision,
    InvalidValue,
    StorageError,
    Busy,
}

/// Résultat d'une écriture de configuration ; `field` nomme la clé fautive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigResult {
    pub status: ConfigStatus,
    pub field: Option<&'static str>,
}

impl ConfigResult {
    pub const OK: Self = Self {
        status: ConfigStatus::Ok,
        field: None,
    };
    pub const fn new(status: ConfigStatus, field: &'static str) -> Self {
        Self {
            status,
            field: Some(field),
        }
    }
}

// --- Validation ------------------------------------------------------------

fn valid_step(value: f32, (min, max, step): (f32, f32, f32)) -> bool {
    if !value.is_finite() || value < min || value > max {
        return false;
    }
    let scaled = (value - min) / step;
    libm::fabsf(scaled - libm::roundf(scaled)) < 0.001
}

fn valid_pct(value: u8, (min, max, step): (u8, u8, u8)) -> bool {
    !(value < min || value > max || !value.is_multiple_of(step))
}

/// Premier champ invalide, `None` si la configuration est valide.
pub fn validate(c: &Config) -> Option<&'static str> {
    use bounds as b;
    if c.version != CONFIG_SCHEMA_VERSION {
        return Some("version");
    }
    if !valid_step(c.target_weight_g, b::TARGET_WEIGHT_G) {
        return Some("brew.target_weight_g");
    }
    if c.target_time_s < b::TARGET_TIME_S.0 || c.target_time_s > b::TARGET_TIME_S.1 {
        return Some("brew.target_time_s");
    }
    if !valid_step(c.target_pressure_bar, b::TARGET_PRESSURE_BAR) {
        return Some("brew.target_pressure_bar");
    }
    if !valid_step(c.brew_temperature_c, b::BREW_TEMPERATURE_C) {
        return Some("heating.brew_temperature_c");
    }
    if !valid_step(c.brew_preheat_time_s, b::BREW_PREHEAT_TIME_S) {
        return Some("heating.brew_preheat_time_s");
    }
    if c.filling_time_s < b::FILLING_TIME_S.0 || c.filling_time_s > b::FILLING_TIME_S.1 {
        return Some("filling.time_s");
    }
    if !valid_step(c.filling_pressure_bar, b::FILLING_PRESSURE_BAR) {
        return Some("filling.pressure_bar");
    }
    if !valid_pct(c.filling_pump_pct, b::FILLING_PUMP_PCT) {
        return Some("filling.pump_pct");
    }
    if (c.preinfusion_mode.0 & !PREINFUSION_MODE_MASK) != 0 {
        return Some("preinfusion.mode");
    }
    if c.preinfusion_time_s > b::PREINFUSION_TIME_S_MAX {
        return Some("preinfusion.time_s");
    }
    if c.preinfusion_pump_pct > b::PREINFUSION_PUMP_PCT_MAX
        || !c.preinfusion_pump_pct.is_multiple_of(5)
    {
        return Some("preinfusion.pump_pct");
    }
    if !valid_step(c.rampdown_lead_time_s, b::RAMPDOWN_LEAD_TIME_S) {
        return Some("rampdown.lead_time_s");
    }
    if !valid_step(c.rampdown_lead_weight_g, b::RAMPDOWN_LEAD_WEIGHT_G) {
        return Some("rampdown.lead_weight_g");
    }
    if !valid_step(c.rampdown_pressure_drop_bar, b::RAMPDOWN_PRESSURE_DROP_BAR) {
        return Some("rampdown.pressure_drop_bar");
    }
    if !valid_pct(c.brew_pump_pct, b::BREW_PUMP_PCT) {
        return Some("brew.pump_pct");
    }
    if !valid_pct(c.purge_pump_pct, b::PURGE_PUMP_PCT) {
        return Some("purge.pump_pct");
    }
    if !valid_pct16(c.purge_max_s, b::PURGE_MAX_S) {
        return Some("purge.max_s");
    }
    if !valid_pct16(c.dim_after_s, b::DIM_AFTER_S) {
        return Some("ui.dim_after_s");
    }
    if !valid_pct16(c.standby_after_s, b::STANDBY_AFTER_S) {
        return Some("ui.standby_after_s");
    }
    if c.standby_after_s < c.dim_after_s {
        return Some("ui.standby_after_s");
    }
    None
}

fn valid_pct16(value: u16, (min, max, step): (u16, u16, u16)) -> bool {
    !(value < min || value > max || !value.is_multiple_of(step))
}

// --- Disposition des blobs NVS ---------------------------------------------

const MAGIC: u32 = 0x4346_4731; // CFG1

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ty {
    U8,
    U16,
    U32,
    F32,
    Bool,
}

impl Ty {
    const fn size(self) -> usize {
        match self {
            Self::U8 | Self::Bool => 1,
            Self::U16 => 2,
            Self::U32 | Self::F32 => 4,
        }
    }
}

use Ty::{Bool, F32, U8, U16, U32};

/// Un champ d'un blob : nom (celui de `Config` pour le schéma courant),
/// type C.
type Field = (&'static str, Ty);

const V10: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("target_pressure_bar", F32),
    ("brew_temperature_c", F32),
    ("heating_enabled", Bool),
    ("brew_preheat_time_s", F32),
    ("filling_time_s", U16),
    ("filling_pressure_bar", F32),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
    ("scace_enabled", Bool),
];

// Le schéma 9 est le schéma 10 sans l'interrupteur de la sonde SCACE.
const V9: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("target_pressure_bar", F32),
    ("brew_temperature_c", F32),
    ("heating_enabled", Bool),
    ("brew_preheat_time_s", F32),
    ("filling_time_s", U16),
    ("filling_pressure_bar", F32),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
];

// Le schéma 8 : montée relative de pression au lieu de la cible absolue.
const V8: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("target_pressure_bar", F32),
    ("brew_temperature_c", F32),
    ("heating_enabled", Bool),
    ("brew_preheat_time_s", F32),
    ("filling_time_s", U16),
    ("filling_pressure_rise_bar", F32),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
];

// Le schéma 7 a encore la pression de pré-infusion et la cible absolue de
// remplissage.
const V7: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("target_pressure_bar", F32),
    ("brew_temperature_c", F32),
    ("heating_enabled", Bool),
    ("brew_preheat_time_s", F32),
    ("filling_time_s", U16),
    ("filling_pressure_target_bar", F32),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pressure_bar", F32),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
];

// Le schéma 6 n'a pas la précharge d'infusion.
const V6: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("target_pressure_bar", F32),
    ("brew_temperature_c", F32),
    ("heating_enabled", Bool),
    ("filling_time_s", U16),
    ("filling_pressure_target_bar", F32),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pressure_bar", F32),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
];

// Le schéma 5 n'a pas la chauffe.
const V5: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("target_pressure_bar", F32),
    ("filling_time_s", U16),
    ("filling_pressure_target_bar", F32),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pressure_bar", F32),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
];

// Le schéma 4 ne mémorisait pas la pression d'infusion.
const V4: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("filling_time_s", U16),
    ("filling_pressure_target_bar", F32),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pressure_bar", F32),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
];

// Le schéma 3 : écart relatif de pression au lieu de la cible absolue.
const V3: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("filling_time_s", U16),
    ("filling_pressure_delta_bar", F32),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pressure_bar", F32),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
];

const V2: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("target_weight_g", F32),
    ("target_time_s", U16),
    ("preinfusion_mode", U8),
    ("preinfusion_time_s", U16),
    ("preinfusion_pressure_bar", F32),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
    ("brew_pump_pct", U8),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
];

// Le schéma 1 a la même disposition que le 2 ; son mode (temps ou pression)
// devient toujours une échéance en temps.
const V1: &[Field] = V2;

/// Position des champs et taille d'un blob, comme le compilateur C les
/// dispose : alignement naturel, structure alignée sur 4.
struct Layout {
    offsets: Vec<usize>,
    /// Taille de la structure de configuration, complétée à 4.
    config_size: usize,
}

impl Layout {
    fn of(fields: &[Field]) -> Self {
        let mut offsets = Vec::with_capacity(fields.len());
        let mut at = 0usize;
        for (_, ty) in fields {
            let size = ty.size();
            at = at.div_ceil(size) * size;
            offsets.push(at);
            at += size;
        }
        Self {
            offsets,
            config_size: at.div_ceil(4) * 4,
        }
    }

    /// Octet du `magic` : 0 ; de la configuration : 4 ; de la somme de
    /// contrôle : juste après la configuration.
    fn checksum_offset(&self) -> usize {
        4 + self.config_size
    }

    fn blob_size(&self) -> usize {
        self.checksum_offset() + 4
    }
}

fn fnv1a(bytes: &[u8]) -> u32 {
    let mut result: u32 = 2_166_136_261;
    for &b in bytes {
        result = (result ^ u32::from(b)).wrapping_mul(16_777_619);
    }
    result
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Valeurs lues d'un blob, par nom de champ.
struct Raw {
    fields: &'static [Field],
    values: Vec<f64>,
}

impl Raw {
    fn get(&self, name: &str) -> Option<f64> {
        self.fields
            .iter()
            .position(|(n, _)| *n == name)
            .map(|i| self.values[i])
    }
}

/// Lit un blob de la forme `fields` ; `None` si la taille, la magie, la
/// version ou la somme de contrôle ne correspondent pas.
fn decode_blob(bytes: &[u8], fields: &'static [Field], version: u16) -> Option<Raw> {
    let layout = Layout::of(fields);
    if bytes.len() != layout.blob_size() || read_u32(bytes, 0) != MAGIC {
        return None;
    }
    if read_u32(bytes, layout.checksum_offset()) != fnv1a(&bytes[..layout.checksum_offset()]) {
        return None;
    }
    let mut values = Vec::with_capacity(fields.len());
    for ((_, ty), offset) in fields.iter().zip(&layout.offsets) {
        let at = 4 + offset;
        values.push(match ty {
            Ty::U8 | Ty::Bool => f64::from(bytes[at]),
            Ty::U16 => f64::from(u16::from_le_bytes([bytes[at], bytes[at + 1]])),
            Ty::U32 => f64::from(read_u32(bytes, at)),
            Ty::F32 => f64::from(f32::from_bits(read_u32(bytes, at))),
        });
    }
    let raw = Raw { fields, values };
    if raw.get("version") != Some(f64::from(version)) {
        return None;
    }
    Some(raw)
}

fn config_value(c: &Config, name: &str) -> f64 {
    match name {
        "version" => f64::from(c.version),
        "revision" => f64::from(c.revision),
        "target_weight_g" => f64::from(c.target_weight_g),
        "target_time_s" => f64::from(c.target_time_s),
        "target_pressure_bar" => f64::from(c.target_pressure_bar),
        "brew_temperature_c" => f64::from(c.brew_temperature_c),
        "heating_enabled" => f64::from(u8::from(c.heating_enabled)),
        "brew_preheat_time_s" => f64::from(c.brew_preheat_time_s),
        "filling_time_s" => f64::from(c.filling_time_s),
        "filling_pressure_bar" => f64::from(c.filling_pressure_bar),
        "filling_pump_pct" => f64::from(c.filling_pump_pct),
        "preinfusion_mode" => f64::from(c.preinfusion_mode.0),
        "preinfusion_time_s" => f64::from(c.preinfusion_time_s),
        "preinfusion_pump_pct" => f64::from(c.preinfusion_pump_pct),
        "rampdown_mode" => f64::from(c.rampdown_mode as u8),
        "rampdown_lead_time_s" => f64::from(c.rampdown_lead_time_s),
        "rampdown_lead_weight_g" => f64::from(c.rampdown_lead_weight_g),
        "rampdown_pressure_drop_bar" => f64::from(c.rampdown_pressure_drop_bar),
        "brew_pump_pct" => f64::from(c.brew_pump_pct),
        "purge_pump_pct" => f64::from(c.purge_pump_pct),
        "purge_max_s" => f64::from(c.purge_max_s),
        "dim_after_s" => f64::from(c.dim_after_s),
        "standby_after_s" => f64::from(c.standby_after_s),
        "scace_enabled" => f64::from(u8::from(c.scace_enabled)),
        _ => 0.0,
    }
}

/// Blob NVS du schéma 10 : magie, structure `Config` (avec le remplissage du
/// compilateur C, à zéro), somme de contrôle FNV-1a de tout ce qui précède.
pub fn encode_blob(config: &Config) -> Vec<u8> {
    let layout = Layout::of(V10);
    let mut out = vec![0u8; layout.blob_size()];
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    for ((name, ty), offset) in V10.iter().zip(&layout.offsets) {
        let at = 4 + offset;
        let value = config_value(config, name);
        match ty {
            Ty::U8 | Ty::Bool => out[at] = value as u8,
            Ty::U16 => out[at..at + 2].copy_from_slice(&(value as u16).to_le_bytes()),
            Ty::U32 => out[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes()),
            Ty::F32 => out[at..at + 4].copy_from_slice(&(value as f32).to_le_bytes()),
        }
    }
    let cs = fnv1a(&out[..layout.checksum_offset()]);
    let cs_at = layout.checksum_offset();
    out[cs_at..cs_at + 4].copy_from_slice(&cs.to_le_bytes());
    out
}

/// Taille du blob du schéma 10, en octets.
pub fn blob_size() -> usize {
    Layout::of(V10).blob_size()
}

/// La sortie par pression n'existe plus. Une pré-infusion qui n'avait que
/// celle-ci garde une échéance en temps plutôt que de disparaître.
fn migrate_preinfusion_mode(legacy: u8) -> PreinfusionMode {
    const LEGACY_PRESSURE_BIT: u8 = 1 << 1;
    let kept = legacy & PREINFUSION_MODE_MASK;
    if (legacy & LEGACY_PRESSURE_BIT) != 0 && kept == 0 {
        return PreinfusionMode::TIME;
    }
    PreinfusionMode(kept)
}

fn set_field(c: &mut Config, name: &str, v: f64) {
    match name {
        "revision" => c.revision = v as u32,
        "target_weight_g" => c.target_weight_g = v as f32,
        "target_time_s" => c.target_time_s = v as u16,
        "target_pressure_bar" => c.target_pressure_bar = v as f32,
        "brew_temperature_c" => c.brew_temperature_c = v as f32,
        "heating_enabled" => c.heating_enabled = v != 0.0,
        "brew_preheat_time_s" => c.brew_preheat_time_s = v as f32,
        "filling_time_s" => c.filling_time_s = v as u16,
        "filling_pressure_bar" => c.filling_pressure_bar = v as f32,
        "filling_pump_pct" => c.filling_pump_pct = v as u8,
        "preinfusion_time_s" => c.preinfusion_time_s = v as u16,
        "preinfusion_pump_pct" => c.preinfusion_pump_pct = v as u8,
        "rampdown_mode" => c.rampdown_mode = RampdownMode::from_u8(v as u8).unwrap_or_default(),
        "rampdown_lead_time_s" => c.rampdown_lead_time_s = v as f32,
        "rampdown_lead_weight_g" => c.rampdown_lead_weight_g = v as f32,
        "rampdown_pressure_drop_bar" => c.rampdown_pressure_drop_bar = v as f32,
        "brew_pump_pct" => c.brew_pump_pct = v as u8,
        "purge_pump_pct" => c.purge_pump_pct = v as u8,
        "purge_max_s" => c.purge_max_s = v as u16,
        "dim_after_s" => c.dim_after_s = v as u16,
        "standby_after_s" => c.standby_after_s = v as u16,
        "scace_enabled" => c.scace_enabled = v != 0.0,
        _ => {}
    }
}

/// Champs repris d'un ancien schéma, dans le sens du C++ : tout ce qui n'est
/// pas listé reprend sa valeur par défaut.
fn migrated_fields(version: u16) -> &'static [&'static str] {
    const FROM_V2: &[&str] = &[
        "revision",
        "target_weight_g",
        "target_time_s",
        "preinfusion_time_s",
        "preinfusion_pump_pct",
        "rampdown_mode",
        "rampdown_lead_time_s",
        "rampdown_lead_weight_g",
        "rampdown_pressure_drop_bar",
        "brew_pump_pct",
        "purge_pump_pct",
        "purge_max_s",
        "dim_after_s",
        "standby_after_s",
    ];
    const FROM_V3: &[&str] = &[
        "revision",
        "target_weight_g",
        "target_time_s",
        "filling_time_s",
        "filling_pump_pct",
        "preinfusion_time_s",
        "preinfusion_pump_pct",
        "rampdown_mode",
        "rampdown_lead_time_s",
        "rampdown_lead_weight_g",
        "rampdown_pressure_drop_bar",
        "brew_pump_pct",
        "purge_pump_pct",
        "purge_max_s",
        "dim_after_s",
        "standby_after_s",
    ];
    const FROM_V5: &[&str] = &[
        "revision",
        "target_weight_g",
        "target_time_s",
        "target_pressure_bar",
        "filling_time_s",
        "filling_pump_pct",
        "preinfusion_time_s",
        "preinfusion_pump_pct",
        "rampdown_mode",
        "rampdown_lead_time_s",
        "rampdown_lead_weight_g",
        "rampdown_pressure_drop_bar",
        "brew_pump_pct",
        "purge_pump_pct",
        "purge_max_s",
        "dim_after_s",
        "standby_after_s",
    ];
    const FROM_V6: &[&str] = &[
        "revision",
        "target_weight_g",
        "target_time_s",
        "target_pressure_bar",
        "brew_temperature_c",
        "heating_enabled",
        "filling_time_s",
        "filling_pump_pct",
        "preinfusion_time_s",
        "preinfusion_pump_pct",
        "rampdown_mode",
        "rampdown_lead_time_s",
        "rampdown_lead_weight_g",
        "rampdown_pressure_drop_bar",
        "brew_pump_pct",
        "purge_pump_pct",
        "purge_max_s",
        "dim_after_s",
        "standby_after_s",
    ];
    const FROM_V7: &[&str] = &[
        "revision",
        "target_weight_g",
        "target_time_s",
        "target_pressure_bar",
        "brew_temperature_c",
        "heating_enabled",
        "brew_preheat_time_s",
        "filling_time_s",
        "filling_pump_pct",
        "preinfusion_time_s",
        "preinfusion_pump_pct",
        "rampdown_mode",
        "rampdown_lead_time_s",
        "rampdown_lead_weight_g",
        "rampdown_pressure_drop_bar",
        "brew_pump_pct",
        "purge_pump_pct",
        "purge_max_s",
        "dim_after_s",
        "standby_after_s",
    ];
    // La montée relative du schéma 8 n'a pas d'équivalent absolu : la
    // pression reprend sa valeur par défaut. La durée change de rôle
    // (secours) et reprend elle aussi son défaut de 10 s.
    const FROM_V8: &[&str] = &[
        "revision",
        "target_weight_g",
        "target_time_s",
        "target_pressure_bar",
        "brew_temperature_c",
        "heating_enabled",
        "brew_preheat_time_s",
        "filling_pump_pct",
        "preinfusion_time_s",
        "preinfusion_pump_pct",
        "rampdown_mode",
        "rampdown_lead_time_s",
        "rampdown_lead_weight_g",
        "rampdown_pressure_drop_bar",
        "brew_pump_pct",
        "purge_pump_pct",
        "purge_max_s",
        "dim_after_s",
        "standby_after_s",
    ];
    const FROM_V9: &[&str] = &[
        "revision",
        "target_weight_g",
        "target_time_s",
        "target_pressure_bar",
        "brew_temperature_c",
        "heating_enabled",
        "brew_preheat_time_s",
        "filling_time_s",
        "filling_pressure_bar",
        "filling_pump_pct",
        "preinfusion_time_s",
        "preinfusion_pump_pct",
        "rampdown_mode",
        "rampdown_lead_time_s",
        "rampdown_lead_weight_g",
        "rampdown_pressure_drop_bar",
        "brew_pump_pct",
        "purge_pump_pct",
        "purge_max_s",
        "dim_after_s",
        "standby_after_s",
    ];
    match version {
        9 => FROM_V9,
        8 => FROM_V8,
        7 => FROM_V7,
        6 => FROM_V6,
        5 => FROM_V5,
        // Les schémas 3 et 4 ne mémorisaient pas la pression d'infusion : le
        // défaut s'applique.
        4 | 3 => FROM_V3,
        _ => FROM_V2,
    }
}

/// Convertit un blob d'un ancien schéma en configuration courante.
fn migrate(version: u16, raw: &Raw) -> Config {
    let mut migrated = Config::default();
    let names = migrated_fields(version);
    for name in names {
        if let Some(value) = raw.get(name) {
            set_field(&mut migrated, name, value);
        }
    }
    migrated.preinfusion_mode = match version {
        // Les deux modes du schéma 1 deviennent une échéance en temps : la
        // sortie par pression a été retirée au schéma 8.
        1 => PreinfusionMode::TIME,
        // Les schémas 8 et 9 ont déjà le codage courant.
        8 | 9 => PreinfusionMode(raw.get("preinfusion_mode").unwrap_or(1.0) as u8),
        _ => migrate_preinfusion_mode(raw.get("preinfusion_mode").unwrap_or(0.0) as u8),
    };
    migrated
}

// --- Stockage ----------------------------------------------------------------

/// Emplacement transactionnel : les écritures alternent pour qu'une coupure
/// ne perde jamais la dernière configuration valide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    A,
    B,
}

impl Slot {
    /// Clé NVS (espace de noms [`NVS_NAMESPACE`]).
    pub const fn key(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
        }
    }
}

pub const NVS_NAMESPACE: &str = "ui";

/// Accès synchrone aux deux blobs NVS.
///
/// `write_slot` doit attendre la validation durable avant de répondre : le
/// cœur ne publie la nouvelle configuration qu'après ce succès. L'adaptateur
/// matériel peut déléguer l'écriture à un service sur l'autre cœur, puis
/// attendre son acquittement avec les interruptions actives. Il ne doit pas
/// maintenir de section critique ni de verrou de flash durant cette attente.
/// Les modifications sont refusées pendant l'infusion et la purge.
pub trait ConfigStore {
    /// Blob complet de l'emplacement, `None` s'il n'existe pas.
    fn read_slot(&mut self, slot: Slot) -> Option<Vec<u8>>;
    /// Écrit et valide le blob ; `false` en cas d'erreur NVS.
    fn write_slot(&mut self, slot: Slot, blob: &[u8]) -> bool;
}

/// Stockage en mémoire (tests, simulateur).
#[derive(Debug, Clone, Default)]
pub struct MemoryConfigStore {
    pub a: Option<Vec<u8>>,
    pub b: Option<Vec<u8>>,
    /// Si vrai, toute écriture échoue.
    pub fail_writes: bool,
    pub writes: u32,
}

impl ConfigStore for MemoryConfigStore {
    fn read_slot(&mut self, slot: Slot) -> Option<Vec<u8>> {
        match slot {
            Slot::A => self.a.clone(),
            Slot::B => self.b.clone(),
        }
    }

    fn write_slot(&mut self, slot: Slot, blob: &[u8]) -> bool {
        if self.fail_writes {
            return false;
        }
        self.writes += 1;
        match slot {
            Slot::A => self.a = Some(blob.to_vec()),
            Slot::B => self.b = Some(blob.to_vec()),
        }
        true
    }
}

fn read_current(store: &mut dyn ConfigStore, slot: Slot) -> Option<Config> {
    let bytes = store.read_slot(slot)?;
    let raw = decode_blob(&bytes, V10, CONFIG_SCHEMA_VERSION)?;
    let mut c = Config::default();
    for (name, _) in V10 {
        let v = raw.get(name).unwrap_or(0.0);
        match *name {
            "version" => c.version = v as u16,
            "preinfusion_mode" => c.preinfusion_mode = PreinfusionMode(v as u8),
            _ => set_field(&mut c, name, v),
        }
    }
    Some(c)
}

fn persist(store: &mut dyn ConfigStore, config: &Config) -> bool {
    let a = read_current(store, Slot::A);
    let b = read_current(store, Slot::B);
    let slot = match (&a, &b) {
        (None, _) => Slot::A,
        (Some(a), Some(b)) if b.revision > a.revision => Slot::A,
        _ => Slot::B,
    };
    store.write_slot(slot, &encode_blob(config))
}

/// Ce que [`ConfigManager::init`] a trouvé et décidé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LoadReport {
    /// Une configuration valide a été lue (courante ou migrée).
    pub found: bool,
    /// Elle venait d'un ancien schéma.
    pub migrated: bool,
    /// Le chauffage a été désactivé : une machine mise à jour ne doit pas
    /// chauffer avant un choix explicite.
    pub heating_disabled_by_migration: bool,
    /// La pompe d'infusion ou la consigne de température ont été ramenées
    /// dans la plage.
    pub normalized: bool,
    /// Une écriture de la configuration retenue a échoué.
    pub persist_failed: bool,
}

/// Configuration courante et sa persistance.
#[derive(Debug, Clone)]
pub struct ConfigManager {
    current: Config,
}

impl ConfigManager {
    /// Charge les deux emplacements : schéma courant d'abord, puis chaque
    /// ancien schéma du plus récent au plus ancien ; le premier qui a un
    /// emplacement valide gagne. Sans rien de valide, les défauts sont
    /// écrits avec la révision 1.
    pub fn init(store: &mut dyn ConfigStore) -> (Self, LoadReport) {
        let mut report = LoadReport::default();
        let mut selected = Config::default();

        let a = read_current(store, Slot::A);
        let b = read_current(store, Slot::B);
        match (a, b) {
            (Some(a), Some(b)) => {
                selected = if a.revision >= b.revision { a } else { b };
                report.found = true;
            }
            (Some(c), None) | (None, Some(c)) => {
                selected = c;
                report.found = true;
            }
            (None, None) => {}
        }

        let mut require_heating_confirmation = false;
        if !report.found {
            const LEGACY: &[(u16, &[Field])] = &[
                (9, V9),
                (8, V8),
                (7, V7),
                (6, V6),
                (5, V5),
                (4, V4),
                (3, V3),
                (2, V2),
                (1, V1),
            ];
            for &(version, fields) in LEGACY {
                let a = store
                    .read_slot(Slot::A)
                    .and_then(|bytes| decode_blob(&bytes, fields, version));
                let b = store
                    .read_slot(Slot::B)
                    .and_then(|bytes| decode_blob(&bytes, fields, version));
                let revision = |r: &Raw| r.get("revision").unwrap_or(0.0) as u32;
                let chosen = match (&a, &b) {
                    (Some(a), Some(b)) => Some(if revision(a) >= revision(b) { a } else { b }),
                    (Some(r), None) | (None, Some(r)) => Some(r),
                    (None, None) => None,
                };
                if let Some(raw) = chosen {
                    selected = migrate(version, raw);
                    report.found = true;
                    report.migrated = true;
                    require_heating_confirmation = version <= 5;
                    break;
                }
            }
        }

        // Une machine mise à jour ne doit pas chauffer avant un choix
        // explicite. Une installation neuve conserve le défaut `true`.
        if require_heating_confirmation {
            selected.heating_enabled = false;
            report.heating_disabled_by_migration = true;
        }
        let normalized_brew_pump = selected.brew_pump_pct < MINIMUM_BREW_PUMP_PCT;
        if normalized_brew_pump {
            selected.brew_pump_pct = MINIMUM_BREW_PUMP_PCT;
        }
        // Un essai temporaire a autorisé 110 °C avec le même schéma NVS :
        // ramener une consigne sauvegardée dans la plage rétablie.
        let normalized_brew_temperature = selected.brew_temperature_c > MAXIMUM_BREW_TEMPERATURE_C;
        if normalized_brew_temperature {
            selected.brew_temperature_c = MAXIMUM_BREW_TEMPERATURE_C;
        }
        report.normalized = report.found && (normalized_brew_pump || normalized_brew_temperature);
        if (report.migrated || report.normalized) && !persist(store, &selected) {
            report.persist_failed = true;
        }
        if !report.found {
            selected = Config::default();
            selected.revision = 1;
            if !persist(store, &selected) {
                report.persist_failed = true;
            }
        }
        (Self { current: selected }, report)
    }

    /// Gestionnaire sans stockage (simulateur) : défauts, révision 1.
    pub fn with_defaults() -> Self {
        Self {
            current: Config {
                revision: 1,
                ..Config::default()
            },
        }
    }

    pub fn get(&self) -> Config {
        self.current
    }

    /// Valide, numérote (`révision + 1`), écrit puis remplace la
    /// configuration courante, tout ou rien.
    pub fn apply(
        &mut self,
        store: &mut dyn ConfigStore,
        candidate: &Config,
        expected_revision: u32,
    ) -> ConfigResult {
        let invalid = validate(candidate);
        if candidate.version != CONFIG_SCHEMA_VERSION {
            return ConfigResult {
                status: ConfigStatus::VersionMismatch,
                field: invalid,
            };
        }
        if let Some(field) = invalid {
            return ConfigResult::new(ConfigStatus::InvalidValue, field);
        }
        if self.current.revision != expected_revision {
            return ConfigResult::new(ConfigStatus::StaleRevision, "revision");
        }
        let mut next = *candidate;
        next.revision = self.current.revision + 1;
        if !persist(store, &next) {
            return ConfigResult::new(ConfigStatus::StorageError, "nvs");
        }
        self.current = next;
        ConfigResult::OK
    }

    /// Remet les valeurs par défaut (nouvelle révision).
    pub fn reset(&mut self, store: &mut dyn ConfigStore) -> ConfigResult {
        let revision = self.current.revision;
        self.apply(store, &Config::default(), revision)
    }
}

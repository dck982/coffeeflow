//! Configuration utilisateur persistante : valeurs par défaut, validation,
//! bornes, profils d'infusion, blob NVS (schéma 11) et migrations des
//! schémas 1 à 10.
//!
//! Les calibrations machine ne passent ni par NVS ni par `/config` : elles
//! sont compilées dans [`crate::calibration`].
//!
//! Le stockage est abstrait par [`ConfigStore`] : deux emplacements
//! transactionnels `a` et `b` (blobs d'une centaine d'octets, espace de noms
//! NVS `ui`, de l'ordre de 500 octets), écrits en alternance. L'écriture est synchrone parce que
//! `/config` doit pouvoir répondre `500 storage` sans avoir rien modifié.

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::BitOr;

pub const CONFIG_SCHEMA_VERSION: u16 = 11;
pub const MINIMUM_BREW_TEMPERATURE_C: f32 = 50.0;
/// 5 °C sous la coupure de sécurité de la chaudière.
pub const MAXIMUM_BREW_TEMPERATURE_C: f32 = 95.0;
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
    pub const TARGET_PRESSURE_BAR: (f32, f32, f32) = (4.0, 12.0, 0.1);
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
    /// Dose d'un profil : 0 (non renseignée) ou (min, max, pas).
    pub const PROFILE_DOSE_G: (f32, f32, f32) = (3.0, 30.0, 0.1);
}

/// Nombre maximal de profils.
pub const MAX_PROFILES: usize = 4;
/// Taille des tampons UTF-8 du nom et du panier d'un profil.
pub const PROFILE_TEXT_BYTES: usize = 32;
/// Longueurs admises, en caractères.
pub const PROFILE_NAME_MAX_CHARS: usize = 20;
pub const PROFILE_BASKET_MAX_CHARS: usize = 24;

/// Profil d'infusion : une recette (nom, dose, panier) et les réglages que la
/// machine applique à l'infusion. Chaque profil porte toutes ses valeurs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Profile {
    pub name: [u8; PROFILE_TEXT_BYTES],
    /// Longueur de `name` en octets.
    pub name_len: u8,
    pub basket: [u8; PROFILE_TEXT_BYTES],
    /// Longueur de `basket` en octets.
    pub basket_len: u8,
    /// 0 : non renseignée.
    pub dose_g: f32,
    pub target_weight_g: f32,
    pub target_time_s: u16,
    pub target_pressure_bar: f32,
    pub brew_pump_pct: u8,
    pub brew_temperature_c: f32,
    pub brew_preheat_time_s: f32,
    pub filling_time_s: u16,
    pub filling_pressure_bar: f32,
    pub filling_pump_pct: u8,
    pub preinfusion_mode: PreinfusionMode,
    pub preinfusion_time_s: u16,
    pub preinfusion_pump_pct: u8,
    pub rampdown_mode: RampdownMode,
    pub rampdown_lead_time_s: f32,
    pub rampdown_lead_weight_g: f32,
    pub rampdown_pressure_drop_bar: f32,
}

impl Profile {
    /// Emplacement inutilisé : tout à zéro.
    pub const EMPTY: Self = Self {
        name: [0; PROFILE_TEXT_BYTES],
        name_len: 0,
        basket: [0; PROFILE_TEXT_BYTES],
        basket_len: 0,
        dose_g: 0.0,
        target_weight_g: 0.0,
        target_time_s: 0,
        target_pressure_bar: 0.0,
        brew_pump_pct: 0,
        brew_temperature_c: 0.0,
        brew_preheat_time_s: 0.0,
        filling_time_s: 0,
        filling_pressure_bar: 0.0,
        filling_pump_pct: 0,
        preinfusion_mode: PreinfusionMode::NONE,
        preinfusion_time_s: 0,
        preinfusion_pump_pct: 0,
        rampdown_mode: RampdownMode::None,
        rampdown_lead_time_s: 0.0,
        rampdown_lead_weight_g: 0.0,
        rampdown_pressure_drop_bar: 0.0,
    };

    /// Profil dont les réglages sont ceux des champs plats de `c`.
    pub fn from_flat(c: &Config, name: &str, dose_g: f32, basket: &str) -> Self {
        let mut profile = Self::EMPTY;
        profile.set_name(name);
        profile.set_basket(basket);
        profile.dose_g = dose_g;
        profile.take_settings(c);
        profile
    }

    pub fn name(&self) -> &str {
        text_of(&self.name, self.name_len)
    }

    pub fn basket(&self) -> &str {
        text_of(&self.basket, self.basket_len)
    }

    /// Écrit le nom ; `false` (rien n'est écrit) s'il dépasse le tampon.
    pub fn set_name(&mut self, name: &str) -> bool {
        set_text(&mut self.name, &mut self.name_len, name)
    }

    /// Écrit le panier ; `false` (rien n'est écrit) s'il dépasse le tampon.
    pub fn set_basket(&mut self, basket: &str) -> bool {
        set_text(&mut self.basket, &mut self.basket_len, basket)
    }

    /// Copie les réglages des champs plats de `c` dans ce profil ; la recette
    /// reste.
    pub fn take_settings(&mut self, c: &Config) {
        self.target_weight_g = c.target_weight_g;
        self.target_time_s = c.target_time_s;
        self.target_pressure_bar = c.target_pressure_bar;
        self.brew_pump_pct = c.brew_pump_pct;
        self.brew_temperature_c = c.brew_temperature_c;
        self.brew_preheat_time_s = c.brew_preheat_time_s;
        self.filling_time_s = c.filling_time_s;
        self.filling_pressure_bar = c.filling_pressure_bar;
        self.filling_pump_pct = c.filling_pump_pct;
        self.preinfusion_mode = c.preinfusion_mode;
        self.preinfusion_time_s = c.preinfusion_time_s;
        self.preinfusion_pump_pct = c.preinfusion_pump_pct;
        self.rampdown_mode = c.rampdown_mode;
        self.rampdown_lead_time_s = c.rampdown_lead_time_s;
        self.rampdown_lead_weight_g = c.rampdown_lead_weight_g;
        self.rampdown_pressure_drop_bar = c.rampdown_pressure_drop_bar;
    }

    /// Copie les réglages de ce profil dans les champs plats de `c`.
    pub fn give_settings(&self, c: &mut Config) {
        c.target_weight_g = self.target_weight_g;
        c.target_time_s = self.target_time_s;
        c.target_pressure_bar = self.target_pressure_bar;
        c.brew_pump_pct = self.brew_pump_pct;
        c.brew_temperature_c = self.brew_temperature_c;
        c.brew_preheat_time_s = self.brew_preheat_time_s;
        c.filling_time_s = self.filling_time_s;
        c.filling_pressure_bar = self.filling_pressure_bar;
        c.filling_pump_pct = self.filling_pump_pct;
        c.preinfusion_mode = self.preinfusion_mode;
        c.preinfusion_time_s = self.preinfusion_time_s;
        c.preinfusion_pump_pct = self.preinfusion_pump_pct;
        c.rampdown_mode = self.rampdown_mode;
        c.rampdown_lead_time_s = self.rampdown_lead_time_s;
        c.rampdown_lead_weight_g = self.rampdown_lead_weight_g;
        c.rampdown_pressure_drop_bar = self.rampdown_pressure_drop_bar;
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self::EMPTY
    }
}

fn text_of(buffer: &[u8; PROFILE_TEXT_BYTES], len: u8) -> &str {
    let len = usize::from(len).min(PROFILE_TEXT_BYTES);
    core::str::from_utf8(&buffer[..len]).unwrap_or("")
}

fn set_text(buffer: &mut [u8; PROFILE_TEXT_BYTES], len: &mut u8, text: &str) -> bool {
    if text.len() > PROFILE_TEXT_BYTES {
        return false;
    }
    *buffer = [0; PROFILE_TEXT_BYTES];
    buffer[..text.len()].copy_from_slice(text.as_bytes());
    *len = text.len() as u8;
    true
}

/// Nom du profil unique issu d'une migration ou d'une NVS vierge.
const DEFAULT_PROFILE_NAME: &str = "single espresso";
const DEFAULT_PROFILE_DOSE_G: f32 = 8.0;
const DEFAULT_PROFILE_BASKET: &str = "LM1 41 mm";

/// Configuration effective (schéma 11) : la machine et le profil actif. Les
/// champs plats « profil » (cible, température, remplissage, pré-infusion,
/// rampe) sont toujours égaux à `profiles[active_profile]` ; toute écriture
/// acceptée rétablit cet invariant.
#[derive(Debug, Clone, PartialEq)]
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
    /// Emplacements au-delà de `profile_count` : à zéro. Hors de la pile
    /// (`Arc`, copie à l'écriture) : une `Config` reste petite à copier.
    pub profiles: Arc<[Profile; MAX_PROFILES]>,
    /// De 1 à [`MAX_PROFILES`].
    pub profile_count: u8,
    /// Indice dans `profiles`, inférieur à `profile_count`.
    pub active_profile: u8,
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Self {
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
            profiles: Arc::new([Profile::EMPTY; MAX_PROFILES]),
            profile_count: 1,
            active_profile: 0,
        };
        config.adopt_flat_as_single_profile();
        config
    }
}

impl Config {
    /// Les champs plats deviennent le profil unique et actif : celui des
    /// migrations et d'une NVS vierge.
    fn adopt_flat_as_single_profile(&mut self) {
        let mut profiles = [Profile::EMPTY; MAX_PROFILES];
        profiles[0] = Profile::from_flat(
            self,
            DEFAULT_PROFILE_NAME,
            DEFAULT_PROFILE_DOSE_G,
            DEFAULT_PROFILE_BASKET,
        );
        self.profiles = Arc::new(profiles);
        self.profile_count = 1;
        self.active_profile = 0;
    }

    /// Profils en service.
    pub fn profiles(&self) -> &[Profile] {
        &self.profiles[..usize::from(self.profile_count).min(MAX_PROFILES)]
    }

    /// Profils en écriture : copie à l'écriture si le tableau est partagé.
    pub fn profiles_mut(&mut self) -> &mut [Profile; MAX_PROFILES] {
        Arc::make_mut(&mut self.profiles)
    }

    /// Profil actif ; le premier si l'indice est hors plage.
    pub fn active(&self) -> &Profile {
        self.profiles
            .get(usize::from(self.active_profile))
            .unwrap_or(&self.profiles[0])
    }

    /// Recopie le profil actif dans les champs plats.
    pub(crate) fn load_active(&mut self) {
        let profile = *self.active();
        profile.give_settings(self);
    }

    /// Rétablit l'invariant « champs plats = profil actif » sur une
    /// configuration proposée : si le profil actif est celui de
    /// `previous_active`, les champs plats modifiés s'écrivent dans le
    /// profil ; sinon le choix de profil l'emporte. Les emplacements au-delà
    /// de `profile_count` sont remis à zéro. Un indice ou un nombre hors
    /// plage est laissé à [`validate`].
    fn reconcile_profiles(&mut self, previous_active: u8) {
        let count = usize::from(self.profile_count);
        if count == 0 || count > MAX_PROFILES || self.active_profile >= self.profile_count {
            return;
        }
        if self.active_profile == previous_active {
            self.store_active();
        } else {
            self.load_active();
        }
        if self.profiles[count..].iter().any(|p| *p != Profile::EMPTY) {
            for profile in &mut self.profiles_mut()[count..] {
                *profile = Profile::EMPTY;
            }
        }
    }

    /// Écrit les champs plats dans le profil actif.
    pub(crate) fn store_active(&mut self) {
        let index = usize::from(self.active_profile);
        let Some(mut profile) = self.profiles.get(index).copied() else {
            return;
        };
        profile.take_settings(self);
        if self.profiles[index] != profile {
            self.profiles_mut()[index] = profile;
        }
    }
}

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

/// Premier réglage invalide d'un profil (nom, panier et dose exclus), `None`
/// s'ils sont tous valides.
fn validate_settings(c: &Profile) -> Option<&'static str> {
    use bounds as b;
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
    None
}

/// Premier champ plat invalide, `None` s'ils sont tous valides.
fn validate_flat(c: &Config) -> Option<&'static str> {
    use bounds as b;
    if let Some(field) = validate_settings(&Profile::from_flat(c, "", 0.0, "")) {
        return Some(field);
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

/// Noms des champs d'un profil dans les refus, un tableau par indice : les
/// noms sont `'static`, comme ceux du reste de la validation.
macro_rules! profile_field_names {
    ($($field:literal),* $(,)?) => {
        pub(crate) const PROFILE_FIELD_NAMES: &[(&str, [&str; MAX_PROFILES])] = &[
            $((
                $field,
                [
                    concat!("profiles[0].", $field),
                    concat!("profiles[1].", $field),
                    concat!("profiles[2].", $field),
                    concat!("profiles[3].", $field),
                ],
            )),*
        ];
    };
}

profile_field_names!(
    "name",
    "basket",
    "dose_g",
    "brew.target_weight_g",
    "brew.target_time_s",
    "brew.target_pressure_bar",
    "brew.pump_pct",
    "heating.brew_temperature_c",
    "heating.brew_preheat_time_s",
    "filling.time_s",
    "filling.pressure_bar",
    "filling.pump_pct",
    "preinfusion.mode",
    "preinfusion.time_s",
    "preinfusion.pump_pct",
    "rampdown.mode",
    "rampdown.lead_time_s",
    "rampdown.lead_weight_g",
    "rampdown.pressure_drop_bar",
);

/// Nom d'un champ du profil `index` dans un refus (`profiles[2].name`).
pub fn profile_field(index: usize, field: &str) -> &'static str {
    PROFILE_FIELD_NAMES
        .iter()
        .find(|(name, _)| *name == field)
        .and_then(|(_, names)| names.get(index))
        .copied()
        .unwrap_or("profiles")
}

/// Caractères admis dans un nom ou un panier : ASCII imprimable et accents
/// français.
fn allowed_char(c: char) -> bool {
    matches!(c, ' '..='~') || "°µéèêëàâäçîïôöùûüÿœæÉÈÀÇŒ".contains(c)
}

/// Texte de profil valide : de `min` à `max` caractères admis, tenant dans le
/// tampon, sans espace en tête ni en fin.
fn valid_profile_text(text: &str, min: usize, max: usize) -> bool {
    let chars = text.chars().count();
    chars >= min
        && chars <= max
        && text.len() <= PROFILE_TEXT_BYTES
        && text.chars().all(allowed_char)
        && !text.starts_with(' ')
        && !text.ends_with(' ')
}

/// Premier champ invalide d'un profil, `None` s'il est valide. Les unicités
/// se vérifient entre profils, dans [`validate`].
fn validate_profile(index: usize, p: &Profile) -> Option<&'static str> {
    if usize::from(p.name_len) > PROFILE_TEXT_BYTES
        || !valid_profile_text(p.name(), 1, PROFILE_NAME_MAX_CHARS)
    {
        return Some(profile_field(index, "name"));
    }
    if usize::from(p.basket_len) > PROFILE_TEXT_BYTES
        || !valid_profile_text(p.basket(), 0, PROFILE_BASKET_MAX_CHARS)
    {
        return Some(profile_field(index, "basket"));
    }
    if p.dose_g != 0.0 && !valid_step(p.dose_g, bounds::PROFILE_DOSE_G) {
        return Some(profile_field(index, "dose_g"));
    }
    validate_settings(p).map(|field| profile_field(index, field))
}

/// Premier champ invalide, `None` si la configuration est valide.
pub fn validate(c: &Config) -> Option<&'static str> {
    if c.version != CONFIG_SCHEMA_VERSION {
        return Some("version");
    }
    if let Some(field) = validate_flat(c) {
        return Some(field);
    }
    let count = usize::from(c.profile_count);
    if count == 0 || count > MAX_PROFILES {
        return Some("profiles");
    }
    if usize::from(c.active_profile) >= count {
        return Some("active_profile");
    }
    validate_profile_list(&c.profiles[..count])
}

/// Premier champ invalide d'une liste de profils : chaque profil, puis
/// l'unicité des noms (le second nom identique est fautif).
pub fn validate_profile_list(profiles: &[Profile]) -> Option<&'static str> {
    for (index, profile) in profiles.iter().enumerate() {
        if let Some(field) = validate_profile(index, profile) {
            return Some(field);
        }
        if profiles[..index]
            .iter()
            .any(|other| other.name() == profile.name())
        {
            return Some(profile_field(index, "name"));
        }
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

// Le schéma 11 ne garde dans la structure que la machine et le choix du
// profil ; les réglages d'infusion passent dans les enregistrements de
// profil qui suivent (voir `PROFILE_FIELDS`).
const V11: &[Field] = &[
    ("version", U16),
    ("revision", U32),
    ("heating_enabled", Bool),
    ("purge_pump_pct", U8),
    ("purge_max_s", U16),
    ("dim_after_s", U16),
    ("standby_after_s", U16),
    ("scace_enabled", Bool),
    ("active_profile", U8),
    ("profile_count", U8),
];

/// Un enregistrement de profil du schéma 11 : le nom puis le panier
/// (`PROFILE_TEXT_BYTES` octets chacun, complétés par des zéros), puis ces
/// champs. Quatre enregistrements suivent la structure de la machine ; ceux
/// au-delà de `profile_count` sont à zéro.
const PROFILE_FIELDS: &[Field] = &[
    ("name_len", U8),
    ("basket_len", U8),
    ("brew_pump_pct", U8),
    ("filling_pump_pct", U8),
    ("preinfusion_mode", U8),
    ("preinfusion_pump_pct", U8),
    ("rampdown_mode", U8),
    ("target_time_s", U16),
    ("filling_time_s", U16),
    ("preinfusion_time_s", U16),
    ("dose_g", F32),
    ("target_weight_g", F32),
    ("target_pressure_bar", F32),
    ("brew_temperature_c", F32),
    ("brew_preheat_time_s", F32),
    ("filling_pressure_bar", F32),
    ("rampdown_lead_time_s", F32),
    ("rampdown_lead_weight_g", F32),
    ("rampdown_pressure_drop_bar", F32),
];

// Le schéma 10 : un seul jeu de réglages, sans profils.
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
    /// Taille des champs, complétée à 4 : les enregistrements de profil
    /// commencent juste après.
    fields_size: usize,
    /// Taille de la structure de configuration : les champs puis les
    /// enregistrements de profil.
    config_size: usize,
}

/// Taille d'un enregistrement de profil dans le blob.
fn profile_record_size() -> usize {
    2 * PROFILE_TEXT_BYTES + Layout::of(PROFILE_FIELDS).fields_size
}

impl Layout {
    /// Disposition du schéma 11 : la machine et quatre enregistrements de
    /// profil.
    fn current() -> Self {
        let mut layout = Self::of(V11);
        layout.config_size += MAX_PROFILES * profile_record_size();
        layout
    }

    fn of(fields: &[Field]) -> Self {
        let mut offsets = Vec::with_capacity(fields.len());
        let mut at = 0usize;
        for (_, ty) in fields {
            let size = ty.size();
            at = at.div_ceil(size) * size;
            offsets.push(at);
            at += size;
        }
        let fields_size = at.div_ceil(4) * 4;
        Self {
            offsets,
            fields_size,
            config_size: fields_size,
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
fn decode_blob(
    bytes: &[u8],
    fields: &'static [Field],
    layout: &Layout,
    version: u16,
) -> Option<Raw> {
    if bytes.len() != layout.blob_size() || read_u32(bytes, 0) != MAGIC {
        return None;
    }
    if read_u32(bytes, layout.checksum_offset()) != fnv1a(&bytes[..layout.checksum_offset()]) {
        return None;
    }
    let mut values = Vec::with_capacity(fields.len());
    for ((_, ty), offset) in fields.iter().zip(&layout.offsets) {
        let at = 4 + offset;
        values.push(read_value(bytes, at, *ty));
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
        "active_profile" => f64::from(c.active_profile),
        "profile_count" => f64::from(c.profile_count),
        _ => 0.0,
    }
}

/// Valeur du champ `name` d'un enregistrement de profil.
fn profile_value(p: &Profile, name: &str) -> f64 {
    match name {
        "target_weight_g" => f64::from(p.target_weight_g),
        "target_time_s" => f64::from(p.target_time_s),
        "target_pressure_bar" => f64::from(p.target_pressure_bar),
        "brew_temperature_c" => f64::from(p.brew_temperature_c),
        "brew_preheat_time_s" => f64::from(p.brew_preheat_time_s),
        "filling_time_s" => f64::from(p.filling_time_s),
        "filling_pressure_bar" => f64::from(p.filling_pressure_bar),
        "filling_pump_pct" => f64::from(p.filling_pump_pct),
        "preinfusion_mode" => f64::from(p.preinfusion_mode.0),
        "preinfusion_time_s" => f64::from(p.preinfusion_time_s),
        "preinfusion_pump_pct" => f64::from(p.preinfusion_pump_pct),
        "rampdown_mode" => f64::from(p.rampdown_mode as u8),
        "rampdown_lead_time_s" => f64::from(p.rampdown_lead_time_s),
        "rampdown_lead_weight_g" => f64::from(p.rampdown_lead_weight_g),
        "rampdown_pressure_drop_bar" => f64::from(p.rampdown_pressure_drop_bar),
        "brew_pump_pct" => f64::from(p.brew_pump_pct),
        _ => 0.0,
    }
}

/// Écrit `value` à `at` selon `ty`.
fn write_value(out: &mut [u8], at: usize, ty: Ty, value: f64) {
    match ty {
        Ty::U8 | Ty::Bool => out[at] = value as u8,
        Ty::U16 => out[at..at + 2].copy_from_slice(&(value as u16).to_le_bytes()),
        Ty::U32 => out[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes()),
        Ty::F32 => out[at..at + 4].copy_from_slice(&(value as f32).to_le_bytes()),
    }
}

/// Lit la valeur de type `ty` à `at`.
fn read_value(bytes: &[u8], at: usize, ty: Ty) -> f64 {
    match ty {
        Ty::U8 | Ty::Bool => f64::from(bytes[at]),
        Ty::U16 => f64::from(u16::from_le_bytes([bytes[at], bytes[at + 1]])),
        Ty::U32 => f64::from(read_u32(bytes, at)),
        Ty::F32 => f64::from(f32::from_bits(read_u32(bytes, at))),
    }
}

/// Octets d'un enregistrement de profil (`size` octets, à zéro pour un
/// emplacement inutilisé).
fn encode_profile(profile: &Profile, used: bool, size: usize) -> Vec<u8> {
    let mut out = vec![0u8; size];
    if !used {
        return out;
    }
    out[..PROFILE_TEXT_BYTES].copy_from_slice(&profile.name);
    out[PROFILE_TEXT_BYTES..2 * PROFILE_TEXT_BYTES].copy_from_slice(&profile.basket);
    let layout = Layout::of(PROFILE_FIELDS);
    for ((name, ty), offset) in PROFILE_FIELDS.iter().zip(&layout.offsets) {
        let value = match *name {
            "name_len" => f64::from(profile.name_len),
            "basket_len" => f64::from(profile.basket_len),
            "dose_g" => f64::from(profile.dose_g),
            _ => profile_value(profile, name),
        };
        write_value(&mut out, 2 * PROFILE_TEXT_BYTES + offset, *ty, value);
    }
    out
}

/// Profil d'un enregistrement ; `None` si une longueur dépasse son tampon ou
/// si un texte n'est pas de l'UTF-8.
fn decode_profile(record: &[u8]) -> Option<Profile> {
    let layout = Layout::of(PROFILE_FIELDS);
    let mut profile = Profile::EMPTY;
    for ((name, ty), offset) in PROFILE_FIELDS.iter().zip(&layout.offsets) {
        let value = read_value(record, 2 * PROFILE_TEXT_BYTES + offset, *ty);
        match *name {
            "name_len" => profile.name_len = value as u8,
            "basket_len" => profile.basket_len = value as u8,
            "dose_g" => profile.dose_g = value as f32,
            _ => set_profile_field(&mut profile, name, value),
        }
    }
    profile.name.copy_from_slice(&record[..PROFILE_TEXT_BYTES]);
    profile
        .basket
        .copy_from_slice(&record[PROFILE_TEXT_BYTES..2 * PROFILE_TEXT_BYTES]);
    if usize::from(profile.name_len) > PROFILE_TEXT_BYTES
        || usize::from(profile.basket_len) > PROFILE_TEXT_BYTES
        || core::str::from_utf8(&profile.name[..usize::from(profile.name_len)]).is_err()
        || core::str::from_utf8(&profile.basket[..usize::from(profile.basket_len)]).is_err()
    {
        return None;
    }
    Some(profile)
}

/// Blob NVS du schéma 11 : magie, structure de la machine (avec le
/// remplissage du compilateur C, à zéro), quatre enregistrements de profil,
/// somme de contrôle FNV-1a de tout ce qui précède.
pub fn encode_blob(config: &Config) -> Vec<u8> {
    let layout = Layout::current();
    let mut out = vec![0u8; layout.blob_size()];
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    for ((name, ty), offset) in V11.iter().zip(&layout.offsets) {
        write_value(&mut out, 4 + offset, *ty, config_value(config, name));
    }
    let record_size = profile_record_size();
    for (index, profile) in config.profiles.iter().enumerate() {
        let used = index < usize::from(config.profile_count);
        let at = 4 + layout.fields_size + index * record_size;
        out[at..at + record_size].copy_from_slice(&encode_profile(profile, used, record_size));
    }
    let cs = fnv1a(&out[..layout.checksum_offset()]);
    let cs_at = layout.checksum_offset();
    out[cs_at..cs_at + 4].copy_from_slice(&cs.to_le_bytes());
    out
}

/// Taille du blob du schéma 11, en octets.
pub fn blob_size() -> usize {
    Layout::current().blob_size()
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

fn set_profile_field(p: &mut Profile, name: &str, v: f64) {
    match name {
        "target_weight_g" => p.target_weight_g = v as f32,
        "target_time_s" => p.target_time_s = v as u16,
        "target_pressure_bar" => p.target_pressure_bar = v as f32,
        "brew_temperature_c" => p.brew_temperature_c = v as f32,
        "brew_preheat_time_s" => p.brew_preheat_time_s = v as f32,
        "filling_time_s" => p.filling_time_s = v as u16,
        "filling_pressure_bar" => p.filling_pressure_bar = v as f32,
        "filling_pump_pct" => p.filling_pump_pct = v as u8,
        "preinfusion_mode" => p.preinfusion_mode = PreinfusionMode(v as u8),
        "preinfusion_time_s" => p.preinfusion_time_s = v as u16,
        "preinfusion_pump_pct" => p.preinfusion_pump_pct = v as u8,
        "rampdown_mode" => p.rampdown_mode = RampdownMode::from_u8(v as u8).unwrap_or_default(),
        "rampdown_lead_time_s" => p.rampdown_lead_time_s = v as f32,
        "rampdown_lead_weight_g" => p.rampdown_lead_weight_g = v as f32,
        "rampdown_pressure_drop_bar" => p.rampdown_pressure_drop_bar = v as f32,
        "brew_pump_pct" => p.brew_pump_pct = v as u8,
        _ => {}
    }
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
        "active_profile" => c.active_profile = v as u8,
        "profile_count" => c.profile_count = v as u8,
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
    // Le schéma 10 ajoute l'interrupteur de la sonde SCACE.
    const FROM_V10: &[&str] = &[
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
        "scace_enabled",
    ];
    match version {
        10 => FROM_V10,
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

/// Convertit un blob d'un ancien schéma (1 à 10) en configuration courante.
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
        // Les schémas 8 à 10 ont déjà le codage courant.
        8..=10 => PreinfusionMode(raw.get("preinfusion_mode").unwrap_or(1.0) as u8),
        _ => migrate_preinfusion_mode(raw.get("preinfusion_mode").unwrap_or(0.0) as u8),
    };
    // Les anciens schémas n'ont qu'un jeu de réglages : il devient l'unique
    // profil, actif.
    migrated.adopt_flat_as_single_profile();
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
    let layout = Layout::current();
    let raw = decode_blob(&bytes, V11, &layout, CONFIG_SCHEMA_VERSION)?;
    let mut c = Config::default();
    for (name, _) in V11 {
        let v = raw.get(name).unwrap_or(0.0);
        match *name {
            "version" => c.version = v as u16,
            _ => set_field(&mut c, name, v),
        }
    }
    if c.profile_count == 0
        || usize::from(c.profile_count) > MAX_PROFILES
        || c.active_profile >= c.profile_count
    {
        return None;
    }
    let record_size = profile_record_size();
    let mut profiles = [Profile::EMPTY; MAX_PROFILES];
    for index in 0..usize::from(c.profile_count) {
        let at = 4 + layout.fields_size + index * record_size;
        profiles[index] = decode_profile(&bytes[at..at + record_size])?;
    }
    c.profiles = Arc::new(profiles);
    c.load_active();
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
                (10, V10),
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
                    .and_then(|bytes| decode_blob(&bytes, fields, &Layout::of(fields), version));
                let b = store
                    .read_slot(Slot::B)
                    .and_then(|bytes| decode_blob(&bytes, fields, &Layout::of(fields), version));
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
        // Chaque profil est ramené dans la plage, puis les champs plats
        // reprennent le profil actif. Un essai temporaire a autorisé 110 °C,
        // et le maximum est passé de 100 à 95 °C, avec le même schéma NVS :
        // une consigne sauvegardée revient dans la plage actuelle.
        let mut normalized_brew_pump = false;
        let mut normalized_brew_temperature = false;
        let count = usize::from(selected.profile_count);
        for profile in &mut selected.profiles_mut()[..count] {
            if profile.brew_pump_pct < MINIMUM_BREW_PUMP_PCT {
                profile.brew_pump_pct = MINIMUM_BREW_PUMP_PCT;
                normalized_brew_pump = true;
            }
            if profile.brew_temperature_c > MAXIMUM_BREW_TEMPERATURE_C {
                profile.brew_temperature_c = MAXIMUM_BREW_TEMPERATURE_C;
                normalized_brew_temperature = true;
            }
        }
        selected.load_active();
        report.normalized = report.found && (normalized_brew_pump || normalized_brew_temperature);
        if (report.migrated || report.normalized) && !persist(store, &selected) {
            report.persist_failed = true;
        }
        if !report.found {
            selected = Config {
                revision: 1,
                ..Config::default()
            };
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

    pub fn get(&self) -> &Config {
        &self.current
    }

    /// Valide, numérote (`révision + 1`), écrit puis remplace la
    /// configuration courante, tout ou rien. L'invariant des profils est
    /// rétabli avant la validation : si le profil actif est celui en
    /// vigueur, les champs plats modifiés s'écrivent dans ce profil ; s'il
    /// change, ses réglages passent dans les champs plats.
    pub fn apply(
        &mut self,
        store: &mut dyn ConfigStore,
        candidate: &Config,
        expected_revision: u32,
    ) -> ConfigResult {
        let mut next = candidate.clone();
        next.reconcile_profiles(self.current.active_profile);
        let invalid = validate(&next);
        if next.version != CONFIG_SCHEMA_VERSION {
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
        next.revision = self.current.revision + 1;
        if !persist(store, &next) {
            return ConfigResult::new(ConfigStatus::StorageError, "nvs");
        }
        self.current = next;
        ConfigResult::OK
    }

    /// Rend actif le profil `index` (nouvelle révision), tout ou rien. Sans
    /// effet, ni écriture, si c'est déjà le profil actif.
    pub fn select_profile(&mut self, store: &mut dyn ConfigStore, index: u8) -> ConfigResult {
        if index >= self.current.profile_count {
            return ConfigResult::new(ConfigStatus::InvalidValue, "active_profile");
        }
        if index == self.current.active_profile {
            return ConfigResult::OK;
        }
        let mut next = self.current.clone();
        next.active_profile = index;
        next.load_active();
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

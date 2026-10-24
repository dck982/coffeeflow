//! Capture haute fréquence : tampon d'échantillons à 10 Hz tenu en RAM
//! volatile, et export JSON `coffeeflow.hf_capture.v3` de `/hf-capture`.
//!
//! Une capture commence au premier SET dimmer > 0 émis par le cœur, quelle
//! que soit sa porte d'entrée. Elle reste en RAM jusqu'à la session suivante
//! et n'est exportable qu'une fois l'arrêt confirmé par les capteurs.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::config::Profile;
use crate::config_json::encode_profile;
use crate::guided::Session;
use crate::json::format_g;

/// Période d'échantillonnage.
pub const PERIOD_MS: u16 = 100;
/// 115,2 s couvrent le réglage temporel maximal de 60 s, 15 s de précharge et
/// les 10 s de récupération conservées après une infusion, avec de la marge.
pub const CAPACITY: u16 = 1152;
/// Durée de récupération gardée après l'arrêt de la pompe d'une infusion.
pub const BREW_COOLDOWN_US: i64 = 10 * 1000 * 1000;
/// Idem pour les autres origines.
pub const OTHER_COOLDOWN_US: i64 = 5 * 1000 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Origin {
    #[default]
    SetActuators,
    Brew,
    Purge,
}

impl Origin {
    pub const fn text(self) -> &'static str {
        match self {
            Self::SetActuators => "set_actuators",
            Self::Brew => "brew",
            Self::Purge => "purge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleMode {
    #[default]
    Purge,
    Preinfusion,
    Infusion,
    RampDown,
    Cooldown,
    Filling,
    ThermalPreheat,
}

impl SampleMode {
    pub const fn text(self) -> &'static str {
        match self {
            Self::Purge => "purge",
            Self::Preinfusion => "preinfusion",
            Self::Infusion => "infusion",
            Self::RampDown => "ramp_down",
            Self::Cooldown => "cooldown",
            Self::Filling => "filling",
            Self::ThermalPreheat => "thermal_preheat",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Unavailable,
    Active,
    Complete,
}

/// Bits de [`Sample::flags`].
pub mod flags {
    pub const PRESSURE_VALID: u8 = 0x01;
    pub const FLOW_VALID: u8 = 0x02;
    pub const SCALE_PRESENT: u8 = 0x04;
    /// Chaudière valide et fraîche.
    pub const BOILER_VALID: u8 = 0x08;
    /// SSR chaudière actif.
    pub const HEATER_ON: u8 = 0x10;
    /// A2 lu.
    pub const PRESSURE_A2_READ: u8 = 0x20;
    pub const SCACE_VALID: u8 = 0x40;
}

/// Un échantillon de la capture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub t_ms: u32,
    pub pressure_raw: u32,
    pub xdb401_temperature_raw: u16,
    pub boiler_ntc_a0_raw: i16,
    pub boiler_ntc_a1_raw: i16,
    pub pressure_a2_raw: i16,
    /// Centièmes de °C, valable si le drapeau `SCACE_VALID` est levé.
    pub scace_centi_c: i16,
    pub boiler_temperature_age_ms: u32,
    pub flow_pulse_count: u32,
    pub flow_last_edge_age_ms: u32,
    pub pressure_bar: f32,
    pub xdb401_temperature_c: f32,
    pub boiler_temperature_c: f32,
    pub heating_power_pct: f32,
    pub volume_ml: f32,
    pub flow_ml_s: f32,
    pub weight_g: f32,
    pub pump_pct_commanded: u8,
    pub pump_pct_reported: u8,
    pub mode: SampleMode,
    pub flags: u8,
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            t_ms: 0,
            pressure_raw: 0,
            xdb401_temperature_raw: 0,
            boiler_ntc_a0_raw: 0,
            boiler_ntc_a1_raw: 0,
            pressure_a2_raw: 0,
            scace_centi_c: 0,
            boiler_temperature_age_ms: u32::MAX,
            flow_pulse_count: 0,
            flow_last_edge_age_ms: 0,
            pressure_bar: 0.0,
            xdb401_temperature_c: 0.0,
            boiler_temperature_c: 0.0,
            heating_power_pct: 0.0,
            volume_ml: 0.0,
            flow_ml_s: 0.0,
            weight_g: 0.0,
            pump_pct_commanded: 0,
            pump_pct_reported: 0,
            mode: SampleMode::Purge,
            flags: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Info {
    pub status: Status,
    pub origin: Origin,
    pub started_at_us: i64,
    pub ended_at_us: i64,
    pub started_at_unix_s: i64,
    pub ended_at_unix_s: i64,
    pub sample_period_ms: u16,
    pub capacity: u16,
    pub count: u16,
    pub dropped_samples: u16,
}

/// État interne de la capture (voir [`crate::core::Core`]).
#[derive(Debug, Clone)]
pub struct Capture {
    pub samples: Vec<Sample>,
    pub origin: Origin,
    pub started_at_us: i64,
    pub ended_at_us: i64,
    pub started_at_unix_s: i64,
    pub ended_at_unix_s: i64,
    pub next_sample_us: i64,
    pub cooldown_ends_at_us: i64,
    pub volume_ml_reference: f32,
    pub dropped_samples: u16,
    pub pump_pct_commanded: u8,
    pub active: bool,
    pub complete: bool,
    pub actuator_seen_on: bool,
    pub cooldown: bool,
    /// Profil actif au départ d'une infusion ; `None` pour les autres origines.
    pub profile: Option<Profile>,
}

impl Default for Capture {
    fn default() -> Self {
        Self::new()
    }
}

impl Capture {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            origin: Origin::SetActuators,
            started_at_us: 0,
            ended_at_us: 0,
            started_at_unix_s: 0,
            ended_at_unix_s: 0,
            next_sample_us: 0,
            cooldown_ends_at_us: 0,
            volume_ml_reference: 0.0,
            dropped_samples: 0,
            pump_pct_commanded: 0,
            active: false,
            complete: false,
            actuator_seen_on: false,
            cooldown: false,
            profile: None,
        }
    }

    pub fn info(&self) -> Info {
        Info {
            status: if self.active {
                Status::Active
            } else if self.complete {
                Status::Complete
            } else {
                Status::Unavailable
            },
            origin: self.origin,
            started_at_us: self.started_at_us,
            ended_at_us: self.ended_at_us,
            started_at_unix_s: self.started_at_unix_s,
            ended_at_unix_s: self.ended_at_unix_s,
            sample_period_ms: PERIOD_MS,
            capacity: CAPACITY,
            count: self.samples.len() as u16,
            dropped_samples: self.dropped_samples,
        }
    }
}

/// Vue exportée : brute, calibrée, ou les deux (défaut).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Raw,
    Calibrated,
    Both,
}

impl View {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "raw" => Some(Self::Raw),
            "calibrated" => Some(Self::Calibrated),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    pub const fn text(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Calibrated => "calibrated",
            Self::Both => "both",
        }
    }
}

/// Ce que l'en-tête ajoute à l'état de la capture : les versions des
/// firmwares et le profil de l'infusion.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Context {
    /// Version de l'écran (majeure, mineure, correctif).
    pub screen_version: (u8, u8, u8),
    /// Version des capteurs, `None` si elle n'est pas connue.
    pub sensors_version: Option<(u8, u8, u8)>,
    /// Profil copié au départ d'une infusion.
    pub profile: Option<Profile>,
}

fn version_text((major, minor, patch): (u8, u8, u8)) -> String {
    format!("\"{major}.{minor}.{patch}\"")
}

/// Début du document : tout jusqu'à `"samples":[` inclus. La clé `firmware`
/// donne les versions de l'écran et des capteurs (`null` si inconnue). Une
/// infusion porte ensuite `profile` (le profil actif au départ), puis `session` :
/// le déroulé du guide, ou `null` pour une infusion directe. Les autres
/// origines n'ont ni `profile` ni `session`.
pub fn export_header(
    info: &Info,
    view: View,
    session: Option<&Session>,
    context: &Context,
) -> String {
    let firmware = format!(
        "\"firmware\":{{\"screen\":{},\"sensors\":{}}},",
        version_text(context.screen_version),
        context
            .sensors_version
            .map_or_else(|| "null".to_string(), version_text)
    );
    let profile = match (info.origin, &context.profile) {
        (Origin::Brew, Some(profile)) => {
            format!("\"profile\":{},", encode_profile(profile).to_json_string())
        }
        _ => String::new(),
    };
    let session = match (info.origin, session) {
        (Origin::Brew, Some(session)) => {
            format!("\"session\":{},", session.to_json(info.started_at_us))
        }
        (Origin::Brew, None) => "\"session\":null,".to_string(),
        _ => String::new(),
    };
    format!(
        "{{\"schema\":\"coffeeflow.hf_capture.v3\",\"origin\":\"{}\",\"view\":\"{}\",\
\"started_at_us\":{},\"ended_at_us\":{},\"started_at_unix_s\":{},\
\"ended_at_unix_s\":{},\"sample_period_ms\":{},\"sample_count\":{},\
\"dropped_samples\":{},{firmware}{profile}{session}\"samples\":[",
        info.origin.text(),
        view.text(),
        info.started_at_us,
        info.ended_at_us,
        info.started_at_unix_s,
        info.ended_at_unix_s,
        info.sample_period_ms,
        info.count,
        info.dropped_samples
    )
}

/// Fin du document.
pub const EXPORT_FOOTER: &str = "]}";

fn g6(value: f32) -> String {
    format_g(f64::from(value), 6)
}

/// Un échantillon, précédé d'une virgule sauf le premier.
pub fn export_sample(sample: &Sample, view: View, first: bool) -> String {
    let scace_temperature = if sample.flags & flags::SCACE_VALID != 0 {
        format!("{:.2}", f64::from(sample.scace_centi_c) / 100.0)
    } else {
        "null".to_string()
    };
    let pressure_a2 = if sample.flags & flags::PRESSURE_A2_READ != 0 {
        sample.pressure_a2_raw.to_string()
    } else {
        "null".to_string()
    };
    let (boiler_temperature, boiler_age) = if sample.boiler_temperature_age_ms == u32::MAX {
        ("null".to_string(), "null".to_string())
    } else {
        (
            g6(sample.boiler_temperature_c),
            sample.boiler_temperature_age_ms.to_string(),
        )
    };
    let comma = if first { "" } else { "," };
    let boiler_valid = if sample.flags & flags::BOILER_VALID != 0 {
        "true"
    } else {
        "false"
    };
    let heater_on = if sample.flags & flags::HEATER_ON != 0 {
        "true"
    } else {
        "false"
    };
    let mode = sample.mode.text();
    match view {
        View::Raw => format!(
            "{comma}{{\"t_ms\":{},\"pressure_raw\":{},\"pressure_a2_raw\":{pressure_a2},\
\"xdb401_temperature_raw\":{},\"boiler_ntc_a0_raw\":{},\"boiler_ntc_a1_raw\":{},\
\"boiler_temperature_age_ms\":{boiler_age},\"boiler_temperature_valid\":{boiler_valid},\
\"flow_pulse_count\":{},\"flow_last_edge_age_ms\":{},\
\"pump_pct_commanded\":{},\"pump_pct_reported\":{},\"heating_power_pct\":{},\
\"heater_on\":{heater_on},\"mode\":\"{mode}\",\"flags\":{}}}",
            sample.t_ms,
            sample.pressure_raw,
            sample.xdb401_temperature_raw,
            sample.boiler_ntc_a0_raw,
            sample.boiler_ntc_a1_raw,
            sample.flow_pulse_count,
            sample.flow_last_edge_age_ms,
            sample.pump_pct_commanded,
            sample.pump_pct_reported,
            g6(sample.heating_power_pct),
            sample.flags
        ),
        View::Calibrated => format!(
            "{comma}{{\"t_ms\":{},\"pressure_bar\":{},\"xdb401_temperature_c\":{},\
\"boiler_temperature_c\":{boiler_temperature},\"boiler_temperature_age_ms\":{boiler_age},\
\"boiler_temperature_valid\":{boiler_valid},\"scace_temperature_c\":{scace_temperature},\
\"volume_ml\":{},\"flow_ml_s\":{},\"weight_g\":{},\
\"pump_pct_commanded\":{},\"pump_pct_reported\":{},\"heating_power_pct\":{},\
\"heater_on\":{heater_on},\"mode\":\"{mode}\",\"flags\":{}}}",
            sample.t_ms,
            g6(sample.pressure_bar),
            g6(sample.xdb401_temperature_c),
            g6(sample.volume_ml),
            g6(sample.flow_ml_s),
            g6(sample.weight_g),
            sample.pump_pct_commanded,
            sample.pump_pct_reported,
            g6(sample.heating_power_pct),
            sample.flags
        ),
        View::Both => format!(
            "{comma}{{\"t_ms\":{},\"pressure_raw\":{},\"pressure_a2_raw\":{pressure_a2},\
\"xdb401_temperature_raw\":{},\"boiler_ntc_a0_raw\":{},\"boiler_ntc_a1_raw\":{},\
\"boiler_temperature_age_ms\":{boiler_age},\"boiler_temperature_valid\":{boiler_valid},\
\"flow_pulse_count\":{},\"flow_last_edge_age_ms\":{},\
\"pressure_bar\":{},\"xdb401_temperature_c\":{},\
\"boiler_temperature_c\":{boiler_temperature},\"scace_temperature_c\":{scace_temperature},\"volume_ml\":{},\
\"flow_ml_s\":{},\"weight_g\":{},\"pump_pct_commanded\":{},\
\"pump_pct_reported\":{},\"heating_power_pct\":{},\"heater_on\":{heater_on},\
\"mode\":\"{mode}\",\"flags\":{}}}",
            sample.t_ms,
            sample.pressure_raw,
            sample.xdb401_temperature_raw,
            sample.boiler_ntc_a0_raw,
            sample.boiler_ntc_a1_raw,
            sample.flow_pulse_count,
            sample.flow_last_edge_age_ms,
            g6(sample.pressure_bar),
            g6(sample.xdb401_temperature_c),
            g6(sample.volume_ml),
            g6(sample.flow_ml_s),
            g6(sample.weight_g),
            sample.pump_pct_commanded,
            sample.pump_pct_reported,
            g6(sample.heating_power_pct),
            sample.flags
        ),
    }
}

/// Document complet.
pub fn export(
    info: &Info,
    samples: &[Sample],
    view: View,
    session: Option<&Session>,
    context: &Context,
) -> String {
    let mut out = export_header(info, view, session, context);
    for (i, sample) in samples.iter().enumerate() {
        out.push_str(&export_sample(sample, view, i == 0));
    }
    out.push_str(EXPORT_FOOTER);
    out
}

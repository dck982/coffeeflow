//! Document JSON de `/config` (schéma 10) : écriture de la configuration et
//! application d'un correctif partiel validé champ par champ.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::config::{Config, PreinfusionMode, RampdownMode};
use crate::json::Json;

/// Décimal de `/config` : deux décimales au plus, une au moins (`9.0`, `0.1`,
/// `0.05`).
fn config_decimal(value: f32) -> Json {
    let mut formatted = format!("{:.2}", f64::from(value));
    if formatted.len() > 1 && formatted.ends_with('0') {
        formatted.pop();
    }
    Json::Raw(formatted)
}

/// Document de `GET /config`, dans l'ordre d'écriture du firmware C++.
pub fn encode_config(config: &Config) -> Json {
    let mut root = Json::object();
    root.push_number("version", f64::from(config.version));

    let mut brew = Json::object();
    brew.push("target_weight_g", config_decimal(config.target_weight_g));
    brew.push_number("target_time_s", f64::from(config.target_time_s));
    brew.push(
        "target_pressure_bar",
        config_decimal(config.target_pressure_bar),
    );
    brew.push_number("pump_pct", f64::from(config.brew_pump_pct));
    root.push("brew", brew);

    let mut heating = Json::object();
    heating.push_bool("enabled", config.heating_enabled);
    heating.push(
        "brew_temperature_c",
        config_decimal(config.brew_temperature_c),
    );
    heating.push(
        "brew_preheat_time_s",
        config_decimal(config.brew_preheat_time_s),
    );
    root.push("heating", heating);

    let mut filling = Json::object();
    filling.push_number("time_s", f64::from(config.filling_time_s));
    filling.push("pressure_bar", config_decimal(config.filling_pressure_bar));
    filling.push_number("pump_pct", f64::from(config.filling_pump_pct));
    root.push("filling", filling);

    let mut preinfusion = Json::object();
    preinfusion.push_bool("time", config.preinfusion_mode.has(PreinfusionMode::TIME));
    preinfusion.push_bool(
        "weight",
        config.preinfusion_mode.has(PreinfusionMode::WEIGHT),
    );
    preinfusion.push_number("time_s", f64::from(config.preinfusion_time_s));
    preinfusion.push_number("pump_pct", f64::from(config.preinfusion_pump_pct));
    root.push("preinfusion", preinfusion);

    let mut rampdown = Json::object();
    rampdown.push_str("mode", config.rampdown_mode.text());
    rampdown.push("lead_time_s", config_decimal(config.rampdown_lead_time_s));
    rampdown.push(
        "lead_weight_g",
        config_decimal(config.rampdown_lead_weight_g),
    );
    rampdown.push(
        "pressure_drop_bar",
        config_decimal(config.rampdown_pressure_drop_bar),
    );
    root.push("rampdown", rampdown);

    let mut purge = Json::object();
    purge.push_number("pump_pct", f64::from(config.purge_pump_pct));
    purge.push_number("max_s", f64::from(config.purge_max_s));
    root.push("purge", purge);

    let mut ui = Json::object();
    ui.push_number("dim_after_s", f64::from(config.dim_after_s));
    ui.push_number("standby_after_s", f64::from(config.standby_after_s));
    root.push("ui", ui);

    let mut scace = Json::object();
    scace.push_bool("enabled", config.scace_enabled);
    root.push("scace", scace);

    root.push("profiles", Json::Array(Vec::new()));
    root
}

// --- Conversions de nombres ---------------------------------------------------

fn integer_in(node: &Json, max: f64) -> Option<f64> {
    let value = node.as_f64()?;
    (value.is_finite() && value == libm::floor(value) && value >= 0.0 && value <= max)
        .then_some(value)
}

fn as_u16(node: &Json) -> Option<u16> {
    integer_in(node, 65535.0).map(|v| v as u16)
}

fn as_u8(node: &Json) -> Option<u8> {
    integer_in(node, 255.0).map(|v| v as u8)
}

fn as_float(node: &Json) -> Option<f32> {
    let value = node.as_f64()?;
    value.is_finite().then_some(value as f32)
}

/// Nom de champ tronqué à 47 octets, comme le tampon de 48 du C++.
fn field_name(name: &str) -> String {
    let mut end = name.len().min(47);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name[..end].to_string()
}

fn joined(parent: &str, key: &str) -> String {
    field_name(&format!("{parent}.{key}"))
}

/// Applique le correctif `root` sur `config` (clés dans l'ordre du document).
/// En cas de refus, renvoie le nom du champ fautif.
pub fn apply_json_patch(root: &Json, config: &mut Config) -> Result<(), String> {
    if !root.is_object() {
        return Err("version".to_string());
    }
    let version = root.get("version").ok_or_else(|| "version".to_string())?;
    config.version = as_u16(version).ok_or_else(|| "version".to_string())?;

    let Json::Object(pairs) = root else {
        return Err("version".to_string());
    };
    for (key, value) in pairs {
        match key.as_str() {
            "version" => {}
            "brew" => overlay(value, "brew", config, apply_brew_key)?,
            "heating" => overlay(value, "heating", config, apply_heating_key)?,
            "filling" => overlay(value, "filling", config, apply_filling_key)?,
            "preinfusion" => overlay(value, "preinfusion", config, apply_preinfusion_key)?,
            "rampdown" => overlay(value, "rampdown", config, apply_rampdown_key)?,
            "purge" => overlay(value, "purge", config, apply_purge_key)?,
            "ui" => overlay(value, "ui", config, apply_ui_key)?,
            "scace" => overlay(value, "scace", config, apply_scace_key)?,
            "profiles" => match value {
                Json::Array(items) if items.is_empty() => {}
                _ => return Err("profiles".to_string()),
            },
            other => return Err(field_name(other)),
        }
    }
    Ok(())
}

type KeyApplier = fn(&str, &Json, &mut Config) -> Result<(), String>;

fn overlay(
    object: &Json,
    parent: &str,
    config: &mut Config,
    apply: KeyApplier,
) -> Result<(), String> {
    let Json::Object(pairs) = object else {
        return Err(parent.to_string());
    };
    for (key, value) in pairs {
        apply(key, value, config)?;
    }
    Ok(())
}

fn number_u16(value: &Json, field: &str, dest: &mut u16) -> Result<(), String> {
    *dest = as_u16(value).ok_or_else(|| field.to_string())?;
    Ok(())
}

fn number_u8(value: &Json, field: &str, dest: &mut u8) -> Result<(), String> {
    *dest = as_u8(value).ok_or_else(|| field.to_string())?;
    Ok(())
}

fn number_float(value: &Json, field: &str, dest: &mut f32) -> Result<(), String> {
    *dest = as_float(value).ok_or_else(|| field.to_string())?;
    Ok(())
}

fn boolean(value: &Json, field: &str, dest: &mut bool) -> Result<(), String> {
    *dest = value.as_bool().ok_or_else(|| field.to_string())?;
    Ok(())
}

fn apply_brew_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "target_weight_g" => number_float(value, "brew.target_weight_g", &mut c.target_weight_g),
        "target_time_s" => number_u16(value, "brew.target_time_s", &mut c.target_time_s),
        "target_pressure_bar" => number_float(
            value,
            "brew.target_pressure_bar",
            &mut c.target_pressure_bar,
        ),
        "pump_pct" => number_u8(value, "brew.pump_pct", &mut c.brew_pump_pct),
        _ => Err(joined("brew", key)),
    }
}

fn apply_heating_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "enabled" => boolean(value, "heating.enabled", &mut c.heating_enabled),
        "brew_temperature_c" => number_float(
            value,
            "heating.brew_temperature_c",
            &mut c.brew_temperature_c,
        ),
        "brew_preheat_time_s" => number_float(
            value,
            "heating.brew_preheat_time_s",
            &mut c.brew_preheat_time_s,
        ),
        _ => Err(joined("heating", key)),
    }
}

fn apply_preinfusion_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "time" | "weight" => {
            let enabled = value.as_bool().ok_or_else(|| joined("preinfusion", key))?;
            let bit = if key == "time" {
                PreinfusionMode::TIME.0
            } else {
                PreinfusionMode::WEIGHT.0
            };
            let modes = c.preinfusion_mode.0;
            c.preinfusion_mode = PreinfusionMode(if enabled { modes | bit } else { modes & !bit });
            Ok(())
        }
        "mode" => match value.as_str() {
            Some("time") => {
                c.preinfusion_mode = PreinfusionMode::TIME;
                Ok(())
            }
            _ => Err("preinfusion.mode".to_string()),
        },
        "time_s" => number_u16(value, "preinfusion.time_s", &mut c.preinfusion_time_s),
        "pump_pct" => number_u8(value, "preinfusion.pump_pct", &mut c.preinfusion_pump_pct),
        _ => Err(joined("preinfusion", key)),
    }
}

fn apply_filling_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "time_s" => number_u16(value, "filling.time_s", &mut c.filling_time_s),
        "pressure_bar" => number_float(value, "filling.pressure_bar", &mut c.filling_pressure_bar),
        "pump_pct" => number_u8(value, "filling.pump_pct", &mut c.filling_pump_pct),
        _ => Err(joined("filling", key)),
    }
}

fn apply_rampdown_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "mode" => {
            let Some(text) = value.as_str() else {
                return Err("rampdown.mode".to_string());
            };
            c.rampdown_mode = match text {
                "none" => RampdownMode::None,
                "time" => RampdownMode::Time,
                "weight" => RampdownMode::Weight,
                "pressure_drop" => RampdownMode::PressureDrop,
                _ => return Err("rampdown.mode".to_string()),
            };
            Ok(())
        }
        "lead_time_s" => number_float(value, "rampdown.lead_time_s", &mut c.rampdown_lead_time_s),
        "lead_weight_g" => number_float(
            value,
            "rampdown.lead_weight_g",
            &mut c.rampdown_lead_weight_g,
        ),
        "pressure_drop_bar" => number_float(
            value,
            "rampdown.pressure_drop_bar",
            &mut c.rampdown_pressure_drop_bar,
        ),
        _ => Err(joined("rampdown", key)),
    }
}

fn apply_purge_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "pump_pct" => number_u8(value, "purge.pump_pct", &mut c.purge_pump_pct),
        "max_s" => number_u16(value, "purge.max_s", &mut c.purge_max_s),
        _ => Err(joined("purge", key)),
    }
}

fn apply_ui_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "dim_after_s" => number_u16(value, "ui.dim_after_s", &mut c.dim_after_s),
        "standby_after_s" => number_u16(value, "ui.standby_after_s", &mut c.standby_after_s),
        _ => Err(joined("ui", key)),
    }
}

fn apply_scace_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "enabled" => boolean(value, "scace.enabled", &mut c.scace_enabled),
        _ => Err(joined("scace", key)),
    }
}

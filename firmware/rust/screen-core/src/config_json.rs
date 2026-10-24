//! Document JSON de `/config` (schéma 11) : écriture de la configuration et
//! application d'un correctif partiel validé champ par champ.

use alloc::format;
use alloc::string::{String, ToString};

use crate::config::{
    Config, MAX_PROFILES, PreinfusionMode, Profile, RampdownMode, validate_profile_list,
};
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

/// Section `brew` : les quatre réglages d'infusion du profil.
fn encode_brew(p: &Profile) -> Json {
    let mut brew = Json::object();
    brew.push("target_weight_g", config_decimal(p.target_weight_g));
    brew.push_number("target_time_s", f64::from(p.target_time_s));
    brew.push("target_pressure_bar", config_decimal(p.target_pressure_bar));
    brew.push_number("pump_pct", f64::from(p.brew_pump_pct));
    brew
}

fn encode_filling(p: &Profile) -> Json {
    let mut filling = Json::object();
    filling.push_number("time_s", f64::from(p.filling_time_s));
    filling.push("pressure_bar", config_decimal(p.filling_pressure_bar));
    filling.push_number("pump_pct", f64::from(p.filling_pump_pct));
    filling
}

fn encode_preinfusion(p: &Profile) -> Json {
    let mut preinfusion = Json::object();
    preinfusion.push_bool("time", p.preinfusion_mode.has(PreinfusionMode::TIME));
    preinfusion.push_bool("weight", p.preinfusion_mode.has(PreinfusionMode::WEIGHT));
    preinfusion.push_number("time_s", f64::from(p.preinfusion_time_s));
    preinfusion.push_number("pump_pct", f64::from(p.preinfusion_pump_pct));
    preinfusion
}

fn encode_rampdown(p: &Profile) -> Json {
    let mut rampdown = Json::object();
    rampdown.push_str("mode", p.rampdown_mode.text());
    rampdown.push("lead_time_s", config_decimal(p.rampdown_lead_time_s));
    rampdown.push("lead_weight_g", config_decimal(p.rampdown_lead_weight_g));
    rampdown.push(
        "pressure_drop_bar",
        config_decimal(p.rampdown_pressure_drop_bar),
    );
    rampdown
}

/// Un profil, tel qu'un élément de `profiles` dans `/config` et le profil
/// copié dans une capture HF.
pub fn encode_profile(p: &Profile) -> Json {
    let mut profile = Json::object();
    profile.push_str("name", p.name());
    profile.push("dose_g", config_decimal(p.dose_g));
    profile.push_str("basket", p.basket());
    profile.push("brew", encode_brew(p));
    let mut heating = Json::object();
    heating.push("brew_temperature_c", config_decimal(p.brew_temperature_c));
    heating.push("brew_preheat_time_s", config_decimal(p.brew_preheat_time_s));
    profile.push("heating", heating);
    profile.push("filling", encode_filling(p));
    profile.push("preinfusion", encode_preinfusion(p));
    profile.push("rampdown", encode_rampdown(p));
    profile
}

/// Document de `GET /config` : la configuration effective en tête (machine
/// et profil actif), puis la liste des profils.
pub fn encode_config(config: &Config) -> Json {
    // Les champs plats égalent le profil actif : les sections en tête en
    // sont la copie.
    let active = config.active();
    let mut root = Json::object();
    root.push_number("version", f64::from(config.version));
    root.push_str("active_profile", active.name());
    root.push("brew", encode_brew(active));

    let mut heating = Json::object();
    heating.push_bool("enabled", config.heating_enabled);
    heating.push(
        "brew_temperature_c",
        config_decimal(active.brew_temperature_c),
    );
    heating.push(
        "brew_preheat_time_s",
        config_decimal(active.brew_preheat_time_s),
    );
    root.push("heating", heating);

    root.push("filling", encode_filling(active));
    root.push("preinfusion", encode_preinfusion(active));
    root.push("rampdown", encode_rampdown(active));

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

    root.push(
        "profiles",
        Json::Array(config.profiles().iter().map(encode_profile).collect()),
    );
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

/// Nom d'un champ inconnu ou fautif. La troncature à 47 octets ne se fait
/// qu'à la sortie de [`apply_json_patch`], après l'ajout éventuel du préfixe
/// `profiles[i].`, pour que le nom reste lisible.
fn joined(parent: &str, key: &str) -> String {
    format!("{parent}.{key}")
}

/// Applique le correctif `root` sur `config`, dans l'ordre `profiles`,
/// `active_profile`, puis les sections en tête (le dernier doublon gagne).
/// Les champs « profil » des sections s'écrivent dans le profil actif retenu.
/// En cas de refus, renvoie le nom du champ fautif (47 octets au plus).
pub fn apply_json_patch(root: &Json, config: &mut Config) -> Result<(), String> {
    patch(root, config).map_err(|field| field_name(&field))
}

fn patch(root: &Json, config: &mut Config) -> Result<(), String> {
    if !root.is_object() {
        return Err("version".to_string());
    }
    let version = root.get("version").ok_or_else(|| "version".to_string())?;
    config.version = as_u16(version).ok_or_else(|| "version".to_string())?;

    let Json::Object(pairs) = root else {
        return Err("version".to_string());
    };
    let last = |name: &str| {
        pairs
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    };

    // Le profil actif avant la liste, retrouvé par son nom dans la nouvelle.
    let previous_name = config.active().name().to_string();
    let mut wanted = None;
    if let Some(list) = last("profiles") {
        replace_profiles(list, config)?;
        wanted = Some(previous_name);
    }
    if let Some(name) = last("active_profile") {
        wanted = Some(name.as_str().ok_or("active_profile")?.to_string());
    }
    if let Some(name) = wanted {
        let index = config
            .profiles()
            .iter()
            .position(|profile| profile.name() == name)
            .ok_or("active_profile")?;
        config.active_profile = index as u8;
    }
    config.load_active();

    for (key, value) in pairs {
        match key.as_str() {
            "version" | "profiles" | "active_profile" => {}
            "brew" => overlay(value, "brew", config, apply_brew_key)?,
            "heating" => overlay(value, "heating", config, apply_heating_key)?,
            "filling" => overlay(value, "filling", config, apply_filling_key)?,
            "preinfusion" => overlay(value, "preinfusion", config, apply_preinfusion_key)?,
            "rampdown" => overlay(value, "rampdown", config, apply_rampdown_key)?,
            "purge" => overlay(value, "purge", config, apply_purge_key)?,
            "ui" => overlay(value, "ui", config, apply_ui_key)?,
            "scace" => overlay(value, "scace", config, apply_scace_key)?,
            other => return Err(other.to_string()),
        }
    }
    config.store_active();
    Ok(())
}

/// Clés qu'un élément de `profiles` doit porter, dans l'ordre du document.
const PROFILE_REQUIRED: &[&str] = &[
    "name",
    "dose_g",
    "basket",
    "brew.target_weight_g",
    "brew.target_time_s",
    "brew.target_pressure_bar",
    "brew.pump_pct",
    "heating.brew_temperature_c",
    "heating.brew_preheat_time_s",
    "filling.time_s",
    "filling.pressure_bar",
    "filling.pump_pct",
    "preinfusion.time",
    "preinfusion.weight",
    "preinfusion.time_s",
    "preinfusion.pump_pct",
    "rampdown.mode",
    "rampdown.lead_time_s",
    "rampdown.lead_weight_g",
    "rampdown.pressure_drop_bar",
];

/// Remplace la liste entière. Les règles de nom, de panier, de dose et
/// d'unicité sont celles de la validation de la configuration.
fn replace_profiles(list: &Json, config: &mut Config) -> Result<(), String> {
    let Json::Array(items) = list else {
        return Err("profiles".to_string());
    };
    if items.is_empty() || items.len() > MAX_PROFILES {
        return Err("profiles".to_string());
    }
    let mut profiles = [Profile::EMPTY; MAX_PROFILES];
    let mut settings = Config::default();
    for (index, item) in items.iter().enumerate() {
        profiles[index] = parse_profile(index, item, &mut settings)?;
    }
    // Les règles de la liste se vérifient ici, avant le choix du profil
    // actif : un refus de la liste prime sur celui de `active_profile`.
    if let Some(field) = validate_profile_list(&profiles[..items.len()]) {
        return Err(field.to_string());
    }
    config.profiles = alloc::sync::Arc::new(profiles);
    config.profile_count = items.len() as u8;
    Ok(())
}

/// Un élément complet de `profiles`. Les erreurs portent le préfixe
/// `profiles[index].`.
fn parse_profile(index: usize, item: &Json, settings: &mut Config) -> Result<Profile, String> {
    let at = |field: &str| format!("profiles[{index}].{field}");
    let Json::Object(pairs) = item else {
        return Err(format!("profiles[{index}]"));
    };
    for path in PROFILE_REQUIRED {
        let present = match path.split_once('.') {
            Some((section, key)) => item.get(section).and_then(|node| node.get(key)).is_some(),
            None => item.get(path).is_some(),
        };
        if !present {
            return Err(at(path));
        }
    }
    // Les réglages passent par une configuration de travail, partagée par
    // tous les éléments, pour réutiliser les lecteurs des sections en tête.
    // Chaque réglage étant exigé, seul le mode de pré-infusion, que les
    // lecteurs composent bit à bit, repart de la valeur par défaut.
    settings.preinfusion_mode = PreinfusionMode::TIME;
    let mut profile = Profile::EMPTY;
    for (key, value) in pairs {
        let result = match key.as_str() {
            "name" => {
                let text = value.as_str().ok_or_else(|| at("name"))?;
                if profile.set_name(text) {
                    Ok(())
                } else {
                    Err("name".to_string())
                }
            }
            "basket" => {
                let text = value.as_str().ok_or_else(|| at("basket"))?;
                if profile.set_basket(text) {
                    Ok(())
                } else {
                    Err("basket".to_string())
                }
            }
            "dose_g" => number_float(value, "dose_g", &mut profile.dose_g),
            "brew" => overlay(value, "brew", settings, apply_brew_key),
            "heating" => overlay(value, "heating", settings, apply_profile_heating_key),
            "filling" => overlay(value, "filling", settings, apply_filling_key),
            "preinfusion" => overlay(value, "preinfusion", settings, apply_preinfusion_key),
            "rampdown" => overlay(value, "rampdown", settings, apply_rampdown_key),
            other => Err(other.to_string()),
        };
        result.map_err(|field| at(&field))?;
    }
    profile.take_settings(settings);
    Ok(profile)
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

/// Section `heating` d'un profil : sans l'interrupteur, qui est à la machine.
fn apply_profile_heating_key(key: &str, value: &Json, c: &mut Config) -> Result<(), String> {
    match key {
        "enabled" => Err(joined("heating", key)),
        _ => apply_heating_key(key, value, c),
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

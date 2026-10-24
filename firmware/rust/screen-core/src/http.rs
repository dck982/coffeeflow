//! Documents et décisions HTTP de l'écran, sous forme de fonctions pures :
//! corps de requête → [`HttpResponse`]. Le serveur (tâche 9) fournit le corps
//! lu, l'en-tête `Authorization`, la chaîne de requête, et envoie la réponse ;
//! tout le reste (clés JSON, codes d'état, refus 409) se décide ici et
//! reproduit le firmware C++ archivé.
//!
//! Règles de la politique radio : une infusion ne se lance jamais par HTTP
//! (`/action` `start_brew` répond `409 unavailable`), `/config` et
//! `/firmware` répondent `409 busy` pendant une infusion ou une purge.

use alloc::format;
use alloc::string::{String, ToString};

use crate::config::ConfigStatus;
use crate::config::ConfigStore;
use crate::config_json::{apply_json_patch, encode_config};
use crate::core::{Action, ActionCommand, ActionSource, ActionStatus, Clock, Core, FlashStart};
use crate::hf_capture::{self, Status as CaptureStatus, View};
use crate::json::{self, Json};
use crate::snapshot::{FlashTarget, Freshness, Snapshot};

/// Taille maximale d'un corps de requête JSON. Le `GET /config` de quatre
/// profils pèse un peu plus de 2 ko : son re-`POST` doit passer.
pub const MAX_BODY: usize = 4096;

/// Réponse HTTP : ligne d'état (`"409 Conflict"`) et corps JSON. L'en-tête
/// `Content-Type: application/json` et `Cache-Control: no-store` sont ajoutés
/// par le serveur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: &'static str,
    pub body: String,
}

impl HttpResponse {
    fn new(status: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    fn json(status: &'static str, value: &Json) -> Self {
        Self::new(status, value.to_json_string())
    }
}

/// `{"error": ..., "field": ...}`.
pub fn error_response(status: &'static str, error: &str, field: Option<&str>) -> HttpResponse {
    let mut object = Json::object();
    object.push_str("error", error);
    if let Some(field) = field {
        object.push_str("field", field);
    }
    HttpResponse::json(status, &object)
}

// --- Authentification ----------------------------------------------------------------

/// Vérifie `Authorization: Bearer <jeton>` en temps constant. Un refus est
/// signalé aux capteurs (LOG) et renvoie la réponse `401`.
pub fn authorize<S: ConfigStore>(
    core: &mut Core<S>,
    authorization_header: Option<&str>,
    token: &str,
) -> Result<(), HttpResponse> {
    if authorized(authorization_header, token) {
        return Ok(());
    }
    core.note_http_auth_refused();
    Err(HttpResponse::new(
        "401 Unauthorized",
        "{\"error\":\"unauthorized\"}",
    ))
}

fn authorized(header: Option<&str>, token: &str) -> bool {
    // L'en-tête est lu dans un tampon de 128 octets côté C++.
    let Some(header) = header else {
        return false;
    };
    const PREFIX: &[u8] = b"Bearer ";
    let header = header.as_bytes();
    let token = token.as_bytes();
    if header.len() != PREFIX.len() + token.len() || header.len() >= 128 {
        return false;
    }
    let mut diff = 0u8;
    for (i, &p) in PREFIX.iter().enumerate() {
        diff |= header[i] ^ p;
    }
    for (i, &t) in token.iter().enumerate() {
        diff |= header[PREFIX.len() + i] ^ t;
    }
    diff == 0
}

// --- /telemetry ---------------------------------------------------------------------------

/// État de la plateforme que le cœur ne connaît pas (partitions OTA).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlatformInfo {
    /// Image courante en attente de confirmation.
    pub ota_pending_verify: bool,
    /// Taille de la partition `ota_staging`, `None` si elle n'existe pas.
    pub ota_staging_size: Option<u32>,
    /// Partition en cours d'exécution, `None` si inconnue.
    pub running_partition: Option<RunningPartition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningPartition {
    pub label: String,
    /// `"new"`, `"pending_verify"`, `"valid"`, `"invalid"`, `"aborted"`,
    /// `"undefined"` ou `"unavailable"` ; voir [`ota_state_text`].
    pub state: &'static str,
}

/// État d'image OTA tel que le C++ l'écrit (`esp_ota_img_states_t`).
pub const fn ota_state_text(state: i32) -> &'static str {
    match state {
        0 => "new",
        1 => "pending_verify",
        2 => "valid",
        3 => "invalid",
        4 => "aborted",
        -1 => "undefined",
        _ => "unknown",
    }
}

/// Liste des clés de `/telemetry`, dans l'ordre d'écriture, sous la forme
/// `chemin.de.la.clé`. Sert de contrat aux tests dorés.
pub const TELEMETRY_KEYS: &[&str] = &[
    "uptime_ms",
    "temperature.boiler.c",
    "temperature.boiler.sensor_c",
    "temperature.boiler.valid",
    "temperature.boiler.freshness",
    "temperature.boiler.age_ms",
    "temperature.boiler.ntc_a0_raw",
    "temperature.boiler.ntc_a1_raw",
    "temperature.xdb401.c",
    "temperature.xdb401.raw",
    "enclosure.screen.c",
    "enclosure.screen.rh_pct",
    "enclosure.screen.valid",
    "enclosure.screen.freshness",
    "enclosure.screen.age_ms",
    "enclosure.sensors.c",
    "enclosure.sensors.valid",
    "enclosure.sensors.freshness",
    "enclosure.sensors.age_ms",
    "pressure.bar",
    "pressure.raw",
    "pressure.a2_raw",
    "pressure.valid",
    "pressure.freshness",
    "pressure.age_ms",
    "flow.ml_s",
    "flow.volume_ml",
    "flow.pulse_count",
    "flow.valid",
    "flow.freshness",
    "flow.age_ms",
    "flow.last_edge_age_ms",
    "scale.weight_g",
    "scale.connected",
    "scale.present",
    "scale.age_ms",
    "brew.freshness",
    "brew.age_ms",
    "brew.valve_open",
    "brew.maintenance_valve_open",
    "brew.dimmer_pct",
    "brew.pump_pct",
    "brew.dimmer_ready",
    "brew.dimmer_valid",
    "brew.dimmer_error_active",
    "brew.lockout",
    "brew.lease_remaining_ms",
    "brew.continuous_on_ms",
    "brew.state",
    "brew.elapsed_ms",
    "brew.weight_goal",
    "brew.capture_cooldown",
    "brew.last_shot.available",
    "heating.enabled",
    "heating.target_c",
    "heating.target_sensor_c",
    "heating.ready",
    "heating.power_pct",
    "heating.accepted_power_pct",
    "heating.capable",
    "heating.power_capable",
    "heating.window_restart_capable",
    "heating.requested",
    "heating.freshness",
    "heating.age_ms",
    "heating.on",
    "heating.lease_remaining_ms",
    "firmware.sensors.alive",
    "firmware.sensors.version_major",
    "firmware.sensors.version_minor",
    "firmware.sensors.version_patch",
    "firmware.sensors.uptime_s",
    "firmware.sensors.twai_rx_errors",
    "firmware.sensors.twai_tx_errors",
    "firmware.sensors.twai_bus_errors",
    "firmware.screen.ota.pending_verify",
    "firmware.screen.ota.running_partition",
    "firmware.screen.ota.state",
    "firmware.screen.version_major",
    "firmware.screen.version_minor",
    "firmware.screen.version_patch",
    "firmware.screen.uptime_s",
    "firmware.ota_staging.available",
    "firmware.ota_staging.size",
    "firmware.flash.active",
    "firmware.flash.target",
    "firmware.flash.bytes_done",
    "firmware.flash.bytes_total",
    "inputs.touch_ready",
    "inputs.touch_press_count",
    "network.radio_mode",
    "network.transition",
    "network.state",
    "network.ipv4",
    "network.time_known",
    "network.wall_time_unix_s",
];

fn push_age(object: &mut Json, key: &str, age_ms: u32) {
    if age_ms == u32::MAX {
        object.push_null(key);
    } else {
        object.push_number(key, f64::from(age_ms));
    }
}

fn num(value: f32) -> f64 {
    f64::from(value)
}

fn u(value: u32) -> f64 {
    f64::from(value)
}

/// Document de `GET /telemetry`.
pub fn encode_telemetry(
    snapshot: &Snapshot,
    config: &crate::config::Config,
    platform: &PlatformInfo,
    clock: Clock,
) -> Json {
    let mut root = Json::object();
    root.push_number("uptime_ms", (clock.mono_us / 1000) as f64);

    // temperature
    let mut boiler = Json::object();
    if snapshot.boiler_temperature_age_ms == u32::MAX {
        boiler.push_null("c");
        boiler.push_null("sensor_c");
    } else {
        boiler.push_number("c", num(snapshot.boiler_temperature_c));
        boiler.push_number("sensor_c", num(snapshot.boiler_sensor_temperature_c));
    }
    boiler.push_bool("valid", snapshot.boiler_temperature_valid);
    boiler.push_str("freshness", snapshot.boiler_temperature_freshness.text());
    push_age(&mut boiler, "age_ms", snapshot.boiler_temperature_age_ms);
    boiler.push_number("ntc_a0_raw", f64::from(snapshot.boiler_ntc_a0_raw));
    boiler.push_number("ntc_a1_raw", f64::from(snapshot.boiler_ntc_a1_raw));
    // Seul l'ancien XDB401 I2C mesurait sa température ; la sonde analogique
    // n'en donne pas.
    let mut xdb401 = Json::object();
    xdb401.push_null("c");
    xdb401.push_number("raw", f64::from(snapshot.xdb401_temperature_raw));
    let mut temperature = Json::object();
    temperature.push("boiler", boiler);
    temperature.push("xdb401", xdb401);
    root.push("temperature", temperature);

    // enclosure
    let mut screen = Json::object();
    if snapshot.enclosure_age_ms == u32::MAX {
        screen.push_null("c");
        screen.push_null("rh_pct");
    } else {
        screen.push_number("c", num(snapshot.enclosure_temperature_c));
        screen.push_number("rh_pct", num(snapshot.enclosure_humidity_pct));
    }
    screen.push_bool("valid", snapshot.enclosure_valid);
    screen.push_str("freshness", snapshot.enclosure_freshness.text());
    push_age(&mut screen, "age_ms", snapshot.enclosure_age_ms);
    let mut sensors = Json::object();
    if snapshot.sensors_enclosure_age_ms == u32::MAX {
        sensors.push_null("c");
    } else {
        sensors.push_number("c", num(snapshot.sensors_enclosure_temperature_c));
    }
    sensors.push_bool("valid", snapshot.sensors_enclosure_valid);
    sensors.push_str("freshness", snapshot.sensors_enclosure_freshness.text());
    push_age(&mut sensors, "age_ms", snapshot.sensors_enclosure_age_ms);
    let mut enclosure = Json::object();
    enclosure.push("screen", screen);
    enclosure.push("sensors", sensors);
    root.push("enclosure", enclosure);

    let mut pressure = Json::object();
    pressure.push_number("bar", num(snapshot.pressure_bar));
    pressure.push_number("raw", u(snapshot.pressure_raw));
    if snapshot.pressure_a2_valid {
        pressure.push_number("a2_raw", f64::from(snapshot.pressure_a2_raw));
    } else {
        pressure.push_null("a2_raw");
    }
    pressure.push_bool("valid", snapshot.pressure_valid);
    pressure.push_str("freshness", snapshot.pressure_freshness.text());
    push_age(&mut pressure, "age_ms", snapshot.pressure_age_ms);
    root.push("pressure", pressure);

    let mut flow = Json::object();
    flow.push_number("ml_s", num(snapshot.flow_ml_s));
    flow.push_number("volume_ml", num(snapshot.volume_ml));
    flow.push_number("pulse_count", u(snapshot.flow_pulse_count));
    flow.push_bool("valid", snapshot.flow_valid);
    flow.push_str("freshness", snapshot.flow_freshness.text());
    push_age(&mut flow, "age_ms", snapshot.flow_age_ms);
    push_age(
        &mut flow,
        "last_edge_age_ms",
        snapshot.flow_last_edge_age_ms,
    );
    root.push("flow", flow);

    let mut scale = Json::object();
    scale.push_number("weight_g", num(snapshot.weight_g));
    scale.push_bool("connected", snapshot.scale_connected);
    scale.push_bool("present", snapshot.scale_present);
    push_age(&mut scale, "age_ms", snapshot.scale_age_ms);
    root.push("scale", scale);

    let mut brew = Json::object();
    brew.push_str("freshness", snapshot.actuators_freshness.text());
    push_age(&mut brew, "age_ms", snapshot.actuators_age_ms);
    brew.push_bool("valve_open", snapshot.valve_open);
    brew.push_bool("maintenance_valve_open", snapshot.maintenance_valve_open);
    brew.push_number("dimmer_pct", f64::from(snapshot.dimmer_pct));
    brew.push_number("pump_pct", f64::from(snapshot.pump_pct));
    brew.push_bool("dimmer_ready", snapshot.dimmer_ready);
    brew.push_bool("dimmer_valid", snapshot.dimmer_valid);
    brew.push_bool("dimmer_error_active", snapshot.dimmer_error_active);
    brew.push_bool("lockout", snapshot.lockout);
    brew.push_number("lease_remaining_ms", f64::from(snapshot.lease_remaining_ms));
    brew.push_number("continuous_on_ms", f64::from(snapshot.continuous_on_ms));
    brew.push_str("state", snapshot.cycle_state.text());
    brew.push_number("elapsed_ms", u(snapshot.cycle_elapsed_ms));
    brew.push_bool("weight_goal", snapshot.cycle_weight_goal);
    brew.push_bool("capture_cooldown", snapshot.capture_cooldown);
    let mut last_shot = Json::object();
    last_shot.push_bool("available", snapshot.last_shot_available);
    if snapshot.last_shot_available {
        last_shot.push_number("weight_g", num(snapshot.last_shot_weight_g));
        last_shot.push_number("duration_ms", u(snapshot.last_shot_duration_ms));
        last_shot.push_number("flow_ml_s", num(snapshot.last_shot_flow_ml_s));
        if snapshot.last_shot_unix_s != 0 {
            last_shot.push_number("unix_s", snapshot.last_shot_unix_s as f64);
        } else {
            last_shot.push_null("unix_s");
        }
    }
    brew.push("last_shot", last_shot);
    root.push("brew", brew);

    let mut heating = Json::object();
    heating.push_bool("enabled", config.heating_enabled);
    heating.push_number("target_c", num(config.brew_temperature_c));
    heating.push_number(
        "target_sensor_c",
        num(crate::calibration::boiler_sensor_temperature_c(
            config.brew_temperature_c,
        )),
    );
    heating.push_bool("ready", snapshot.brew_temperature_ready);
    heating.push_number("power_pct", num(snapshot.heating_power_pct));
    if snapshot.heating_freshness == Freshness::Fresh {
        heating.push_number(
            "accepted_power_pct",
            num(snapshot.heating_power_accepted_pct),
        );
    } else {
        heating.push_null("accepted_power_pct");
    }
    heating.push_bool("capable", snapshot.heating_capable);
    heating.push_bool("power_capable", snapshot.heating_power_capable);
    heating.push_bool(
        "window_restart_capable",
        snapshot.heating_window_restart_capable,
    );
    heating.push_bool("requested", snapshot.heating_requested);
    heating.push_str("freshness", snapshot.heating_freshness.text());
    push_age(&mut heating, "age_ms", snapshot.heating_age_ms);
    if snapshot.heating_freshness == Freshness::Fresh {
        heating.push_bool("on", snapshot.heater_on);
    } else {
        heating.push_null("on");
    }
    heating.push_number(
        "lease_remaining_ms",
        f64::from(snapshot.heating_lease_remaining_ms),
    );
    root.push("heating", heating);

    // firmware
    let mut sensors = Json::object();
    sensors.push_bool("alive", snapshot.sensors_alive);
    sensors.push_number("version_major", f64::from(snapshot.sensors_version_major));
    sensors.push_number("version_minor", f64::from(snapshot.sensors_version_minor));
    sensors.push_number("version_patch", f64::from(snapshot.sensors_version_patch));
    sensors.push_number("uptime_s", u(snapshot.sensors_uptime_s));
    sensors.push_number("twai_rx_errors", f64::from(snapshot.sensors_twai_rx_errors));
    sensors.push_number("twai_tx_errors", f64::from(snapshot.sensors_twai_tx_errors));
    sensors.push_number("twai_bus_errors", u(snapshot.sensors_twai_bus_errors));

    let mut ota = Json::object();
    ota.push_bool("pending_verify", platform.ota_pending_verify);
    if let Some(running) = &platform.running_partition {
        ota.push_str("running_partition", &running.label);
        ota.push_str("state", running.state);
    }
    let mut screen = Json::object();
    screen.push("ota", ota);
    screen.push_number("version_major", f64::from(snapshot.screen_version_major));
    screen.push_number("version_minor", f64::from(snapshot.screen_version_minor));
    screen.push_number("version_patch", f64::from(snapshot.screen_version_patch));
    screen.push_number("uptime_s", u(snapshot.screen_uptime_s));

    let mut staging = Json::object();
    staging.push_bool("available", platform.ota_staging_size.is_some());
    if let Some(size) = platform.ota_staging_size {
        staging.push_number("size", u(size));
    }

    let mut flash = Json::object();
    flash.push_bool("active", snapshot.flash_active);
    flash.push_str(
        "target",
        match snapshot.flash_target {
            FlashTarget::Screen => "screen",
            FlashTarget::Sensors => "sensors",
            FlashTarget::None => "none",
        },
    );
    flash.push_number("bytes_done", u(snapshot.flash_bytes_done));
    flash.push_number("bytes_total", u(snapshot.flash_bytes_total));

    let mut firmware = Json::object();
    firmware.push("sensors", sensors);
    firmware.push("screen", screen);
    firmware.push("ota_staging", staging);
    firmware.push("flash", flash);
    root.push("firmware", firmware);

    let mut inputs = Json::object();
    inputs.push_bool("touch_ready", snapshot.touch_ready);
    inputs.push_number("touch_press_count", u(snapshot.touch_press_count));
    root.push("inputs", inputs);

    let mut network = Json::object();
    // Le Wi-Fi et le BLE tournent ensemble : `radio_mode` ne garde que les
    // valeurs « wifi » (radio Wi-Fi en marche, station ou point d'accès) et
    // « off » (bascule des réglages sur OFF). `transition` reste faux.
    let wifi_on = snapshot.provisioning || snapshot.wifi_enabled;
    network.push_str("radio_mode", if wifi_on { "wifi" } else { "off" });
    network.push_bool("transition", false);
    network.push_str("state", snapshot.network_state.text());
    if snapshot.ipv4_address == 0 {
        network.push_null("ipv4");
    } else {
        let a = snapshot.ipv4_address;
        network.push_str(
            "ipv4",
            &format!(
                "{}.{}.{}.{}",
                a & 0xFF,
                (a >> 8) & 0xFF,
                (a >> 16) & 0xFF,
                (a >> 24) & 0xFF
            ),
        );
    }
    network.push_bool("time_known", snapshot.time_known);
    match (snapshot.time_known, clock.wall_unix_s) {
        (true, Some(wall)) => network.push_number("wall_time_unix_s", wall as f64),
        (true, None) => network.push_number("wall_time_unix_s", 0.0),
        _ => network.push_null("wall_time_unix_s"),
    }
    root.push("network", network);
    root
}

/// `GET /telemetry`.
pub fn handle_get_telemetry<S: ConfigStore>(
    core: &mut Core<S>,
    clock: Clock,
    platform: &PlatformInfo,
) -> HttpResponse {
    let snapshot = core.snapshot(clock);
    let config = core.config();
    HttpResponse::json(
        "200 OK",
        &encode_telemetry(&snapshot, config, platform, clock),
    )
}

// --- /config ----------------------------------------------------------------------------------

/// `GET /config`.
pub fn handle_get_config<S: ConfigStore>(core: &Core<S>) -> HttpResponse {
    HttpResponse::json("200 OK", &encode_config(core.config()))
}

fn invalid_json() -> HttpResponse {
    error_response("400 Bad Request", "invalid_json", None)
}

fn read_body(body: &[u8]) -> Option<Json> {
    if body.is_empty() || body.len() > MAX_BODY {
        return None;
    }
    json::parse(body)
}

fn reject_config<S: ConfigStore>(
    core: &mut Core<S>,
    error: &str,
    field: Option<&str>,
) -> HttpResponse {
    core.note_config_rejected(field);
    error_response("400 Bad Request", error, field)
}

/// `POST /config` : correctif partiel validé tout ou rien. `409 busy`
/// pendant une infusion ou une purge.
pub fn handle_post_config<S: ConfigStore>(
    core: &mut Core<S>,
    clock: Clock,
    body: &[u8],
) -> HttpResponse {
    let Some(root) = read_body(body) else {
        return invalid_json();
    };
    let mut candidate = core.config().clone();
    if let Err(field) = apply_json_patch(&root, &mut candidate) {
        return reject_config(core, "invalid_value", Some(&field));
    }
    let result = core.put_config(clock, &candidate);
    match result.status {
        ConfigStatus::Ok => HttpResponse::json("200 OK", &encode_config(core.config())),
        ConfigStatus::Busy => error_response("409 Conflict", "busy", result.field),
        ConfigStatus::StorageError => {
            error_response("500 Internal Server Error", "storage", result.field)
        }
        ConfigStatus::StaleRevision => error_response("409 Conflict", "conflict", result.field),
        status => {
            let error = if status == ConfigStatus::VersionMismatch {
                "unknown_version"
            } else {
                "invalid_value"
            };
            reject_config(core, error, result.field)
        }
    }
}

// --- /action -----------------------------------------------------------------------------------

fn parse_action(root: &Json) -> Result<ActionCommand, String> {
    let fail = |field: &str| field.to_string();
    if !root.is_object() {
        return Err(fail("action"));
    }
    let Some(name) = root.get("action").and_then(Json::as_str) else {
        return Err(fail("action"));
    };
    let action = match name {
        "set_actuators" => Action::SetActuators,
        "set_brew_actuators" => Action::SetBrewActuators,
        "set_heating" => Action::SetHeating,
        "start_brew" => Action::StartBrew,
        "stop_brew" => Action::StopBrew,
        "reset_sensors" => Action::ResetSensors,
        "purge_press" => Action::PurgePress,
        "purge_release" => Action::PurgeRelease,
        "tare" => Action::Tare,
        "dismiss_summary" => Action::DismissSummary,
        "start_flash" => Action::StartFlash,
        _ => return Err(fail("action")),
    };
    let mut command = ActionCommand::new(action);
    let Json::Object(pairs) = root else {
        return Err(fail("action"));
    };
    let as_int = |value: &Json, max: f64| -> Option<f64> {
        let v = value.as_f64()?;
        (v.is_finite() && v == libm::floor(v) && v >= 0.0 && v <= max).then_some(v)
    };
    for (key, value) in pairs {
        if key == "action" {
            continue;
        }
        if action == Action::SetHeating {
            if key == "on"
                && let Some(on) = value.as_bool()
            {
                command.heating.on = on;
                continue;
            }
            if key == "duration_ms"
                && let Some(v) = as_int(value, 65535.0)
            {
                command.heating.duration_ms = v as u16;
                continue;
            }
            return Err(super_field(key));
        }
        if action != Action::SetActuators && action != Action::SetBrewActuators {
            return Err(super_field(key));
        }
        let pump_field = if action == Action::SetActuators {
            "dimmer"
        } else {
            "pump_pct"
        };
        if key == pump_field {
            let Some(v) = as_int(value, 255.0) else {
                return Err(fail(pump_field));
            };
            command.brew.pump_pct = v as u8;
            continue;
        }
        if key == "ttl_ms" {
            let Some(v) = as_int(value, 65535.0) else {
                return Err(fail("ttl_ms"));
            };
            command.brew.ttl_ms = v as u16;
            continue;
        }
        return Err(super_field(key));
    }
    if action == Action::SetActuators || action == Action::SetBrewActuators {
        let pump_field = if action == Action::SetActuators {
            "dimmer"
        } else {
            "pump_pct"
        };
        if root.get(pump_field).is_none()
            || (action == Action::SetBrewActuators && root.get("ttl_ms").is_none())
        {
            return Err(fail(if root.get(pump_field).is_none() {
                pump_field
            } else {
                "ttl_ms"
            }));
        }
    } else if action == Action::SetHeating {
        if !root.get("on").is_some_and(Json::is_bool) {
            return Err(fail("on"));
        }
        let has_duration = root.get("duration_ms").is_some();
        if command.heating.on && !has_duration {
            return Err(fail("duration_ms"));
        }
        if !command.heating.on && has_duration {
            return Err(fail("duration_ms"));
        }
    }
    Ok(command)
}

/// Nom de champ inconnu, tronqué comme le tampon de 48 octets du C++.
fn super_field(key: &str) -> String {
    let mut end = key.len().min(47);
    while !key.is_char_boundary(end) {
        end -= 1;
    }
    key[..end].to_string()
}

/// `POST /action`. Une infusion ne se lance jamais par HTTP.
pub fn handle_post_action<S: ConfigStore>(
    core: &mut Core<S>,
    clock: Clock,
    body: &[u8],
) -> HttpResponse {
    let Some(root) = read_body(body) else {
        return invalid_json();
    };
    let command = match parse_action(&root) {
        Ok(command) => command,
        Err(field) => return error_response("400 Bad Request", "invalid_value", Some(&field)),
    };
    let result = core.perform_action(clock, &command, ActionSource::Http);
    let mut response = Json::object();
    response.push_bool("ok", result.status == ActionStatus::Ok);
    response.push_str("reason", result.reason);
    if command.action == Action::SetHeating {
        let snapshot = core.snapshot(clock);
        response.push_bool("heating_requested", snapshot.heating_requested);
        response.push_str("heating_freshness", snapshot.heating_freshness.text());
        if snapshot.heating_freshness == Freshness::Fresh {
            response.push_bool("heater_on", snapshot.heater_on);
        } else {
            response.push_null("heater_on");
        }
    }
    match result.status {
        ActionStatus::Ok => HttpResponse::json("200 OK", &response),
        ActionStatus::InvalidValue => error_response(
            "400 Bad Request",
            "invalid_value",
            Some(if command.action == Action::SetHeating {
                "duration_ms"
            } else {
                "pump_pct"
            }),
        ),
        _ => HttpResponse::json("409 Conflict", &response),
    }
}

// --- /hf-capture -------------------------------------------------------------------------------

/// Réponse de `GET /hf-capture`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HfCaptureReply {
    /// Erreur à envoyer telle quelle.
    Error(HttpResponse),
    /// `200` en morceaux : envoyer `header`, puis pour chaque index de 0 à
    /// `count` exclu [`hf_capture::export_sample`] de
    /// [`Core::hf_capture_sample`] (`first` pour l'index 0), puis
    /// [`hf_capture::EXPORT_FOOTER`].
    Stream {
        header: String,
        view: View,
        count: u16,
    },
}

/// Valeur de `view` inconnue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidView;

/// Valeur du paramètre `view` de la requête (`?view=raw`). Absent, ou requête
/// de 32 octets ou plus, ou valeur de 12 octets ou plus : `both`, comme les
/// tampons du C++. `Err(InvalidView)` pour une valeur inconnue.
pub fn parse_hf_view(query: Option<&str>) -> Result<View, InvalidView> {
    let Some(query) = query else {
        return Ok(View::Both);
    };
    if query.len() >= 32 {
        return Ok(View::Both);
    }
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == "view" {
            if value.len() >= 12 {
                return Ok(View::Both);
            }
            return View::parse(value).ok_or(InvalidView);
        }
    }
    Ok(View::Both)
}

/// `GET /hf-capture` : refus (`409 capture_active`, `404 capture_missing`...)
/// ou en-tête du document.
pub fn handle_get_hf_capture<S: ConfigStore>(
    core: &Core<S>,
    query: Option<&str>,
) -> HfCaptureReply {
    let Ok(view) = parse_hf_view(query) else {
        return HfCaptureReply::Error(error_response(
            "400 Bad Request",
            "invalid_value",
            Some("view"),
        ));
    };
    let info = core.hf_capture_info();
    match info.status {
        CaptureStatus::Active => {
            return HfCaptureReply::Error(error_response("409 Conflict", "capture_active", None));
        }
        CaptureStatus::Unavailable => {
            return HfCaptureReply::Error(if info.capacity == 0 {
                error_response("503 Service Unavailable", "capture_unavailable", None)
            } else {
                error_response("404 Not Found", "capture_missing", None)
            });
        }
        CaptureStatus::Complete => {}
    }
    HfCaptureReply::Stream {
        header: hf_capture::export_header(
            &info,
            view,
            core.hf_capture_session(),
            &core.hf_capture_context(),
        ),
        view,
        count: info.count,
    }
}

/// Document complet (tests, petits tampons).
pub fn hf_capture_document<S: ConfigStore>(core: &Core<S>, query: Option<&str>) -> HttpResponse {
    match handle_get_hf_capture(core, query) {
        HfCaptureReply::Error(response) => response,
        HfCaptureReply::Stream {
            header,
            view,
            count,
        } => {
            let mut body = header;
            for index in 0..count {
                if let Some(sample) = core.hf_capture_sample(index) {
                    body.push_str(&hf_capture::export_sample(&sample, view, index == 0));
                }
            }
            body.push_str(hf_capture::EXPORT_FOOTER);
            HttpResponse::new("200 OK", body)
        }
    }
}

// --- /firmware ------------------------------------------------------------------------------

/// Cible d'une mise à jour demandée par HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareTarget {
    Screen,
    Sensors,
}

/// Corps de `202` pour une image d'écran : redémarrage après la réponse.
pub const FIRMWARE_SCREEN_ACCEPTED: &str = "{\"ok\":true,\"rebooting\":true}";
/// Corps de `202` pour une image de capteurs mise en file.
pub const FIRMWARE_SENSORS_QUEUED: &str = "{\"ok\":true,\"queued\":true}";
/// Corps de `200` pour la confirmation de l'image courante.
pub const FIRMWARE_CONFIRMED: &str = "{\"ok\":true,\"confirmed\":true,\"rebooting\":true}";

/// Valeur de `target` dans la requête (tampons de 48 et 12 octets du C++).
pub fn query_target(query: Option<&str>) -> Option<&str> {
    let query = query?;
    if query.len() >= 48 {
        return None;
    }
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == "target" && value.len() < 12).then_some(value)
    })
}

/// Début de `POST /firmware` : contrôle de la cible et de la longueur, refus
/// `409 pending_verification` tant que l'image courante n'est pas confirmée,
/// puis [`Core::begin_flash`] (`409 busy` pendant une infusion, une purge, la
/// récupération de capture, ou si les actionneurs ne sont pas au repos). Les
/// contrôles de partition et d'écriture restent à la plateforme ; ses échecs
/// se répondent par [`firmware_failure`] après [`Core::finish_flash`].
pub fn begin_firmware_upload<S: ConfigStore>(
    core: &mut Core<S>,
    clock: Clock,
    query: Option<&str>,
    content_len: i64,
    pending_verify: bool,
) -> Result<(FirmwareTarget, FlashStart), HttpResponse> {
    let bad_target = || error_response("400 Bad Request", "invalid_firmware", Some("target"));
    let Some(target) = query_target(query) else {
        return Err(bad_target());
    };
    if content_len <= 0 {
        return Err(bad_target());
    }
    let target = match target {
        "screen" => FirmwareTarget::Screen,
        "sensors" => FirmwareTarget::Sensors,
        _ => return Err(bad_target()),
    };
    if pending_verify {
        return Err(error_response("409 Conflict", "pending_verification", None));
    }
    let flash_target = match target {
        FirmwareTarget::Screen => FlashTarget::Screen,
        FirmwareTarget::Sensors => FlashTarget::Sensors,
    };
    let start = core.begin_flash(clock, flash_target, content_len as u32);
    if start == FlashStart::Refused {
        return Err(error_response("409 Conflict", "busy", None));
    }
    Ok((target, start))
}

/// Échecs de téléversement : `invalid_firmware` (400, champ `size` si la
/// partition est trop petite), `incomplete_upload` (400), `write_failed`
/// (500), `queue_failed` (500).
pub fn firmware_failure(kind: &str, field: Option<&str>) -> HttpResponse {
    let status = match kind {
        "write_failed" | "queue_failed" => "500 Internal Server Error",
        _ => "400 Bad Request",
    };
    error_response(status, kind, field)
}

/// `POST /firmware/confirm` : `ok` vient de la plateforme (image en attente
/// de confirmation et capteurs vivants).
pub fn firmware_confirm_response(ready: bool) -> HttpResponse {
    if ready {
        HttpResponse::new("200 OK", FIRMWARE_CONFIRMED)
    } else {
        error_response("409 Conflict", "not_ready", None)
    }
}

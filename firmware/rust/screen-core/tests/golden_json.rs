//! Tests dorés : `/telemetry` comparé au JSON que le vrai cJSON écrit pour le
//! même état dans le firmware C++ (`vectors/cjson_vectors.txt`, régénéré par
//! `vectors/gen_cjson_vectors.sh`), et `/config` du schéma 12 comparé au
//! document écrit à la main.

#![allow(clippy::field_reassign_with_default)]

use screen_core::config::{Config, PreinfusionMode};
use screen_core::config_json::encode_config;
use screen_core::core::Clock;
use screen_core::http::{PlatformInfo, RunningPartition, TELEMETRY_KEYS, encode_telemetry};
use screen_core::json::{self, Json, number_to_string};
use screen_core::snapshot::{CycleState, FlashTarget, Freshness, NetworkState, Snapshot};

const VECTORS: &str = include_str!("vectors/cjson_vectors.txt");

fn vector(name: &str) -> &'static str {
    VECTORS
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix('\t'))
        .unwrap_or_else(|| panic!("{name}"))
}

#[test]
fn numbers_are_printed_like_cjson() {
    let mut checked = 0;
    for line in VECTORS.lines().filter(|l| l.starts_with("number\t")) {
        let mut parts = line.split('\t').skip(1);
        let value: f32 = parts.next().unwrap().parse().unwrap();
        let expected = parts.next().unwrap();
        let expected = expected
            .strip_prefix("{\"v\":")
            .unwrap()
            .strip_suffix('}')
            .unwrap();
        assert_eq!(number_to_string(f64::from(value)), expected, "{value}");
        checked += 1;
    }
    assert!(checked >= 30);
}

#[test]
fn config_default_matches_the_schema_12_document() {
    let mut config = Config::default();
    config.revision = 1;
    let brew =
        r#"{"target_weight_g":36.0,"stop_lead_g":4.0,"target_time_s":28,"target_pressure_bar":9.0,"pump_pct":100}"#;
    let filling = r#"{"time_s":10,"pressure_bar":1.0,"pump_pct":100}"#;
    let preinfusion = r#"{"time":true,"weight":false,"time_s":4,"pump_pct":35}"#;
    let expected = format!(
        concat!(
            r#"{{"version":12,"active_profile":"single espresso","brew":{brew},"#,
            r#""heating":{{"enabled":true,"brew_temperature_c":90.0,"brew_preheat_time_s":2.5}},"#,
            r#""filling":{filling},"preinfusion":{preinfusion},"#,
            r#""purge":{{"pump_pct":100,"max_s":20}},"ui":{{"dim_after_s":240,"standby_after_s":1800}},"#,
            r#""scace":{{"enabled":false}},"profiles":[{{"name":"single espresso","dose_g":8.0,"#,
            r#""basket":"LM1 41 mm","brew":{brew},"#,
            r#""heating":{{"brew_temperature_c":90.0,"brew_preheat_time_s":2.5}},"#,
            r#""filling":{filling},"preinfusion":{preinfusion}}}]}}"#
        ),
        brew = brew,
        filling = filling,
        preinfusion = preinfusion,
    );
    assert_eq!(encode_config(&config).to_json_string(), expected);
}

#[test]
fn config_custom_matches_the_schema_12_document() {
    let mut c = Config::default();
    c.target_weight_g = 36.5;
    c.target_time_s = 30;
    c.target_pressure_bar = 9.1;
    c.brew_temperature_c = 92.5;
    c.heating_enabled = false;
    c.brew_preheat_time_s = 7.5;
    c.filling_time_s = 12;
    c.filling_pressure_bar = 0.3;
    c.filling_pump_pct = 75;
    c.preinfusion_mode = PreinfusionMode::TIME | PreinfusionMode::WEIGHT;
    c.preinfusion_time_s = 6;
    c.preinfusion_pump_pct = 40;
    c.stop_lead_g = 2.5;
    c.brew_pump_pct = 85;
    c.purge_pump_pct = 60;
    c.purge_max_s = 30;
    c.dim_after_s = 120;
    c.standby_after_s = 900;
    c.scace_enabled = true;
    let flat = c.clone();
    c.profiles_mut()[0].take_settings(&flat);
    let brew =
        r#"{"target_weight_g":36.5,"stop_lead_g":2.5,"target_time_s":30,"target_pressure_bar":9.1,"pump_pct":85}"#;
    let filling = r#"{"time_s":12,"pressure_bar":0.3,"pump_pct":75}"#;
    let preinfusion = r#"{"time":true,"weight":true,"time_s":6,"pump_pct":40}"#;
    let heating = r#"{"brew_temperature_c":92.5,"brew_preheat_time_s":7.5}"#;
    let expected = format!(
        concat!(
            r#"{{"version":12,"active_profile":"single espresso","brew":{brew},"#,
            r#""heating":{{"enabled":false,"brew_temperature_c":92.5,"brew_preheat_time_s":7.5}},"#,
            r#""filling":{filling},"preinfusion":{preinfusion},"#,
            r#""purge":{{"pump_pct":60,"max_s":30}},"ui":{{"dim_after_s":120,"standby_after_s":900}},"#,
            r#""scace":{{"enabled":true}},"profiles":[{{"name":"single espresso","dose_g":8.0,"#,
            r#""basket":"LM1 41 mm","brew":{brew},"heating":{heating},"#,
            r#""filling":{filling},"preinfusion":{preinfusion}}}]}}"#
        ),
        brew = brew,
        filling = filling,
        preinfusion = preinfusion,
        heating = heating
    );
    assert_eq!(encode_config(&c).to_json_string(), expected);
}

fn clock() -> Clock {
    Clock::with_wall(5_000_000, 1_790_001_234)
}

#[test]
fn telemetry_default_matches_cpp() {
    let mut snapshot = Snapshot::default();
    snapshot.wifi_enabled = false; // le C++ avait `radio_mode` = off
    let document = encode_telemetry(
        &snapshot,
        &Config::default(),
        &PlatformInfo::default(),
        Clock::new(5_000_000),
    );
    assert_eq!(document.to_json_string(), vector("telemetry_default"));
}

#[test]
fn telemetry_busy_matches_cpp() {
    let mut s = Snapshot::default();
    s.pressure_bar = 8.7345;
    s.boiler_temperature_c = 93.456;
    s.boiler_sensor_temperature_c = 103.956;
    s.boiler_ntc_a0_raw = 20000;
    s.boiler_ntc_a1_raw = 9000;
    s.pressure_a2_raw = 6000;
    s.pressure_a2_valid = true;
    s.boiler_temperature_valid = true;
    s.boiler_temperature_freshness = Freshness::Fresh;
    s.boiler_temperature_age_ms = 120;
    s.enclosure_temperature_c = 24.56;
    s.enclosure_humidity_pct = 42.9;
    s.enclosure_valid = true;
    s.enclosure_freshness = Freshness::Fresh;
    s.enclosure_age_ms = 150;
    s.sensors_enclosure_temperature_c = 31.25;
    s.sensors_enclosure_valid = true;
    s.sensors_enclosure_freshness = Freshness::Fresh;
    s.sensors_enclosure_age_ms = 900;
    s.flow_ml_s = 1.2345;
    s.volume_ml = 17.3;
    s.flow_pulse_count = 3_000_000_000;
    s.flow_last_edge_age_ms = 40;
    s.weight_g = -0.05;
    s.scale_connected = true;
    s.scale_present = true;
    s.scale_age_ms = 90;
    s.sensors_alive = true;
    s.pressure_valid = true;
    s.flow_valid = true;
    s.valve_open = true;
    s.dimmer_pct = 70;
    s.pump_pct = 70;
    s.heating_capable = true;
    s.heating_power_capable = true;
    s.heating_window_restart_capable = true;
    s.heater_on = true;
    s.heating_power_pct = 35.5;
    s.heating_power_accepted_pct = 35.4;
    s.brew_temperature_ready = true;
    s.heating_requested = false;
    s.heating_lease_remaining_ms = 1200;
    s.heating_freshness = Freshness::Fresh;
    s.heating_age_ms = 80;
    s.dimmer_ready = true;
    s.dimmer_valid = true;
    s.lease_remaining_ms = 450;
    s.continuous_on_ms = 12000;
    s.pressure_freshness = Freshness::Stale;
    s.flow_freshness = Freshness::Fresh;
    s.actuators_freshness = Freshness::Fresh;
    s.pressure_age_ms = 210;
    s.flow_age_ms = 60;
    s.actuators_age_ms = 100;
    s.sensors_version_minor = 4;
    s.sensors_uptime_s = 86400;
    s.sensors_twai_rx_errors = 3;
    s.sensors_twai_tx_errors = 1;
    s.sensors_twai_bus_errors = 7;
    s.touch_ready = true;
    s.touch_press_count = 42;
    s.network_state = NetworkState::StaConnected;
    s.ipv4_address = 0x0100_A8C0; // 192.168.0.1
    s.wifi_enabled = true;
    s.time_known = true;
    s.pressure_raw = 123_456;
    s.xdb401_temperature_raw = 777;
    s.screen_version_minor = 4;
    s.screen_uptime_s = 3600;
    s.flash_active = true;
    s.flash_target = FlashTarget::Sensors;
    s.flash_bytes_done = 2048;
    s.flash_bytes_total = 400_000;
    s.cycle_state = CycleState::Brew;
    s.cycle_elapsed_ms = 15500;
    s.cycle_weight_goal = true;
    s.capture_cooldown = true;
    s.last_shot_available = true;
    s.last_shot_weight_g = 36.2;
    s.last_shot_duration_ms = 28100;
    s.last_shot_flow_ml_s = 1.05;
    s.last_shot_unix_s = 1_790_000_000;
    let platform = PlatformInfo {
        ota_pending_verify: true,
        ota_staging_size: Some(1_048_576),
        running_partition: Some(RunningPartition {
            label: "ota_0".into(),
            state: "pending_verify",
        }),
    };
    let document = encode_telemetry(&s, &Config::default(), &platform, clock());
    assert_eq!(document.to_json_string(), vector("telemetry_busy"));
}

#[test]
fn telemetry_idle_variant_matches_cpp() {
    let mut t = Snapshot::default();
    t.last_shot_available = true;
    t.last_shot_weight_g = 18.0;
    t.last_shot_duration_ms = 25000;
    t.last_shot_flow_ml_s = 0.0;
    t.last_shot_unix_s = 0;
    t.heating_freshness = Freshness::Stale;
    t.heating_power_accepted_pct = 12.0;
    t.pressure_a2_valid = false;
    t.cycle_state = CycleState::Finished;
    t.flash_target = FlashTarget::Screen;
    t.wifi_enabled = false;
    let platform = PlatformInfo {
        ota_pending_verify: false,
        ota_staging_size: None,
        running_partition: Some(RunningPartition {
            label: "ota_1".into(),
            state: "unavailable",
        }),
    };
    let document = encode_telemetry(&t, &Config::default(), &platform, Clock::new(5_000_000));
    assert_eq!(document.to_json_string(), vector("telemetry_idle_variant"));
}

/// Chemins des feuilles d'un document, dans l'ordre d'écriture.
fn leaf_paths(value: &Json, prefix: &str, out: &mut Vec<String>) {
    match value {
        Json::Object(pairs) if !pairs.is_empty() => {
            for (key, child) in pairs {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                leaf_paths(child, &path, out);
            }
        }
        _ => out.push(prefix.to_string()),
    }
}

#[test]
fn telemetry_keys_golden_list() {
    // La liste de clés de référence vient du document du C++ (cas chargé, où
    // les clés conditionnelles sont toutes présentes), moins les détails de
    // `last_shot` qui n'existent que si une infusion est disponible.
    let busy = json::parse(vector("telemetry_busy").as_bytes()).unwrap();
    let mut cpp = Vec::new();
    leaf_paths(&busy, "", &mut cpp);
    let expected: Vec<String> = cpp
        .into_iter()
        .filter(|p| {
            !["weight_g", "duration_ms", "flow_ml_s", "unix_s"]
                .iter()
                .any(|k| p == &format!("brew.last_shot.{k}"))
        })
        .collect();
    let listed: Vec<String> = TELEMETRY_KEYS.iter().map(|k| (*k).to_string()).collect();
    assert_eq!(listed, expected);

    // Et le document produit n'a ni plus ni moins que cette liste (avec les
    // clés conditionnelles de la variante au repos).
    let mut s = Snapshot::default();
    s.last_shot_available = false;
    let platform = PlatformInfo {
        ota_staging_size: Some(1),
        running_partition: Some(RunningPartition {
            label: "ota_0".into(),
            state: "valid",
        }),
        ..PlatformInfo::default()
    };
    let document = encode_telemetry(&s, &Config::default(), &platform, Clock::new(0));
    let mut produced = Vec::new();
    leaf_paths(&document, "", &mut produced);
    assert_eq!(produced, listed);
}

#[test]
fn parser_roundtrips_the_golden_documents() {
    for name in [
        "config_default",
        "config_custom",
        "telemetry_busy",
        "telemetry_idle_variant",
    ] {
        let text = vector(name);
        let parsed = json::parse(text.as_bytes()).unwrap();
        // Les décimaux de `/config` sont réécrits en nombres normaux : seul
        // l'ordre et la forme de l'arbre nous intéressent ici.
        let mut paths = Vec::new();
        leaf_paths(&parsed, "", &mut paths);
        assert!(!paths.is_empty());
        if name.starts_with("telemetry") {
            assert_eq!(parsed.to_json_string(), text, "{name}");
        }
    }
}

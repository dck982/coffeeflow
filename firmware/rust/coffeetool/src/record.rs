//! `telemetry --record` : interroge `/telemetry` en boucle et garde **toute**
//! la réponse de chaque échantillon.
//!
//! Le fichier garde l'enveloppe des anciens `record_probe.py` et
//! `record_heating.py` : `started_at_utc`, `interval_s`, `max_duration_s`,
//! `samples`, `stop_reason`, `ended_at_utc`, plus un instantané de `/config`
//! pris au départ (il remplace la consigne que relevait `record_heating.py`).
//! Un échantillon est `{elapsed_s, received_at_utc, telemetry}`, avec
//! `scace_c` et `scace_age_ms` quand la SCACE est lue en BLE.

use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Map, Value, json};

use crate::http::JsonClient;
use crate::paths::utc_now;
use crate::runtime::Runtime;

/// 5 Hz par défaut.
pub const DEFAULT_RATE_HZ: f64 = 5.0;
const STATUS_INTERVAL_S: f64 = 10.0;

/// Dernière température de la sonde SCACE lue en BLE.
pub trait ScaceSource {
    /// Température (`None` si absente ou périmée) et âge de la dernière
    /// trame en ms.
    fn read(&self) -> (Option<f64>, Option<u64>);
}

pub struct RecordOptions {
    pub rate_hz: f64,
    pub duration_s: Option<f64>,
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// Ligne d'état : température et consigne de la chaudière, puissance,
/// pression, rapport a0/a1 et SCACE si elle est lue.
pub fn status_line(
    elapsed: f64,
    telemetry: &Value,
    target_c: Option<f64>,
    scace: Option<Option<f64>>,
) -> String {
    let boiler = &telemetry["temperature"]["boiler"];
    let current = boiler["c"]
        .as_f64()
        .filter(|c| c.is_finite() && boiler["valid"].as_bool() == Some(true))
        .map_or("indisponible".to_string(), |c| format!("{c:.1} °C"));
    let target = target_c.map_or("indisponible".to_string(), |c| format!("{c:.1} °C"));
    let power = telemetry["heating"]["power_pct"]
        .as_f64()
        .filter(|p| p.is_finite())
        .map_or("indisponible".to_string(), |p| format!("{p:.1} %"));
    let pressure = &telemetry["pressure"];
    let bar = pressure["bar"]
        .as_f64()
        .filter(|b| b.is_finite() && pressure["valid"].as_bool() == Some(true))
        .map_or("indisponible".to_string(), |b| format!("{b:.3} bar"));
    let ratio = match (boiler["ntc_a0_raw"].as_f64(), boiler["ntc_a1_raw"].as_f64()) {
        (Some(a0), Some(a1)) if a1 > 0.0 => format!("{:.5}", a0 / a1 - 1.0),
        _ => "indisponible".to_string(),
    };
    let mut line = format!(
        "{elapsed:6.1} s : chaudière {current} / consigne {target} / puissance {power} / pression {bar} / a0/a1−1 {ratio}"
    );
    if let Some(scace_c) = scace {
        match scace_c {
            Some(c) => line.push_str(&format!(" / SCACE {c:.2} °C")),
            None => line.push_str(" / SCACE indisponible"),
        }
    }
    line
}

/// Enregistre dans `output` (refuse d'écraser) et renvoie le contenu écrit.
/// Une erreur réseau, ou Ctrl-C, ne perd pas les échantillons : le fichier
/// est écrit avec `stop_reason` (`duration`, `interrupted` ou `error`).
pub fn record_telemetry(
    output: &Path,
    client: &mut dyn JsonClient,
    runtime: &dyn Runtime,
    options: &RecordOptions,
    scace: Option<&dyn ScaceSource>,
    log: &mut dyn FnMut(&str),
) -> Result<Value> {
    if !options.rate_hz.is_finite() || options.rate_hz <= 0.0 {
        bail!("rate doit être un nombre de Hz positif");
    }
    if let Some(duration) = options.duration_s
        && (!duration.is_finite() || duration <= 0.0)
    {
        bail!("duration doit être un nombre de secondes positif");
    }
    if output.exists() {
        bail!("le fichier existe déjà : {}", output.display());
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let interval = 1.0 / options.rate_hz;

    let mut capture = Map::new();
    capture.insert("started_at_utc".into(), json!(utc_now()));
    capture.insert("interval_s".into(), json!(interval));
    capture.insert("max_duration_s".into(), json!(options.duration_s));
    capture.insert("config".into(), Value::Null);
    capture.insert("samples".into(), json!([]));
    capture.insert("stop_reason".into(), json!("error"));

    let mut samples: Vec<Value> = Vec::new();
    let result = (|| -> Result<&'static str> {
        let config = client.call("GET", "/config", None)?;
        let target_c = config["heating"]["brew_temperature_c"]
            .as_f64()
            .filter(|c| c.is_finite());
        capture.insert("config".into(), config);

        let started = runtime.now();
        let mut next_poll = started;
        let mut next_status = started;
        loop {
            if options
                .duration_s
                .is_some_and(|d| runtime.now() >= started + d)
            {
                return Ok("duration");
            }
            runtime.sleep((next_poll - runtime.now()).max(0.0));
            if runtime.interrupted() {
                return Ok("interrupted");
            }
            if options
                .duration_s
                .is_some_and(|d| runtime.now() >= started + d)
            {
                return Ok("duration");
            }
            let telemetry = client.call("GET", "/telemetry", None)?;
            let scace_reading = scace.map(|s| s.read());
            let received = runtime.now();
            let mut sample = Map::new();
            sample.insert("elapsed_s".into(), json!(round3(received - started)));
            sample.insert("received_at_utc".into(), json!(utc_now()));
            sample.insert("telemetry".into(), telemetry.clone());
            if let Some((celsius, age_ms)) = scace_reading {
                sample.insert("scace_c".into(), json!(celsius));
                sample.insert("scace_age_ms".into(), json!(age_ms));
            }
            // Tolérance : l'horloge avance par additions de flottants.
            if received + 1e-9 >= next_status {
                let line = status_line(
                    received - started,
                    &telemetry,
                    target_c,
                    scace_reading.map(|(celsius, _)| celsius),
                );
                log(&line);
                while next_status <= received + 1e-9 {
                    next_status += STATUS_INTERVAL_S;
                }
            }
            samples.push(Value::Object(sample));
            next_poll += interval;
            while next_poll < received {
                next_poll += interval;
            }
        }
    })();

    match result {
        Ok(reason) => {
            capture.insert("stop_reason".into(), json!(reason));
        }
        Err(error) => {
            capture.insert("stop_reason".into(), json!("error"));
            capture.insert("error".into(), json!(format!("{error:#}")));
        }
    }
    capture.insert("samples".into(), Value::Array(samples));
    capture.insert("ended_at_utc".into(), json!(utc_now()));
    let capture = Value::Object(capture);
    crate::paths::write_json_new(output, &capture)?;
    Ok(capture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::FakeClient;
    use crate::runtime::fake::FakeRuntime;
    use anyhow::anyhow;

    fn telemetry() -> Value {
        json!({
            "temperature": {"boiler": {
                "c": 90.2, "valid": true, "freshness": "fresh", "age_ms": 120,
                "ntc_a0_raw": 26305, "ntc_a1_raw": 1151}},
            "pressure": {"a2_raw": 3182, "bar": 8.5, "valid": true},
            "heating": {"power_pct": 37.5},
            "extra": {"kept": [1, 2, 3]},
        })
    }

    fn healthy_client() -> FakeClient {
        FakeClient::new(|_, path, _| {
            Ok(if path == "/config" {
                json!({"version": 7, "heating": {"enabled": true, "brew_temperature_c": 93}})
            } else {
                telemetry()
            })
        })
    }

    struct Fixed(Option<f64>, Option<u64>);
    impl ScaceSource for Fixed {
        fn read(&self) -> (Option<f64>, Option<u64>) {
            (self.0, self.1)
        }
    }

    struct Run {
        capture: Value,
        file: Value,
        lines: Vec<String>,
        calls: Vec<(String, String)>,
    }

    fn run(
        client: FakeClient,
        runtime: &FakeRuntime,
        options: RecordOptions,
        scace: Option<&dyn ScaceSource>,
    ) -> Run {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("sub").join("probe.json");
        let mut client = client;
        let mut lines = Vec::new();
        let capture = record_telemetry(&output, &mut client, runtime, &options, scace, &mut |l| {
            lines.push(l.to_string())
        })
        .unwrap();
        let file: Value = serde_json::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
        let calls = client
            .calls
            .iter()
            .map(|(m, p, _)| (m.clone(), p.clone()))
            .collect();
        Run {
            capture,
            file,
            lines,
            calls,
        }
    }

    fn at_five_hertz(duration: Option<f64>) -> RecordOptions {
        RecordOptions {
            rate_hz: DEFAULT_RATE_HZ,
            duration_s: duration,
        }
    }

    #[test]
    fn records_whole_telemetry_at_five_hertz() {
        let runtime = FakeRuntime::new();
        let result = run(healthy_client(), &runtime, at_five_hertz(Some(1.9)), None);
        assert_eq!(result.capture, result.file);
        let capture = &result.capture;
        assert_eq!(capture["stop_reason"], "duration");
        assert_eq!(capture["interval_s"], 0.2);
        assert_eq!(capture["max_duration_s"], 1.9);
        assert_eq!(capture["samples"].as_array().unwrap().len(), 10);
        let first = &capture["samples"][0];
        assert_eq!(first["telemetry"], telemetry());
        assert_eq!(first["elapsed_s"], 0.0);
        assert!(
            first["received_at_utc"]
                .as_str()
                .unwrap()
                .ends_with("+00:00")
        );
        assert!(first.get("scace_c").is_none());
        assert_eq!(capture["config"]["heating"]["brew_temperature_c"], 93);
        assert!(capture["started_at_utc"].is_string());
        assert!(capture["ended_at_utc"].is_string());
        // Seuls GET /config puis GET /telemetry : jamais d'écriture.
        assert_eq!(result.calls[0], ("GET".into(), "/config".into()));
        assert!(
            result.calls[1..]
                .iter()
                .all(|c| c == &("GET".into(), "/telemetry".into()))
        );
        assert_eq!(result.calls.len(), 11);
    }

    #[test]
    fn keeps_the_envelope_key_order() {
        let runtime = FakeRuntime::new();
        let result = run(healthy_client(), &runtime, at_five_hertz(Some(0.5)), None);
        let keys: Vec<&str> = result
            .capture
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "started_at_utc",
                "interval_s",
                "max_duration_s",
                "config",
                "samples",
                "stop_reason",
                "ended_at_utc"
            ]
        );
        let sample_keys: Vec<&str> = result.capture["samples"][0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(sample_keys, ["elapsed_s", "received_at_utc", "telemetry"]);
    }

    #[test]
    fn rate_sets_the_interval() {
        let runtime = FakeRuntime::new();
        let options = RecordOptions {
            rate_hz: 2.0,
            duration_s: Some(2.0),
        };
        let result = run(healthy_client(), &runtime, options, None);
        assert_eq!(result.capture["interval_s"], 0.5);
        assert_eq!(result.capture["samples"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn scace_fields_ride_next_to_the_telemetry() {
        let runtime = FakeRuntime::new();
        let scace = Fixed(Some(91.5), Some(40));
        let result = run(
            healthy_client(),
            &runtime,
            at_five_hertz(Some(1.0)),
            Some(&scace),
        );
        let first = &result.capture["samples"][0];
        assert_eq!(first["scace_c"], 91.5);
        assert_eq!(first["scace_age_ms"], 40);
        assert!(first["telemetry"].get("scace_c").is_none());
        assert!(
            result.lines[0].ends_with("/ SCACE 91.50 °C"),
            "{}",
            result.lines[0]
        );

        let absent = Fixed(None, None);
        let result = run(
            healthy_client(),
            &runtime_new(),
            at_five_hertz(Some(1.0)),
            Some(&absent),
        );
        assert!(result.capture["samples"][0]["scace_c"].is_null());
        assert!(result.lines[0].ends_with("/ SCACE indisponible"));
    }

    fn runtime_new() -> FakeRuntime {
        FakeRuntime::new()
    }

    #[test]
    fn status_line_every_ten_seconds() {
        let runtime = FakeRuntime::new();
        let result = run(healthy_client(), &runtime, at_five_hertz(Some(21.0)), None);
        assert_eq!(result.lines.len(), 3, "{:?}", result.lines);
        assert_eq!(
            result.lines[0],
            "   0.0 s : chaudière 90.2 °C / consigne 93.0 °C / puissance 37.5 % / pression 8.500 bar / a0/a1−1 21.85404"
        );
        assert!(
            result.lines[1].starts_with("  10.0 s"),
            "{}",
            result.lines[1]
        );
        assert!(
            result.lines[2].starts_with("  20.0 s"),
            "{}",
            result.lines[2]
        );
    }

    #[test]
    fn interrupt_keeps_samples() {
        let runtime = FakeRuntime::new();
        runtime.interrupt_after.set(Some(4));
        let result = run(healthy_client(), &runtime, at_five_hertz(None), None);
        assert_eq!(result.capture["stop_reason"], "interrupted");
        assert_eq!(result.capture["samples"].as_array().unwrap().len(), 3);
        assert_eq!(result.capture, result.file);
    }

    #[test]
    fn network_error_keeps_the_partial_file() {
        let runtime = FakeRuntime::new();
        let mut count = 0;
        let client = FakeClient::new(move |_, path, _| {
            if path == "/config" {
                return Ok(json!({"version": 1}));
            }
            count += 1;
            if count > 3 {
                Err(anyhow!("GET /telemetry: réseau perdu"))
            } else {
                Ok(telemetry())
            }
        });
        let result = run(client, &runtime, at_five_hertz(None), None);
        assert_eq!(result.capture["stop_reason"], "error");
        assert_eq!(result.capture["error"], "GET /telemetry: réseau perdu");
        assert_eq!(result.capture["samples"].as_array().unwrap().len(), 3);
        assert_eq!(result.file["stop_reason"], "error");
    }

    #[test]
    fn config_failure_is_recorded_as_an_error() {
        let runtime = FakeRuntime::new();
        let client = FakeClient::new(|_, _, _| Err(anyhow!("GET /config: HTTP 500: boum")));
        let result = run(client, &runtime, at_five_hertz(Some(1.0)), None);
        assert_eq!(result.capture["stop_reason"], "error");
        assert_eq!(result.capture["samples"], json!([]));
        assert_eq!(result.calls.len(), 1);
    }

    #[test]
    fn refuses_existing_output_and_bad_options() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("probe.json");
        std::fs::write(&output, "{}").unwrap();
        let runtime = FakeRuntime::new();
        let mut client = FakeClient::new(|_, _, _| panic!("aucune requête attendue"));
        let error = record_telemetry(
            &output,
            &mut client,
            &runtime,
            &at_five_hertz(Some(1.0)),
            None,
            &mut |_| {},
        )
        .unwrap_err();
        assert!(error.to_string().contains("existe déjà"), "{error}");
        let fresh = dir.path().join("new.json");
        for (rate, duration) in [
            (0.0, None),
            (5.0, Some(0.0)),
            (5.0, Some(f64::NAN)),
            (f64::NAN, None),
        ] {
            let options = RecordOptions {
                rate_hz: rate,
                duration_s: duration,
            };
            assert!(
                record_telemetry(&fresh, &mut client, &runtime, &options, None, &mut |_| {})
                    .is_err()
            );
        }
        assert!(!fresh.exists());
    }
}

//! `purge <s>` : purge Wi-Fi de la durée demandée, en secondes.
//!
//! L'arrêt est confié au firmware : `purge.max_s`, qui n'accepte que des
//! multiples de 5 s de 5 à 60 s, est réglé sur la durée arrondie au multiple
//! supérieur avant `purge_press`, puis rétabli. Pour un multiple de 5, la
//! purge s'arrête donc à la durée exacte sans dépendre d'un `purge_release`
//! qui pourrait se perdre en Wi-Fi. Sinon, `purge_release` l'arrête à la durée
//! demandée, et le firmware au plus tard au multiple suivant.

use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};

use crate::http::JsonClient;
use crate::runtime::{Interrupted, Runtime};

const POLL_INTERVAL_S: f64 = 0.5;
/// Marge laissée au firmware après la durée avant d'envoyer `purge_release`.
const STOP_GRACE_S: f64 = 3.0;

fn describe_telemetry(label: &str, telemetry: &Value) {
    let boiler = &telemetry["temperature"]["boiler"]["c"];
    let temperature = boiler
        .as_f64()
        .map_or("indisponible".to_string(), |c| format!("{c:.2} °C"));
    let power = telemetry["heating"]["power_pct"]
        .as_f64()
        .map_or("indisponible".to_string(), |p| format!("{p:.1} %"));
    let state = telemetry["brew"]["state"].as_str().unwrap_or("None");
    println!("{label} : chaudière {temperature}, chauffe {power}, cycle {state}");
}

fn cycle_state(client: &mut dyn JsonClient) -> Result<Option<String>> {
    let telemetry = client.call("GET", "/telemetry", None)?;
    Ok(telemetry["brew"]["state"].as_str().map(str::to_string))
}

fn set_purge_max_s(client: &mut dyn JsonClient, version: i64, max_s: i64) -> Result<()> {
    let body = json!({"version": version, "purge": {"max_s": max_s}});
    let response = client.call("POST", "/config", Some(&body))?;
    if response["purge"]["max_s"].as_i64() != Some(max_s) {
        bail!("POST /config: purge.max_s non appliqué ({max_s} s)");
    }
    Ok(())
}

fn press(client: &mut dyn JsonClient) -> Result<()> {
    let body = json!({"action": "purge_press"});
    match client.call("POST", "/action", Some(&body)) {
        Ok(response) => {
            if response["ok"].as_bool() != Some(true) {
                let reason = response.get("reason").map_or("None".to_string(), |r| {
                    r.as_str().map_or_else(|| r.to_string(), str::to_string)
                });
                bail!("purge_press refusé : {reason}");
            }
            Ok(())
        }
        Err(error) => {
            // La commande a pu être appliquée malgré la réponse perdue. Ne pas
            // la renvoyer : une purge courte déjà terminée serait relancée.
            if matches!(cycle_state(client), Ok(Some(state)) if state == "purge") {
                return Ok(());
            }
            Err(anyhow!(
                "purge non démarrée après une réponse perdue : {error}"
            ))
        }
    }
}

/// Renvoie `purge_release` jusqu'à l'arrêt, au plus tard jusqu'à l'arrêt
/// firmware.
fn release_until_stopped(
    client: &mut dyn JsonClient,
    backstop: f64,
    runtime: &dyn Runtime,
) -> Result<()> {
    loop {
        let release = json!({"action": "purge_release"});
        // Une réponse perdue ne vaut pas arrêt : on réessaie.
        if client.call("POST", "/action", Some(&release)).is_ok()
            && let Ok(state) = cycle_state(client)
            && state.as_deref() != Some("purge")
        {
            return Ok(());
        }
        if runtime.now() >= backstop {
            bail!("purge toujours active après purge_release, l'arrêter depuis la machine");
        }
        runtime.sleep(POLL_INTERVAL_S);
    }
}

fn wait_for_stop(
    client: &mut dyn JsonClient,
    duration_s: f64,
    backstop: f64,
    firmware_timed: bool,
    runtime: &dyn Runtime,
) -> Result<()> {
    let deadline = runtime.now() + duration_s + if firmware_timed { STOP_GRACE_S } else { 0.0 };
    while runtime.now() < deadline {
        runtime.sleep(POLL_INTERVAL_S.min((deadline - runtime.now()).max(0.0)));
        if runtime.interrupted() {
            return Err(Interrupted.into());
        }
        match cycle_state(client) {
            Ok(state) if state.as_deref() != Some("purge") => return Ok(()),
            // Télémétrie perdue : le firmware arrête la purge seul.
            Ok(_) | Err(_) => {}
        }
    }
    if firmware_timed {
        eprintln!("Purge encore active après sa durée, envoi de purge_release");
    }
    release_until_stopped(client, backstop, runtime)
}

pub fn purge(duration_s: f64, client: &mut dyn JsonClient, runtime: &dyn Runtime) -> Result<()> {
    if !duration_s.is_finite() || duration_s <= 0.0 || duration_s > 60.0 {
        bail!("la durée doit être comprise entre 0 et 60 s (plafond de purge.max_s)");
    }
    let timeout_s = (5.0 * (duration_s / 5.0).ceil()).max(5.0) as i64;
    let firmware_timed = timeout_s as f64 == duration_s;

    let config = client.call("GET", "/config", None)?;
    let version = config["version"].as_i64();
    let max_s = config["purge"]["max_s"].as_i64();
    let (Some(version), Some(max_s)) = (version, max_s) else {
        bail!("GET /config: version ou purge.max_s absent ou invalide");
    };

    let before = client.call("GET", "/telemetry", None)?;
    let cycle = before["brew"]["state"].as_str();
    if !matches!(cycle, Some("idle" | "finished")) {
        let shown = cycle.map_or("None".to_string(), |c| format!("{c:?}"));
        bail!("purge impossible : cycle actuel {shown}");
    }
    describe_telemetry("Avant", &before);

    let outcome = (|| -> Result<()> {
        if max_s != timeout_s {
            set_purge_max_s(client, version, timeout_s)?;
        }
        let run = (|| -> Result<()> {
            press(client)?;
            // Arrêt firmware attendu au plus tard ici, marge comprise.
            let backstop = runtime.now() + timeout_s as f64 + STOP_GRACE_S;
            let stop_text = if firmware_timed {
                "arrêt par le firmware".to_string()
            } else {
                format!("arrêt par purge_release, firmware à {timeout_s} s")
            };
            println!("Purge démarrée pour {duration_s} s ({stop_text})");
            wait_for_stop(client, duration_s, backstop, firmware_timed, runtime)
        })();
        match run {
            Err(error) if error.downcast_ref::<Interrupted>().is_some() => {
                let backstop = runtime.now() + timeout_s as f64 + STOP_GRACE_S;
                release_until_stopped(client, backstop, runtime)?;
                Err(error)
            }
            other => other,
        }?;
        println!("Purge arrêtée");
        Ok(())
    })();

    // Dans tous les cas, `purge.max_s` retrouve sa valeur.
    if max_s != timeout_s
        && let Err(error) = set_purge_max_s(client, version, max_s)
    {
        eprintln!("purge.max_s reste à {timeout_s} s, le remettre à {max_s} s : {error}");
    }
    outcome?;

    describe_telemetry("Après", &client.call("GET", "/telemetry", None)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::FakeClient;
    use crate::runtime::fake::FakeRuntime;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// Écran simulé : la purge s'arrête seule à `purge.max_s`, figé à l'appui.
    struct Machine {
        max_s: Cell<i64>,
        purge_until: Cell<Option<f64>>,
        calls: RefCell<Vec<(String, String, Option<Value>)>>,
        /// (méthode, action ou chemin) dont la réponse se perd, une fois.
        lose: RefCell<Vec<(String, String)>>,
        release_ignored: Cell<bool>,
        /// `/config` ignore la valeur demandée et rend ce `max_s` (firmware
        /// qui n'applique pas le réglage).
        config_reports: Cell<Option<i64>>,
        /// `purge_press` n'a aucun effet et sa réponse se perd.
        press_never_applied: Cell<bool>,
        runtime: Rc<FakeRuntime>,
    }

    impl Machine {
        fn new(max_s: i64) -> Rc<Self> {
            Rc::new(Self {
                max_s: Cell::new(max_s),
                purge_until: Cell::new(None),
                calls: RefCell::new(Vec::new()),
                lose: RefCell::new(Vec::new()),
                release_ignored: Cell::new(false),
                config_reports: Cell::new(None),
                press_never_applied: Cell::new(false),
                runtime: Rc::new(FakeRuntime::new()),
            })
        }

        fn state(&self) -> &'static str {
            if self
                .purge_until
                .get()
                .is_some_and(|t| self.runtime.now.get() >= t)
            {
                self.purge_until.set(None);
            }
            if self.purge_until.get().is_some() {
                "purge"
            } else {
                "finished"
            }
        }

        fn handle(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
            let key = body
                .and_then(|b| b["action"].as_str())
                .unwrap_or(path)
                .to_string();
            self.calls
                .borrow_mut()
                .push((method.into(), key.clone(), body.cloned()));
            let response = if path == "/config" && method == "GET" {
                json!({"version": 2, "purge": {"max_s": self.max_s.get()}})
            } else if path == "/config" {
                let requested = body.unwrap()["purge"]["max_s"].as_i64().unwrap();
                self.max_s.set(requested);
                let reported = self.config_reports.get().unwrap_or(requested);
                json!({"version": 2, "purge": {"max_s": reported}})
            } else if path == "/telemetry" {
                json!({"brew": {"state": self.state()}})
            } else if key == "purge_press" {
                if self.press_never_applied.get() {
                    return Err(anyhow!("requête perdue"));
                }
                self.state();
                if self.purge_until.get().is_none() {
                    self.purge_until
                        .set(Some(self.runtime.now.get() + self.max_s.get() as f64));
                }
                json!({"ok": true})
            } else {
                if !self.release_ignored.get() {
                    self.purge_until.set(None);
                }
                json!({"ok": true})
            };
            let mut lose = self.lose.borrow_mut();
            if let Some(i) = lose.iter().position(|(m, k)| m == method && *k == key) {
                lose.remove(i);
                return Err(anyhow!("réponse Wi-Fi perdue"));
            }
            Ok(response)
        }

        fn client(self: &Rc<Self>) -> FakeClient {
            let machine = Rc::clone(self);
            FakeClient::new(move |m, p, b| machine.handle(m, p, b))
        }

        fn actions(&self) -> Vec<String> {
            self.calls
                .borrow()
                .iter()
                .filter(|(m, _, _)| m == "POST")
                .map(|(_, k, _)| k.clone())
                .collect()
        }

        fn max_s_posts(&self) -> Vec<i64> {
            self.calls
                .borrow()
                .iter()
                .filter(|(m, k, _)| m == "POST" && k == "/config")
                .map(|(_, _, b)| b.as_ref().unwrap()["purge"]["max_s"].as_i64().unwrap())
                .collect()
        }

        fn run(self: &Rc<Self>, duration: f64) -> Result<()> {
            purge(duration, &mut self.client(), self.runtime.as_ref())
        }
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn firmware_times_the_purge_and_max_s_is_restored() {
        let machine = Machine::new(20);
        machine.run(15.0).unwrap();
        assert_eq!(
            machine.actions(),
            strings(&["/config", "purge_press", "/config"])
        );
        assert_eq!(machine.max_s_posts(), vec![15, 20]);
        assert_eq!(machine.max_s.get(), 20);
        assert!(machine.runtime.now.get() >= 15.0);
    }

    #[test]
    fn config_untouched_when_max_s_already_matches() {
        let machine = Machine::new(15);
        machine.run(15.0).unwrap();
        assert_eq!(machine.actions(), strings(&["purge_press"]));
    }

    #[test]
    fn lost_press_response_is_checked_on_telemetry() {
        let machine = Machine::new(20);
        machine
            .lose
            .borrow_mut()
            .push(("POST".into(), "purge_press".into()));
        machine.run(15.0).unwrap();
        let presses = machine
            .actions()
            .iter()
            .filter(|a| *a == "purge_press")
            .count();
        assert_eq!(presses, 1);
        assert_eq!(machine.max_s.get(), 20);
    }

    #[test]
    fn unapplied_press_is_not_resent() {
        let machine = Machine::new(20);
        machine.press_never_applied.set(true);
        let error = machine.run(15.0).unwrap_err().to_string();
        assert!(error.contains("non démarrée"), "{error}");
        let presses = machine
            .actions()
            .iter()
            .filter(|a| *a == "purge_press")
            .count();
        assert_eq!(presses, 1);
        assert_eq!(machine.max_s.get(), 20);
    }

    #[test]
    fn release_is_a_fallback_when_purge_outlives_its_duration() {
        let machine = Machine::new(20);
        // Le firmware prétend avoir réglé 15 s mais garde 60 s.
        let m = Rc::clone(&machine);
        let mut client = FakeClient::new(move |method, path, body| {
            if method == "POST" && path == "/config" {
                let mut answer = m.handle(method, path, body)?;
                // Le réglage est annoncé appliqué, mais la purge dure 60 s.
                m.max_s.set(60);
                answer["purge"]["max_s"] = body.unwrap()["purge"]["max_s"].clone();
                return Ok(answer);
            }
            m.handle(method, path, body)
        });
        purge(15.0, &mut client, machine.runtime.as_ref()).unwrap();
        assert!(machine.actions().contains(&"purge_release".to_string()));
        assert_eq!(machine.state(), "finished");
    }

    #[test]
    fn stuck_purge_is_reported() {
        let machine = Machine::new(20);
        machine.release_ignored.set(true);
        let m = Rc::clone(&machine);
        let mut client = FakeClient::new(move |method, path, body| {
            if method == "POST" && path == "/config" {
                let mut answer = m.handle(method, path, body)?;
                m.max_s.set(60);
                answer["purge"]["max_s"] = body.unwrap()["purge"]["max_s"].clone();
                return Ok(answer);
            }
            m.handle(method, path, body)
        });
        let error = purge(15.0, &mut client, machine.runtime.as_ref())
            .unwrap_err()
            .to_string();
        assert!(error.contains("toujours active"), "{error}");
    }

    #[test]
    fn interruption_releases() {
        let machine = Machine::new(20);
        machine.runtime.interrupt_after.set(Some(1));
        let error = machine.run(15.0).unwrap_err();
        assert!(error.downcast_ref::<Interrupted>().is_some(), "{error}");
        assert_eq!(
            machine.actions(),
            strings(&["/config", "purge_press", "purge_release", "/config"])
        );
        assert_eq!(machine.max_s.get(), 20);
    }

    #[test]
    fn other_durations_are_released_with_a_firmware_backstop() {
        let machine = Machine::new(20);
        machine.run(8.0).unwrap();
        assert_eq!(
            machine.actions(),
            strings(&["/config", "purge_press", "purge_release", "/config"])
        );
        assert_eq!(machine.max_s_posts(), vec![10, 20]);
        assert!((machine.runtime.now.get() - 8.0).abs() < 1e-9);
    }

    #[test]
    fn lost_release_still_stops_at_the_backstop() {
        let machine = Machine::new(20);
        machine.release_ignored.set(true);
        // Le firmware arrête la purge à 10 s (max_s réglé) même sans release.
        machine.run(8.0).unwrap();
        let releases = machine
            .actions()
            .iter()
            .filter(|a| *a == "purge_release")
            .count();
        assert!(releases >= 2, "{releases}");
        assert_eq!(machine.state(), "finished");
        assert!(machine.runtime.now.get() <= 11.0);
    }

    #[test]
    fn rejects_duration_outside_firmware_range() {
        for duration in [0.0, -1.0, 65.0, f64::NAN, f64::INFINITY] {
            let mut client = FakeClient::new(|_, _, _| panic!("aucune requête attendue"));
            let runtime = FakeRuntime::new();
            let error = purge(duration, &mut client, &runtime)
                .unwrap_err()
                .to_string();
            assert!(error.contains("entre 0 et 60"), "{error}");
        }
    }

    #[test]
    fn refuses_active_cycle() {
        let machine = Machine::new(20);
        machine.purge_until.set(Some(100.0));
        let error = machine.run(15.0).unwrap_err().to_string();
        assert!(error.contains("cycle actuel"), "{error}");
        assert!(machine.actions().is_empty());
    }

    #[test]
    fn unapplied_max_s_is_an_error_and_nothing_is_pressed() {
        let machine = Machine::new(20);
        machine.config_reports.set(Some(20));
        let error = machine.run(15.0).unwrap_err().to_string();
        assert!(error.contains("purge.max_s non appliqué"), "{error}");
        // Pas de purge ; le rétablissement est tenté quand même, comme le
        // `finally` du script.
        assert_eq!(machine.actions(), strings(&["/config", "/config"]));
    }
}

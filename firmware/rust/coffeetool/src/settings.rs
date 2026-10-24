//! Commandes de configuration : `set`, `config save` / `config restore`,
//! `download-hf`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};

use crate::http::JsonClient;
use crate::paths;

/// Un réglage booléen de `/config`. Une nouvelle clé s'ajoute ici : il n'y a
/// pas de `set <chemin> <valeur>` générique, pour qu'une faute de frappe ne
/// parte pas vers la machine.
pub struct Setting {
    pub name: &'static str,
    pub section: &'static str,
    pub key: &'static str,
    /// Erreur si `GET /config` n'a pas la section (firmware trop ancien).
    pub missing_section: Option<&'static str>,
    /// Erreur si la réponse ne confirme pas la valeur.
    pub unconfirmed: &'static str,
}

pub const SETTINGS: &[Setting] = &[
    Setting {
        name: "heating",
        section: "heating",
        key: "enabled",
        missing_section: None,
        unconfirmed: "POST /config: état de chauffe non confirmé",
    },
    // Désactivée, l'écran ne cherche plus la sonde SCACE et s'en déconnecte ;
    // la balance n'est pas touchée.
    Setting {
        name: "scace",
        section: "scace",
        key: "enabled",
        missing_section: Some("GET /config: pas de section scace (écran antérieur à 0.3.47)"),
        unconfirmed: "POST /config: état de la sonde non confirmé",
    },
];

pub fn setting(name: &str) -> Option<&'static Setting> {
    SETTINGS.iter().find(|s| s.name == name)
}

/// Version du schéma lue dans une réponse de `/config` : un entier, jamais
/// un booléen.
fn config_version(config: &Value) -> Option<i64> {
    config.get("version").and_then(Value::as_i64)
}

/// Relit `/config` pour sa `version`, n'envoie que la clé changée avec cette
/// version, puis vérifie la réponse.
pub fn set_bool(setting: &Setting, enabled: bool, client: &mut dyn JsonClient) -> Result<()> {
    let config = client.call("GET", "/config", None)?;
    let version = config_version(&config)
        .ok_or_else(|| anyhow!("GET /config: version absente ou invalide"))?;
    if let Some(message) = setting.missing_section
        && config.get(setting.section).is_none()
    {
        bail!("{message}");
    }
    let body = json!({"version": version, setting.section: {setting.key: enabled}});
    let response = client.call("POST", "/config", Some(&body))?;
    let confirmed = response
        .get(setting.section)
        .and_then(|section| section.get(setting.key))
        .and_then(Value::as_bool)
        == Some(enabled);
    if !confirmed {
        bail!("{}", setting.unconfirmed);
    }
    Ok(())
}

/// Lit `/config`, vérifie le schéma et renvoie la configuration.
fn load_config(client: &mut dyn JsonClient) -> Result<Value> {
    let config = client.call("GET", "/config", None)?;
    if config_version(&config).is_none() {
        bail!("réponse /config inconnue ou incompatible");
    }
    Ok(config)
}

/// `config save` : écrit `GET /config` dans `configs/yymmdd-HHMMSS.json`, ou
/// dans `output`. Renvoie le fichier écrit.
pub fn save_config(
    client: &mut dyn JsonClient,
    output: Option<PathBuf>,
    default_dir: &Path,
) -> Result<PathBuf> {
    let config = load_config(client)?;
    let output =
        output.unwrap_or_else(|| default_dir.join(format!("{}.json", paths::short_stamp())));
    paths::write_json_new(&output, &config)?;
    Ok(output)
}

/// Fichier de configuration à restaurer : un objet JSON avec une `version`
/// entière.
pub fn read_config(path: &Path) -> Result<Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
    let payload: Value = serde_json::from_str(&text)
        .map_err(|e| anyhow!("JSON invalide dans {}: {e}", path.display()))?;
    if config_version(&payload).is_none() {
        bail!("configuration inconnue ou incompatible: {}", path.display());
    }
    Ok(payload)
}

/// `config restore` : `POST /config` avec le fichier tel quel. Renvoie la
/// version de la réponse.
pub fn restore_config(client: &mut dyn JsonClient, config: &Value) -> Result<i64> {
    let response = client.call("POST", "/config", Some(config))?;
    config_version(&response).ok_or_else(|| anyhow!("réponse /config inconnue ou incompatible"))
}

/// `download-hf` : dernière capture haute fréquence, dans `captures/`.
pub fn download_hf(client: &mut dyn JsonClient, capture_dir: &Path) -> Result<PathBuf> {
    let capture = client.call("GET", "/hf-capture?view=both", None)?;
    let schema_ok = matches!(
        capture.get("schema").and_then(Value::as_str),
        Some("coffeeflow.hf_capture.v1" | "coffeeflow.hf_capture.v2" | "coffeeflow.hf_capture.v3")
    );
    if !schema_ok || !capture.get("samples").is_some_and(Value::is_array) {
        bail!("réponse /hf-capture inconnue ou incompatible");
    }
    let output = capture_dir.join(format!("{}.json", paths::short_stamp()));
    paths::write_json_new(&output, &capture)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::FakeClient;

    fn heating() -> &'static Setting {
        setting("heating").unwrap()
    }

    fn scace() -> &'static Setting {
        setting("scace").unwrap()
    }

    #[test]
    fn heating_reads_version_and_sets_both_states() {
        for enabled in [true, false] {
            let mut client = FakeClient::new(move |method, _path, body| {
                Ok(if method == "GET" {
                    json!({"version": 7})
                } else {
                    json!({"heating": {"enabled": body.unwrap()["heating"]["enabled"]}})
                })
            });
            set_bool(heating(), enabled, &mut client).unwrap();
            assert_eq!(
                client.calls,
                vec![
                    ("GET".into(), "/config".into(), None),
                    (
                        "POST".into(),
                        "/config".into(),
                        Some(json!({"version": 7, "heating": {"enabled": enabled}}))
                    ),
                ]
            );
        }
    }

    #[test]
    fn heating_rejects_unconfirmed_change() {
        let mut client = FakeClient::new(|method, _, _| {
            Ok(if method == "GET" {
                json!({"version": 7})
            } else {
                json!({"heating": {"enabled": false}})
            })
        });
        let error = set_bool(heating(), true, &mut client)
            .unwrap_err()
            .to_string();
        assert!(error.contains("non confirmé"), "{error}");
    }

    #[test]
    fn rejects_invalid_version() {
        for version in [json!(true), json!("7"), json!(null), json!(1.5)] {
            let mut client = FakeClient::new(move |method, _, _| {
                assert_eq!(method, "GET");
                Ok(json!({"version": version.clone()}))
            });
            let error = set_bool(heating(), true, &mut client)
                .unwrap_err()
                .to_string();
            assert!(error.contains("version absente"), "{error}");
        }
    }

    #[test]
    fn scace_reads_version_and_sets_both_states() {
        for enabled in [true, false] {
            let mut client = FakeClient::new(move |method, _, body| {
                Ok(if method == "GET" {
                    json!({"version": 10, "scace": {"enabled": !enabled}})
                } else {
                    json!({"scace": {"enabled": body.unwrap()["scace"]["enabled"]}})
                })
            });
            set_bool(scace(), enabled, &mut client).unwrap();
            assert_eq!(
                client.calls[1].2,
                Some(json!({"version": 10, "scace": {"enabled": enabled}}))
            );
        }
    }

    #[test]
    fn scace_rejects_unconfirmed_change() {
        let mut client = FakeClient::new(|method, _, _| {
            Ok(if method == "GET" {
                json!({"version": 10, "scace": {"enabled": false}})
            } else {
                json!({"scace": {"enabled": false}})
            })
        });
        let error = set_bool(scace(), true, &mut client)
            .unwrap_err()
            .to_string();
        assert!(error.contains("non confirmé"), "{error}");
    }

    #[test]
    fn scace_rejects_firmware_without_the_section() {
        let mut client = FakeClient::new(|method, _, _| {
            assert_eq!(method, "GET");
            Ok(json!({"version": 9}))
        });
        let error = set_bool(scace(), true, &mut client)
            .unwrap_err()
            .to_string();
        assert!(error.contains("0.3.47"), "{error}");
    }

    #[test]
    fn unknown_setting_is_not_in_the_table() {
        assert!(setting("pump").is_none());
    }

    #[test]
    fn save_writes_the_config_like_save_config_py() {
        let dir = tempfile::tempdir().unwrap();
        let mut client = FakeClient::new(|_, _, _| {
            Ok(json!({"version": 10, "name": "café", "heating": {"enabled": true}}))
        });
        let path = save_config(&mut client, None, dir.path()).unwrap();
        assert_eq!(path.parent().unwrap(), dir.path());
        let name = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(name.len(), "yymmdd-HHMMSS.json".len(), "{name}");
        // Accents tels quels, deux espaces d'indentation, ordre des clés
        // conservé, retour à la ligne final.
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\n  \"version\": 10,\n  \"name\": \"café\",\n  \"heating\": {\n    \"enabled\": true\n  }\n}\n"
        );
        assert_eq!(client.calls, vec![("GET".into(), "/config".into(), None)]);
    }

    #[test]
    fn save_refuses_an_existing_file_and_a_bad_answer() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.json");
        std::fs::write(&target, "{}").unwrap();
        let mut client = FakeClient::new(|_, _, _| Ok(json!({"version": 1})));
        let error = save_config(&mut client, Some(target), dir.path()).unwrap_err();
        assert!(error.to_string().contains("existe déjà"), "{error}");
        let mut client = FakeClient::new(|_, _, _| Ok(json!({"x": 1})));
        let error = save_config(&mut client, None, dir.path()).unwrap_err();
        assert!(error.to_string().contains("incompatible"), "{error}");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn restore_posts_the_file_as_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(&path, "{\"version\": 4, \"heating\": {\"enabled\": false}}").unwrap();
        let config = read_config(&path).unwrap();
        let mut client = FakeClient::new(|_, _, _| Ok(json!({"version": 5})));
        assert_eq!(restore_config(&mut client, &config).unwrap(), 5);
        assert_eq!(
            client.calls,
            vec![(
                "POST".into(),
                "/config".into(),
                Some(json!({"version": 4, "heating": {"enabled": false}}))
            )]
        );
    }

    #[test]
    fn restore_posts_a_schema_11_save_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let saved = json!({
            "version": 11,
            "active_profile": "single espresso",
            "brew": {"target_weight_g": 22.0},
            "profiles": [{"name": "single espresso", "dose_g": 8.0, "basket": "LM1 41 mm"}],
        });
        let mut get = FakeClient::new({
            let saved = saved.clone();
            move |_, _, _| Ok(saved.clone())
        });
        let path = dir.path().join("c.json");
        save_config(&mut get, Some(path.clone()), dir.path()).unwrap();
        let config = read_config(&path).unwrap();
        assert_eq!(config, saved);
        let mut post = FakeClient::new(|_, _, body| Ok(body.unwrap().clone()));
        assert_eq!(restore_config(&mut post, &config).unwrap(), 11);
        assert_eq!(post.calls[0].2, Some(saved));
    }

    #[test]
    fn restore_refuses_bad_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(&path, "pas du json").unwrap();
        assert!(
            read_config(&path)
                .unwrap_err()
                .to_string()
                .contains("JSON invalide")
        );
        std::fs::write(&path, "[1]").unwrap();
        assert!(
            read_config(&path)
                .unwrap_err()
                .to_string()
                .contains("incompatible")
        );
        std::fs::write(&path, "{\"version\": true}").unwrap();
        assert!(read_config(&path).is_err());
    }

    #[test]
    fn download_hf_checks_the_schema_and_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut client = FakeClient::new(|_, _, _| {
            Ok(json!({"schema": "coffeeflow.hf_capture.v2", "samples": [1, 2]}))
        });
        let path = download_hf(&mut client, dir.path()).unwrap();
        assert_eq!(
            client.calls,
            vec![("GET".into(), "/hf-capture?view=both".into(), None)]
        );
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(saved["samples"], json!([1, 2]));

        let mut client = FakeClient::new(|_, _, _| {
            Ok(json!({"schema": "coffeeflow.hf_capture.v3", "samples": [3]}))
        });
        let other = tempfile::tempdir().unwrap();
        assert!(download_hf(&mut client, other.path()).is_ok());

        let mut client = FakeClient::new(|_, _, _| Ok(json!({"schema": "autre", "samples": []})));
        assert!(download_hf(&mut client, dir.path()).is_err());
        let mut client =
            FakeClient::new(|_, _, _| Ok(json!({"schema": "coffeeflow.hf_capture.v1"})));
        assert!(download_hf(&mut client, dir.path()).is_err());
    }
}

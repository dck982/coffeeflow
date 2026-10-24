//! `profile list|add|rename|set|rm|use` : les profils d'infusion passent par
//! `/config`. Chaque commande relit la configuration, modifie la liste puis la
//! renvoie entière avec le nom du profil actif ; l'écran vérifie tout et
//! applique tout ou rien.

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::http::JsonClient;

const MAX_PROFILES: usize = 4;
const NAME_MAX_CHARS: usize = 20;
const BASKET_MAX_CHARS: usize = 24;
const DOSE_RANGE_G: (f64, f64) = (3.0, 30.0);

#[derive(Subcommand, Debug, PartialEq)]
pub enum ProfileAction {
    /// Liste les profils ; l'actif est marqué d'une étoile
    List,
    /// Crée un profil : copie `--from`, ou par défaut le profil actif
    Add {
        name: String,
        #[arg(long)]
        from: Option<String>,
        /// Dose en grammes (3 à 30, 0 : non renseignée)
        #[arg(long)]
        dose: Option<f64>,
        #[arg(long)]
        basket: Option<String>,
    },
    /// Renomme un profil ; si c'est l'actif, il le reste
    Rename { old: String, new: String },
    /// Change la recette : `--dose 0` efface la dose, `--basket ""` le panier
    Set {
        name: String,
        #[arg(long)]
        dose: Option<f64>,
        #[arg(long)]
        basket: Option<String>,
    },
    /// Supprime un profil (ni l'actif, ni le dernier)
    Rm { name: String },
    /// Rend un profil actif
    Use { name: String },
}

/// Liste lue dans `/config`, avec le nom du profil actif.
struct State {
    version: i64,
    profiles: Vec<Value>,
    active: String,
}

fn name_of(profile: &Value) -> &str {
    profile.get("name").and_then(Value::as_str).unwrap_or("")
}

fn read_state(client: &mut dyn JsonClient) -> Result<State> {
    let config = client.call("GET", "/config", None)?;
    let version = config
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("GET /config: version absente ou invalide"))?;
    let profiles = config.get("profiles").and_then(Value::as_array);
    let active = config.get("active_profile").and_then(Value::as_str);
    let (Some(profiles), Some(active)) = (profiles, active) else {
        bail!("GET /config: pas de profils (écran antérieur au schéma 11)");
    };
    Ok(State {
        version,
        profiles: profiles.clone(),
        active: active.to_string(),
    })
}

/// Envoie la liste et vérifie que l'écran la renvoie telle quelle.
fn write_state(client: &mut dyn JsonClient, state: &State) -> Result<()> {
    let body = json!({
        "version": state.version,
        "profiles": state.profiles,
        "active_profile": state.active,
    });
    let response = client.call("POST", "/config", Some(&body))?;
    let names = |list: &[Value]| {
        list.iter()
            .map(|p| name_of(p).to_string())
            .collect::<Vec<_>>()
    };
    let confirmed = response.get("active_profile").and_then(Value::as_str) == Some(&state.active)
        && response
            .get("profiles")
            .and_then(Value::as_array)
            .is_some_and(|list| names(list) == names(&state.profiles));
    if !confirmed {
        bail!("POST /config: profils non confirmés");
    }
    Ok(())
}

fn find(state: &State, name: &str) -> Result<usize> {
    state
        .profiles
        .iter()
        .position(|p| name_of(p) == name)
        .ok_or_else(|| anyhow!("profil inconnu : « {name} »"))
}

fn check_name(name: &str) -> Result<()> {
    let length = name.chars().count();
    if !(1..=NAME_MAX_CHARS).contains(&length) {
        bail!("le nom d'un profil fait 1 à {NAME_MAX_CHARS} caractères");
    }
    if name.trim() != name {
        bail!("le nom d'un profil n'a pas d'espace en tête ni en fin");
    }
    Ok(())
}

fn check_basket(basket: &str) -> Result<()> {
    if basket.chars().count() > BASKET_MAX_CHARS {
        bail!("le panier fait au plus {BASKET_MAX_CHARS} caractères");
    }
    Ok(())
}

/// 0 efface la dose ; sinon 3,0 à 30,0 g au dixième.
fn check_dose(dose: f64) -> Result<f64> {
    let tenths = (dose * 10.0).round();
    if (dose * 10.0 - tenths).abs() > 1e-6 {
        bail!("la dose va par pas de 0,1 g");
    }
    let dose = tenths / 10.0;
    if dose != 0.0 && !(DOSE_RANGE_G.0..=DOSE_RANGE_G.1).contains(&dose) {
        bail!(
            "la dose vaut 0 (non renseignée) ou {} à {} g",
            DOSE_RANGE_G.0,
            DOSE_RANGE_G.1
        );
    }
    Ok(dose)
}

fn apply_recipe(profile: &mut Value, dose: Option<f64>, basket: Option<&str>) -> Result<()> {
    if let Some(dose) = dose {
        profile["dose_g"] = json!(check_dose(dose)?);
    }
    if let Some(basket) = basket {
        check_basket(basket)?;
        profile["basket"] = json!(basket);
    }
    Ok(())
}

/// Nombre à la française, sans décimale inutile : « 8 », « 16,5 ».
fn grams(value: &Value) -> String {
    let v = value.as_f64().unwrap_or(0.0);
    let text = if (v - v.round()).abs() < 0.05 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    };
    text.replace('.', ",")
}

/// « 8 g → 22 g · LM1 41 mm », comme sur l'écran.
fn recipe(profile: &Value) -> String {
    let dose = profile.get("dose_g").and_then(Value::as_f64).unwrap_or(0.0);
    let target = profile
        .get("brew")
        .and_then(|b| b.get("target_weight_g"))
        .map_or_else(|| "?".to_string(), grams);
    let mut text = if dose > 0.0 {
        format!("{} g → {target} g", grams(&json!(dose)))
    } else {
        format!("→ {target} g")
    };
    let basket = profile.get("basket").and_then(Value::as_str).unwrap_or("");
    if !basket.is_empty() {
        text.push_str(" · ");
        text.push_str(basket);
    }
    text
}

fn list(state: &State) -> String {
    let width = state
        .profiles
        .iter()
        .map(|p| name_of(p).chars().count())
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for profile in &state.profiles {
        let name = name_of(profile);
        let mark = if name == state.active { '*' } else { ' ' };
        let pad = " ".repeat(width - name.chars().count());
        out.push_str(&format!("{mark} {name}{pad}  {}\n", recipe(profile)));
    }
    out
}

/// Exécute la commande et renvoie ce qu'il faut afficher.
pub fn run(action: &ProfileAction, client: &mut dyn JsonClient) -> Result<String> {
    let mut state = read_state(client).context("lecture des profils")?;
    match action {
        ProfileAction::List => Ok(list(&state)),
        ProfileAction::Add {
            name,
            from,
            dose,
            basket,
        } => {
            check_name(name)?;
            if state.profiles.iter().any(|p| name_of(p) == name) {
                bail!("un profil s'appelle déjà « {name} »");
            }
            if state.profiles.len() >= MAX_PROFILES {
                bail!("{MAX_PROFILES} profils au plus : en supprimer un d'abord");
            }
            let source = from.as_deref().unwrap_or(&state.active);
            let mut profile = state.profiles[find(&state, source)?].clone();
            profile["name"] = json!(name);
            apply_recipe(&mut profile, *dose, basket.as_deref())?;
            state.profiles.push(profile);
            write_state(client, &state)?;
            Ok(format!("profil « {name} » créé\n"))
        }
        ProfileAction::Rename { old, new } => {
            check_name(new)?;
            let index = find(&state, old)?;
            if old != new && state.profiles.iter().any(|p| name_of(p) == new) {
                bail!("un profil s'appelle déjà « {new} »");
            }
            state.profiles[index]["name"] = json!(new);
            if state.active == *old {
                state.active = new.clone();
            }
            write_state(client, &state)?;
            Ok(format!("profil « {old} » renommé « {new} »\n"))
        }
        ProfileAction::Set { name, dose, basket } => {
            if dose.is_none() && basket.is_none() {
                bail!("rien à changer : donner --dose et/ou --basket");
            }
            let index = find(&state, name)?;
            apply_recipe(&mut state.profiles[index], *dose, basket.as_deref())?;
            write_state(client, &state)?;
            Ok(format!(
                "profil « {name} » : {}\n",
                recipe(&state.profiles[index])
            ))
        }
        ProfileAction::Rm { name } => {
            let index = find(&state, name)?;
            if *name == state.active {
                bail!("« {name} » est le profil actif : choisir un autre profil à l'écran d'abord");
            }
            if state.profiles.len() == 1 {
                bail!("impossible de supprimer le dernier profil");
            }
            state.profiles.remove(index);
            write_state(client, &state)?;
            Ok(format!("profil « {name} » supprimé\n"))
        }
        ProfileAction::Use { name } => {
            find(&state, name)?;
            state.active = name.clone();
            write_state(client, &state)?;
            Ok(format!("profil actif : « {name} »\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::FakeClient;

    fn profile(name: &str, dose: f64, target: f64, basket: &str) -> Value {
        json!({
            "name": name, "dose_g": dose, "basket": basket,
            "brew": {"target_weight_g": target, "target_time_s": 27,
                     "target_pressure_bar": 9.0, "pump_pct": 100},
            "heating": {"brew_temperature_c": 91.0, "brew_preheat_time_s": 8.0},
        })
    }

    fn config() -> Value {
        json!({
            "version": 11,
            "active_profile": "single espresso",
            "profiles": [
                profile("single espresso", 8.0, 22.0, "LM1 41 mm"),
                profile("double espresso", 16.5, 36.0, "58 mm 18 g"),
                profile("turbo shot", 0.0, 45.0, ""),
            ],
        })
    }

    /// Écran simulé : sert `config()` en GET et renvoie le corps en POST.
    fn screen() -> FakeClient {
        FakeClient::new(|method, _, body| {
            Ok(if method == "GET" {
                config()
            } else {
                body.unwrap().clone()
            })
        })
    }

    fn posted(client: &FakeClient) -> Value {
        assert_eq!(client.calls[0].0, "GET");
        assert_eq!(client.calls.len(), 2, "{:?}", client.calls);
        assert_eq!(
            (client.calls[1].0.as_str(), client.calls[1].1.as_str()),
            ("POST", "/config")
        );
        client.calls[1].2.clone().unwrap()
    }

    fn names(body: &Value) -> Vec<&str> {
        body["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(name_of)
            .collect()
    }

    #[test]
    fn list_marks_the_active_profile_and_reads_only() {
        let mut client = screen();
        let out = run(&ProfileAction::List, &mut client).unwrap();
        assert_eq!(client.calls.len(), 1);
        assert_eq!(
            out,
            "* single espresso  8 g → 22 g · LM1 41 mm\n  \
             double espresso  16,5 g → 36 g · 58 mm 18 g\n  \
             turbo shot       → 45 g\n"
        );
    }

    #[test]
    fn add_copies_the_active_profile_by_default() {
        let mut client = FakeClient::new(|method, _, body| {
            let mut config = config();
            config["profiles"].as_array_mut().unwrap().pop();
            Ok(if method == "GET" {
                config
            } else {
                body.unwrap().clone()
            })
        });
        let action = ProfileAction::Add {
            name: "lungo".into(),
            from: None,
            dose: Some(18.0),
            basket: Some("58 mm 18 g".into()),
        };
        run(&action, &mut client).unwrap();
        let body = posted(&client);
        assert_eq!(body["version"], 11);
        assert_eq!(body["active_profile"], "single espresso");
        assert_eq!(
            names(&body),
            ["single espresso", "double espresso", "lungo"]
        );
        let added = &body["profiles"][2];
        assert_eq!(added["dose_g"], 18.0);
        assert_eq!(added["basket"], "58 mm 18 g");
        assert_eq!(added["brew"], body["profiles"][0]["brew"]);
    }

    #[test]
    fn add_from_another_profile_keeps_its_recipe_without_options() {
        let mut client = FakeClient::new(|method, _, body| {
            let mut config = config();
            config["profiles"].as_array_mut().unwrap().pop();
            Ok(if method == "GET" {
                config
            } else {
                body.unwrap().clone()
            })
        });
        let action = ProfileAction::Add {
            name: "copie".into(),
            from: Some("double espresso".into()),
            dose: None,
            basket: None,
        };
        run(&action, &mut client).unwrap();
        let body = posted(&client);
        assert_eq!(body["profiles"][2]["dose_g"], 16.5);
        assert_eq!(body["profiles"][2]["basket"], "58 mm 18 g");
    }

    #[test]
    fn add_refuses_duplicates_bad_names_and_a_fifth_profile() {
        let add = |name: &str| ProfileAction::Add {
            name: name.into(),
            from: None,
            dose: None,
            basket: None,
        };
        let mut client = screen();
        let error = run(&add("turbo shot"), &mut client)
            .unwrap_err()
            .to_string();
        assert!(error.contains("déjà"), "{error}");
        for bad in ["", " a", "a ", "un nom beaucoup trop long"] {
            assert!(run(&add(bad), &mut screen()).is_err(), "{bad:?}");
        }
        let mut full = FakeClient::new(|_, _, _| {
            let mut config = config();
            let list = config["profiles"].as_array_mut().unwrap();
            list.push(profile("quatrième", 0.0, 30.0, ""));
            Ok(config)
        });
        let error = run(&add("cinquième"), &mut full).unwrap_err().to_string();
        assert!(error.contains("4 profils"), "{error}");
        assert_eq!(full.calls.len(), 1);
    }

    #[test]
    fn rename_follows_the_active_profile() {
        let mut client = screen();
        let action = ProfileAction::Rename {
            old: "single espresso".into(),
            new: "ristretto".into(),
        };
        run(&action, &mut client).unwrap();
        let body = posted(&client);
        assert_eq!(body["active_profile"], "ristretto");
        assert_eq!(names(&body)[0], "ristretto");

        let mut client = screen();
        let action = ProfileAction::Rename {
            old: "turbo shot".into(),
            new: "lungo".into(),
        };
        run(&action, &mut client).unwrap();
        assert_eq!(posted(&client)["active_profile"], "single espresso");
    }

    #[test]
    fn rename_refuses_unknown_and_taken_names() {
        for (old, new) in [("absent", "x"), ("turbo shot", "double espresso")] {
            let action = ProfileAction::Rename {
                old: old.into(),
                new: new.into(),
            };
            let mut client = screen();
            assert!(run(&action, &mut client).is_err());
            assert_eq!(client.calls.len(), 1);
        }
    }

    #[test]
    fn set_changes_and_clears_the_recipe() {
        let mut client = screen();
        let action = ProfileAction::Set {
            name: "double espresso".into(),
            dose: Some(0.0),
            basket: Some(String::new()),
        };
        run(&action, &mut client).unwrap();
        let body = posted(&client);
        assert_eq!(body["profiles"][1]["dose_g"], 0.0);
        assert_eq!(body["profiles"][1]["basket"], "");

        let mut client = screen();
        let action = ProfileAction::Set {
            name: "turbo shot".into(),
            dose: Some(11.0),
            basket: None,
        };
        run(&action, &mut client).unwrap();
        let body = posted(&client);
        assert_eq!(body["profiles"][2]["dose_g"], 11.0);
        assert_eq!(body["profiles"][2]["basket"], "");
    }

    #[test]
    fn set_checks_the_dose_and_needs_an_option() {
        let set = |dose: Option<f64>, basket: Option<&str>| ProfileAction::Set {
            name: "turbo shot".into(),
            dose,
            basket: basket.map(String::from),
        };
        for action in [
            set(None, None),
            set(Some(2.0), None),
            set(Some(31.0), None),
            set(Some(8.25), None),
            set(None, Some("un panier de plus de vingt-quatre caractères")),
        ] {
            let mut client = screen();
            assert!(run(&action, &mut client).is_err(), "{action:?}");
            assert_eq!(client.calls.len(), 1, "{action:?}");
        }
    }

    #[test]
    fn rm_refuses_the_active_and_the_last_profile() {
        let mut client = screen();
        let action = ProfileAction::Rm {
            name: "single espresso".into(),
        };
        let error = run(&action, &mut client).unwrap_err().to_string();
        assert!(
            error.contains("choisir un autre profil à l'écran d'abord"),
            "{error}"
        );
        assert_eq!(client.calls.len(), 1);

        let mut alone = FakeClient::new(|_, _, _| {
            Ok(json!({
                "version": 11,
                "active_profile": "a",
                "profiles": [profile("a", 0.0, 22.0, "")],
            }))
        });
        let action = ProfileAction::Rm { name: "a".into() };
        assert!(run(&action, &mut alone).is_err());

        // Le dernier profil d'une liste dont l'actif est ailleurs reste supprimable.
        let mut client = screen();
        let action = ProfileAction::Rm {
            name: "double espresso".into(),
        };
        run(&action, &mut client).unwrap();
        assert_eq!(names(&posted(&client)), ["single espresso", "turbo shot"]);
    }

    #[test]
    fn use_switches_the_active_profile() {
        let mut client = screen();
        run(
            &ProfileAction::Use {
                name: "turbo shot".into(),
            },
            &mut client,
        )
        .unwrap();
        let body = posted(&client);
        assert_eq!(body["active_profile"], "turbo shot");
        assert_eq!(names(&body).len(), 3);
        let mut client = screen();
        assert!(
            run(
                &ProfileAction::Use {
                    name: "absent".into()
                },
                &mut client
            )
            .is_err()
        );
        assert_eq!(client.calls.len(), 1);
    }

    #[test]
    fn unconfirmed_answers_and_old_screens_are_reported() {
        let mut client = FakeClient::new(|method, _, _| Ok(config_for(method)));
        fn config_for(method: &str) -> Value {
            let mut config = config();
            if method == "POST" {
                config["active_profile"] = json!("turbo shot");
            }
            config
        }
        let action = ProfileAction::Rm {
            name: "turbo shot".into(),
        };
        let error = run(&action, &mut client).unwrap_err().to_string();
        assert!(error.contains("non confirmés"), "{error}");

        let mut old = FakeClient::new(|_, _, _| Ok(json!({"version": 10})));
        let error = format!("{:#}", run(&ProfileAction::List, &mut old).unwrap_err());
        assert!(error.contains("schéma 11"), "{error}");
    }
}

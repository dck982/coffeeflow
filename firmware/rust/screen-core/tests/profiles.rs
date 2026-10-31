//! Profils d'infusion : blob NVS du schéma 12, `/config` (GET, POST, refus),
//! choix du profil, invariant « champs plats = profil actif » et en-tête de
//! la capture HF.

#![allow(clippy::field_reassign_with_default)]

mod common;

use common::Rig;
use protocol::FIRMWARE_VERSION;
use screen_core::config::{
    Config, ConfigManager, ConfigStatus, MAX_PROFILES, MemoryConfigStore, Profile, blob_size,
    encode_blob, validate,
};
use screen_core::core::{Action, ActionCommand, ActionSource, Input, Reply};
use screen_core::hf_capture::{self, View};
use screen_core::http::{HttpResponse, handle_get_config, handle_post_config, hf_capture_document};
use screen_core::json::{self, Json};

/// Corps d'un élément de `profiles` aux réglages par défaut, dont le poids
/// cible vaut `weight`.
fn profile_json(name: &str, dose: &str, basket: &str, weight: &str) -> String {
    format!(
        r#"{{"name":"{name}","dose_g":{dose},"basket":"{basket}",
"brew":{{"target_weight_g":{weight},"stop_lead_g":4.0,"target_time_s":28,"target_pressure_bar":9.0,"pump_pct":100}},
"heating":{{"brew_temperature_c":90.0,"brew_preheat_time_s":2.5}},
"filling":{{"time_s":10,"pressure_bar":1.0,"pump_pct":100}},
"preinfusion":{{"time":true,"weight":false,"time_s":4,"pump_pct":35}}}}"#
    )
}

fn profiles_body(items: &[String]) -> String {
    format!(r#"{{"version":12,"profiles":[{}]}}"#, items.join(","))
}

fn post(rig: &mut Rig, body: &str) -> HttpResponse {
    let clock = rig.clock();
    handle_post_config(&mut rig.core, clock, body.as_bytes())
}

fn field_of(response: &HttpResponse) -> Option<String> {
    let root = json::parse(response.body.as_bytes())?;
    root.get("field").and_then(Json::as_str).map(str::to_string)
}

/// Trois profils aux poids cibles distincts ; le profil `active` est actif.
fn three_profiles(active: u8) -> Config {
    let base = Config::default();
    let mut c = base.clone();
    let mut turbo = base.clone();
    turbo.target_weight_g = 30.0;
    turbo.target_time_s = 15;
    let mut lungo = base.clone();
    lungo.target_weight_g = 60.0;
    lungo.brew_temperature_c = 93.5;
    c.profiles_mut()[1] = Profile::from_flat(&turbo, "turbo shot", 0.0, "");
    c.profiles_mut()[2] = Profile::from_flat(&lungo, "lungo é°", 14.5, "IMS 58 mm");
    c.profile_count = 3;
    c.active_profile = active;
    let active = c.profiles[usize::from(active)];
    active.give_settings(&mut c);
    c
}

fn rig_with(config: &Config) -> Rig {
    let mut rig = Rig::new();
    let clock = rig.clock();
    assert_eq!(rig.core.put_config(clock, config).status, ConfigStatus::Ok);
    rig
}

// --- Blob NVS -----------------------------------------------------------------

#[test]
fn blob_round_trips_every_profile() {
    let mut config = three_profiles(2);
    config.revision = 7;
    config.heating_enabled = false;
    config.scace_enabled = true;
    config.purge_max_s = 30;
    let blob = encode_blob(&config);
    assert_eq!(blob.len(), blob_size());
    assert_eq!(blob_size(), 444);
    let mut store = MemoryConfigStore {
        a: Some(blob),
        ..MemoryConfigStore::default()
    };
    let (manager, report) = ConfigManager::init(&mut store);
    assert!(report.found && !report.migrated && !report.normalized);
    assert_eq!(*manager.get(), config);
    assert_eq!(manager.get().profiles()[2].name(), "lungo é°");
    assert_eq!(store.writes, 0);
}

#[test]
fn unused_profile_records_are_zero_in_the_blob() {
    let blob = encode_blob(&Config::default());
    // Les trois derniers enregistrements (104 octets chacun) précèdent la
    // somme de contrôle.
    let end = blob.len() - 4;
    assert!(blob[end - 3 * 104..end].iter().all(|b| *b == 0));
}

#[test]
fn corrupt_profile_counts_are_ignored() {
    let mut c = Config::default();
    c.profile_count = 5;
    let mut store = MemoryConfigStore {
        a: Some(encode_blob(&c)),
        ..MemoryConfigStore::default()
    };
    assert!(!ConfigManager::init(&mut store).1.found);
    let mut c = Config::default();
    c.active_profile = 1;
    let mut store = MemoryConfigStore {
        a: Some(encode_blob(&c)),
        ..MemoryConfigStore::default()
    };
    assert!(!ConfigManager::init(&mut store).1.found);
}

// --- Validation ---------------------------------------------------------------

#[test]
fn profile_rules_are_validated_with_indexed_field_names() {
    let check = |edit: &dyn Fn(&mut Config)| {
        let mut c = three_profiles(0);
        edit(&mut c);
        validate(&c)
    };
    assert_eq!(check(&|_| {}), None);
    assert_eq!(check(&|c| c.profile_count = 0), Some("profiles"));
    assert_eq!(check(&|c| c.profile_count = 5), Some("profiles"));
    assert_eq!(check(&|c| c.active_profile = 3), Some("active_profile"));
    assert_eq!(
        check(&|c| {
            c.profiles_mut()[2].set_name("turbo shot");
        }),
        Some("profiles[2].name")
    );
    assert_eq!(
        check(&|c| {
            c.profiles_mut()[1].set_name("");
        }),
        Some("profiles[1].name")
    );
    for bad in [
        " turbo",
        "turbo ",
        "tab\there",
        "señor",
        "😀",
        "un nom de vingt-et-un",
    ] {
        assert_eq!(
            check(&|c| {
                c.profiles_mut()[1].set_name(bad);
            }),
            Some("profiles[1].name"),
            "{bad:?}"
        );
    }
    assert_eq!(
        check(&|c| {
            c.profiles_mut()[1].set_name("12345678901234567890");
        }),
        None
    );
    // Vingt caractères, mais plus de 32 octets : ne tient pas dans le tampon.
    let mut profile = Profile::EMPTY;
    assert!(!profile.set_name("éééééééééééééééééééé"));
    assert_eq!(profile.name(), "");
    assert_eq!(
        check(&|c| {
            c.profiles_mut()[0].set_basket("1234567890123456789012345");
        }),
        Some("profiles[0].basket")
    );
    assert_eq!(
        check(&|c| {
            c.profiles_mut()[0].set_basket("trailing ");
        }),
        Some("profiles[0].basket")
    );
    assert_eq!(
        check(&|c| c.profiles_mut()[0].dose_g = 2.9),
        Some("profiles[0].dose_g")
    );
    assert_eq!(
        check(&|c| c.profiles_mut()[0].dose_g = 30.1),
        Some("profiles[0].dose_g")
    );
    assert_eq!(
        check(&|c| c.profiles_mut()[0].dose_g = 8.05),
        Some("profiles[0].dose_g")
    );
    assert_eq!(check(&|c| c.profiles_mut()[0].dose_g = 3.0), None);
    assert_eq!(check(&|c| c.profiles_mut()[0].dose_g = 0.0), None);
    assert_eq!(
        check(&|c| c.profiles_mut()[2].target_pressure_bar = 3.9),
        Some("profiles[2].brew.target_pressure_bar")
    );
    assert_eq!(
        check(&|c| c.profiles_mut()[1].stop_lead_g = 20.5),
        Some("profiles[1].brew.stop_lead_g")
    );
}

// --- /config --------------------------------------------------------------------

#[test]
fn get_then_post_is_neutral() {
    let mut rig = rig_with(&three_profiles(1));
    let before = rig.core.config().clone();
    let document = handle_get_config(&rig.core).body;
    let response = post(&mut rig, &document);
    assert_eq!(response.status, "200 OK", "{}", response.body);
    let after = rig.core.config().clone();
    assert_eq!(after.revision, before.revision + 1);
    let mut expected = before;
    expected.revision = after.revision;
    assert_eq!(after, expected);
    assert_eq!(handle_get_config(&rig.core).body, document);
}

#[test]
fn get_lists_every_profile_and_mirrors_the_active_one() {
    let rig = rig_with(&three_profiles(2));
    let root = json::parse(handle_get_config(&rig.core).body.as_bytes()).unwrap();
    assert_eq!(
        root.get("active_profile").and_then(Json::as_str),
        Some("lungo é°")
    );
    let weight = |node: &Json| {
        node.get("brew")
            .and_then(|b| b.get("target_weight_g"))
            .and_then(Json::as_f64)
    };
    assert_eq!(weight(&root), Some(60.0));
    let Some(Json::Array(profiles)) = root.get("profiles") else {
        panic!("profiles");
    };
    assert_eq!(profiles.len(), 3);
    assert_eq!(weight(&profiles[1]), Some(30.0));
    assert_eq!(
        profiles[2].get("basket").and_then(Json::as_str),
        Some("IMS 58 mm")
    );
    assert_eq!(profiles[2].get("dose_g").and_then(Json::as_f64), Some(14.5));
}

#[test]
fn post_refusals_name_the_faulty_field_and_change_nothing() {
    let mut rig = rig_with(&three_profiles(0));
    let before = rig.core.config().clone();
    let writes = rig.core.store.writes;
    let ok = |name: &str| profile_json(name, "8.0", "LM1", "36.0");
    let five: Vec<String> = (0..5).map(|i| ok(&format!("p{i}"))).collect();
    // Le premier `pump_pct` est celui de `brew`.
    let missing = ok("c").replacen(r#","pump_pct":100"#, "", 1);
    let cases: Vec<(String, &str, String)> = vec![
        (profiles_body(&[]), "invalid_value", "profiles".into()),
        (profiles_body(&five), "invalid_value", "profiles".into()),
        (
            r#"{"version":12,"profiles":{}}"#.into(),
            "invalid_value",
            "profiles".into(),
        ),
        (
            profiles_body(&[ok("a"), ok("b"), ok("a")]),
            "invalid_value",
            "profiles[2].name".into(),
        ),
        (
            profiles_body(&[ok("a"), ok("b\\u00e9\\u00ef"), ok("c\\u0001")]),
            "invalid_value",
            "profiles[2].name".into(),
        ),
        (
            profiles_body(&[ok("vingt et un caracteres")]),
            "invalid_value",
            "profiles[0].name".into(),
        ),
        (
            profiles_body(&[ok(" lead")]),
            "invalid_value",
            "profiles[0].name".into(),
        ),
        (
            profiles_body(&[
                ok("a"),
                profile_json("b", "8.0", "0123456789012345678901234", "36.0"),
            ]),
            "invalid_value",
            "profiles[1].basket".into(),
        ),
        (
            profiles_body(&[ok("a"), profile_json("b", "2.0", "x", "36.0")]),
            "invalid_value",
            "profiles[1].dose_g".into(),
        ),
        (
            profiles_body(&[ok("a"), profile_json("b", "8.0", "x", "9.5")]),
            "invalid_value",
            "profiles[1].brew.target_weight_g".into(),
        ),
        (
            profiles_body(&[ok("a"), ok("b"), missing]),
            "invalid_value",
            "profiles[2].brew.pump_pct".into(),
        ),
        (
            r#"{"version":12,"profiles":[{"name":"a"}]}"#.into(),
            "invalid_value",
            "profiles[0].dose_g".into(),
        ),
        (
            r#"{"version":12,"active_profile":"inconnu"}"#.into(),
            "invalid_value",
            "active_profile".into(),
        ),
        (
            r#"{"version":12,"active_profile":3}"#.into(),
            "invalid_value",
            "active_profile".into(),
        ),
        (
            // Le profil actif ("single espresso") disparaît de la liste.
            profiles_body(&[ok("a")]),
            "invalid_value",
            "active_profile".into(),
        ),
        (
            r#"{"version":10,"active_profile":"turbo shot"}"#.into(),
            "unknown_version",
            "version".into(),
        ),
    ];
    for (body, error, field) in cases {
        let response = post(&mut rig, &body);
        assert_eq!(response.status, "400 Bad Request", "{body}");
        let root = json::parse(response.body.as_bytes()).unwrap();
        assert_eq!(
            root.get("error").and_then(Json::as_str),
            Some(error),
            "{body}"
        );
        assert_eq!(
            field_of(&response).as_deref(),
            Some(field.as_str()),
            "{body}"
        );
        assert_eq!(*rig.core.config(), before, "{body}");
        assert_eq!(rig.core.store.writes, writes, "{body}");
    }
}

#[test]
fn long_faulty_keys_stay_readable() {
    let mut rig = Rig::new();
    let body = profiles_body(&[profile_json("a", "8.0", "x", "36.0").replace(
        r#""brew":{"#,
        r#""brew":{"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz":1,"#,
    )]);
    let response = post(&mut rig, &body);
    assert_eq!(response.status, "400 Bad Request");
    let field = field_of(&response).unwrap();
    assert!(field.starts_with("profiles[0].brew.zzzz"), "{field}");
    assert_eq!(field.len(), 47);
}

#[test]
fn replacing_the_list_finds_the_active_profile_by_name() {
    let mut rig = rig_with(&three_profiles(1)); // "turbo shot" actif
    let body = profiles_body(&[
        profile_json("lungo", "9.0", "x", "50.0"),
        profile_json("turbo shot", "7.5", "y", "28.0"),
    ]);
    let response = post(&mut rig, &body);
    assert_eq!(response.status, "200 OK", "{}", response.body);
    let c = rig.core.config().clone();
    assert_eq!((c.profile_count, c.active_profile), (2, 1));
    assert_eq!(c.active().name(), "turbo shot");
    assert_eq!(c.target_weight_g, 28.0);
    assert_eq!(c.profiles()[0].name(), "lungo");
    // Les emplacements au-delà du nombre de profils sont remis à zéro.
    assert_eq!(c.profiles[2], Profile::EMPTY);
    // Persisté.
    let mut store = rig.core.store.clone();
    assert_eq!(*ConfigManager::init(&mut store).0.get(), c);
}

#[test]
fn active_profile_by_name_then_sections_apply_to_it() {
    let mut rig = rig_with(&three_profiles(0));
    // `brew` précède `active_profile` dans le document, mais s'applique au
    // profil choisi.
    let response = post(
        &mut rig,
        r#"{"version":12,"brew":{"target_weight_g":33.5},"active_profile":"turbo shot"}"#,
    );
    assert_eq!(response.status, "200 OK", "{}", response.body);
    let c = rig.core.config().clone();
    assert_eq!(c.active_profile, 1);
    assert_eq!(c.target_weight_g, 33.5);
    assert_eq!(c.profiles()[1].target_weight_g, 33.5);
    assert_eq!(c.profiles()[0].target_weight_g, 36.0);
    // Liste et choix dans le même document : le nom vise la nouvelle liste.
    let body = format!(
        r#"{{"version":12,"active_profile":"b","profiles":[{},{}]}}"#,
        profile_json("a", "8.0", "x", "36.0"),
        profile_json("b", "8.0", "x", "40.0")
    );
    assert_eq!(post(&mut rig, &body).status, "200 OK");
    let c = rig.core.config().clone();
    assert_eq!(
        (c.profile_count, c.active_profile, c.target_weight_g),
        (2, 1, 40.0)
    );
}

#[test]
fn head_sections_edit_the_active_profile_and_the_machine() {
    let mut rig = rig_with(&three_profiles(2));
    let response = post(
        &mut rig,
        r#"{"version":12,"brew":{"target_time_s":33},"heating":{"enabled":false,"brew_temperature_c":88.5},"purge":{"max_s":15}}"#,
    );
    assert_eq!(response.status, "200 OK", "{}", response.body);
    let c = rig.core.config().clone();
    assert_eq!(c.profiles()[2].target_time_s, 33);
    assert_eq!(c.profiles()[2].brew_temperature_c, 88.5);
    assert_eq!(c.profiles()[0].target_time_s, 28);
    assert_eq!(c.profiles()[0].brew_temperature_c, 90.0);
    assert!(!c.heating_enabled);
    assert_eq!(c.purge_max_s, 15);
    // `enabled` appartient à la machine, pas à un élément de profil.
    let body = profiles_body(&[profile_json("a", "8.0", "x", "36.0")
        .replace(r#""heating":{"#, r#""heating":{"enabled":true,"#)]);
    let response = post(&mut rig, &body);
    assert_eq!(
        field_of(&response).as_deref(),
        Some("profiles[0].heating.enabled")
    );
}

// --- Choix du profil ------------------------------------------------------------------

#[test]
fn select_profile_persists_and_copies_the_settings() {
    let mut rig = rig_with(&three_profiles(0));
    let before = rig.core.config().clone();
    let writes = rig.core.store.writes;
    let clock = rig.clock();
    let result = rig.core.select_profile(clock, 2);
    assert_eq!(result.status, ConfigStatus::Ok);
    let c = rig.core.config().clone();
    assert_eq!(c.revision, before.revision + 1);
    assert_eq!(c.active_profile, 2);
    assert_eq!((c.target_weight_g, c.brew_temperature_c), (60.0, 93.5));
    assert_eq!(c.profiles, before.profiles);
    assert_eq!(rig.core.store.writes, writes + 1);
    let mut store = rig.core.store.clone();
    assert_eq!(*ConfigManager::init(&mut store).0.get(), c);

    // Même profil : rien à écrire. Indice inconnu : refusé.
    let clock = rig.clock();
    assert_eq!(rig.core.select_profile(clock, 2).status, ConfigStatus::Ok);
    assert_eq!(rig.core.store.writes, writes + 1);
    let result = rig.core.select_profile(clock, 3);
    assert_eq!(
        (result.status, result.field),
        (ConfigStatus::InvalidValue, Some("active_profile"))
    );
    assert_eq!(*rig.core.config(), c);
}

#[test]
fn select_profile_through_the_input_loop() {
    let mut rig = rig_with(&three_profiles(0));
    let clock = rig.clock();
    let Reply::Config(result) = rig.core.handle(clock, Input::SelectProfile(1)) else {
        panic!("réponse de configuration");
    };
    assert_eq!(result.status, ConfigStatus::Ok);
    assert_eq!(rig.core.config().active().name(), "turbo shot");
}

#[test]
fn select_profile_is_refused_during_a_cycle() {
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    assert_eq!(
        rig.core.put_config(clock, &three_profiles(0)).status,
        ConfigStatus::Ok
    );
    rig.advance(4000);
    let clock = rig.clock();
    let started = rig.core.perform_action(
        clock,
        &ActionCommand::new(Action::StartBrew),
        ActionSource::Ui,
    );
    assert_eq!(started.reason, "ok");
    let before = rig.core.config().clone();
    let writes = rig.core.store.writes;
    let clock = rig.clock();
    let result = rig.core.select_profile(clock, 1);
    assert_eq!(
        (result.status, result.field),
        (ConfigStatus::Busy, Some("cycle"))
    );
    assert_eq!(*rig.core.config(), before);
    assert_eq!(rig.core.store.writes, writes);
}

#[test]
fn select_profile_changes_nothing_when_nvs_fails() {
    let mut rig = rig_with(&three_profiles(0));
    let before = rig.core.config().clone();
    rig.core.store.fail_writes = true;
    let clock = rig.clock();
    let result = rig.core.select_profile(clock, 1);
    assert_eq!(
        (result.status, result.field),
        (ConfigStatus::StorageError, Some("nvs"))
    );
    assert_eq!(*rig.core.config(), before);
}

// --- Invariant ------------------------------------------------------------------------

#[test]
fn editing_a_flat_field_writes_the_active_profile() {
    let mut rig = rig_with(&three_profiles(1));
    let mut edited = rig.core.config().clone();
    edited.target_weight_g = 31.5;
    edited.preinfusion_time_s = 6;
    edited.purge_pump_pct = 80; // machine
    let clock = rig.clock();
    assert_eq!(rig.core.put_config(clock, &edited).status, ConfigStatus::Ok);
    let c = rig.core.config().clone();
    assert_eq!(c.profiles()[1].target_weight_g, 31.5);
    assert_eq!(c.profiles()[1].preinfusion_time_s, 6);
    assert_eq!(c.profiles()[0].target_weight_g, 36.0);
    assert_eq!(c.profiles()[2].target_weight_g, 60.0);
    assert_eq!(c.purge_pump_pct, 80);
    let mut store = rig.core.store.clone();
    assert_eq!(
        ConfigManager::init(&mut store).0.get().profiles()[1].target_weight_g,
        31.5
    );
}

#[test]
fn a_candidate_that_changes_the_active_profile_takes_its_settings() {
    let mut rig = rig_with(&three_profiles(0));
    let mut candidate = rig.core.config().clone();
    candidate.active_profile = 2;
    let clock = rig.clock();
    assert_eq!(
        rig.core.put_config(clock, &candidate).status,
        ConfigStatus::Ok
    );
    let c = rig.core.config().clone();
    assert_eq!(
        (c.active_profile, c.target_weight_g, c.brew_temperature_c),
        (2, 60.0, 93.5)
    );
    assert_eq!(c.profiles()[0].target_weight_g, 36.0);
}

#[test]
fn at_most_four_profiles() {
    let mut c = three_profiles(0);
    c.profiles_mut()[3] = Profile::from_flat(&Config::default(), "quatre", 0.0, "");
    c.profile_count = MAX_PROFILES as u8;
    assert_eq!(validate(&c), None);
    let mut rig = rig_with(&c);
    assert_eq!(rig.core.config().profiles().len(), 4);
    let clock = rig.clock();
    assert_eq!(rig.core.select_profile(clock, 3).status, ConfigStatus::Ok);
    // Le document de quatre profils tient dans un corps de requête.
    let document = handle_get_config(&rig.core).body;
    assert!(
        document.len() <= screen_core::http::MAX_BODY,
        "{}",
        document.len()
    );
    assert_eq!(post(&mut rig, &document).status, "200 OK");
}

// --- Capture HF ------------------------------------------------------------------------

fn header_of(rig: &Rig) -> Json {
    let response = hf_capture_document(&rig.core, None);
    assert_eq!(response.status, "200 OK");
    json::parse(response.body.as_bytes()).expect("document JSON valide")
}

fn run_brew(rig: &mut Rig) {
    let clock = rig.clock();
    let started = rig.core.perform_action(
        clock,
        &ActionCommand::new(Action::StartBrew),
        ActionSource::Ui,
    );
    assert_eq!(started.reason, "ok");
    rig.collect();
    rig.advance(6500);
    rig.advance(11_000);
}

#[test]
fn hf_capture_v3_header_carries_firmware_and_the_starting_profile() {
    let mut config = three_profiles(2);
    config.brew_preheat_time_s = 0.0;
    config.profiles_mut()[2].brew_preheat_time_s = 0.0;
    config.profiles_mut()[2].filling_time_s = 1;
    config.profiles_mut()[2].preinfusion_time_s = 0;
    config.profiles_mut()[2].target_time_s = 5;
    config.profiles_mut()[2].brew_temperature_c = 90.0;
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    assert_eq!(rig.core.put_config(clock, &config).status, ConfigStatus::Ok);
    rig.advance(4000);
    run_brew(&mut rig);

    let root = header_of(&rig);
    assert_eq!(
        root.get("schema").and_then(Json::as_str),
        Some("coffeeflow.hf_capture.v3")
    );
    let firmware = root.get("firmware").expect("firmware");
    let screen = format!(
        "{}.{}.{}",
        FIRMWARE_VERSION.major, FIRMWARE_VERSION.minor, FIRMWARE_VERSION.patch
    );
    assert_eq!(
        firmware.get("screen").and_then(Json::as_str),
        Some(screen.as_str())
    );
    assert_eq!(
        firmware.get("sensors").and_then(Json::as_str),
        Some("0.4.0")
    );
    // Le profil est celui du départ : la forme d'un élément de `profiles`.
    let profile = root.get("profile").expect("profile");
    assert_eq!(profile.get("name").and_then(Json::as_str), Some("lungo é°"));
    assert_eq!(profile.get("dose_g").and_then(Json::as_f64), Some(14.5));
    assert_eq!(
        profile.get("basket").and_then(Json::as_str),
        Some("IMS 58 mm")
    );
    let config_root = json::parse(handle_get_config(&rig.core).body.as_bytes()).unwrap();
    let Some(Json::Array(profiles)) = config_root.get("profiles") else {
        panic!("profiles");
    };
    assert_eq!(profile, &profiles[2]);
    // Clés dans l'ordre : firmware, profile, session.
    let text = hf_capture_document(&rig.core, None).body;
    let at = |key: &str| text.find(&format!("\"{key}\":")).unwrap();
    assert!(at("dropped_samples") < at("firmware"));
    assert!(at("firmware") < at("profile"));
    assert!(at("profile") < at("session"));
    assert!(at("session") < at("samples"));

    // Un changement de profil après l'infusion ne réécrit pas la capture.
    let clock = rig.clock();
    assert_eq!(rig.core.select_profile(clock, 0).status, ConfigStatus::Ok);
    assert_eq!(
        header_of(&rig)
            .get("profile")
            .and_then(|p| p.get("name"))
            .and_then(Json::as_str),
        Some("lungo é°")
    );
}

#[test]
fn hf_capture_other_origins_carry_firmware_without_profile() {
    let mut rig = Rig::new();
    rig.ready();
    let clock = rig.clock();
    rig.core.perform_action(
        clock,
        &ActionCommand::new(Action::PurgePress),
        ActionSource::Ui,
    );
    rig.advance(500);
    let clock = rig.clock();
    rig.core.perform_action(
        clock,
        &ActionCommand::new(Action::PurgeRelease),
        ActionSource::Ui,
    );
    rig.advance(6000);
    let root = header_of(&rig);
    assert_eq!(root.get("origin").and_then(Json::as_str), Some("purge"));
    assert!(root.get("firmware").is_some());
    assert!(root.get("profile").is_none());
    assert!(root.get("session").is_none());
}

#[test]
fn unknown_sensors_version_is_null() {
    let info = hf_capture::Info::default();
    let context = hf_capture::Context {
        screen_version: (0, 4, 22),
        sensors_version: None,
        profile: None,
    };
    let header = hf_capture::export_header(&info, View::Both, None, &context);
    assert!(header.contains(r#""firmware":{"screen":"0.4.22","sensors":null},"samples":["#));
}

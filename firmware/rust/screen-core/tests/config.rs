//! Configuration : défauts, validation, blob NVS identique à celui du C++,
//! migrations des schémas 1 à 9, écriture alternée des deux emplacements.
//!
//! `vectors/config_blobs.txt` vient du vrai compilateur C sur les structures
//! de `config.cpp` (voir `vectors/gen_config_blobs.sh`).

#![allow(clippy::field_reassign_with_default)]

use screen_core::config::{
    CONFIG_SCHEMA_VERSION, Config, ConfigManager, ConfigStatus, ConfigStore, MemoryConfigStore,
    PreinfusionMode, RampdownMode, Slot, blob_size, encode_blob, validate,
};

const BLOBS: &str = include_str!("vectors/config_blobs.txt");

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn blob(version: &str, slot: &str) -> Vec<u8> {
    BLOBS
        .lines()
        .find_map(|line| {
            let mut parts = line.split(' ');
            (parts.next() == Some(version) && parts.next() == Some(slot))
                .then(|| hex(parts.next().unwrap()))
        })
        .unwrap_or_else(|| panic!("{version} {slot}"))
}

fn store_with(version: &str) -> MemoryConfigStore {
    MemoryConfigStore {
        a: Some(blob(version, "a")),
        b: Some(blob(version, "b")),
        ..MemoryConfigStore::default()
    }
}

#[test]
fn defaults_match_the_cpp_defaults() {
    let c = Config::default();
    assert_eq!(c.version, 10);
    assert_eq!(c.revision, 0);
    assert_eq!(
        (c.target_weight_g, c.target_time_s, c.target_pressure_bar),
        (36.0, 28, 9.0)
    );
    assert_eq!((c.brew_temperature_c, c.heating_enabled), (90.0, true));
    assert_eq!(c.brew_preheat_time_s, 2.5);
    assert_eq!(
        (c.filling_time_s, c.filling_pressure_bar, c.filling_pump_pct),
        (10, 1.0, 100)
    );
    assert_eq!(c.preinfusion_mode, PreinfusionMode::TIME);
    assert_eq!((c.preinfusion_time_s, c.preinfusion_pump_pct), (4, 35));
    assert_eq!(c.rampdown_mode, RampdownMode::None);
    assert_eq!(
        (
            c.rampdown_lead_time_s,
            c.rampdown_lead_weight_g,
            c.rampdown_pressure_drop_bar
        ),
        (3.0, 4.0, 1.0)
    );
    assert_eq!(
        (c.brew_pump_pct, c.purge_pump_pct, c.purge_max_s),
        (100, 100, 20)
    );
    assert_eq!(
        (c.dim_after_s, c.standby_after_s, c.scace_enabled),
        (240, 1800, false)
    );
    assert_eq!(validate(&c), None);
}

#[test]
fn validation_bounds_and_steps() {
    let ok = |edit: &dyn Fn(&mut Config)| {
        let mut c = Config::default();
        edit(&mut c);
        validate(&c)
    };
    assert_eq!(ok(&|c| c.version = 9), Some("version"));
    assert_eq!(
        ok(&|c| c.target_weight_g = 9.5),
        Some("brew.target_weight_g")
    );
    assert_eq!(ok(&|c| c.target_weight_g = 100.0), None);
    assert_eq!(
        ok(&|c| c.target_weight_g = 36.25),
        Some("brew.target_weight_g")
    );
    assert_eq!(
        ok(&|c| c.target_weight_g = f32::NAN),
        Some("brew.target_weight_g")
    );
    assert_eq!(ok(&|c| c.target_time_s = 4), Some("brew.target_time_s"));
    assert_eq!(ok(&|c| c.target_time_s = 61), Some("brew.target_time_s"));
    assert_eq!(
        ok(&|c| c.target_pressure_bar = 5.9),
        Some("brew.target_pressure_bar")
    );
    assert_eq!(ok(&|c| c.target_pressure_bar = 12.0), None);
    assert_eq!(
        ok(&|c| c.brew_temperature_c = 49.5),
        Some("heating.brew_temperature_c")
    );
    assert_eq!(
        ok(&|c| c.brew_temperature_c = 95.5),
        Some("heating.brew_temperature_c")
    );
    assert_eq!(ok(&|c| c.brew_temperature_c = 95.0), None);
    assert_eq!(
        ok(&|c| c.brew_preheat_time_s = 15.5),
        Some("heating.brew_preheat_time_s")
    );
    assert_eq!(ok(&|c| c.brew_preheat_time_s = 0.0), None);
    assert_eq!(ok(&|c| c.filling_time_s = 0), Some("filling.time_s"));
    assert_eq!(ok(&|c| c.filling_time_s = 21), Some("filling.time_s"));
    assert_eq!(
        ok(&|c| c.filling_pressure_bar = 0.2),
        Some("filling.pressure_bar")
    );
    assert_eq!(ok(&|c| c.filling_pressure_bar = 2.0), None);
    assert_eq!(ok(&|c| c.filling_pump_pct = 15), Some("filling.pump_pct"));
    assert_eq!(ok(&|c| c.filling_pump_pct = 22), Some("filling.pump_pct"));
    assert_eq!(
        ok(&|c| c.preinfusion_mode = PreinfusionMode(0x02)),
        Some("preinfusion.mode")
    );
    assert_eq!(
        ok(&|c| c.preinfusion_mode = PreinfusionMode::TIME | PreinfusionMode::WEIGHT),
        None
    );
    assert_eq!(
        ok(&|c| c.preinfusion_time_s = 21),
        Some("preinfusion.time_s")
    );
    assert_eq!(ok(&|c| c.preinfusion_pump_pct = 0), None);
    assert_eq!(
        ok(&|c| c.preinfusion_pump_pct = 7),
        Some("preinfusion.pump_pct")
    );
    assert_eq!(
        ok(&|c| c.rampdown_lead_time_s = 15.5),
        Some("rampdown.lead_time_s")
    );
    assert_eq!(
        ok(&|c| c.rampdown_lead_weight_g = 20.5),
        Some("rampdown.lead_weight_g")
    );
    assert_eq!(
        ok(&|c| c.rampdown_pressure_drop_bar = 0.0),
        Some("rampdown.pressure_drop_bar")
    );
    assert_eq!(ok(&|c| c.brew_pump_pct = 45), Some("brew.pump_pct"));
    assert_eq!(ok(&|c| c.brew_pump_pct = 50), None);
    assert_eq!(ok(&|c| c.purge_pump_pct = 15), Some("purge.pump_pct"));
    assert_eq!(ok(&|c| c.purge_max_s = 7), Some("purge.max_s"));
    assert_eq!(ok(&|c| c.purge_max_s = 65), Some("purge.max_s"));
    assert_eq!(ok(&|c| c.dim_after_s = 90), Some("ui.dim_after_s"));
    assert_eq!(ok(&|c| c.standby_after_s = 400), Some("ui.standby_after_s"));
    assert_eq!(
        ok(&|c| {
            c.dim_after_s = 600;
            c.standby_after_s = 300;
        }),
        Some("ui.standby_after_s")
    );
}

#[test]
fn v10_blob_layout_matches_the_c_layout() {
    // Le C laisse un remplissage indéterminé (0xFF dans ces vecteurs) là où
    // nous écrivons des zéros : on compare donc la taille, puis les valeurs
    // relues des deux côtés, et les octets hors remplissage.
    assert_eq!(blob_size(), 80);
    let mut defaults = Config::default();
    defaults.revision = 3;
    let mut changed = Config::default();
    changed.revision = 4;
    changed.target_weight_g = 41.5;
    changed.target_time_s = 31;
    changed.heating_enabled = false;
    changed.preinfusion_mode = PreinfusionMode::TIME | PreinfusionMode::WEIGHT;
    changed.rampdown_mode = RampdownMode::PressureDrop;
    changed.scace_enabled = true;
    // Octets de remplissage de la structure `Config` (hors magie de 4 octets).
    let padding = [
        6usize,
        7,
        4 + 14,
        4 + 15,
        4 + 25,
        4 + 26,
        4 + 27,
        4 + 46,
        4 + 47,
        4 + 69,
        4 + 70,
        4 + 71,
    ];
    for (config, c_blob) in [(defaults, blob("v10", "a")), (changed, blob("v10", "b"))] {
        let mine = encode_blob(&config);
        assert_eq!(mine.len(), c_blob.len());
        for i in 0..mine.len() - 4 {
            if !padding.contains(&i) {
                assert_eq!(mine[i], c_blob[i], "octet {i}");
            }
        }
        for blob in [mine, c_blob] {
            let mut store = MemoryConfigStore {
                a: Some(blob),
                ..MemoryConfigStore::default()
            };
            assert_eq!(ConfigManager::init(&mut store).0.get(), config);
        }
    }
}

#[test]
fn v10_load_picks_the_highest_revision() {
    let mut store = store_with("v10");
    let (manager, report) = ConfigManager::init(&mut store);
    let c = manager.get();
    assert_eq!(c.revision, 4);
    assert_eq!(c.target_weight_g, 41.5);
    assert!(c.scace_enabled && !c.heating_enabled);
    assert!(report.found && !report.migrated && !report.normalized);
    assert_eq!(store.writes, 0);

    // Révisions égales : l'emplacement `a` gagne, comme `a >= b` du C++.
    let mut tie = MemoryConfigStore {
        a: Some(blob("v10", "a")),
        b: Some({
            let mut c = Config::default();
            c.revision = 3;
            c.target_time_s = 50;
            encode_blob(&c)
        }),
        ..MemoryConfigStore::default()
    };
    let (manager, _) = ConfigManager::init(&mut tie);
    assert_eq!(manager.get().target_time_s, 28);
}

#[test]
fn fresh_store_writes_defaults_with_revision_one() {
    let mut store = MemoryConfigStore::default();
    let (manager, report) = ConfigManager::init(&mut store);
    assert!(!report.found && !report.persist_failed);
    let mut expected = Config::default();
    expected.revision = 1;
    assert_eq!(manager.get(), expected);
    assert_eq!(store.writes, 1);
    // Écrit sur `a` (aucun emplacement valide).
    assert_eq!(store.a.as_deref(), Some(encode_blob(&expected).as_slice()));
}

#[test]
fn damaged_slots_are_ignored() {
    let mut good = blob("v10", "a");
    let mut bad_checksum = good.clone();
    bad_checksum[10] ^= 0xFF;
    let truncated = good[..79].to_vec();
    good[0] ^= 1; // magie
    let mut store = MemoryConfigStore {
        a: Some(bad_checksum),
        b: Some(truncated),
        ..MemoryConfigStore::default()
    };
    let (_, report) = ConfigManager::init(&mut store);
    assert!(!report.found);
    let mut store = MemoryConfigStore {
        a: Some(good),
        ..MemoryConfigStore::default()
    };
    let (_, report) = ConfigManager::init(&mut store);
    assert!(!report.found);
}

fn migrated(version: &str) -> (Config, MemoryConfigStore, bool) {
    let mut store = store_with(version);
    let (manager, report) = ConfigManager::init(&mut store);
    assert!(report.found && report.migrated, "{version}");
    (manager.get(), store, report.heating_disabled_by_migration)
}

/// Valeurs communes aux blobs d'essai, côté emplacement `b` (révision 6).
fn common_expected() -> Config {
    let mut c = Config::default();
    c.revision = 6;
    c.target_weight_g = 40.5;
    c.target_time_s = 44;
    c.preinfusion_time_s = 5;
    c.preinfusion_pump_pct = 45;
    c.rampdown_mode = RampdownMode::Weight;
    c.rampdown_lead_time_s = 3.5;
    c.rampdown_lead_weight_g = 2.5;
    c.rampdown_pressure_drop_bar = 1.5;
    c.brew_pump_pct = 90;
    c.purge_pump_pct = 70;
    c.purge_max_s = 25;
    c.dim_after_s = 180;
    c.standby_after_s = 900;
    c
}

#[test]
fn migrate_v9() {
    let (c, store, disabled) = migrated("v9");
    let mut e = common_expected();
    e.target_pressure_bar = 8.5;
    e.brew_temperature_c = 88.5;
    e.brew_preheat_time_s = 4.5;
    e.filling_time_s = 7;
    e.filling_pressure_bar = 1.5;
    e.filling_pump_pct = 80;
    e.preinfusion_mode = PreinfusionMode::TIME | PreinfusionMode::WEIGHT;
    assert_eq!(c, e);
    assert!(!disabled);
    assert_eq!(store.writes, 1);
}

#[test]
fn migrate_v8_resets_filling_pressure_and_time() {
    let (c, _, disabled) = migrated("v8");
    let mut e = common_expected();
    e.target_pressure_bar = 8.5;
    e.brew_temperature_c = 88.5;
    e.brew_preheat_time_s = 4.5;
    e.filling_pump_pct = 80;
    e.preinfusion_mode = PreinfusionMode::WEIGHT;
    assert_eq!(c, e); // filling_time_s 10 et filling_pressure_bar 1,0 par défaut
    assert!(!disabled);
}

#[test]
fn migrate_v7_drops_the_pressure_exit() {
    let (c, _, disabled) = migrated("v7");
    let mut e = common_expected();
    e.target_pressure_bar = 8.5;
    e.brew_temperature_c = 88.5;
    e.brew_preheat_time_s = 4.5;
    e.filling_time_s = 7;
    e.filling_pump_pct = 80;
    // Le seul bit « pression » devient une échéance en temps.
    e.preinfusion_mode = PreinfusionMode::TIME;
    assert_eq!(c, e);
    assert!(!disabled);
}

#[test]
fn migrate_v6_keeps_time_when_pressure_and_time_were_set() {
    let (c, _, disabled) = migrated("v6");
    let mut e = common_expected();
    e.target_pressure_bar = 8.5;
    e.brew_temperature_c = 88.5;
    e.filling_time_s = 7;
    e.filling_pump_pct = 80;
    e.preinfusion_mode = PreinfusionMode::TIME;
    assert_eq!(c, e);
    assert!(!disabled);
}

#[test]
fn migrate_v5_disables_heating_until_confirmed() {
    let (c, store, disabled) = migrated("v5");
    let mut e = common_expected();
    e.target_pressure_bar = 8.5;
    e.filling_time_s = 7;
    e.filling_pump_pct = 80;
    e.preinfusion_mode = PreinfusionMode::WEIGHT; // pression + poids → poids
    e.heating_enabled = false;
    assert_eq!(c, e);
    assert!(disabled);
    // La configuration migrée est écrite au schéma courant.
    let (again, report) = ConfigManager::init(&mut store.clone());
    assert!(report.found && !report.migrated);
    assert_eq!(again.get(), e);
}

#[test]
fn migrate_v4_and_v3_use_default_pressure() {
    for (version, mode) in [
        ("v4", PreinfusionMode::TIME),
        ("v3", PreinfusionMode::WEIGHT),
    ] {
        let (c, _, disabled) = migrated(version);
        let mut e = common_expected();
        e.filling_time_s = 7;
        e.filling_pump_pct = 80;
        e.preinfusion_mode = mode;
        e.heating_enabled = false;
        assert_eq!(c, e, "{version}");
        assert!(disabled);
    }
}

#[test]
fn migrate_v2_and_v1() {
    for version in ["v2", "v1"] {
        let (c, _, disabled) = migrated(version);
        let mut e = common_expected();
        e.preinfusion_mode = PreinfusionMode::TIME;
        e.heating_enabled = false;
        assert_eq!(c, e, "{version}");
        assert!(disabled);
    }
}

#[test]
fn normalisation_of_out_of_range_values() {
    let mut c = Config::default();
    c.revision = 9;
    c.brew_pump_pct = 20;
    c.brew_temperature_c = 110.0;
    let mut store = MemoryConfigStore {
        a: Some(encode_blob(&c)),
        ..MemoryConfigStore::default()
    };
    let (manager, report) = ConfigManager::init(&mut store);
    let got = manager.get();
    assert_eq!((got.brew_pump_pct, got.brew_temperature_c), (50, 95.0));
    assert!(report.normalized && !report.migrated);
    assert_eq!(store.writes, 1);
    // Écrit sur `b` : `a` est valide.
    assert!(store.b.is_some());
}

#[test]
fn apply_numbers_validates_and_alternates_slots() {
    let mut store = MemoryConfigStore::default();
    let (mut manager, _) = ConfigManager::init(&mut store); // révision 1 sur `a`
    let mut next = manager.get();
    next.target_time_s = 30;

    let r = manager.apply(&mut store, &next, 0);
    assert_eq!(
        (r.status, r.field),
        (ConfigStatus::StaleRevision, Some("revision"))
    );

    let r = manager.apply(&mut store, &next, 1);
    assert_eq!(r.status, ConfigStatus::Ok);
    assert_eq!(manager.get().revision, 2);
    assert_eq!(manager.get().target_time_s, 30);
    // Révision 1 sur `a`, donc la 2 va sur `b`, puis la 3 sur `a`.
    let slot_revision = |blob: &Option<Vec<u8>>| {
        let mut s = MemoryConfigStore::default();
        s.a = blob.clone();
        ConfigManager::init(&mut s).0.get().revision
    };
    assert_eq!(slot_revision(&store.b), 2);
    next.target_time_s = 31;
    assert_eq!(manager.apply(&mut store, &next, 2).status, ConfigStatus::Ok);
    assert_eq!(slot_revision(&store.a), 3);
    assert_eq!(slot_revision(&store.b), 2);

    let mut invalid = manager.get();
    invalid.target_time_s = 3;
    let r = manager.apply(&mut store, &invalid, 3);
    assert_eq!(
        (r.status, r.field),
        (ConfigStatus::InvalidValue, Some("brew.target_time_s"))
    );
    let mut wrong_version = manager.get();
    wrong_version.version = 9;
    let r = manager.apply(&mut store, &wrong_version, 3);
    assert_eq!(
        (r.status, r.field),
        (ConfigStatus::VersionMismatch, Some("version"))
    );
    assert_eq!(manager.get().revision, 3);
}

#[test]
fn storage_error_changes_nothing() {
    let mut store = MemoryConfigStore::default();
    let (mut manager, _) = ConfigManager::init(&mut store);
    store.fail_writes = true;
    let mut next = manager.get();
    next.target_time_s = 40;
    let r = manager.apply(&mut store, &next, 1);
    assert_eq!(
        (r.status, r.field),
        (ConfigStatus::StorageError, Some("nvs"))
    );
    assert_eq!(manager.get().revision, 1);
    assert_eq!(manager.get().target_time_s, 28);
}

#[test]
fn reset_restores_defaults_with_a_new_revision() {
    let mut store = store_with("v10");
    let (mut manager, _) = ConfigManager::init(&mut store);
    assert_eq!(manager.get().revision, 4);
    assert_eq!(manager.reset(&mut store).status, ConfigStatus::Ok);
    let mut expected = Config::default();
    expected.revision = 5;
    assert_eq!(manager.get(), expected);
}

#[test]
fn schema_constant() {
    assert_eq!(CONFIG_SCHEMA_VERSION, 10);
    let mut store = MemoryConfigStore::default();
    assert!(store.read_slot(Slot::A).is_none());
    assert_eq!(Slot::A.key(), "a");
    assert_eq!(Slot::B.key(), "b");
}

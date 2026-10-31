//! Configuration : défauts, validation, blob NVS du schéma 12, migrations des
//! schémas 1 à 11, écriture alternée des deux emplacements.
//!
//! `vectors/config_blobs.txt` vient du vrai compilateur C sur les structures
//! de `config.cpp` (voir `vectors/gen_config_blobs.sh`) ; ses blobs sont ceux
//! des anciens schémas, que le schéma 12 migre.

#![allow(clippy::field_reassign_with_default)]

use screen_core::config::{
    CONFIG_SCHEMA_VERSION, Config, ConfigManager, ConfigStatus, ConfigStore, MemoryConfigStore,
    PreinfusionMode, Slot, encode_blob, validate,
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

/// `c` dont le profil unique reprend les champs plats, comme après une
/// migration.
fn single(c: Config) -> Config {
    let mut c = c;
    let flat = c.clone();
    c.profiles_mut()[0].take_settings(&flat);
    c
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
    assert_eq!(c.version, 12);
    assert_eq!(c.revision, 0);
    assert_eq!((c.profile_count, c.active_profile), (1, 0));
    let profile = c.active();
    assert_eq!(
        (profile.name(), profile.basket(), profile.dose_g),
        ("single espresso", "LM1 41 mm", 8.0)
    );
    assert_eq!(profile.target_weight_g, 36.0);
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
    assert_eq!(c.stop_lead_g, 4.0);
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
        ok(&|c| c.target_pressure_bar = 3.9),
        Some("brew.target_pressure_bar")
    );
    assert_eq!(ok(&|c| c.target_pressure_bar = 4.0), None);
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
    assert_eq!(ok(&|c| c.stop_lead_g = 20.5), Some("brew.stop_lead_g"));
    assert_eq!(ok(&|c| c.stop_lead_g = 3.25), Some("brew.stop_lead_g"));
    assert_eq!(ok(&|c| c.stop_lead_g = 0.0), None);
    assert_eq!(ok(&|c| c.stop_lead_g = 20.0), None);
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
fn v10_blobs_from_the_c_compiler_migrate_to_a_single_profile() {
    let mut defaults = single(Config::default());
    defaults.revision = 3;
    let mut changed = Config::default();
    changed.revision = 4;
    changed.target_weight_g = 41.5;
    changed.target_time_s = 31;
    changed.heating_enabled = false;
    changed.preinfusion_mode = PreinfusionMode::TIME | PreinfusionMode::WEIGHT;
    changed.scace_enabled = true;
    let changed = single(changed);
    for (config, slot) in [(defaults, "a"), (changed, "b")] {
        let mut store = MemoryConfigStore {
            a: Some(blob("v10", slot)),
            ..MemoryConfigStore::default()
        };
        let (manager, report) = ConfigManager::init(&mut store);
        assert!(report.found && report.migrated);
        assert_eq!(*manager.get(), config);
        let c = manager.get().clone();
        assert_eq!((c.profile_count, c.active_profile), (1, 0));
        assert_eq!(c.active().name(), "single espresso");
        assert_eq!(c.active().dose_g, 8.0);
        assert_eq!(c.active().basket(), "LM1 41 mm");
        assert_eq!(c.active().target_weight_g, config.target_weight_g);
        // La configuration migrée est écrite au schéma 12, et relue telle quelle.
        assert_eq!(store.writes, 1);
        let (again, report) = ConfigManager::init(&mut store);
        assert!(report.found && !report.migrated);
        assert_eq!(*again.get(), config);
    }
}

#[test]
fn v10_load_picks_the_highest_revision() {
    let mut store = store_with("v10");
    let (manager, report) = ConfigManager::init(&mut store);
    let c = manager.get().clone();
    assert_eq!(c.revision, 4);
    assert_eq!(c.target_weight_g, 41.5);
    assert_eq!(c.active().target_weight_g, 41.5);
    assert!(c.scace_enabled && !c.heating_enabled);
    assert!(report.found && report.migrated && !report.normalized);
    assert_eq!(store.writes, 1);
}

fn put_u16(out: &mut [u8], at: usize, v: u16) {
    out[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32(out: &mut [u8], at: usize, v: u32) {
    out[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_f32(out: &mut [u8], at: usize, v: f32) {
    out[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// Enregistrement de profil du schéma 11 (116 octets) : nom, panier, puis les
/// champs dont la rampe de fin d'infusion.
fn v11_profile_record(name: &str, rampdown_mode: u8, lead_weight_g: f32) -> Vec<u8> {
    let mut r = vec![0u8; 116];
    r[..name.len()].copy_from_slice(name.as_bytes());
    let f = 64;
    r[f] = name.len() as u8;
    r[f + 2] = 100; // brew_pump_pct
    r[f + 3] = 100; // filling_pump_pct
    r[f + 4] = 1; // preinfusion_mode : temps
    r[f + 5] = 35; // preinfusion_pump_pct
    r[f + 6] = rampdown_mode;
    put_u16(&mut r, f + 8, 28); // target_time_s
    put_u16(&mut r, f + 10, 10); // filling_time_s
    put_u16(&mut r, f + 12, 4); // preinfusion_time_s
    put_f32(&mut r, f + 16, 8.0); // dose_g
    put_f32(&mut r, f + 20, 36.0); // target_weight_g
    put_f32(&mut r, f + 24, 9.0); // target_pressure_bar
    put_f32(&mut r, f + 28, 90.0); // brew_temperature_c
    put_f32(&mut r, f + 32, 2.5); // brew_preheat_time_s
    put_f32(&mut r, f + 36, 1.0); // filling_pressure_bar
    put_f32(&mut r, f + 40, 3.0); // rampdown_lead_time_s
    put_f32(&mut r, f + 44, lead_weight_g);
    put_f32(&mut r, f + 48, 1.0); // rampdown_pressure_drop_bar
    r
}

/// Blob NVS du schéma 11 : magie, machine (20 octets), quatre enregistrements,
/// somme de contrôle FNV-1a.
fn v11_blob(revision: u32, records: &[Vec<u8>], active: u8) -> Vec<u8> {
    let mut out = vec![0u8; 4 + 20 + 4 * 116 + 4];
    out[0..4].copy_from_slice(&0x4346_4731u32.to_le_bytes());
    put_u16(&mut out, 4, 11);
    put_u32(&mut out, 8, revision);
    out[12] = 1; // heating_enabled
    out[13] = 100; // purge_pump_pct
    put_u16(&mut out, 14, 20); // purge_max_s
    put_u16(&mut out, 16, 240); // dim_after_s
    put_u16(&mut out, 18, 1800); // standby_after_s
    out[21] = active;
    out[22] = records.len() as u8;
    for (i, record) in records.iter().enumerate() {
        let at = 4 + 20 + i * 116;
        out[at..at + 116].copy_from_slice(record);
    }
    let end = out.len() - 4;
    let mut hash: u32 = 2_166_136_261;
    for &b in &out[..end] {
        hash = (hash ^ u32::from(b)).wrapping_mul(16_777_619);
    }
    out[end..].copy_from_slice(&hash.to_le_bytes());
    out
}

#[test]
fn v11_profiles_keep_their_lead_unless_the_rampdown_stopped_at_the_target() {
    // Modes de la rampe : 0 aucune, 1 temps, 2 poids, 3 chute de pression.
    let records = [
        v11_profile_record("none", 0, 4.0),
        v11_profile_record("time", 1, 2.5),
        v11_profile_record("weight", 2, 3.0),
        v11_profile_record("drop", 3, 1.5),
    ];
    let mut store = MemoryConfigStore {
        a: Some(v11_blob(7, &records, 1)),
        ..MemoryConfigStore::default()
    };
    let (manager, report) = ConfigManager::init(&mut store);
    assert!(report.found && report.migrated && !report.normalized);
    let c = manager.get().clone();
    assert_eq!(c.version, CONFIG_SCHEMA_VERSION);
    assert_eq!((c.revision, c.profile_count, c.active_profile), (7, 4, 1));
    let leads: Vec<f32> = c.profiles().iter().map(|p| p.stop_lead_g).collect();
    assert_eq!(leads, [4.0, 2.5, 0.0, 1.5]);
    assert_eq!(c.profiles()[2].name(), "weight");
    // Les champs plats suivent le profil actif.
    assert_eq!(c.stop_lead_g, 2.5);
    assert_eq!(validate(&c), None);
    // La configuration est réécrite au schéma courant.
    assert_eq!(store.writes, 1);
    let (again, report) = ConfigManager::init(&mut store);
    assert!(report.found && !report.migrated);
    assert_eq!(*again.get(), c);
}

#[test]
fn v11_load_picks_the_highest_revision_and_a_wins_ties() {
    let mut low = Config::default();
    low.revision = 3;
    low.profiles_mut()[0].target_time_s = 50;
    let mut high = Config::default();
    high.revision = 4;
    let mut store = MemoryConfigStore {
        a: Some(encode_blob(&low)),
        b: Some(encode_blob(&high)),
        ..MemoryConfigStore::default()
    };
    let (manager, report) = ConfigManager::init(&mut store);
    assert_eq!(manager.get().revision, 4);
    assert_eq!(manager.get().target_time_s, 28);
    assert!(report.found && !report.migrated);
    assert_eq!(store.writes, 0);

    // Révisions égales : l'emplacement `a` gagne, comme `a >= b` du C++.
    high.revision = 3;
    store.b = Some(encode_blob(&high));
    let (manager, _) = ConfigManager::init(&mut store);
    assert_eq!(manager.get().target_time_s, 50);
}

#[test]
fn fresh_store_writes_defaults_with_revision_one() {
    let mut store = MemoryConfigStore::default();
    let (manager, report) = ConfigManager::init(&mut store);
    assert!(!report.found && !report.persist_failed);
    let mut expected = Config::default();
    expected.revision = 1;
    assert_eq!(*manager.get(), expected);
    assert_eq!(store.writes, 1);
    // Écrit sur `a` (aucun emplacement valide).
    assert_eq!(store.a.as_deref(), Some(encode_blob(&expected).as_slice()));
}

#[test]
fn damaged_slots_are_ignored() {
    let mut good = encode_blob(&Config::default());
    let mut bad_checksum = good.clone();
    bad_checksum[10] ^= 0xFF;
    let truncated = good[..good.len() - 1].to_vec();
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
    (manager.get().clone(), store, report.heating_disabled_by_migration)
}

/// Valeurs communes aux blobs d'essai, côté emplacement `b` (révision 6).
fn common_expected() -> Config {
    let mut c = Config::default();
    c.revision = 6;
    c.target_weight_g = 40.5;
    c.target_time_s = 44;
    c.preinfusion_time_s = 5;
    c.preinfusion_pump_pct = 45;
    // Le mode du blob d'essai est « poids » : la pompe s'arrêtait à la
    // cible, donc aucune avance.
    c.stop_lead_g = 0.0;
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
    assert_eq!(c, single(e));
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
    assert_eq!(c, single(e)); // filling_time_s 10 et filling_pressure_bar 1,0 par défaut
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
    assert_eq!(c, single(e));
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
    assert_eq!(c, single(e));
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
    assert_eq!(c, single(e.clone()));
    assert!(disabled);
    // La configuration migrée est écrite au schéma courant.
    let (again, report) = ConfigManager::init(&mut store.clone());
    assert!(report.found && !report.migrated);
    assert_eq!(*again.get(), single(e));
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
        assert_eq!(c, single(e), "{version}");
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
        assert_eq!(c, single(e), "{version}");
        assert!(disabled);
    }
}

#[test]
fn normalisation_of_out_of_range_values() {
    let mut c = Config::default();
    c.revision = 9;
    c.profiles_mut()[0].brew_pump_pct = 20;
    c.profiles_mut()[0].brew_temperature_c = 110.0;
    let mut store = MemoryConfigStore {
        a: Some(encode_blob(&c)),
        ..MemoryConfigStore::default()
    };
    let (manager, report) = ConfigManager::init(&mut store);
    let got = manager.get().clone();
    assert_eq!((got.brew_pump_pct, got.brew_temperature_c), (50, 95.0));
    assert_eq!(
        (got.active().brew_pump_pct, got.active().brew_temperature_c),
        (50, 95.0)
    );
    assert!(report.normalized && !report.migrated);
    assert_eq!(store.writes, 1);
    // Écrit sur `b` : `a` est valide.
    assert!(store.b.is_some());
}

#[test]
fn apply_numbers_validates_and_alternates_slots() {
    let mut store = MemoryConfigStore::default();
    let (mut manager, _) = ConfigManager::init(&mut store); // révision 1 sur `a`
    let mut next = manager.get().clone();
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

    let mut invalid = manager.get().clone();
    invalid.target_time_s = 3;
    let r = manager.apply(&mut store, &invalid, 3);
    assert_eq!(
        (r.status, r.field),
        (ConfigStatus::InvalidValue, Some("brew.target_time_s"))
    );
    let mut wrong_version = manager.get().clone();
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
    let mut next = manager.get().clone();
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
    assert_eq!(*manager.get(), expected);
}

#[test]
fn schema_constant() {
    assert_eq!(CONFIG_SCHEMA_VERSION, 12);
    let mut store = MemoryConfigStore::default();
    assert!(store.read_slot(Slot::A).is_none());
    assert_eq!(Slot::A.key(), "a");
    assert_eq!(Slot::B.key(), "b");
}

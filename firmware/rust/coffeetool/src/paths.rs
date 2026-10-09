//! Dossiers et horodatages des fichiers écrits par l'outil.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};

/// Racine du dépôt : `captures/` et `configs/` y vivent, comme avec les
/// scripts Python.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("le crate est à trois niveaux de la racine")
        .to_path_buf()
}

pub fn captures_dir() -> PathBuf {
    repo_root().join("captures")
}

pub fn configs_dir() -> PathBuf {
    repo_root().join("configs")
}

/// `2026-10-09T12:34:56.789+00:00`, comme `isoformat(timespec="milliseconds")`.
pub fn utc_now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.3f+00:00").to_string()
}

/// Nom de fichier `yymmdd-HHMMSS` des captures HF et des configurations.
pub fn short_stamp() -> String {
    Local::now().format("%y%m%d-%H%M%S").to_string()
}

/// `YYYYmmdd-HHMMSS-ffffff` des relevés de télémétrie.
pub fn long_stamp() -> String {
    Local::now().format("%Y%m%d-%H%M%S-%6f").to_string()
}

/// Heure locale `HH:MM:SS.mmm` d'un horodatage Unix, tronquée à la
/// milliseconde comme `strftime("%H:%M:%S.%f")[:-3]`.
pub fn format_timestamp(ts: f64) -> String {
    let secs = ts.floor();
    let nanos = ((ts - secs) * 1e9) as u32;
    match DateTime::from_timestamp(secs as i64, nanos.min(999_999_999)) {
        Some(utc) => utc.with_timezone(&Local).format("%H:%M:%S%.3f").to_string(),
        None => "??:??:??.???".to_string(),
    }
}

/// Écrit un JSON indenté de deux espaces, terminé par un retour à la ligne,
/// dans un fichier neuf (refuse d'écraser).
pub fn write_json_new(path: &Path, value: &serde_json::Value) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("création de {}", parent.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                anyhow::anyhow!("le fichier existe déjà : {}", path.display())
            } else {
                anyhow::Error::new(e).context(format!("écriture de {}", path.display()))
            }
        })?;
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    file.write_all(text.as_bytes())?;
    Ok(())
}

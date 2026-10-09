// Génère `log_codes.rs` depuis `log_codes.yaml`, source unique de la table
// des codes LOG. Même format d'entrée que `gen_log_codes.py` : une liste à
// plat de mappings `name` / `severity` / `comment`, pas du YAML général.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

const SEVERITIES: [&str; 4] = ["debug", "info", "warn", "error"];

struct Entry {
    name: String,
    severity: String,
    comment: String,
}

fn parse(text: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut current: Option<(String, Option<String>, Option<String>)> = None;
    let mut finish = |current: &mut Option<(String, Option<String>, Option<String>)>| {
        if let Some((name, severity, comment)) = current.take() {
            let severity = severity.unwrap_or_else(|| panic!("entrée incomplète: {name}"));
            let comment = comment.unwrap_or_else(|| panic!("entrée incomplète: {name}"));
            assert!(
                SEVERITIES.contains(&severity.as_str()),
                "sévérité inconnue pour {name}: {severity}"
            );
            entries.push(Entry {
                name,
                severity,
                comment,
            });
        }
    };
    for raw in text.lines() {
        // Comme le générateur Python, tout ce qui suit un `#` est un commentaire.
        let line = raw.split('#').next().unwrap_or("").trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix("- name:") {
            finish(&mut current);
            current = Some((name.trim().to_string(), None, None));
        } else if let Some(rest) = line.trim_start().strip_prefix("severity:") {
            current.as_mut().expect("severity hors entrée").1 = Some(rest.trim().to_string());
        } else if let Some(rest) = line.trim_start().strip_prefix("comment:") {
            current.as_mut().expect("comment hors entrée").2 = Some(rest.trim().to_string());
        } else {
            panic!("ligne non reconnue dans log_codes.yaml: {raw:?}");
        }
    }
    finish(&mut current);
    assert!(
        entries.len() <= 256,
        "plus de 256 codes LOG, le champ code est un octet"
    );
    entries
}

fn to_camel(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                }
                None => String::new(),
            }
        })
        .collect()
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest.join("log_codes.yaml");
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed=build.rs");
    let entries = parse(&fs::read_to_string(&source).expect("lecture de log_codes.yaml"));
    for (i, a) in entries.iter().enumerate() {
        assert!(
            entries[i + 1..].iter().all(|b| b.name != a.name),
            "code LOG en double: {}",
            a.name
        );
    }

    let mut out = String::new();
    out.push_str("// GÉNÉRÉ par build.rs depuis log_codes.yaml — ne pas éditer.\n\n");

    out.push_str("/// Code LOG (octet 0 de la charge utile du message LOG).\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    out.push_str(
        "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]\n",
    );
    out.push_str("#[repr(u8)]\npub enum LogCode {\n");
    for (i, e) in entries.iter().enumerate() {
        writeln!(out, "    #[doc = {:?}]", e.comment).unwrap();
        writeln!(out, "    {} = {i},", to_camel(&e.name)).unwrap();
    }
    out.push_str("}\n\n");

    out.push_str("/// Gravité d'un LOG (octet 1 de la charge utile).\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\n");
    out.push_str(
        "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]\n",
    );
    out.push_str("#[repr(u8)]\npub enum LogSeverity {\n");
    for (i, s) in SEVERITIES.iter().enumerate() {
        writeln!(out, "    {} = {i},", to_camel(s)).unwrap();
    }
    out.push_str("}\n\n");

    out.push_str("/// Nombre de codes LOG définis.\n");
    writeln!(
        out,
        "pub const LOG_CODE_COUNT: usize = {};\n",
        entries.len()
    )
    .unwrap();

    out.push_str("/// Table des codes, dans l'ordre des valeurs : nom, gravité, description\n");
    out.push_str("/// (qui précise le sens de `arg16` et `arg32`).\n");
    out.push_str("pub const LOG_CODES: [LogCodeInfo; LOG_CODE_COUNT] = [\n");
    for e in &entries {
        writeln!(
            out,
            "    LogCodeInfo {{ code: LogCode::{}, name: {:?}, severity: LogSeverity::{}, description: {:?} }},",
            to_camel(&e.name),
            e.name,
            to_camel(&e.severity),
            e.comment
        )
        .unwrap();
    }
    out.push_str("];\n");

    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("log_codes.rs"),
        out,
    )
    .expect("écriture de log_codes.rs");
}

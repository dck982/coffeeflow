//! Données factices de la zone d'essai, les mêmes que `nvs/donnees.py`.
//! Aucune ne vient du vrai NVS.

use alloc::string::String;
use alloc::vec::Vec;

use esp_nvs::{Get, Key, Nvs};

use crate::flash::Flash;

pub fn motif(n: usize, a: usize, b: usize) -> Vec<u8> {
    (0..n).map(|i| (i * a + b) as u8).collect()
}

/// `Credentials` de net_wifi.cpp : ssid[33] puis password[64].
pub fn identifiants(ssid: &str, mot_de_passe: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(97);
    out.extend_from_slice(ssid.as_bytes());
    out.resize(33, 0);
    out.extend_from_slice(mot_de_passe.as_bytes());
    out.resize(97, 0);
    out
}

#[derive(PartialEq, Debug)]
pub enum Valeur {
    U8(u8),
    I8(i8),
    U32(u32),
    I32(i32),
    U64(u64),
    Texte(String),
    Blob(Vec<u8>),
}

impl Valeur {
    /// Relit la clé avec le type de la valeur attendue.
    fn lire(&self, nvs: &mut Nvs<Flash>, ns: &Key, cle: &Key) -> Result<Valeur, esp_nvs::error::Error> {
        Ok(match self {
            Self::U8(_) => Self::U8(nvs.get(ns, cle)?),
            Self::I8(_) => Self::I8(nvs.get(ns, cle)?),
            Self::U32(_) => Self::U32(nvs.get(ns, cle)?),
            Self::I32(_) => Self::I32(nvs.get(ns, cle)?),
            Self::U64(_) => Self::U64(nvs.get(ns, cle)?),
            Self::Texte(_) => Self::Texte(nvs.get(ns, cle)?),
            Self::Blob(_) => Self::Blob(nvs.get(ns, cle)?),
        })
    }

    fn ecrire(&self, nvs: &mut Nvs<Flash>, ns: &Key, cle: &Key) -> Result<(), esp_nvs::error::Error> {
        match self {
            Self::U8(v) => nvs.set(ns, cle, *v),
            Self::I8(v) => nvs.set(ns, cle, *v),
            Self::U32(v) => nvs.set(ns, cle, *v),
            Self::I32(v) => nvs.set(ns, cle, *v),
            Self::U64(v) => nvs.set(ns, cle, *v),
            Self::Texte(v) => nvs.set(ns, cle, v.as_str()),
            Self::Blob(v) => nvs.set(ns, cle, v.as_slice()),
        }
    }

    fn apercu(&self) -> heapless::String<48> {
        let mut s = heapless::String::new();
        let _ = match self {
            Self::Blob(v) => core::fmt::write(&mut s, format_args!("blob {} octets", v.len())),
            Self::Texte(v) => core::fmt::write(&mut s, format_args!("texte {:?}", v)),
            other => core::fmt::write(&mut s, format_args!("{other:?}")),
        };
        s
    }
}

pub type Entree = (&'static str, &'static str, Valeur);

pub fn avant() -> Vec<Entree> {
    alloc::vec![
        ("ui", "a", Valeur::Blob(motif(120, 7, 1))),
        ("ui", "b", Valeur::Blob(motif(120, 13, 5))),
        ("wifi", "credentials", Valeur::Blob(identifiants("essai-ssid", "faux-mot-de-passe"))),
        ("essai", "compteur", Valeur::U32(123_456)),
        ("essai", "signe", Valeur::I32(-42)),
        ("essai", "octet", Valeur::U8(7)),
        ("essai", "nom", Valeur::Texte("coffeeflow-essai".into())),
        ("essai", "gros", Valeur::Blob(motif(6000, 31, 3))),
    ]
}

/// Ce que la Rust écrit : une valeur modifiée, un blob réécrit, et des
/// types que coffeeflow n'utilise pas encore. `signe` est effacée.
pub fn ecritures() -> Vec<Entree> {
    alloc::vec![
        ("essai", "compteur", Valeur::U32(654_321)),
        ("ui", "b", Valeur::Blob(motif(120, 11, 2))),
        ("essai", "nouveau", Valeur::Texte("ecrit par rust".into())),
        ("essai", "rust_blob", Valeur::Blob(motif(300, 3, 1))),
        ("essai", "grand", Valeur::U64(0x0123_4567_89AB_CDEF)),
        ("essai", "petit", Valeur::I8(-5)),
    ]
}

pub const EFFACEE: (&str, &str) = ("essai", "signe");

pub fn apres() -> Vec<Entree> {
    let mut out: Vec<Entree> = avant()
        .into_iter()
        .filter(|(ns, cle, _)| (*ns, *cle) != EFFACEE)
        .collect();
    for (ns, cle, v) in ecritures() {
        match out.iter_mut().find(|(n, c, _)| (*n, *c) == (ns, cle)) {
            Some(entree) => entree.2 = v,
            None => out.push((ns, cle, v)),
        }
    }
    out
}

/// Relit chaque entrée attendue ; rend le nombre d'écarts. Vérifie aussi
/// que la clé effacée a disparu, si `effacee` est vrai.
pub fn controler(nvs: &mut Nvs<Flash>, attendu: &[Entree], effacee: bool) -> u32 {
    let mut ecarts = 0;
    for (ns, cle, voulu) in attendu {
        let lu = voulu.lire(nvs, &Key::from_str(ns), &Key::from_str(cle));
        match lu {
            Ok(lu) if lu == *voulu => log::info!("ok     {ns}/{cle} {}", voulu.apercu()),
            Ok(lu) => {
                ecarts += 1;
                log::info!("ECART  {ns}/{cle} lu {}, attendu {}", lu.apercu(), voulu.apercu());
            }
            Err(err) => {
                ecarts += 1;
                log::info!("ECART  {ns}/{cle} {err:?}");
            }
        }
    }
    if effacee {
        let (ns, cle) = EFFACEE;
        match <Nvs<Flash> as Get<i32>>::get(nvs, &Key::from_str(ns), &Key::from_str(cle)) {
            Err(esp_nvs::error::Error::KeyNotFound) => log::info!("ok     {ns}/{cle} effacée"),
            other => {
                ecarts += 1;
                log::info!("ECART  {ns}/{cle} encore là : {other:?}");
            }
        }
    }
    ecarts
}

/// Fait les écritures de `ecritures()` et l'effacement.
pub fn modifier(nvs: &mut Nvs<Flash>) -> u32 {
    let mut erreurs = 0;
    for (ns, cle, v) in ecritures() {
        if let Err(err) = v.ecrire(nvs, &Key::from_str(ns), &Key::from_str(cle)) {
            erreurs += 1;
            log::info!("écriture {ns}/{cle} : {err:?}");
        }
    }
    let (ns, cle) = EFFACEE;
    if let Err(err) = nvs.delete(&Key::from_str(ns), &Key::from_str(cle)) {
        erreurs += 1;
        log::info!("effacement {ns}/{cle} : {err:?}");
    }
    erreurs
}

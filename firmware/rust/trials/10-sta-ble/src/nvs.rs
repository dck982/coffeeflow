//! Identifiants Wi-Fi de coffeeflow, relus dans le vrai NVS en lecture
//! seule : la flash passée à esp-nvs refuse toute écriture, donc rien ne
//! change sur la carte. Ni le SSID ni le mot de passe ne sont journalisés,
//! seulement leurs longueurs.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering::Relaxed;

use esp_nvs::{Key, Nvs};
use esp_storage::FlashStorage;
use log::info;

use crate::flash::{self, Flash, REFUSED, WRITES};

/// SSID et mot de passe de `wifi/credentials` (`Credentials` de
/// net_wifi.cpp : ssid[33] puis password[64], terminés par un zéro).
#[allow(
    clippy::large_stack_frames,
    reason = "pages d'esp-nvs (version corrigée) lues une fois au démarrage, sur la pile du cœur 1"
)]
pub fn read_credentials(mut storage: FlashStorage<'static>) -> Option<(String, String)> {
    let (at, len) = flash::nvs_region(&mut storage);
    let mut nvs = match Nvs::new(at as usize, len as usize, Flash::new(storage, false)) {
        Ok(nvs) => nvs,
        Err(err) => {
            info!("nvs: ouverture impossible, {err:?}");
            return None;
        }
    };
    let blob: Vec<u8> = match nvs.get(&Key::from_str("wifi"), &Key::from_str("credentials")) {
        Ok(blob) => blob,
        Err(err) => {
            info!("nvs: wifi/credentials, {err:?}");
            return None;
        }
    };
    info!(
        "nvs: {} écriture(s), {} refusée(s)",
        WRITES.load(Relaxed),
        REFUSED.load(Relaxed)
    );
    if blob.len() != 97 {
        info!("nvs: wifi/credentials fait {} octets, 97 attendus", blob.len());
        return None;
    }
    let field = |bytes: &[u8]| {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8(bytes[..end].to_vec()).ok()
    };
    let (Some(ssid), Some(password)) = (field(&blob[..33]), field(&blob[33..])) else {
        info!("nvs: wifi/credentials n'est pas de l'UTF-8");
        return None;
    };
    info!("nvs: SSID de {} car., mot de passe de {} car.", ssid.len(), password.len());
    Some((ssid, password))
}

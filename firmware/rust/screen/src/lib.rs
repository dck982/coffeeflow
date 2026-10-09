//! Services de l'écran compilés indépendamment de la boucle d'affichage.
#![no_std]
extern crate alloc;
pub mod ble;
pub mod breadcrumb;
pub mod can;
pub mod flash;
mod flash_words;
pub mod heap;
pub mod network;
pub mod nvs;
pub mod ota_local;
pub mod ota_proxy;
pub mod otadata;
pub mod proxy_service;
pub mod serial_bridge;
pub mod service;

mod serial_log;

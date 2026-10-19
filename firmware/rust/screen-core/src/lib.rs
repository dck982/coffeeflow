//! Logique de l'écran coffeeflow, sans matériel : cycle d'infusion,
//! régulation de chauffe, configuration, documents HTTP, protocoles BLE de la
//! balance et de la sonde, état de la liaison CAN.
//!
//! Le crate est `no_std` + `alloc`, testé sur le Mac. Il reprend le
//! comportement du firmware C++ archivé (`archive/firmware/screen/main/core`
//! et la partie logique des autres fichiers) au bit près sur le bus et à la
//! clé près en HTTP. Le temps est injecté ([`core::Clock`]) ; l'heure murale
//! est facultative.
//!
//! Point d'entrée : [`core::Core`] (entrées, sorties, instantané).
//! Les documents HTTP sont dans [`http`], la configuration dans [`config`].

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod acaia;
pub mod ble_policy;
pub mod brew_heating;
pub mod calibration;
pub mod can_link;
pub mod config;
pub mod config_json;
pub mod core;
pub mod enclosure;
pub mod events;
pub mod guided;
pub mod hf_capture;
pub mod http;
pub mod json;
pub mod machine;
pub mod ntc;
pub mod sample_schedule;
pub mod scace;
pub mod shot_summary;
pub mod snapshot;
pub mod thermal;

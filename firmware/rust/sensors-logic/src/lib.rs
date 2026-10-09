//! Logique du module capteurs, sans matériel : elle compile sur le Mac et
//! s'y teste avec `cargo test`. Le firmware (`firmware/rust/sensors`)
//! l'enveloppe avec les broches, l'I2C, le TWAI et la flash.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

pub mod flash_rx;
pub mod flow;
pub mod heating_pwm;
pub mod machine;
pub mod zero_guard;

#[cfg(test)]
mod machine_tests;

pub use crate::machine::{Action, Config, Hw, Machine, TwaiCounters, lockout_after_boot};
pub use flash_rx::{FlashReceiver, FlashSink, image_header_ok};
pub use flow::FlowCounter;
pub use heating_pwm::HeatingPwm;
pub use zero_guard::DimmerZeroGuard;

#![no_std]

extern crate alloc;

slint::include_modules!();

pub mod demo;
pub mod image;
pub mod otadata;
pub mod xip;

/// Fin d'une panique, après la trace d'esp-backtrace. Par défaut, il boucle
/// sans fin, interruptions masquées, et `esp_hal::init` a coupé tous les
/// watchdogs : la carte resterait figée. On redémarre, pour que le
/// bootloader revienne sur l'image précédente si celle-ci n'est pas validée.
/// 50 ms laissent l'UART vider sa FIFO.
#[unsafe(no_mangle)]
extern "Rust" fn custom_halt() -> ! {
    esp_hal::delay::Delay::new().delay_millis(50);
    esp_hal::system::software_reset()
}

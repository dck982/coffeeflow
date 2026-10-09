//! Cœur 1 : radio, réseau et flash, plus tard. Son exécuteur tourne déjà, sans
//! tâche : le BLE, le Wi-Fi et le reste s'y ajoutent dans `start`.
//!
//! Le cœur 1 ne journalise rien lui-même : esp-println écrit sous une section
//! critique partagée par les deux cœurs, et le cœur 0 resterait bloqué,
//! interruptions masquées, le temps que l'autre vide sa ligne. Il alloue en
//! SRAM interne (voir `heap`).

use esp_hal::peripherals::{CPU_CTRL, FROM_CPU_INTR1};
use esp_hal::system::Stack;
use static_cell::StaticCell;

const STACK_BYTES: usize = 16384;

/// Démarre le second cœur avec son exécuteur embassy. À appeler une fois
/// l'affichage lancé : esp-hal active les interruptions radio sur le cœur qui
/// crée le contrôleur, et le Wi-Fi y épingle sa tâche. Les périphériques
/// radio (`WIFI`, `BT`) seront passés ici et confiés à la tâche de départ.
pub fn start(cpu_ctrl: CPU_CTRL<'static>, intr1: FROM_CPU_INTR1<'static>) {
    static STACK: StaticCell<Stack<STACK_BYTES>> = StaticCell::new();
    static EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();
    let stack = STACK.init_with(Stack::new);
    esp_rtos::start_second_core(cpu_ctrl, intr1, stack, move || {
        let executor = EXECUTOR.init(esp_rtos::embassy::Executor::new());
        // Les tâches radio et réseau se lanceront ici : `spawner.spawn(...)`.
        executor.run(|_spawner| {});
    });
}

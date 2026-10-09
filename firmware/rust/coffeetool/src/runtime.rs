//! Horloge, pause et Ctrl-C derrière un trait : les boucles d'attente
//! (purge, enregistrement) tournent sur une horloge simulée dans les tests,
//! sans jamais dormir pour de vrai.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub trait Runtime {
    /// Horloge monotone, en secondes.
    fn now(&self) -> f64;
    fn sleep(&self, seconds: f64);
    /// Ctrl-C reçu. Les boucles le testent après chaque pause : une pause ne
    /// s'écourte jamais, pour que la purge puisse encore envoyer son arrêt.
    fn interrupted(&self) -> bool;
}

/// Erreur portée par une boucle arrêtée par Ctrl-C.
#[derive(Debug)]
pub struct Interrupted;

impl std::fmt::Display for Interrupted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("interrompu")
    }
}

impl std::error::Error for Interrupted {}

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Installe le gestionnaire de Ctrl-C : il lève un drapeau, rien d'autre,
/// pour que chaque commande range ses fichiers avant de s'arrêter.
pub fn install_ctrlc() {
    // Un second appel (tests) échoue sans conséquence.
    let _ = ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::SeqCst));
}

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

pub struct SystemRuntime {
    start: Instant,
}

impl SystemRuntime {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for SystemRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime for SystemRuntime {
    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn sleep(&self, seconds: f64) {
        if seconds > 0.0 {
            std::thread::sleep(Duration::from_secs_f64(seconds));
        }
    }

    fn interrupted(&self) -> bool {
        interrupted()
    }
}

#[cfg(test)]
pub mod fake {
    use super::Runtime;
    use std::cell::Cell;

    /// Horloge simulée : `sleep` avance le temps. `interrupt_after` lève
    /// Ctrl-C à la n-ième pause.
    pub struct FakeRuntime {
        pub now: Cell<f64>,
        pub sleeps: Cell<u32>,
        pub interrupt_after: Cell<Option<u32>>,
    }

    impl FakeRuntime {
        pub fn new() -> Self {
            Self {
                now: Cell::new(0.0),
                sleeps: Cell::new(0),
                interrupt_after: Cell::new(None),
            }
        }
    }

    impl Runtime for FakeRuntime {
        fn now(&self) -> f64 {
            self.now.get()
        }
        fn sleep(&self, seconds: f64) {
            self.now.set(self.now.get() + seconds);
            self.sleeps.set(self.sleeps.get() + 1);
        }
        fn interrupted(&self) -> bool {
            self.interrupt_after
                .get()
                .is_some_and(|n| self.sleeps.get() >= n)
        }
    }
}

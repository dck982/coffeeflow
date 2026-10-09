//! Garde de zéro du dimmer.
//!
//! Le dimmer conserve son dernier niveau si l'écriture I2C d'arrêt échoue.
//! Tant que la vanne est fermée, réaffirmer zéro même après un succès : cela
//! couvre aussi un redémarrage du contrôleur ou une écriture perdue.

pub const RETRY_AFTER_ERROR_US: i64 = 100 * 1000;
pub const REFRESH_US: i64 = 1000 * 1000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DimmerZeroGuard {
    next_attempt_us: i64,
}

impl DimmerZeroGuard {
    pub const fn new() -> Self {
        Self { next_attempt_us: 0 }
    }

    pub fn tick(&mut self, now_us: i64, valve_open: bool, write_zero: impl FnOnce() -> bool) {
        if valve_open || now_us < self.next_attempt_us {
            return;
        }
        self.next_attempt_us = now_us
            + if write_zero() {
                REFRESH_US
            } else {
                RETRY_AFTER_ERROR_US
            };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_zero(ok: bool, level: &mut u8, writes: &mut i32) -> bool {
        *writes += 1;
        if ok {
            *level = 0;
        }
        ok
    }

    #[test]
    fn retries_after_error_then_refreshes_but_never_with_the_valve_open() {
        let mut guard = DimmerZeroGuard::new();
        let mut level = 100u8;
        let mut writes = 0;

        // L'arrêt immédiat a fermé la vanne, mais son écriture I2C a échoué.
        guard.tick(0, false, || write_zero(false, &mut level, &mut writes));
        assert_eq!((writes, level), (1, 100));
        guard.tick(99_999, false, || write_zero(false, &mut level, &mut writes));
        assert_eq!(writes, 1);

        // Une reprise du bus arrête la pompe sans nouvelle commande CAN.
        guard.tick(100_000, false, || write_zero(true, &mut level, &mut writes));
        assert_eq!((writes, level), (2, 0));

        // À l'arrêt, le zéro est réaffirmé périodiquement, jamais vanne ouverte.
        level = 80;
        guard.tick(1_100_000, true, || {
            write_zero(true, &mut level, &mut writes)
        });
        assert_eq!((writes, level), (2, 80));
        guard.tick(1_100_000, false, || {
            write_zero(true, &mut level, &mut writes)
        });
        assert_eq!((writes, level), (3, 0));
    }
}

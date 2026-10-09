//! Compteur du débitmètre, alimenté par l'interruption du front descendant.
//!
//! À 0,5 L/min et 2382 imp/L, deux fronts réels sont espacés d'environ
//! 50,4 ms. Rejeter ceux qui arrivent plus tôt élimine les rebonds sans
//! limiter le débit utile du Digmesa (spécifié à 0,40 L/min maximum).

pub const MIN_EDGE_INTERVAL_MS: i64 = 50;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlowCounter {
    pub pulse_count: u32,
    /// 0 tant qu'aucun front n'a été accepté.
    pub last_edge_us: i64,
}

impl FlowCounter {
    pub const fn new() -> Self {
        Self {
            pulse_count: 0,
            last_edge_us: 0,
        }
    }

    /// Front descendant à `edge_us`. `true` s'il est compté.
    pub fn on_edge(&mut self, edge_us: i64) -> bool {
        if self.last_edge_us != 0 && edge_us - self.last_edge_us < MIN_EDGE_INTERVAL_MS * 1000 {
            return false;
        }
        self.pulse_count = self.pulse_count.wrapping_add(1);
        self.last_edge_us = edge_us;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounces_inside_50_ms_are_rejected() {
        let mut flow = FlowCounter::new();
        assert!(flow.on_edge(1_000_000));
        assert!(!flow.on_edge(1_049_999));
        assert!(flow.on_edge(1_050_000));
        assert_eq!(flow.pulse_count, 2);
        assert_eq!(flow.last_edge_us, 1_050_000);
    }

    #[test]
    fn counter_wraps() {
        let mut flow = FlowCounter {
            pulse_count: u32::MAX,
            last_edge_us: 0,
        };
        assert!(flow.on_edge(5));
        assert_eq!(flow.pulse_count, 0);
    }
}

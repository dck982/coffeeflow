//! Échéancier d'un échantillonnage périodique appelé depuis une boucle plus
//! rapide dont le réveil est quantifié et dont l'heure de lecture varie avec
//! le travail qui précède. Les échéances restent sur la grille de départ :
//! poser « maintenant + période » ferait glisser chaque échantillon manqué de
//! quelques microsecondes d'un tour de boucle entier.

/// Période et avance tolérée, en microsecondes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleSchedule {
    pub period_us: i64,
    /// Avance tolérée sur l'échéance, un demi-tour de boucle : l'échantillon
    /// est pris au tour le plus proche de l'échéance plutôt qu'au premier qui
    /// la suit.
    pub early_us: i64,
}

impl SampleSchedule {
    pub const fn new(period_us: i64, early_us: i64) -> Self {
        Self {
            period_us,
            early_us,
        }
    }

    pub const fn due(&self, now_us: i64, deadline_us: i64) -> bool {
        now_us >= deadline_us - self.early_us
    }

    /// Échéance suivante, une période après la précédente. Si elle serait déjà
    /// due (retard d'au moins une période moins l'avance), la grille repart de
    /// maintenant plutôt que d'enchaîner des échantillons en rafale.
    pub const fn next(&self, now_us: i64, deadline_us: i64) -> i64 {
        let following = deadline_us + self.period_us;
        if self.due(now_us, following) {
            now_us + self.period_us
        } else {
            following
        }
    }
}

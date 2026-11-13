//! Pompe à engrenage MPB003 : conversion consigne vers rapport cyclique, et
//! tachymètre.
//!
//! La consigne de la pompe est une tension (0 à ≈ 4,83 V à son fil jaune),
//! produite par un filtre RC derrière une sortie PWM de la carte. Cette
//! tension n'est pas linéaire en rapport cyclique (la capacité se charge
//! par 1,5 kΩ et se décharge par 11,5 kΩ) : le rapport cyclique se lit dans
//! une table mesurée sur la carte.

/// Résolution du LEDC de l'ESP32-S3 pour la consigne (il s'arrête à 14 bits).
pub const PUMP_DUTY_BITS: u32 = 14;
/// Rapport cyclique de 100 % : sortie toujours haute. Le LEDC d'esp-hal
/// accepte 0..=2^bits ; 0 est une sortie toujours basse, donc 0 V exact.
pub const PUMP_DUTY_FULL: u16 = 1 << PUMP_DUTY_BITS;

/// Vitesse maximale, que `pump_pct` = 100 % demande : le maximum annoncé
/// pour l'article. Elle ne demande que ≈ 65,5 % de rapport cyclique (4,5 V
/// sur les [`PUMP_VMAX_V`] de la carte). Le rapport cyclique plein donne
/// ≈ 4 800 tr/min : pour ces ≈ 7 % de débit en plus, relever cette vitesse
/// et [`PUMP_RPM_PER_PCT`] ensemble (48 tr/min par %), ce qui change le sens
/// des réglages en % de l'écran.
pub const PUMP_MAX_RPM: u16 = 4500;
/// Vitesse visée par pour cent de `pump_pct`, linéaire.
pub const PUMP_RPM_PER_PCT: u16 = 45;
/// Tr/min par volt de consigne, mesuré sur la pompe.
pub const PUMP_RPM_PER_VOLT: f32 = 1000.0;
/// Tension de consigne maximale que la carte donne à la pompe : 5,09 V au
/// banc, × 4,98 / 5,16 parce que le fil jaune de la pompe vaut 4,98 V et non
/// les 5,16 V de la carte, × 0,983 pour les 176 kΩ d'entrée de la pompe.
/// À recaler à la première mise en route dans la machine.
pub const PUMP_VMAX_V: f32 = 4.83;

/// Pleine échelle du banc, avec laquelle la table ci-dessous a été mesurée.
const BENCH_FULL_SCALE_V: f32 = 5.09;

/// Rapport cyclique mesuré, en %, et tension de consigne relevée au banc.
const DUTY_PCT: [f32; 13] = [
    0.0, 0.1, 0.3, 0.5, 1.0, 2.0, 3.09, 7.79, 15.77, 32.37, 50.0, 87.87, 100.0,
];
// 0,318 V est une mesure, pas 1/π.
#[allow(clippy::approx_constant)]
const SETPOINT_V: [f32; 13] = [
    0.0, 0.071, 0.318, 0.576, 1.045, 1.481, 1.727, 2.455, 3.240, 4.09, 4.55, 5.02, 5.09,
];

/// Vitesse visée pour `pump_pct` (0 à 100, plafonné), en tr/min.
pub const fn pump_target_rpm(pump_pct: u8) -> u16 {
    let pct = if pump_pct > 100 { 100 } else { pump_pct };
    pct as u16 * PUMP_RPM_PER_PCT
}

/// Rapport cyclique LEDC (14 bits) qui donne la vitesse visée pour
/// `pump_pct`. 0 % donne exactement 0 ; une fraction de Vmax de 1 ou plus
/// donne [`PUMP_DUTY_FULL`].
pub fn pump_duty(pump_pct: u8) -> u16 {
    let volts = f32::from(pump_target_rpm(pump_pct)) / PUMP_RPM_PER_VOLT;
    duty_for_fraction(volts / PUMP_VMAX_V)
}

/// Interpolation linéaire dans la table, où la tension est exprimée en
/// fraction de la pleine échelle du banc.
fn duty_for_fraction(fraction: f32) -> u16 {
    if fraction <= 0.0 {
        return 0;
    }
    if fraction >= 1.0 {
        return PUMP_DUTY_FULL;
    }
    let mut i = 1;
    while i < SETPOINT_V.len() - 1 && SETPOINT_V[i] / BENCH_FULL_SCALE_V < fraction {
        i += 1;
    }
    let (lo_f, hi_f) = (
        SETPOINT_V[i - 1] / BENCH_FULL_SCALE_V,
        SETPOINT_V[i] / BENCH_FULL_SCALE_V,
    );
    let t = (fraction - lo_f) / (hi_f - lo_f);
    let pct = DUTY_PCT[i - 1] + t * (DUTY_PCT[i] - DUTY_PCT[i - 1]);
    let duty = (pct / 100.0 * f32::from(PUMP_DUTY_FULL) + 0.5) as u16;
    duty.min(PUMP_DUTY_FULL)
}

/// Écart minimal entre deux fronts du tachymètre : 5 000 tr/min donnent 6 ms
/// entre impulsions, ce qui est plus rapide que tout front réel.
pub const MIN_TACH_EDGE_INTERVAL_US: i64 = 3_000;
/// Sans front depuis ce délai, la pompe est arrêtée (sous 150 tr/min), et
/// une période plus longue n'en est pas une.
pub const TACH_STOPPED_AFTER_US: i64 = 200_000;
/// Un front plus proche du précédent que cette fraction de tour, en
/// millièmes, est un parasite. Les fronts parasites tombent au front opposé
/// du signal, au quart de tour ; les vrais sont à un demi-tour, ou à 35 %
/// d'un tour pour des demi-tours inégaux de 35 / 65.
pub const TACH_MIN_TURN_PERMILLE: u64 = 350;

/// Tachymètre de la pompe, alimenté par l'interruption du front descendant :
/// 2 impulsions par tour.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TachCounter {
    /// Cumulé depuis le démarrage, avec bouclage.
    pub pulse_count: u32,
    /// 0 tant qu'aucun front n'a été accepté.
    pub last_edge_us: i64,
    /// Dernière période entre deux fronts, 0 si inconnue.
    pub period_us: u32,
    /// Période d'avant, 0 si inconnue.
    pub prev_period_us: u32,
    /// Durée d'un tour à la vitesse visée par la consigne, 0 sans consigne.
    pub target_turn_us: u32,
}

impl TachCounter {
    pub const fn new() -> Self {
        Self {
            pulse_count: 0,
            last_edge_us: 0,
            period_us: 0,
            prev_period_us: 0,
            target_turn_us: 0,
        }
    }

    /// Vitesse visée par la consigne, 0 à l'arrêt. Elle borne la durée de
    /// tour de référence du filtre : une période mesurée trop longue (un
    /// vrai front rejeté pendant une accélération) ne peut pas le bloquer à
    /// un front sur deux.
    pub fn set_target_rpm(&mut self, rpm: u16) {
        self.target_turn_us = if rpm == 0 { 0 } else { 60_000_000 / u32::from(rpm) };
    }

    /// Durée de tour de référence du filtre : la plus courte de la
    /// mesure et de la consigne, 0 si aucune n'est connue.
    fn reference_turn_us(&self) -> u64 {
        let measured = match (self.period_us, self.prev_period_us) {
            (0, _) => 0,
            (p, 0) => 2 * u64::from(p),
            (p, q) => u64::from(p) + u64::from(q),
        };
        let target = u64::from(self.target_turn_us);
        match (measured, target) {
            (0, t) => t,
            (m, 0) => m,
            (m, t) => m.min(t),
        }
    }

    /// Front descendant à `edge_us`. `true` s'il est compté.
    pub fn on_edge(&mut self, edge_us: i64) -> bool {
        if self.last_edge_us != 0 {
            let period = edge_us - self.last_edge_us;
            if period < MIN_TACH_EDGE_INTERVAL_US {
                return false;
            }
            if period <= TACH_STOPPED_AFTER_US
                && (period as u64) * 1000 < self.reference_turn_us() * TACH_MIN_TURN_PERMILLE
            {
                return false;
            }
            if period > TACH_STOPPED_AFTER_US {
                // La pompe était arrêtée : cette durée n'est pas une période.
                self.period_us = 0;
                self.prev_period_us = 0;
            } else {
                self.prev_period_us = self.period_us;
                self.period_us = period as u32;
            }
        }
        self.pulse_count = self.pulse_count.wrapping_add(1);
        self.last_edge_us = edge_us;
        true
    }

    /// `true` si un front a été accepté depuis le démarrage.
    pub const fn seen(&self) -> bool {
        self.last_edge_us != 0
    }

    /// Vitesse en tr/min à `now_us`. Sur deux périodes (un tour), la somme
    /// moyenne l'écart entre les deux impulsions du tour ; sur une seule,
    /// elle donne un demi-tour. 0 sans front depuis 200 ms.
    pub fn rpm(&self, now_us: i64) -> u16 {
        if self.last_edge_us == 0 || now_us - self.last_edge_us > TACH_STOPPED_AFTER_US {
            return 0;
        }
        let rpm = if self.period_us > 0 && self.prev_period_us > 0 {
            60_000_000 / (u64::from(self.period_us) + u64::from(self.prev_period_us))
        } else if self.period_us > 0 {
            30_000_000 / u64::from(self.period_us)
        } else {
            0
        };
        rpm.min(u64::from(u16::MAX)) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_rpm_is_linear_and_capped() {
        assert_eq!(pump_target_rpm(0), 0);
        assert_eq!(pump_target_rpm(20), 900);
        assert_eq!(pump_target_rpm(100), PUMP_MAX_RPM);
        assert_eq!(pump_target_rpm(200), PUMP_MAX_RPM);
    }

    #[test]
    fn zero_percent_is_exactly_zero_duty() {
        assert_eq!(pump_duty(0), 0);
    }

    #[test]
    fn duty_is_monotonic_and_below_full_at_100_percent() {
        let mut previous = 0;
        for pct in 1..=100 {
            let duty = pump_duty(pct);
            assert!(duty > previous, "{pct} % : {duty} après {previous}");
            previous = duty;
        }
        assert!(pump_duty(100) < PUMP_DUTY_FULL);
    }

    #[test]
    fn hand_checked_points() {
        // 100 % : 4,5 V sur 4,83 V, soit 0,9317 ; la table donne 50 % à
        // 4,55 V (0,8939) et 87,87 % à 5,02 V (0,9862) : t = 0,4091, soit
        // 50 + 0,4091 × 37,87 = 65,49 %, 10 730 pas.
        let duty = pump_duty(100);
        assert!((10_725..=10_735).contains(&duty), "{duty}");
        // 20 % : 0,9 V sur 4,83 V, soit 0,1863 ; entre 0,5 % (0,1132) et 1 %
        // (0,2053) : 0,5 + 0,7925 × 0,5 = 0,896 %, 147 pas.
        let duty = pump_duty(20);
        assert!((145..=149).contains(&duty), "{duty}");
        // 1 % : 45 tr/min, 0,045 V, entre 0 et 0,1 % : 0,0635 %, 10 pas.
        let duty = pump_duty(1);
        assert!((9..=11).contains(&duty), "{duty}");
    }

    #[test]
    fn fraction_at_or_above_one_is_full() {
        assert_eq!(duty_for_fraction(1.0), PUMP_DUTY_FULL);
        assert_eq!(duty_for_fraction(1.5), PUMP_DUTY_FULL);
        assert_eq!(duty_for_fraction(-0.5), 0);
    }

    #[test]
    fn table_is_consistent() {
        for window in DUTY_PCT.windows(2) {
            assert!(window[0] < window[1]);
        }
        for window in SETPOINT_V.windows(2) {
            assert!(window[0] < window[1]);
        }
        // Les points de la table se retrouvent à l'arrondi près.
        assert_eq!(duty_for_fraction(4.55 / BENCH_FULL_SCALE_V), 8192);
    }

    /// Fronts réguliers à `period_us`, le premier à `start_us`.
    fn feed(tach: &mut TachCounter, start_us: i64, period_us: i64, count: i64) -> i64 {
        let mut t = start_us;
        for _ in 0..count {
            assert!(tach.on_edge(t));
            t += period_us;
        }
        t - period_us
    }

    #[test]
    fn no_edge_means_zero_rpm() {
        let tach = TachCounter::new();
        assert_eq!(tach.rpm(5_000_000), 0);
        assert!(!tach.seen());
    }

    #[test]
    fn rpm_from_regular_edges() {
        // 4 500 tr/min : 2 impulsions par tour, 6 667 µs entre deux.
        let mut tach = TachCounter::new();
        let last = feed(&mut tach, 1_000_000, 6_667, 10);
        let rpm = tach.rpm(last + 1_000);
        assert!((4_499..=4_501).contains(&rpm), "{rpm}");
        assert_eq!(tach.pulse_count, 10);
        assert!(tach.seen());
    }

    #[test]
    fn unequal_halves_average_over_one_turn() {
        // Demi-tours de 12 et 18 ms : un tour dure 30 ms, 2 000 tr/min.
        let mut tach = TachCounter::new();
        tach.on_edge(1_000_000);
        tach.on_edge(1_012_000);
        tach.on_edge(1_030_000);
        assert_eq!(tach.rpm(1_031_000), 2_000);
        tach.on_edge(1_042_000);
        assert_eq!(tach.rpm(1_043_000), 2_000);
    }

    #[test]
    fn single_period_counts_half_a_turn() {
        let mut tach = TachCounter::new();
        tach.on_edge(1_000_000);
        assert_eq!(tach.rpm(1_001_000), 0);
        tach.on_edge(1_020_000);
        // 20 ms par demi-tour : 1 500 tr/min.
        assert_eq!(tach.rpm(1_021_000), 1_500);
    }

    #[test]
    fn bounces_within_3_ms_are_rejected() {
        let mut tach = TachCounter::new();
        assert!(tach.on_edge(1_000_000));
        assert!(!tach.on_edge(1_002_999));
        assert!(tach.on_edge(1_003_000));
        assert_eq!(tach.pulse_count, 2);
        assert_eq!(tach.last_edge_us, 1_003_000);
    }

    #[test]
    fn speed_drops_to_zero_after_200_ms_without_edge() {
        let mut tach = TachCounter::new();
        let last = feed(&mut tach, 1_000_000, 10_000, 5);
        assert_ne!(tach.rpm(last + 200_000), 0);
        assert_eq!(tach.rpm(last + 200_001), 0);
    }

    #[test]
    fn restart_after_a_stop_forgets_the_old_periods() {
        let mut tach = TachCounter::new();
        let last = feed(&mut tach, 1_000_000, 10_000, 5);
        // Arrêt d'une seconde, puis un seul front : pas de période valide.
        tach.on_edge(last + 1_000_000);
        assert_eq!((tach.period_us, tach.prev_period_us), (0, 0));
        assert_eq!(tach.rpm(last + 1_001_000), 0);
        tach.on_edge(last + 1_010_000);
        assert_eq!(tach.rpm(last + 1_011_000), 3_000);
    }

    #[test]
    fn edge_at_a_quarter_turn_is_rejected() {
        // 4 605 tr/min, demi-tour de 6 515 µs : un parasite au front opposé,
        // 3 260 µs après un vrai front, comme dans la purge du 13/11.
        let mut tach = TachCounter::new();
        tach.set_target_rpm(4_500);
        let last = feed(&mut tach, 1_000_000, 6_515, 10);
        assert!(!tach.on_edge(last + 3_260));
        assert!(tach.on_edge(last + 6_515));
        assert_eq!(tach.pulse_count, 11);
        let rpm = tach.rpm(last + 7_000);
        assert!((4_600..=4_610).contains(&rpm), "{rpm}");
    }

    #[test]
    fn quarter_turn_glitch_is_rejected_without_a_target() {
        // Ralentissement après la consigne à 0 : seule la mesure sert.
        let mut tach = TachCounter::new();
        let last = feed(&mut tach, 1_000_000, 10_000, 5);
        assert!(!tach.on_edge(last + 5_000));
        assert!(tach.on_edge(last + 11_000));
    }

    #[test]
    fn unequal_halves_of_35_65_are_kept() {
        let mut tach = TachCounter::new();
        tach.set_target_rpm(2_000);
        let mut t = 1_000_000;
        for _ in 0..10 {
            assert!(tach.on_edge(t));
            t += 10_500;
            assert!(tach.on_edge(t));
            t += 19_500;
        }
        assert_eq!(tach.pulse_count, 20);
    }

    #[test]
    fn a_fast_start_is_followed_up_to_the_target() {
        // Démarrage où chaque demi-tour raccourcit de moitié : la consigne
        // borne la référence, aucun vrai front n'est perdu.
        let mut tach = TachCounter::new();
        tach.set_target_rpm(4_500);
        let mut t = 1_000_000;
        let mut period = 80_000;
        assert!(tach.on_edge(t));
        while period > 6_667 {
            t += period;
            assert!(tach.on_edge(t), "période {period}");
            period /= 2;
        }
        let last = feed(&mut tach, t + 6_667, 6_667, 5);
        assert!((4_499..=4_501).contains(&tach.rpm(last + 100)));
    }

    #[test]
    fn a_rejected_real_edge_does_not_lock_at_half_rate() {
        // Une période mesurée deux fois trop longue (front perdu) : la
        // consigne ramène la référence, le front suivant passe.
        let mut tach = TachCounter::new();
        tach.set_target_rpm(3_000);
        tach.on_edge(1_000_000);
        tach.on_edge(1_020_000);
        tach.on_edge(1_040_000);
        // 20 000 µs mesurés, 10 000 µs visés.
        let mut t = 1_040_000;
        for _ in 0..5 {
            t += 10_000;
            assert!(tach.on_edge(t));
        }
    }

    #[test]
    fn pulse_count_wraps() {
        let mut tach = TachCounter {
            pulse_count: u32::MAX,
            ..TachCounter::new()
        };
        assert!(tach.on_edge(5));
        assert_eq!(tach.pulse_count, 0);
    }
}

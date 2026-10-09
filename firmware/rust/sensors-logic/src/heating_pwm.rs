//! Modulation de la chauffe sur des fenêtres de 1 s.
//!
//! Répartit les faibles consignes sur plusieurs fenêtres. La sortie minimale
//! est une impulsion de 100 ms, soit 10 % d'une fenêtre ; l'accumulateur
//! conserve le crédit des fenêtres sans impulsion (1 % = 100 ms / 10 s).

pub const PERIOD_US: i64 = 1_000_000;
pub const QUANTUM_US: i64 = 100_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeatingPwm {
    active: bool,
    power_permille: u16,
    window_start_us: i64,
    window_on_us: i64,
    credit_us: i64,
    window_closed: bool,
    output_on: bool,
    evaluated: bool,
}

impl HeatingPwm {
    pub const fn new() -> Self {
        Self {
            active: false,
            power_permille: 0,
            window_start_us: 0,
            window_on_us: 0,
            credit_us: 0,
            window_closed: false,
            output_on: false,
            evaluated: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn set_power(&mut self, now_us: i64, permille: u16) {
        if permille > 1000 {
            self.reset();
            return;
        }
        if !self.active {
            if permille == 0 {
                return;
            }
            self.active = true;
            self.window_start_us = now_us;
            self.credit_us = QUANTUM_US / 2;
            self.power_permille = permille;
            self.allocate_window();
            return;
        }
        let previous_permille = self.power_permille;
        self.power_permille = permille;
        if permille == 0 {
            self.credit_us = QUANTUM_US / 2;
        }
        if self.advance_window(now_us) {
            if permille == 0 {
                self.window_closed = true;
            }
            return;
        }
        if permille == 0 {
            self.window_on_us = 0;
            self.window_closed = true;
            self.output_on = false;
            return;
        }
        // Une fois la sortie éteinte, aucune consigne ne peut la rallumer
        // avant la prochaine fenêtre. La nouvelle consigne reste celle de
        // cette fenêtre.
        if self.window_closed || (self.evaluated && !self.output_on) {
            self.window_closed = true;
            return;
        }
        if permille == previous_permille {
            return;
        }
        let requested_us = PERIOD_US * i64::from(permille) / 1000;
        if requested_us > self.window_on_us {
            self.window_on_us = requested_us / QUANTUM_US * QUANTUM_US;
        } else {
            // Une baisse importante coupe la fenêtre courante au plus tard
            // après le quantum minimal ; le crédit futur suit déjà la
            // nouvelle consigne.
            self.window_on_us = self
                .window_on_us
                .min(QUANTUM_US.max(requested_us / QUANTUM_US * QUANTUM_US));
        }
        if now_us - self.window_start_us >= self.window_on_us {
            self.window_closed = true;
            self.output_on = false;
        }
    }

    /// Ouvre une nouvelle période à `now_us` avec cette consigne, au lieu
    /// d'attendre la fin de la période en cours. Demandé par l'écran au début
    /// d'une chauffe forte (précharge, remplissage) : sans cela, une hausse
    /// reçue après l'impulsion de repos attend jusqu'à 1 s. Le crédit repart
    /// d'un demi-quantum, comme au premier démarrage.
    pub fn restart(&mut self, now_us: i64, permille: u16) {
        if permille == 0 || permille > 1000 {
            self.set_power(now_us, permille);
            return;
        }
        self.active = true;
        self.power_permille = permille;
        self.window_start_us = now_us;
        self.window_closed = false;
        self.output_on = false;
        self.evaluated = false;
        self.credit_us = QUANTUM_US / 2;
        self.allocate_window();
    }

    /// État voulu de la sortie à `now_us`.
    pub fn tick(&mut self, now_us: i64) -> bool {
        if !self.active {
            return false;
        }
        self.advance_window(now_us);
        let on = !self.window_closed
            && now_us >= self.window_start_us
            && now_us - self.window_start_us < self.window_on_us;
        if self.output_on && !on {
            self.window_closed = true;
        }
        self.output_on = on;
        self.evaluated = true;
        on
    }

    fn advance_window(&mut self, now_us: i64) -> bool {
        if now_us - self.window_start_us < PERIOD_US {
            return false;
        }
        self.window_start_us = now_us;
        self.window_closed = false;
        self.output_on = false;
        self.evaluated = false;
        self.allocate_window();
        true
    }

    fn allocate_window(&mut self) {
        self.credit_us += PERIOD_US * i64::from(self.power_permille) / 1000;
        self.window_on_us = PERIOD_US.min(self.credit_us / QUANTUM_US * QUANTUM_US);
        self.credit_us -= self.window_on_us;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_on(pwm: &mut HeatingPwm, from: i64, to: i64, step: i64) -> (usize, usize) {
        let (mut on, mut longest, mut run) = (0, 0, 0);
        let mut t = from;
        while t < to {
            if pwm.tick(t) {
                on += 1;
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
            t += step;
        }
        (on, longest)
    }

    #[test]
    fn one_percent_is_ten_pulses_of_100_ms_over_100_s() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 10);
        let (on, longest) = count_on(&mut pwm, 0, 100_000_000, 100_000);
        assert_eq!(on, 10);
        assert_eq!(longest, 1);
    }

    #[test]
    fn six_permille_gives_six_pulses() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 6);
        assert_eq!(count_on(&mut pwm, 0, 100_000_000, 100_000).0, 6);
    }

    #[test]
    fn a_drop_applies_without_waiting_for_the_window_end() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 800);
        assert!(pwm.tick(0));
        pwm.set_power(400_000, 6);
        assert!(!pwm.tick(400_000));
        // La hausse reçue après l'arrêt est retenue pour la fenêtre suivante.
        pwm.set_power(500_000, 1000);
        assert!(!pwm.tick(500_000));
        assert!(!pwm.tick(900_000));
        assert!(pwm.tick(1_000_000));
    }

    #[test]
    fn a_rise_extends_a_running_pulse() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 200);
        assert!(pwm.tick(0));
        pwm.set_power(100_000, 800);
        assert!(pwm.tick(400_000));
        pwm.set_power(500_000, 100);
        assert!(!pwm.tick(500_000));
        pwm.set_power(600_000, 1000);
        assert!(!pwm.tick(600_000));
        assert!(pwm.tick(1_000_000));
    }

    #[test]
    fn no_relight_inside_a_window() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 1000);
        assert!(pwm.tick(0));
        for i in 1..10 {
            let t = i * 100_000;
            pwm.set_power(t, if i % 2 == 0 { 1000 } else { 0 });
            assert!(!pwm.tick(t));
        }
        pwm.set_power(950_000, 1000);
        assert!(!pwm.tick(950_000));
        assert!(pwm.tick(1_000_000));
    }

    #[test]
    fn stop_then_lease_expiry_then_new_power_waits_for_next_window() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 1000);
        assert!(pwm.tick(0));
        pwm.set_power(200_000, 0);
        assert!(!pwm.tick(200_000));
        pwm.set_power(500_000, 0);
        pwm.set_power(600_000, 1000);
        assert!(!pwm.tick(600_000));
        assert!(pwm.tick(1_000_000));
    }

    #[test]
    fn one_percent_credit_reaches_the_quantum_after_five_windows() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 1000);
        assert!(pwm.tick(0));
        pwm.set_power(200_000, 0);
        assert!(!pwm.tick(200_000));
        pwm.set_power(400_000, 10);
        assert!(!pwm.tick(400_000));
        let mut t = 1_000_000;
        while t < 5_000_000 {
            assert!(!pwm.tick(t));
            t += 1_000_000;
        }
        assert!(pwm.tick(5_000_000));
        assert!(!pwm.tick(5_100_000));
    }

    #[test]
    fn initial_credit_alone_is_not_enough_for_one_percent() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 10);
        assert!(!pwm.tick(0));
        pwm.set_power(200_000, 1000);
        assert!(!pwm.tick(200_000));
        assert!(pwm.tick(1_000_000));
    }

    #[test]
    fn first_start_has_no_delay() {
        let mut pwm = HeatingPwm::new();
        assert!(!pwm.tick(0));
        pwm.set_power(0, 0);
        pwm.set_power(100_000, 1000);
        assert!(pwm.tick(100_000));
    }

    #[test]
    fn full_power_stays_on_across_window_boundaries() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 1000);
        let mut t = 0;
        while t <= 20_000_000 {
            assert!(pwm.tick(t));
            t += 100_000;
        }
    }

    #[test]
    fn restart_opens_a_new_window_at_once() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 30);
        assert_eq!(count_on(&mut pwm, 0, 10_000_000, 100_000).0, 3);
        pwm.tick(10_000_000);
        assert!(!pwm.tick(10_300_000));
        pwm.restart(10_300_000, 900);
        assert_eq!(count_on(&mut pwm, 10_300_000, 11_300_000, 100_000).0, 9);
        assert!(!pwm.tick(11_200_000));
        assert!(pwm.tick(11_300_000));
    }

    #[test]
    fn same_rise_without_restart_waits_until_11_s() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 30);
        let mut t = 0;
        while t <= 10_000_000 {
            pwm.tick(t);
            t += 100_000;
        }
        pwm.tick(10_300_000);
        pwm.set_power(10_300_000, 900);
        assert!(!pwm.tick(10_300_000));
        assert!(!pwm.tick(10_900_000));
        assert!(pwm.tick(11_000_000));
    }

    #[test]
    fn restart_during_a_pulse_extends_it_without_a_cut() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 200);
        assert!(pwm.tick(0));
        pwm.restart(100_000, 900);
        let mut t = 100_000;
        while t < 1_000_000 {
            assert!(pwm.tick(t));
            t += 100_000;
        }
        assert!(!pwm.tick(1_000_000));
    }

    #[test]
    fn restart_twice_does_not_accumulate_windows() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 30);
        pwm.tick(0);
        pwm.restart(300_000, 500);
        pwm.restart(350_000, 500);
        assert_eq!(count_on(&mut pwm, 350_000, 1_350_000, 50_000).0, 10);
    }

    #[test]
    fn restart_with_zero_behaves_like_set_power() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 1000);
        assert!(pwm.tick(0));
        pwm.restart(200_000, 0);
        assert!(!pwm.tick(200_000));
        pwm.restart(300_000, 1000);
        assert!(pwm.tick(300_000));
    }

    #[test]
    fn out_of_range_power_resets() {
        let mut pwm = HeatingPwm::new();
        pwm.set_power(0, 500);
        assert!(pwm.active());
        pwm.set_power(10, 1001);
        assert!(!pwm.active());
        assert!(!pwm.tick(20));
    }
}

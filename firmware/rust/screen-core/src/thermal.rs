//! Régulateur de chauffe de la chaudière.
//!
//! Régulateur de repos, de purge et de récupération. Pendant la précharge et
//! l'écoulement d'une infusion, il délègue la commande à
//! [`BrewHeating`](crate::brew_heating::BrewHeating) et ne fait que tenir à
//! jour son propre état.
//!
//! Au repos, un prédicteur de retard ; en purge et en récupération, la
//! prédiction par la pente et la chaleur en transit.

use crate::brew_heating::BrewHeating;
use crate::calibration::boiler_sensor_temperature_c;
use crate::config::BREW_TEMPERATURE_TOLERANCE_C;

/// `Brew` : remplissage et pré-infusion ; `Infusion` : infusion, rampe de fin
/// comprise. Le remboursement de la précharge commence avec `Infusion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    ThermalPreheat,
    Brew,
    Infusion,
    Purge,
}

/// Mesure de débit accompagnant un pas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flow {
    pub ml_s: f32,
    pub valid: bool,
    /// Début du remplissage, débit pas encore lu.
    pub starting: bool,
}

impl Flow {
    /// Aucune mesure de débit.
    pub const NONE: Self = Self {
        ml_s: 0.0,
        valid: false,
        starting: false,
    };

    /// L'eau entre mais le débitmètre n'a pas encore assez d'impulsions :
    /// la loi d'infusion chauffe au plafond.
    pub const STARTING: Self = Self {
        ml_s: 0.0,
        valid: false,
        starting: true,
    };

    pub const fn new(ml_s: f32, valid: bool) -> Self {
        Self {
            ml_s,
            valid,
            starting: false,
        }
    }
}

/// Commande de puissance et état « prêt ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Output {
    pub power_permille: u16,
    pub ready: bool,
    /// Première commande non nulle de la précharge ou de l'écoulement d'une
    /// infusion. Le module capteurs ouvre alors une nouvelle période SSR au
    /// lieu d'attendre la fin de la période de repos en cours (jusqu'à 1 s
    /// perdue).
    pub restart_window: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowPhase {
    None,
    Preheat,
    Flow,
}

#[derive(Debug, Clone, Copy, Default)]
struct CommandSample {
    at_ms: u64,
    power_pct: f32,
}

/// 32 s à un pas de 250 ms : couvre le retard du repos et la fenêtre de 22 s.
const COMMAND_HISTORY_CAPACITY: usize = 128;

#[derive(Debug, Clone)]
pub struct Controller {
    last_ms: u64,
    ready_since_ms: u64,
    last_temperature_c: f32,
    slope_c_per_s: f32,
    idle_integral_pct: f32,
    target_c: f32,
    filtered_power_pct: f32,
    has_filtered_power: bool,
    was_flowing: bool,
    was_preheating: bool,
    uncompensated_purge: bool,
    brew: BrewHeating,
    restarted_phase: WindowPhase,
    recovery_until_ms: u64,
    /// Début de la lecture NTC manquante en cours pendant l'infusion, 0 sinon.
    reading_lost_since_ms: u64,
    command_history: [CommandSample; COMMAND_HISTORY_CAPACITY],
    command_history_count: usize,
    command_history_next: usize,
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller {
    // Repos. Modèle ajusté sur des captures de repos : retard pur de 23,5 s
    // entre commande et NTC, 1,28 kJ/K, pertes de 0,45 W/K.
    pub const IDLE_DELAY_S: f32 = 23.5;
    pub const IDLE_HEAT_GAIN_C_PER_PCT_SECOND: f32 = 12.0 / 1280.0;
    pub const IDLE_LOSS_PCT_PER_K: f32 = 0.45 / 12.0;
    /// Les pertes suivent la température à la sonde, pas la consigne
    /// utilisateur. Ajustées avec l'offset de -10 °C alors en vigueur : 24,5 °C
    /// utilisateur, soit 34,5 °C à la sonde.
    pub const IDLE_LOSS_ZERO_SENSOR_C: f32 = 34.5;
    pub const IDLE_PROPORTIONAL_PCT_PER_C: f32 = 6.0;
    pub const IDLE_INTEGRAL_PCT_PER_C_SECOND: f32 = 0.05;
    /// L'intégrale corrige l'erreur du modèle de pertes : elle peut être
    /// négative.
    pub const IDLE_INTEGRAL_MIN_PCT: f32 = -10.0;
    pub const IDLE_INTEGRAL_MAX_PCT: f32 = 35.0;
    pub const IDLE_INTEGRAL_BAND_C: f32 = 8.0;
    /// L'intégrale n'apprend que près de l'équilibre : en montée, sa chaleur
    /// n'est pas comptée comme en route et provoquerait un dépassement.
    pub const IDLE_INTEGRAL_PREDICTED_BAND_C: f32 = 0.5;

    // Purge et récupération.
    pub const PREDICTION_HORIZON_S: f32 = 20.0;
    pub const RECOVERY_PREDICTION_HORIZON_S: f32 = 10.0;
    pub const COMMAND_TRANSIT_WINDOW_S: f32 = 22.0;
    pub const COMMAND_HEAT_GAIN_C_PER_PCT_SECOND: f32 = 0.007;
    pub const POWER_FILTER_TIME_CONSTANT_S: f32 = 5.0;
    pub const SLOPE_FILTER_TIME_CONSTANT_S: f32 = 8.0;
    pub const HOLD_POWER_PCT: f32 = 3.5;
    pub const FLOW_FEEDFORWARD_PCT: f32 = 18.0;
    pub const FLOW_POWER_LIMIT_PCT: f32 = 35.0;
    pub const FLOW_BONUS_START_ML_S: f32 = 2.0;
    pub const FLOW_BONUS_FULL_ML_S: f32 = 4.0;
    pub const FLOW_BONUS_MAX_PCT: f32 = 10.0;
    pub const FLOW_BONUS_ABOVE_TARGET_BAND_C: f32 = 2.0;
    pub const PURGE_FEEDFORWARD_ABOVE_TARGET_BAND_C: f32 = 2.0;
    pub const PURGE_COMPENSATION_BAND_C: f32 = 5.0;
    pub const RECOVERY_POWER_LIMIT_PCT: f32 = 35.0;
    pub const RECOVERY_DURATION_MS: u64 = 30000;
    /// L'entrée est dans le domaine utilisateur : 100 °C ici = 114 °C à la
    /// sonde avec l'offset de -14 °C appliqué lors de la lecture NTC.
    pub const MAXIMUM_BOILER_USER_TEMPERATURE_C: f32 = 100.0;
    /// Pendant la précharge et l'écoulement, une lecture NTC manquante coupe
    /// la chauffe sans effacer la loi d'infusion jusqu'à cette durée. Une
    /// trame arrivée quelques ms après le seuil de fraîcheur effaçait sinon la
    /// dette de précharge, et l'infusion chauffait l'eau du remplissage une
    /// seconde fois.
    pub const BREW_READING_GAP_MAX_MS: u64 = 1000;

    pub fn new() -> Self {
        Self {
            last_ms: 0,
            ready_since_ms: 0,
            last_temperature_c: 0.0,
            slope_c_per_s: 0.0,
            idle_integral_pct: 0.0,
            target_c: 0.0,
            filtered_power_pct: 0.0,
            has_filtered_power: false,
            was_flowing: false,
            was_preheating: false,
            uncompensated_purge: false,
            brew: BrewHeating::new(),
            restarted_phase: WindowPhase::None,
            recovery_until_ms: 0,
            reading_lost_since_ms: 0,
            command_history: [CommandSample {
                at_ms: 0,
                power_pct: 0.0,
            }; COMMAND_HISTORY_CAPACITY],
            command_history_count: 0,
            command_history_next: 0,
        }
    }

    /// Un pas de régulation. `valid` : mesure fraîche ; `enabled` : chauffage
    /// autorisé. L'un ou l'autre faux, ou une température hors plage, remet
    /// tout à zéro et coupe, sauf une lecture manquante brève pendant
    /// l'infusion (`BREW_READING_GAP_MAX_MS`), qui coupe seulement.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        now_ms: u64,
        temperature_c: f32,
        target_c: f32,
        valid: bool,
        enabled: bool,
        mode: Mode,
        flow: Flow,
    ) -> Output {
        let flow_ml_s = flow.ml_s;
        let flow_valid = flow.valid;
        let reading_lost = !valid || !temperature_c.is_finite();
        let brew_phase = matches!(mode, Mode::ThermalPreheat | Mode::Brew | Mode::Infusion)
            && (self.was_preheating || self.was_flowing);
        if reading_lost && enabled && target_c == self.target_c && brew_phase {
            if self.reading_lost_since_ms == 0 {
                self.reading_lost_since_ms = now_ms;
            }
            if now_ms.saturating_sub(self.reading_lost_since_ms) <= Self::BREW_READING_GAP_MAX_MS {
                self.brew.pause(now_ms);
                self.record_command(now_ms, 0.0);
                return Output::default();
            }
        }
        if !reading_lost {
            self.reading_lost_since_ms = 0;
        }
        if !valid
            || !enabled
            || !temperature_c.is_finite()
            || !target_c.is_finite()
            || temperature_c > Self::MAXIMUM_BOILER_USER_TEMPERATURE_C
        {
            self.reset();
            return Output::default();
        }
        if self.last_ms != 0 && target_c != self.target_c {
            self.reset();
        }
        self.target_c = target_c;
        if self.last_ms == 0 || now_ms <= self.last_ms || now_ms - self.last_ms > 2000 {
            self.slope_c_per_s = 0.0;
            self.ready_since_ms = 0;
            self.clear_command_history();
            self.last_temperature_c = temperature_c;
            self.last_ms = now_ms;
        }
        let preheating = mode == Mode::ThermalPreheat;
        let brewing = mode == Mode::Brew || mode == Mode::Infusion;
        let flowing = brewing || mode == Mode::Purge;
        // La dette de précharge passe à l'écoulement qui la suit ; tout autre
        // début de précharge ou d'écoulement part d'une loi d'infusion neuve.
        if preheating && !self.was_preheating {
            self.brew.reset();
        }
        if flowing != self.was_flowing {
            // La pente de température pendant l'échange d'eau ne prédit pas
            // celle de la chaudière une fois l'écoulement arrêté (ou démarré).
            self.slope_c_per_s = 0.0;
            self.last_temperature_c = temperature_c;
            self.last_ms = now_ms;
            if !flowing || !self.was_preheating {
                self.brew.reset();
            }
            if !flowing {
                self.recovery_until_ms = now_ms + Self::RECOVERY_DURATION_MS;
            } else {
                self.uncompensated_purge = mode == Mode::Purge
                    && libm::fabsf(temperature_c - target_c) >= Self::PURGE_COMPENSATION_BAND_C;
            }
            self.was_flowing = flowing;
        }
        self.was_preheating = preheating;
        // Une nouvelle précharge est une phase active à part entière : elle ne
        // doit pas hériter du plafond de récupération du cycle précédent.
        let recovering = !flowing && !preheating && now_ms < self.recovery_until_ms;
        let dt_s = ((now_ms.wrapping_sub(self.last_ms)) as f32 / 1000.0).min(1.0);
        if dt_s > 0.0 {
            let measured_slope = (temperature_c - self.last_temperature_c) / dt_s;
            let alpha = dt_s / (Self::SLOPE_FILTER_TIME_CONSTANT_S + dt_s);
            self.slope_c_per_s += alpha * (measured_slope - self.slope_c_per_s);
            self.last_temperature_c = temperature_c;
            self.last_ms = now_ms;
        }

        let error = target_c - temperature_c;
        // En purge, réduire progressivement les appoints entre la consigne et
        // consigne + 2 °C, sans couper net dès le premier dépassement.
        let purge_taper = if mode == Mode::Purge {
            (1.0 + error / Self::PURGE_FEEDFORWARD_ABOVE_TARGET_BAND_C).clamp(0.0, 1.0)
        } else {
            1.0
        };
        // En purge, l'appoint suit le débit et disparaît dès que celui-ci
        // baisse. Ne pas utiliser une mesure absente ou périmée.
        let flow_bonus_pct = if mode == Mode::Purge
            && flow_valid
            && flow_ml_s.is_finite()
            && error > -Self::FLOW_BONUS_ABOVE_TARGET_BAND_C
        {
            ((flow_ml_s - Self::FLOW_BONUS_START_ML_S)
                / (Self::FLOW_BONUS_FULL_ML_S - Self::FLOW_BONUS_START_ML_S))
                .clamp(0.0, 1.0)
                * Self::FLOW_BONUS_MAX_PCT
                * purge_taper
        } else {
            0.0
        };
        if libm::fabsf(error) <= BREW_TEMPERATURE_TOLERANCE_C {
            if self.ready_since_ms == 0 {
                self.ready_since_ms = now_ms;
            }
        } else {
            self.ready_since_ms = 0;
        }
        let ready = self.ready_since_ms != 0 && now_ms - self.ready_since_ms >= 3000;

        if preheating || brewing {
            // Loi d'infusion séparée : ni la prédiction, ni l'intégrale, ni le
            // filtre de sortie du régulateur de repos n'interviennent. La
            // commande est enregistrée pour l'estimation de chaleur en transit
            // de la reprise.
            let power = if preheating {
                self.brew.preheat_pct(now_ms, temperature_c, target_c)
            } else {
                let (ml_s, measured) = if flow.starting {
                    (BrewHeating::CAP_FLOW_ML_S, true)
                } else {
                    (flow_ml_s, flow_valid)
                };
                self.brew.flow_pct(
                    now_ms,
                    temperature_c,
                    target_c,
                    ml_s,
                    measured,
                    mode == Mode::Infusion,
                )
            };
            self.filtered_power_pct = power;
            self.has_filtered_power = true;
            let power_permille = libm::roundf(power.clamp(0.0, 100.0) * 10.0) as u16;
            self.record_command(now_ms, f32::from(power_permille) / 10.0);
            // Une seule fois par phase : répété à chaque trame, le flag
            // relancerait la période toutes les 500 ms et chaufferait en
            // continu.
            let phase = if preheating {
                WindowPhase::Preheat
            } else {
                WindowPhase::Flow
            };
            let restart_window = power_permille > 0 && self.restarted_phase != phase;
            if restart_window {
                self.restarted_phase = phase;
            }
            return Output {
                power_permille,
                ready,
                restart_window,
            };
        }
        self.restarted_phase = WindowPhase::None;

        if !flowing && !recovering {
            // Repos : prédicteur de retard. La chaleur commandée pendant les
            // `IDLE_DELAY_S` dernières secondes n'est pas encore visible à la
            // NTC ; elle est ajoutée à la mesure au lieu d'extrapoler la
            // pente, qui reflète des commandes passées. L'intégrale corrige
            // le modèle de pertes : la chaleur qu'elle commande compense des
            // pertes et n'est pas en route. La compter laisserait la NTC sous
            // la consigne d'autant.
            let loss_pct = Self::idle_loss_pct(temperature_c);
            let pending_c = self.command_pct_seconds(
                now_ms,
                Self::IDLE_DELAY_S,
                loss_pct + self.idle_integral_pct,
                false,
            ) * Self::IDLE_HEAT_GAIN_C_PER_PCT_SECOND;
            let predicted_error = target_c - (temperature_c + pending_c);
            let unclamped_pct = loss_pct
                + Self::IDLE_PROPORTIONAL_PCT_PER_C * predicted_error
                + self.idle_integral_pct;
            // Anti-emballement : ne pas intégrer dans le sens d'une sortie
            // saturée.
            let saturated = (unclamped_pct >= 100.0 && predicted_error > 0.0)
                || (unclamped_pct <= 0.0 && predicted_error < 0.0);
            if libm::fabsf(error) < Self::IDLE_INTEGRAL_BAND_C
                && libm::fabsf(predicted_error) < Self::IDLE_INTEGRAL_PREDICTED_BAND_C
                && dt_s > 0.0
                && !saturated
            {
                self.idle_integral_pct = (self.idle_integral_pct
                    + Self::IDLE_INTEGRAL_PCT_PER_C_SECOND * predicted_error * dt_s)
                    .clamp(Self::IDLE_INTEGRAL_MIN_PCT, Self::IDLE_INTEGRAL_MAX_PCT);
            }
            let power = (loss_pct
                + Self::IDLE_PROPORTIONAL_PCT_PER_C * predicted_error
                + self.idle_integral_pct)
                .clamp(0.0, 100.0);
            // Le filtre de la purge et de la récupération repart de cette
            // commande ; une commande nulle le laisse s'initialiser au
            // premier pas suivant.
            self.filtered_power_pct = power;
            if power > 0.0 {
                self.has_filtered_power = true;
            }
            let power_permille = libm::roundf(power * 10.0) as u16;
            self.record_command(now_ms, f32::from(power_permille) / 10.0);
            return Output {
                power_permille,
                ready,
                restart_window: false,
            };
        }

        // Purge et récupération : prédiction par la pente et la chaleur en
        // transit. La décision est figée au début de la purge : une purge
        // loin de la consigne peut viser à refroidir la chaudière, même si la
        // NTC franchit ensuite la consigne.
        if mode == Mode::Purge && self.uncompensated_purge {
            self.filtered_power_pct = 0.0;
            self.record_command(now_ms, 0.0);
            return Output {
                power_permille: 0,
                ready,
                restart_window: false,
            };
        }

        // Anticiper la chaleur encore en route. Pendant l'écoulement et la
        // récupération, une pente négative n'est pas extrapolée.
        let transit_heat_c = (if recovering { 6.0f32 } else { 1.5f32 }).min(
            self.command_pct_seconds(
                now_ms,
                Self::COMMAND_TRANSIT_WINDOW_S,
                Self::HOLD_POWER_PCT,
                true,
            ) * Self::COMMAND_HEAT_GAIN_C_PER_PCT_SECOND,
        );
        let predictive_slope = self.slope_c_per_s.max(0.0);
        let horizon_s = if recovering {
            Self::RECOVERY_PREDICTION_HORIZON_S
        } else {
            Self::PREDICTION_HORIZON_S
        };
        // Pendant la reprise, la pente montante contient déjà une partie de
        // la chaleur des commandes récentes : additionner les deux la
        // compterait deux fois.
        let predicted_rise_c = if recovering {
            (predictive_slope * horizon_s).max(transit_heat_c)
        } else {
            predictive_slope * horizon_s + transit_heat_c
        };
        let predicted_c = temperature_c + predicted_rise_c;
        let predicted_error = target_c - predicted_c;
        let mut power = Self::HOLD_POWER_PCT + 8.0 * predicted_error;
        if flowing {
            power = power.min(Self::FLOW_POWER_LIMIT_PCT);
        } else if recovering {
            power = power.min(Self::RECOVERY_POWER_LIMIT_PCT);
        }

        // Un dépassement prédit coupe immédiatement. Lisser les variations
        // positives restantes évite de poursuivre le bruit de la dérivée. La
        // commande effectivement envoyée alimente l'estimation d'inertie.
        if power <= 0.0 {
            self.filtered_power_pct = 0.0;
        } else {
            let limited_power = power.clamp(0.0, 100.0);
            if !self.has_filtered_power {
                self.filtered_power_pct = limited_power;
                self.has_filtered_power = true;
            } else if flowing && limited_power > self.filtered_power_pct {
                // Appoint de débit modeste, sans retard de filtre de sortie.
                self.filtered_power_pct = limited_power;
            } else if dt_s > 0.0 {
                let alpha = dt_s / (Self::POWER_FILTER_TIME_CONSTANT_S + dt_s);
                self.filtered_power_pct += alpha * (limited_power - self.filtered_power_pct);
            }
        }
        if flowing {
            self.filtered_power_pct = self.filtered_power_pct.min(Self::FLOW_POWER_LIMIT_PCT);
        }
        if recovering {
            self.filtered_power_pct = self.filtered_power_pct.min(Self::RECOVERY_POWER_LIMIT_PCT);
        }
        // Les appoints de purge et de débit suivent directement leurs
        // conditions courantes : le filtre de 5 s ne doit pas prolonger leur
        // réduction.
        let purge_feedforward_pct = if mode == Mode::Purge {
            Self::FLOW_FEEDFORWARD_PCT * purge_taper
        } else {
            0.0
        };
        let commanded_power_pct =
            self.filtered_power_pct.max(purge_feedforward_pct) + flow_bonus_pct;
        let power_permille = libm::roundf(commanded_power_pct.clamp(0.0, 100.0) * 10.0) as u16;
        self.record_command(now_ms, f32::from(power_permille) / 10.0);
        Output {
            power_permille,
            ready,
            restart_window: false,
        }
    }

    pub fn reset(&mut self) {
        self.last_ms = 0;
        self.ready_since_ms = 0;
        self.idle_integral_pct = 0.0;
        self.slope_c_per_s = 0.0;
        self.filtered_power_pct = 0.0;
        self.has_filtered_power = false;
        self.was_flowing = false;
        self.was_preheating = false;
        self.uncompensated_purge = false;
        self.brew.reset();
        self.restarted_phase = WindowPhase::None;
        self.recovery_until_ms = 0;
        self.reading_lost_since_ms = 0;
        self.clear_command_history();
    }

    fn idle_loss_pct(temperature_c: f32) -> f32 {
        (Self::IDLE_LOSS_PCT_PER_K
            * (boiler_sensor_temperature_c(temperature_c) - Self::IDLE_LOSS_ZERO_SENSOR_C))
            .max(0.0)
    }

    fn clear_command_history(&mut self) {
        self.command_history_count = 0;
        self.command_history_next = 0;
    }

    fn record_command(&mut self, now_ms: u64, power_pct: f32) {
        self.command_history[self.command_history_next] = CommandSample {
            at_ms: now_ms,
            power_pct,
        };
        self.command_history_next = (self.command_history_next + 1) % COMMAND_HISTORY_CAPACITY;
        if self.command_history_count < COMMAND_HISTORY_CAPACITY {
            self.command_history_count += 1;
        }
    }

    /// Somme, sur les `window_s` dernières secondes, de la commande moins
    /// `offset_pct`, en %·s. `positive_only` ignore les commandes sous
    /// l'offset. Chaque commande vaut jusqu'à la suivante ; la dernière,
    /// jusqu'à `now_ms`.
    fn command_pct_seconds(
        &self,
        now_ms: u64,
        window_s: f32,
        offset_pct: f32,
        positive_only: bool,
    ) -> f32 {
        if self.command_history_count == 0 {
            return 0.0;
        }
        let window_ms = (window_s * 1000.0) as u64;
        let cutoff_ms = now_ms.saturating_sub(window_ms);
        let excess = |power_pct: f32| {
            let value = power_pct - offset_pct;
            if positive_only { value.max(0.0) } else { value }
        };
        let mut pct_seconds = 0.0f32;
        let oldest = (self.command_history_next + COMMAND_HISTORY_CAPACITY
            - self.command_history_count)
            % COMMAND_HISTORY_CAPACITY;
        for i in 0..self.command_history_count {
            let sample = &self.command_history[(oldest + i) % COMMAND_HISTORY_CAPACITY];
            let end_ms = if i + 1 < self.command_history_count {
                self.command_history[(oldest + i + 1) % COMMAND_HISTORY_CAPACITY].at_ms
            } else {
                now_ms
            };
            let start_ms = sample.at_ms.max(cutoff_ms);
            let clipped_end_ms = end_ms.min(now_ms);
            if clipped_end_ms > start_ms {
                pct_seconds +=
                    excess(sample.power_pct) * (clipped_end_ms - start_ms) as f32 / 1000.0;
            }
        }
        pct_seconds
    }
}

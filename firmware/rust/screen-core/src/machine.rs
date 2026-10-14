//! Logique pure du cycle café : précharge thermique, remplissage,
//! pré-infusion, infusion régulée en pression, fin progressive et purge.
//! Aucune dépendance au bus ni à l'horloge : le temps est un argument.

use crate::config::{MINIMUM_BREW_PUMP_PCT, PreinfusionMode, RampdownMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Idle,
    ThermalPreheat,
    Filling,
    Preinfusion,
    Brew,
    Rampdown,
    Finished,
    Purge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum StopReason {
    #[default]
    None = 0,
    TargetTime = 1,
    TargetWeight = 2,
    Manual = 3,
    ScaleLost = 4,
    PurgeReleased = 5,
    PurgeTimeout = 6,
}

/// Réglages utiles au cycle (sous-ensemble de [`crate::config::Config`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    pub target_weight_g: f32,
    pub target_time_s: u16,
    pub target_pressure_bar: f32,
    pub brew_preheat_time_s: f32,
    pub filling_time_s: u16,
    pub filling_pressure_bar: f32,
    pub filling_pump_pct: u8,
    pub preinfusion_mode: PreinfusionMode,
    pub preinfusion_time_s: u16,
    pub preinfusion_pump_pct: u8,
    pub rampdown_mode: RampdownMode,
    pub rampdown_lead_time_s: f32,
    pub rampdown_lead_weight_g: f32,
    pub rampdown_pressure_drop_bar: f32,
    pub brew_pump_pct: u8,
    pub purge_pump_pct: u8,
    pub purge_max_s: u16,
}

impl From<&crate::config::Config> for Config {
    fn from(c: &crate::config::Config) -> Self {
        Self {
            target_weight_g: c.target_weight_g,
            target_time_s: c.target_time_s,
            target_pressure_bar: c.target_pressure_bar,
            brew_preheat_time_s: c.brew_preheat_time_s,
            filling_time_s: c.filling_time_s,
            filling_pressure_bar: c.filling_pressure_bar,
            filling_pump_pct: c.filling_pump_pct,
            preinfusion_mode: c.preinfusion_mode,
            preinfusion_time_s: c.preinfusion_time_s,
            preinfusion_pump_pct: c.preinfusion_pump_pct,
            rampdown_mode: c.rampdown_mode,
            rampdown_lead_time_s: c.rampdown_lead_time_s,
            rampdown_lead_weight_g: c.rampdown_lead_weight_g,
            rampdown_pressure_drop_bar: c.rampdown_pressure_drop_bar,
            brew_pump_pct: c.brew_pump_pct,
            purge_pump_pct: c.purge_pump_pct,
            purge_max_s: c.purge_max_s,
        }
    }
}

/// Mesures lues à chaque pas.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Input {
    pub weight_g: f32,
    pub scale_present: bool,
    pub pressure_bar: f32,
    pub pressure_valid: bool,
}

/// Commande de pompe et durée de validité (bail) à envoyer en SET.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Output {
    pub dimmer: u8,
    pub ttl_ms: u16,
}

const LEASE_MS: u16 = 500;
const SCALE_BACKWARDS_G: f32 = 5.0;
const FILLING_PRESSURE_GUARD_MS: u64 = 1000;
// La pression arrive toutes les 100 ms et `tick` tourne toutes les 50 ms :
// 150 ms au-dessus du seuil exigent deux mesures consécutives, ce qui écarte
// un paquet isolé.
const FILLING_PRESSURE_CONFIRM_MS: u64 = 150;
const BREW_PRESSURE_CONTROL_PERIOD_MS: u64 = 200;
// Transition hydraulique douce après la pré-infusion. La période des pas est
// adaptée à l'écart afin que la montée complète dure environ 2,5 s.
const BREW_RAMP_STEP_PCT: u8 = 5;
const BREW_RAMP_DURATION_MS: u64 = 2500;
// Au-delà, une indisponibilité prolongée ne doit pas provoquer un rattrapage
// d'intégrale brutal quand la pression réapparaît.
const BREW_PRESSURE_MAX_INTEGRATION_PERIOD_MS: u64 = 400;
const BREW_PRESSURE_ACTIVATION_MARGIN_BAR: f32 = 3.0;
// Réglage initial tiré de la première capture réelle : autour de 9 bar, le
// point de fonctionnement est proche de 70 %, avec 300 à 400 ms de retard.
const BREW_PRESSURE_KP_PCT_PER_BAR: f32 = 15.0;
const BREW_PRESSURE_KI_PCT_PER_BAR_SECOND: f32 = 3.0;

/// Machine d'état du cycle.
#[derive(Debug, Clone)]
pub struct Machine {
    state: State,
    stop_reason: StopReason,
    config: Config,
    started_ms: u64,
    phase_started_ms: u64,
    finished_ms: u64,
    /// Durée de la purge en cours, au bout de laquelle la pompe s'arrête.
    purge_duration_ms: u64,
    starting_weight_g: f32,
    preinfusion_pressure_start_bar: f32,
    weight_goal: bool,
    effective_preinfusion_mode: PreinfusionMode,
    preinfusion_start_weight_g: f32,
    filling_pressure_since_ms: u64,
    preinfusion_scale_armed: bool,
    brew_pump_pct: u8,
    brew_ramp_start_pct: u8,
    brew_ramp_step_period_ms: u16,
    brew_ramp_active: bool,
    brew_pressure_control_active: bool,
    brew_pressure_integral_pct: f32,
    last_brew_pressure_control_ms: u64,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Self {
        Self {
            state: State::Idle,
            stop_reason: StopReason::None,
            config: (&crate::config::Config::default()).into(),
            started_ms: 0,
            phase_started_ms: 0,
            finished_ms: 0,
            purge_duration_ms: 0,
            starting_weight_g: 0.0,
            preinfusion_pressure_start_bar: 0.0,
            weight_goal: false,
            effective_preinfusion_mode: PreinfusionMode::NONE,
            preinfusion_start_weight_g: 0.0,
            filling_pressure_since_ms: 0,
            preinfusion_scale_armed: false,
            brew_pump_pct: 0,
            brew_ramp_start_pct: 0,
            brew_ramp_step_period_ms: 0,
            brew_ramp_active: false,
            brew_pressure_control_active: false,
            brew_pressure_integral_pct: 0.0,
            last_brew_pressure_control_ms: 0,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn stop_reason(&self) -> StopReason {
        self.stop_reason
    }

    pub fn active(&self) -> bool {
        matches!(
            self.state,
            State::ThermalPreheat
                | State::Filling
                | State::Preinfusion
                | State::Brew
                | State::Rampdown
                | State::Purge
        )
    }

    pub fn weight_goal(&self) -> bool {
        self.weight_goal
    }

    pub fn starting_weight_g(&self) -> f32 {
        self.starting_weight_g
    }

    /// Poids en tasse, compté depuis le début du cycle, qui arrête la pompe.
    pub fn stop_weight_g(&self) -> f32 {
        if self.config.rampdown_mode == RampdownMode::Weight {
            self.config.target_weight_g
        } else {
            self.config.target_weight_g - self.config.rampdown_lead_weight_g
        }
    }

    pub fn target_time_s(&self) -> u16 {
        self.config.target_time_s
    }

    pub fn elapsed_ms(&self, now_ms: u64) -> u32 {
        let now_ms = if self.state == State::Finished {
            self.finished_ms
        } else {
            now_ms
        };
        if self.started_ms == 0 || now_ms < self.started_ms {
            0
        } else {
            (now_ms - self.started_ms) as u32
        }
    }

    pub fn phase_elapsed_ms(&self, now_ms: u64) -> u32 {
        if self.phase_started_ms == 0 || now_ms < self.phase_started_ms {
            0
        } else {
            (now_ms - self.phase_started_ms) as u32
        }
    }

    /// Démarre une infusion ; `false` si un cycle est déjà actif.
    pub fn start(&mut self, now_ms: u64, config: &Config, input: &Input) -> bool {
        if self.active() {
            return false;
        }
        self.config = *config;
        self.phase_started_ms = now_ms;
        self.started_ms = if self.config.brew_preheat_time_s > 0.0 {
            0
        } else {
            now_ms
        };
        self.finished_ms = 0;
        self.starting_weight_g = input.weight_g;
        self.preinfusion_start_weight_g = input.weight_g;
        self.preinfusion_pressure_start_bar = input.pressure_bar;
        self.filling_pressure_since_ms = 0;
        self.weight_goal = input.scale_present;
        self.effective_preinfusion_mode = self.config.preinfusion_mode;
        self.preinfusion_scale_armed =
            self.effective_preinfusion_mode.has(PreinfusionMode::WEIGHT) && input.scale_present;
        if !self.preinfusion_scale_armed {
            self.effective_preinfusion_mode =
                PreinfusionMode(self.effective_preinfusion_mode.0 & !PreinfusionMode::WEIGHT.0);
        }
        self.stop_reason = StopReason::None;
        self.state = if self.config.brew_preheat_time_s > 0.0 {
            State::ThermalPreheat
        } else {
            State::Filling
        };
        true
    }

    /// Arrêt manuel d'une infusion (pas d'une purge).
    pub fn stop(&mut self, now_ms: u64) -> bool {
        if !self.active() || self.state == State::Purge {
            return false;
        }
        self.finish(StopReason::Manual, now_ms);
        true
    }

    /// Démarre une purge qui s'arrête seule après `duration_ms`, sauf
    /// relâchement plus tôt ; `true` aussi si une purge est déjà en cours.
    pub fn purge_press(&mut self, now_ms: u64, config: &Config, duration_ms: u32) -> bool {
        if self.active() {
            return self.state == State::Purge;
        }
        self.config = *config;
        self.purge_duration_ms = u64::from(duration_ms);
        self.started_ms = now_ms;
        self.phase_started_ms = now_ms;
        self.finished_ms = 0;
        // Une purge n'a pas de but au poids : sans cette remise à zéro, elle
        // hériterait de celui de l'infusion précédente et l'écran afficherait
        // un poids, balance éteinte.
        self.weight_goal = false;
        self.stop_reason = StopReason::None;
        self.state = State::Purge;
        true
    }

    pub fn purge_release(&mut self, now_ms: u64) -> bool {
        if self.state != State::Purge {
            return false;
        }
        self.finish(StopReason::PurgeReleased, now_ms);
        true
    }

    /// Ferme l'écran de résumé.
    pub fn dismiss(&mut self) -> bool {
        if self.state != State::Finished {
            return false;
        }
        self.state = State::Idle;
        self.stop_reason = StopReason::None;
        true
    }

    fn finish(&mut self, reason: StopReason, now_ms: u64) {
        self.finished_ms = now_ms;
        self.state = State::Finished;
        self.stop_reason = reason;
    }

    fn target_pump_pct(&self) -> u8 {
        self.config.brew_pump_pct.max(MINIMUM_BREW_PUMP_PCT)
    }

    fn enter_brew(&mut self, now_ms: u64, ramp_from_preinfusion: bool) {
        self.state = State::Brew;
        self.phase_started_ms = now_ms;
        let target_pct = self.target_pump_pct();
        // Sans charge, la pompe ne débite rien sous 40 %. Après une pause à
        // 35 %, partir de MINIMUM_BREW_PUMP_PCT évite de perdre le début de
        // la rampe.
        self.brew_ramp_start_pct = if ramp_from_preinfusion {
            self.config
                .preinfusion_pump_pct
                .clamp(MINIMUM_BREW_PUMP_PCT, target_pct)
        } else {
            target_pct
        };
        self.brew_pump_pct = self.brew_ramp_start_pct;
        self.brew_ramp_active = self.brew_pump_pct < target_pct;
        if self.brew_ramp_active {
            let difference_pct = target_pct - self.brew_ramp_start_pct;
            let step_count = difference_pct.div_ceil(BREW_RAMP_STEP_PCT);
            self.brew_ramp_step_period_ms = (BREW_RAMP_DURATION_MS / u64::from(step_count)) as u16;
        } else {
            self.brew_ramp_step_period_ms = 0;
        }
        self.brew_pressure_control_active = false;
        self.brew_pressure_integral_pct = f32::from(self.brew_pump_pct);
        self.last_brew_pressure_control_ms = now_ms;
    }

    /// Avance le cycle ; renvoie la commande de pompe à émettre.
    pub fn tick(&mut self, now_ms: u64, input: &Input) -> Output {
        if self.state == State::Purge {
            if now_ms.wrapping_sub(self.started_ms) >= self.purge_duration_ms {
                self.finish(StopReason::PurgeTimeout, now_ms);
            } else {
                return Output {
                    dimmer: self.config.purge_pump_pct,
                    ttl_ms: LEASE_MS,
                };
            }
        }
        if !self.active() {
            return Output::default();
        }

        if self.state == State::ThermalPreheat {
            let duration_ms = (self.config.brew_preheat_time_s * 1000.0) as u64;
            if now_ms.wrapping_sub(self.phase_started_ms) < duration_ms {
                return Output {
                    dimmer: 0,
                    ttl_ms: LEASE_MS,
                };
            }
            self.state = State::Filling;
            self.started_ms = now_ms;
            self.phase_started_ms = now_ms;
            self.starting_weight_g = input.weight_g;
            self.preinfusion_start_weight_g = input.weight_g;
            self.preinfusion_pressure_start_bar = input.pressure_bar;
        }

        if self.weight_goal {
            let delta = input.weight_g - self.starting_weight_g;
            if !input.scale_present || delta < -SCALE_BACKWARDS_G {
                self.finish(StopReason::ScaleLost, now_ms);
                return Output::default();
            }
            let ramp_start = self.config.target_weight_g - self.config.rampdown_lead_weight_g;
            if self.state == State::Brew
                && self.config.rampdown_mode == RampdownMode::Weight
                && delta >= ramp_start
            {
                self.state = State::Rampdown;
                self.phase_started_ms = now_ms;
            }
            if delta >= self.stop_weight_g() {
                self.finish(StopReason::TargetWeight, now_ms);
                return Output::default();
            }
        } else if now_ms.wrapping_sub(self.started_ms)
            >= u64::from(self.config.target_time_s) * 1000
        {
            self.finish(StopReason::TargetTime, now_ms);
            return Output::default();
        }

        if self.state == State::Filling {
            let elapsed = now_ms.wrapping_sub(self.started_ms);
            if elapsed < FILLING_PRESSURE_GUARD_MS {
                return Output {
                    dimmer: self.config.filling_pump_pct,
                    ttl_ms: LEASE_MS,
                };
            }

            // Pression absolue : headspace plein et galette mouillée. Le
            // volume à 1 bar ne suit pas la mouture (23,5 à 30,7 ml du 26/09
            // au 04/10). La pression au repos, jusqu'à 1,44 bar, retombe
            // pendant la garde.
            let mut pressure_done = false;
            if input.pressure_valid && input.pressure_bar >= self.config.filling_pressure_bar {
                if self.filling_pressure_since_ms == 0 {
                    self.filling_pressure_since_ms = now_ms;
                }
                pressure_done = now_ms.wrapping_sub(self.filling_pressure_since_ms)
                    >= FILLING_PRESSURE_CONFIRM_MS;
            } else {
                self.filling_pressure_since_ms = 0;
            }

            let time_done = elapsed >= u64::from(self.config.filling_time_s) * 1000;
            if !time_done && !pressure_done {
                return Output {
                    dimmer: self.config.filling_pump_pct,
                    ttl_ms: LEASE_MS,
                };
            }

            if self.effective_preinfusion_mode == PreinfusionMode::NONE
                || (self.effective_preinfusion_mode == PreinfusionMode::TIME
                    && self.config.preinfusion_time_s == 0)
            {
                self.enter_brew(now_ms, false);
            } else {
                self.state = State::Preinfusion;
                self.phase_started_ms = now_ms;
            }
        }

        if self.state == State::Preinfusion {
            let mut done = false;
            if self.effective_preinfusion_mode.has(PreinfusionMode::TIME) {
                done |= now_ms.wrapping_sub(self.phase_started_ms)
                    >= u64::from(self.config.preinfusion_time_s) * 1000;
            }
            if self.preinfusion_scale_armed
                && input.weight_g - self.preinfusion_start_weight_g >= 0.1
            {
                done = true;
            }
            if !done {
                return Output {
                    dimmer: self.config.preinfusion_pump_pct,
                    ttl_ms: LEASE_MS,
                };
            }
            self.enter_brew(now_ms, true);
        }
        if self.state == State::Brew {
            let ramp = self.config.rampdown_mode == RampdownMode::Time
                && now_ms.wrapping_sub(self.started_ms)
                    >= ((f32::from(self.config.target_time_s) - self.config.rampdown_lead_time_s)
                        * 1000.0) as u64;
            if ramp
                || (self.config.rampdown_mode == RampdownMode::PressureDrop
                    && self.preinfusion_pressure_start_bar - input.pressure_bar
                        >= self.config.rampdown_pressure_drop_bar)
            {
                self.state = State::Rampdown;
                self.phase_started_ms = now_ms;
            }
        }
        if self.state == State::Brew && self.brew_ramp_active {
            let target_pct = self.target_pump_pct();
            let completed_steps = now_ms.wrapping_sub(self.phase_started_ms)
                / u64::from(self.brew_ramp_step_period_ms);
            let requested_pct = u64::from(self.brew_ramp_start_pct)
                + completed_steps * u64::from(BREW_RAMP_STEP_PCT);
            self.brew_pump_pct = requested_pct.min(u64::from(target_pct)) as u8;
            let pressure_error_bar = self.config.target_pressure_bar - input.pressure_bar;
            let minimum_bumpless_command_pct = f32::from(MINIMUM_BREW_PUMP_PCT)
                + BREW_PRESSURE_KP_PCT_PER_BAR * pressure_error_bar;
            let pressure_control_ready = input.pressure_valid
                && input.pressure_bar
                    >= self.config.target_pressure_bar - BREW_PRESSURE_ACTIVATION_MARGIN_BAR
                && f32::from(self.brew_pump_pct) >= minimum_bumpless_command_pct;
            if pressure_control_ready {
                // Une galette restrictive peut approcher la cible avant la fin
                // de la rampe. Laisser alors le PI prendre la main depuis la
                // commande courante évite de poursuivre mécaniquement jusqu'au
                // plafond.
                self.brew_ramp_active = false;
            } else if self.brew_pump_pct >= target_pct {
                self.brew_ramp_active = false;
                self.brew_pressure_integral_pct = f32::from(self.brew_pump_pct);
                self.last_brew_pressure_control_ms = now_ms;
            }
        }
        if self.state == State::Brew
            && !self.brew_ramp_active
            && now_ms.wrapping_sub(self.last_brew_pressure_control_ms)
                >= BREW_PRESSURE_CONTROL_PERIOD_MS
            && input.pressure_valid
        {
            // Cette date n'avance que lorsqu'une commande PI est réellement
            // calculée. Une courte absence de mesure ne réduit donc pas Ki.
            let control_elapsed_ms = now_ms
                .wrapping_sub(self.last_brew_pressure_control_ms)
                .min(BREW_PRESSURE_MAX_INTEGRATION_PERIOD_MS);
            self.last_brew_pressure_control_ms = now_ms;
            let error_bar = self.config.target_pressure_bar - input.pressure_bar;
            let pump_floor = f32::from(MINIMUM_BREW_PUMP_PCT);
            let pump_ceiling = f32::from(self.target_pump_pct());

            if !self.brew_pressure_control_active
                && input.pressure_bar
                    >= self.config.target_pressure_bar - BREW_PRESSURE_ACTIVATION_MARGIN_BAR
            {
                // Initialiser I pour que P + I reproduise la commande
                // courante : le passage en boucle fermée ne crée ainsi aucun
                // saut de puissance.
                self.brew_pressure_control_active = true;
                self.brew_pressure_integral_pct = (f32::from(self.brew_pump_pct)
                    - BREW_PRESSURE_KP_PCT_PER_BAR * error_bar)
                    .clamp(pump_floor, pump_ceiling);
            } else if self.brew_pressure_control_active {
                let proportional_pct = BREW_PRESSURE_KP_PCT_PER_BAR * error_bar;
                let candidate_integral_pct = self.brew_pressure_integral_pct
                    + BREW_PRESSURE_KI_PCT_PER_BAR_SECOND
                        * error_bar
                        * (control_elapsed_ms as f32 / 1000.0);
                let candidate_output_pct = proportional_pct + candidate_integral_pct;

                // Anti-windup conditionnel : ne pas pousser davantage
                // l'intégrale lorsqu'une saturation empêche déjà la commande
                // demandée.
                let winds_up_high = candidate_output_pct > pump_ceiling && error_bar > 0.0;
                let winds_up_low = candidate_output_pct < pump_floor && error_bar < 0.0;
                if !winds_up_high && !winds_up_low {
                    self.brew_pressure_integral_pct =
                        candidate_integral_pct.clamp(pump_floor, pump_ceiling);
                }
            }

            if self.brew_pressure_control_active {
                let output_pct = (BREW_PRESSURE_KP_PCT_PER_BAR * error_bar
                    + self.brew_pressure_integral_pct)
                    .clamp(pump_floor, pump_ceiling);
                self.brew_pump_pct = libm::roundf(output_pct) as u8;
            }
        }
        Output {
            dimmer: self.brew_pump_pct,
            ttl_ms: LEASE_MS,
        }
    }
}

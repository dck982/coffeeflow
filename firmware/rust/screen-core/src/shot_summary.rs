//! Résumé d'une infusion, calculé échantillon par échantillon pendant la
//! capture HF. Il ne relit jamais le tampon : une capture tronquée garde un
//! résumé complet.

const NAN: f32 = f32::NAN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ShotSegment {
    Preheat = 0,
    Filling = 1,
    Preinfusion = 2,
    Ramp = 3,
    Infusion = 4,
    Drip = 5,
}

pub const SHOT_SEGMENT_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShotSampleMode {
    Preheat,
    Filling,
    Preinfusion,
    Infusion,
    Cooldown,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotSummary {
    pub active: bool,
    /// Début et fin de chaque segment, en secondes depuis le début de la
    /// capture ; NaN pour un segment absent. Le segment en cours finit au
    /// dernier échantillon.
    pub segment_start_s: [f32; SHOT_SEGMENT_COUNT],
    pub segment_end_s: [f32; SHOT_SEGMENT_COUNT],
    pub elapsed_s: f32,
    pub pump_start_s: f32,
    pub pump_stop_s: f32,
    /// Infusion : de la consigne − 1 bar à l'arrêt de la pompe ; sans pression
    /// atteinte, toute la phase d'infusion.
    pub infusion_s: f32,
    pub infusion_cup_g_s: f32,
    pub infusion_ml_s: f32,
    pub cup_weight_g: f32,
    pub drip_gain_g: f32,
    pub drip_done: bool,
    pub cup_rate_g_s: f32,
    /// Moyenne NTC pondérée par la tasse jusqu'à l'arrêt de la pompe.
    pub cup_mean_c: f32,
    pub start_temperature_c: f32,
    pub min_temperature_c: f32,
    pub max_temperature_c: f32,
    /// Échelle de la frise, en secondes : arrêt prévu, puis réel, plus la
    /// réserve des gouttes ; la fin des gouttes une fois celles-ci terminées.
    pub axis_s: f32,
}

impl Default for ShotSummary {
    fn default() -> Self {
        // Comme `ShotSummary{}` en C++ : les tableaux de segments sont à zéro
        // jusqu'au premier `reset`, qui les passe à NaN.
        Self {
            active: false,
            segment_start_s: [0.0; SHOT_SEGMENT_COUNT],
            segment_end_s: [0.0; SHOT_SEGMENT_COUNT],
            elapsed_s: 0.0,
            pump_start_s: NAN,
            pump_stop_s: NAN,
            infusion_s: NAN,
            infusion_cup_g_s: NAN,
            infusion_ml_s: NAN,
            cup_weight_g: NAN,
            drip_gain_g: 0.0,
            drip_done: false,
            cup_rate_g_s: NAN,
            cup_mean_c: NAN,
            start_temperature_c: NAN,
            min_temperature_c: NAN,
            max_temperature_c: NAN,
            axis_s: 0.0,
        }
    }
}

impl ShotSummary {
    pub fn pump_running(&self) -> bool {
        self.pump_start_s.is_finite() && !self.pump_stop_s.is_finite()
    }

    pub fn pump_stopped(&self) -> bool {
        self.pump_stop_s.is_finite()
    }

    pub fn has(&self, segment: ShotSegment) -> bool {
        self.segment_start_s[segment as usize].is_finite()
    }

    pub fn duration_s(&self, segment: ShotSegment) -> f32 {
        let i = segment as usize;
        if self.has(segment) {
            self.segment_end_s[i] - self.segment_start_s[i]
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotSample {
    pub t_s: f32,
    pub mode: ShotSampleMode,
    pub pump_pct: u8,
    pub pressure_valid: bool,
    pub pressure_bar: f32,
    pub scale_present: bool,
    pub weight_g: f32,
    pub volume_ml: f32,
    pub temperature_valid: bool,
    pub temperature_c: f32,
}

impl Default for ShotSample {
    fn default() -> Self {
        Self {
            t_s: 0.0,
            mode: ShotSampleMode::Other,
            pump_pct: 0,
            pressure_valid: false,
            pressure_bar: 0.0,
            scale_present: false,
            weight_g: 0.0,
            volume_ml: 0.0,
            temperature_valid: false,
            temperature_c: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct WeightPoint {
    t_s: f32,
    weight_g: f32,
}

/// 2 s à 100 ms, plus de la marge pour une capture plus lente.
const WINDOW_CAPACITY: usize = 32;

#[derive(Debug, Clone)]
pub struct ShotSummaryBuilder {
    summary: ShotSummary,
    pressure_threshold_bar: f32,
    expected_stop_s: f32,
    pressure_reached: bool,
    pressure_t_s: f32,
    pressure_weight_g: f32,
    pressure_volume_ml: f32,
    infusion_start_s: f32,
    infusion_weight_g: f32,
    infusion_volume_ml: f32,
    last_weight_g: f32,
    last_volume_ml: f32,
    stopped_flows_frozen: bool,
    weight_origin_g: f32,
    weight_max_g: f32,
    weighted_sum: f32,
    weight_sum: f32,
    last_temperature_c: f32,
    stop_weight_g: f32,
    window: [WeightPoint; WINDOW_CAPACITY],
    window_count: usize,
    window_next: usize,
    /// `None` : aucun segment ouvert (le `kCount` du C++).
    current: Option<ShotSegment>,
}

impl Default for ShotSummaryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ShotSummaryBuilder {
    pub const PRESSURE_BELOW_TARGET_BAR: f32 = 1.0;
    /// Débit en tasse sur 2 s, comme l'estimation de fin de la chauffe.
    pub const CUP_RATE_WINDOW_S: f32 = 2.0;
    /// Arrêt prévu : 30 s de pompe après son démarrage. La machine est
    /// souvent éteinte entre deux cafés ; le cycle précédent n'est donc pas
    /// retenu.
    pub const EXPECTED_PUMP_RUN_S: f32 = 30.0;
    pub const AXIS_GROWTH_AT: f32 = 0.9;
    pub const AXIS_GROWTH_FACTOR: f32 = 1.25;
    pub const DRIP_RESERVE_S: f32 = 4.0;

    pub fn new() -> Self {
        Self {
            summary: ShotSummary::default(),
            pressure_threshold_bar: 8.0,
            expected_stop_s: Self::EXPECTED_PUMP_RUN_S,
            pressure_reached: false,
            pressure_t_s: 0.0,
            pressure_weight_g: 0.0,
            pressure_volume_ml: 0.0,
            infusion_start_s: 0.0,
            infusion_weight_g: 0.0,
            infusion_volume_ml: 0.0,
            last_weight_g: NAN,
            last_volume_ml: 0.0,
            stopped_flows_frozen: false,
            weight_origin_g: 0.0,
            weight_max_g: 0.0,
            weighted_sum: 0.0,
            weight_sum: 0.0,
            last_temperature_c: 0.0,
            stop_weight_g: 0.0,
            window: [WeightPoint::default(); WINDOW_CAPACITY],
            window_count: 0,
            window_next: 0,
            current: None,
        }
    }

    pub fn reset(&mut self, target_pressure_bar: f32, preheat_s: f32) {
        self.summary = ShotSummary {
            active: true,
            segment_start_s: [NAN; SHOT_SEGMENT_COUNT],
            segment_end_s: [NAN; SHOT_SEGMENT_COUNT],
            ..ShotSummary::default()
        };
        self.pressure_threshold_bar = target_pressure_bar - Self::PRESSURE_BELOW_TARGET_BAR;
        self.expected_stop_s = preheat_s.max(0.0) + Self::EXPECTED_PUMP_RUN_S;
        self.pressure_reached = false;
        self.pressure_t_s = NAN;
        self.pressure_weight_g = NAN;
        self.pressure_volume_ml = NAN;
        self.infusion_start_s = NAN;
        self.infusion_weight_g = NAN;
        self.infusion_volume_ml = NAN;
        self.last_weight_g = NAN;
        self.last_volume_ml = 0.0;
        self.stopped_flows_frozen = false;
        self.weight_origin_g = NAN;
        self.weight_max_g = 0.0;
        self.weighted_sum = 0.0;
        self.weight_sum = 0.0;
        self.last_temperature_c = NAN;
        self.stop_weight_g = NAN;
        self.window_count = 0;
        self.window_next = 0;
        self.current = None;
        self.summary.axis_s = self.expected_stop_s + Self::DRIP_RESERVE_S;
    }

    pub fn clear(&mut self) {
        self.summary = ShotSummary::default();
    }

    pub fn summary(&self) -> &ShotSummary {
        &self.summary
    }

    pub fn add(&mut self, sample: &ShotSample) {
        if !self.summary.active {
            return;
        }
        let t = sample.t_s;
        self.summary.elapsed_s = t;
        let pump_on = sample.pump_pct > 0;
        if pump_on && !self.summary.pump_start_s.is_finite() {
            self.summary.pump_start_s = t;
            self.expected_stop_s = t + Self::EXPECTED_PUMP_RUN_S;
        }
        if !pump_on && self.summary.pump_running() {
            self.summary.pump_stop_s = t;
        }

        let weight = self.relative_weight(sample);
        self.push_weight(t, weight);
        self.summary.cup_rate_g_s = self.cup_rate(t);
        self.track_temperature(sample, weight);
        self.track_segments(sample, weight);
        self.track_drips(t, weight);
        self.update_infusion(t);
        self.update_axis(t);
    }

    fn relative_weight(&mut self, sample: &ShotSample) -> f32 {
        if !sample.scale_present || !sample.weight_g.is_finite() {
            return NAN;
        }
        if !self.weight_origin_g.is_finite() {
            self.weight_origin_g = sample.weight_g;
        }
        sample.weight_g - self.weight_origin_g
    }

    fn push_weight(&mut self, t: f32, weight: f32) {
        self.window[self.window_next] = WeightPoint {
            t_s: t,
            weight_g: weight,
        };
        self.window_next = (self.window_next + 1) % WINDOW_CAPACITY;
        if self.window_count < WINDOW_CAPACITY {
            self.window_count += 1;
        }
    }

    /// Poids sur les 2 dernières secondes. Une mesure manquante ou négative
    /// dans la fenêtre la rend inconnue : une tasse retirée n'est pas un débit.
    fn cup_rate(&self, t: f32) -> f32 {
        if self.window_count < 2 {
            return NAN;
        }
        let last = self.window[(self.window_next + WINDOW_CAPACITY - 1) % WINDOW_CAPACITY];
        if !last.weight_g.is_finite() || last.weight_g < 0.0 {
            return NAN;
        }
        for i in 1..self.window_count {
            let p = self.window[(self.window_next + WINDOW_CAPACITY - 1 - i) % WINDOW_CAPACITY];
            if !p.weight_g.is_finite() || p.weight_g < 0.0 {
                return NAN;
            }
            if t - p.t_s >= Self::CUP_RATE_WINDOW_S - 1e-3 {
                return (last.weight_g - p.weight_g) / (t - p.t_s);
            }
        }
        NAN
    }

    fn track_temperature(&mut self, sample: &ShotSample, weight: f32) {
        if !sample.temperature_valid || !sample.temperature_c.is_finite() {
            return;
        }
        let temperature = sample.temperature_c;
        if !self.summary.start_temperature_c.is_finite() {
            self.summary.start_temperature_c = temperature;
        }
        if self.summary.pump_running() {
            self.summary.min_temperature_c = if self.summary.min_temperature_c.is_finite() {
                self.summary.min_temperature_c.min(temperature)
            } else {
                temperature
            };
            self.summary.max_temperature_c = if self.summary.max_temperature_c.is_finite() {
                self.summary.max_temperature_c.max(temperature)
            } else {
                temperature
            };
            // Chaque gramme compte avec la NTC du même instant ; le poids suit
            // son maximum courant, les reculs de la balance ne retirent rien.
            if weight.is_finite() && weight > self.weight_max_g {
                let previous = if self.last_temperature_c.is_finite() {
                    self.last_temperature_c
                } else {
                    temperature
                };
                self.weighted_sum += (weight - self.weight_max_g) * (previous + temperature) * 0.5;
                self.weight_sum += weight - self.weight_max_g;
                self.weight_max_g = weight;
                if self.weight_sum > 0.5 {
                    self.summary.cup_mean_c = self.weighted_sum / self.weight_sum;
                }
            }
        }
        self.last_temperature_c = temperature;
    }

    fn open(&mut self, segment: ShotSegment, t: f32) {
        if self.current == Some(segment) {
            return;
        }
        if let Some(current) = self.current {
            self.summary.segment_end_s[current as usize] = t;
        }
        self.current = Some(segment);
        self.summary.segment_start_s[segment as usize] = t;
    }

    fn track_segments(&mut self, sample: &ShotSample, weight: f32) {
        let t = sample.t_s;
        match sample.mode {
            ShotSampleMode::Preheat => self.open(ShotSegment::Preheat, t),
            ShotSampleMode::Filling => self.open(ShotSegment::Filling, t),
            ShotSampleMode::Preinfusion => self.open(ShotSegment::Preinfusion, t),
            ShotSampleMode::Infusion => {
                if !self.infusion_start_s.is_finite() {
                    self.infusion_start_s = t;
                    self.infusion_weight_g = weight;
                    self.infusion_volume_ml = sample.volume_ml;
                }
                if !self.pressure_reached
                    && sample.pressure_valid
                    && sample.pressure_bar >= self.pressure_threshold_bar
                {
                    self.pressure_reached = true;
                    self.pressure_t_s = t;
                    self.pressure_weight_g = weight;
                    self.pressure_volume_ml = sample.volume_ml;
                }
                self.open(
                    if self.pressure_reached {
                        ShotSegment::Infusion
                    } else {
                        ShotSegment::Ramp
                    },
                    t,
                );
                self.last_volume_ml = sample.volume_ml;
                if weight.is_finite() {
                    self.last_weight_g = weight;
                }
            }
            ShotSampleMode::Cooldown => {
                if !self.summary.drip_done {
                    self.open(ShotSegment::Drip, t);
                }
            }
            ShotSampleMode::Other => {}
        }
        if let Some(current) = self.current
            && !self.summary.drip_done
        {
            self.summary.segment_end_s[current as usize] = t;
        }
        // Pression jamais atteinte : la montée devient l'infusion à l'arrêt.
        if self.summary.pump_stopped()
            && !self.pressure_reached
            && self.summary.has(ShotSegment::Ramp)
            && !self.summary.has(ShotSegment::Infusion)
        {
            let ramp = ShotSegment::Ramp as usize;
            let infusion = ShotSegment::Infusion as usize;
            self.summary.segment_start_s[infusion] = self.summary.segment_start_s[ramp];
            self.summary.segment_end_s[infusion] = self.summary.segment_end_s[ramp];
            self.summary.segment_start_s[ramp] = NAN;
            self.summary.segment_end_s[ramp] = NAN;
        }
    }

    /// Gouttes : du stop de la pompe au premier retour à 0 du débit en tasse,
    /// ou à une tasse retirée. Le poids est alors figé.
    fn track_drips(&mut self, t: f32, weight: f32) {
        if self.summary.drip_done {
            return;
        }
        if weight.is_finite() && weight >= 0.0 {
            self.summary.cup_weight_g = weight;
        }
        if !self.summary.pump_stopped() {
            return;
        }
        if !self.stop_weight_g.is_finite() {
            self.stop_weight_g = self.summary.cup_weight_g;
        }
        if self.summary.cup_weight_g.is_finite() && self.stop_weight_g.is_finite() {
            self.summary.drip_gain_g = (self.summary.cup_weight_g - self.stop_weight_g).max(0.0);
        }
        let rate = self.summary.cup_rate_g_s;
        if t > self.summary.pump_stop_s && (!rate.is_finite() || rate <= 0.0) {
            self.summary.drip_done = true;
            if self.summary.has(ShotSegment::Drip) {
                self.summary.segment_end_s[ShotSegment::Drip as usize] = t;
            }
        }
    }

    fn update_infusion(&mut self, t: f32) {
        let reached = self.pressure_reached;
        let start = if reached {
            self.pressure_t_s
        } else {
            self.infusion_start_s
        };
        if !start.is_finite() {
            return;
        }
        // Sans pression atteinte, la durée n'est connue qu'à l'arrêt de la
        // pompe.
        if !reached && !self.summary.pump_stopped() {
            return;
        }
        let end = if self.summary.pump_stopped() {
            self.summary.pump_stop_s
        } else {
            t
        };
        let duration = (end - start).max(0.0);
        self.summary.infusion_s = duration;
        if duration < 1.0 {
            return;
        }
        let start_weight = if reached {
            self.pressure_weight_g
        } else {
            self.infusion_weight_g
        };
        let start_volume = if reached {
            self.pressure_volume_ml
        } else {
            self.infusion_volume_ml
        };
        if !self.summary.pump_stopped() || !self.stopped_flows_frozen {
            if start_weight.is_finite() && self.last_weight_g.is_finite() {
                self.summary.infusion_cup_g_s = (self.last_weight_g - start_weight) / duration;
            }
            if start_volume.is_finite() {
                self.summary.infusion_ml_s = (self.last_volume_ml - start_volume) / duration;
            }
            if self.summary.pump_stopped() {
                self.stopped_flows_frozen = true;
            }
        }
    }

    /// Arrêt prévu, agrandi de 25 % quand l'écoulé en atteint 90 % ; jamais
    /// réduit tant que la pompe tourne, pour que la frise ne se recompresse
    /// pas.
    fn update_axis(&mut self, t: f32) {
        if self.summary.drip_done {
            self.summary.axis_s = self.summary.segment_end_s[ShotSegment::Drip as usize];
            if !self.summary.axis_s.is_finite() {
                self.summary.axis_s = t;
            }
            return;
        }
        if self.summary.pump_stopped() {
            self.summary.axis_s = (self.summary.pump_stop_s + Self::DRIP_RESERVE_S).max(t + 0.5);
            return;
        }
        while t > Self::AXIS_GROWTH_AT * self.expected_stop_s {
            self.expected_stop_s *= Self::AXIS_GROWTH_FACTOR;
        }
        self.summary.axis_s = self.expected_stop_s + Self::DRIP_RESERVE_S;
    }
}

//! Loi de chauffe de l'infusion : précharge, puis remplissage, pré-infusion et
//! infusion. Elle est volontairement séparée du régulateur de repos
//! ([`crate::thermal::Controller`]) : régler la stabilisation à la consigne
//! ne doit pas modifier la compensation de l'eau admise.
//!
//! La commande ne dépend pas de la NTC, qui voit l'eau froide environ 11 s
//! avant la chaleur.
//!
//! La précharge est une avance sur l'appoint : dès l'entrée en infusion,
//! l'appoint est retenu jusqu'à avoir rendu l'énergie de la précharge. Sans ce
//! remboursement, l'eau du remplissage est chauffée deux fois, et l'excédent
//! tombe en tasse d'autant plus que l'infusion est longue.

#[derive(Debug, Clone, Default)]
pub struct BrewHeating {
    debt_pct_s: f32,
    last_ms: u64,
    last_preheat_pct: f32,
    last_withheld_pct: f32,
}

impl BrewHeating {
    pub const PREHEAT_POWER_PCT: f32 = 90.0;
    /// La précharge est supprimée si la NTC dépasse déjà la cible de 0,5 °C.
    pub const PREHEAT_ABOVE_TARGET_BAND_C: f32 = 0.5;
    pub const HOLD_POWER_PCT: f32 = 3.5;
    /// 4,18 J/(g·K) × 70,5 K (eau à 24,5 °C portée à 95 °C réels) / 12 J par
    /// %·s.
    pub const WATER_HEAT_PCT_PER_ML_S: f32 = 4.18 * 70.5 / 12.0;
    pub const POWER_LIMIT_PCT: f32 = 90.0;
    /// Débit qui porte l'appoint au plafond (≈ 3,5 ml/s, celui du remplissage
    /// à 75 %).
    pub const CAP_FLOW_ML_S: f32 =
        (Self::POWER_LIMIT_PCT - Self::HOLD_POWER_PCT) / Self::WATER_HEAT_PCT_PER_ML_S;
    /// Sans mesure de débit fraîche, reprendre l'ancien plancher d'infusion.
    pub const FLOW_FALLBACK_PCT: f32 = 45.0;
    /// Sécurité seulement. Une coupure proche de la consigne recréerait le
    /// trou de chauffe provoqué par le pic de précharge.
    pub const SAFETY_ABOVE_TARGET_C: f32 = 4.0;
    /// Une commande vaut au plus jusqu'à 1 s : au-delà, le pas manqué n'est
    /// pas compté, comme dans le régulateur.
    pub const MAXIMUM_STEP_MS: u64 = 1000;

    pub fn new() -> Self {
        Self::default()
    }

    /// Début d'un cycle, avec ou sans précharge, et fin de l'écoulement.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Précharge : 90 % sauf si la NTC dépasse déjà la cible de 0,5 °C. La
    /// commande envoyée s'ajoute à la dette jusqu'au pas suivant.
    pub fn preheat_pct(&mut self, now_ms: u64, temperature_c: f32, target_c: f32) -> f32 {
        self.settle(now_ms);
        self.last_preheat_pct = if temperature_c - target_c < Self::PREHEAT_ABOVE_TARGET_BAND_C {
            Self::PREHEAT_POWER_PCT
        } else {
            0.0
        };
        self.last_preheat_pct
    }

    /// Remplissage, pré-infusion (`infusing` faux) et infusion, rampe de fin
    /// comprise. La chauffe suit le débit jusqu'à l'arrêt de la pompe.
    pub fn flow_pct(
        &mut self,
        now_ms: u64,
        temperature_c: f32,
        target_c: f32,
        flow_ml_s: f32,
        flow_valid: bool,
        infusing: bool,
    ) -> f32 {
        self.settle(now_ms);
        let law_pct = Self::law(temperature_c, target_c, flow_ml_s, flow_valid);
        // Seul l'appoint de la loi est retenu : la sécurité, qui donne déjà
        // 0 %, ne rembourse rien.
        self.last_withheld_pct = if infusing && self.debt_pct_s > 0.0 {
            law_pct
        } else {
            0.0
        };
        law_pct - self.last_withheld_pct
    }

    /// Énergie de précharge encore à rendre, en %·s (12 J par %·s).
    pub fn debt_pct_s(&self) -> f32 {
        self.debt_pct_s
    }

    fn law(temperature_c: f32, target_c: f32, flow_ml_s: f32, flow_valid: bool) -> f32 {
        if temperature_c - target_c > Self::SAFETY_ABOVE_TARGET_C {
            return 0.0;
        }
        if !flow_valid || !flow_ml_s.is_finite() {
            return Self::FLOW_FALLBACK_PCT;
        }
        Self::POWER_LIMIT_PCT
            .min(Self::HOLD_POWER_PCT + flow_ml_s.max(0.0) * Self::WATER_HEAT_PCT_PER_ML_S)
    }

    /// Impute la commande du pas précédent sur la durée écoulée depuis : la
    /// précharge augmente la dette, l'appoint retenu la rembourse. Le dernier
    /// pas de remboursement peut rendre jusqu'à 250 ms de trop (≈ 0,27 kJ à
    /// 90 %).
    fn settle(&mut self, now_ms: u64) {
        if self.last_ms != 0 && now_ms > self.last_ms {
            let dt_s = (now_ms - self.last_ms).min(Self::MAXIMUM_STEP_MS) as f32 / 1000.0;
            self.debt_pct_s = (self.debt_pct_s
                + (self.last_preheat_pct - self.last_withheld_pct) * dt_s)
                .max(0.0);
        }
        self.last_ms = now_ms;
        self.last_preheat_pct = 0.0;
        self.last_withheld_pct = 0.0;
    }
}

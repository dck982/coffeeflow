//! Conversion des codes ADS1115 en grandeurs physiques : NTC de la chaudière
//! (pont A0/A1) et pression (A2).

use crate::calibration as cal;

/// Résultat d'une conversion de la paire NTC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoilerReading {
    /// Mesure exploitable (lecture réussie, rapport A1/A0 plausible,
    /// température entre -10 et 160 °C à la sonde).
    pub valid: bool,
    /// Température en domaine utilisateur (sonde moins l'offset), 0 si
    /// invalide.
    pub temperature_c: f32,
}

/// Convertit les codes `a0` (rail) et `a1` (point milieu) en température
/// chaudière. Le rapport `a0/a1` donne la résistance de la NTC dans le pont
/// à résistance fixe ; la loi bêta donne la température de la sonde, puis
/// l'offset utilisateur s'applique.
pub fn boiler_temperature(a0_raw: i16, a1_raw: i16, read_ok: bool) -> BoilerReading {
    let mut valid = read_ok && (16000..32767).contains(&a0_raw) && a1_raw > 0 && a1_raw < a0_raw;
    let mut temperature_c = 0.0f32;
    if valid {
        let resistance = cal::BOILER_NTC_FIXED_OHM * (f32::from(a0_raw) / f32::from(a1_raw) - 1.0);
        let inverse_k = 1.0 / cal::BOILER_NTC_T0_K
            + libm::logf(resistance / cal::BOILER_NTC_R0_OHM) / cal::BOILER_NTC_BETA_K;
        let unadjusted_temperature_c = 1.0 / inverse_k - 273.15;
        temperature_c = cal::boiler_user_temperature_c(unadjusted_temperature_c);
        valid = resistance.is_finite()
            && unadjusted_temperature_c.is_finite()
            && resistance > 0.0
            && (-10.0..=160.0).contains(&unadjusted_temperature_c);
    }
    BoilerReading {
        valid,
        temperature_c,
    }
}

/// Pression relative en bar d'un code A2 de l'ADS1115.
pub fn pressure_bar_from_a2(raw: i16) -> f32 {
    (f32::from(raw) - cal::PRESSURE_A2_ZERO_CODE) * cal::PRESSURE_BAR_PER_A2_CODE
}

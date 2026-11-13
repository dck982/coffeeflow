//! Calibration propre à cette machine. Ces valeurs sont compilées : une image
//! doit être reproductible avec ce qui a été mesuré. Les réglages utilisateur
//! vivent en NVS (module `config`).

pub const SCHEMA_VERSION: u32 = 1;
/// Mesuré à la balance le 2026-11-13 avec la pompe à engrenage : purge à
/// 0 bar, 150 impulsions pour 58,5 g, à ≈ 6 ml/s, près du haut de la plage
/// du Digmesa. Le 3 450 imp/L pris avec la pompe vibratoire (débit pulsé, à
/// ~8,5 bar) ne vaut plus. Le K catalogue est de 2 382 imp/L.
pub const FLOW_PULSES_PER_LITER: f32 = 2564.0;
/// Tachymètre de la pompe à engrenage MPB003 : impulsions par tour.
pub const PUMP_TACH_PULSES_PER_TURN: u32 = 2;
/// Cylindrée de la pompe, en ml par tour. Le « refoulé » est tours ×
/// cylindrée : le glissement n'est pas retiré, ce n'est pas le débit net.
/// L'écart avec le Digmesa est le glissement.
pub const PUMP_ML_PER_TURN: f32 = 0.1;
/// XDB401 analogique (0,4 à 2,4 V pour 0 à 12 bar) sur A2 de l'ADS1115, PGA
/// ±4,096 V, soit 125 µV par code. Zéro lu sonde à l'air libre : la pression
/// est relative à l'atmosphère.
pub const PRESSURE_A2_ZERO_CODE: f32 = 3167.3;
pub const PRESSURE_BAR_PER_A2_CODE: f32 = 0.00075;
/// Résistance fixe du pont NTC chaudière, déduite du point de glace.
pub const BOILER_NTC_FIXED_OHM: f32 = 4676.0;
pub const BOILER_NTC_R0_OHM: f32 = 47000.0;
pub const BOILER_NTC_BETA_K: f32 = 3930.0;
pub const BOILER_NTC_T0_K: f32 = 298.15;
/// L'offset concerne la consigne utilisateur, pas la conversion physique de
/// la NTC. Mesuré au panier avec la sonde SCACE le 2026-10-10 : la
/// température utilisateur suit l'eau au panier, moyennée sur l'infusion.
pub const BOILER_NTC_TEMPERATURE_OFFSET_C: f32 = -14.0;

pub const fn boiler_user_temperature_c(sensor_c: f32) -> f32 {
    sensor_c + BOILER_NTC_TEMPERATURE_OFFSET_C
}

pub const fn boiler_sensor_temperature_c(user_c: f32) -> f32 {
    user_c - BOILER_NTC_TEMPERATURE_OFFSET_C
}

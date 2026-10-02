// Calibration propre à cette machine. Ce fichier est volontairement versionné :
// une image OTA doit être reproductible avec les valeurs qui ont été mesurées.
// Les réglages utilisateur, eux, vivent dans NVS (core/config.cpp).
#pragma once

namespace core::calibration_machine {

inline constexpr unsigned kSchemaVersion = 1;
// Mesuré avec le porte-filtre de simulation à ~8,5 bar le 2026-09-17.
// Le K catalogue (2382 imp/L) sur-estimait le débit d'environ 45 % sur le
// montage réel ; conserver cette valeur machine plutôt que le nominal.
inline constexpr float kFlowPulsesPerLiter = 3450.0f;
// XDB401 annoncé pour une plage de 0 à 16 bar. Les essais de purge du
// 2026-09-17 sont cohérents avec cette pleine échelle; aucun offset fiable
// n'a été établi avec le manomètre filmé.
inline constexpr float kPressureFullScaleBar = 16.0f;
inline constexpr float kPressureOffsetBar = 0.0f;
// Pont NTC chaudière : courbe de la sonde démontée, étalonnée sur banc le
// 2026-10-02 (glace, 25 °C, ébullition ; docs/ntc_ads1115_calibration.md).
// Les essais de flashing du 2026-09-27 placent 94 °C utilisateur près de
// 104 °C à la sonde avec l'ancienne courbe 3950 K, 104,5 °C avec 3930 K, et
// 98 à 98,5 °C en sortie de groupe. L'offset concerne la consigne
// utilisateur, pas la conversion physique de la NTC : passé de −10 à −10,5 °C
// avec la courbe 3930 K pour garder la même cible physique.
// Résistance fixe 4,7 kΩ du banc PT1000, 4676 Ω déduits du point de glace
// (docs/ntc_ads1115_calibration.md) ; elle remplace la 2193 Ω depuis 0.3.33.
inline constexpr float kBoilerNtcFixedOhm = 4676.0f;
inline constexpr float kBoilerNtcR0Ohm = 47000.0f;
inline constexpr float kBoilerNtcBetaK = 3930.0f;
inline constexpr float kBoilerNtcT0K = 298.15f;
inline constexpr float kBoilerNtcTemperatureOffsetC = -10.5f;
inline constexpr float boiler_user_temperature_c(float sensor_c) {
  return sensor_c + kBoilerNtcTemperatureOffsetC;
}
inline constexpr float boiler_sensor_temperature_c(float user_c) {
  return user_c - kBoilerNtcTemperatureOffsetC;
}

}  // namespace core::calibration_machine

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
// Pont NTC chaudière : courbe locale indicative de la sonde démontée.
// Les essais de flashing du 2026-09-27 placent 94 °C utilisateur près de
// 104 °C à la sonde et 98 à 98,5 °C en sortie de groupe. L'offset concerne
// la consigne utilisateur, pas la conversion physique de la NTC.
inline constexpr float kBoilerNtcFixedOhm = 2193.0f;
inline constexpr float kBoilerNtcR0Ohm = 47000.0f;
inline constexpr float kBoilerNtcBetaK = 3950.0f;
inline constexpr float kBoilerNtcT0K = 298.15f;
inline constexpr float kBoilerNtcTemperatureOffsetC = -10.0f;
inline constexpr float boiler_user_temperature_c(float sensor_c) {
  return sensor_c + kBoilerNtcTemperatureOffsetC;
}
inline constexpr float boiler_sensor_temperature_c(float user_c) {
  return user_c - kBoilerNtcTemperatureOffsetC;
}

}  // namespace core::calibration_machine

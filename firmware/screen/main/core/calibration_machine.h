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
// Pont NTC chaudière : valeurs provisoires de docs/ntc_ads1115_calibration.md.
// Remplacer après une mesure stabilisée, sans changer le pilote ADS1115.
inline constexpr float kBoilerNtcFixedOhm = 2193.0f;
inline constexpr float kBoilerNtcR0Ohm = 27290.0f;
inline constexpr float kBoilerNtcBetaK = 3728.0f;
inline constexpr float kBoilerNtcT0K = 298.15f;

}  // namespace core::calibration_machine

// Calibration propre à cette machine. Ce fichier est volontairement versionné :
// une image OTA doit être reproductible avec les valeurs qui ont été mesurées.
// Les réglages utilisateur, eux, vivent dans NVS (core/config.cpp).
#pragma once

namespace core::calibration_machine {

inline constexpr unsigned kSchemaVersion = 1;
inline constexpr float kFlowPulsesPerLiter = 2382.0f;
// XDB401 annoncé pour une plage de 0 à 16 bar. Les essais de purge du
// 2026-09-17 sont cohérents avec cette pleine échelle; aucun offset fiable
// n'a été établi avec le manomètre filmé.
inline constexpr float kPressureFullScaleBar = 16.0f;
inline constexpr float kPressureOffsetBar = 0.0f;

}  // namespace core::calibration_machine

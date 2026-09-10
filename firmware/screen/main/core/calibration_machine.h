// Calibration propre à cette machine. Ce fichier est volontairement versionné :
// une image OTA doit être reproductible avec les valeurs qui ont été mesurées.
// Les réglages utilisateur, eux, vivent dans NVS (core/config.cpp).
#pragma once

namespace core::calibration_machine {

inline constexpr unsigned kSchemaVersion = 1;
inline constexpr float kFlowPulsesPerLiter = 2382.0f;
inline constexpr float kPressureFullScaleBar = 10.0f;  // à confirmer sur la pièce montée

}  // namespace core::calibration_machine

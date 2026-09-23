#pragma once

#include <cstdint>

namespace common {

// Version de l'image courante, remontée dans PONG (voir protocol.md / firmware.md).
// À incrémenter à chaque image flashée, dans les deux projets.
inline constexpr uint8_t kFirmwareVersionMajor = 0;
inline constexpr uint8_t kFirmwareVersionMinor = 2;
inline constexpr uint8_t kFirmwareVersionPatch = 67;
// Version minimale de l'interface chauffage. Le jeton de confirmation CAN
// reste stable pendant la migration screen 0.2.63 -> sensors 0.2.64.
inline constexpr uint8_t kHeatingProtocolMinPatch = 63;

}  // namespace common

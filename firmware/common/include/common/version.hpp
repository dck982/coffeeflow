#pragma once

#include <cstdint>

namespace common {

// Version de l'image courante, remontée dans PONG (voir protocol.md / firmware.md).
// À incrémenter à chaque image flashée, dans les deux projets.
inline constexpr uint8_t kFirmwareVersionMajor = 0;
inline constexpr uint8_t kFirmwareVersionMinor = 2;
inline constexpr uint8_t kFirmwareVersionPatch = 23;

}  // namespace common

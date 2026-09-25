#pragma once

#include <cstdint>

namespace common {

// Version de l'image courante, remontée dans PONG (voir protocol.md / firmware.md).
// À incrémenter à chaque image flashée, dans les deux projets.
inline constexpr uint8_t kFirmwareVersionMajor = 0;
inline constexpr uint8_t kFirmwareVersionMinor = 3;
inline constexpr uint8_t kFirmwareVersionPatch = 11;
// Version minimale de l'interface chauffage, comparée sur les trois nombres.
inline constexpr uint8_t kHeatingProtocolMinMajor = 0;
inline constexpr uint8_t kHeatingProtocolMinMinor = 2;
inline constexpr uint8_t kHeatingProtocolMinPatch = 63;
inline constexpr bool heating_protocol_supported(uint8_t major, uint8_t minor, uint8_t patch) {
  return major == kHeatingProtocolMinMajor &&
         (minor > kHeatingProtocolMinMinor ||
          (minor == kHeatingProtocolMinMinor && patch >= kHeatingProtocolMinPatch));
}
// Le jeton CAN de confirmation OTA reste stable lors du passage à 0.3.0.
inline constexpr uint8_t kHeatingProtocolConfirmationToken = 63;

}  // namespace common

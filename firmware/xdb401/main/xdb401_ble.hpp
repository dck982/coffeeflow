// Service BLE de la sonde XDB401 : une trame par mesure, à 10 Hz. Lu par
// firmware/analysis/xdb401_ble_log.py, qui reprend ces UUID et ce format.
// Service distinct de SCACE, pour que l'écran de la machine ne s'y connecte
// pas.
#pragma once

#include <cstddef>
#include <cstdint>

namespace xdb401 {

inline constexpr char kDeviceName[] = "XDB401";
inline constexpr char kServiceUuid[] = "33ebc0eb-99ef-44b7-bc7a-482d582acac4";
inline constexpr char kFrameUuid[] = "29f79e86-5891-4811-9310-56d44f05ce02";

// Mêmes UUID, octets dans l'ordre de l'air (petit-boutiste), pour NimBLE.
inline constexpr uint8_t kServiceUuidLe[16] = {0xc4, 0xca, 0x2a, 0x58, 0x2d, 0x48, 0x7a, 0xbc,
                                               0xb7, 0x44, 0xef, 0x99, 0xeb, 0xc0, 0xeb, 0x33};
inline constexpr uint8_t kFrameUuidLe[16] = {0x02, 0xce, 0x05, 0x4f, 0xd4, 0x56, 0x10, 0x93,
                                             0x11, 0x48, 0x91, 0x58, 0x86, 0x9e, 0xf7, 0x29};

enum class Status : uint8_t {
  // Port A coupé ou sonde introuvable. Envoyé une fois par seconde.
  kNoMeasurement = 0,
  kOk = 1,
};

// Trame de 12 octets, petit-boutiste :
//   0  uint16  numéro de séquence
//   2  uint16  temps sonde en ms modulo 65 536
//   4  int32   code pression signé 24 bits ; bar = code × 16 / 2^23
//   8  int16   température du corps de la sonde en centièmes de °C
//  10  uint8   état
//  11  uint8   réservé, 0
struct Frame {
  uint16_t seq = 0;
  uint16_t ms = 0;
  int32_t raw = 0;
  int16_t centi_c = 0;
  Status status = Status::kNoMeasurement;
};

inline constexpr size_t kFrameSize = 12;

inline void encode(const Frame& frame, uint8_t out[kFrameSize]) {
  const auto put = [out](size_t at, uint32_t v, size_t len) {
    for (size_t i = 0; i < len; ++i) out[at + i] = static_cast<uint8_t>(v >> (8 * i));
  };
  put(0, frame.seq, 2);
  put(2, frame.ms, 2);
  put(4, static_cast<uint32_t>(frame.raw), 4);
  put(8, static_cast<uint16_t>(frame.centi_c), 2);
  out[10] = static_cast<uint8_t>(frame.status);
  out[11] = 0;
}

}  // namespace xdb401

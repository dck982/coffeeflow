#pragma once

#include <cstddef>
#include <cstdint>

// Service BLE de la sonde SCACE : le Core2 est périphérique GATT et notifie
// une trame par mesure, à 10 Hz. Lu par l'écran et par
// tools/scace_ble_log.py, qui reprend ces UUID et ce format.

namespace common::scace {

inline constexpr char kDeviceName[] = "SCACE";
inline constexpr char kServiceUuid[] = "5017120a-7fa7-46d8-a22e-61b24b1201fe";
inline constexpr char kFrameUuid[] = "84fef574-1171-4c9a-b221-6f17aea79a38";

// Mêmes UUID, octets dans l'ordre de l'air (petit-boutiste), pour NimBLE.
inline constexpr uint8_t kServiceUuidLe[16] = {0xfe, 0x01, 0x12, 0x4b, 0xb2, 0x61, 0x2e, 0xa2,
                                               0xd8, 0x46, 0xa7, 0x7f, 0x0a, 0x12, 0x17, 0x50};
inline constexpr uint8_t kFrameUuidLe[16] = {0x38, 0x9a, 0xa7, 0xae, 0x17, 0x6f, 0x21, 0xb2,
                                             0x9a, 0x4c, 0x71, 0x11, 0x74, 0xf5, 0xfe, 0x84};

enum class Status : uint8_t {
  // Pont hors tension, ADS1115 introuvable ou erreur I2C. Envoyé une fois
  // par seconde, codes à 0.
  kNoMeasurement = 0,
  kOk = 1,
  kOpen = 2,
  kShorted = 3,
};

// Température absente (NTC ouverte, court-circuit, pas de mesure).
inline constexpr int16_t kNoTemperature = INT16_MIN;

// Trame de 12 octets, petit-boutiste. Elle tient dans le MTU par défaut de
// 23 octets (20 octets de valeur).
//   0  uint16  numéro de séquence, incrémenté à chaque trame
//   2  uint16  temps sonde en ms modulo 65 536, à la fin de la conversion A1
//   4  int16   code A0 (rail)
//   6  int16   code A1 (point milieu)
//   8  int16   température en centièmes de °C, constantes de la sonde
//  10  uint8   état
//  11  uint8   réservé, 0
struct Frame {
  uint16_t seq = 0;
  uint16_t ms = 0;
  int16_t a0 = 0;
  int16_t a1 = 0;
  int16_t centi_c = kNoTemperature;
  Status status = Status::kNoMeasurement;
};

inline constexpr size_t kFrameSize = 12;

inline void encode(const Frame& frame, uint8_t out[kFrameSize]) {
  const auto put16 = [out](size_t at, uint16_t v) {
    out[at] = static_cast<uint8_t>(v);
    out[at + 1] = static_cast<uint8_t>(v >> 8);
  };
  put16(0, frame.seq);
  put16(2, frame.ms);
  put16(4, static_cast<uint16_t>(frame.a0));
  put16(6, static_cast<uint16_t>(frame.a1));
  put16(8, static_cast<uint16_t>(frame.centi_c));
  out[10] = static_cast<uint8_t>(frame.status);
  out[11] = 0;
}

// Refuse une trame trop courte ou un état inconnu. Les octets au-delà de 12
// sont ignorés, pour qu'une trame allongée reste lisible.
inline bool decode(const uint8_t* data, size_t len, Frame* out) {
  if (len < kFrameSize || data[10] > static_cast<uint8_t>(Status::kShorted)) return false;
  const auto get16 = [data](size_t at) {
    return static_cast<uint16_t>(data[at] | (data[at + 1] << 8));
  };
  out->seq = get16(0);
  out->ms = get16(2);
  out->a0 = static_cast<int16_t>(get16(4));
  out->a1 = static_cast<int16_t>(get16(6));
  out->centi_c = static_cast<int16_t>(get16(8));
  out->status = static_cast<Status>(data[10]);
  return true;
}

}  // namespace common::scace

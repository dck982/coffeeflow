#pragma once

#include <array>
#include <cstddef>
#include <cstdint>

// Cadrage série (USB CDC) — voir docs/firmware.md et
// docs/firmware-implementation.md, phase 1. Le WebSocket transporte le PDU
// tel quel, un message = une trame ; seul le fil série a besoin d'un
// cadrage, parce qu'un flux d'octets n'a pas de bordure.
//
// PDU (identique sur les deux transports), little-endian :
//   [0..1]  id     uint16 — bits 10..0 = identifiant CAN, bits 15..11 = 0
//   [2]     dlc    0..8
//   [3..]   data   dlc octets
//   [fin]   crc16  uint16, CRC-16-CCITT (poly 0x1021, init 0xFFFF) sur id..data
//
// Sur le fil série : COBS(PDU) puis un octet 0x00 de fin de trame. COBS
// élimine tous les zéros du PDU, donc une image OTA pleine de zéros ne pose
// pas de problème de resynchronisation ; le premier 0x00 rencontré recadre
// après un câble mal branché ou un octet perdu.

namespace common {

constexpr size_t kMaxPduSize = 2 /*id*/ + 1 /*dlc*/ + 8 /*data*/ + 2 /*crc16*/;  // 13
constexpr size_t kMaxCobsSize = kMaxPduSize + (kMaxPduSize / 254) + 1;

struct RawFrame {
  uint16_t can_id = 0;  // 11 bits significatifs
  uint8_t dlc = 0;
  std::array<uint8_t, 8> data{};
};

// Sérialise `frame` en PDU (id, dlc, data, crc16) dans `out`. `out` doit
// pouvoir contenir kMaxPduSize octets. Ne valide pas `frame` : voir
// encode_framed pour la version qui refuse un gabarit hors norme.
size_t encode_pdu(const RawFrame& frame, uint8_t* out);

// Décode un PDU de longueur exacte `len`. Renvoie false si la longueur est
// incohérente avec le `dlc` qu'elle contient, si le CRC ne correspond pas,
// ou si l'identifiant dépasse 11 bits.
bool decode_pdu(const uint8_t* in, size_t len, RawFrame* out);

// COBS — encode `len` octets de `in` vers `out` (kMaxCobsSize au plus pour
// un PDU de notre taille). Renvoie la longueur écrite. N'ajoute pas le 0x00
// de fin de trame.
size_t cobs_encode(const uint8_t* in, size_t len, uint8_t* out);

// COBS inverse. `out` doit pouvoir contenir au moins `len` octets. Renvoie
// la longueur décodée, ou 0 si le buffer est malformé — aucun PDU valide
// n'a une longueur nulle (le plus court, STOP, fait 5 octets), donc 0 est
// un code d'erreur non ambigu.
size_t cobs_decode(const uint8_t* in, size_t len, uint8_t* out);

// Assemble le cadrage complet : COBS(PDU(frame)) + 0x00 dans `out`, qui doit
// pouvoir contenir kMaxCobsSize + 1 octets. Renvoie la longueur écrite, ou 0
// si `frame` est hors gabarit (can_id > 0x7FF ou dlc > 8) — rien n'est écrit
// dans ce cas.
size_t encode_framed(const RawFrame& frame, uint8_t* out);

// Décodeur incrémental pour un flux d'octets sans bordure (USB CDC). On lui
// pousse les octets reçus au fil de l'eau ; il rappelle `on_frame` pour
// chaque trame valide trouvée entre deux 0x00, et se contente d'incrémenter
// `dropped_count` pour tout ce qui ne décode pas — un octet perdu ne doit
// jamais bloquer le flux, seulement coûter la trame en cours.
class StreamDecoder {
 public:
  using FrameCallback = void (*)(const RawFrame& frame, void* ctx);

  void push_byte(uint8_t byte, FrameCallback on_frame, void* ctx);

  uint32_t dropped_count = 0;

 private:
  std::array<uint8_t, kMaxCobsSize> buffer_{};
  size_t len_ = 0;
};

}  // namespace common

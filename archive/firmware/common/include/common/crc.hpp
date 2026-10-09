#pragma once

#include <cstddef>
#include <cstdint>

namespace common {

// CRC16-CCITT (poly 0x1021, init 0xFFFF), utilisé pour l'acquittement de
// bloc du flash (FLASH_CTRL BLOCK_ACK, tous les 2 ko).
uint16_t crc16_ccitt(const uint8_t* data, size_t len);

// CRC32 (poly 0xEDB88320, init 0xFFFFFFFF, xorout 0xFFFFFFFF — variante
// zlib/IEEE 802.3), utilisé pour la vérification globale de l'image
// (FLASH_CTRL END).
uint32_t crc32_ieee(const uint8_t* data, size_t len);

// Même CRC32 que crc32_ieee(), calculé au fil de l'eau : utilisé côté
// récepteur du flash (FLASH_CTRL END) pour vérifier l'image sans la
// bufferiser entière en RAM.
class Crc32Incremental {
 public:
  void update(const uint8_t* data, size_t len);
  uint32_t finish() const { return crc_ ^ 0xFFFFFFFFu; }

 private:
  uint32_t crc_ = 0xFFFFFFFFu;
};

}  // namespace common

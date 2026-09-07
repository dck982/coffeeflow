#pragma once

#include <cstdint>

// Identifiant CAN 11 bits — voir docs/firmware.md, section "Protocole CAN".
//
//   bits 10..5   type   (64 valeurs) — valeur basse = priorité bus haute
//   bits  4..3   dest   (0 broadcast, 1 écran, 2 capteurs)
//   bits  2..0   src    (1 écran, 2 capteurs)
//
// Le type EST la priorité : pas de champ séparé.

namespace common {

enum class Node : uint8_t {
  kScreen = 1,
  kSensors = 2,
};

enum class Dest : uint8_t {
  kBroadcast = 0,
  kScreen = 1,
  kSensors = 2,
};

enum class MessageType : uint8_t {
  kStop = 0x00,
  kSet = 0x01,
  kReset = 0x02,
  kPing = 0x08,
  kPong = 0x09,
  kReqStatus = 0x10,
  kStatusPressure = 0x20,
  kStatusFlow = 0x21,
  kStatusActuators = 0x22,
  kLog = 0x30,
  kFlashCtrl = 0x38,
  kFlashData = 0x39,
};

// true si `type` est une valeur connue du protocole. Un ID reçu qui ne
// décode vers aucun MessageType valide doit être ignoré, pas planté dessus.
constexpr bool is_known_message_type(uint8_t type) {
  switch (static_cast<MessageType>(type)) {
    case MessageType::kStop:
    case MessageType::kSet:
    case MessageType::kReset:
    case MessageType::kPing:
    case MessageType::kPong:
    case MessageType::kReqStatus:
    case MessageType::kStatusPressure:
    case MessageType::kStatusFlow:
    case MessageType::kStatusActuators:
    case MessageType::kLog:
    case MessageType::kFlashCtrl:
    case MessageType::kFlashData:
      return true;
  }
  return false;
}

struct CanId {
  MessageType type;
  Dest dest;
  Node src;
};

// Encode un CanId en identifiant 11 bits (bits 10..0 significatifs).
constexpr uint16_t encode_can_id(const CanId& id) {
  const uint16_t type_bits = static_cast<uint16_t>(id.type) & 0x3F;
  const uint16_t dest_bits = static_cast<uint16_t>(id.dest) & 0x03;
  const uint16_t src_bits = static_cast<uint16_t>(id.src) & 0x07;
  return static_cast<uint16_t>((type_bits << 5) | (dest_bits << 3) | src_bits);
}

// Décode un identifiant 11 bits. Ne valide pas que `type` est connu :
// voir is_known_message_type.
constexpr CanId decode_can_id(uint16_t raw) {
  CanId id{};
  id.type = static_cast<MessageType>((raw >> 5) & 0x3F);
  id.dest = static_cast<Dest>((raw >> 3) & 0x03);
  id.src = static_cast<Node>(raw & 0x07);
  return id;
}

}  // namespace common

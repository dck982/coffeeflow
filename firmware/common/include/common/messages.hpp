#pragma once

#include <array>
#include <cstdint>
#include <cstring>

#include "common/protocol.hpp"

// Charges utiles du protocole — voir docs/firmware.md, section "Charges
// utiles". Chaque message se sérialise dans une trame CAN de 8 octets au
// plus ; STOP / RESET / PING sont vides (DLC 0).
//
// Convention : `pack()` remplit `out[8]` (les octets non utilisés sont mis
// à zéro) et renvoie le DLC réel. `unpack()` lit `in[len]` et renvoie false
// si `len` est trop court pour le message.

namespace common {

using Frame = std::array<uint8_t, 8>;

inline void put_u16(uint8_t* out, uint16_t v) {
  out[0] = static_cast<uint8_t>(v & 0xFF);
  out[1] = static_cast<uint8_t>((v >> 8) & 0xFF);
}
inline uint16_t get_u16(const uint8_t* in) {
  return static_cast<uint16_t>(in[0]) | (static_cast<uint16_t>(in[1]) << 8);
}
inline void put_u32(uint8_t* out, uint32_t v) {
  out[0] = static_cast<uint8_t>(v & 0xFF);
  out[1] = static_cast<uint8_t>((v >> 8) & 0xFF);
  out[2] = static_cast<uint8_t>((v >> 16) & 0xFF);
  out[3] = static_cast<uint8_t>((v >> 24) & 0xFF);
}
inline uint32_t get_u32(const uint8_t* in) {
  return static_cast<uint32_t>(in[0]) | (static_cast<uint32_t>(in[1]) << 8) |
         (static_cast<uint32_t>(in[2]) << 16) |
         (static_cast<uint32_t>(in[3]) << 24);
}

// SET (0x01) — écran → capteurs
struct SetPayload {
  bool set_ssr = false;
  bool set_dimmer = false;
  bool ssr = false;
  uint8_t dimmer = 0;      // 0..100
  uint16_t ttl_ms = 0;     // 0 = défaut (500 ms), voir firmware.md §1

  Frame pack() const {
    Frame f{};
    f[0] = static_cast<uint8_t>((set_ssr ? 0x01 : 0) | (set_dimmer ? 0x02 : 0));
    f[1] = ssr ? 1 : 0;
    f[2] = dimmer;
    put_u16(&f[3], ttl_ms);
    return f;
  }
  static bool unpack(const uint8_t* in, size_t len, SetPayload* out) {
    if (len < 5) return false;
    out->set_ssr = (in[0] & 0x01) != 0;
    out->set_dimmer = (in[0] & 0x02) != 0;
    out->ssr = in[1] != 0;
    out->dimmer = in[2];
    out->ttl_ms = get_u16(&in[3]);
    return true;
  }
};

// PONG (0x09) — dans les deux sens
struct PongPayload {
  Node node = Node::kSensors;
  uint8_t version_major = 0;
  uint8_t version_minor = 0;
  uint8_t version_patch = 0;
  uint32_t uptime_s = 0;

  Frame pack() const {
    Frame f{};
    f[0] = static_cast<uint8_t>(node);
    f[1] = version_major;
    f[2] = version_minor;
    f[3] = version_patch;
    put_u32(&f[4], uptime_s);
    return f;
  }
  static bool unpack(const uint8_t* in, size_t len, PongPayload* out) {
    if (len < 8) return false;
    out->node = static_cast<Node>(in[0]);
    out->version_major = in[1];
    out->version_minor = in[2];
    out->version_patch = in[3];
    out->uptime_s = get_u32(&in[4]);
    return true;
  }
};

// REQSTATUS (0x10) — écran → capteurs
struct ReqStatusPayload {
  MessageType target_type = MessageType::kStatusPressure;  // 0x20 | 0x21 | 0x22
  uint16_t period_ms = 0;  // 0 = arrêt

  Frame pack() const {
    Frame f{};
    f[0] = static_cast<uint8_t>(target_type);
    put_u16(&f[1], period_ms);
    return f;
  }
  static bool unpack(const uint8_t* in, size_t len, ReqStatusPayload* out) {
    if (len < 3) return false;
    out->target_type = static_cast<MessageType>(in[0]);
    out->period_ms = get_u16(&in[1]);
    return true;
  }
};

// STATUS_PRESSURE (0x20) — recopie du registre 0x06 du XDB401
struct StatusPressurePayload {
  uint32_t pressure_raw = 0;      // 24 bits utiles
  uint16_t temperature_raw = 0;   // 16 bits
  uint16_t timestamp_ms = 0;      // 16 bits bas
  // bit0 capteur valide (détection réelle : le XDB401 répond sur I2C),
  // bit1 timeout conversion. Voir la convention générale du bit0 dans
  // docs/firmware.md, "Charges utiles" — même bit, même sens, sur tous les
  // STATUS_* qui en ont un.
  uint8_t flags = 0;

  Frame pack() const {
    Frame f{};
    f[0] = static_cast<uint8_t>(pressure_raw & 0xFF);
    f[1] = static_cast<uint8_t>((pressure_raw >> 8) & 0xFF);
    f[2] = static_cast<uint8_t>((pressure_raw >> 16) & 0xFF);
    put_u16(&f[3], temperature_raw);
    put_u16(&f[5], timestamp_ms);
    f[7] = flags;
    return f;
  }
  static bool unpack(const uint8_t* in, size_t len, StatusPressurePayload* out) {
    if (len < 8) return false;
    out->pressure_raw = static_cast<uint32_t>(in[0]) |
                         (static_cast<uint32_t>(in[1]) << 8) |
                         (static_cast<uint32_t>(in[2]) << 16);
    out->temperature_raw = get_u16(&in[3]);
    out->timestamp_ms = get_u16(&in[5]);
    out->flags = in[7];
    return true;
  }
};

// STATUS_FLOW (0x21)
struct StatusFlowPayload {
  uint32_t pulse_count = 0;      // cumulé depuis reset
  uint16_t last_edge_ms = 0;     // 16 bits bas
  // bit0 capteur valide — toujours 1 ici : une simple entrée GPIO ne permet
  // pas de détecter l'absence du débitmètre (contrairement au XDB401 en
  // I2C), seule l'absence d'impulsions attendues le laisse deviner
  // (LOG FLOWMETER_SILENT). Même bit que StatusPressurePayload::flags, sens
  // identique, juste jamais mis à 0 sur ce message.
  uint8_t flags = 0;

  Frame pack() const {
    Frame f{};
    put_u32(&f[0], pulse_count);
    put_u16(&f[4], last_edge_ms);
    f[6] = flags;
    return f;
  }
  static bool unpack(const uint8_t* in, size_t len, StatusFlowPayload* out) {
    if (len < 7) return false;
    out->pulse_count = get_u32(&in[0]);
    out->last_edge_ms = get_u16(&in[4]);
    out->flags = in[6];
    return true;
  }
};

// STATUS_ACTUATORS (0x22) — accusé de réception d'un SET
struct StatusActuatorsPayload {
  bool ssr = false;
  uint8_t dimmer = 0;
  uint16_t lease_remaining_ms = 0;
  uint16_t continuous_on_ms = 0;
  // bit0 verrou actif, bit1 dimmer prêt, bit2 dimmer valide (détection I2C,
  // même sens que le bit0 des autres STATUS_* — pas le bit0 ici, la place
  // est prise par le verrou).
  uint8_t flags = 0;

  Frame pack() const {
    Frame f{};
    f[0] = ssr ? 1 : 0;
    f[1] = dimmer;
    put_u16(&f[2], lease_remaining_ms);
    put_u16(&f[4], continuous_on_ms);
    f[6] = flags;
    return f;
  }
  static bool unpack(const uint8_t* in, size_t len, StatusActuatorsPayload* out) {
    if (len < 7) return false;
    out->ssr = in[0] != 0;
    out->dimmer = in[1];
    out->lease_remaining_ms = get_u16(&in[2]);
    out->continuous_on_ms = get_u16(&in[4]);
    out->flags = in[6];
    return true;
  }
};

// LOG (0x30) — pas de texte sur le bus, voir log_codes.hpp (généré)
struct LogPayload {
  uint8_t code = 0;
  uint8_t severity = 0;
  uint16_t arg16 = 0;
  uint32_t arg32 = 0;

  Frame pack() const {
    Frame f{};
    f[0] = code;
    f[1] = severity;
    put_u16(&f[2], arg16);
    put_u32(&f[4], arg32);
    return f;
  }
  static bool unpack(const uint8_t* in, size_t len, LogPayload* out) {
    if (len < 8) return false;
    out->code = in[0];
    out->severity = in[1];
    out->arg16 = get_u16(&in[2]);
    out->arg32 = get_u32(&in[4]);
    return true;
  }
};

// FLASH_CTRL (0x38) — sous-commande. Le layout des paramètres n'est pas
// encore figé dans docs/firmware.md ; celui-ci est la première proposition,
// à valider en phase 4 (voir docs/firmware-implementation.md).
enum class FlashSubCmd : uint8_t {
  kBegin = 0,
  kBlockAck = 1,
  kEnd = 2,
  kAbort = 3,
};

struct FlashCtrlPayload {
  FlashSubCmd subcmd = FlashSubCmd::kAbort;
  uint32_t image_size = 0;   // BEGIN
  uint16_t block_number = 0; // BLOCK_ACK
  uint16_t block_crc16 = 0;  // BLOCK_ACK
  uint32_t image_crc32 = 0;  // END

  Frame pack() const {
    Frame f{};
    f[0] = static_cast<uint8_t>(subcmd);
    switch (subcmd) {
      case FlashSubCmd::kBegin:
        put_u32(&f[1], image_size);
        break;
      case FlashSubCmd::kBlockAck:
        put_u16(&f[1], block_number);
        put_u16(&f[3], block_crc16);
        break;
      case FlashSubCmd::kEnd:
        put_u32(&f[1], image_crc32);
        break;
      case FlashSubCmd::kAbort:
        break;
    }
    return f;
  }
  static bool unpack(const uint8_t* in, size_t len, FlashCtrlPayload* out) {
    if (len < 1) return false;
    out->subcmd = static_cast<FlashSubCmd>(in[0]);
    switch (out->subcmd) {
      case FlashSubCmd::kBegin:
        if (len < 5) return false;
        out->image_size = get_u32(&in[1]);
        return true;
      case FlashSubCmd::kBlockAck:
        if (len < 5) return false;
        out->block_number = get_u16(&in[1]);
        out->block_crc16 = get_u16(&in[3]);
        return true;
      case FlashSubCmd::kEnd:
        if (len < 5) return false;
        out->image_crc32 = get_u32(&in[1]);
        return true;
      case FlashSubCmd::kAbort:
        return true;
    }
    return false;
  }
};

// FLASH_DATA (0x39) — 8 octets bruts, aucun en-tête. Rien à empaqueter :
// la trame CAN elle-même EST la charge utile.

}  // namespace common

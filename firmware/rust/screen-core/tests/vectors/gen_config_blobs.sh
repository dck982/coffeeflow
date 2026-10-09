#!/bin/sh
# Régénère config_blobs.txt : les blobs NVS (schémas 1 à 10) tels que le
# compilateur C les dispose, avec les structures recopiées sans changement de
# archive/firmware/screen/main/core/config.cpp.
#
# Format d'une ligne : `v<schéma> <emplacement> <octets en hexadécimal>`.
# Les valeurs d'essai sont documentées dans tests/config.rs.
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../../../.." && pwd)"
SRC="$ROOT/archive/firmware/screen/main"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

python3 - "$SRC/core/config.cpp" "$WORK/structs.inc" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
a = text.index("struct StoredConfig {")
b = text.index("portMUX_TYPE g_lock")
open(sys.argv[2], "w", encoding="utf-8").write(text[a:b])
PY

cat > "$WORK/harness.cpp" <<'CPP'
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>

#include "core/config.h"

namespace core {
namespace {
constexpr uint32_t kMagic = 0x43464731;

#include "structs.inc"

uint32_t checksum_bytes(const void* value, size_t size) {
  const auto* bytes = static_cast<const uint8_t*>(value);
  uint32_t result = 2166136261u;
  for (size_t i = 0; i < size; ++i) result = (result ^ bytes[i]) * 16777619u;
  return result;
}

void print(const char* version, const char* slot, const void* blob, size_t size) {
  std::printf("%s %s ", version, slot);
  const auto* bytes = static_cast<const uint8_t*>(blob);
  for (size_t i = 0; i < size; ++i) std::printf("%02x", bytes[i]);
  std::printf("\n");
}

// Remplit les champs communs avec les valeurs d'essai ; `second` change le
// temps cible pour distinguer les deux emplacements.
template <typename C>
void common(C& c, uint32_t revision, bool second) {
  c.revision = revision;
  c.target_weight_g = 40.5f;
  c.target_time_s = second ? 44 : 33;
  c.rampdown_mode = RampdownMode::kWeight;
  c.rampdown_lead_time_s = 3.5f;
  c.rampdown_lead_weight_g = 2.5f;
  c.rampdown_pressure_drop_bar = 1.5f;
  c.brew_pump_pct = 90;
  c.purge_pump_pct = 70;
  c.purge_max_s = 25;
  c.dim_after_s = 180;
  c.standby_after_s = 900;
  c.preinfusion_time_s = 5;
  c.preinfusion_pump_pct = 45;
}

template <typename S>
void emit(const char* version, const char* slot, S& stored) {
  stored.checksum = checksum_bytes(&stored, offsetof(S, checksum));
  print(version, slot, &stored, sizeof(stored));
}

template <typename S, typename F>
void both(const char* version, uint16_t schema, F fill) {
  S a;
  std::memset(&a, 0, sizeof(a));
  a.magic = kMagic;
  a.config.version = schema;
  fill(a.config, 5, false);
  emit(version, "a", a);
  S b;
  std::memset(&b, 0, sizeof(b));
  b.magic = kMagic;
  b.config.version = schema;
  fill(b.config, 6, true);
  emit(version, "b", b);
}
}  // namespace
}  // namespace core

int main() {
  using namespace core;
  // Schéma 10 : défauts (révision 3) et configuration modifiée (révision 4).
  {
    StoredConfig a;
    std::memset(&a, 0, sizeof(a));
    a.magic = kMagic;
    a.config = Config{};
    a.config.revision = 3;
    emit("v10", "a", a);
    StoredConfig b;
    std::memset(&b, 0, sizeof(b));
    b.magic = kMagic;
    b.config = Config{};
    b.config.revision = 4;
    b.config.target_weight_g = 41.5f;
    b.config.target_time_s = 31;
    b.config.heating_enabled = false;
    b.config.preinfusion_mode = PreinfusionMode::kTime | PreinfusionMode::kWeight;
    b.config.rampdown_mode = RampdownMode::kPressureDrop;
    b.config.scace_enabled = true;
    emit("v10", "b", b);
  }
  both<StoredConfigV9>("v9", 9, [](ConfigV9& c, uint32_t r, bool s) {
    common(c, r, s);
    c.target_pressure_bar = 8.5f;
    c.brew_temperature_c = 88.5f;
    c.heating_enabled = true;
    c.brew_preheat_time_s = 4.5f;
    c.filling_time_s = 7;
    c.filling_pressure_bar = 1.5f;
    c.filling_pump_pct = 80;
    c.preinfusion_mode = PreinfusionMode::kTime | PreinfusionMode::kWeight;
  });
  both<StoredConfigV8>("v8", 8, [](ConfigV8& c, uint32_t r, bool s) {
    common(c, r, s);
    c.target_pressure_bar = 8.5f;
    c.brew_temperature_c = 88.5f;
    c.heating_enabled = true;
    c.brew_preheat_time_s = 4.5f;
    c.filling_time_s = 7;
    c.filling_pressure_rise_bar = 0.7f;
    c.filling_pump_pct = 80;
    c.preinfusion_mode = PreinfusionMode::kWeight;
  });
  both<StoredConfigV7>("v7", 7, [](ConfigV7& c, uint32_t r, bool s) {
    common(c, r, s);
    c.target_pressure_bar = 8.5f;
    c.brew_temperature_c = 88.5f;
    c.heating_enabled = true;
    c.brew_preheat_time_s = 4.5f;
    c.filling_time_s = 7;
    c.filling_pressure_target_bar = 1.2f;
    c.filling_pump_pct = 80;
    c.preinfusion_pressure_bar = 6.0f;
    c.preinfusion_mode = static_cast<PreinfusionMode>(0x02);  // pression seule
  });
  both<StoredConfigV6>("v6", 6, [](ConfigV6& c, uint32_t r, bool s) {
    common(c, r, s);
    c.target_pressure_bar = 8.5f;
    c.brew_temperature_c = 88.5f;
    c.heating_enabled = true;
    c.filling_time_s = 7;
    c.filling_pressure_target_bar = 1.2f;
    c.filling_pump_pct = 80;
    c.preinfusion_pressure_bar = 6.0f;
    c.preinfusion_mode = static_cast<PreinfusionMode>(0x03);  // temps + pression
  });
  both<StoredConfigV5>("v5", 5, [](ConfigV5& c, uint32_t r, bool s) {
    common(c, r, s);
    c.target_pressure_bar = 8.5f;
    c.filling_time_s = 7;
    c.filling_pressure_target_bar = 1.2f;
    c.filling_pump_pct = 80;
    c.preinfusion_pressure_bar = 6.0f;
    c.preinfusion_mode = static_cast<PreinfusionMode>(0x06);  // pression + poids
  });
  both<StoredConfigV4>("v4", 4, [](ConfigV4& c, uint32_t r, bool s) {
    common(c, r, s);
    c.filling_time_s = 7;
    c.filling_pressure_target_bar = 1.2f;
    c.filling_pump_pct = 80;
    c.preinfusion_pressure_bar = 6.0f;
    c.preinfusion_mode = static_cast<PreinfusionMode>(0x02);
  });
  both<StoredConfigV3>("v3", 3, [](ConfigV3& c, uint32_t r, bool s) {
    common(c, r, s);
    c.filling_time_s = 7;
    c.filling_pressure_delta_bar = 0.4f;
    c.filling_pump_pct = 80;
    c.preinfusion_pressure_bar = 6.0f;
    c.preinfusion_mode = static_cast<PreinfusionMode>(0x04);  // poids seul
  });
  both<StoredConfigV2>("v2", 2, [](ConfigV2& c, uint32_t r, bool s) {
    common(c, r, s);
    c.preinfusion_pressure_bar = 6.0f;
    c.preinfusion_mode = static_cast<PreinfusionMode>(0x02);
  });
  both<StoredConfigV1>("v1", 1, [](ConfigV1& c, uint32_t r, bool s) {
    common(c, r, s);
    c.preinfusion_pressure_bar = 6.0f;
    c.preinfusion_mode = PreinfusionModeV1::kPressure;
  });
  std::fprintf(stderr, "tailles: v10=%zu v9=%zu v8=%zu v7=%zu v6=%zu v5=%zu v4=%zu v3=%zu v2=%zu v1=%zu\n",
               sizeof(StoredConfig), sizeof(StoredConfigV9), sizeof(StoredConfigV8),
               sizeof(StoredConfigV7), sizeof(StoredConfigV6), sizeof(StoredConfigV5),
               sizeof(StoredConfigV4), sizeof(StoredConfigV3), sizeof(StoredConfigV2),
               sizeof(StoredConfigV1));
  return 0;
}
CPP

c++ -std=c++17 -Wall -I "$SRC" -I "$WORK" "$WORK/harness.cpp" -o "$WORK/harness"
"$WORK/harness" > "$HERE/config_blobs.txt"
echo "écrit $HERE/config_blobs.txt"

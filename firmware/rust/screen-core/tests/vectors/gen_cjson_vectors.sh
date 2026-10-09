#!/bin/sh
# Régénère cjson_vectors.txt : les documents JSON que le firmware C++ archivé
# produit pour quelques états, écrits par le vrai cJSON.
#
# Le harnais recopie sans changement `encode_config`, `encode_telemetry` et
# leurs aides depuis archive/firmware/screen/main/net_http.cpp (seules les
# lignes qui touchent ESP-IDF sont remplacées par des paramètres), puis les
# compile avec cJSON et les en-têtes purs du cœur.
#
# Usage : gen_cjson_vectors.sh CHEMIN/VERS/cJSON    (dossier avec cJSON.c)
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../../../.." && pwd)"
CJSON="${1:?dossier cJSON (avec cJSON.c et cJSON.h)}"
SRC="$ROOT/archive/firmware/screen/main"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

python3 - "$SRC/net_http.cpp" "$WORK/extract.inc" <<'PY'
import re, sys
lines = open(sys.argv[1], encoding="utf-8").read().split("\n")
def find(prefix, start=0):
    for i in range(start, len(lines)):
        if lines[i].startswith(prefix):
            return i
    raise SystemExit("introuvable: " + prefix)
a = find("const char* freshness_text")
b = find("const char* ota_state_text")
c = find("const char* network_text")
d = find("bool is_integer_number")
text = "\n".join(lines[a:b] + lines[c:d])
text = text.replace("static_cast<double>(esp_timer_get_time() / 1000)", "static_cast<double>(g_uptime_us / 1000)")
text = text.replace("const core::Config config = core::get_config();", "const core::Config config = core::Config{};")
text = text.replace("ota_local::pending_verify()", "g_pending_verify")
text = text.replace("static_cast<double>(std::time(nullptr))", "static_cast<double>(g_wall)")
# Bloc des partitions OTA : remplacé par des paramètres.
start = text.index("  const esp_partition_t* staging")
end = text.index("  cJSON_AddBoolToObject(brew, \"valve_open\"")
text = text[:start] + """  cJSON* staging_info = cJSON_AddObjectToObject(firmware, "ota_staging");
  cJSON_AddBoolToObject(staging_info, "available", g_staging_size != 0);
  if (g_staging_size != 0) cJSON_AddNumberToObject(staging_info, "size", g_staging_size);
  if (g_running_label != nullptr) {
    cJSON_AddStringToObject(screen_ota, "running_partition", g_running_label);
    cJSON_AddStringToObject(screen_ota, "state", g_running_state);
  }
""" + text[end:]
open(sys.argv[2], "w", encoding="utf-8").write(text)
PY

cat > "$WORK/harness.cpp" <<'CPP'
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <ctime>
#include <limits>

#include "cJSON.h"
#include "core/core.h"

namespace {
int64_t g_uptime_us = 0;
bool g_pending_verify = false;
uint32_t g_staging_size = 0;
const char* g_running_label = nullptr;
const char* g_running_state = "";
int64_t g_wall = 0;

#include "extract.inc"

void emit(const char* name, cJSON* object) {
  char* printed = cJSON_PrintUnformatted(object);
  std::printf("%s\t%s\n", name, printed);
  cJSON_free(printed);
  cJSON_Delete(object);
}

void number(float value) {
  cJSON* object = cJSON_CreateObject();
  cJSON_AddNumberToObject(object, "v", value);
  char* printed = cJSON_PrintUnformatted(object);
  std::printf("number\t%.9g\t%s\n", static_cast<double>(value), printed);
  cJSON_free(printed);
  cJSON_Delete(object);
}
}  // namespace

int main() {
  emit("config_default", encode_config(core::Config{}));
  core::Config custom{};
  custom.target_weight_g = 36.5f;
  custom.target_time_s = 30;
  custom.target_pressure_bar = 9.1f;
  custom.brew_temperature_c = 92.5f;
  custom.heating_enabled = false;
  custom.brew_preheat_time_s = 7.5f;
  custom.filling_time_s = 12;
  custom.filling_pressure_bar = 0.3f;
  custom.filling_pump_pct = 75;
  custom.preinfusion_mode = core::PreinfusionMode::kTime | core::PreinfusionMode::kWeight;
  custom.preinfusion_time_s = 6;
  custom.preinfusion_pump_pct = 40;
  custom.rampdown_mode = core::RampdownMode::kPressureDrop;
  custom.rampdown_lead_time_s = 4.5f;
  custom.rampdown_lead_weight_g = 2.5f;
  custom.rampdown_pressure_drop_bar = 1.5f;
  custom.brew_pump_pct = 85;
  custom.purge_pump_pct = 60;
  custom.purge_max_s = 30;
  custom.dim_after_s = 120;
  custom.standby_after_s = 900;
  custom.scace_enabled = true;
  emit("config_custom", encode_config(custom));

  g_uptime_us = 5000 * 1000LL;
  emit("telemetry_default", encode_telemetry(core::Snapshot{}));

  core::Snapshot s{};
  s.pressure_bar = 8.7345f;
  s.boiler_temperature_c = 93.456f;
  s.boiler_sensor_temperature_c = 103.956f;
  s.boiler_ntc_a0_raw = 20000;
  s.boiler_ntc_a1_raw = 9000;
  s.pressure_a2_raw = 6000;
  s.pressure_a2_valid = true;
  s.boiler_temperature_valid = true;
  s.boiler_temperature_freshness = core::Freshness::kFresh;
  s.boiler_temperature_age_ms = 120;
  s.flow_ml_s = 1.2345f;
  s.volume_ml = 17.3f;
  s.flow_pulse_count = 3000000000u;
  s.flow_last_edge_age_ms = 40;
  s.weight_g = -0.05f;
  s.scale_connected = true;
  s.scale_present = true;
  s.scale_age_ms = 90;
  s.sensors_alive = true;
  s.pressure_valid = true;
  s.flow_valid = true;
  s.valve_open = true;
  s.dimmer_pct = 70;
  s.pump_pct = 70;
  s.heating_capable = true;
  s.heating_power_capable = true;
  s.heating_window_restart_capable = true;
  s.heater_on = true;
  s.heating_power_pct = 35.5f;
  s.heating_power_accepted_pct = 35.4f;
  s.brew_temperature_ready = true;
  s.heating_requested = false;
  s.heating_lease_remaining_ms = 1200;
  s.heating_freshness = core::Freshness::kFresh;
  s.heating_age_ms = 80;
  s.dimmer_ready = true;
  s.dimmer_valid = true;
  s.lease_remaining_ms = 450;
  s.continuous_on_ms = 12000;
  s.pressure_freshness = core::Freshness::kStale;
  s.flow_freshness = core::Freshness::kFresh;
  s.actuators_freshness = core::Freshness::kFresh;
  s.pressure_age_ms = 210;
  s.flow_age_ms = 60;
  s.actuators_age_ms = 100;
  s.sensors_version_major = 0;
  s.sensors_version_minor = 4;
  s.sensors_version_patch = 0;
  s.sensors_uptime_s = 86400;
  s.sensors_twai_rx_errors = 3;
  s.sensors_twai_tx_errors = 1;
  s.sensors_twai_bus_errors = 7;
  s.touch_ready = true;
  s.touch_press_count = 42;
  s.network_state = static_cast<uint8_t>(core::NetworkState::kStaConnected);
  s.ipv4_address = 0x0100A8C0u;  // 192.168.0.1
  s.radio_mode = core::RadioMode::kWifi;
  s.time_known = true;
  s.pressure_raw = 123456;
  s.xdb401_temperature_raw = 777;
  s.screen_version_major = 0;
  s.screen_version_minor = 4;
  s.screen_version_patch = 0;
  s.screen_uptime_s = 3600;
  s.flash_active = true;
  s.flash_target = core::FlashTarget::kSensors;
  s.flash_bytes_done = 2048;
  s.flash_bytes_total = 400000;
  s.cycle_state = core::CycleState::kBrew;
  s.cycle_elapsed_ms = 15500;
  s.cycle_weight_goal = true;
  s.capture_cooldown = true;
  s.last_shot_available = true;
  s.last_shot_weight_g = 36.2f;
  s.last_shot_duration_ms = 28100;
  s.last_shot_flow_ml_s = 1.05f;
  s.last_shot_unix_s = 1790000000;
  g_pending_verify = true;
  g_staging_size = 1048576;
  g_running_label = "ota_0";
  g_running_state = "pending_verify";
  g_wall = 1790001234;
  emit("telemetry_busy", encode_telemetry(s));

  core::Snapshot t{};
  t.last_shot_available = true;
  t.last_shot_weight_g = 18.0f;
  t.last_shot_duration_ms = 25000;
  t.last_shot_flow_ml_s = 0.0f;
  t.last_shot_unix_s = 0;
  t.heating_freshness = core::Freshness::kStale;
  t.heating_power_accepted_pct = 12.0f;
  t.pressure_a2_valid = false;
  t.cycle_state = core::CycleState::kFinished;
  t.flash_target = core::FlashTarget::kScreen;
  t.radio_mode = core::RadioMode::kOff;
  g_pending_verify = false;
  g_staging_size = 0;
  g_running_label = "ota_1";
  g_running_state = "unavailable";
  emit("telemetry_idle_variant", encode_telemetry(t));

  const float values[] = {0.0f, -0.0f, 1.0f, -1.0f, 0.1f, 0.5f, 9.0f, 8.7345f, 93.456f, 103.956f,
                          1.2345f, 17.3f, 35.5f, 36.2f, 1.05f, 0.05f, 100.5f, 90.0f, 12.0f,
                          3.0e9f, 4294967040.0f, 1.0e-5f, 123456.789f, 1.0e10f, 2147483648.0f,
                          -2147483648.0f, 0.3f, 7.0e-7f, 16777216.0f, 33.333332f};
  for (float v : values) number(v);
  return 0;
}
CPP

cc -c -o "$WORK/cJSON.o" -I "$CJSON" "$CJSON/cJSON.c"
c++ -std=c++17 -Wall -Wextra -I "$CJSON" -I "$SRC" -I "$WORK" "$WORK/harness.cpp" "$WORK/cJSON.o" -o "$WORK/harness"
"$WORK/harness" > "$HERE/cjson_vectors.txt"
echo "écrit $HERE/cjson_vectors.txt"

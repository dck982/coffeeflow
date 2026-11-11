#!/bin/sh
# Régénère cjson_vectors.txt : les documents JSON que le firmware C++ archivé
# produit pour quelques états, écrits par le vrai cJSON.
#
# Seuls `/config` et l'impression des nombres restent comparés : `/telemetry`
# a quitté cette comparaison (objet `pump`, clés `dimmer_*` retirées) et sa
# référence est `golden_json.rs`.
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

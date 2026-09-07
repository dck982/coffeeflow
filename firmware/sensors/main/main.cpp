// Module capteurs (XIAO ESP32-S3) — squelette phase 0.
//
// Rien d'autre ici tant que la phase 1 (outil Mac) et la phase 2
// (capteurs factory) ne sont pas commencées : voir
// docs/firmware-implementation.md.

#include "esp_log.h"

extern "C" void app_main() {
  ESP_LOGI("sensors", "phase 0 : socle uniquement, aucune logique embarquée");
}

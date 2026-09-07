// Module écran (Waveshare ESP32-S3-Touch-LCD-4.3) — squelette phase 0.
//
// Rien d'autre ici tant que la phase 1 (outil Mac) et la phase 3
// (écran factory, pont USB <-> CAN) ne sont pas commencées : voir
// docs/firmware-implementation.md.

#include "esp_log.h"

extern "C" void app_main() {
  ESP_LOGI("screen", "phase 0 : socle uniquement, aucune logique embarquée");
}

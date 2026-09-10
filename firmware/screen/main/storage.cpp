#include "storage.h"

#include "esp_err.h"
#include "nvs_flash.h"

namespace storage {
void init() {
  // Ne jamais effacer automatiquement : les réglages/calibrations sont des
  // données de machine. Une NVS incompatible doit être diagnostiquée, pas
  // silencieusement remplacée par des défauts.
  ESP_ERROR_CHECK(nvs_flash_init());
}
}  // namespace storage

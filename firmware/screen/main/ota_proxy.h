// Flash des capteurs à travers le CAN — écran émetteur de la séquence
// BEGIN / blocs FLASH_DATA acquittés / END sur le bus, portage côté
// firmware de firmware/tools/flash_client.py. N'existe pas encore dans le
// pont actuel (main.cpp de la phase 3 ne fait que son propre OTA local,
// voir ota_local.h) : ce fichier est un stub posé par docs/plan-phase6.md
// lot 1, rempli au lot 7.
#pragma once

#include <cstddef>
#include <cstdint>

namespace ota_proxy {

bool begin_upload(uint32_t image_size);
bool write_upload(const uint8_t* data, size_t len);
bool commit_upload();
void abort_upload();
bool active();
void on_flash_ctrl_received(const uint8_t* data, size_t len);

}  // namespace ota_proxy

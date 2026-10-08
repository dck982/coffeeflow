// Client BLE GATT Acaia Lunar (lot 8). Le protocole est documenté sous
// docs/reference/acaia-ble/; cette interface cache NimBLE au reste du projet.
// Le scan et la pile NimBLE appartiennent à ble_central.
#pragma once

#include <cstdint>

#include "host/ble_gap.h"
#include "host/ble_hs_adv.h"

namespace ble_scale {

// Publie chaque pesée pendant un cycle plutôt qu'une valeur par seconde.
void set_active(bool active);

// Commande Acaia TARE. False signifie qu'aucune caractéristique écrivable
// n'est actuellement prête.
bool tare();

// Interface réservée à ble_central.
bool matches(const ble_hs_adv_fields& fields);
bool connected();
int gap_event(ble_gap_event* event, void* arg);
void tick(int64_t now_us);  // heartbeat Acaia
void reset();               // pile NimBLE réinitialisée ou arrêtée

}  // namespace ble_scale

// Client BLE de la sonde SCACE : température de l'eau dans le panier, à
// 10 Hz. Seule la température et l'état de la sonde sont publiés au coeur.
// Le scan et la pile NimBLE appartiennent à ble_central.
#pragma once

#include "host/ble_gap.h"
#include "host/ble_hs_adv.h"

namespace ble_scace {

// Interface réservée à ble_central.
bool matches(const ble_hs_adv_fields& fields);
bool connected();
int gap_event(ble_gap_event* event, void* arg);
void disconnect();  // scace.enabled vient de passer à false
void reset();       // pile NimBLE réinitialisée ou arrêtée

}  // namespace ble_scace

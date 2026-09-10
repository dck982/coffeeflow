// Client BLE GATT Acaia Lunar (lot 8). Le protocole est documenté sous
// docs/reference/acaia-ble/; cette interface cache NimBLE au reste du projet.
#pragma once

namespace ble_scale {

void init();
void stop();

// Le lot 9 appellera cette fonction au départ et à la fin d'un cycle pour
// publier chaque pesée pendant un shot plutôt qu'une valeur par seconde.
void set_active(bool active);

// Commande Acaia TARE. False signifie qu'aucune caractéristique écrivable
// n'est actuellement prête.
bool tare();

}  // namespace ble_scale

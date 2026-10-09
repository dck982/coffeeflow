// Rôle central BLE de l'écran : pile NimBLE, scan et connexions. Il sert deux
// périphériques, la balance Acaia (ble_scale) et la sonde SCACE (ble_scace),
// chacun avec sa propre connexion et ses propres événements GAP.
#pragma once

namespace ble_central {

void init();
void stop();

// Infusion ou purge en cours : la balance publie chaque pesée et la sonde
// seule ne déclenche plus de scan.
void set_active(bool active);

// Appelé par un client sur l'événement BLE_GAP_EVENT_CONNECT de sa connexion,
// réussie ou non : le scan peut reprendre pour l'autre périphérique.
void connection_attempt_finished();

}  // namespace ble_central

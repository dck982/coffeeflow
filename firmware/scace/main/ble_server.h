// Serveur GATT de la sonde : une caractéristique en lecture et notification,
// une trame common::scace par mesure. Deux centraux à la fois, pour que le
// Mac puisse écouter pendant que l'écran est connecté.
#pragma once

#include "common/scace_ble.hpp"

namespace ble_server {

// Initialise NVS et NimBLE, puis annonce le service. Appelé une fois.
void start();

// Notifie la trame aux centraux abonnés et la garde pour les lectures.
// Appelée depuis la boucle principale.
void publish(const common::scace::Frame& frame);

// Nombre de centraux connectés, pour l'écran.
int connections();

}  // namespace ble_server

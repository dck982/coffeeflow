// Serveur GATT de la sonde : une caractéristique en lecture et notification,
// une trame xdb401 par mesure. Deux centraux à la fois.
#pragma once

#include "xdb401_ble.hpp"

namespace ble_server {

// Initialise NVS et NimBLE, puis annonce le service. Appelé une fois.
void start();

// Notifie la trame aux centraux abonnés et la garde pour les lectures.
// Appelée depuis la boucle principale.
void publish(const xdb401::Frame& frame);

// Nombre de centraux connectés, pour l'écran.
int connections();

}  // namespace ble_server

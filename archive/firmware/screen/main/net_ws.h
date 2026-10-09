// Miroir WebSocket du trafic CAN. Le message WebSocket est le PDU nu défini
// dans common/framing.hpp : aucun COBS, un message = une trame.
#pragma once

#include "esp_http_server.h"

#include "common/framing.hpp"

namespace net_ws {

using Authorize = bool (*)(httpd_req_t* request);

// Enregistre /ws sur le serveur HTTP déjà démarré. `authorize` est aussi
// appelée pendant le handshake, avant que la connexion ne soit promue en WS.
void start(httpd_handle_t server, Authorize authorize);

// Invalide les clients avant l'arrêt du httpd.
void stop();

// Publication best-effort depuis les tâches CAN : elle ne fait aucune E/S
// réseau et ne bloque jamais la tâche appelante.
void publish(const common::RawFrame& frame);

}  // namespace net_ws

// TWAI : émission/réception, dispatch protocolaire, présence — voir
// docs/firmware.md §2 et docs/plan-phase6.md lot 1.
#pragma once

#include <cstdint>

#include "driver/twai.h"

#include "common/protocol.hpp"
#include "log_codes.hpp"

namespace can_link {

// Installe et démarre le driver TWAI (board::kCanTx/kCanRx, 500 kbit/s,
// aucun filtre) et amorce le compteur de présence.
void init();

// Transmet sur le bus ET mirroir sur l'UART du pont : TWAI ne boucle pas nos
// propres trames, donc sans ce mirroir coffeetool ne verrait jamais un
// PONG/LOG que nous générons nous-mêmes.
void send_message(common::MessageType type, common::Dest dest, const uint8_t* data, uint8_t dlc);

void send_log(common::LogCode code, common::LogSeverity severity, uint16_t arg16 = 0, uint32_t arg32 = 0);

void send_pong();

// Redémarre le module capteurs par le protocole CAN. Cette commande n'est
// exposée au reste de l'écran que via le cœur, qui applique les interverrouillages.
void reset_sensors();

// Toute trame valide du pair maintient la présence. Après 1,5 s de silence,
// trois PING sont envoyés à 500 ms d'intervalle avant de déclarer la perte.
// La validation OTA reste plus stricte et requiert un PONG.
bool presence_lost();
bool peer_roundtrip_confirmed();

// Appelé par la tâche télémétrie du cœur 0, jamais par LVGL.
void tick_presence();

// Notre propre rôle de nœud kScreen : ne réagit qu'à ce qui vient des
// capteurs et nous est adressé ou en broadcast. Tout le reste (y compris ce
// qui vient des capteurs mais ne nous concerne pas) est déjà passé sur
// l'USB par le pont série, sans repasser ici.
void dispatch_own_protocol(const twai_message_t& msg);

}  // namespace can_link

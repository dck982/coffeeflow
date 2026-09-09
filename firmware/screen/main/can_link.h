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

// Présence — voir docs/firmware.md §2 : un PING ou un PONG reçu du pair
// suffit, c'est le nœud qui répond qui doit remettre son propre compteur à
// zéro (TWAI ne boucle pas ses propres trames). Réutilisée aussi comme
// preuve de vie pour la validation OTA de l'écran lui-même (ota_local.cpp).
bool presence_lost();

// À appeler périodiquement (service_screen, ~200 ms) : détecte le timeout de
// présence (3 s, même valeur que sensors/main.cpp) et publie un événement du
// cœur (core::EventKind::kCanPresenceLost) à la transition. La transition
// inverse (retrouvée) est publiée directement par mark_presence() dès qu'un
// PING/PONG du pair revient — voir can_link.cpp.
void tick_presence();

// Notre propre rôle de nœud kScreen : ne réagit qu'à ce qui vient des
// capteurs et nous est adressé ou en broadcast. Tout le reste (y compris ce
// qui vient des capteurs mais ne nous concerne pas) est déjà passé sur
// l'USB par le pont série, sans repasser ici.
void dispatch_own_protocol(const twai_message_t& msg);

}  // namespace can_link

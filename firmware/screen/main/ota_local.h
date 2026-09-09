// OTA de l'écran lui-même — voir docs/firmware-implementation.md, phase 4
// point 3 côté écran : Mac -> USB -> screen directement, sans passer par le
// CAN (screen étant à la fois pont et destinataire). Code existant déplacé
// tel quel, voir docs/plan-phase6.md lot 1.
#pragma once

#include <cstddef>
#include <cstdint>

namespace ota_local {

// Lit l'état OTA au boot (PENDING_VERIFY ou non) et amorce le temporisateur
// d'invalidation le cas échéant. À appeler une fois, tôt dans app_main.
void init_pending_verify();

bool pending_verify();

// Crée la tâche de validation (PING/PONG confirmé ou rollback au bout de
// 30 s), épinglée sur le cœur 0 avec le pont TWAI/UART.
void start_validation_task();

void on_flash_ctrl_received(const uint8_t* data, size_t len);
void on_flash_data_received(const uint8_t* data, size_t len);

}  // namespace ota_local

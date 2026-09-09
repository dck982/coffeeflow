// Pont série (UART2 réaffecté) <-> CAN — voir docs/firmware-implementation.md
// et docs/firmware.md. Code existant déplacé tel quel depuis main.cpp
// (phase 3), voir docs/plan-phase6.md lot 1.
#pragma once

#include <cstddef>
#include <cstdint>

namespace serial_bridge {

// Installe le driver UART2, corrige le pad GPIO44 (régression IDF v5->v6.1,
// voir le commentaire en tête de serial_bridge.cpp) et prépare la lecture.
void init();

// Écrit des octets bruts sur l'UART du pont — utilisé par can_link (mirroir
// des trames émises) et ota_local (accusés de flash) en plus des deux tâches
// ci-dessous.
void write_raw(const uint8_t* data, size_t len);

// Crée les deux tâches du pont (CAN -> série, série -> CAN), épinglées sur
// le cœur 0 (TWAI/UART/pont ; le cœur 1 est réservé au LCD/LVGL, voir
// docs/screen-issue.md).
void start_tasks();

}  // namespace serial_bridge

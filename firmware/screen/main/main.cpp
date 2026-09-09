// Module écran (Waveshare ESP32-S3-Touch-LCD-4.3) — phase 3 : pont USB <->
// CAN, voir docs/firmware-implementation.md et docs/firmware.md.
//
// Deux rôles à la fois, comme prévu par la doc :
//   - Pont transparent, même cadrage que firmware/can-monitor (COBS + PDU +
//     CRC16, common/framing.hpp) : toute trame CAN reçue est réémise telle
//     quelle sur l'USB, tout PDU décodé depuis l'USB est réémis tel quel sur
//     le bus. coffeetool s'en sert exactement comme de can-monitor.
//   - Nœud kScreen à part entière (contrairement à can-monitor qui n'a
//     aucune identité) : répond aux PING venus des capteurs, se reboote sur
//     RESET, LOG au boot. Nécessaire pour que la présence côté capteurs
//     (tick_presence(), sensors/main.cpp) ait un vrai répondant sur le bus.
//
// Le pont passe par UART_NUM_2 réaffecté sur GPIO43(TX)/44(RX) via la
// matrice GPIO, pas l'USB natif de l'ESP32-S3 : sur ce banc, le câble USB-C
// est branché sur le port "UART" de la carte (bridge WCH CH343P externe).
//
// app_main() ne fait qu'initialiser le matériel et créer les tâches — voir
// board.h, can_link.h, serial_bridge.h et ota_local.h pour le détail de
// chaque rôle (docs/plan-phase6.md, lot 1 : restructuration sans changement
// de comportement).
//
// Ni LVGL, ni Wi-Fi, ni BLE, ni logique d'infusion : voir docs/firmware.md,
// "Le Waveshare reste atteignable en USB-C... image factory minuscule".

#include "board.h"
#include "can_link.h"
#include "ota_local.h"
#include "serial_bridge.h"
#include "service_screen.h"

extern "C" void app_main() {
  // Pont série d'abord : UART2 installé et son bug de bring-up (bug 1,
  // GPIO44) corrigé avant tout le reste.
  serial_bridge::init();

  // CAN_SEL avant toute initialisation TWAI, voir docs/firmware.md : sans
  // ça, le transceiver n'est pas sélectionné et le bus reste muet, sans
  // erreur visible.
  board::ch422g_init();
  board::select_can();

  // PENDING_VERIFY : voir docs/firmware-implementation.md, phase 4 point 3,
  // et sensors/main.cpp (même mécanique). Ne jamais valider l'image tout de
  // suite ici — ota_local::start_validation_task() ne le fait qu'après un
  // PING/PONG confirmé sur le bus, ou rollback au bout du délai prévu.
  ota_local::init_pending_verify();

  can_link::init();

  can_link::send_log(common::LogCode::kBoot, common::LogSeverity::kInfo);
  if (ota_local::pending_verify()) {
    can_link::send_log(common::LogCode::kOtaPendingVerify, common::LogSeverity::kInfo);
  }

  // Pont TWAI/UART et validation OTA sur le cœur 0 ; LCD/LVGL sur le
  // cœur 1 (docs/screen-issue.md). start_tasks() et start_validation_task()
  // épinglent explicitement.
  serial_bridge::start_tasks();
  ota_local::start_validation_task();

  can_link::send_log(common::LogCode::kReady, common::LogSeverity::kInfo);

  // Écran de service (docs/plan-phase6.md, lot 2) : après le pont et le CAN,
  // pour que le conflit CH422G (dalle vs CAN_SEL) se révèle contre un bus
  // déjà vivant plutôt qu'un bus qui n'a jamais tourné.
  service_screen::init();
}

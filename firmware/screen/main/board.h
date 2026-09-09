// Brochage et CH422G (Waveshare ESP32-S3-Touch-LCD-4.3) — voir
// docs/firmware.md, "Module écran", et docs/plan-phase6.md pour le piège du
// registre de sortie partagé.
#pragma once

#include <cstdint>

#include "driver/gpio.h"
#include "driver/uart.h"

namespace board {

// I2C partagé CH422G/tactile (tactile non utilisé par ce pont).
constexpr gpio_num_t kI2cSda = GPIO_NUM_8;
constexpr gpio_num_t kI2cScl = GPIO_NUM_9;

// TWAI — GPIO20=TX, GPIO19=RX, broches natives D+/D- de l'USB de
// l'ESP32-S3, basculées vers CAN_TX/CAN_RX par le mux analogique FSUSB42UMX
// quand CAN_SEL (EXIO5) passe haut. Voir can_link.cpp.
constexpr gpio_num_t kCanTx = GPIO_NUM_20;
constexpr gpio_num_t kCanRx = GPIO_NUM_19;

// UART2 réaffecté sur GPIO43/44 (port "UART" réellement câblé sur ce banc).
// Voir serial_bridge.cpp pour les deux bugs de bring-up IDF v6.1 corrigés
// autour de ces broches.
constexpr uart_port_t kUartNum = UART_NUM_2;
constexpr gpio_num_t kUartTx = GPIO_NUM_43;
constexpr gpio_num_t kUartRx = GPIO_NUM_44;

// CH422G : pas de registre au sens I2C classique, l'adresse elle-même
// sélectionne la fonction. 0x24 = registre de mode (direction des EXIO),
// 0x38 = registre de sortie.
//
// Le registre de sortie est PARTAGÉ entre CAN_SEL/USB_SEL (EXIO5), LCD_BL
// (EXIO2), LCD_RST (EXIO3), TP_RST (EXIO1) et SD_CS (EXIO4) — voir
// docs/plan-phase6.md. Toute écriture doit passer par ch422g_set_bit(),
// JAMAIS écrire le registre de sortie directement : une écriture partielle
// écraserait les autres bits, typiquement en coupant le transceiver CAN au
// moment d'allumer la dalle.
enum Ch422gBit : uint8_t {
  kCh422gTpRst = 1 << 1,   // TP_RST — non piloté par ce lot
  kCh422gLcdBl = 1 << 2,   // LCD_BL — non piloté par ce lot
  kCh422gLcdRst = 1 << 3,  // LCD_RST — non piloté par ce lot
  kCh422gSdCs = 1 << 4,    // SD_CS — non piloté par ce lot
  // CAN_SEL == USB_SEL : mux analogique GPIO19/20 USB natif <-> CAN. Ne
  // jamais rebasculer ce bit hors de cet usage (docs/plan-phase6.md).
  kCh422gCanSel = 1 << 5,
};

// Bus I2C + registre de mode (push-pull outputs) ; initialise le miroir RAM
// du registre de sortie à 0 (état de sortie réel du CH422G au repos, avant
// toute écriture de ce firmware).
void ch422g_init();

// Lit/modifie/écrit le miroir RAM du registre de sortie CH422G, puis publie
// l'octet complet sur le bus I2C. C'est la seule fonction qui doit écrire ce
// registre.
void ch422g_set_bit(uint8_t bit, bool value);

// CAN_SEL avant tout le reste : voir docs/firmware.md, "doit être tenu haut,
// sinon le transceiver n'est pas sélectionné".
void select_can();

}  // namespace board

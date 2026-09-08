// Adaptateur CAN de secours — M5Stack Atom S3 + Unit CAN, indépendant de
// sensors/ et screen/. Voir docs/firmware-implementation.md, phase 2.
//
// Pur moniteur : accept-all, aucune émission, pas de décodage du protocole.
// Chaque trame reçue est imprimée en texte brut sur l'UART USB-C. Le
// décodage fin reste le travail de l'outil Mac (firmware/tools).

#include <cstdio>

#include "driver/twai.h"
#include "esp_log.h"

namespace {

// GPIO — Port.A de l'Atom S3, modifiables sans fouiller le reste du code.
// GPIO 26/36 documentés initialement étaient faux pour cet exemplaire ;
// confirmé au multimètre puis par auto-test de bouclage transceiver
// (firmware/can-selftest) le 2026-09-08. Voir docs/firmware-implementation.md.
constexpr gpio_num_t kCanTx = GPIO_NUM_2;
constexpr gpio_num_t kCanRx = GPIO_NUM_1;

}  // namespace

extern "C" void app_main() {
  // NORMAL, pas LISTEN_ONLY : le mode écoute seule ne pose pas le bit ACK,
  // ce qui ferait échouer indéfiniment les transmissions de l'autre carte
  // s'il n'y a qu'elles deux sur le bus. « Aucune émission » ci-dessous ne
  // veut dire qu'on n'appelle jamais twai_transmit(), pas qu'on prive le
  // bus de l'ACK matériel dont CAN a besoin.
  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(kCanTx, kCanRx, TWAI_MODE_NORMAL);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  printf("can-monitor: écoute à 500 kbit/s, TX=%d RX=%d\n", kCanTx, kCanRx);

  for (;;) {
    twai_message_t msg;
    if (twai_receive(&msg, portMAX_DELAY) != ESP_OK) {
      continue;
    }
    printf("id=0x%03lx dlc=%u data=", static_cast<unsigned long>(msg.identifier), msg.data_length_code);
    for (uint8_t i = 0; i < msg.data_length_code; ++i) {
      printf("%02x ", msg.data[i]);
    }
    printf("\n");
  }
}

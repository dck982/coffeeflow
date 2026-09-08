// Adaptateur CAN de secours — M5Stack Atom S3 + Unit CAN, indépendant de
// sensors/ et screen/. Voir docs/firmware-implementation.md, phase 2.
//
// Moniteur passif (accept-all, texte brut sur l'UART USB-C, pas de décodage
// du protocole — ça reste le travail de l'outil Mac, firmware/tools) plus
// un PING périodique : avant que l'écran (phase 3) n'existe, rien d'autre
// sur ce banc ne peut faire émettre un PONG au module capteurs pour
// vérifier l'aller-retour. Émis avec src=kScreen, seul node que sensors/
// écoute (voir dispatch_frame dans firmware/sensors/main.cpp) — un vrai
// écran sur le même bus entrerait en conflit d'identité avec ce PING, donc
// ne pas laisser tourner can-monitor une fois la phase 3 en place.

#include <cstdio>

#include "driver/twai.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "common/protocol.hpp"

namespace {

// GPIO — Port.A de l'Atom S3, modifiables sans fouiller le reste du code.
// GPIO 26/36 documentés initialement étaient faux pour cet exemplaire ;
// confirmé au multimètre puis par auto-test de bouclage transceiver
// (firmware/can-selftest) le 2026-09-08. Voir docs/firmware-implementation.md.
constexpr gpio_num_t kCanTx = GPIO_NUM_2;
constexpr gpio_num_t kCanRx = GPIO_NUM_1;

constexpr int64_t kPingPeriodMs = 2000;

void ping_task(void*) {
  for (;;) {
    common::CanId id{common::MessageType::kPing, common::Dest::kSensors, common::Node::kScreen};
    twai_message_t msg{};
    msg.identifier = common::encode_can_id(id);
    msg.data_length_code = 0;
    esp_err_t err = twai_transmit(&msg, pdMS_TO_TICKS(50));
    if (err != ESP_OK) {
      printf("can-monitor: PING échec: %s\n", esp_err_to_name(err));
    }
    vTaskDelay(pdMS_TO_TICKS(kPingPeriodMs));
  }
}

}  // namespace

extern "C" void app_main() {
  // NORMAL, pas LISTEN_ONLY : le mode écoute seule ne pose pas le bit ACK,
  // ce qui ferait échouer indéfiniment les transmissions de l'autre carte
  // s'il n'y a qu'elles deux sur le bus.
  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(kCanTx, kCanRx, TWAI_MODE_NORMAL);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  printf("can-monitor: écoute à 500 kbit/s, TX=%d RX=%d, PING toutes les %lld ms\n", kCanTx, kCanRx,
         static_cast<long long>(kPingPeriodMs));

  xTaskCreate(ping_task, "ping", 4096, nullptr, 5, nullptr);

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

// Auto-test transceiver CAN, sans câble — isole "mon transceiver marche"
// de "le câble/la terminaison marche". Voir la discussion de bring-up
// phase 2 dans docs/firmware-implementation.md.
//
// TWAI_MODE_NO_ACK : la trame part par le vrai TXD, traverse le
// transceiver, sort sur CANH/CANL, et revient par le même transceiver sur
// RXD — donc un vrai aller-retour analogique, pas un rebouclage logiciel
// interne du contrôleur (ça, c'est le flag TWAI_MSG_FLAG_SELF, qu'on
// n'utilise pas ici). NO_ACK évite juste d'attendre un accusé de réception
// qui ne viendra jamais, puisqu'on est seul sur le bus par construction.
//
// Câble CAN débranché pendant ce test, sur les deux cartes : sinon on ne
// sait plus si un échec vient du transceiver local ou du câble/de l'autre
// carte. Voir le fil de discussion pour le détail.

#include <cstdio>
#include <cstring>

#include "driver/gpio.h"
#include "driver/twai.h"
#include "esp_log.h"
#include "esp_rom_sys.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

// 1 = test TWAI (NO_ACK, décrit ci-dessus). 0 = test brut : GPIO nus, sans
// contrôleur TWAI ni notion de bit timing CAN, pour savoir si le TJA1051
// est simplement vivant (RXD suit TXD au niveau DC) avant de se soucier de
// savoir s'il tient 500 kbit/s.
#define TEST_MODE_TWAI 1

// Une seule carte à la fois : mettre 1 pour la carte testée, 0 pour
// l'autre, puis reconstruire (idf.py build) avant de flasher.
#define BOARD_XIAO_SENSORS 1
#define BOARD_ATOM_CANMON 0    // Unit CAN, Port.A — TX 2, RX 1 (confirmé)

// Brochage XIAO port R3 (D8=GPIO7, D9=GPIO8). Deux conventions en jeu :
//   1 = CAN Pal (docs/cablage.md) : TX → blanc → GPIO7, RX → jaune → GPIO8
//   0 = Unit CAN actuel : GPIO7 = RX, GPIO8 = TX
#define PINOUT_CANPAL 1

#if BOARD_XIAO_SENSORS
#if PINOUT_CANPAL
constexpr gpio_num_t kCanTx = GPIO_NUM_7;
constexpr gpio_num_t kCanRx = GPIO_NUM_8;
constexpr const char* kBoardName = "XIAO sensors (CAN Pal, TX=7 RX=8)";
#else
constexpr gpio_num_t kCanTx = GPIO_NUM_8;
constexpr gpio_num_t kCanRx = GPIO_NUM_7;
constexpr const char* kBoardName = "XIAO sensors (Unit CAN, TX=8 RX=7)";
#endif
#elif BOARD_ATOM_CANMON
// Port.A réel de cet Atom S3 : fil jaune (broche extérieure, étiquette G1)
// = TX du Unit CAN, fil blanc (G2) = RX. Donc TX du XIAO... pardon, TX de
// l'ESP32 → GPIO 2 (vers RX du Unit CAN), RX de l'ESP32 → GPIO 1 (depuis TX
// du Unit CAN). Pas 26/36 comme documenté par défaut : à corriger dans
// docs/firmware.md une fois confirmé. Si ça ne marche toujours pas,
// essayer l'inverse (TX=1, RX=2).
constexpr gpio_num_t kCanTx = GPIO_NUM_2;
constexpr gpio_num_t kCanRx = GPIO_NUM_1;
constexpr const char* kBoardName = "Atom can-monitor (Unit CAN)";
#else
#error "Choisir BOARD_XIAO_SENSORS ou BOARD_ATOM_CANMON"
#endif

namespace {
constexpr uint32_t kTestId = 0x123;
}

#if !TEST_MODE_TWAI

// TJA1051 non-inverseur : TXD bas → bus dominant → RXD bas. TXD haut →
// recessif → RXD haut. Aucun contrôleur TWAI ici, juste deux GPIO — ça
// répond uniquement à "la puce est-elle vivante", pas "tient-elle
// 500 kbit/s".
extern "C" void app_main() {
  printf("can-selftest (brut, sans TWAI): %s, TXD=%d RXD=%d, câble débranché attendu\n", kBoardName, kCanTx, kCanRx);

  gpio_config_t tx_cfg{};
  tx_cfg.pin_bit_mask = 1ULL << kCanTx;
  tx_cfg.mode = GPIO_MODE_OUTPUT;
  tx_cfg.pull_up_en = GPIO_PULLUP_DISABLE;
  tx_cfg.pull_down_en = GPIO_PULLDOWN_DISABLE;
  tx_cfg.intr_type = GPIO_INTR_DISABLE;
  gpio_config(&tx_cfg);

  gpio_config_t rx_cfg{};
  rx_cfg.pin_bit_mask = 1ULL << kCanRx;
  rx_cfg.mode = GPIO_MODE_INPUT;
  rx_cfg.pull_up_en = GPIO_PULLUP_DISABLE;
  rx_cfg.pull_down_en = GPIO_PULLDOWN_DISABLE;
  rx_cfg.intr_type = GPIO_INTR_DISABLE;
  gpio_config(&rx_cfg);

  gpio_set_level(kCanTx, 1);  // recessif au repos
  printf("bit-bang LENT (lisible au multimètre): TXD=GPIO%d doit basculer 0/3.3V ~1 Hz\n", (int)kCanTx);
  printf("impulsion trop longue pour le TJA1051: RXD qui ne suit pas ne prouve rien ici\n");

  for (;;) {
    gpio_set_level(kCanTx, 0);
    int rx_low = gpio_get_level(kCanRx);
    printf("TXD=0  RXD=%d  (pad TX doit lire ~0 V)\n", rx_low);
    vTaskDelay(pdMS_TO_TICKS(400));
    gpio_set_level(kCanTx, 1);
    int rx_high = gpio_get_level(kCanRx);
    printf("TXD=1  RXD=%d  (pad TX doit lire ~3.3 V)\n", rx_high);
    vTaskDelay(pdMS_TO_TICKS(400));
  }
}

#else

extern "C" void app_main() {
  printf("can-selftest: %s, TX=%d RX=%d, TWAI_MODE_NO_ACK, câble débranché attendu\n", kBoardName, kCanTx, kCanRx);

  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(kCanTx, kCanRx, TWAI_MODE_NO_ACK);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  uint8_t counter = 0;
  uint32_t pass = 0, fail = 0;

  for (;;) {
    twai_message_t tx{};
    tx.identifier = kTestId;
    tx.data_length_code = 1;
    tx.data[0] = counter;
    // Confirmation temporaire : self-reception request, pour distinguer
    // "le contrôleur ne met jamais nos propres trames NO_ACK en file RX"
    // (comportement normal, sans rapport avec le transceiver) d'un vrai
    // problème matériel. Voir la discussion en cours. À retirer une fois
    // confirmé.
    tx.flags = TWAI_MSG_FLAG_SELF;

    esp_err_t tx_err = twai_transmit(&tx, pdMS_TO_TICKS(100));
    if (tx_err != ESP_OK) {
      fail++;
      printf("[%3u] TX échec: %s (total pass=%lu fail=%lu)\n", counter, esp_err_to_name(tx_err),
             static_cast<unsigned long>(pass), static_cast<unsigned long>(fail));
    } else {
      twai_message_t rx{};
      esp_err_t rx_err = twai_receive(&rx, pdMS_TO_TICKS(100));
      bool ok = (rx_err == ESP_OK) && rx.identifier == kTestId && rx.data_length_code == 1 && rx.data[0] == counter;
      if (ok) {
        pass++;
        printf("[%3u] PASS — bouclé par le transceiver (pass=%lu fail=%lu)\n", counter,
               static_cast<unsigned long>(pass), static_cast<unsigned long>(fail));
      } else {
        fail++;
        printf("[%3u] FAIL — TX ok mais rien reçu en retour (%s) (pass=%lu fail=%lu)\n", counter,
               esp_err_to_name(rx_err), static_cast<unsigned long>(pass), static_cast<unsigned long>(fail));
      }
    }

    twai_status_info_t status;
    if (twai_get_status_info(&status) == ESP_OK) {
      const char* state_name = "?";
      switch (status.state) {
        case TWAI_STATE_STOPPED: state_name = "STOPPED"; break;
        case TWAI_STATE_RUNNING: state_name = "RUNNING"; break;
        case TWAI_STATE_BUS_OFF: state_name = "BUS_OFF"; break;
        case TWAI_STATE_RECOVERING: state_name = "RECOVERING"; break;
      }
      printf("      state=%s tx_queued=%lu rx_queued=%lu tx_err=%lu rx_err=%lu tx_failed=%lu bus_err=%lu arb_lost=%lu\n",
             state_name, static_cast<unsigned long>(status.msgs_to_tx), static_cast<unsigned long>(status.msgs_to_rx),
             static_cast<unsigned long>(status.tx_error_counter), static_cast<unsigned long>(status.rx_error_counter),
             static_cast<unsigned long>(status.tx_failed_count), static_cast<unsigned long>(status.bus_error_count),
             static_cast<unsigned long>(status.arb_lost_count));
    }

    counter++;
    vTaskDelay(pdMS_TO_TICKS(1000));
  }
}

#endif  // TEST_MODE_TWAI

// Pont série <-> CAN — voir board.h pour le détail du brochage.
//
// Deux bugs de bring-up trouvés et corrigés ici (2026-09-09), tous deux
// silencieux — aucun message d'erreur, juste une réception qui ne marche
// jamais :
//   1. Régression IDF v5.x -> v6.1 : `uart_set_pin()` route bien GPIO44 vers
//      UART2 via la matrice GPIO, mais sa branche RX (gpio_hal_matrix_in())
//      ne force plus le pad en PIN_FUNC_GPIO comme le faisait v5.1 (la
//      branche TX, elle, le fait toujours) — sans forcer nous-mêmes le pad,
//      GPIO44 reste sur sa fonction IOMUX de reset (U0RXD), jamais lue.
//      Corrigé par un appel explicite à gpio_func_sel()/gpio_input_enable()/
//      gpio_pullup_en() juste après uart_set_pin().
//   2. `uart_read_bytes(..., portMAX_DELAY)` ne se réveille jamais sur cet
//      UART2 réaffecté, même une fois le bug 1 corrigé et la FIFO
//      effectivement pleine (confirmé par uart_get_buffered_data_len()) —
//      un timeout court (20 ms) en boucle, comme le fait l'exemple officiel
//      Waveshare (05_UART_Test), fonctionne à chaque essai.
// Les deux bugs ont été diagnostiqués en comparant notre code à
// waveshareteam/ESP32-S3-Touch-LCD-4.3 (cloné dans tmp/), et en confirmant
// par bissection matérielle (sonde directe de GPIO44, uart_get_buffered_data_len())
// que le signal physique et la FIFO UART étaient sains avant de soupçonner
// l'appel bloquant. Voir docs/firmware-implementation.md pour le détail.

#include "serial_bridge.h"

#include <cstring>

#include "driver/gpio.h"
#include "driver/twai.h"
#include "driver/uart.h"
#include "esp_err.h"
#include "esp_private/gpio.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "board.h"
#include "can_link.h"
#include "common/framing.hpp"
#include "common/protocol.hpp"
#include "ota_local.h"

namespace serial_bridge {

namespace {

// uart_read_bytes(..., portMAX_DELAY) ne se réveille jamais sur cet UART2 —
// voir le commentaire en tête de fichier, bug 2. Boucler avec ce délai court
// à la place.
constexpr TickType_t kUartReadTimeout = pdMS_TO_TICKS(20);

// Série -> CAN : relais aveugle, comme can-monitor, SAUF pour notre propre
// OTA (FLASH_CTRL/FLASH_DATA adressé à kScreen) — c'est un échange direct
// Mac<->screen par USB, voir docs/firmware-implementation.md : rien à
// relayer sur le bus dans ce cas, et surtout pas les 256 trames FLASH_DATA
// par bloc qui n'ont aucun sens pour sensors. coffeetool émet déjà avec
// l'identité qu'il veut pour le reste, rien à réinterpréter ici.
void on_frame_from_serial(const common::RawFrame& frame, void* /*ctx*/) {
  common::CanId id = common::decode_can_id(frame.can_id);
  if (id.dest == common::Dest::kScreen &&
      (id.type == common::MessageType::kFlashCtrl || id.type == common::MessageType::kFlashData)) {
    if (id.type == common::MessageType::kFlashCtrl) {
      ota_local::on_flash_ctrl_received(frame.data.data(), frame.dlc);
    } else {
      ota_local::on_flash_data_received(frame.data.data(), frame.dlc);
    }
    return;
  }

  twai_message_t msg{};
  msg.identifier = frame.can_id;
  msg.data_length_code = frame.dlc;
  std::memcpy(msg.data, frame.data.data(), frame.dlc);
  twai_transmit(&msg, pdMS_TO_TICKS(50));
}

// CAN -> série : chaque trame reçue devient un PDU encadré écrit tel quel
// sur l'USB (pont, comme can-monitor), et est aussi soumise à notre propre
// protocole (nœud kScreen).
void can_to_serial_task(void*) {
  for (;;) {
    twai_message_t msg;
    if (twai_receive(&msg, portMAX_DELAY) != ESP_OK) {
      continue;
    }

    common::RawFrame frame;
    frame.can_id = static_cast<uint16_t>(msg.identifier);
    frame.dlc = msg.data_length_code;
    std::memcpy(frame.data.data(), msg.data, frame.dlc);

    uint8_t out[common::kMaxCobsSize + 1];
    size_t len = common::encode_framed(frame, out);
    if (len > 0) {
      write_raw(out, len);
    }

    can_link::dispatch_own_protocol(msg);
  }
}

void serial_to_can_task(void*) {
  common::StreamDecoder decoder;
  uint8_t buf[64];
  for (;;) {
    // Timeout court, pas portMAX_DELAY : voir le commentaire en tête de
    // fichier, bug 2 — uart_read_bytes() bloquant indéfiniment ne se
    // réveille jamais sur cet UART2 réaffecté.
    int n = uart_read_bytes(board::kUartNum, buf, sizeof(buf), kUartReadTimeout);
    for (int i = 0; i < n; ++i) {
      decoder.push_byte(buf[i], on_frame_from_serial, nullptr);
    }
  }
}

}  // namespace

void init() {
  uart_config_t uart_config{};
  uart_config.baud_rate = 115200;
  uart_config.data_bits = UART_DATA_8_BITS;
  uart_config.parity = UART_PARITY_DISABLE;
  uart_config.stop_bits = UART_STOP_BITS_1;
  uart_config.flow_ctrl = UART_HW_FLOWCTRL_DISABLE;
  uart_config.source_clk = UART_SCLK_DEFAULT;
  ESP_ERROR_CHECK(uart_driver_install(board::kUartNum, 2048, 0, 0, nullptr, 0));
  ESP_ERROR_CHECK(uart_param_config(board::kUartNum, &uart_config));
  ESP_ERROR_CHECK(uart_set_pin(board::kUartNum, board::kUartTx, board::kUartRx, UART_PIN_NO_CHANGE,
                                UART_PIN_NO_CHANGE));
  // Voir le commentaire en tête de fichier, bug 1 : régression IDF v5->v6.1,
  // gpio_hal_matrix_in() (branche RX de uart_set_pin) ne force plus le pad
  // en PIN_FUNC_GPIO — sans ça GPIO44 reste sur sa fonction IOMUX de reset
  // (U0RXD) et ne reçoit jamais rien.
  gpio_func_sel(board::kUartRx, PIN_FUNC_GPIO);
  gpio_input_enable(board::kUartRx);
  gpio_pullup_en(board::kUartRx);
}

void write_raw(const uint8_t* data, size_t len) {
  uart_write_bytes(board::kUartNum, reinterpret_cast<const char*>(data), len);
}

void start_tasks() {
  xTaskCreatePinnedToCore(can_to_serial_task, "can2ser", 4096, nullptr, 10, nullptr, 1);
  xTaskCreatePinnedToCore(serial_to_can_task, "ser2can", 4096, nullptr, 10, nullptr, 1);
}

}  // namespace serial_bridge

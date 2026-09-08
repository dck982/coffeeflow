// Adaptateur CAN de secours — M5Stack Atom S3 + Unit CAN, indépendant de
// sensors/ et screen/. Voir docs/firmware-implementation.md, phase 2.
//
// Pont binaire bidirectionnel série <-> CAN, même cadrage que celui prévu
// pour l'écran en phase 3 (COBS + PDU + CRC16, voir
// firmware/common/include/common/framing.hpp) : coffeetool (firmware/tools)
// peut donc lui parler directement sur son port USB-C, sans attendre
// l'écran, pour envoyer SET/STOP/RESET/REQSTATUS/PING vers sensors/ sur le
// vrai bus et lire les réponses.
//
// Piège : le port USB-C de l'Atom S3 est le périphérique USB_SERIAL_JTAG
// natif du chip, le même que celui qu'ESP-IDF utilise par défaut pour la
// console (printf/ESP_LOGx). Si la console reste active dessus, du texte de
// debug s'intercale dans le flux binaire et casse le cadrage COBS côté
// coffeetool — donc console désactivée (CONFIG_ESP_CONSOLE_NONE +
// CONFIG_ESP_CONSOLE_SECONDARY_NONE, voir sdkconfig.defaults), et ce fichier
// pilote le driver usb_serial_jtag directement (lecture/écriture d'octets
// bruts, sans la conversion de fins de ligne que fait le VFS console — une
// donnée CAN quelconque peut très bien contenir 0x0A ou 0x0D). Plus aucun
// printf/ESP_LOG possible : les erreurs qui comptent (LOG côté firmware)
// passent déjà par le bus CAN, pas par ce port.
//
// Le PING périodique ajouté plus tôt (avant ce pont, pour voir un PONG sans
// outil côté Mac) est retiré : coffeetool peut désormais l'envoyer lui-même
// avec la bonne identité, pas de raison de garder un second émetteur.

#include <cstring>

#include "driver/twai.h"
#include "driver/usb_serial_jtag.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "common/framing.hpp"

namespace {

// GPIO — Port.A de l'Atom S3, modifiables sans fouiller le reste du code.
// GPIO 26/36 documentés initialement étaient faux pour cet exemplaire ;
// confirmé au multimètre puis par auto-test de bouclage transceiver
// (firmware/can-selftest) le 2026-09-08. Voir docs/firmware-implementation.md.
constexpr gpio_num_t kCanTx = GPIO_NUM_2;
constexpr gpio_num_t kCanRx = GPIO_NUM_1;

// CAN -> série : une trame reçue sur le bus devient un PDU encadré (COBS +
// 0x00) écrit tel quel sur l'USB.
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
    if (len == 0) {
      continue;  // gabarit hors norme (ne devrait pas arriver depuis un twai_message_t valide)
    }
    usb_serial_jtag_write_bytes(out, len, portMAX_DELAY);
  }
}

// Série -> CAN : chaque PDU décadré devient une trame transmise sur le bus.
void on_frame_from_serial(const common::RawFrame& frame, void* /*ctx*/) {
  twai_message_t msg{};
  msg.identifier = frame.can_id;
  msg.data_length_code = frame.dlc;
  std::memcpy(msg.data, frame.data.data(), frame.dlc);
  twai_transmit(&msg, pdMS_TO_TICKS(50));
}

void serial_to_can_task(void*) {
  common::StreamDecoder decoder;
  uint8_t buf[64];
  for (;;) {
    int n = usb_serial_jtag_read_bytes(buf, sizeof(buf), portMAX_DELAY);
    for (int i = 0; i < n; ++i) {
      decoder.push_byte(buf[i], on_frame_from_serial, nullptr);
    }
  }
}

}  // namespace

extern "C" void app_main() {
  usb_serial_jtag_driver_config_t usj_config = USB_SERIAL_JTAG_DRIVER_CONFIG_DEFAULT();
  ESP_ERROR_CHECK(usb_serial_jtag_driver_install(&usj_config));

  // NORMAL, pas LISTEN_ONLY : le mode écoute seule ne pose pas le bit ACK,
  // ce qui ferait échouer indéfiniment les transmissions de l'autre carte
  // s'il n'y a qu'elles deux sur le bus.
  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(kCanTx, kCanRx, TWAI_MODE_NORMAL);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  xTaskCreate(can_to_serial_task, "can2ser", 4096, nullptr, 10, nullptr);
  xTaskCreate(serial_to_can_task, "ser2can", 4096, nullptr, 10, nullptr);
}

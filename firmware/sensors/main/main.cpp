// Module capteurs (XIAO ESP32-S3) — phase 5 : application capteurs, livrée
// par OTA. Voir docs/firmware-implementation.md et docs/firmware.md.
//
// Socle phase 2 (bus, PING/PONG, LOG, RESET, machine à états de sécurité)
// plus, depuis la phase 5, le XDB401 (I2C, port R1) et le débitmètre Digmesa
// (GPIO, port R2). Pas encore de logique d'infusion.

#include <cstring>

#include "driver/gpio.h"
#include "driver/i2c_master.h"
#include "driver/twai.h"
#include "esp_log.h"
#include "esp_ota_ops.h"
#include "esp_partition.h"
#include "esp_private/gpio.h"
#include "esp_system.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"

#include "common/crc.hpp"
#include "common/framing.hpp"
#include "common/messages.hpp"
#include "common/protocol.hpp"
#include "common/version.hpp"
#include "log_codes.hpp"

namespace {

constexpr const char* kTag = "sensors";

// Brochage — voir docs/firmware.md, "Module capteurs". GPIO natif de
// l'ESP32-S3, pas le D-number imprimé sur le silkscreen du Grove Shield
// XIAO (D8/D9/D10 ≠ GPIO8/9/10 : voir la note dans docs/cablage.md).
//
// Module Adafruit CAN Pal (TJA1051T/3, clone AliExpress) — SLNT reworké
// au GND (voir docs/canpal-findings.md), remplace le Unit CAN de
// contournement. Câblage Pal (docs/cablage.md) : TX → blanc → GPIO7,
// RX → jaune → GPIO8. Confirmé par self-test TWAI en boucle isolée
// (firmware/can-selftest, PINOUT_CANPAL 1) le 2026-09-09 : 17/17 PASS,
// tx_err=0 rx_err=0 bus_err=0.
constexpr gpio_num_t kGpioSsr = GPIO_NUM_9;    // D10
constexpr gpio_num_t kGpioCanTx = GPIO_NUM_7;  // D8
constexpr gpio_num_t kGpioCanRx = GPIO_NUM_8;  // D9

// Débitmètre Digmesa 932-9525-B, port R2 — voir docs/firmware.md,
// "Débitmètre". Front descendant déjà mis en forme 3,3 V par le filtre RC du
// shield (1 kΩ vers 3,3 V, 10 nF vers GND) : pull-up interne éteinte.
constexpr gpio_num_t kGpioFlow = GPIO_NUM_44;  // D7

// XDB401 (pression/température), port R1 — voir docs/firmware.md, "XDB401 —
// pression et température" et docs/cablage.md. Bus I2C partagé avec le
// dimmer (pas encore câblé) : les seules pull-ups du bus sont les 4,7 kΩ du
// XDB401, ne pas activer les pull-ups internes ni en empiler d'autres.
constexpr gpio_num_t kGpioI2cSda = GPIO_NUM_5;  // D4
constexpr gpio_num_t kGpioI2cScl = GPIO_NUM_6;  // D5
constexpr uint16_t kXdb401Addr = 0x7F;
constexpr uint8_t kXdb401RegTrigger = 0x30;
constexpr uint8_t kXdb401TriggerValue = 0x0A;
constexpr uint8_t kXdb401RegPressure = 0x06;  // 3 octets, 0x06..0x08
constexpr uint8_t kXdb401RegTemperature = 0x09;  // 2 octets, 0x09..0x0A
constexpr uint32_t kXdb401ConversionDelayMs = 50;

// Dimmer RBDimmer/DimmerLink, même bus I2C (port L4) — voir docs/firmware.md,
// "Dimmer — pompe", et tmp/DimmerLink/04_I2C_COMMUNICATION.md (doc officielle
// du fabricant, trouvée et confirmée en bring-up le 2026-09-09 — le
// tests/test_rbi2c.py initial, sur un autre banc, avait la bonne carte de
// registres mais une table d'erreurs incomplète). 0x10 est la seule écriture
// du cycle d'infusion. 0x00 (STATUS, bit0=READY bit1=ERROR) est la source de
// vérité documentée pour l'état — pas le registre erreur seul, qui peut
// contenir des codes non documentés (0x01 observé en pratique, absent de la
// table officielle). Jamais écrire 0x01=SWITCH_UART au registre COMMAND
// (bascule UART, EEPROM) depuis ce firmware.
constexpr uint16_t kDimmerAddr = 0x50;
constexpr uint8_t kDimmerRegStatus = 0x00;
constexpr uint8_t kDimmerRegCommand = 0x01;
constexpr uint8_t kDimmerRegError = 0x02;
constexpr uint8_t kDimmerRegLevel = 0x10;
constexpr uint8_t kDimmerRegFreq = 0x20;  // informatif seulement, voir read_dimmer_health()
constexpr uint8_t kDimmerCmdRecalibrate = 0x02;
// Lu en dehors des écritures (voir tick_dimmer_health()), pour que les flags
// STATUS_ACTUATORS restent à jour même sans SET récent — contrairement au
// SSR, un écran qui ne rampe jamais le dimmer ne déclenche autrement aucune
// lecture I2C.
constexpr int64_t kDimmerHealthPeriodUs = 2000 * 1000;
// Marge avant un premier (et unique) essai de repli COMMAND=RECALIBRATE si
// le module reste "pas prêt" — voir dimmer_recalibrate() : le module calibre
// tout seul à la mise sous tension secteur, sans notre intervention
// (confirmé par un vrai power-cycle en bring-up le 2026-09-09, convergence
// naturelle observée sans jamais appeler RECALIBRATE). Volontairement large
// pour ne pas courir après une calibration déjà en cours.
constexpr int64_t kDimmerAutoRecalibrateDelayUs = 20 * 1000 * 1000;
// Plancher défensif : en dessous, deux lectures se chevaucheraient avec la
// conversion (~50 ms) du cycle précédent. Voir docs/firmware.md §"période".
constexpr uint16_t kPressurePeriodFloorMs = 100;

// Sécurité — voir docs/firmware.md, section "Sécurité".
constexpr int64_t kLeaseDefaultUs = 500 * 1000;
constexpr int64_t kPresenceTimeoutUs = 3000 * 1000;
constexpr int64_t kPresencePingIntervalUs = 1500 * 1000;
constexpr int64_t kRuntimeLockoutUs = 60 * 1000 * 1000;
constexpr int64_t kRuntimeRearmGapUs = 2000 * 1000;
constexpr int64_t kTwaiCountersLogPeriodUs = 5000 * 1000;
constexpr int64_t kTickPeriodUs = 100 * 1000;

// OTA — voir docs/firmware-implementation.md, phase 4. IDF ne redémarre
// jamais tout seul une image en PENDING_VERIFY : ce délai est le
// temporisateur d'invalidation explicite qui s'en charge. Réutilise le même
// signal que la présence (mark_presence(), donc un PING/PONG venu de
// l'écran/pont) comme preuve que l'image tourne assez pour parler au bus.
constexpr int64_t kOtaValidationTimeoutUs = 30 * 1000 * 1000;
constexpr size_t kFlashBlockSize = 2048;

// Verrou 60 s — tenu en mémoire RTC, ne survit qu'à un démarrage à froid.
// Voir docs/firmware.md §3 : une commande RESET venue du bus ne le lève pas.
constexpr uint32_t kRtcMagic = 0xC0FFEE02;
RTC_NOINIT_ATTR uint32_t s_rtc_magic;
RTC_NOINIT_ATTR bool s_lockout_active;

// État des actionneurs — RAM ordinaire, remis à zéro à chaque redémarrage,
// logiciel ou non (seul le verrou ci-dessus doit survivre).
bool g_ssr = false;
uint8_t g_dimmer = 0;
// Santé dimmer, mise à jour à chaque écriture (voir apply_dimmer()) : reflète
// la dernière transaction I2C réelle, pas un état supposé.
bool g_dimmer_present = false;      // dernière transaction I2C a abouti (adresse ack)
bool g_dimmer_ready = false;        // STATUS bit0 (READY) à la dernière lecture
bool g_dimmer_error_active = false; // STATUS bit1 (ERROR) à la dernière lecture
int64_t g_lease_deadline_us = 0;  // 0 = pas de bail actif

// Plafond de marche continue — voir docs/firmware.md §3, "Rearmement".
int64_t g_run_start_us = 0;   // 0 = pas de marche en cours
int64_t g_off_since_us = 0;   // 0 = pas actuellement à l'arrêt

// Présence — voir docs/firmware.md §2.
int64_t g_last_presence_rx_us = 0;
int64_t g_last_own_ping_us = 0;
bool g_presence_lost = true;  // état de repos correct, carte seule sur la table

// Validation OTA — voir docs/firmware-implementation.md, phase 4.
bool g_ota_pending_verify = false;
int64_t g_ota_pending_since_us = 0;

// XDB401 — voir docs/firmware.md, "I2C partagé" : le mutex sérialise
// l'accès au bus (utile dès que le dimmer sera câblé dessus), mais ne
// couvre pas l'attente de conversion (kXdb401ConversionDelayMs).
i2c_master_bus_handle_t g_i2c_bus = nullptr;
i2c_master_dev_handle_t g_xdb401_dev = nullptr;
i2c_master_dev_handle_t g_dimmer_dev = nullptr;
SemaphoreHandle_t g_i2c_mutex = nullptr;
uint16_t g_pressure_period_ms = 0;  // 0 = arrêt, voir REQSTATUS
int64_t g_pressure_last_sent_us = 0;
common::StatusPressurePayload g_last_pressure;  // dernière valeur connue (flags à jour)

// Débitmètre — compteur incrémenté depuis l'ISR, lu depuis les tâches. Pas
// de mutex : lecture d'un uint32_t/int64_t alignés sur un cœur unique, la
// pire chose qui puisse arriver est un STATUS_FLOW envoyé avec l'horodatage
// de l'impulsion juste précédente ou juste suivante, jamais une valeur
// déchirée en pratique sur cette architecture.
volatile uint32_t g_flow_pulse_count = 0;
volatile int64_t g_flow_last_edge_us = 0;
uint16_t g_flow_period_ms = 0;  // 0 = arrêt, voir REQSTATUS
int64_t g_flow_last_sent_us = 0;

void IRAM_ATTR flow_isr_handler(void*) {
  g_flow_pulse_count = g_flow_pulse_count + 1;
  g_flow_last_edge_us = esp_timer_get_time();
}

// Flash — un seul transfert à la fois, pas de file d'attente. `partition`
// non nul == transfert en cours (BEGIN reçu, END/ABORT pas encore traité).
struct FlashState {
  const esp_partition_t* partition = nullptr;
  esp_ota_handle_t ota_handle = 0;
  uint32_t image_size = 0;
  uint32_t bytes_written = 0;
  uint16_t block_number = 0;  // nombre de blocs déjà acquittés
  uint8_t block_buf[kFlashBlockSize];
  size_t block_buf_len = 0;
  common::Crc32Incremental image_crc;
};
FlashState g_flash;

int64_t now_us() { return esp_timer_get_time(); }

void apply_ssr() { gpio_set_level(kGpioSsr, g_ssr ? 1 : 0); }

// Déclaration en avance : apply_dimmer() a besoin de send_log(), défini plus
// bas dans ce fichier avec les autres émetteurs.
void send_log(common::LogCode code, common::LogSeverity severity, uint16_t arg16, uint32_t arg32);

// Relit STATUS (0x00, bit0=READY bit1=ERROR — voir tmp/DimmerLink/
// 04_I2C_COMMUNICATION.md, doc officielle) et, informativement seulement,
// la fréquence secteur (0x20). Utilisée aussi bien après une écriture
// (apply_dimmer()) que périodiquement à vide (tick_dimmer_health()), pour
// que g_dimmer_present/ready/error_active restent à jour même sans SET
// récent. Transaction combinée (write pointeur + read en repeated-start,
// pas deux transactions séparées) — c'est ce que fait la doc officielle et
// tests/test_rbi2c.py (i2c.readfrom_mem()).
void read_dimmer_health() {
  const uint8_t reg_status = kDimmerRegStatus;
  uint8_t status_value = 0;

  xSemaphoreTake(g_i2c_mutex, portMAX_DELAY);
  esp_err_t err = i2c_master_transmit_receive(g_dimmer_dev, &reg_status, 1, &status_value, 1, pdMS_TO_TICKS(100));
  xSemaphoreGive(g_i2c_mutex);

  if (err != ESP_OK) {
    g_dimmer_present = false;
    g_dimmer_ready = false;
    g_dimmer_error_active = false;
    send_log(common::LogCode::kI2cError, common::LogSeverity::kError, kDimmerAddr, 0);
    return;
  }
  g_dimmer_present = true;
  g_dimmer_ready = (status_value & 0x01) != 0;
  g_dimmer_error_active = (status_value & 0x02) != 0;

  if (!g_dimmer_ready) {
    send_log(common::LogCode::kDimmerCalibrating, common::LogSeverity::kWarn, 0, 0);
  }
  if (g_dimmer_error_active) {
    const uint8_t reg_error = kDimmerRegError;
    uint8_t error_value = 0xFF;
    xSemaphoreTake(g_i2c_mutex, portMAX_DELAY);
    esp_err_t eerr = i2c_master_transmit_receive(g_dimmer_dev, &reg_error, 1, &error_value, 1, pdMS_TO_TICKS(100));
    xSemaphoreGive(g_i2c_mutex);
    send_log(common::LogCode::kDimmerError, common::LogSeverity::kError, eerr == ESP_OK ? error_value : 0xFFFF, 0);
  }

  // Fréquence secteur : purement informatif. Observée à 0 en continu sur ce
  // module même une fois READY=1 et sans ERROR (bring-up 2026-09-09), donc
  // pas fiable comme critère de "secteur détecté" malgré ce que documente
  // tmp/DimmerLink — journalisée pour diagnostic futur, jamais utilisée pour
  // piloter un flag.
  const uint8_t reg_freq = kDimmerRegFreq;
  uint8_t freq_value = 0;
  xSemaphoreTake(g_i2c_mutex, portMAX_DELAY);
  esp_err_t ferr = i2c_master_transmit_receive(g_dimmer_dev, &reg_freq, 1, &freq_value, 1, pdMS_TO_TICKS(100));
  xSemaphoreGive(g_i2c_mutex);
  send_log(common::LogCode::kDimmerMainsFreq, common::LogSeverity::kDebug, ferr == ESP_OK ? freq_value : 0xFFFF, 0);
}

// COMMAND=RECALIBRATE (0x02) : sans ça après une mise sous tension secteur
// tardive ou une correction de câblage, le module peut rester avec
// STATUS.READY=0 même une fois le secteur stable — confirmé en bring-up
// 2026-09-09 (STATUS passe de 0x32 à "ready=1,err=0" juste après ce
// recalibrate). Non fatal si le module n'est pas encore présent au boot.
void dimmer_recalibrate() {
  const uint8_t cmd[2] = {kDimmerRegCommand, kDimmerCmdRecalibrate};
  xSemaphoreTake(g_i2c_mutex, portMAX_DELAY);
  esp_err_t err = i2c_master_transmit(g_dimmer_dev, cmd, sizeof(cmd), pdMS_TO_TICKS(100));
  xSemaphoreGive(g_i2c_mutex);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "dimmer recalibrate échec: %s", esp_err_to_name(err));
  }
}

// Écrit le niveau courant sur le DimmerLink puis relit sa santé (STATUS,
// erreur si besoin, fréquence en informatif) — voir docs/firmware.md,
// "Dimmer — pompe".
void apply_dimmer() {
  const uint8_t write_level[2] = {kDimmerRegLevel, g_dimmer};

  xSemaphoreTake(g_i2c_mutex, portMAX_DELAY);
  esp_err_t err = i2c_master_transmit(g_dimmer_dev, write_level, sizeof(write_level), pdMS_TO_TICKS(100));
  xSemaphoreGive(g_i2c_mutex);
  if (err != ESP_OK) {
    if (g_dimmer_present) {
      send_log(common::LogCode::kI2cError, common::LogSeverity::kError, kDimmerAddr, 0);
    }
    g_dimmer_present = false;
    g_dimmer_ready = false;
    g_dimmer_error_active = false;
    return;
  }

  read_dimmer_health();
}

// Lecture périodique à vide (sans écrire de niveau) — voir le commentaire de
// kDimmerHealthPeriodUs : sans elle, un dimmer jamais commandé (aucun SET
// reçu depuis le boot) resterait avec des flags STATUS_ACTUATORS figés à
// leur valeur initiale.
void tick_dimmer_health() {
  static int64_t s_last_us = 0;
  static bool s_recalibrate_sent = false;
  int64_t t = now_us();
  if (t - s_last_us < kDimmerHealthPeriodUs) return;
  s_last_us = t;
  read_dimmer_health();

  // Repli, une seule fois par épisode "pas prêt" : voir le commentaire de
  // dimmer_recalibrate() sur la race avec la calibration propre au module.
  // g_dimmer_present exigé : pas de repli tant qu'on n'a jamais eu de vraie
  // réponse I2C (câblage/alimentation, pas un problème de calibration).
  if (g_dimmer_present && !g_dimmer_ready && !s_recalibrate_sent && t > kDimmerAutoRecalibrateDelayUs) {
    s_recalibrate_sent = true;
    dimmer_recalibrate();
  }
  if (g_dimmer_ready) {
    s_recalibrate_sent = false;  // réarme pour une future coupure/reprise secteur
  }
}

void send_message(common::MessageType type, common::Dest dest, const uint8_t* data, uint8_t dlc) {
  common::CanId id{type, dest, common::Node::kSensors};
  twai_message_t msg{};
  msg.identifier = common::encode_can_id(id);
  msg.data_length_code = dlc;
  if (dlc > 0) {
    std::memcpy(msg.data, data, dlc);
  }
  esp_err_t err = twai_transmit(&msg, pdMS_TO_TICKS(50));
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "twai_transmit type=0x%02x échec: %s", static_cast<int>(type), esp_err_to_name(err));
  }
}

void send_log(common::LogCode code, common::LogSeverity severity, uint16_t arg16 = 0, uint32_t arg32 = 0) {
  common::LogPayload payload;
  payload.code = static_cast<uint8_t>(code);
  payload.severity = static_cast<uint8_t>(severity);
  payload.arg16 = arg16;
  payload.arg32 = arg32;
  common::Frame f = payload.pack();
  send_message(common::MessageType::kLog, common::Dest::kBroadcast, f.data(), 8);
}

void send_pong() {
  common::PongPayload payload;
  payload.node = common::Node::kSensors;
  payload.version_major = common::kFirmwareVersionMajor;
  payload.version_minor = common::kFirmwareVersionMinor;
  payload.version_patch = common::kFirmwareVersionPatch;
  payload.uptime_s = static_cast<uint32_t>(now_us() / 1000000);
  common::Frame f = payload.pack();
  send_message(common::MessageType::kPong, common::Dest::kScreen, f.data(), 8);
}

void send_status_actuators() {
  common::StatusActuatorsPayload payload;
  payload.ssr = g_ssr;
  payload.dimmer = g_dimmer;
  int64_t t = now_us();
  int64_t lease_remaining = g_lease_deadline_us > t ? (g_lease_deadline_us - t) / 1000 : 0;
  payload.lease_remaining_ms = static_cast<uint16_t>(lease_remaining > 0xFFFF ? 0xFFFF : lease_remaining);
  int64_t continuous_ms = g_run_start_us != 0 ? (t - g_run_start_us) / 1000 : 0;
  payload.continuous_on_ms = static_cast<uint16_t>(continuous_ms > 0xFFFF ? 0xFFFF : continuous_ms);
  payload.flags = static_cast<uint8_t>((s_lockout_active ? 0x01 : 0) | (g_dimmer_ready ? 0x02 : 0) |
                                        (g_dimmer_present ? 0x04 : 0) | (g_dimmer_error_active ? 0x08 : 0));
  common::Frame f = payload.pack();
  send_message(common::MessageType::kStatusActuators, common::Dest::kScreen, f.data(), 7);
}

// Coupe les actionneurs immédiatement, sans toucher au verrou lui-même.
void force_actuators_off() {
  g_ssr = false;
  g_dimmer = 0;
  g_lease_deadline_us = 0;
  apply_ssr();
  apply_dimmer();
}

void init_flow() {
  gpio_config_t flow_cfg{};
  flow_cfg.pin_bit_mask = 1ULL << kGpioFlow;
  flow_cfg.mode = GPIO_MODE_INPUT;
  flow_cfg.pull_up_en = GPIO_PULLUP_DISABLE;  // filtre RC du shield, pas la pull-up interne
  flow_cfg.pull_down_en = GPIO_PULLDOWN_DISABLE;
  flow_cfg.intr_type = GPIO_INTR_NEGEDGE;
  ESP_ERROR_CHECK(gpio_config(&flow_cfg));

  // GPIO44 est le RX par défaut de la console UART0 (CONFIG_ESP_CONSOLE_UART_
  // DEFAULT) : sans forcer nous-mêmes le pad en PIN_FUNC_GPIO, il reste sur
  // sa fonction IOMUX de reset (U0RXD) et l'ISR ne voit jamais rien — même
  // piège déjà rencontré et corrigé sur ce même GPIO côté screen/main.cpp
  // (bug 1, régression IDF v5.x→v6.1 dans le forçage de pad).
  gpio_func_sel(kGpioFlow, PIN_FUNC_GPIO);
  gpio_input_enable(kGpioFlow);

  ESP_ERROR_CHECK(gpio_install_isr_service(0));
  ESP_ERROR_CHECK(gpio_isr_handler_add(kGpioFlow, flow_isr_handler, nullptr));
}

void init_i2c() {
  i2c_master_bus_config_t bus_cfg{};
  bus_cfg.i2c_port = -1;
  bus_cfg.sda_io_num = kGpioI2cSda;
  bus_cfg.scl_io_num = kGpioI2cScl;
  bus_cfg.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_cfg.glitch_ignore_cnt = 7;
  bus_cfg.flags.enable_internal_pullup = false;  // pull-ups déjà sur le XDB401
  ESP_ERROR_CHECK(i2c_new_master_bus(&bus_cfg, &g_i2c_bus));

  i2c_device_config_t dev_cfg{};
  dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  dev_cfg.device_address = kXdb401Addr;
  dev_cfg.scl_speed_hz = 100000;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(g_i2c_bus, &dev_cfg, &g_xdb401_dev));

  i2c_device_config_t dimmer_cfg{};
  dimmer_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  dimmer_cfg.device_address = kDimmerAddr;
  dimmer_cfg.scl_speed_hz = 100000;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(g_i2c_bus, &dimmer_cfg, &g_dimmer_dev));

  g_i2c_mutex = xSemaphoreCreateMutex();
}

// Déclenche une conversion, relâche le mutex pendant l'attente (voir
// docs/firmware.md, "Le mutex I2C ne couvre pas l'attente de conversion"),
// puis lit les 5 octets à partir de 0x06 et met à jour g_last_pressure.
void read_pressure() {
  const uint8_t trigger[2] = {kXdb401RegTrigger, kXdb401TriggerValue};

  xSemaphoreTake(g_i2c_mutex, portMAX_DELAY);
  esp_err_t err = i2c_master_transmit(g_xdb401_dev, trigger, sizeof(trigger), pdMS_TO_TICKS(100));
  xSemaphoreGive(g_i2c_mutex);

  if (err != ESP_OK) {
    send_log(common::LogCode::kI2cError, common::LogSeverity::kError, kXdb401Addr);
    g_last_pressure.flags &= ~0x01;  // capteur invalide (I2C injoignable)
    return;
  }

  // Attente de fin de conversion par scrutation du bit Sco (bit 3 du
  // registre 0x30) plutôt qu'un délai fixe — voir datasheet XDB401,
  // "I2C Digital Output Read Process" : le délai fixe de 50 ms donnait une
  // pression aberrante d'une lecture à l'autre (température stable, donc
  // pas un problème de bus, plutôt une conversion pression pas toujours
  // terminée à 50 ms).
  bool conversion_done = false;
  for (int attempt = 0; attempt < 10; ++attempt) {
    vTaskDelay(pdMS_TO_TICKS(kXdb401ConversionDelayMs / 10));
    uint8_t status = 0;
    const uint8_t reg_status = kXdb401RegTrigger;
    xSemaphoreTake(g_i2c_mutex, portMAX_DELAY);
    esp_err_t status_err = i2c_master_transmit(g_xdb401_dev, &reg_status, 1, pdMS_TO_TICKS(100));
    if (status_err == ESP_OK) {
      status_err = i2c_master_receive(g_xdb401_dev, &status, 1, pdMS_TO_TICKS(100));
    }
    xSemaphoreGive(g_i2c_mutex);
    if (status_err == ESP_OK && (status & 0x08) == 0) {
      conversion_done = true;
      break;
    }
  }
  if (!conversion_done) {
    vTaskDelay(pdMS_TO_TICKS(kXdb401ConversionDelayMs));  // repli : délai fixe si Sco ne retombe jamais
  }

  // Deux transactions séparées par registre, chacune écriture-pointeur puis
  // lecture avec son propre STOP (pas de repeated-start) — reproduit
  // exactement l'exemple du fabricant (datasheet XDB401, "I2C_ReadNByte
  // (0x06, Pressure, 3)" puis "I2C_ReadNByte(0x09, Temp, 2)" en deux appels
  // distincts, pas une rafale unique de 5 octets 0x06..0x0A). Une rafale
  // unique donnait une pression aberrante d'une lecture à l'autre alors
  // que la température, elle, restait cohérente.
  uint8_t pressure_bytes[3] = {0};
  uint8_t temp_bytes[2] = {0};
  const uint8_t reg_pressure = kXdb401RegPressure;
  const uint8_t reg_temp = kXdb401RegTemperature;
  xSemaphoreTake(g_i2c_mutex, portMAX_DELAY);
  err = i2c_master_transmit(g_xdb401_dev, &reg_pressure, 1, pdMS_TO_TICKS(100));
  if (err == ESP_OK) {
    err = i2c_master_receive(g_xdb401_dev, pressure_bytes, sizeof(pressure_bytes), pdMS_TO_TICKS(100));
  }
  if (err == ESP_OK) {
    err = i2c_master_transmit(g_xdb401_dev, &reg_temp, 1, pdMS_TO_TICKS(100));
  }
  if (err == ESP_OK) {
    err = i2c_master_receive(g_xdb401_dev, temp_bytes, sizeof(temp_bytes), pdMS_TO_TICKS(100));
  }
  xSemaphoreGive(g_i2c_mutex);

  if (err != ESP_OK) {
    send_log(common::LogCode::kXdb401Timeout, common::LogSeverity::kError);
    g_last_pressure.flags = static_cast<uint8_t>(g_last_pressure.flags | 0x02);  // timeout
    g_last_pressure.flags &= ~0x01;  // capteur invalide (conversion jamais terminée)
    return;
  }

  // Les octets partent tels quels sur le CAN (voir docs/firmware.md,
  // "XDB401 — pression et température") : reconstruits ici en little-endian
  // pour que StatusPressurePayload::pack() les réémette dans le même ordre
  // que la lecture I2C, sans réinterprétation — la calibration vit côté
  // écran.
  g_last_pressure.pressure_raw = static_cast<uint32_t>(pressure_bytes[0]) |
                                  (static_cast<uint32_t>(pressure_bytes[1]) << 8) |
                                  (static_cast<uint32_t>(pressure_bytes[2]) << 16);
  g_last_pressure.temperature_raw = static_cast<uint16_t>(temp_bytes[0]) | (static_cast<uint16_t>(temp_bytes[1]) << 8);
  g_last_pressure.flags = 0x01;  // capteur valide, pas de timeout
}

void send_status_pressure() {
  g_last_pressure.timestamp_ms = static_cast<uint16_t>((now_us() / 1000) & 0xFFFF);
  common::Frame f = g_last_pressure.pack();
  send_message(common::MessageType::kStatusPressure, common::Dest::kScreen, f.data(), 8);
}

void send_status_flow() {
  common::StatusFlowPayload payload;
  payload.pulse_count = g_flow_pulse_count;
  payload.last_edge_ms = static_cast<uint16_t>((g_flow_last_edge_us / 1000) & 0xFFFF);
  // bit0 toujours 1 : une entrée GPIO ne permet pas de détecter l'absence
  // du débitmètre, voir le commentaire de StatusFlowPayload::flags.
  payload.flags = 0x01;
  common::Frame f = payload.pack();
  send_message(common::MessageType::kStatusFlow, common::Dest::kScreen, f.data(), 7);
}

void mark_presence() {
  bool was_lost = g_presence_lost;
  g_last_presence_rx_us = now_us();
  g_presence_lost = false;
  if (was_lost) {
    ESP_LOGI(kTag, "présence retrouvée");
  }
}

void on_ping_received() {
  mark_presence();
  send_pong();
}

void on_pong_received() { mark_presence(); }

void on_reset_received() {
  send_log(common::LogCode::kRebootRequested, common::LogSeverity::kInfo);
  vTaskDelay(pdMS_TO_TICKS(50));  // laisser partir le LOG avant le reboot
  esp_restart();
}

void on_set_received(const uint8_t* data, size_t len) {
  common::SetPayload payload;
  if (!common::SetPayload::unpack(data, len, &payload)) {
    return;
  }
  if (s_lockout_active) {
    send_log(common::LogCode::kCommandRefusedLocked, common::LogSeverity::kWarn);
    send_status_actuators();
    return;
  }
  if (payload.set_ssr) {
    g_ssr = payload.ssr;
  }
  if (payload.set_dimmer) {
    g_dimmer = payload.dimmer;
  }
  apply_ssr();
  apply_dimmer();
  int64_t ttl_us = (payload.ttl_ms == 0 ? kLeaseDefaultUs : static_cast<int64_t>(payload.ttl_ms) * 1000);
  g_lease_deadline_us = now_us() + ttl_us;
  send_status_actuators();
}

void on_stop_received() {
  force_actuators_off();
  send_status_actuators();
}

void on_reqstatus_received(const uint8_t* data, size_t len) {
  common::ReqStatusPayload payload;
  if (!common::ReqStatusPayload::unpack(data, len, &payload)) {
    return;
  }
  switch (payload.target_type) {
    case common::MessageType::kStatusPressure:
      if (payload.period_ms == 0) {
        g_pressure_period_ms = 0;
      } else {
        g_pressure_period_ms = payload.period_ms < kPressurePeriodFloorMs ? kPressurePeriodFloorMs
                                                                            : payload.period_ms;
      }
      g_pressure_last_sent_us = 0;  // publie dès le prochain tick, pas d'attente d'une période complète
      break;
    case common::MessageType::kStatusFlow:
      g_flow_period_ms = payload.period_ms;
      g_flow_last_sent_us = 0;  // publie dès le prochain tick, pas d'attente d'une période complète
      break;
    default:
      // STATUS_ACTUATORS part déjà à chaque SET/STOP traité, pas de
      // streaming périodique dédié.
      break;
  }
}

void send_flash_ack(common::FlashSubCmd subcmd, uint16_t block_number, uint16_t block_crc16) {
  common::FlashCtrlPayload payload;
  payload.subcmd = subcmd;
  payload.block_number = block_number;
  payload.block_crc16 = block_crc16;
  common::Frame f = payload.pack();
  send_message(common::MessageType::kFlashCtrl, common::Dest::kScreen, f.data(), 8);
}

// Remet l'état de flash à zéro sans toucher au handle OTA (à faire avant, si
// un handle est ouvert).
void reset_flash_state() {
  g_flash.partition = nullptr;
  g_flash.ota_handle = 0;
  g_flash.image_size = 0;
  g_flash.bytes_written = 0;
  g_flash.block_number = 0;
  g_flash.block_buf_len = 0;
  g_flash.image_crc = common::Crc32Incremental{};
}

// Abandonne un transfert en cours (BEGIN suivi d'un autre BEGIN, ABORT
// explicite, ou échec de vérification en END).
void flash_abort(const char* reason) {
  if (g_flash.partition != nullptr) {
    esp_ota_abort(g_flash.ota_handle);
    ESP_LOGW(kTag, "flash abandonné : %s", reason);
  }
  reset_flash_state();
}

void on_flash_begin(uint32_t image_size) {
  if (g_flash.partition != nullptr) {
    flash_abort("nouveau BEGIN reçu avant la fin du précédent transfert");
  }

  const esp_partition_t* partition = esp_ota_get_next_update_partition(nullptr);
  if (partition == nullptr || image_size == 0 || image_size > partition->size) {
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, image_size);
    return;
  }

  // esp_ota_begin() efface les secteurs nécessaires à `image_size` avant de
  // renvoyer — c'est l'effacement exigé avant d'acquitter le BEGIN (voir
  // docs/firmware-implementation.md, phase 4, point 1).
  esp_ota_handle_t handle;
  esp_err_t err = esp_ota_begin(partition, image_size, &handle);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_begin échec: %s", esp_err_to_name(err));
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, image_size);
    return;
  }

  reset_flash_state();
  g_flash.partition = partition;
  g_flash.ota_handle = handle;
  g_flash.image_size = image_size;

  send_log(common::LogCode::kFlashBegin, common::LogSeverity::kInfo, 0, image_size);
  send_flash_ack(common::FlashSubCmd::kBlockAck, 0, 0);
}

void on_flash_end(uint32_t expected_crc32) {
  if (g_flash.partition == nullptr) {
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }
  // Bloc final incomplet : END n'aurait pas dû arriver avant le dernier
  // BLOCK_ACK. Traité comme un échec, pas rattrapable ici.
  if (g_flash.block_buf_len != 0 || g_flash.bytes_written != g_flash.image_size ||
      g_flash.image_crc.finish() != expected_crc32) {
    flash_abort("CRC32 global ou taille reçue incohérente au END");
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, g_flash.bytes_written);
    return;
  }

  esp_err_t err = esp_ota_end(g_flash.ota_handle);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_end échec: %s", esp_err_to_name(err));
    reset_flash_state();
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }
  err = esp_ota_set_boot_partition(g_flash.partition);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_set_boot_partition échec: %s", esp_err_to_name(err));
    reset_flash_state();
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError);
    return;
  }

  reset_flash_state();
  send_log(common::LogCode::kFlashDone, common::LogSeverity::kInfo);
  // flash_client.py n'attend qu'un FLASH_CTRL de sous-commande END en retour
  // (le contenu importe peu) pour considérer le transfert confirmé.
  send_flash_ack(common::FlashSubCmd::kEnd, 0, 0);

  vTaskDelay(pdMS_TO_TICKS(50));  // laisser partir le LOG/ACK avant le reboot
  esp_restart();
}

void on_flash_ctrl_received(const uint8_t* data, size_t len) {
  common::FlashCtrlPayload payload;
  if (!common::FlashCtrlPayload::unpack(data, len, &payload)) {
    return;
  }
  switch (payload.subcmd) {
    case common::FlashSubCmd::kBegin:
      on_flash_begin(payload.image_size);
      break;
    case common::FlashSubCmd::kEnd:
      on_flash_end(payload.image_crc32);
      break;
    case common::FlashSubCmd::kAbort:
      if (g_flash.partition != nullptr) {
        flash_abort("ABORT reçu");
        send_log(common::LogCode::kFlashFailed, common::LogSeverity::kWarn);
      }
      break;
    case common::FlashSubCmd::kBlockAck:
      // BLOCK_ACK n'est émis que par le récepteur (nous) ; rien à faire si
      // on le reçoit (écran mal aligné sur son propre rôle).
      break;
  }
}

// FLASH_DATA (0x39) : la trame CAN entière (jusqu'à 8 octets) est la
// donnée, aucun en-tête. Écrit au fil de l'eau, jamais l'image entière en
// RAM — voir docs/firmware-implementation.md, phase 4, point 2.
//
// Limite connue : si un bloc est rejoué par flash_client.py après un
// BLOCK_ACK dont le CRC ne correspondait pas à ce qu'il a envoyé, ce
// firmware n'a aucun moyen de distinguer ce rejeu d'un bloc suivant — les
// octets rejoués sont alors traités comme la suite de l'image, ce qui la
// corrompt. Le CRC16 par bloc protège contre une corruption silencieuse
// (le transfert échouera au CRC32 global du END plutôt que de flasher une
// image fausse), mais ne permet pas un vrai rattrapage bloc par bloc tant
// que le protocole n'a pas de signal explicite de rejeu. Non exercé sur le
// vrai bus avant cette session : le CAN a son propre CRC/ACK matériel, donc
// le cas ne devrait se déclencher qu'en cas de bug logiciel, pas de bruit
// électrique — mais c'est un point ouvert, pas une garantie.
void on_flash_data_received(const uint8_t* data, size_t len) {
  if (g_flash.partition == nullptr || len == 0) {
    return;
  }

  size_t remaining_in_image = g_flash.image_size - g_flash.bytes_written - g_flash.block_buf_len;
  size_t to_copy = len < remaining_in_image ? len : remaining_in_image;
  if (to_copy == 0) {
    return;
  }
  std::memcpy(&g_flash.block_buf[g_flash.block_buf_len], data, to_copy);
  g_flash.block_buf_len += to_copy;

  size_t bytes_left_total = g_flash.image_size - g_flash.bytes_written;
  size_t expected_block_size = bytes_left_total < kFlashBlockSize ? bytes_left_total : kFlashBlockSize;
  if (g_flash.block_buf_len < expected_block_size) {
    return;
  }

  esp_err_t err = esp_ota_write(g_flash.ota_handle, g_flash.block_buf, g_flash.block_buf_len);
  if (err != ESP_OK) {
    ESP_LOGW(kTag, "esp_ota_write échec: %s", esp_err_to_name(err));
    flash_abort("esp_ota_write échec");
    send_log(common::LogCode::kFlashFailed, common::LogSeverity::kError, 0, g_flash.bytes_written);
    return;
  }
  g_flash.image_crc.update(g_flash.block_buf, g_flash.block_buf_len);
  uint16_t block_crc16 = common::crc16_ccitt(g_flash.block_buf, g_flash.block_buf_len);
  g_flash.bytes_written += g_flash.block_buf_len;
  g_flash.block_buf_len = 0;
  g_flash.block_number += 1;

  send_log(common::LogCode::kFlashProgress, common::LogSeverity::kDebug, 0, g_flash.bytes_written);
  send_flash_ack(common::FlashSubCmd::kBlockAck, g_flash.block_number, block_crc16);
}

void dispatch_frame(const twai_message_t& msg) {
  common::CanId id = common::decode_can_id(static_cast<uint16_t>(msg.identifier));
  if (!common::is_known_message_type(static_cast<uint8_t>(id.type))) {
    return;
  }
  // On n'écoute que ce qui nous est adressé ou en broadcast, et qui vient de
  // l'écran (self-filtering : TWAI ne boucle pas nos propres trames, mais un
  // troisième émetteur hypothétique serait ignoré ici aussi).
  if (id.dest != common::Dest::kBroadcast && id.dest != common::Dest::kSensors) {
    return;
  }
  if (id.src != common::Node::kScreen) {
    return;
  }

  switch (id.type) {
    case common::MessageType::kPing:
      on_ping_received();
      break;
    case common::MessageType::kPong:
      on_pong_received();
      break;
    case common::MessageType::kReset:
      on_reset_received();
      break;
    case common::MessageType::kStop:
      on_stop_received();
      break;
    case common::MessageType::kSet:
      on_set_received(msg.data, msg.data_length_code);
      break;
    case common::MessageType::kReqStatus:
      on_reqstatus_received(msg.data, msg.data_length_code);
      break;
    case common::MessageType::kFlashCtrl:
      on_flash_ctrl_received(msg.data, msg.data_length_code);
      break;
    case common::MessageType::kFlashData:
      on_flash_data_received(msg.data, msg.data_length_code);
      break;
    default:
      // STATUS_* venant du bus : rien à faire ici avant la phase 5.
      break;
  }
}

void twai_rx_task(void*) {
  for (;;) {
    twai_message_t msg;
    if (twai_receive(&msg, portMAX_DELAY) == ESP_OK) {
      dispatch_frame(msg);
    }
  }
}

// Bail : coupe les actionneurs si le SET le plus récent a expiré.
void tick_lease() {
  if (g_lease_deadline_us == 0) return;
  if (now_us() < g_lease_deadline_us) return;
  if (!g_ssr && g_dimmer == 0) {
    g_lease_deadline_us = 0;
    return;
  }
  force_actuators_off();
  send_log(common::LogCode::kLeaseExpired, common::LogSeverity::kWarn);
  send_status_actuators();
}

// Présence : pingue si silence depuis kPresenceTimeoutUs, et coupe les
// actionneurs (repli sécurité) tant que personne ne répond.
void tick_presence() {
  int64_t t = now_us();
  if (t - g_last_presence_rx_us > kPresenceTimeoutUs) {
    if (!g_presence_lost) {
      g_presence_lost = true;
      force_actuators_off();
      send_log(common::LogCode::kPresenceLost, common::LogSeverity::kWarn);
    }
    if (t - g_last_own_ping_us > kPresencePingIntervalUs) {
      g_last_own_ping_us = t;
      send_message(common::MessageType::kPing, common::Dest::kBroadcast, nullptr, 0);
    }
  }
}

// Plafond 60 s — voir docs/firmware.md §3. Le compteur de marche continue
// n'est remis à zéro que par un arrêt d'au moins kRuntimeRearmGapUs : un
// écran qui commuterait juste avant l'échéance ne rearme rien.
void tick_runtime_lockout() {
  bool running = g_ssr || g_dimmer > 0;
  int64_t t = now_us();

  if (running) {
    if (g_off_since_us != 0 && (t - g_off_since_us) >= kRuntimeRearmGapUs) {
      g_run_start_us = t;  // coupure assez longue : nouveau cycle
    } else if (g_run_start_us == 0) {
      g_run_start_us = t;
    }
    g_off_since_us = 0;

    if (!s_lockout_active && g_run_start_us != 0 && (t - g_run_start_us) > kRuntimeLockoutUs) {
      uint32_t activated_ms = static_cast<uint32_t>((t - g_run_start_us) / 1000);
      s_lockout_active = true;
      force_actuators_off();
      send_log(common::LogCode::kRuntimeLockoutTriggered, common::LogSeverity::kError, 0, activated_ms);
      send_status_actuators();
    }
  } else {
    if (g_off_since_us == 0) {
      g_off_since_us = t;
    } else if ((t - g_off_since_us) >= kRuntimeRearmGapUs) {
      g_run_start_us = 0;
    }
  }
}

// Compteurs d'erreur TWAI — seul diagnostic de câblage sans oscilloscope,
// voir docs/firmware.md. Republiés périodiquement en LOG.
void tick_twai_counters() {
  static int64_t s_last_log_us = 0;
  int64_t t = now_us();
  if (t - s_last_log_us < kTwaiCountersLogPeriodUs) return;
  s_last_log_us = t;

  twai_status_info_t status;
  if (twai_get_status_info(&status) != ESP_OK) return;

  uint16_t arg16 = static_cast<uint16_t>(((status.rx_error_counter & 0xFF) << 8) | (status.tx_error_counter & 0xFF));
  send_log(common::LogCode::kTwaiErrorCounters, common::LogSeverity::kDebug, arg16, status.bus_error_count);
}

// Temporisateur d'invalidation OTA — voir docs/firmware-implementation.md,
// phase 4, point 3 : une image en PENDING_VERIFY qui ne reçoit jamais de
// PING/PONG doit rollback elle-même, IDF ne le fait pas tout seul.
// `mark_presence()` (donc `!g_presence_lost`) sert de preuve : n'importe
// quel PING/PONG reçu de l'écran/pont confirme que l'image tourne assez
// pour parler sur le bus.
void tick_ota_validation() {
  if (!g_ota_pending_verify) return;

  if (!g_presence_lost) {
    esp_ota_mark_app_valid_cancel_rollback();
    g_ota_pending_verify = false;
    send_log(common::LogCode::kOtaValidated, common::LogSeverity::kInfo);
    return;
  }

  if (now_us() - g_ota_pending_since_us > kOtaValidationTimeoutUs) {
    send_log(common::LogCode::kOtaRollback, common::LogSeverity::kError);
    vTaskDelay(pdMS_TO_TICKS(50));  // laisser partir le LOG avant le reboot
    esp_ota_mark_app_invalid_rollback_and_reboot();
    // N'atteint ce point que si l'appel ci-dessus a échoué (pas d'image
    // précédente valide, p.ex.) : pas de seconde tentative, la carte reste
    // en PENDING_VERIFY jusqu'au prochain reset.
    ESP_LOGE(kTag, "esp_ota_mark_app_invalid_rollback_and_reboot a échoué");
    g_ota_pending_verify = false;
  }
}

// Débitmètre : lecture d'un compteur incrémenté par ISR, non bloquante —
// contrairement à la pression (I2C + délai de conversion), pas besoin d'une
// tâche dédiée, un tick de plus dans safety_task suffit.
void tick_flow() {
  if (g_flow_period_ms == 0) return;
  int64_t t = now_us();
  if (t - g_flow_last_sent_us < static_cast<int64_t>(g_flow_period_ms) * 1000) return;
  send_status_flow();
  g_flow_last_sent_us = t;
}

void safety_task(void*) {
  for (;;) {
    tick_lease();
    tick_presence();
    tick_runtime_lockout();
    tick_twai_counters();
    tick_ota_validation();
    tick_flow();
    tick_dimmer_health();
    vTaskDelay(pdMS_TO_TICKS(kTickPeriodUs / 1000));
  }
}

// Tâche dédiée plutôt qu'un tick de plus dans safety_task : la lecture XDB401
// bloque ~kXdb401ConversionDelayMs (50 ms) sans tenir le mutex I2C, ce qui
// serait un délai grossier pour la boucle bail/présence/verrou (100 ms).
void pressure_task(void*) {
  for (;;) {
    if (g_pressure_period_ms == 0) {
      vTaskDelay(pdMS_TO_TICKS(200));  // au repos, revérifie périodiquement si un REQSTATUS est arrivé
      continue;
    }
    int64_t t = now_us();
    if (t - g_pressure_last_sent_us < static_cast<int64_t>(g_pressure_period_ms) * 1000) {
      vTaskDelay(pdMS_TO_TICKS(10));
      continue;
    }
    read_pressure();
    send_status_pressure();
    g_pressure_last_sent_us = now_us();
  }
}

void init_lockout_state() {
  esp_reset_reason_t reason = esp_reset_reason();
  if (reason == ESP_RST_POWERON || s_rtc_magic != kRtcMagic) {
    // Démarrage à froid, ou mémoire RTC jamais initialisée : seul cas où le
    // verrou 60 s se lève. Voir docs/firmware.md §3.
    s_rtc_magic = kRtcMagic;
    s_lockout_active = false;
  }
  // Tout autre motif de reset (logiciel, watchdog, brownout...) : le verrou
  // déjà posé, le cas échéant, reste posé.
}

}  // namespace

extern "C" void app_main() {
  // GPIO 10 tenu bas avant toute autre initialisation, y compris le CAN —
  // voir docs/firmware.md, "Le SSR est bas au boot, avant toute
  // initialisation du CAN."
  gpio_config_t ssr_cfg{};
  ssr_cfg.pin_bit_mask = 1ULL << kGpioSsr;
  ssr_cfg.mode = GPIO_MODE_OUTPUT;
  ssr_cfg.pull_up_en = GPIO_PULLUP_DISABLE;
  ssr_cfg.pull_down_en = GPIO_PULLDOWN_DISABLE;
  ssr_cfg.intr_type = GPIO_INTR_DISABLE;
  gpio_config(&ssr_cfg);
  gpio_set_level(kGpioSsr, 0);

  init_lockout_state();

  // PENDING_VERIFY : voir docs/firmware-implementation.md, phase 4, point 3.
  // Ne jamais valider l'image tout de suite ici — tick_ota_validation() ne
  // le fait qu'après un PING/PONG confirmé sur le bus, ou rollback au bout
  // de kOtaValidationTimeoutUs.
  const esp_partition_t* running = esp_ota_get_running_partition();
  esp_ota_img_states_t ota_state;
  if (running != nullptr && esp_ota_get_state_partition(running, &ota_state) == ESP_OK &&
      ota_state == ESP_OTA_IMG_PENDING_VERIFY) {
    g_ota_pending_verify = true;
    g_ota_pending_since_us = now_us();
  }

  twai_general_config_t g_config = TWAI_GENERAL_CONFIG_DEFAULT(kGpioCanTx, kGpioCanRx, TWAI_MODE_NORMAL);
  twai_timing_config_t t_config = TWAI_TIMING_CONFIG_500KBITS();
  twai_filter_config_t f_config = TWAI_FILTER_CONFIG_ACCEPT_ALL();
  ESP_ERROR_CHECK(twai_driver_install(&g_config, &t_config, &f_config));
  ESP_ERROR_CHECK(twai_start());

  int64_t t0 = now_us();
  g_last_presence_rx_us = t0 - kPresenceTimeoutUs;  // en attente, pas encore vu de pair
  g_last_own_ping_us = 0;

  send_log(common::LogCode::kBoot, common::LogSeverity::kInfo);
  if (g_ota_pending_verify) {
    send_log(common::LogCode::kOtaPendingVerify, common::LogSeverity::kInfo);
  }
  ESP_LOGI(kTag, "boot, version %d.%d.%d, verrou=%s", common::kFirmwareVersionMajor,
           common::kFirmwareVersionMinor, common::kFirmwareVersionPatch,
           s_lockout_active ? "actif" : "libre");

  init_i2c();
  init_flow();

  xTaskCreate(twai_rx_task, "twai_rx", 4096, nullptr, 10, nullptr);
  xTaskCreate(safety_task, "safety", 4096, nullptr, 5, nullptr);
  xTaskCreate(pressure_task, "pressure", 4096, nullptr, 4, nullptr);

  send_log(common::LogCode::kReady, common::LogSeverity::kInfo);
}

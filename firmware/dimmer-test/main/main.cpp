// Firmware de bring-up jetable pour le dimmer RBDimmer/DimmerLink — voir
// docs/firmware.md, "Dimmer — pompe", et tests/test_rbi2c.py (protocole de
// référence, testé sur un autre banc avec un autre brochage). Pas de CAN, pas
// de machine de sécurité : juste une boucle I2C rapide sur le même brochage
// que firmware/sensors (GPIO5 SDA / GPIO6 SCL), pour itérer sans les
// contraintes de présence/bail du firmware réel. Jetable : à reflasher par
// sensors une fois le bring-up terminé.

#include "driver/i2c_master.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

namespace {

constexpr const char* kTag = "dimmer_test";

constexpr gpio_num_t kGpioI2cSda = GPIO_NUM_5;  // D4, même brochage que sensors
constexpr gpio_num_t kGpioI2cScl = GPIO_NUM_6;  // D5

constexpr uint16_t kDimmerAddr = 0x50;
constexpr uint8_t kRegStatus = 0x00;
constexpr uint8_t kRegCommand = 0x01;
constexpr uint8_t kRegError = 0x02;
constexpr uint8_t kRegVersion = 0x03;
constexpr uint8_t kRegLevel = 0x10;
constexpr uint8_t kRegCurve = 0x11;
constexpr uint8_t kRegFreq = 0x20;
constexpr uint8_t kRegCalibration = 0x23;

constexpr uint8_t kCmdReset = 0x01;
constexpr uint8_t kCmdRecalibrate = 0x02;

i2c_master_bus_handle_t g_bus = nullptr;
i2c_master_dev_handle_t g_dimmer = nullptr;

// Transaction combinée (write pointeur + read en repeated-start), comme
// i2c.readfrom_mem() côté MicroPython (tests/test_rbi2c.py).
esp_err_t read_reg(uint8_t reg, uint8_t* value) {
  return i2c_master_transmit_receive(g_dimmer, &reg, 1, value, 1, pdMS_TO_TICKS(200));
}

esp_err_t write_reg(uint8_t reg, uint8_t value) {
  const uint8_t buf[2] = {reg, value};
  return i2c_master_transmit(g_dimmer, buf, sizeof(buf), pdMS_TO_TICKS(200));
}

void scan_bus() {
  ESP_LOGI(kTag, "scan I2C...");
  for (uint8_t addr = 1; addr < 0x78; ++addr) {
    i2c_device_config_t cfg{};
    cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
    cfg.device_address = addr;
    cfg.scl_speed_hz = 100000;
    i2c_master_dev_handle_t dev;
    if (i2c_master_bus_add_device(g_bus, &cfg, &dev) != ESP_OK) continue;
    esp_err_t err = i2c_master_probe(g_bus, addr, 50);
    i2c_master_bus_rm_device(dev);
    if (err == ESP_OK) {
      ESP_LOGI(kTag, "  trouvé 0x%02x", addr);
    }
  }
  ESP_LOGI(kTag, "scan terminé");
}

void print_all_regs(const char* label) {
  uint8_t status = 0xFF, error = 0xFF, curve = 0xFF, freq = 0xFF, calib = 0xFF, version = 0xFF;
  esp_err_t e_status = read_reg(kRegStatus, &status);
  esp_err_t e_error = read_reg(kRegError, &error);
  esp_err_t e_curve = read_reg(kRegCurve, &curve);
  esp_err_t e_freq = read_reg(kRegFreq, &freq);
  esp_err_t e_calib = read_reg(kRegCalibration, &calib);
  esp_err_t e_version = read_reg(kRegVersion, &version);
  ESP_LOGI(kTag,
           "%-12s status=0x%02x(ready=%d,err=%d) error=0x%02x curve=0x%02x freq=%d calib=%d(%s) version=0x%02x",
           label, status, status & 0x01, (status >> 1) & 0x01, error, curve, freq, calib,
           esp_err_to_name(e_calib), version);
  (void)e_status;
  (void)e_error;
  (void)e_curve;
  (void)e_freq;
  (void)e_version;
}

}  // namespace

extern "C" void app_main() {
  i2c_master_bus_config_t bus_cfg{};
  bus_cfg.i2c_port = -1;
  bus_cfg.sda_io_num = kGpioI2cSda;
  bus_cfg.scl_io_num = kGpioI2cScl;
  bus_cfg.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_cfg.glitch_ignore_cnt = 7;
  bus_cfg.flags.enable_internal_pullup = false;  // pull-ups déjà sur le XDB401
  ESP_ERROR_CHECK(i2c_new_master_bus(&bus_cfg, &g_bus));

  scan_bus();

  i2c_device_config_t dev_cfg{};
  dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  dev_cfg.device_address = kDimmerAddr;
  dev_cfg.scl_speed_hz = 100000;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(g_bus, &dev_cfg, &g_dimmer));

  ESP_LOGI(kTag, "prêt");
  print_all_regs("boot");

  // Pas de RECALIBRATE ici : test de convergence naturelle après un vrai
  // power-cycle (5V + secteur coupés puis rebranchés), pour savoir si le
  // module calibre tout seul sans commande explicite — voir la discussion
  // bring-up 2026-09-09 (on ne sait pas si le RECALIBRATE précédent a réglé
  // quelque chose ou si le temps écoulé aurait suffi de toute façon).
  ESP_LOGI(kTag, "-- observation passive, pas de RECALIBRATE --");
  for (int i = 0; i < 60; ++i) {
    vTaskDelay(pdMS_TO_TICKS(500));
    char label[16];
    snprintf(label, sizeof(label), "+%dms", (i + 1) * 500);
    print_all_regs(label);
  }

  ESP_LOGI(kTag, "prêt, boucle de test");

  const uint8_t levels[] = {0, 15, 30, 50, 75, 100, 0};
  for (;;) {
    for (uint8_t level : levels) {
      esp_err_t werr = write_reg(kRegLevel, level);
      ESP_LOGI(kTag, "== écriture level=%d (%s) ==", level, esp_err_to_name(werr));
      for (int i = 0; i < 5; ++i) {
        char label[16];
        snprintf(label, sizeof(label), "+%dms", i * 200);
        print_all_regs(label);
        vTaskDelay(pdMS_TO_TICKS(200));
      }
      vTaskDelay(pdMS_TO_TICKS(2000));
    }
  }
}

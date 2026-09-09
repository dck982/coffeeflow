#include "board.h"

#include "driver/i2c_master.h"
#include "esp_err.h"

namespace board {

namespace {

constexpr uint16_t kCh422gModeAddr = 0x24;
constexpr uint16_t kCh422gOutAddr = 0x38;
constexpr uint8_t kCh422gModeOutputs = 0x01;  // EXIO0-7 en push-pull

i2c_master_bus_handle_t g_bus = nullptr;
i2c_master_dev_handle_t g_mode_dev = nullptr;
i2c_master_dev_handle_t g_out_dev = nullptr;

// État maintenu en RAM du registre de sortie CH422G (docs/plan-phase6.md) :
// seule ch422g_set_bit() doit y toucher.
uint8_t g_ch422g_out_mirror = 0;

}  // namespace

void ch422g_init() {
  i2c_master_bus_config_t bus_config{};
  bus_config.i2c_port = -1;
  bus_config.sda_io_num = kI2cSda;
  bus_config.scl_io_num = kI2cScl;
  bus_config.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_config.glitch_ignore_cnt = 7;
  bus_config.flags.enable_internal_pullup = true;
  ESP_ERROR_CHECK(i2c_new_master_bus(&bus_config, &g_bus));

  i2c_device_config_t mode_dev_cfg{};
  mode_dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  mode_dev_cfg.device_address = kCh422gModeAddr;
  mode_dev_cfg.scl_speed_hz = 400000;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(g_bus, &mode_dev_cfg, &g_mode_dev));

  i2c_device_config_t out_dev_cfg{};
  out_dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  out_dev_cfg.device_address = kCh422gOutAddr;
  out_dev_cfg.scl_speed_hz = 400000;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(g_bus, &out_dev_cfg, &g_out_dev));

  uint8_t mode_val = kCh422gModeOutputs;
  ESP_ERROR_CHECK(i2c_master_transmit(g_mode_dev, &mode_val, 1, -1));

  // Miroir RAM initialisé à 0 : c'est l'état de sortie réel du CH422G avant
  // toute écriture de ce firmware.
  g_ch422g_out_mirror = 0;
}

void ch422g_set_bit(uint8_t bit, bool value) {
  if (value) {
    g_ch422g_out_mirror |= bit;
  } else {
    g_ch422g_out_mirror &= static_cast<uint8_t>(~bit);
  }
  ESP_ERROR_CHECK(i2c_master_transmit(g_out_dev, &g_ch422g_out_mirror, 1, -1));
}

void select_can() { ch422g_set_bit(kCh422gCanSel, true); }

}  // namespace board

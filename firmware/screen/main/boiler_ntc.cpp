#include "boiler_ntc.h"

#include <cstdint>

#include "board.h"
#include "core/core.h"
#include "driver/i2c_master.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

namespace boiler_ntc {
namespace {

constexpr char kTag[] = "boiler_ntc";
constexpr int kI2cTimeoutMs = 25;
constexpr uint32_t kPairPeriodMs = 100;
i2c_master_dev_handle_t g_device = nullptr;

bool ensure_device() {
  if (g_device != nullptr) return true;
  // ADDR peut être câblé à GND, VDD, SDA ou SCL. Aucun de ces quatre
  // emplacements n'est utilisé par le CH422G ou le GT911.
  for (uint16_t address = 0x48; address <= 0x4B; ++address) {
    if (i2c_master_probe(board::i2c_bus(), address, 10) != ESP_OK) continue;
    i2c_device_config_t config{};
    config.dev_addr_length = I2C_ADDR_BIT_LEN_7;
    config.device_address = address;
    config.scl_speed_hz = 400000;
    if (i2c_master_bus_add_device(board::i2c_bus(), &config, &g_device) == ESP_OK) {
      ESP_LOGI(kTag, "ADS1115 detected at 0x%02x", static_cast<unsigned>(address));
      return true;
    }
  }
  return false;
}

bool read_channel(uint16_t mux, int16_t* raw) {
  // OS=1, MUX=A0/A1-GND, PGA=±4,096 V, single-shot, 128 SPS,
  // comparateur désactivé. Les deux canaux utilisent le même PGA.
  const uint16_t config = static_cast<uint16_t>(0x8000 | (mux << 12) | 0x0200 | 0x0100 | 0x0080 | 0x0003);
  const uint8_t command[] = {0x01, static_cast<uint8_t>(config >> 8), static_cast<uint8_t>(config)};
  if (i2c_master_transmit(g_device, command, sizeof(command), kI2cTimeoutMs) != ESP_OK) return false;
  for (int attempt = 0; attempt < 15; ++attempt) {
    // Le tick FreeRTOS de cette image vaut 10 ms ; 2 ms donnerait 0 tick.
    vTaskDelay(pdMS_TO_TICKS(10));
    const uint8_t pointer = 0x01;
    uint8_t status[2]{};
    if (i2c_master_transmit_receive(g_device, &pointer, 1, status, sizeof(status), kI2cTimeoutMs) != ESP_OK)
      return false;
    if ((status[0] & 0x80) != 0) {
      const uint8_t conversion = 0x00;
      uint8_t bytes[2]{};
      if (i2c_master_transmit_receive(g_device, &conversion, 1, bytes, sizeof(bytes), kI2cTimeoutMs) != ESP_OK)
        return false;
      *raw = static_cast<int16_t>((static_cast<uint16_t>(bytes[0]) << 8) | bytes[1]);
      return true;
    }
  }
  return false;
}

void task(void*) {
  TickType_t next = xTaskGetTickCount();
  for (;;) {
    int16_t a0 = 0;
    int16_t a1 = 0;
    const bool ok = ensure_device() && read_channel(4, &a0) && read_channel(5, &a1);
    core::on_boiler_ntc_reading(a0, a1, ok);
    if (ok) {
      vTaskDelayUntil(&next, pdMS_TO_TICKS(kPairPeriodMs));
    } else {
      // Un ADS absent ne monopolise pas le bus du tactile et du CH422G.
      vTaskDelay(pdMS_TO_TICKS(1000));
      next = xTaskGetTickCount();
    }
  }
}

}  // namespace

void start() {
  if (board::i2c_bus() == nullptr) return;
  if (xTaskCreatePinnedToCore(task, "boiler_ntc", 3072, nullptr, 3, nullptr, 0) != pdPASS) {
    ESP_LOGW(kTag, "ADS1115 task creation failed");
  }
}

}  // namespace boiler_ntc

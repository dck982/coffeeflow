#include "boiler_ntc.h"

#include <cstdint>

#include "board.h"
#include "can_link.h"
#include "core/core.h"
#include "driver/i2c_master.h"
#include "esp_err.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

namespace boiler_ntc {
namespace {

constexpr char kTag[] = "boiler_ntc";
constexpr int kI2cTimeoutMs = 25;
constexpr uint32_t kPairPeriodMs = 100;
constexpr int64_t kRepeatedDiagnosticUs = 5 * 1000 * 1000;
i2c_master_dev_handle_t g_device = nullptr;
uint8_t g_device_address = 0;

struct Diagnostic {
  common::LogCode code = common::LogCode::kBoilerAdcNotFound;
  uint16_t arg16 = 0;
  uint32_t arg32 = 0;
};

void set_i2c_error(Diagnostic* diagnostic, uint8_t stage, uint8_t address, esp_err_t error) {
  diagnostic->code = common::LogCode::kBoilerAdcI2cError;
  diagnostic->arg16 = static_cast<uint16_t>((stage << 8) | address);
  diagnostic->arg32 = static_cast<uint32_t>(error);
}

bool ensure_device(Diagnostic* diagnostic) {
  if (g_device != nullptr) return true;
  // ADDR peut être câblé à GND, VDD, SDA ou SCL. Aucun de ces quatre
  // emplacements n'est utilisé par le CH422G ou le GT911.
  esp_err_t last_probe_error = ESP_FAIL;
  for (uint16_t address = 0x48; address <= 0x4B; ++address) {
    last_probe_error = i2c_master_probe(board::i2c_bus(), address, 10);
    if (last_probe_error != ESP_OK) continue;
    i2c_device_config_t config{};
    config.dev_addr_length = I2C_ADDR_BIT_LEN_7;
    config.device_address = address;
    config.scl_speed_hz = 400000;
    const esp_err_t error = i2c_master_bus_add_device(board::i2c_bus(), &config, &g_device);
    if (error == ESP_OK) {
      g_device_address = static_cast<uint8_t>(address);
      ESP_LOGI(kTag, "ADS1115 detected at 0x%02x", static_cast<unsigned>(address));
      return true;
    }
    set_i2c_error(diagnostic, 1, static_cast<uint8_t>(address), error);
    return false;
  }
  diagnostic->code = common::LogCode::kBoilerAdcNotFound;
  diagnostic->arg32 = static_cast<uint32_t>(last_probe_error);
  return false;
}

bool read_channel(uint16_t mux, int16_t* raw, Diagnostic* diagnostic) {
  const uint8_t stage_offset = mux == 4 ? 0 : 3;
  // OS=1, MUX=A0/A1-GND, PGA=±4,096 V, single-shot, 128 SPS,
  // comparateur désactivé. Les deux canaux utilisent le même PGA.
  const uint16_t config = static_cast<uint16_t>(0x8000 | (mux << 12) | 0x0200 | 0x0100 | 0x0080 | 0x0003);
  const uint8_t command[] = {0x01, static_cast<uint8_t>(config >> 8), static_cast<uint8_t>(config)};
  esp_err_t error = i2c_master_transmit(g_device, command, sizeof(command), kI2cTimeoutMs);
  if (error != ESP_OK) {
    set_i2c_error(diagnostic, 2 + stage_offset, g_device_address, error);
    return false;
  }
  for (int attempt = 0; attempt < 15; ++attempt) {
    // Le tick FreeRTOS de cette image vaut 10 ms ; 2 ms donnerait 0 tick.
    vTaskDelay(pdMS_TO_TICKS(10));
    const uint8_t pointer = 0x01;
    uint8_t status[2]{};
    error = i2c_master_transmit_receive(g_device, &pointer, 1, status, sizeof(status), kI2cTimeoutMs);
    if (error != ESP_OK) {
      set_i2c_error(diagnostic, 3 + stage_offset, g_device_address, error);
      return false;
    }
    if ((status[0] & 0x80) != 0) {
      const uint8_t conversion = 0x00;
      uint8_t bytes[2]{};
      error = i2c_master_transmit_receive(g_device, &conversion, 1, bytes, sizeof(bytes), kI2cTimeoutMs);
      if (error != ESP_OK) {
        set_i2c_error(diagnostic, 4 + stage_offset, g_device_address, error);
        return false;
      }
      *raw = static_cast<int16_t>((static_cast<uint16_t>(bytes[0]) << 8) | bytes[1]);
      return true;
    }
  }
  diagnostic->code = common::LogCode::kBoilerAdcConversionTimeout;
  diagnostic->arg16 = static_cast<uint16_t>(((mux == 4 ? 0 : 1) << 8) | g_device_address);
  return false;
}

void task(void*) {
  TickType_t next = xTaskGetTickCount();
  bool failure_active = false;
  Diagnostic last_diagnostic;
  int64_t last_diagnostic_us = 0;
  int64_t failure_started_us = 0;
  for (;;) {
    int16_t a0 = 0;
    int16_t a1 = 0;
    Diagnostic diagnostic;
    const bool ok = ensure_device(&diagnostic) && read_channel(4, &a0, &diagnostic) &&
                    read_channel(5, &a1, &diagnostic);
    const bool valid = core::on_boiler_ntc_reading(a0, a1, ok);
    if (ok && !valid) {
      diagnostic.code = common::LogCode::kBoilerNtcInvalidReading;
      diagnostic.arg16 = g_device_address;
      diagnostic.arg32 = (static_cast<uint32_t>(static_cast<uint16_t>(a0)) << 16) |
                         static_cast<uint16_t>(a1);
    }
    if (!valid) {
      const int64_t now = esp_timer_get_time();
      if (!failure_active) failure_started_us = now;
      const bool error_changed = diagnostic.code != common::LogCode::kBoilerNtcInvalidReading &&
                                 diagnostic.arg32 != last_diagnostic.arg32;
      if (!failure_active || diagnostic.code != last_diagnostic.code ||
          diagnostic.arg16 != last_diagnostic.arg16 || error_changed ||
          now - last_diagnostic_us >= kRepeatedDiagnosticUs) {
        can_link::send_log(diagnostic.code,
                           diagnostic.code == common::LogCode::kBoilerNtcInvalidReading
                               ? common::LogSeverity::kWarn : common::LogSeverity::kError,
                           diagnostic.arg16, diagnostic.arg32);
        last_diagnostic = diagnostic;
        last_diagnostic_us = now;
      }
      failure_active = true;
    } else {
      if (failure_active) {
        const uint32_t interruption_ms = static_cast<uint32_t>((esp_timer_get_time() - failure_started_us) / 1000);
        can_link::send_log(common::LogCode::kBoilerAdcRecovered, common::LogSeverity::kInfo,
                           g_device_address, interruption_ms);
      }
      failure_active = false;
    }
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
  if (board::i2c_bus() == nullptr) {
    can_link::send_log(common::LogCode::kBoilerAdcI2cError, common::LogSeverity::kError,
                       0, ESP_ERR_INVALID_STATE);
    return;
  }
  if (xTaskCreatePinnedToCore(task, "boiler_ntc", 3072, nullptr, 3, nullptr, 0) != pdPASS) {
    ESP_LOGW(kTag, "ADS1115 task creation failed");
  }
}

}  // namespace boiler_ntc

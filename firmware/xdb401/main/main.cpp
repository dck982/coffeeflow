// Sonde de pression : XDB401 I2C sur le port A du Core2, pour les essais au
// banc. Une mesure toutes les 100 ms, affichée, écrite en CSV sur la
// console USB (ms,raw,bar,celsius) et notifiée en BLE (ble_server).
//
// La sonde s'alimente en 3,3 V : ses pull-ups de 4,7 kΩ vont à son VCC, et
// les GPIO 32/33 du port A ne tolèrent pas 5 V. GPIO32 = SDA (vert),
// GPIO33 = SCL (blanc).
//
// Touche A : bascule le 5 V du port A (allumé au démarrage). Touche B : zéro
// sur la pression courante, appui long pour l'annuler. Touche C, maintenue
// une seconde : extinction.

#include <M5Unified.h>

#include "ble_server.h"
#include "driver/gpio.h"
#include "driver/i2c_master.h"

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <deque>

namespace {

constexpr char kFirmwareVersion[] = "0.1.0";

constexpr uint8_t kAddress = 0x7F;
constexpr uint8_t kRegCommand = 0x30;
constexpr uint8_t kCommandConvert = 0x0A;
// Bit Sco du registre 0x30 : conversion en cours.
constexpr uint8_t kBusyBit = 0x08;
constexpr uint8_t kRegPressure = 0x06;
constexpr uint8_t kRegTemperature = 0x09;
constexpr uint32_t kI2cHz = 100000;
// Pleine échelle recalée sur les purges du 17 septembre 2026.
constexpr float kFullScaleBar = 16.0f;

constexpr uint32_t kSamplePeriodMs = 100;
constexpr uint32_t kPollPeriodMs = 5;
// Au-delà, la mesure est lue quand même, comme l'ancien pilote.
constexpr uint32_t kConversionTimeoutMs = 50;
constexpr uint32_t kSearchRetryMs = 500;
constexpr uint32_t kPowerSettleMs = 50;
constexpr uint32_t kScanLogPeriodMs = 2000;
constexpr uint32_t kWindowMs = 5000;
// Sans mesure, une trame « pas de mesure » par seconde dit aux centraux que
// la sonde est là.
constexpr uint32_t kIdleFrameMs = 1000;
constexpr uint32_t kLoopMs = 5;
constexpr uint32_t kRenderPeriodMs = 250;
constexpr uint32_t kBatteryPeriodMs = 2000;
constexpr uint32_t kHoldMs = 1000;

enum class Phase { kOff, kSettling, kSearching, kIdle, kConverting };

struct Sample {
  int32_t raw = 0;
  float bar = NAN;
  float celsius = NAN;
};

struct WindowPoint {
  uint32_t ms;
  float bar;
};

Phase g_phase = Phase::kOff;
uint32_t g_phase_ms = 0;
uint32_t g_last_sample_ms = 0;
uint32_t g_last_poll_ms = 0;
uint32_t g_last_scan_log_ms = 0;
uint16_t g_frame_seq = 0;
uint32_t g_last_frame_ms = 0;
bool g_bus_open = false;
bool g_found = false;
bool g_has_sample = false;
Sample g_sample;
std::deque<WindowPoint> g_window;
uint32_t g_i2c_errors = 0;
float g_zero_bar = 0.0f;

bool g_power_on = false;
bool g_power_refused = false;
int32_t g_battery_pct = -1;
bool g_charging = false;
M5Canvas g_canvas(&M5.Display);

uint32_t now_ms() { return static_cast<uint32_t>(esp_timer_get_time() / 1000); }

void enter(Phase phase, uint32_t now) {
  g_phase = phase;
  g_phase_ms = now;
}

// Port A par le pilote i2c_master d'ESP-IDF, celui de l'ancien pilote de la
// sonde : par M5.Ex_I2C (M5GFX), la sonde ne renvoyait aucun ACK.
constexpr i2c_port_t kPort = I2C_NUM_0;
constexpr gpio_num_t kSda = GPIO_NUM_32;
constexpr gpio_num_t kScl = GPIO_NUM_33;
constexpr int kTimeoutMs = 50;

i2c_master_bus_handle_t g_bus = nullptr;
i2c_master_dev_handle_t g_dev = nullptr;
esp_err_t g_last_err = ESP_OK;

bool check(esp_err_t err) {
  if (err != ESP_OK) g_last_err = err;
  return err == ESP_OK;
}

bool open_bus() {
  i2c_master_bus_config_t bus_cfg{};
  bus_cfg.i2c_port = kPort;
  bus_cfg.sda_io_num = kSda;
  bus_cfg.scl_io_num = kScl;
  bus_cfg.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_cfg.glitch_ignore_cnt = 7;
  bus_cfg.flags.enable_internal_pullup = true;
  if (!check(i2c_new_master_bus(&bus_cfg, &g_bus))) {
    std::printf("# i2c_new_master_bus: %s\n", esp_err_to_name(g_last_err));
    return false;
  }
  i2c_device_config_t dev_cfg{};
  dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  dev_cfg.device_address = kAddress;
  dev_cfg.scl_speed_hz = kI2cHz;
  if (!check(i2c_master_bus_add_device(g_bus, &dev_cfg, &g_dev))) {
    std::printf("# i2c_master_bus_add_device: %s\n", esp_err_to_name(g_last_err));
    return false;
  }
  return true;
}

void close_bus() {
  if (g_dev != nullptr) i2c_master_bus_rm_device(g_dev);
  if (g_bus != nullptr) i2c_del_master_bus(g_bus);
  g_dev = nullptr;
  g_bus = nullptr;
}

// Pointeur puis lecture en deux transactions, comme l'exemple du fabricant.
bool read_bytes(uint8_t reg, uint8_t* out, size_t len) {
  return check(i2c_master_transmit(g_dev, &reg, 1, kTimeoutMs)) &&
         check(i2c_master_receive(g_dev, out, len, kTimeoutMs));
}

bool start_conversion() {
  const uint8_t bytes[] = {kRegCommand, kCommandConvert};
  return check(i2c_master_transmit(g_dev, bytes, sizeof(bytes), kTimeoutMs));
}

bool read_sample(Sample* s) {
  uint8_t p[3]{};
  uint8_t t[2]{};
  if (!read_bytes(kRegPressure, p, sizeof(p)) || !read_bytes(kRegTemperature, t, sizeof(t))) return false;
  int32_t raw = (static_cast<int32_t>(p[0]) << 16) | (static_cast<int32_t>(p[1]) << 8) | p[2];
  if (raw & 0x800000) raw -= 0x1000000;
  s->raw = raw;
  s->bar = static_cast<float>(raw) * kFullScaleBar / 8388608.0f;
  s->celsius = static_cast<float>(static_cast<int16_t>((t[0] << 8) | t[1])) / 256.0f;
  return true;
}

bool probe(uint8_t address) { return check(i2c_master_probe(g_bus, address, kTimeoutMs)); }

// Diagnostic pendant la recherche : niveaux des lignes au repos, réponse de
// 0x7F (ESP_ERR_NOT_FOUND = NACK, ESP_ERR_TIMEOUT = bus bloqué) et toutes
// les adresses qui répondent.
void log_bus_scan() {
  const int sda = gpio_get_level(kSda);
  const int scl = gpio_get_level(kScl);
  const esp_err_t err = i2c_master_probe(g_bus, kAddress, kTimeoutMs);
  std::printf("# scan SDA=GPIO%d:%d SCL=GPIO%d:%d, 0x%02X: %s, ack:", kSda, sda, kScl, scl, kAddress,
              esp_err_to_name(err));
  for (int address = 0x08; address <= 0x7F; ++address) {
    if (i2c_master_probe(g_bus, static_cast<uint8_t>(address), kTimeoutMs) == ESP_OK) std::printf(" 0x%02X", address);
  }
  std::printf("\n");
}

void on_i2c_error(uint32_t now) {
  ++g_i2c_errors;
  g_found = false;
  enter(Phase::kSearching, now);
}

void publish(uint32_t now, const Sample* s) {
  xdb401::Frame frame;
  frame.seq = g_frame_seq++;
  frame.ms = static_cast<uint16_t>(now);
  if (s != nullptr) {
    frame.raw = s->raw;
    frame.centi_c = static_cast<int16_t>(std::lround(s->celsius * 100.0f));
    frame.status = xdb401::Status::kOk;
  }
  ble_server::publish(frame);
  g_last_frame_ms = now;
}

void finish_sample(uint32_t now) {
  Sample s;
  if (!read_sample(&s)) return on_i2c_error(now);
  g_sample = s;
  g_has_sample = true;
  g_window.push_back({now, s.bar});
  while (!g_window.empty() && now - g_window.front().ms > kWindowMs) g_window.pop_front();
  std::printf("%lu,%ld,%.4f,%.2f\n", static_cast<unsigned long>(now), static_cast<long>(s.raw), s.bar, s.celsius);
  publish(now, &s);
  enter(Phase::kIdle, now);
}

void step_measurement(uint32_t now) {
  switch (g_phase) {
    case Phase::kOff:
      return;
    case Phase::kSettling:
      if (now - g_phase_ms < kPowerSettleMs) return;
      if (!g_bus_open) g_bus_open = open_bus();
      if (!g_bus_open) return;
      enter(Phase::kSearching, now - kSearchRetryMs);
      return;
    case Phase::kSearching:
      if (now - g_phase_ms < kSearchRetryMs) return;
      g_phase_ms = now;
      if (now - g_last_scan_log_ms >= kScanLogPeriodMs) {
        g_last_scan_log_ms = now;
        log_bus_scan();
      }
      if (g_bus_open && probe(kAddress)) {
        g_found = true;
        g_last_sample_ms = now - kSamplePeriodMs;
        enter(Phase::kIdle, now);
      }
      return;
    case Phase::kIdle:
      if (now - g_last_sample_ms < kSamplePeriodMs) return;
      g_last_sample_ms += kSamplePeriodMs;
      if (now - g_last_sample_ms >= kSamplePeriodMs) g_last_sample_ms = now;
      if (!start_conversion()) return on_i2c_error(now);
      g_last_poll_ms = now;
      enter(Phase::kConverting, now);
      return;
    case Phase::kConverting: {
      if (now - g_last_poll_ms < kPollPeriodMs) return;
      g_last_poll_ms = now;
      uint8_t status = 0;
      const bool polled = read_bytes(kRegCommand, &status, 1);
      if ((polled && (status & kBusyBit) == 0) || now - g_phase_ms >= kConversionTimeoutMs) finish_sample(now);
      return;
    }
  }
}

void power_on() {
  M5.Power.setExtOutput(true);
  // Le Core2 v1.1 refuse le 5 V sur batterie presque vide, sans USB.
  g_power_refused = !M5.Power.getExtOutput();
  if (g_power_refused) return;
  g_power_on = true;
  g_i2c_errors = 0;
  enter(Phase::kSettling, now_ms());
}

void power_off() {
  // Libérer le bus avant de couper, pour ne pas alimenter la sonde par SDA/SCL.
  if (g_bus_open) {
    close_bus();
    g_bus_open = false;
  }
  M5.Power.setExtOutput(false);
  g_power_on = false;
  g_found = false;
  g_has_sample = false;
  g_window.clear();
  enter(Phase::kOff, now_ms());
}

void draw_centered(const char* text, int y, const lgfx::IFont* font, uint16_t color) {
  g_canvas.setFont(font);
  g_canvas.setTextColor(color);
  g_canvas.setTextDatum(textdatum_t::top_center);
  g_canvas.drawString(text, g_canvas.width() / 2, y);
}

void draw_message(const char* line1, const char* line2) {
  draw_centered(line1, 80, &fonts::DejaVu24, TFT_WHITE);
  if (line2 != nullptr) draw_centered(line2, 120, &fonts::DejaVu18, TFT_LIGHTGREY);
}

void draw_header() {
  char text[32];
  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_LIGHTGREY);
  g_canvas.setTextDatum(textdatum_t::top_left);
  g_canvas.drawString("XDB401", 8, 4);
  const int version_x = 8 + g_canvas.textWidth("XDB401 ");
  g_canvas.setFont(&fonts::DejaVu12);
  g_canvas.drawString(kFirmwareVersion, version_x, 9);
  g_canvas.setFont(&fonts::DejaVu18);
  if (g_battery_pct >= 0) {
    std::snprintf(text, sizeof(text), g_charging ? "USB %ld%%" : "Bat %ld%%", static_cast<long>(g_battery_pct));
    g_canvas.setTextDatum(textdatum_t::top_right);
    g_canvas.drawString(text, g_canvas.width() - 8, 4);
  }
}

void draw_pressure() {
  char text[48];
  std::snprintf(text, sizeof(text), "%.2f", g_sample.bar - g_zero_bar);
  g_canvas.setFont(&fonts::DejaVu72);
  g_canvas.setTextColor(TFT_WHITE);
  g_canvas.setTextDatum(textdatum_t::top_right);
  const int right = 240;
  g_canvas.drawString(text, right, 34);
  g_canvas.setFont(&fonts::DejaVu40);
  g_canvas.setTextDatum(textdatum_t::top_left);
  g_canvas.drawString("bar", right + 8, 52);

  if (!g_window.empty()) {
    float lo = g_window.front().bar;
    float hi = lo;
    for (const WindowPoint& p : g_window) {
      lo = std::fmin(lo, p.bar);
      hi = std::fmax(hi, p.bar);
    }
    std::snprintf(text, sizeof(text), "5 s   min %.2f   max %.2f", lo - g_zero_bar, hi - g_zero_bar);
    draw_centered(text, 116, &fonts::DejaVu18, TFT_LIGHTGREY);
  }

  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_CYAN);
  g_canvas.setTextDatum(textdatum_t::top_left);
  std::snprintf(text, sizeof(text), "brut %ld  %.4f bar", static_cast<long>(g_sample.raw), g_sample.bar);
  g_canvas.drawString(text, 12, 146);
  std::snprintf(text, sizeof(text), "sonde %.1f C   zero %.3f", g_sample.celsius, g_zero_bar);
  g_canvas.drawString(text, 12, 168);
  std::snprintf(text, sizeof(text), "erreurs I2C %lu", static_cast<unsigned long>(g_i2c_errors));
  g_canvas.drawString(text, 12, 190);
}

void draw_footer() {
  const int w = g_canvas.width() / 3;
  const int y = g_canvas.height() - 24;
  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_WHITE);
  g_canvas.setTextDatum(textdatum_t::middle_center);
  g_canvas.fillRoundRect(8, y, w - 16, 22, 6, g_power_on ? TFT_DARKGREEN : TFT_DARKGREY);
  g_canvas.drawString(g_power_on ? "5V ON" : "5V OFF", w / 2, y + 11);
  g_canvas.fillRoundRect(w + 8, y, w - 16, 22, 6, TFT_NAVY);
  g_canvas.drawString("ZERO", w + w / 2, y + 11);
  g_canvas.fillRoundRect(2 * w + 8, y, w - 16, 22, 6, TFT_MAROON);
  g_canvas.drawString("OFF", 2 * w + w / 2, y + 11);
  const int peers = ble_server::connections();
  if (peers > 0) {
    char text[16];
    std::snprintf(text, sizeof(text), "BLE %d", peers);
    g_canvas.setFont(&fonts::DejaVu12);
    g_canvas.setTextColor(TFT_CYAN);
    g_canvas.drawString(text, w + w / 2, y - 10);
  }
}

void render() {
  g_canvas.fillScreen(TFT_BLACK);
  draw_header();
  if (!g_power_on) {
    draw_message(g_power_refused ? "Batterie trop faible" : "Port A hors tension",
                 g_power_refused ? "Brancher l'USB" : "Touche A : alimenter");
  } else if (!g_found) {
    char text[32];
    std::snprintf(text, sizeof(text), "port A, 0x%02X, %lu erreur(s)", kAddress,
                  static_cast<unsigned long>(g_i2c_errors));
    draw_message("Recherche XDB401", text);
  } else if (!g_has_sample) {
    draw_message("Mesure...", nullptr);
  } else {
    draw_pressure();
  }
  draw_footer();
  g_canvas.pushSprite(0, 0);
}

void shutdown() {
  power_off();
  g_canvas.fillScreen(TFT_BLACK);
  draw_centered("Extinction", 100, &fonts::DejaVu24, TFT_WHITE);
  g_canvas.pushSprite(0, 0);
  vTaskDelay(pdMS_TO_TICKS(500));
  M5.Power.powerOff();
}

}  // namespace

extern "C" void app_main() {
  auto cfg = M5.config();
  cfg.output_power = false;
  cfg.internal_imu = false;
  cfg.internal_mic = false;
  cfg.internal_spk = false;
  M5.begin(cfg);
  M5.Display.setRotation(1);
  M5.BtnB.setHoldThresh(kHoldMs);
  M5.BtnC.setHoldThresh(kHoldMs);

  g_canvas.setPsram(true);
  g_canvas.setColorDepth(16);
  g_canvas.createSprite(M5.Display.width(), M5.Display.height());

  ble_server::start();
  std::printf("ms,raw,bar,celsius\n");
  power_on();

  uint32_t last_render_ms = 0;
  uint32_t last_battery_ms = 0;
  TickType_t wake = xTaskGetTickCount();
  for (;;) {
    M5.update();
    const uint32_t now = now_ms();
    if (M5.BtnA.wasPressed()) {
      if (g_power_on) {
        power_off();
      } else {
        power_on();
      }
    }
    if (M5.BtnB.wasHold()) {
      g_zero_bar = 0.0f;
    } else if (M5.BtnB.wasClicked() && g_has_sample) {
      g_zero_bar = g_sample.bar;
    }
    if (M5.BtnC.wasHold()) shutdown();
    step_measurement(now);
    if (now - g_last_frame_ms >= kIdleFrameMs) publish(now, nullptr);
    if (last_battery_ms == 0 || now - last_battery_ms >= kBatteryPeriodMs) {
      last_battery_ms = now;
      g_battery_pct = M5.Power.getBatteryLevel();
      g_charging = M5.Power.isCharging() == m5::Power_Class::is_charging;
    }
    if (now - last_render_ms >= kRenderPeriodMs) {
      last_render_ms = now;
      render();
    }
    vTaskDelayUntil(&wake, pdMS_TO_TICKS(kLoopMs));
  }
}

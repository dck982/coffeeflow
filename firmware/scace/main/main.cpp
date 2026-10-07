// Sonde SCACE : NTC du puck lue par un ADS1115 sur le port A du Core2.
//
// Le pont est alimenté par le 5 V du port A, que le firmware laisse coupé au
// démarrage : la sonde ne chauffe que lorsqu'on la mesure. La touche A bascule
// l'alimentation. La touche C, maintenue une seconde, éteint le Core2.
//
// Une seule boucle, sans tâche de mesure : l'ADS1115 convertit seul pendant
// que l'écran se redessine, et la boucle vient chercher le résultat. Le bus
// interne (PMIC, tactile) n'est ainsi jamais partagé entre deux tâches.

#include <M5Unified.h>

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <deque>

namespace {

// Pont : 3,3 V -> R fixe -> A1 -> NTC -> GND, A0 sur le 3,3 V.
// R fixe nominale tant que le point de glace ne l'a pas recalée.
constexpr float kFixedOhm = 1000.0f;
// EPCOS B57861S : 10 kΩ à 25 °C, B25/100 = 3988 K.
constexpr float kNtcR25Ohm = 10000.0f;
constexpr float kNtcBetaK = 3988.0f;
constexpr float kKelvinAt0C = 273.15f;
constexpr float kKelvinAt25C = 298.15f;

constexpr uint32_t kI2cHz = 400000;
constexpr float kVoltsPerCode = 4.096f / 32768.0f;
// Un ADS1115 dont le 3,3 V vient d'apparaître répond en moins d'une
// milliseconde ; 50 ms couvrent aussi la montée du LDO et du boost du Core2.
constexpr uint32_t kPowerSettleMs = 50;
constexpr uint32_t kSearchRetryMs = 500;
constexpr uint32_t kSamplePeriodMs = 100;
// 128 SPS : 7,8 ms par conversion. Au-delà de 50 ms, l'ADS ne convertit plus.
constexpr uint32_t kConversionTimeoutMs = 50;
constexpr uint32_t kWindowMs = 5000;
constexpr uint32_t kLoopMs = 10;
constexpr uint32_t kBatteryPeriodMs = 2000;
// Un effleurement de C ne doit pas éteindre l'appareil en pleine mesure.
constexpr uint32_t kShutdownHoldMs = 1000;

constexpr uint16_t kMuxA0 = 4;
constexpr uint16_t kMuxA1 = 5;

enum class Phase { kOff, kSettling, kSearching, kIdle, kConvA0, kConvA1 };
enum class Status { kNone, kOk, kOpen, kShorted };

struct Sample {
  int16_t a0 = 0;
  int16_t a1 = 0;
  float ohm = NAN;
  float celsius = NAN;
  Status status = Status::kNone;
};

struct WindowPoint {
  uint32_t ms;
  float celsius;
};

Phase g_phase = Phase::kOff;
uint32_t g_phase_ms = 0;
uint32_t g_last_sample_start_ms = 0;
uint8_t g_address = 0;
bool g_bus_open = false;
bool g_power_refused = false;
int16_t g_pending_a0 = 0;
Sample g_sample;
std::deque<WindowPoint> g_window;
uint32_t g_i2c_errors = 0;
int32_t g_battery_pct = -1;
bool g_charging = false;
M5Canvas g_canvas(&M5.Display);

void enter(Phase phase, uint32_t now) {
  g_phase = phase;
  g_phase_ms = now;
}

Sample evaluate(int16_t a0, int16_t a1) {
  Sample s;
  s.a0 = a0;
  s.a1 = a1;
  // Ratiométrique : le rail se simplifie, seuls les codes comptent.
  // NTC ouverte : A1 rejoint A0. NTC en court-circuit : A1 tombe à 0.
  if (a0 <= 0 || a1 >= a0 - 4) {
    s.status = Status::kOpen;
    return s;
  }
  if (a1 <= 4) {
    s.status = Status::kShorted;
    return s;
  }
  s.ohm = kFixedOhm * static_cast<float>(a1) / static_cast<float>(a0 - a1);
  const float inv_t = 1.0f / kKelvinAt25C + std::log(s.ohm / kNtcR25Ohm) / kNtcBetaK;
  s.celsius = 1.0f / inv_t - kKelvinAt0C;
  s.status = Status::kOk;
  return s;
}

bool start_conversion(uint16_t mux) {
  // OS=1, MUX=Ax-GND, PGA=±4,096 V, single-shot, 128 SPS, comparateur coupé.
  const uint16_t config = static_cast<uint16_t>(0x8000 | (mux << 12) | 0x0200 | 0x0100 | 0x0080 | 0x0003);
  const uint8_t bytes[] = {static_cast<uint8_t>(config >> 8), static_cast<uint8_t>(config)};
  return M5.Ex_I2C.writeRegister(g_address, 0x01, bytes, sizeof(bytes), kI2cHz);
}

// Renvoie false sur erreur I2C ; *ready indique si la conversion est finie.
bool poll_conversion(bool* ready, int16_t* raw) {
  uint8_t status[2]{};
  if (!M5.Ex_I2C.readRegister(g_address, 0x01, status, sizeof(status), kI2cHz)) return false;
  *ready = (status[0] & 0x80) != 0;
  if (!*ready) return true;
  uint8_t bytes[2]{};
  if (!M5.Ex_I2C.readRegister(g_address, 0x00, bytes, sizeof(bytes), kI2cHz)) return false;
  *raw = static_cast<int16_t>((static_cast<uint16_t>(bytes[0]) << 8) | bytes[1]);
  return true;
}

void on_i2c_error(uint32_t now) {
  ++g_i2c_errors;
  g_sample = Sample{};
  g_window.clear();
  enter(Phase::kSearching, now);
}

void power_on(uint32_t now) {
  M5.Power.setExtOutput(true);
  // Le Core2 v1.1 refuse le 5 V sur batterie presque vide, sans USB.
  g_power_refused = !M5.Power.getExtOutput();
  if (g_power_refused) return;
  g_i2c_errors = 0;
  enter(Phase::kSettling, now);
}

void power_off(uint32_t now) {
  // Libérer le bus avant de couper : un maître qui tient SDA/SCL hauts
  // alimenterait l'ADS1115 éteint par ses diodes de protection.
  if (g_bus_open) {
    M5.Ex_I2C.release();
    g_bus_open = false;
  }
  M5.Power.setExtOutput(false);
  g_address = 0;
  g_sample = Sample{};
  g_window.clear();
  enter(Phase::kOff, now);
}

void push_window(uint32_t now, const Sample& s) {
  if (s.status == Status::kOk) g_window.push_back({now, s.celsius});
  while (!g_window.empty() && now - g_window.front().ms > kWindowMs) g_window.pop_front();
}

void log_sample(uint32_t now, const Sample& s) {
  // CSV sur la console USB, pour les essais de calibration enregistrés au PC.
  std::printf("%lu,%d,%d,%.2f,%.3f\n", static_cast<unsigned long>(now), s.a0, s.a1, s.ohm, s.celsius);
}

void step_measurement(uint32_t now) {
  switch (g_phase) {
    case Phase::kOff:
      return;
    case Phase::kSettling:
      if (now - g_phase_ms < kPowerSettleMs) return;
      if (!g_bus_open) g_bus_open = M5.Ex_I2C.begin();
      enter(Phase::kSearching, now - kSearchRetryMs);
      return;
    case Phase::kSearching:
      if (now - g_phase_ms < kSearchRetryMs) return;
      g_phase_ms = now;
      for (uint8_t address = 0x48; address <= 0x4B; ++address) {
        if (M5.Ex_I2C.scanID(address, kI2cHz)) {
          g_address = address;
          enter(Phase::kIdle, now);
          g_last_sample_start_ms = now - kSamplePeriodMs;
          return;
        }
      }
      return;
    case Phase::kIdle:
      if (now - g_last_sample_start_ms < kSamplePeriodMs) return;
      g_last_sample_start_ms += kSamplePeriodMs;
      // Rattraper un retard sans enchaîner les mesures en rafale.
      if (now - g_last_sample_start_ms >= kSamplePeriodMs) g_last_sample_start_ms = now;
      if (!start_conversion(kMuxA0)) return on_i2c_error(now);
      enter(Phase::kConvA0, now);
      return;
    case Phase::kConvA0:
    case Phase::kConvA1: {
      bool ready = false;
      int16_t raw = 0;
      if (!poll_conversion(&ready, &raw)) return on_i2c_error(now);
      if (!ready) {
        if (now - g_phase_ms > kConversionTimeoutMs) on_i2c_error(now);
        return;
      }
      if (g_phase == Phase::kConvA0) {
        g_pending_a0 = raw;
        if (!start_conversion(kMuxA1)) return on_i2c_error(now);
        enter(Phase::kConvA1, now);
        return;
      }
      g_sample = evaluate(g_pending_a0, raw);
      push_window(now, g_sample);
      log_sample(now, g_sample);
      enter(Phase::kIdle, now);
      return;
    }
  }
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

void draw_temperature() {
  char text[32];
  std::snprintf(text, sizeof(text), "%.2f", g_sample.celsius);
  g_canvas.setFont(&fonts::DejaVu72);
  g_canvas.setTextColor(TFT_WHITE);
  g_canvas.setTextDatum(textdatum_t::top_right);
  const int right = 270;
  g_canvas.drawString(text, right, 34);
  // Les polices intégrées sont en ASCII : le signe degré est dessiné.
  g_canvas.drawCircle(right + 10, 46, 4, TFT_WHITE);
  g_canvas.drawCircle(right + 10, 46, 3, TFT_WHITE);
  g_canvas.setFont(&fonts::DejaVu40);
  g_canvas.setTextDatum(textdatum_t::top_left);
  g_canvas.drawString("C", right + 16, 40);

  if (!g_window.empty()) {
    float lo = g_window.front().celsius;
    float hi = lo;
    for (const WindowPoint& p : g_window) {
      lo = std::fmin(lo, p.celsius);
      hi = std::fmax(hi, p.celsius);
    }
    std::snprintf(text, sizeof(text), "5 s   min %.2f   max %.2f", lo, hi);
    draw_centered(text, 116, &fonts::DejaVu18, TFT_LIGHTGREY);
  }
}

void draw_raw() {
  char text[48];
  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_CYAN);
  g_canvas.setTextDatum(textdatum_t::top_left);
  std::snprintf(text, sizeof(text), "A0 %6d  %.4f V", g_sample.a0, g_sample.a0 * kVoltsPerCode);
  g_canvas.drawString(text, 12, 146);
  std::snprintf(text, sizeof(text), "A1 %6d  %.4f V", g_sample.a1, g_sample.a1 * kVoltsPerCode);
  g_canvas.drawString(text, 12, 168);
  if (g_sample.status == Status::kOk) {
    std::snprintf(text, sizeof(text), "R  %.1f ohm", g_sample.ohm);
    g_canvas.drawString(text, 12, 190);
  }
}

void draw_header() {
  char text[32];
  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_LIGHTGREY);
  g_canvas.setTextDatum(textdatum_t::top_left);
  g_canvas.drawString("SCACE", 8, 4);
  if (g_battery_pct >= 0) {
    std::snprintf(text, sizeof(text), g_charging ? "USB %ld%%" : "Bat %ld%%", static_cast<long>(g_battery_pct));
    g_canvas.setTextDatum(textdatum_t::top_right);
    g_canvas.drawString(text, g_canvas.width() - 8, 4);
  }
  if (g_phase != Phase::kOff && g_phase != Phase::kSettling && g_address != 0) {
    std::snprintf(text, sizeof(text), "ADS 0x%02X", g_address);
    g_canvas.setTextDatum(textdatum_t::top_center);
    g_canvas.drawString(text, g_canvas.width() / 2, 4);
  }
}

void draw_footer() {
  // Les touches du Core2 sont sous l'écran : A à gauche, C à droite.
  const bool on = g_phase != Phase::kOff;
  const int w = g_canvas.width() / 3;
  const int y = g_canvas.height() - 24;
  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_WHITE);
  g_canvas.setTextDatum(textdatum_t::middle_center);
  g_canvas.fillRoundRect(8, y, w - 16, 22, 6, on ? TFT_DARKGREEN : TFT_DARKGREY);
  g_canvas.drawString(on ? "5V ON" : "5V OFF", w / 2, y + 11);
  g_canvas.fillRoundRect(2 * w + 8, y, w - 16, 22, 6, TFT_MAROON);
  g_canvas.drawString("OFF", 2 * w + w / 2, y + 11);
}

void shutdown(uint32_t now) {
  power_off(now);
  g_canvas.fillScreen(TFT_BLACK);
  draw_centered("Extinction", 100, &fonts::DejaVu24, TFT_WHITE);
  g_canvas.pushSprite(0, 0);
  vTaskDelay(pdMS_TO_TICKS(500));
  M5.Power.powerOff();
}

void render() {
  g_canvas.fillScreen(TFT_BLACK);
  draw_header();
  if (g_phase == Phase::kOff) {
    draw_message(g_power_refused ? "Batterie trop faible" : "Sonde hors tension",
                 g_power_refused ? "Brancher l'USB" : "Touche A : alimenter");
  } else if (g_phase == Phase::kSettling || (g_phase == Phase::kSearching && g_i2c_errors == 0)) {
    draw_message("Recherche ADS1115", "port A, 0x48-0x4B");
  } else if (g_phase == Phase::kSearching) {
    char text[32];
    std::snprintf(text, sizeof(text), "%lu erreur(s)", static_cast<unsigned long>(g_i2c_errors));
    draw_message("Erreur I2C", text);
  } else if (g_sample.status == Status::kNone) {
    draw_message("Mesure...", nullptr);
  } else {
    if (g_sample.status == Status::kOk) {
      draw_temperature();
    } else {
      draw_message(g_sample.status == Status::kOpen ? "NTC ouverte" : "NTC en court-circuit", nullptr);
    }
    draw_raw();
  }
  draw_footer();
  g_canvas.pushSprite(0, 0);
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
  M5.BtnC.setHoldThresh(kShutdownHoldMs);

  g_canvas.setPsram(true);
  g_canvas.setColorDepth(16);
  g_canvas.createSprite(M5.Display.width(), M5.Display.height());

  std::printf("ms,a0,a1,ohm,celsius\n");
  uint32_t last_render_ms = 0;
  uint32_t last_battery_ms = 0;
  TickType_t wake = xTaskGetTickCount();
  for (;;) {
    M5.update();
    const uint32_t now = static_cast<uint32_t>(esp_timer_get_time() / 1000);
    if (M5.BtnA.wasPressed()) {
      if (g_phase == Phase::kOff) {
        power_on(now);
      } else {
        power_off(now);
      }
    }
    if (M5.BtnC.wasHold()) shutdown(now);
    step_measurement(now);
    if (last_battery_ms == 0 || now - last_battery_ms >= kBatteryPeriodMs) {
      last_battery_ms = now;
      g_battery_pct = M5.Power.getBatteryLevel();
      g_charging = M5.Power.isCharging() == m5::Power_Class::is_charging;
    }
    if (now - last_render_ms >= kSamplePeriodMs) {
      last_render_ms = now;
      render();
    }
    vTaskDelayUntil(&wake, pdMS_TO_TICKS(kLoopMs));
  }
}

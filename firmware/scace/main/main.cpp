// Sonde SCACE : NTC du puck lue par un ADS1115 sur le port A du Core2.
//
// Le pont est alimenté par le 5 V du port A, que le firmware laisse coupé au
// démarrage : la sonde ne chauffe que lorsqu'on la mesure. La touche A bascule
// l'alimentation. La touche C, maintenue une seconde, éteint le Core2.
//
// Deux tâches. La tâche de mesure, prioritaire et seule à toucher le bus du
// port A (Ex_I2C), lit l'ADS1115 à 10 Hz et envoie chaque mesure sur la
// console et en notification BLE (ble_server). La boucle principale garde les
// touches, la puce d'alimentation sur le bus interne et l'écran, redessiné à
// 4 Hz : un dessin complet prend environ 75 ms et ne retarde plus la mesure.

#include <M5Unified.h>

#include <atomic>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <deque>

#include "ble_server.h"
#include "freertos/semphr.h"

namespace {

// Version de l'image, affichée en haut de l'écran. Indépendante de celle des
// cartes de la machine (common/version.hpp) ; à incrémenter à chaque image
// flashée.
constexpr char kFirmwareVersion[] = "0.3.1";

// Pont : 3,3 V -> R fixe -> A1 -> NTC -> GND, A0 sur le 3,3 V.
// Recalée au point de glace : elle absorbe la R fixe réelle et la tolérance
// de ±1 % sur R25, car seul leur rapport compte. Ce n'est donc pas la valeur
// lue au multimètre (989 Ω).
constexpr double kFixedOhm = 999.5;
// EPCOS B57861S0103F, courbe R/T 8016 : Steinhart–Hart ajustée sur la table
// du fabricant de -10 à 110 °C, 1/T = A + B ln R + C (ln R)^3, R en ohms.
// Écart à la table ≤ 0,02 K, l'ordre de son arrondi. La loi Beta avec
// B25/100 lisait +0,77 K à 0 °C et -0,39 K à 60 °C.
constexpr double kShA = 1.12476949e-03;
constexpr double kShB = 2.34824463e-04;
constexpr double kShC = 8.50854659e-08;
// Écart de B à la valeur nominale (tolérance ±0,3 %, soit ±12 K), recalé au
// point d'ébullition. Référencé à 0 °C pour ne pas déplacer le point de glace.
constexpr double kBetaShiftK = 0.0;
constexpr double kKelvinAt0C = 273.15;

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
// Une paire A0 puis A1 tient en quatre tours de 5 ms.
constexpr uint32_t kMeasureLoopMs = 5;
constexpr uint32_t kUiLoopMs = 10;
constexpr uint32_t kRenderPeriodMs = 250;
constexpr uint32_t kBatteryPeriodMs = 2000;
// Un effleurement de C ne doit pas éteindre l'appareil en pleine mesure.
constexpr uint32_t kShutdownHoldMs = 1000;
// Sans mesure, une trame « pas de mesure » par seconde dit aux centraux que
// la sonde est là mais que le pont est coupé ou l'ADS absent.
constexpr uint32_t kIdleFrameMs = 1000;
// La tâche de mesure libère le bus en un tour ; au-delà, on coupe quand même.
constexpr TickType_t kReleaseTimeout = pdMS_TO_TICKS(200);

constexpr uint16_t kMuxA0 = 4;
constexpr uint16_t kMuxA1 = 5;

enum class Phase { kOff, kSettling, kSearching, kIdle, kConvA0, kConvA1 };
enum class Status { kNone, kOk, kOpen, kShorted };
enum class Request { kNone, kOn, kOff };

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

// Ce que l'écran lit de la mesure, copié sous verrou.
struct Snapshot {
  Phase phase = Phase::kOff;
  uint8_t address = 0;
  uint32_t i2c_errors = 0;
  Sample sample;
  bool has_window = false;
  float lo = NAN;
  float hi = NAN;
};

uint32_t now_ms() { return static_cast<uint32_t>(esp_timer_get_time() / 1000); }

// --- Échanges entre les deux tâches ---------------------------------------

std::atomic<Request> g_request{Request::kNone};
SemaphoreHandle_t g_bus_released = nullptr;
portMUX_TYPE g_snapshot_lock = portMUX_INITIALIZER_UNLOCKED;
Snapshot g_snapshot;

// --- État de la tâche de mesure --------------------------------------------

Phase g_phase = Phase::kOff;
uint32_t g_phase_ms = 0;
uint32_t g_last_sample_start_ms = 0;
uint8_t g_address = 0;
bool g_bus_open = false;
int16_t g_pending_a0 = 0;
Sample g_sample;
std::deque<WindowPoint> g_window;
uint32_t g_i2c_errors = 0;
uint16_t g_frame_seq = 0;
uint32_t g_last_frame_ms = 0;

// --- État de la boucle principale ------------------------------------------

bool g_bridge_on = false;
bool g_power_refused = false;
int32_t g_battery_pct = -1;
bool g_charging = false;
M5Canvas g_canvas(&M5.Display);

void enter(Phase phase, uint32_t now) {
  g_phase = phase;
  g_phase_ms = now;
}

double sh_inv_kelvin(double ln_r) { return kShA + kShB * ln_r + kShC * ln_r * ln_r * ln_r; }

// R = R_table(T) · exp(ΔB · (1/T − 1/273,15)) : on retire le décalage de B
// avant la table. 1/T dépend de lui-même, mais à peine (pente ~0,003) :
// trois itérations suffisent largement.
double celsius_from_ohm(double ohm) {
  const double ln_r = std::log(ohm);
  double inv_t = sh_inv_kelvin(ln_r);
  for (int i = 0; i < 3; ++i) inv_t = sh_inv_kelvin(ln_r - kBetaShiftK * (inv_t - 1.0 / kKelvinAt0C));
  return 1.0 / inv_t - kKelvinAt0C;
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
  const double ohm = kFixedOhm * a1 / (a0 - a1);
  s.ohm = static_cast<float>(ohm);
  s.celsius = static_cast<float>(celsius_from_ohm(ohm));
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

// Appelée par la tâche de mesure à la demande de coupure, avant que la
// boucle principale coupe le 5 V.
void stop_bridge(uint32_t now) {
  // Libérer le bus avant de couper : un maître qui tient SDA/SCL hauts
  // alimenterait l'ADS1115 éteint par ses diodes de protection.
  if (g_bus_open) {
    M5.Ex_I2C.release();
    g_bus_open = false;
  }
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

void publish_sample(uint32_t now, const Sample& s) {
  common::scace::Frame frame;
  frame.seq = g_frame_seq++;
  frame.ms = static_cast<uint16_t>(now);
  frame.a0 = s.a0;
  frame.a1 = s.a1;
  switch (s.status) {
    case Status::kOk:
      frame.status = common::scace::Status::kOk;
      frame.centi_c = static_cast<int16_t>(std::lround(std::fmin(std::fmax(s.celsius, -300.0f), 300.0f) * 100.0f));
      break;
    case Status::kOpen:
      frame.status = common::scace::Status::kOpen;
      break;
    case Status::kShorted:
      frame.status = common::scace::Status::kShorted;
      break;
    case Status::kNone:
      frame.status = common::scace::Status::kNoMeasurement;
      break;
  }
  ble_server::publish(frame);
  g_last_frame_ms = now;
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
      publish_sample(now, g_sample);
      enter(Phase::kIdle, now);
      return;
    }
  }
}

void publish_snapshot() {
  Snapshot s;
  s.phase = g_phase;
  s.address = g_address;
  s.i2c_errors = g_i2c_errors;
  s.sample = g_sample;
  if (!g_window.empty()) {
    s.has_window = true;
    s.lo = s.hi = g_window.front().celsius;
    for (const WindowPoint& p : g_window) {
      s.lo = std::fmin(s.lo, p.celsius);
      s.hi = std::fmax(s.hi, p.celsius);
    }
  }
  portENTER_CRITICAL(&g_snapshot_lock);
  g_snapshot = s;
  portEXIT_CRITICAL(&g_snapshot_lock);
}

void measurement_task(void*) {
  TickType_t wake = xTaskGetTickCount();
  for (;;) {
    const uint32_t now = now_ms();
    switch (g_request.exchange(Request::kNone)) {
      case Request::kOn:
        g_i2c_errors = 0;
        enter(Phase::kSettling, now);
        break;
      case Request::kOff:
        stop_bridge(now);
        xSemaphoreGive(g_bus_released);
        break;
      case Request::kNone:
        break;
    }
    step_measurement(now);
    if (now - g_last_frame_ms >= kIdleFrameMs) publish_sample(now, Sample{});
    publish_snapshot();
    vTaskDelayUntil(&wake, pdMS_TO_TICKS(kMeasureLoopMs));
  }
}

// --- Boucle principale ------------------------------------------------------

Snapshot snapshot() {
  portENTER_CRITICAL(&g_snapshot_lock);
  const Snapshot s = g_snapshot;
  portEXIT_CRITICAL(&g_snapshot_lock);
  return s;
}

void power_on() {
  M5.Power.setExtOutput(true);
  // Le Core2 v1.1 refuse le 5 V sur batterie presque vide, sans USB.
  g_power_refused = !M5.Power.getExtOutput();
  if (g_power_refused) return;
  g_bridge_on = true;
  g_request = Request::kOn;
}

void power_off() {
  xSemaphoreTake(g_bus_released, 0);
  g_request = Request::kOff;
  xSemaphoreTake(g_bus_released, kReleaseTimeout);
  M5.Power.setExtOutput(false);
  g_bridge_on = false;
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

void draw_temperature(const Snapshot& snap) {
  char text[32];
  std::snprintf(text, sizeof(text), "%.2f", snap.sample.celsius);
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

  if (snap.has_window) {
    std::snprintf(text, sizeof(text), "5 s   min %.2f   max %.2f", snap.lo, snap.hi);
    draw_centered(text, 116, &fonts::DejaVu18, TFT_LIGHTGREY);
  }
}

void draw_raw(const Sample& sample) {
  char text[48];
  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_CYAN);
  g_canvas.setTextDatum(textdatum_t::top_left);
  std::snprintf(text, sizeof(text), "A0 %6d  %.4f V", sample.a0, sample.a0 * kVoltsPerCode);
  g_canvas.drawString(text, 12, 146);
  std::snprintf(text, sizeof(text), "A1 %6d  %.4f V", sample.a1, sample.a1 * kVoltsPerCode);
  g_canvas.drawString(text, 12, 168);
  if (sample.status == Status::kOk) {
    std::snprintf(text, sizeof(text), "R  %.1f ohm", sample.ohm);
    g_canvas.drawString(text, 12, 190);
  }
}

void draw_header(const Snapshot& snap) {
  char text[32];
  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_LIGHTGREY);
  g_canvas.setTextDatum(textdatum_t::top_left);
  g_canvas.drawString("SCACE", 8, 4);
  // En petit, pour laisser la place à l'adresse de l'ADS centrée.
  const int version_x = 8 + g_canvas.textWidth("SCACE ");
  g_canvas.setFont(&fonts::DejaVu12);
  g_canvas.drawString(kFirmwareVersion, version_x, 9);
  g_canvas.setFont(&fonts::DejaVu18);
  if (g_battery_pct >= 0) {
    std::snprintf(text, sizeof(text), g_charging ? "USB %ld%%" : "Bat %ld%%", static_cast<long>(g_battery_pct));
    g_canvas.setTextDatum(textdatum_t::top_right);
    g_canvas.drawString(text, g_canvas.width() - 8, 4);
  }
  if (snap.phase != Phase::kOff && snap.phase != Phase::kSettling && snap.address != 0) {
    std::snprintf(text, sizeof(text), "ADS 0x%02X", snap.address);
    // Décalé de 5 px : centré, il touchait la version.
    g_canvas.setTextDatum(textdatum_t::top_center);
    g_canvas.drawString(text, g_canvas.width() / 2 + 5, 4);
  }
}

void draw_footer() {
  // Les touches du Core2 sont sous l'écran : A à gauche, C à droite.
  const bool on = g_bridge_on;
  const int w = g_canvas.width() / 3;
  const int y = g_canvas.height() - 24;
  g_canvas.setFont(&fonts::DejaVu18);
  g_canvas.setTextColor(TFT_WHITE);
  g_canvas.setTextDatum(textdatum_t::middle_center);
  g_canvas.fillRoundRect(8, y, w - 16, 22, 6, on ? TFT_DARKGREEN : TFT_DARKGREY);
  g_canvas.drawString(on ? "5V ON" : "5V OFF", w / 2, y + 11);
  g_canvas.fillRoundRect(2 * w + 8, y, w - 16, 22, 6, TFT_MAROON);
  g_canvas.drawString("OFF", 2 * w + w / 2, y + 11);
  // B n'a pas de rôle : sa place, sans bouton dessiné, montre les centraux.
  const int peers = ble_server::connections();
  if (peers > 0) {
    char text[16];
    std::snprintf(text, sizeof(text), "BLE %d", peers);
    g_canvas.setTextColor(TFT_CYAN);
    g_canvas.drawString(text, w + w / 2, y + 11);
  }
}

void shutdown() {
  power_off();
  g_canvas.fillScreen(TFT_BLACK);
  draw_centered("Extinction", 100, &fonts::DejaVu24, TFT_WHITE);
  g_canvas.pushSprite(0, 0);
  vTaskDelay(pdMS_TO_TICKS(500));
  M5.Power.powerOff();
}

void render() {
  const Snapshot snap = snapshot();
  const Sample& sample = snap.sample;
  g_canvas.fillScreen(TFT_BLACK);
  draw_header(snap);
  if (!g_bridge_on || snap.phase == Phase::kOff) {
    draw_message(g_power_refused ? "Batterie trop faible" : "Sonde hors tension",
                 g_power_refused ? "Brancher l'USB" : "Touche A : alimenter");
  } else if (snap.phase == Phase::kSettling || (snap.phase == Phase::kSearching && snap.i2c_errors == 0)) {
    draw_message("Recherche ADS1115", "port A, 0x48-0x4B");
  } else if (snap.phase == Phase::kSearching) {
    char text[32];
    std::snprintf(text, sizeof(text), "%lu erreur(s)", static_cast<unsigned long>(snap.i2c_errors));
    draw_message("Erreur I2C", text);
  } else if (sample.status == Status::kNone) {
    draw_message("Mesure...", nullptr);
  } else {
    if (sample.status == Status::kOk) {
      draw_temperature(snap);
    } else {
      draw_message(sample.status == Status::kOpen ? "NTC ouverte" : "NTC en court-circuit", nullptr);
    }
    draw_raw(sample);
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

  ble_server::start();

  std::printf("ms,a0,a1,ohm,celsius\n");
  g_bus_released = xSemaphoreCreateBinary();
  // Cœur 1 : app_main et le contrôleur BLE tournent sur le cœur 0.
  xTaskCreatePinnedToCore(measurement_task, "measure", 6144, nullptr, 5, nullptr, 1);

  uint32_t last_render_ms = 0;
  uint32_t last_battery_ms = 0;
  TickType_t wake = xTaskGetTickCount();
  for (;;) {
    M5.update();
    const uint32_t now = now_ms();
    if (M5.BtnA.wasPressed()) {
      if (g_bridge_on) {
        power_off();
      } else {
        power_on();
      }
    }
    if (M5.BtnC.wasHold()) shutdown();
    if (last_battery_ms == 0 || now - last_battery_ms >= kBatteryPeriodMs) {
      last_battery_ms = now;
      g_battery_pct = M5.Power.getBatteryLevel();
      g_charging = M5.Power.isCharging() == m5::Power_Class::is_charging;
    }
    if (now - last_render_ms >= kRenderPeriodMs) {
      last_render_ms = now;
      render();
    }
    vTaskDelayUntil(&wake, pdMS_TO_TICKS(kUiLoopMs));
  }
}

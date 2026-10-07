#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "lvgl.h"
#include "src/draw/snapshot/lv_snapshot.h"

#include "core/core.h"
#include "ui/ui_home.h"
#include "ui/ui_theme.h"

extern int64_t g_sim_time_us;
char g_sim_clock_override[6]{};

namespace {
uint32_t g_display_buffer[800 * 480];

void flush(lv_display_t *display, const lv_area_t *, uint8_t *) {
  lv_display_flush_ready(display);
}

void save_ppm(const char *path, lv_draw_buf_t *image) {
  FILE *file = std::fopen(path, "wb");
  if (file == nullptr)
    std::abort();
  std::fprintf(file, "P6\n%u %u\n255\n", image->header.w, image->header.h);
  // LVGL range RGB888 as BGR bytes on this little-endian renderer; PPM wants
  // canonical RGB. Keeping this conversion here makes colour reviews useful.
  for (uint32_t i = 0; i < image->header.w * image->header.h; ++i) {
    const uint8_t *pixel = image->data + i * 3;
    std::fputc(pixel[2], file);
    std::fputc(pixel[1], file);
    std::fputc(pixel[0], file);
  }
  std::fclose(file);
}

lv_obj_t *find_caption_parent(lv_obj_t *object, const char *caption) {
  if (lv_obj_check_type(object, &lv_label_class) &&
      std::strcmp(lv_label_get_text(object), caption) == 0)
    return lv_obj_get_parent(object);
  for (uint32_t i = 0; i < lv_obj_get_child_count(object); ++i) {
    if (lv_obj_t *found =
            find_caption_parent(lv_obj_get_child(object, i), caption))
      return found;
  }
  return nullptr;
}

lv_obj_t *find_at(lv_obj_t *object, int x, int y) {
  if (lv_obj_has_flag(object, LV_OBJ_FLAG_HIDDEN))
    return nullptr;
  for (uint32_t i = lv_obj_get_child_count(object); i-- > 0;)
    if (lv_obj_t *found = find_at(lv_obj_get_child(object, i), x, y))
      return found;
  lv_area_t coords{};
  lv_obj_get_coords(object, &coords);
  if (coords.x1 == x && coords.y1 == y &&
      lv_obj_has_flag(object, LV_OBJ_FLAG_CLICKABLE))
    return object;
  return nullptr;
}

void click_at(lv_obj_t *screen, int x, int y) {
  lv_obj_t *button = find_at(screen, x, y);
  if (button != nullptr)
    lv_obj_send_event(button, LV_EVENT_CLICKED, nullptr);
}

int clock_width(const char *clock) {
  int width = 0;
  uint32_t previous = 0;
  for (const char *c = clock; *c != '\0'; ++c) {
    lv_font_glyph_dsc_t dsc{};
    if (lv_font_get_glyph_dsc(ui::theme::kFontStatus, &dsc,
                              static_cast<uint8_t>(*c), previous))
      width += dsc.adv_w;
    previous = static_cast<uint8_t>(*c);
  }
  return width;
}

void select_widest_valid_clock() {
  int widest = -1;
  for (int hour = 0; hour < 24; ++hour) {
    for (int minute = 0; minute < 60; ++minute) {
      char candidate[6];
      std::snprintf(candidate, sizeof(candidate), "%02d:%02d", hour, minute);
      const int width = clock_width(candidate);
      if (width > widest) {
        widest = width;
        std::snprintf(g_sim_clock_override, sizeof(g_sim_clock_override), "%s",
                      candidate);
      }
    }
  }
  std::printf("wide top bar clock: %s (%d px advance)\n",
              g_sim_clock_override, widest);
}
// Infusion synthétique, passée par le vrai ShotSummaryBuilder : précharge
// 5,5 s, remplissage 3 s, pause 4 s, montée jusqu'à 8 bar en 4 s, infusion à
// 1,2 g/s jusqu'à 40 s, puis gouttes. `replay=<csv>` rejoue une capture
// convertie (t mode pompe pv p balance poids volume tv T).
core::ShotSample synthetic_sample(float t) {
  core::ShotSample s;
  s.t_s = t;
  s.scale_present = true;
  s.pressure_valid = true;
  s.temperature_valid = true;
  const float stop = 40.0f;
  s.mode = t < 5.5f ? core::ShotSampleMode::kPreheat
           : t < 8.5f ? core::ShotSampleMode::kFilling
           : t < 12.5f ? core::ShotSampleMode::kPreinfusion
           : t < stop ? core::ShotSampleMode::kInfusion
                      : core::ShotSampleMode::kCooldown;
  s.pump_pct = t < 5.5f ? 0 : t < 8.5f ? 75 : t < 12.5f ? 35 : t < stop ? 65 : 0;
  s.pressure_bar = t < 12.5f ? 0.4f : t < 16.5f ? 0.4f + (t - 12.5f) * 2.1f : t < stop ? 9.0f : 7.2f;
  const float pour = std::max(0.0f, std::min(t, stop) - 16.5f) * 0.9f;
  const float drip = t > stop ? std::min(t - stop, 3.0f) * 0.2f : 0.0f;
  s.weight_g = pour + drip;
  s.volume_ml = t < 5.5f ? 0.0f : std::min(t, stop) * 1.4f - 7.7f;
  s.temperature_c = t < 5.5f ? 90.0f
                    : t < 19.0f ? 90.6f - (t - 5.5f) * 0.14f
                                : std::min(92.6f, 88.7f + (t - 19.0f) * 0.25f);
  return s;
}
core::ShotSummary synthetic_shot(float until) {
  core::ShotSummaryBuilder builder;
  builder.reset(9.0f, 5.5f);
  for (int i = 0; i * 0.1f <= until + 1e-4f; ++i)
    builder.add(synthetic_sample(i * 0.1f));
  return builder.summary();
}
core::ShotSummary replay_shot(const char *path, float until, core::ShotSample *last) {
  core::ShotSummaryBuilder builder;
  builder.reset(9.0f, 5.5f);
  FILE *file = std::fopen(path, "r");
  if (file == nullptr)
    std::abort();
  float t, p, w, v, temperature;
  int mode, pump, pv, scale, tv;
  while (std::fscanf(file, "%f %d %d %d %f %d %f %f %d %f", &t, &mode, &pump, &pv,
                     &p, &scale, &w, &v, &tv, &temperature) == 10 &&
         t <= until) {
    *last = {t, static_cast<core::ShotSampleMode>(mode), static_cast<uint8_t>(pump),
             pv != 0, p, scale != 0, w, v, tv != 0, temperature};
    builder.add(*last);
  }
  std::fclose(file);
  return builder.summary();
}
void apply_shot(core::Snapshot &snapshot, const core::ShotSummary &shot,
                const core::ShotSample &now, float capture_end_s) {
  snapshot.shot = shot;
  snapshot.shot_start_unix_s = 1791098488;  // 04/10 10:01 UTC+2
  snapshot.cycle_weight_goal = true;
  snapshot.weight_g = now.weight_g;
  snapshot.pressure_bar = now.pressure_bar;
  snapshot.boiler_temperature_c = now.temperature_c;
  snapshot.dimmer_pct = now.pump_pct;
  snapshot.flow_ml_s = now.pump_pct > 0 ? 1.25f : 0.0f;
  snapshot.heating_power_pct = shot.pump_running() ? 28.0f : 0.0f;
  const bool finished = shot.pump_stopped();
  snapshot.cycle_state = finished ? core::CycleState::kFinished
                         : now.mode == core::ShotSampleMode::kPreheat ? core::CycleState::kThermalPreheat
                         : now.mode == core::ShotSampleMode::kFilling ? core::CycleState::kFilling
                         : now.mode == core::ShotSampleMode::kPreinfusion ? core::CycleState::kPreinfusion
                                                                          : core::CycleState::kBrew;
  snapshot.capture_cooldown = finished && now.t_s < capture_end_s;
  snapshot.capture_cooldown_remaining_ms =
      snapshot.capture_cooldown ? static_cast<uint32_t>((capture_end_s - now.t_s) * 1000) : 0;
}
} // namespace

int main(int argc, char **argv) {
  const char *output = argc >= 2 ? argv[1] : "ui-home.ppm";
  const char *scenario = argc >= 3 ? argv[2] : "";
  // Anciens noms, conservés : l'infusion passe par le même écran.
  if (std::strcmp(scenario, "brew") == 0)
    scenario = "brew-30";
  else if (std::strcmp(scenario, "preinfusion") == 0)
    scenario = "brew-10";
  lv_init();
  lv_display_t *display =
      lv_display_create(ui::theme::kScreenWidth, ui::theme::kScreenHeight);
  lv_display_set_buffers(display, g_display_buffer, nullptr,
                         sizeof(g_display_buffer),
                         LV_DISPLAY_RENDER_MODE_DIRECT);
  lv_display_set_flush_cb(display, flush);
  lv_obj_t *screen = lv_obj_create(nullptr);
  lv_obj_set_size(screen, ui::theme::kScreenWidth, ui::theme::kScreenHeight);
  lv_obj_set_style_bg_color(screen, ui::theme::kBg, 0);
  lv_obj_set_style_bg_opa(screen, LV_OPA_COVER, 0);
  lv_screen_load(screen);
  if (std::strcmp(scenario, "wide-topbar") == 0)
    select_widest_valid_clock();
  ui::home::create(screen);
  core::Snapshot snapshot{};
  snapshot.sensors_alive = true;
  snapshot.pressure_valid = true;
  snapshot.pressure_freshness = core::Freshness::kFresh;
  snapshot.pressure_bar = std::strcmp(scenario, "wide-topbar") == 0 ? -10.1f : 9.1f;
  snapshot.temperature_c = 93.4f;
  snapshot.boiler_temperature_c = 93.4f;
  snapshot.boiler_temperature_valid = true;
  snapshot.boiler_temperature_freshness = core::Freshness::kFresh;
  snapshot.heating_power_capable = true;
  snapshot.heating_freshness = core::Freshness::kFresh;
  snapshot.heating_power_pct = 62.5f;
  snapshot.heating_power_accepted_pct = 62.5f;
  snapshot.scale_present = std::strcmp(scenario, "keypad-time") != 0 &&
                           std::strcmp(scenario, "no-scale") != 0;
  snapshot.scale_connected = snapshot.scale_present;
  snapshot.time_known = true;
  snapshot.weight_g = 18.2f;
  snapshot.dimmer_ready = true;
  snapshot.dimmer_valid = true;
  snapshot.flow_valid = true;
  snapshot.flow_freshness = core::Freshness::kFresh;
  snapshot.flow_ml_s = 2.1f;
  snapshot.actuators_freshness = core::Freshness::kFresh;
  snapshot.dimmer_pct = 72;
  snapshot.screen_version_major = 0;
  snapshot.screen_version_minor = 2;
  snapshot.screen_version_patch = 3;
  snapshot.internal_heap_free = 61428;
  snapshot.internal_heap_largest = 31744;
  snapshot.sensors_version_major = 0;
  snapshot.sensors_version_minor = 1;
  snapshot.sensors_version_patch = 7;
  snapshot.sensors_twai_rx_errors = 2;
  snapshot.sensors_twai_tx_errors = 1;
  snapshot.sensors_twai_bus_errors = 1231;
  if (std::strcmp(scenario, "cold") == 0) {
    snapshot.boiler_sensor_temperature_c = 24.6f;
    snapshot.boiler_temperature_c = 14.1f;
  }
  if (std::strcmp(scenario, "cold-capped") == 0) {
    snapshot.boiler_sensor_temperature_c = 56.3f;
    snapshot.boiler_temperature_c = 45.8f;
  }
  if (std::strcmp(scenario, "diagnostic-errors") == 0) {
    snapshot.pressure_valid = false;
    snapshot.boiler_temperature_valid = false;
    snapshot.flow_valid = false;
    snapshot.dimmer_valid = false;
    snapshot.actuators_freshness = core::Freshness::kMissing;
    snapshot.heating_freshness = core::Freshness::kMissing;
    snapshot.scale_present = false;
    snapshot.sensors_alive = false;
    snapshot.sensors_twai_rx_errors = UINT16_MAX;
    snapshot.sensors_twai_tx_errors = UINT16_MAX;
    snapshot.sensors_twai_bus_errors = UINT32_MAX;
  }
  if (std::strcmp(scenario, "diagnostic-states") == 0) {
    snapshot.pressure_freshness = core::Freshness::kStale;
    snapshot.dimmer_ready = false;
  }
  if (std::strcmp(scenario, "diagnostic-states") == 0 ||
      std::strcmp(scenario, "heating-menu-off") == 0) {
    core::Config config = core::get_config();
    config.heating_enabled = false;
    core::put_config(config);
  }
  if (std::strcmp(scenario, "valve-menu") == 0 ||
      std::strcmp(scenario, "valve-menu-open") == 0 ||
      std::strcmp(scenario, "valve-menu-heating") == 0) {
    snapshot.maintenance_valve_capable = true;
    snapshot.dimmer_pct = 0;
    core::Config config = core::get_config();
    config.heating_enabled = std::strcmp(scenario, "valve-menu-heating") == 0;
    core::put_config(config);
  }
  if (std::strcmp(scenario, "valve-menu-open") == 0) {
    snapshot.valve_open = true;
    snapshot.maintenance_valve_open = true;
    snapshot.lease_remaining_ms = 22'400;
  }
  if (std::strcmp(scenario, "wifi-mode") == 0) {
    snapshot.radio_mode = core::RadioMode::kWifi;
    snapshot.network_state = static_cast<uint8_t>(core::NetworkState::kStaConnected);
    snapshot.ipv4_address = 192u | (168u << 8) | (2u << 16) | (50u << 24);
  }
  // brew-<t> : infusion synthétique à t secondes (fin de capture à 60 s) ;
  // replay=<csv>@<t> : capture rejouée (fin de capture à 66,9 s).
  if (std::strncmp(scenario, "brew-", 5) == 0) {
    const float t = std::strtof(scenario + 5, nullptr);
    apply_shot(snapshot, synthetic_shot(t), synthetic_sample(t), 60.0f);
  } else if (std::strncmp(scenario, "replay=", 7) == 0) {
    char path[512];
    std::snprintf(path, sizeof(path), "%s", scenario + 7);
    char *at = std::strrchr(path, '@');
    if (at == nullptr)
      std::abort();
    *at = '\0';
    core::ShotSample last;
    const float t = std::strtof(at + 1, nullptr);
    const core::ShotSummary shot = replay_shot(path, t, &last);
    apply_shot(snapshot, shot, last, 66.88f);
  }
  ui::home::refresh(snapshot, false);
  ui::home::snapshot_scenario(scenario);
  ui::home::refresh(snapshot, false);
  if (std::strcmp(scenario, "disabled") == 0) {
    snapshot.dimmer_ready = false;
    ui::home::refresh(snapshot, false);
  }
  if (std::strcmp(scenario, "dim") == 0) {
    g_sim_time_us = 240 * 1000000LL;
    ui::home::refresh(snapshot, false);
  }
  if (std::strcmp(scenario, "standby") == 0) {
    g_sim_time_us = 1800 * 1000000LL;
    ui::home::refresh(snapshot, false);
  }
  lv_timer_handler();
  lv_draw_buf_t *image = lv_snapshot_take(screen, LV_COLOR_FORMAT_RGB888);
  if (image == nullptr) {
    std::fputs("lv_snapshot_take failed\n", stderr);
    return 2;
  }
  save_ppm(output, image);
  lv_draw_buf_destroy(image);
  lv_deinit();
  return 0;
}

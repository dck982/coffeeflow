#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "lvgl.h"
#include "src/draw/snapshot/lv_snapshot.h"

#include "core/core.h"
#include "ui/ui_home.h"
#include "ui/ui_theme.h"

extern int64_t g_sim_time_us;

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
} // namespace

int main(int argc, char **argv) {
  const char *output = argc >= 2 ? argv[1] : "ui-home.ppm";
  const char *scenario = argc >= 3 ? argv[2] : "";
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
  ui::home::create(screen);
  core::Snapshot snapshot{};
  snapshot.sensors_alive = true;
  snapshot.pressure_valid = true;
  snapshot.pressure_freshness = core::Freshness::kFresh;
  snapshot.pressure_bar = 9.1f;
  snapshot.temperature_c = 93.4f;
  snapshot.scale_present = std::strcmp(scenario, "keypad-time") != 0;
  snapshot.weight_g = 18.2f;
  snapshot.dimmer_ready = true;
  snapshot.dimmer_valid = true;
  if (std::strcmp(scenario, "brew") == 0) {
    snapshot.cycle_state = core::CycleState::kBrew;
    snapshot.cycle_weight_goal = true;
    snapshot.cycle_start_weight_g = 0.0f;
    snapshot.cycle_elapsed_ms = 24'000;
    snapshot.flow_ml_s = 1.8f;
    snapshot.dimmer_pct = 80;
  }
  if (std::strcmp(scenario, "preinfusion") == 0) {
    snapshot.cycle_state = core::CycleState::kPreinfusion;
    snapshot.cycle_weight_goal = true;
    snapshot.cycle_start_weight_g = 0.0f;
    snapshot.cycle_elapsed_ms = 4'000;
    snapshot.cycle_phase_elapsed_ms = 4'000;
    snapshot.flow_ml_s = 0.5f;
    snapshot.dimmer_pct = 30;
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

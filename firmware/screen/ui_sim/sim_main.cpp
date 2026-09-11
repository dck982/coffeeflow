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

void flush(lv_display_t* display, const lv_area_t*, uint8_t*) {
  lv_display_flush_ready(display);
}

void save_ppm(const char* path, lv_draw_buf_t* image) {
  FILE* file = std::fopen(path, "wb");
  if (file == nullptr) std::abort();
  std::fprintf(file, "P6\n%u %u\n255\n", image->header.w, image->header.h);
  // LVGL range RGB888 as BGR bytes on this little-endian renderer; PPM wants
  // canonical RGB. Keeping this conversion here makes colour reviews useful.
  for(uint32_t i = 0; i < image->header.w * image->header.h; ++i) {
    const uint8_t* pixel = image->data + i * 3;
    std::fputc(pixel[2], file); std::fputc(pixel[1], file); std::fputc(pixel[0], file);
  }
  std::fclose(file);
}

lv_obj_t* find_caption_parent(lv_obj_t* object, const char* caption) {
  if(lv_obj_check_type(object, &lv_label_class) && std::strcmp(lv_label_get_text(object), caption) == 0)
    return lv_obj_get_parent(object);
  for(uint32_t i = 0; i < lv_obj_get_child_count(object); ++i) {
    if(lv_obj_t* found = find_caption_parent(lv_obj_get_child(object, i), caption)) return found;
  }
  return nullptr;
}
}

int main(int argc, char** argv) {
  const char* output = argc >= 2 ? argv[1] : "ui-home.ppm";
  const char* scenario = argc >= 3 ? argv[2] : "";
  const int settings_page = std::strncmp(scenario, "settings", 8) == 0 ? std::atoi(scenario + 8) : -1;
  const bool wifi_confirm = std::strcmp(scenario, "wifi-confirm") == 0;
  lv_init();
  lv_display_t* display = lv_display_create(ui::theme::kScreenWidth, ui::theme::kScreenHeight);
  lv_display_set_buffers(display, g_display_buffer, nullptr, sizeof(g_display_buffer), LV_DISPLAY_RENDER_MODE_DIRECT);
  lv_display_set_flush_cb(display, flush);
  lv_obj_t* screen = lv_obj_create(nullptr);
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
  snapshot.scale_present = true;
  snapshot.weight_g = 18.2f;
  snapshot.dimmer_ready = true;
  snapshot.dimmer_valid = true;
  ui::home::refresh(snapshot, false);
  if(settings_page >= 0 || wifi_confirm) {
    lv_obj_t* button = find_caption_parent(screen, "réglages");
    if(button == nullptr) return 3;
    lv_obj_send_event(button, LV_EVENT_CLICKED, nullptr);
    for(int i = 0; i < (wifi_confirm ? 2 : settings_page); ++i) {
      button = find_caption_parent(screen, "suite");
      if(button == nullptr) return 4;
      lv_obj_send_event(button, LV_EVENT_CLICKED, nullptr);
    }
    if(wifi_confirm) {
      button = find_caption_parent(screen, "mode wifi");
      if(button == nullptr) return 5;
      lv_obj_send_event(button, LV_EVENT_CLICKED, nullptr);
    }
  }
  if(std::strcmp(scenario, "dim") == 0) { g_sim_time_us = 240 * 1000000LL; ui::home::refresh(snapshot, false); }
  if(std::strcmp(scenario, "standby") == 0) { g_sim_time_us = 1800 * 1000000LL; ui::home::refresh(snapshot, false); }
  lv_timer_handler();
  lv_draw_buf_t* image = lv_snapshot_take(screen, LV_COLOR_FORMAT_RGB888);
  if (image == nullptr) { std::fputs("lv_snapshot_take failed\n", stderr); return 2; }
  save_ppm(output, image);
  lv_draw_buf_destroy(image);
  lv_deinit();
  return 0;
}

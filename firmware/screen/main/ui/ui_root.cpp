#include "ui_root.h"

#include "esp_timer.h"
#include "lvgl.h"

#include "core/core.h"
#include "ui_home.h"
#include "ui_theme.h"

namespace {

constexpr uint32_t kRefreshMs = 100;  // ui.md : jamais plus de 10 Hz.
constexpr int64_t kBootUs = 2 * 1000 * 1000;
int64_t g_started_us = 0;

void refresh_cb(lv_timer_t*) {
  ui::home::refresh(core::get_snapshot(), esp_timer_get_time() - g_started_us < kBootUs);
}

}  // namespace

namespace ui::root {

void build() {
  lv_obj_t* screen = lv_screen_active();
  lv_obj_clean(screen);
  lv_obj_set_style_bg_color(screen, theme::kBg, 0);
  lv_obj_set_style_bg_opa(screen, LV_OPA_COVER, 0);
  ui::home::create(screen);
  g_started_us = esp_timer_get_time();
  lv_timer_create(refresh_cb, kRefreshMs, nullptr);
}

}  // namespace ui::root

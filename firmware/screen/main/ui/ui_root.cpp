#include "ui_root.h"

#include "lvgl.h"

#include "ui_home.h"
#include "ui_theme.h"

namespace ui::root {

void build() {
  lv_obj_t* screen = lv_screen_active();
  lv_obj_clean(screen);
  lv_obj_set_style_bg_color(screen, theme::kBg, 0);
  lv_obj_set_style_bg_opa(screen, LV_OPA_COVER, 0);
  ui::home::create_static(screen);
}

}  // namespace ui::root

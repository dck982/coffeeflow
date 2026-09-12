#include "ui_home.h"
#include "common/version.hpp"
#include "core/core.h"
#include "esp_heap_caps.h"
#include "esp_timer.h"
#include "lvgl.h"
#include "ui_theme.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iterator>

namespace ui::home {
namespace {
using ulong = unsigned long;
enum class Role : uint8_t { Secondary, Primary, Destructive, Disabled };
enum class Edit : uint8_t {
  None,
  Weight,
  Time,
  PreTime,
  PrePressure,
  PrePump,
  RampTime,
  RampWeight,
  RampDrop,
  BrewPump,
  PurgePump,
  PurgeMax
};
enum class Choice : uint8_t { None, Preinfusion, Rampdown };
struct View {
  lv_obj_t *pressure{}, *temperature{}, *weight{}, *presence{}, *target{},
      *detail{}, *warning{}, *minus{}, *plus{}, *tap{}, *brew_button{}, *brew{},
      *purge{}, *settings_button{}, *cycle{}, *phase{}, *hero{},
      *cycle_detail{}, *progress{}, *stop{}, *settings{}, *index{}, *prev{},
      *next{}, *tile[6]{}, *tile_name[6]{}, *tile_value[6]{}, *diag{},
      *diag_val[9]{}, *diag_state[9]{}, *dot[9]{}, *keypad{}, *key_title{},
      *key_value{}, *key_unit{}, *key_error{}, *key_ok{}, *key_comma{},
      *choice{}, *choice_title{}, *choice_button[4]{}, *confirm{},
      *confirm_title{}, *confirm_body{}, *full{}, *full_title{}, *full_body{},
      *wifi_exit{}, *dim{}, *standby{}, *standby_title{}, *standby_body{};
} v;
constexpr size_t N = 96, L = 128;
struct Bind {
  lv_obj_t *l;
  char s[L];
};
// `EXT_RAM_BSS_ATTR` only moves BSS with CONFIG_SPIRAM_ALLOW_BSS_SEG_EXTERNAL_MEMORY.
// That option is deliberately off here, so use an explicit capabilities allocation:
// these 12.4 KiB must not take the internal RAM needed by RGB bounce buffers.
Bind *binds = nullptr;
size_t bn = 0;
int64_t activity = 0;
bool scale = false;
uint8_t page = 0;
Edit editing = Edit::None;
Choice choosing = Choice::None;
char candidate[24]{};
void note(lv_event_t *) { activity = esp_timer_get_time(); }
Bind *get(lv_obj_t *l) {
  for (size_t i = 0; i < bn; ++i)
    if (binds[i].l == l)
      return &binds[i];
  return nullptr;
}
void text(lv_obj_t *l, const char *s) {
  Bind *b = get(l);
  if (!b || !std::strcmp(b->s, s))
    return;
  lv_obj_invalidate(l);
  std::snprintf(b->s, L, "%s", s);
  lv_label_set_text_static(l, b->s);
}
void color(lv_obj_t *o, lv_color_t c) {
  if (!lv_color_eq(lv_obj_get_style_text_color(o, LV_PART_MAIN), c))
    lv_obj_set_style_text_color(o, c, 0);
}
void hidden(lv_obj_t *o, bool h) {
  if (h)
    lv_obj_add_flag(o, LV_OBJ_FLAG_HIDDEN);
  else
    lv_obj_remove_flag(o, LV_OBJ_FLAG_HIDDEN);
}
void lab(lv_obj_t *p, lv_obj_t **out, const char *s, const lv_font_t *f,
         lv_color_t c, int x, int y, bool d = false) {
  *out = lv_label_create(p);
  lv_obj_set_style_text_font(*out, f, 0);
  lv_obj_set_style_text_color(*out, c, 0);
  lv_obj_set_pos(*out, x, y);
  lv_label_set_text(*out, s);
  if (d && bn < N) {
    binds[bn].l = *out;
    std::snprintf(binds[bn].s, L, "%s", s);
    lv_label_set_text_static(*out, binds[bn++].s);
  }
}
void dyn(lv_obj_t *p, lv_obj_t **o, const char *s, const lv_font_t *f,
         lv_color_t c, int x, int y) {
  lab(p, o, s, f, c, x, y, true);
}
void bind(lv_obj_t *b) {
  if (bn >= N)
    return;
  lv_obj_t *l = lv_obj_get_child(b, 0);
  binds[bn].l = l;
  std::snprintf(binds[bn].s, L, "%s", lv_label_get_text(l));
  lv_label_set_text_static(l, binds[bn++].s);
}
// Lucide/ISC cup, droplet, sliders and chevrons are kept as recolourable vector
// strokes, never PNG.
lv_obj_t *box(lv_obj_t *p, int x, int y, int w, int h, lv_color_t c,
              int r = 2) {
  lv_obj_t *o = lv_obj_create(p);
  lv_obj_set_size(o, w, h);
  lv_obj_set_pos(o, x, y);
  lv_obj_set_style_bg_color(o, c, 0);
  lv_obj_set_style_border_width(o, 0, 0);
  lv_obj_set_style_outline_width(o, 0, 0);
  lv_obj_set_style_shadow_width(o, 0, 0);
  lv_obj_remove_flag(o, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_set_style_radius(o, r, 0);
  return o;
}
enum class Icon { Cup, Drop, Sliders, Left, Right, Back };
void stroke_path(lv_obj_t *parent, const lv_point_precise_t *points,
                 uint32_t point_count, int x, int y, lv_color_t color) {
  lv_obj_t *line = lv_line_create(parent);
  lv_line_set_points(line, points, point_count);
  lv_obj_set_pos(line, x, y);
  lv_obj_set_style_line_color(line, color, 0);
  lv_obj_set_style_line_width(line, 3, 0);
  lv_obj_set_style_line_rounded(line, true, 0);
  lv_obj_set_style_bg_opa(line, LV_OPA_TRANSP, 0);
  lv_obj_set_style_border_width(line, 0, 0);
  lv_obj_set_style_outline_width(line, 0, 0);
  lv_obj_set_style_shadow_width(line, 0, 0);
  lv_obj_remove_flag(line, LV_OBJ_FLAG_SCROLLABLE);
}
void icon(lv_obj_t *p, Icon i, int x, int y, lv_color_t c) {
  static constexpr lv_point_precise_t kChevronLeft[] = {
      {24, 4}, {10, 16}, {24, 28}};
  static constexpr lv_point_precise_t kChevronRight[] = {
      {8, 4}, {22, 16}, {8, 28}};
  static constexpr lv_point_precise_t kDrop[] = {{16, 2},  {7, 13},  {6, 18},
                                                 {8, 25},  {16, 29}, {24, 25},
                                                 {26, 18}, {25, 13}, {16, 2}};
  static constexpr lv_point_precise_t kBackOutline[] = {
      {28, 7}, {12, 7}, {4, 16}, {12, 25}, {28, 25}, {28, 7}};
  static constexpr lv_point_precise_t kBackCrossA[] = {{15, 12}, {23, 20}};
  static constexpr lv_point_precise_t kBackCrossB[] = {{23, 12}, {15, 20}};
  if (i == Icon::Cup) {
    box(p, x + 4, y + 10, 23, 15, c, 3);
    box(p, x + 27, y + 13, 5, 9, c);
    box(p, x + 8, y + 27, 17, 3, c);
  } else if (i == Icon::Drop) {
    stroke_path(p, kDrop, std::size(kDrop), x, y, c);
  } else if (i == Icon::Sliders) {
    for (int q : {7, 16, 25})
      box(p, x + 4, y + q, 28, 3, c);
    box(p, x + 10, y + 3, 6, 10, c, 3);
    box(p, x + 22, y + 12, 6, 10, c, 3);
    box(p, x + 15, y + 21, 6, 10, c, 3);
  } else if (i == Icon::Left) {
    stroke_path(p, kChevronLeft, std::size(kChevronLeft), x, y, c);
  } else if (i == Icon::Right) {
    stroke_path(p, kChevronRight, std::size(kChevronRight), x, y, c);
  } else {
    stroke_path(p, kBackOutline, std::size(kBackOutline), x, y, c);
    stroke_path(p, kBackCrossA, std::size(kBackCrossA), x, y, c);
    stroke_path(p, kBackCrossB, std::size(kBackCrossB), x, y, c);
  }
}
lv_obj_t *icon_container(lv_obj_t *parent, Icon icon_type, lv_color_t color) {
  lv_obj_t *container = lv_obj_create(parent);
  lv_obj_remove_style_all(container);
  lv_obj_set_size(container, 32, 32);
  lv_obj_remove_flag(container, LV_OBJ_FLAG_SCROLLABLE);
  icon(container, icon_type, 0, 0, color);
  return container;
}
lv_obj_t *button(lv_obj_t *p, int x, int y, int w, int h, const char *s,
                 Role r = Role::Secondary, Icon ic = Icon::Cup,
                 bool has = false) {
  constexpr auto press =
      static_cast<lv_style_selector_t>(static_cast<uint32_t>(LV_PART_MAIN) |
                                       static_cast<uint32_t>(LV_STATE_PRESSED));
  constexpr auto dis = static_cast<lv_style_selector_t>(
      static_cast<uint32_t>(LV_PART_MAIN) |
      static_cast<uint32_t>(LV_STATE_DISABLED));
  lv_obj_t *b = lv_button_create(p);
  lv_obj_set_size(b, w, h);
  lv_obj_set_pos(b, x, y);
  lv_obj_set_style_bg_opa(b, LV_OPA_COVER, 0);
  lv_obj_set_style_bg_color(
      b, r == Role::Primary ? theme::kSurfaceAccent : theme::kSurface, 0);
  lv_obj_set_style_bg_color(b, theme::kSurfaceHigh, press);
  lv_obj_set_style_border_width(b, 0, 0);
  lv_obj_set_style_border_width(b, 0, press);
  lv_obj_set_style_border_width(b, 0, dis);
  lv_obj_set_style_outline_width(b, 0, 0);
  lv_obj_set_style_outline_width(b, 0, press);
  lv_obj_set_style_outline_width(b, 0, dis);
  lv_obj_set_style_shadow_width(b, 0, 0);
  lv_obj_set_style_shadow_width(b, 0, press);
  lv_obj_set_style_shadow_width(b, 0, dis);
  lv_obj_set_style_radius(b, theme::kRadius, 0);
  lv_obj_t *l = lv_label_create(b);
  lv_obj_set_style_text_font(l, theme::kFontButton, 0);
  lv_obj_set_style_text_color(l,
                              r == Role::Primary       ? theme::kAccent
                              : r == Role::Destructive ? theme::kFault
                                                       : theme::kText,
                              0);
  lv_label_set_text(l, s);
  if (has) {
    lv_obj_align(l, LV_ALIGN_TOP_MID, 0, 42);
    lv_obj_t *glyph = icon_container(b, ic,
                                     r == Role::Primary       ? theme::kAccent
                                     : r == Role::Destructive ? theme::kFault
                                                              : theme::kText);
    lv_obj_align(glyph, LV_ALIGN_TOP_MID, 0, 7);
  } else
    lv_obj_center(l);
  lv_obj_set_style_text_color(l, theme::kText, press);
  if (r == Role::Disabled) {
    lv_obj_add_state(b, LV_STATE_DISABLED);
    lv_obj_set_style_bg_color(b, theme::kBgRaised, dis);
    lv_obj_set_style_text_color(l, theme::kTextFaint, dis);
  }
  lv_obj_add_event_cb(b, note, LV_EVENT_PRESSED, nullptr);
  return b;
}
void disable(lv_obj_t *b, bool d) {
  if (d)
    lv_obj_add_state(b, LV_STATE_DISABLED);
  else
    lv_obj_remove_state(b, LV_STATE_DISABLED);
}
void rule(lv_obj_t *p, int x, int y, int w, int h = 1) {
  box(p, x, y, w, h, theme::kHairline, 0);
}
void fmt(char *out, size_t n, float f, const char *s) {
  std::snprintf(out, n, "%.1f%s", static_cast<double>(f), s);
  for (char *p = out; *p; ++p)
    if (*p == '.')
      *p = ',';
}
bool present(core::Freshness f) { return f != core::Freshness::kMissing; }
void close_all() {
  hidden(v.settings, true);
  hidden(v.diag, true);
  hidden(v.keypad, true);
  hidden(v.choice, true);
}
void back_settings(lv_event_t *) { close_all(); }
void back_diag(lv_event_t *) { hidden(v.diag, true); }
void back_key(lv_event_t *) {
  editing = Edit::None;
  hidden(v.keypad, true);
  hidden(v.settings, false);
}
void back_choice(lv_event_t *) {
  choosing = Choice::None;
  hidden(v.choice, true);
  hidden(v.settings, false);
}
lv_obj_t *navbar(lv_obj_t *p, const char *title, lv_obj_t **out,
                 void (*back)(lv_event_t *), bool accept = false) {
  lv_obj_t *bar = box(p, 0, 0, 800, 88, theme::kBgRaised, 0);
  lv_obj_t *b = button(bar, 0, 0, 80, 80, "", Role::Secondary);
  lv_obj_align(b, LV_ALIGN_LEFT_MID, 16, 0);
  lv_obj_set_style_bg_color(b, theme::kBgRaised, 0);
  lv_obj_t *back_icon = icon_container(b, Icon::Left, theme::kText);
  lv_obj_center(back_icon);
  lv_obj_add_event_cb(b, back, LV_EVENT_CLICKED, nullptr);
  dyn(bar, out, title, theme::kFontButton, theme::kText, 0, 0);
  lv_obj_align(*out, LV_ALIGN_LEFT_MID, 112, 0);
  if (accept) {
    v.key_ok = button(bar, 0, 0, 160, 80, "valider", Role::Primary);
    lv_obj_align(v.key_ok, LV_ALIGN_RIGHT_MID, -32, 0);
    bind(v.key_ok);
  }
  return bar;
}
void base(lv_obj_t *p) {
  lv_obj_set_size(p, 800, 480);
  lv_obj_set_pos(p, 0, 0);
  lv_obj_set_style_bg_color(p, theme::kBg, 0);
  lv_obj_set_style_border_width(p, 0, 0);
  lv_obj_set_style_radius(p, 0, 0);
  lv_obj_set_style_pad_all(p, 0, 0);
  lv_obj_set_style_outline_width(p, 0, 0);
  lv_obj_set_style_shadow_width(p, 0, 0);
  lv_obj_remove_flag(p, LV_OBJ_FLAG_SCROLLABLE);
}
void set_title(Edit e) {
  const char *t = "réglage", *u = "";
  bool dec = false;
  switch (e) {
  case Edit::Weight:
    t = "cible poids";
    u = "g";
    dec = true;
    break;
  case Edit::Time:
    t = "cible temps";
    u = "s";
    break;
  case Edit::PreTime:
    t = "durée pré-inf.";
    u = "s";
    break;
  case Edit::PrePressure:
    t = "seuil pré-inf.";
    u = "bar";
    dec = true;
    break;
  case Edit::PrePump:
    t = "pompe pré-inf.";
    u = "%";
    break;
  case Edit::RampTime:
    t = "avance rampe";
    u = "s";
    dec = true;
    break;
  case Edit::RampWeight:
    t = "avance poids";
    u = "g";
    dec = true;
    break;
  case Edit::RampDrop:
    t = "chute pression";
    u = "bar";
    dec = true;
    break;
  case Edit::BrewPump:
    t = "pompe infusion";
    u = "%";
    break;
  case Edit::PurgePump:
    t = "pompe purge";
    u = "%";
    break;
  case Edit::PurgeMax:
    t = "purge max";
    u = "s";
    break;
  default:
    break;
  }
  text(v.key_title, t);
  text(v.key_unit, u);
  disable(v.key_comma, !dec);
}
void initial(Edit e) {
  auto c = core::get_config();
  float n = 0;
  bool d = false;
  switch (e) {
  case Edit::Weight:
    n = c.target_weight_g;
    d = true;
    break;
  case Edit::Time:
    n = c.target_time_s;
    break;
  case Edit::PreTime:
    n = c.preinfusion_time_s;
    break;
  case Edit::PrePressure:
    n = c.preinfusion_pressure_bar;
    d = true;
    break;
  case Edit::PrePump:
    n = c.preinfusion_pump_pct;
    break;
  case Edit::RampTime:
    n = c.rampdown_lead_time_s;
    d = true;
    break;
  case Edit::RampWeight:
    n = c.rampdown_lead_weight_g;
    d = true;
    break;
  case Edit::RampDrop:
    n = c.rampdown_pressure_drop_bar;
    d = true;
    break;
  case Edit::BrewPump:
    n = c.brew_pump_pct;
    break;
  case Edit::PurgePump:
    n = c.purge_pump_pct;
    break;
  case Edit::PurgeMax:
    n = c.purge_max_s;
    break;
  default:
    break;
  }
  if (d) {
    std::snprintf(candidate, sizeof(candidate), "%.1f", static_cast<double>(n));
    for (char *p = candidate; *p; ++p)
      if (*p == '.')
        *p = ',';
  } else
    std::snprintf(candidate, sizeof(candidate), "%u", static_cast<unsigned>(n));
}
bool valid(float *n) {
  char s[24];
  std::snprintf(s, sizeof(s), "%s", candidate);
  for (char *p = s; *p; ++p)
    if (*p == ',')
      *p = '.';
  char *end = nullptr;
  *n = std::strtof(s, &end);
  if (end == s || *end)
    return false;
  switch (editing) {
  case Edit::Weight:
    return *n >= 10 && *n <= 100 &&
           std::fabs(*n * 2 - std::round(*n * 2)) < .01;
  case Edit::Time:
    return *n >= 5 && *n <= 60 && std::floor(*n) == *n;
  case Edit::PreTime:
    return *n >= 0 && *n <= 20 && std::floor(*n) == *n;
  case Edit::PrePressure:
    return *n >= 1 && *n <= 9;
  case Edit::PrePump:
    return *n >= 0 && *n <= 100 && std::floor(*n) == *n;
  case Edit::RampTime:
    return *n >= 0 && *n <= 15;
  case Edit::RampWeight:
    return *n >= 0 && *n <= 20;
  case Edit::RampDrop:
    return *n >= .5 && *n <= 4;
  case Edit::BrewPump:
  case Edit::PurgePump:
    return *n >= 20 && *n <= 100 && std::floor(*n) == *n;
  case Edit::PurgeMax:
    return *n >= 5 && *n <= 60 && std::floor(*n) == *n;
  default:
    return false;
  }
}
void key_render() {
  float n;
  bool ok = valid(&n);
  text(v.key_value, candidate);
  lv_obj_update_layout(v.key_value);
  lv_obj_align_to(v.key_unit, v.key_value, LV_ALIGN_OUT_RIGHT_MID, 12, 0);
  text(v.key_error, ok ? "" : "valeur hors limites");
  disable(v.key_ok, !ok);
}
void key_press(lv_event_t *e) {
  auto *k = static_cast<const char *>(lv_event_get_user_data(e));
  if (!std::strcmp(k, "back")) {
    size_t n = std::strlen(candidate);
    if (n)
      candidate[n - 1] = 0;
  } else if (std::strlen(candidate) < sizeof(candidate) - 1)
    std::strcat(candidate, k);
  key_render();
}
void key_accept(lv_event_t *) {
  float n;
  if (!valid(&n)) {
    key_render();
    return;
  }
  auto c = core::get_config();
  switch (editing) {
  case Edit::Weight:
    c.target_weight_g = n;
    break;
  case Edit::Time:
    c.target_time_s = n;
    break;
  case Edit::PreTime:
    c.preinfusion_time_s = n;
    break;
  case Edit::PrePressure:
    c.preinfusion_pressure_bar = n;
    break;
  case Edit::PrePump:
    c.preinfusion_pump_pct = n;
    break;
  case Edit::RampTime:
    c.rampdown_lead_time_s = n;
    break;
  case Edit::RampWeight:
    c.rampdown_lead_weight_g = n;
    break;
  case Edit::RampDrop:
    c.rampdown_pressure_drop_bar = n;
    break;
  case Edit::BrewPump:
    c.brew_pump_pct = n;
    break;
  case Edit::PurgePump:
    c.purge_pump_pct = n;
    break;
  case Edit::PurgeMax:
    c.purge_max_s = n;
    break;
  default:
    return;
  }
  if (core::put_config(c).status == core::ConfigStatus::kOk)
    back_key(nullptr);
  else
    text(v.key_error, "valeur refusée");
}
void show_edit(Edit e) {
  editing = e;
  initial(e);
  set_title(e);
  close_all();
  hidden(v.keypad, false);
  key_render();
}
void show_target(lv_event_t *) { show_edit(scale ? Edit::Weight : Edit::Time); }
void render_settings();
void prev(lv_event_t *) {
  if (page) {
    --page;
    render_settings();
  }
}
void next(lv_event_t *) {
  if (page < 2) {
    ++page;
    render_settings();
  }
}
void show_settings(lv_event_t *) {
  page = 0;
  render_settings();
  close_all();
  hidden(v.settings, false);
}
void choose(lv_event_t *e) {
  unsigned i = reinterpret_cast<uintptr_t>(lv_event_get_user_data(e));
  auto c = core::get_config();
  if (choosing == Choice::Preinfusion)
    c.preinfusion_mode =
        i ? core::PreinfusionMode::kPressure : core::PreinfusionMode::kTime;
  else
    c.rampdown_mode = static_cast<core::RampdownMode>(i);
  if (core::put_config(c).status == core::ConfigStatus::kOk)
    back_choice(nullptr);
}
void show_choice(Choice q) {
  choosing = q;
  close_all();
  hidden(v.choice, false);
  text(v.choice_title,
       q == Choice::Preinfusion ? "stratégie pré-inf." : "stratégie rampe");
  const char *n[] = {"temps fixe", "attente pression", "aucune", "temps",
                     "poids",      "chute pression"};
  unsigned count = q == Choice::Preinfusion ? 2 : 4;
  for (unsigned i = 0; i < 4; ++i) {
    hidden(v.choice_button[i], i >= count);
    if (i < count) {
      text(lv_obj_get_child(v.choice_button[i], 0),
           q == Choice::Preinfusion ? n[i] : n[i + 2]);
      auto c = core::get_config();
      bool sel = q == Choice::Preinfusion ? unsigned(c.preinfusion_mode) == i
                                          : unsigned(c.rampdown_mode) == i;
      lv_obj_set_style_bg_color(v.choice_button[i],
                                sel ? theme::kSurfaceHigh : theme::kSurface, 0);
      color(lv_obj_get_child(v.choice_button[i], 0),
            sel ? theme::kAccent : theme::kText);
    }
  }
}
enum class Confirm : uint8_t { Wifi, Forget };
Confirm confirm = Confirm::Wifi;
void hide_confirm(lv_event_t *) { hidden(v.confirm, true); }
void accept_confirm(lv_event_t *) {
  if (confirm == Confirm::Wifi)
    core::request_radio_mode(core::RadioMode::kWifi);
  else
    core::forget_network();
  hidden(v.confirm, true);
}
void show_confirm(Confirm c) {
  confirm = c;
  text(v.confirm_title,
       c == Confirm::Wifi ? "mode wifi" : "réinitialiser le réseau");
  text(v.confirm_body, c == Confirm::Wifi
                           ? "quitter le mode machine et activer le réseau ?"
                           : "effacer les identifiants wifi enregistrés ?");
  hidden(v.confirm, false);
}
void tile_cb(lv_event_t *e) {
  unsigned i = reinterpret_cast<uintptr_t>(lv_event_get_user_data(e));
  if (page == 0) {
    if (i == 2)
      show_choice(Choice::Preinfusion);
    else {
      Edit a[] = {Edit::Weight,  Edit::Time,        Edit::None,
                  Edit::PreTime, Edit::PrePressure, Edit::PrePump};
      show_edit(a[i]);
    }
  } else if (page == 1) {
    if (i == 0)
      show_choice(Choice::Rampdown);
    else {
      Edit a[] = {Edit::None,     Edit::RampTime, Edit::RampWeight,
                  Edit::RampDrop, Edit::BrewPump, Edit::PurgePump};
      show_edit(a[i]);
    }
  } else {
    if (i == 0)
      show_edit(Edit::PurgeMax);
    else if (i == 1)
      show_confirm(Confirm::Wifi);
    else if (i == 2)
      show_confirm(Confirm::Forget);
  }
}
void tile(unsigned i, const char *n, const char *val,
          Role r = Role::Secondary) {
  text(v.tile_name[i], n);
  text(v.tile_value[i], val);
  color(v.tile_value[i], r == Role::Destructive ? theme::kFault : theme::kText);
}
void render_settings() {
  auto c = core::get_config();
  char x[6][40]{}, idx[8];
  std::snprintf(idx, sizeof(idx), "%u/3", page + 1);
  text(v.index, idx);
  disable(v.prev, page == 0);
  disable(v.next, page == 2);
  if (page == 0) {
    fmt(x[0], sizeof(x[0]), c.target_weight_g, " g");
    std::snprintf(x[1], 40, "%u s", c.target_time_s);
    std::snprintf(x[2], 40, "%s",
                  c.preinfusion_mode == core::PreinfusionMode::kTime
                      ? "temps fixe"
                      : "pression");
    std::snprintf(x[3], 40, "%u s", c.preinfusion_time_s);
    fmt(x[4], sizeof(x[4]), c.preinfusion_pressure_bar, " bar");
    std::snprintf(x[5], 40, "%u %%", c.preinfusion_pump_pct);
    const char *n[] = {"cible poids",    "cible temps",    "stratégie pré-inf.",
                       "durée pré-inf.", "seuil pré-inf.", "pompe pré-inf."};
    for (unsigned i = 0; i < 6; ++i)
      tile(i, n[i], x[i]);
  } else if (page == 1) {
    const char *m[] = {"aucune", "temps", "poids", "chute pression"};
    std::snprintf(x[0], 40, "%s", m[unsigned(c.rampdown_mode)]);
    fmt(x[1], sizeof(x[1]), c.rampdown_lead_time_s, " s");
    fmt(x[2], sizeof(x[2]), c.rampdown_lead_weight_g, " g");
    fmt(x[3], sizeof(x[3]), c.rampdown_pressure_drop_bar, " bar");
    std::snprintf(x[4], 40, "%u %%", c.brew_pump_pct);
    std::snprintf(x[5], 40, "%u %%", c.purge_pump_pct);
    const char *n[] = {"stratégie rampe", "avance temps",   "avance poids",
                       "chute pression",  "pompe infusion", "pompe purge"};
    for (unsigned i = 0; i < 6; ++i)
      tile(i, n[i], x[i]);
  } else {
    const char *n[] = {"purge max",    "wifi",     "réinitialiser réseau",
                       "calibrations", "firmware", "veille"};
    const char *val[] = {"",      "ouvrir",     "effacer", "depuis /config",
                         "écran", "automatique"};
    std::snprintf(x[0], 40, "%u s", c.purge_max_s);
    for (unsigned i = 0; i < 6; ++i)
      tile(i, n[i], i ? val[i] : x[0],
           i == 2 ? Role::Destructive : Role::Secondary);
  }
}
void target_step(int d) {
  auto c = core::get_config();
  if (scale)
    c.target_weight_g = std::clamp(c.target_weight_g + .5f * d, 10.f, 100.f);
  else
    c.target_time_s = std::clamp(int(c.target_time_s) + d, 5, 60);
  core::put_config(c);
}
void minus(lv_event_t *) { target_step(-1); }
void plus(lv_event_t *) { target_step(1); }
void purge_down(lv_event_t *) {
  core::perform_action({core::Action::kPurgePress});
}
void purge_up(lv_event_t *) {
  core::perform_action({core::Action::kPurgeRelease});
}
void brew(lv_event_t *) { core::perform_action({core::Action::kStartBrew}); }
void stop(lv_event_t *) {
  auto s = core::get_snapshot();
  core::perform_action({s.cycle_state == core::CycleState::kFinished
                            ? core::Action::kDismissSummary
                            : core::Action::kStopBrew});
}
bool active(const core::Snapshot &s) {
  return s.cycle_state == core::CycleState::kPreinfusion ||
         s.cycle_state == core::CycleState::kBrew ||
         s.cycle_state == core::CycleState::kRampdown ||
         s.cycle_state == core::CycleState::kPurge;
}
void cycle_visible(bool on) {
  hidden(v.cycle, !on);
  for (auto *o : {v.minus, v.plus, v.target, v.tap, v.detail, v.warning,
                  v.brew_button, v.purge, v.settings_button})
    hidden(o, on);
}
float progress(const core::Snapshot &s, const core::Config &c) {
  if (s.cycle_state == core::CycleState::kPreinfusion)
    return c.preinfusion_mode == core::PreinfusionMode::kTime
               ? float(s.cycle_phase_elapsed_ms) /
                     (c.preinfusion_time_s * 1000.f)
               : s.pressure_bar / c.preinfusion_pressure_bar;
  if (s.cycle_state == core::CycleState::kPurge)
    return float(s.cycle_elapsed_ms) / (c.purge_max_s * 1000.f);
  return s.cycle_weight_goal
             ? (s.weight_g - s.cycle_start_weight_g) / c.target_weight_g
             : float(s.cycle_elapsed_ms) / (c.target_time_s * 1000.f);
}
void cycle(const core::Snapshot &s, const core::Config &c) {
  bool a = active(s), done = s.cycle_state == core::CycleState::kFinished;
  cycle_visible(a || done);
  if (!a && !done)
    return;
  char t[96];
  if (done) {
    text(v.phase, "terminé");
    if (s.last_shot_available) {
      fmt(t, sizeof(t), s.last_shot_weight_g, " g");
      text(v.hero, t);
    } else
      text(v.hero, "-");
    lv_obj_set_width(v.progress, 420);
    color(v.hero, theme::kRampFull);
    text(lv_obj_get_child(v.stop, 0), "fermer");
    return;
  }
  text(v.phase, s.cycle_state == core::CycleState::kPurge ? "purge"
                : s.cycle_state == core::CycleState::kPreinfusion
                    ? "pré-infusion"
                : s.cycle_state == core::CycleState::kRampdown ? "rampe"
                                                               : "infusion");
  if (s.cycle_weight_goal && s.cycle_state != core::CycleState::kPurge)
    fmt(t, sizeof(t), s.weight_g - s.cycle_start_weight_g, " g");
  else
    std::snprintf(t, sizeof(t), "%lu s", ulong(s.cycle_elapsed_ms / 1000));
  text(v.hero, t);
  std::snprintf(t, sizeof(t), "%.1f bar · %.1f ml/s · pompe %u %%",
                double(s.pressure_bar), double(s.flow_ml_s), s.dimmer_pct);
  text(v.cycle_detail, t);
  lv_obj_set_width(v.progress, int(420 * std::clamp(progress(s, c), 0.f, 1.f)));
  lv_obj_set_style_bg_color(
      v.progress,
      s.cycle_state == core::CycleState::kPreinfusion ? theme::kRampLow
      : s.cycle_state == core::CycleState::kRampdown  ? theme::kRampFull
                                                      : theme::kAccent,
      0);
  color(v.hero, theme::kAccent);
  text(lv_obj_get_child(v.stop, 0), "arrêter");
}

void fullscreen(bool on, const char *t, const char *b) {
  if (!on) {
    hidden(v.full, true);
    return;
  }
  text(v.full_title, t);
  text(v.full_body, b);
  hidden(v.full, false);
}
void idle(const core::Snapshot &s) {
  if (active(s) || s.flash_active || s.lockout) {
    activity = esp_timer_get_time();
    hidden(v.dim, true);
    return;
  }
  auto i = uint64_t((esp_timer_get_time() - activity) / 1000000);
  auto c = core::get_config();
  if (i < c.dim_after_s) {
    hidden(v.dim, true);
    return;
  }
  hidden(v.dim, false);
  bool st = i >= c.standby_after_s;
  lv_obj_set_style_bg_opa(v.dim, st ? LV_OPA_90 : LV_OPA_50, 0);
  hidden(v.standby, !st);
  if (st) {
    int p = (i - c.standby_after_s) % 120, o = p < 60 ? p - 30 : 90 - p;
    lv_obj_set_pos(v.standby, 270 + o, 194 + o / 3);
    lv_obj_set_style_text_opa(
        v.standby_title,
        lv_opa_t(190 + std::abs(p < 30 ? p : (p < 90 ? 60 - p : p - 120)) * 2),
        0);
  }
}
} // namespace

void create(lv_obj_t *p) {
  if (binds == nullptr) {
    binds = static_cast<Bind *>(heap_caps_calloc(
        N, sizeof(*binds), MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT));
  } else {
    std::memset(binds, 0, N * sizeof(*binds));
  }
  if (binds == nullptr)
    return;
  bn = 0;
  activity = esp_timer_get_time();
  lv_obj_t *ignore = nullptr;
  char profile[40];
  std::snprintf(profile, sizeof(profile), "espresso v%u.%u.%u",
                common::kFirmwareVersionMajor, common::kFirmwareVersionMinor,
                common::kFirmwareVersionPatch);
  lab(p, &ignore, profile, theme::kFontStatus, theme::kText, 32, 24);
  dyn(p, &v.pressure, "-", theme::kFontStatus, theme::kTextDim, 405, 24);
  dyn(p, &v.temperature, "-", theme::kFontStatus, theme::kTextDim, 520, 24);
  dyn(p, &v.weight, "", theme::kFontStatus, theme::kTextDim, 615, 24);
  dyn(p, &v.presence, "bal  can", theme::kFontLabel, theme::kTextFaint, 705,
      31);
  lv_obj_t *t = lv_obj_create(p);
  lv_obj_remove_style_all(t);
  lv_obj_set_size(t, 120, 88);
  lv_obj_set_pos(t, 680, 8);
  lv_obj_add_flag(t, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_add_event_cb(t, note, LV_EVENT_PRESSED, nullptr);
  lv_obj_add_event_cb(
      t,
      [](lv_event_t *) {
        close_all();
        hidden(v.diag, false);
      },
      LV_EVENT_CLICKED, nullptr);
  // Les séparateurs ne font pas partie des chaînes dynamiques : leur position
  // reste stable lorsque la largeur d'une mesure change.
  rule(p, 384, 32, 1, 24);
  rule(p, 504, 32, 1, 24);
  rule(p, 600, 32, 1, 24);
  rule(p, 696, 32, 1, 24);
  dyn(p, &v.target, "-", theme::kFontHeroRest, theme::kText, 0, 112);
  lv_obj_set_width(v.target, 800);
  lv_obj_set_style_text_align(v.target, LV_TEXT_ALIGN_CENTER, 0);
  v.tap = lv_obj_create(p);
  lv_obj_remove_style_all(v.tap);
  lv_obj_set_size(v.tap, 288, 96);
  lv_obj_set_pos(v.tap, 256, 112);
  lv_obj_add_flag(v.tap, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_add_event_cb(v.tap, show_target, LV_EVENT_CLICKED, nullptr);
  v.minus = button(p, 256, 216, 136, 80, "-");
  v.plus = button(p, 408, 216, 136, 80, "+");
  lv_obj_set_style_text_font(lv_obj_get_child(v.minus, 0),
                             theme::kFontSecondary, 0);
  lv_obj_set_style_text_font(lv_obj_get_child(v.plus, 0), theme::kFontSecondary,
                             0);
  lv_obj_add_event_cb(v.minus, minus, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_event_cb(v.plus, plus, LV_EVENT_CLICKED, nullptr);
  dyn(p, &v.detail, "", theme::kFontLabel, theme::kTextFaint, 0, 312);
  lv_obj_set_width(v.detail, 800);
  lv_obj_set_style_text_align(v.detail, LV_TEXT_ALIGN_CENTER, 0);
  dyn(p, &v.warning, "", theme::kFontLabel, theme::kFault, 0, 336);
  lv_obj_set_width(v.warning, 800);
  lv_obj_set_style_text_align(v.warning, LV_TEXT_ALIGN_CENTER, 0);
  v.brew_button =
      button(p, 32, 368, 288, 88, "infuser", Role::Primary, Icon::Cup, true);
  v.brew = lv_obj_get_child(v.brew_button, 0);
  bind(v.brew_button);
  lv_obj_add_event_cb(v.brew_button, brew, LV_EVENT_CLICKED, nullptr);
  v.purge =
      button(p, 336, 368, 200, 88, "purge", Role::Secondary, Icon::Drop, true);
  lv_obj_add_event_cb(v.purge, purge_down, LV_EVENT_PRESSED, nullptr);
  lv_obj_add_event_cb(v.purge, purge_up, LV_EVENT_RELEASED, nullptr);
  lv_obj_add_event_cb(v.purge, purge_up, LV_EVENT_PRESS_LOST, nullptr);
  v.settings_button = button(p, 552, 368, 216, 88, "réglages", Role::Secondary,
                             Icon::Sliders, true);
  lv_obj_add_event_cb(v.settings_button, show_settings, LV_EVENT_CLICKED,
                      nullptr);
  v.cycle = lv_obj_create(p);
  lv_obj_remove_style_all(v.cycle);
  lv_obj_set_size(v.cycle, 800, 370);
  lv_obj_set_pos(v.cycle, 0, 96);
  dyn(v.cycle, &v.phase, "", theme::kFontLabel, theme::kAccent, 0, 24);
  lv_obj_set_width(v.phase, 800);
  lv_obj_set_style_text_align(v.phase, LV_TEXT_ALIGN_CENTER, 0);
  dyn(v.cycle, &v.hero, "", theme::kFontHeroBrew, theme::kAccent, 0, 54);
  lv_obj_set_width(v.hero, 800);
  lv_obj_set_style_text_align(v.hero, LV_TEXT_ALIGN_CENTER, 0);
  rule(v.cycle, 190, 186, 420, 2);
  v.progress = box(v.cycle, 190, 186, 0, 2, theme::kAccent, 0);
  dyn(v.cycle, &v.cycle_detail, "", theme::kFontSecondary, theme::kTextDim, 0,
      208);
  lv_obj_set_width(v.cycle_detail, 800);
  lv_obj_set_style_text_align(v.cycle_detail, LV_TEXT_ALIGN_CENTER, 0);
  v.stop = button(v.cycle, 240, 272, 320, 88, "arrêter", Role::Primary);
  bind(v.stop);
  lv_obj_add_event_cb(v.stop, stop, LV_EVENT_CLICKED, nullptr);
  hidden(v.cycle, true);
  v.settings = lv_obj_create(p);
  base(v.settings);
  lv_obj_t *settings_bar =
      navbar(v.settings, "réglages", &ignore, back_settings);
  dyn(settings_bar, &v.index, "1/3", theme::kFontButton, theme::kText, 0, 0);
  lv_obj_align(v.index, LV_ALIGN_LEFT_MID, 504, 0);
  v.prev = button(settings_bar, 0, 0, 80, 80, "");
  v.next = button(settings_bar, 0, 0, 80, 80, "");
  lv_obj_align(v.prev, LV_ALIGN_LEFT_MID, 592, 0);
  lv_obj_align(v.next, LV_ALIGN_LEFT_MID, 688, 0);
  lv_obj_t *prev_icon = icon_container(v.prev, Icon::Left, theme::kText);
  lv_obj_t *next_icon = icon_container(v.next, Icon::Right, theme::kText);
  lv_obj_center(prev_icon);
  lv_obj_center(next_icon);
  lv_obj_add_event_cb(v.prev, prev, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_event_cb(v.next, next, LV_EVENT_CLICKED, nullptr);
  for (unsigned i = 0; i < 6; ++i) {
    int x = i % 2 ? 408 : 32, y = 104 + (i / 2) * 104;
    v.tile[i] = button(v.settings, x, y, 360, 88, "");
    lv_obj_t *value = lv_obj_get_child(v.tile[i], 0);
    lv_obj_align(value, LV_ALIGN_TOP_LEFT, 16, 42);
    bind(v.tile[i]);
    v.tile_value[i] = value;
    dyn(v.settings, &v.tile_name[i], "", theme::kFontLabel, theme::kTextDim,
        x + 16, y + 12);
    lv_obj_add_event_cb(v.tile[i], tile_cb, LV_EVENT_CLICKED,
                        reinterpret_cast<void *>(uintptr_t(i)));
  }
  hidden(v.settings, true);
  v.diag = lv_obj_create(p);
  base(v.diag);
  navbar(v.diag, "diagnostic", &ignore, back_diag);
  const char *names[] = {"pression", "température", "débit",
                         "pompe",    "vanne",       "balance",
                         "bus can",  "réseau",      "versions"};
  for (unsigned i = 0; i < 9; ++i) {
    int x = 32 + (i % 3) * 248, y = 104 + (i / 3) * 112;
    v.dot[i] = box(v.diag, x, y + 4, 10, 10, theme::kTextFaint, 5);
    lab(v.diag, &ignore, names[i], theme::kFontLabel, theme::kTextDim, x + 18,
        y);
    dyn(v.diag, &v.diag_val[i], "-", theme::kFontButton, theme::kText, x,
        y + 28);
    dyn(v.diag, &v.diag_state[i], "", theme::kFontLabel, theme::kTextFaint, x,
        y + 62);
    if (i % 3 != 2)
      rule(v.diag, x + 240, y, 1, 96);
    if (i < 6)
      rule(v.diag, x, y + 96, 240);
  }
  hidden(v.diag, true);
  v.keypad = lv_obj_create(p);
  base(v.keypad);
  navbar(v.keypad, "cible", &v.key_title, back_key, true);
  lv_obj_add_event_cb(v.key_ok, key_accept, LV_EVENT_CLICKED, nullptr);
  lv_obj_t *key_value_row = lv_obj_create(v.keypad);
  lv_obj_remove_style_all(key_value_row);
  lv_obj_set_size(key_value_row, 208, 104);
  lv_obj_set_pos(key_value_row, 32, 144);
  lv_obj_remove_flag(key_value_row, LV_OBJ_FLAG_SCROLLABLE);
  dyn(key_value_row, &v.key_value, "", theme::kFontHeroRest, theme::kText, 0,
      0);
  lv_obj_align(v.key_value, LV_ALIGN_LEFT_MID, 0, 0);
  dyn(key_value_row, &v.key_unit, "", theme::kFontUnit, theme::kTextDim, 0, 0);
  lv_obj_align_to(v.key_unit, v.key_value, LV_ALIGN_OUT_RIGHT_MID, 12, 0);
  dyn(v.keypad, &v.key_error, "", theme::kFontLabel, theme::kFault, 32, 286);
  const char *keys[] = {"1", "2", "3", "4",    "5", "6",
                        "7", "8", "9", "back", "0", ","};
  for (unsigned i = 0; i < 12; ++i) {
    int x = 256 + (i % 3) * 176, y = 104 + (i / 3) * 96;
    lv_obj_t *b = button(v.keypad, x, y, 160, 80, keys[i]);
    bind(b);
    if (i == 9) {
      text(lv_obj_get_child(b, 0), "");
      lv_obj_t *backspace_icon = icon_container(b, Icon::Back, theme::kText);
      lv_obj_center(backspace_icon);
    }
    lv_obj_add_event_cb(b, key_press, LV_EVENT_CLICKED,
                        const_cast<char *>(keys[i]));
    if (i == 11)
      v.key_comma = b;
  }
  hidden(v.keypad, true);
  v.choice = lv_obj_create(p);
  base(v.choice);
  navbar(v.choice, "stratégie", &v.choice_title, back_choice);
  for (unsigned i = 0; i < 4; ++i) {
    int x = i % 2 ? 408 : 32, y = 120 + (i / 2) * 112;
    v.choice_button[i] = button(v.choice, x, y, 360, 88, "");
    bind(v.choice_button[i]);
    lv_obj_add_event_cb(v.choice_button[i], choose, LV_EVENT_CLICKED,
                        reinterpret_cast<void *>(uintptr_t(i)));
  }
  hidden(v.choice, true);
  v.confirm = lv_obj_create(p);
  lv_obj_set_size(v.confirm, 528, 248);
  lv_obj_set_pos(v.confirm, 136, 116);
  lv_obj_set_style_bg_color(v.confirm, theme::kBgRaised, 0);
  lv_obj_set_style_border_width(v.confirm, 0, 0);
  lv_obj_set_style_outline_width(v.confirm, 0, 0);
  lv_obj_set_style_shadow_width(v.confirm, 0, 0);
  lv_obj_set_style_width(v.confirm, 0, LV_PART_SCROLLBAR);
  lv_obj_remove_flag(v.confirm, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_set_style_radius(v.confirm, theme::kRadius, 0);
  box(v.confirm, 0, 0, 4, 248, theme::kAccent, 0);
  dyn(v.confirm, &v.confirm_title, "", theme::kFontSecondary, theme::kAccent,
      28, 24);
  dyn(v.confirm, &v.confirm_body, "", theme::kFontLabel, theme::kTextDim, 28,
      92);
  lv_obj_t *c = button(v.confirm, 28, 138, 216, 80, "annuler");
  lv_obj_t *o = button(v.confirm, 276, 138, 216, 80, "valider", Role::Primary);
  lv_obj_add_event_cb(c, hide_confirm, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_event_cb(o, accept_confirm, LV_EVENT_CLICKED, nullptr);
  hidden(v.confirm, true);
  v.full = lv_obj_create(p);
  base(v.full);
  dyn(v.full, &v.full_title, "coffeeflow", theme::kFontSecondary,
      theme::kAccent, 32, 154);
  dyn(v.full, &v.full_body, "", theme::kFontButton, theme::kTextDim, 32, 220);
  v.wifi_exit = button(v.full, 32, 300, 320, 88, "quitter le mode wifi");
  lv_obj_add_event_cb(
      v.wifi_exit,
      [](lv_event_t *) { core::request_radio_mode(core::RadioMode::kMachine); },
      LV_EVENT_CLICKED, nullptr);
  hidden(v.full, true);
  v.dim = lv_obj_create(p);
  lv_obj_set_size(v.dim, 800, 480);
  lv_obj_set_pos(v.dim, 0, 0);
  lv_obj_set_style_bg_color(v.dim, theme::kBg, 0);
  lv_obj_set_style_border_width(v.dim, 0, 0);
  lv_obj_set_style_width(v.dim, 0, LV_PART_SCROLLBAR);
  lv_obj_remove_flag(v.dim, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_add_flag(v.dim, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_add_event_cb(v.dim, note, LV_EVENT_PRESSED, nullptr);
  v.standby = lv_obj_create(v.dim);
  lv_obj_set_size(v.standby, 260, 92);
  lv_obj_set_pos(v.standby, 270, 194);
  lv_obj_set_style_bg_opa(v.standby, LV_OPA_TRANSP, 0);
  lv_obj_set_style_border_width(v.standby, 0, 0);
  lab(v.standby, &v.standby_title, "coffeeflow", theme::kFontSecondary,
      theme::kRampLow, 42, 8);
  lab(v.standby, &v.standby_body, "au repos", theme::kFontLabel,
      theme::kTextFaint, 92, 58);
  hidden(v.standby, true);
  hidden(v.dim, true);
}

void refresh(const core::Snapshot &s, bool boot) {
  char t[128];
  scale = s.scale_present;
  auto c = core::get_config();
  bool press = s.pressure_valid && present(s.pressure_freshness);
  if (press)
    fmt(t, sizeof(t), s.pressure_bar, " bar");
  else
    std::snprintf(t, sizeof(t), "-");
  text(v.pressure, t);
  color(v.pressure,
        !press ? theme::kTextFaint
        : s.pressure_freshness == core::Freshness::kStale
            ? theme::kTextDim
            : theme::ramp_color(s.pressure_bar,
                                s.cycle_state == core::CycleState::kPreinfusion
                                    ? c.preinfusion_pressure_bar
                                    : 9));
  if (press)
    fmt(t, sizeof(t), s.temperature_c, "°");
  else
    std::snprintf(t, sizeof(t), "-");
  text(v.temperature, t);
  color(v.temperature, !press ? theme::kTextFaint : theme::kThermal);
  lv_obj_set_style_text_opa(v.temperature,
                            s.pressure_freshness == core::Freshness::kStale
                                ? LV_OPA_60
                                : LV_OPA_COVER,
                            0);
  if (scale) {
    fmt(t, sizeof(t), s.weight_g, " g");
    text(v.weight, t);
  } else
    text(v.weight, "");
  color(v.weight, scale ? theme::kText : theme::kTextFaint);
  color(v.presence, s.sensors_alive ? theme::kText : theme::kTextFaint);
  if (scale) {
    fmt(t, sizeof(t), c.target_weight_g, " g");
    text(v.target, t);
    std::snprintf(t, sizeof(t), "infuser · %.0f g", double(c.target_weight_g));
  } else {
    std::snprintf(t, sizeof(t), "%u s", c.target_time_s);
    text(v.target, t);
    std::snprintf(t, sizeof(t), "infuser · %u s", c.target_time_s);
  }
  text(v.brew, t);
  std::snprintf(t, sizeof(t),
                scale ? "cible · pré-infusion %u s · rampe"
                      : "cible temps · balance absente",
                c.preinfusion_time_s);
  text(v.detail, t);
  text(v.warning, (!s.dimmer_ready || !s.dimmer_valid) && s.sensors_alive
                      ? "dimmer en calibration"
                      : "");
  disable(v.brew_button, !s.dimmer_ready || !s.dimmer_valid);
  disable(v.minus, scale ? c.target_weight_g <= 10 : c.target_time_s <= 5);
  disable(v.plus, scale ? c.target_weight_g >= 100 : c.target_time_s >= 60);
  cycle(s, c);
  if (!lv_obj_has_flag(v.diag, LV_OBJ_FLAG_HIDDEN)) {
    bool flow = s.flow_valid && present(s.flow_freshness),
         states[] = {press,
                     press,
                     flow,
                     s.dimmer_valid,
                     s.valve_open,
                     scale,
                     s.sensors_alive,
                     s.radio_mode == core::RadioMode::kWifi && s.ipv4_address,
                     true};
    for (unsigned i = 0; i < 9; ++i) {
      lv_obj_set_style_bg_color(v.dot[i],
                                (i == 3 && s.dimmer_error_active) ||
                                        (i == 6 && !s.sensors_alive)
                                    ? theme::kFault
                                : states[i] ? theme::kAccent
                                            : theme::kTextFaint,
                                0);
      text(v.diag_state[i], states[i] ? "valide" : "absent");
    }
    if (press) {
      fmt(t, sizeof(t), s.pressure_bar, " bar");
      text(v.diag_val[0], t);
      fmt(t, sizeof(t), s.temperature_c, "°");
      text(v.diag_val[1], t);
    } else {
      text(v.diag_val[0], "-");
      text(v.diag_val[1], "-");
    }
    if (flow)
      fmt(t, sizeof(t), s.flow_ml_s, " ml/s");
    else
      std::snprintf(t, sizeof(t), "-");
    text(v.diag_val[2], t);
    std::snprintf(t, sizeof(t), "%u %%", s.dimmer_pct);
    text(v.diag_val[3], t);
    text(v.diag_val[4], s.valve_open ? "ouverte" : "fermée");
    if (scale)
      fmt(t, sizeof(t), s.weight_g, " g");
    else
      std::snprintf(t, sizeof(t), "-");
    text(v.diag_val[5], t);
    text(v.diag_val[6], s.sensors_alive ? "ok" : "perdu");
    text(v.diag_val[7],
         s.radio_mode == core::RadioMode::kWifi ? "wifi" : "machine");
    std::snprintf(t, sizeof(t), "%u.%u.%u", s.screen_version_major,
                  s.screen_version_minor, s.screen_version_patch);
    text(v.diag_val[8], t);
  }
  if (s.lockout)
    fullscreen(
        true, "verrou de sécurité",
        "couper la machine à l'interrupteur principal, puis la rallumer");
  else if (!s.sensors_alive && !boot)
    fullscreen(true, "module interne injoignable",
               "les commandes de pompe et de vanne sont coupées");
  else if (s.flash_active)
    fullscreen(true, "mise à jour", "ne pas couper la machine");
  else if (s.radio_mode == core::RadioMode::kWifi) {
    if (s.radio_transition)
      std::snprintf(t, sizeof(t), "activation du réseau");
    else if (s.ipv4_address)
      std::snprintf(t, sizeof(t), "adresse ip · %u.%u.%u.%u",
                    static_cast<unsigned>((s.ipv4_address >> 24) & 255),
                    static_cast<unsigned>((s.ipv4_address >> 16) & 255),
                    static_cast<unsigned>((s.ipv4_address >> 8) & 255),
                    static_cast<unsigned>(s.ipv4_address & 255));
    else
      std::snprintf(t, sizeof(t), "configuration wifi ou association en cours");
    fullscreen(true, "wifi mode", t);
  } else if (boot)
    fullscreen(true, "coffeeflow", "démarrage");
  else
    fullscreen(false, "", "");
  idle(s);
}

#ifdef UI_SIM
void snapshot_scenario(const char *scenario) {
  if (std::strcmp(scenario, "settings") == 0 ||
      std::strcmp(scenario, "settings1") == 0 ||
      std::strcmp(scenario, "settings2") == 0) {
    page = static_cast<uint8_t>(std::strcmp(scenario, "settings2") == 0   ? 2
                                : std::strcmp(scenario, "settings1") == 0 ? 1
                                                                          : 0);
    render_settings();
    close_all();
    hidden(v.settings, false);
  } else if (std::strcmp(scenario, "keypad-weight") == 0) {
    show_edit(Edit::Weight);
  } else if (std::strcmp(scenario, "keypad-time") == 0) {
    show_edit(Edit::Time);
  } else if (std::strcmp(scenario, "diagnostic") == 0) {
    close_all();
    hidden(v.diag, false);
  } else if (std::strcmp(scenario, "wifi-confirm") == 0) {
    page = 2;
    render_settings();
    close_all();
    hidden(v.settings, false);
    show_confirm(Confirm::Wifi);
  } else if (std::strcmp(scenario, "pressed") == 0) {
    lv_obj_add_state(v.brew_button, LV_STATE_PRESSED);
  }
}
#endif
} // namespace ui::home

#include "ui_home.h"
#include "common/version.hpp"
#include "core/core.h"
#include "service_screen.h"
#include "esp_heap_caps.h"
#include "esp_timer.h"
#include "lvgl.h"
#include "ui_theme.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <ctime>
#include <cstdlib>
#include <cstring>
#include <iterator>

#if defined(UI_SIM)
extern char g_sim_clock_override[6];
#endif

namespace ui::home {
namespace {
using ulong = unsigned long;
// Délai avant la récupération automatique d'un DimmerLink qui reste en
// calibration. Ajuster cette constante plutôt que la logique de suivi.
constexpr int64_t kDimmerCalibrationResetDelayS = 10;
// Espacement horizontal uniforme du bandeau ; le réduire condense toutes ses
// cellules sans modifier les largeurs réservées aux textes et aux icônes.
constexpr int kTopbarGap = 12;
// ui_font_28 a une hauteur de ligne légèrement supérieure à 32 px : ces deux
// pixels empêchent le parent Flex de rogner les descendantes (notamment le g).
constexpr int kTopbarHeight = 34;
enum class Role : uint8_t { Secondary, Primary, Destructive, Disabled };
enum class Edit : uint8_t {
  None,
  Weight,
  Time,
  BrewPressure,
  BrewTemperature,
  FillingTime,
  FillingPressureTarget,
  FillingPump,
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
enum class KeypadMode : uint8_t { Integer, Decimal };
struct View {
  lv_obj_t *pressure{}, *temperature{}, *weight{}, *weight_group{}, *weight_content{}, *diagnostic{},
      *clock{}, *target{},
      *detail{}, *warning{}, *minus{}, *plus{}, *tap{}, *brew_button{}, *brew{},
      *purge{}, *settings_button{}, *cycle{}, *phase{}, *hero{},
      *hero_time{}, *hero_divider{},
      *cycle_detail{}, *progress{}, *stop{}, *settings{}, *index{}, *prev{},
      *next{}, *tile[6]{}, *tile_name[6]{}, *tile_value[6]{}, *diag{},
      *diag_val[9]{}, *diag_state[9]{}, *dot[9]{}, *keypad{}, *key_title{},
      *key_value{}, *key_unit{}, *key_error{}, *key_ok{}, *key_comma{},
      *choice{}, *choice_title{}, *choice_button[4]{}, *confirm{},
      *confirm_title{}, *confirm_body{}, *full{}, *full_title{}, *full_body{},
      *wifi_exit{}, *dimmer_menu{}, *dimmer_menu_title{}, *dimmer_menu_body{},
      *dimmer_reset{}, *dimmer_recalibrate{}, *dimmer_close{}, *dim{},
      *standby{}, *standby_title{}, *standby_body{};
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
KeypadMode keypad_mode = KeypadMode::Integer;
char candidate[24]{};
bool candidate_edited = false;
int64_t dimmer_calibration_started_us = 0;
bool dimmer_calibration_reset_sent = false;

void recover_stuck_dimmer_calibration(const core::Snapshot& s) {
  // Un passage par l'état prêt et valide réarme la récupération pour la
  // prochaine calibration. Une perte temporaire du module ne doit pas, elle,
  // permettre une seconde tentative dans la même calibration.
  if (s.dimmer_ready && s.dimmer_valid) {
    dimmer_calibration_started_us = 0;
    dimmer_calibration_reset_sent = false;
    return;
  }

  const bool calibration_displayed =
      s.sensors_alive && (!s.dimmer_ready || !s.dimmer_valid);
  if (!calibration_displayed) {
    dimmer_calibration_started_us = 0;
    return;
  }

  const int64_t now_us = esp_timer_get_time();
  if (dimmer_calibration_started_us == 0)
    dimmer_calibration_started_us = now_us;
  if (!dimmer_calibration_reset_sent &&
      now_us - dimmer_calibration_started_us >=
          kDimmerCalibrationResetDelayS * 1000 * 1000) {
    // Verrouiller avant l'envoi : même si la commande est refusée (par exemple
    // pendant un cycle), cette calibration ne doit provoquer qu'une tentative.
    dimmer_calibration_reset_sent = true;
    static_cast<void>(core::perform_action({core::Action::kResetDimmer}));
  }
}

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
enum class Icon { Cup, Drop, Sliders, Left, Right, Back, Wifi };
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
  static constexpr lv_point_precise_t kWifiOuter[] = {
      {3, 14}, {7, 10}, {11, 7}, {16, 6}, {21, 7}, {25, 10}, {29, 14}};
  static constexpr lv_point_precise_t kWifiInner[] = {
      {8, 20}, {11, 17}, {16, 15}, {21, 17}, {24, 20}};
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
  } else if (i == Icon::Wifi) {
    stroke_path(p, kWifiOuter, std::size(kWifiOuter), x, y, c);
    stroke_path(p, kWifiInner, std::size(kWifiInner), x, y, c);
    box(p, x + 14, y + 24, 4, 4, c, 2);
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
    lv_obj_align(l, LV_ALIGN_TOP_MID, 0, 28);
    lv_obj_t *glyph = icon_container(b, ic,
                                     r == Role::Primary       ? theme::kAccent
                                     : r == Role::Destructive ? theme::kFault
                                                              : theme::kText);
    lv_obj_align(glyph, LV_ALIGN_TOP_MID, 0, -7);
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
lv_obj_t *spacer(lv_obj_t *p, int w) {
  lv_obj_t *o = lv_obj_create(p);
  lv_obj_remove_style_all(o);
  lv_obj_set_size(o, w, 1);
  lv_obj_remove_flag(o, LV_OBJ_FLAG_SCROLLABLE);
  return o;
}
lv_obj_t *topbar_group(lv_obj_t *p, int w, bool grow = false) {
  lv_obj_t *o = lv_obj_create(p);
  lv_obj_remove_style_all(o);
  lv_obj_set_size(o, w, kTopbarHeight);
  lv_obj_remove_flag(o, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_set_flex_flow(o, LV_FLEX_FLOW_ROW);
  lv_obj_set_flex_align(o, LV_FLEX_ALIGN_START, LV_FLEX_ALIGN_CENTER,
                        LV_FLEX_ALIGN_CENTER);
  if (grow)
    lv_obj_set_flex_grow(o, 1);
  return o;
}
void topbar_rule(lv_obj_t *p) {
  lv_obj_t *o = box(p, 0, 0, 1, 24, theme::kHairline, 0);
  lv_obj_remove_flag(o, LV_OBJ_FLAG_SCROLLABLE);
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
  hidden(v.dimmer_menu, true);
}
void show_diagnostics(lv_event_t *) {
  close_all();
  hidden(v.diag, false);
}
void render_settings();
void back_settings(lv_event_t *) { close_all(); }
enum class Confirm : uint8_t { Wifi, Forget };
void show_confirm(Confirm c);
void wifi_nav(lv_event_t *) { show_confirm(Confirm::Wifi); }
void back_diag(lv_event_t *) { hidden(v.diag, true); }
void close_dimmer_menu(lv_event_t *) { hidden(v.dimmer_menu, true); }
void run_dimmer_action(lv_event_t *event) {
  auto action = static_cast<core::Action>(reinterpret_cast<uintptr_t>(lv_event_get_user_data(event)));
  core::ActionResult result = core::perform_action({action});
  if (result.status == core::ActionStatus::kOk) {
    hidden(v.dimmer_menu, true);
    return;
  }
  text(v.dimmer_menu_body, result.status == core::ActionStatus::kBusLost
                               ? "module capteurs injoignable"
                               : result.status == core::ActionStatus::kCycleActive
                                     ? "commande refusée pendant un cycle"
                                     : "commande indisponible");
}
void show_dimmer_menu(lv_event_t *) {
  text(v.dimmer_menu_body, "la pompe est arrêtée avant la commande");
  hidden(v.dimmer_menu, false);
}
void back_key(lv_event_t *) {
  editing = Edit::None;
  hidden(v.keypad, true);
  render_settings();
  hidden(v.settings, false);
}
void back_choice(lv_event_t *) {
  choosing = Choice::None;
  hidden(v.choice, true);
  render_settings();
  hidden(v.settings, false);
}
lv_obj_t *navbar(lv_obj_t *p, const char *title, lv_obj_t **out,
                 void (*back)(lv_event_t *), bool accept = false,
                 bool wifi = false) {
  lv_obj_t *bar = box(p, 0, 0, 800, 88, theme::kBgRaised, 0);
  lv_obj_t *b = button(bar, 0, 0, 80, 80, "", Role::Secondary);
  lv_obj_align(b, LV_ALIGN_LEFT_MID, 16, 0);
  lv_obj_set_style_bg_color(b, theme::kBgRaised, 0);
  lv_obj_t *back_icon = icon_container(b, Icon::Left, theme::kText);
  lv_obj_center(back_icon);
  lv_obj_add_event_cb(b, back, LV_EVENT_CLICKED, nullptr);
  dyn(bar, out, title, theme::kFontButton, theme::kText, 0, 0);
  lv_obj_align(*out, LV_ALIGN_LEFT_MID, 112, 0);
  if (wifi) {
    lv_obj_t *wifi_button = button(bar, 0, 0, 80, 80, "");
    lv_obj_align(wifi_button, LV_ALIGN_CENTER, 0, 0);
    lv_obj_set_style_bg_color(wifi_button, theme::kBgRaised, 0);
    lv_obj_t *wifi_icon = icon_container(wifi_button, Icon::Wifi, theme::kText);
    lv_obj_center(wifi_icon);
    lv_obj_add_event_cb(wifi_button, wifi_nav, LV_EVENT_CLICKED, nullptr);
  }
  if (accept) {
    v.key_ok = button(bar, 0, 0, 160, 80, "valider", Role::Primary);
    lv_obj_align(v.key_ok, LV_ALIGN_RIGHT_MID, 0, 0);
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
KeypadMode keypad_mode_for(Edit e) {
  switch (e) {
  // Ces grandeurs acceptent des fractions dans leur plage de validation.
  case Edit::Weight:
  case Edit::BrewPressure:
  case Edit::BrewTemperature:
  case Edit::PrePressure:
  case Edit::FillingPressureTarget:
  case Edit::RampTime:
  case Edit::RampWeight:
  case Edit::RampDrop:
    return KeypadMode::Decimal;

  // Durées et niveaux de pompe sont stockés et validés comme entiers.
  case Edit::Time:
  case Edit::FillingTime:
  case Edit::FillingPump:
  case Edit::PreTime:
  case Edit::PrePump:
  case Edit::BrewPump:
  case Edit::PurgePump:
  case Edit::PurgeMax:
  case Edit::None:
    return KeypadMode::Integer;
  }
  return KeypadMode::Integer;
}
void set_title(Edit e) {
  const char *t = "réglage", *u = "";
  keypad_mode = keypad_mode_for(e);
  switch (e) {
  case Edit::Weight:
    t = "cible poids";
    u = "g";
    break;
  case Edit::Time:
    t = "cible temps";
    u = "s";
    break;
  case Edit::BrewPressure:
    t = "cible pression";
    u = "bar";
    break;
  case Edit::BrewTemperature:
    t = "cible chaudière";
    u = "°C";
    break;
  case Edit::FillingTime:
    t = "durée remplissage";
    u = "s";
    break;
  case Edit::FillingPressureTarget:
    t = "cible pression rempl.";
    u = "bar";
    break;
  case Edit::FillingPump:
    t = "pompe remplissage";
    u = "%";
    break;
  case Edit::PreTime:
    t = "durée pré-inf.";
    u = "s";
    break;
  case Edit::PrePressure:
    t = "seuil pré-inf.";
    u = "bar";
    break;
  case Edit::PrePump:
    t = "pompe pré-inf.";
    u = "%";
    break;
  case Edit::RampTime:
    t = "avance rampe";
    u = "s";
    break;
  case Edit::RampWeight:
    t = "avance poids";
    u = "g";
    break;
  case Edit::RampDrop:
    t = "chute pression";
    u = "bar";
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
  disable(v.key_comma, keypad_mode == KeypadMode::Integer);
}
void initial(Edit e) {
  auto c = core::get_config();
  float n = 0;
  unsigned decimals = 0;
  switch (e) {
  case Edit::Weight:
    n = c.target_weight_g;
    decimals = 1;
    break;
  case Edit::Time:
    n = c.target_time_s;
    break;
  case Edit::BrewPressure:
    n = c.target_pressure_bar;
    decimals = 1;
    break;
  case Edit::BrewTemperature:
    n = c.brew_temperature_c;
    decimals = 1;
    break;
  case Edit::FillingTime:
    n = c.filling_time_s;
    break;
  case Edit::FillingPressureTarget:
    n = c.filling_pressure_target_bar;
    decimals = 1;
    break;
  case Edit::FillingPump:
    n = c.filling_pump_pct;
    break;
  case Edit::PreTime:
    n = c.preinfusion_time_s;
    break;
  case Edit::PrePressure:
    n = c.preinfusion_pressure_bar;
    decimals = 1;
    break;
  case Edit::PrePump:
    n = c.preinfusion_pump_pct;
    break;
  case Edit::RampTime:
    n = c.rampdown_lead_time_s;
    decimals = 1;
    break;
  case Edit::RampWeight:
    n = c.rampdown_lead_weight_g;
    decimals = 1;
    break;
  case Edit::RampDrop:
    n = c.rampdown_pressure_drop_bar;
    decimals = 1;
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
  if (decimals != 0) {
    std::snprintf(candidate, sizeof(candidate), decimals == 2 ? "%.2f" : "%.1f",
                  static_cast<double>(n));
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
  case Edit::BrewPressure:
    return *n >= 6 && *n <= 12 &&
           std::fabs(*n * 10 - std::round(*n * 10)) < .01f;
  case Edit::BrewTemperature:
    return *n >= core::kMinimumBrewTemperatureC && *n <= 100 &&
           std::fabs(*n * 2 - std::round(*n * 2)) < .01f;
  case Edit::FillingTime:
    return *n >= 1 && *n <= 10 && std::floor(*n) == *n;
  case Edit::FillingPressureTarget:
    return *n >= .1f && *n <= 1.0f &&
           std::fabs(*n * 10 - std::round(*n * 10)) < .01f;
  case Edit::FillingPump:
    return *n >= 20 && *n <= 100 && std::floor(*n) == *n &&
           static_cast<unsigned>(*n) % 5 == 0;
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
    return *n >= core::kMinimumBrewPumpPct && *n <= 100 && std::floor(*n) == *n &&
           static_cast<unsigned>(*n) % 5 == 0;
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
  if (keypad_mode == KeypadMode::Integer && !std::strcmp(k, ","))
    return;
  if (!candidate_edited) {
    candidate[0] = '\0';
    candidate_edited = true;
  }
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
  case Edit::BrewPressure:
    c.target_pressure_bar = n;
    break;
  case Edit::BrewTemperature:
    c.brew_temperature_c = n;
    break;
  case Edit::FillingTime:
    c.filling_time_s = n;
    break;
  case Edit::FillingPressureTarget:
    c.filling_pressure_target_bar = n;
    break;
  case Edit::FillingPump:
    c.filling_pump_pct = n;
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
  candidate_edited = false;
  set_title(e);
  close_all();
  hidden(v.keypad, false);
  key_render();
}
void show_target(lv_event_t *) { show_edit(scale ? Edit::Weight : Edit::Time); }
void prev(lv_event_t *) {
  if (page) {
    --page;
    render_settings();
  }
}
void next(lv_event_t *) {
  if (page < 3) {
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
void show_choice(Choice q);
void choose(lv_event_t *e) {
  unsigned i = reinterpret_cast<uintptr_t>(lv_event_get_user_data(e));
  auto c = core::get_config();
  if (choosing == Choice::Preinfusion) {
    const uint8_t bit = static_cast<uint8_t>(1u << i);
    c.preinfusion_mode = static_cast<core::PreinfusionMode>(
        static_cast<uint8_t>(c.preinfusion_mode) ^ bit);
  } else {
    c.rampdown_mode = static_cast<core::RampdownMode>(i);
  }
  if (core::put_config(c).status == core::ConfigStatus::kOk) {
    if (choosing == Choice::Preinfusion)
      show_choice(Choice::Preinfusion);
    else
      back_choice(nullptr);
  }
}
void show_choice(Choice q) {
  choosing = q;
  close_all();
  hidden(v.choice, false);
  text(v.choice_title,
       q == Choice::Preinfusion ? "pré-infusion" : "stratégie rampe");
  const char *n[] = {"temps", "pression", "poids", "aucune", "temps",
                     "poids", "chute pression"};
  unsigned count = q == Choice::Preinfusion ? 3 : 4;
  for (unsigned i = 0; i < 4; ++i) {
    hidden(v.choice_button[i], i >= count);
    if (i < count) {
      text(lv_obj_get_child(v.choice_button[i], 0),
           q == Choice::Preinfusion ? n[i] : n[i + 3]);
      auto c = core::get_config();
      bool sel = q == Choice::Preinfusion
                     ? (static_cast<uint8_t>(c.preinfusion_mode) & (1u << i)) != 0
                     : unsigned(c.rampdown_mode) == i;
      const bool preinfusion_toggle = q == Choice::Preinfusion;
      lv_obj_set_style_bg_color(v.choice_button[i],
                                sel && preinfusion_toggle
                                    ? theme::kSuccess
                                    : sel ? theme::kSurfaceHigh : theme::kSurface,
                                0);
      color(lv_obj_get_child(v.choice_button[i], 0),
            sel && preinfusion_toggle ? theme::kBg : sel ? theme::kAccent : theme::kText);
    }
  }
}
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
    Edit a[] = {Edit::Time, Edit::FillingPump, Edit::Weight,
                Edit::PrePump, Edit::BrewPressure, Edit::BrewPump};
    if (a[i] != Edit::None) show_edit(a[i]);
  } else if (page == 1) {
    if (i == 1)
      show_choice(Choice::Preinfusion);
    else {
      Edit a[] = {Edit::FillingPressureTarget, Edit::None, Edit::FillingTime,
                  Edit::PreTime, Edit::BrewTemperature, Edit::PrePressure};
      if (a[i] != Edit::None) show_edit(a[i]);
    }
  } else if (page == 2) {
    if (i == 0)
      show_choice(Choice::Rampdown);
    else {
      Edit a[] = {Edit::None,     Edit::RampTime, Edit::RampWeight,
                  Edit::RampDrop, Edit::PurgePump, Edit::PurgeMax};
      show_edit(a[i]);
    }
  } else {
    if (i == 0)
      show_confirm(Confirm::Forget);
    else if (i == 2)
      service_screen::restart_lcd();
  }
}
const char *preinfusion_mode_text(core::PreinfusionMode mode, char *buffer, size_t size) {
  if (mode == core::PreinfusionMode::kNone) {
    std::snprintf(buffer, size, "aucune");
    return buffer;
  }
  bool first = true;
  buffer[0] = '\0';
  const char *names[] = {"temps", "pression", "poids"};
  for (unsigned i = 0; i < 3; ++i) {
    if ((static_cast<uint8_t>(mode) & (1u << i)) == 0) continue;
    std::snprintf(buffer + std::strlen(buffer), size - std::strlen(buffer),
                  "%s%s", first ? "" : " + ", names[i]);
    first = false;
  }
  return buffer;
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
  std::snprintf(idx, sizeof(idx), "%u/4", page + 1);
  text(v.index, idx);
  // Les flèches qui n'ont pas de destination ne doivent pas apparaître :
  // affichées mais désactivées, elles pouvaient conserver un rendu "pressed"
  // lors de l'entrée dans les réglages.
  hidden(v.prev, page == 0);
  hidden(v.next, page == 3);
  lv_obj_remove_state(v.prev, LV_STATE_PRESSED);
  lv_obj_remove_state(v.next, LV_STATE_PRESSED);
  for (unsigned i = 0; i < 6; ++i) {
    hidden(v.tile[i], false);
    hidden(v.tile_name[i], false);
    disable(v.tile[i], false);
  }
  if (page == 0) {
    std::snprintf(x[0], 40, "%u s", c.target_time_s);
    std::snprintf(x[1], 40, "%u %%", c.filling_pump_pct);
    fmt(x[2], sizeof(x[2]), c.target_weight_g, " g");
    std::snprintf(x[3], 40, "%u %%", c.preinfusion_pump_pct);
    fmt(x[4], sizeof(x[4]), c.target_pressure_bar, " bar");
    std::snprintf(x[5], 40, "%u %%", c.brew_pump_pct);
    const char *n[] = {"cible temps", "pompe remplissage", "cible poids",
                       "pompe pré-inf.", "cible pression", "pompe infusion"};
    for (unsigned i = 0; i < 6; ++i)
      tile(i, n[i], x[i]);
  } else if (page == 1) {
    std::snprintf(x[0], 40, "%.1f bar", double(c.filling_pressure_target_bar));
    preinfusion_mode_text(c.preinfusion_mode, x[1], sizeof(x[1]));
    std::snprintf(x[2], 40, "%u s", c.filling_time_s);
    std::snprintf(x[3], 40, "%u s", c.preinfusion_time_s);
    fmt(x[4], sizeof(x[4]), c.brew_temperature_c, " °C");
    fmt(x[5], sizeof(x[5]), c.preinfusion_pressure_bar, " bar");
    const char *n[] = {"cible pression rempl.", "critères pré-inf.", "durée remplissage",
                       "échéance pré-inf.", "cible chaudière", "seuil pression pré-inf."};
    for (unsigned i = 0; i < 6; ++i) tile(i, n[i], x[i]);
  } else if (page == 2) {
    const char *m[] = {"aucune", "temps", "poids", "chute pression"};
    std::snprintf(x[0], 40, "%s", m[unsigned(c.rampdown_mode)]);
    fmt(x[1], sizeof(x[1]), c.rampdown_lead_time_s, " s");
    fmt(x[2], sizeof(x[2]), c.rampdown_lead_weight_g, " g");
    fmt(x[3], sizeof(x[3]), c.rampdown_pressure_drop_bar, " bar");
    std::snprintf(x[4], 40, "%u %%", c.purge_pump_pct);
    std::snprintf(x[5], 40, "%u s", c.purge_max_s);
    const char *n[] = {"stratégie rampe", "avance temps",   "avance poids",
                       "chute pression",  "pompe purge", "purge max"};
    for (unsigned i = 0; i < 6; ++i)
      tile(i, n[i], x[i]);
  } else {
    const char *n[] = {"réinitialiser réseau", "calibrations", "réinitialiser LCD",
                       "veille", "", ""};
    const char *val[] = {"effacer", "depuis /config", "redémarrer",
                         "automatique", "", ""};
    for (unsigned i = 0; i < 6; ++i)
      tile(i, n[i], val[i],
           i == 0 ? Role::Destructive : Role::Secondary);
    for (unsigned i : {4u, 5u}) {
      hidden(v.tile[i], true);
      hidden(v.tile_name[i], true);
    }
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
  return s.cycle_state == core::CycleState::kFilling ||
         s.cycle_state == core::CycleState::kPreinfusion ||
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
    hidden(v.hero_time, true);
    hidden(v.hero_divider, true);
    lv_obj_set_width(v.hero, 800);
    lv_obj_set_pos(v.hero, 0, 74);
    lv_obj_set_style_text_align(v.hero, LV_TEXT_ALIGN_CENTER, 0);
    text(v.phase, s.capture_cooldown ? "écoulement" : "terminé");
    if (s.capture_cooldown && s.cycle_weight_goal) {
      fmt(t, sizeof(t), s.weight_g - s.cycle_start_weight_g, " g");
      text(v.hero, t);
    } else if (s.last_shot_available) {
      fmt(t, sizeof(t), s.last_shot_weight_g, " g");
      text(v.hero, t);
    } else {
      std::snprintf(t, sizeof(t), "%lu s", ulong(s.cycle_elapsed_ms / 1000));
      text(v.hero, t);
    }
    lv_obj_set_width(v.progress, 420);
    color(v.hero, theme::kRampFull);
    text(lv_obj_get_child(v.stop, 0), "fermer");
    disable(v.stop, s.capture_cooldown);
    return;
  }
  text(v.phase, s.cycle_state == core::CycleState::kPurge ? "purge"
                : s.cycle_state == core::CycleState::kFilling
                    ? "remplissage"
                : s.cycle_state == core::CycleState::kPreinfusion
                    ? "pré-infusion"
                : s.cycle_state == core::CycleState::kRampdown ? "rampe"
                                                               : "infusion");
  color(v.phase, s.cycle_state == core::CycleState::kPreinfusion
                     ? theme::kRampLow
                     : theme::kAccent);
  const bool show_weight_and_time =
      s.cycle_weight_goal && s.cycle_state != core::CycleState::kPurge;
  if (show_weight_and_time) {
    // Deux zones fixes de part et d'autre de la barre centrale. Aucun objet
    // n'est repositionné lors des mises à jour de télémétrie.
    lv_obj_set_width(v.hero, 399);
    lv_obj_set_pos(v.hero, 0, 74);
    lv_obj_set_style_text_align(v.hero, LV_TEXT_ALIGN_CENTER, 0);
    fmt(t, sizeof(t), s.weight_g - s.cycle_start_weight_g, " g");
    text(v.hero, t);
    std::snprintf(t, sizeof(t), "%lu s", ulong(s.cycle_elapsed_ms / 1000));
    text(v.hero_time, t);
    hidden(v.hero_time, false);
    hidden(v.hero_divider, false);
  } else {
    lv_obj_set_width(v.hero, 800);
    lv_obj_set_pos(v.hero, 0, 74);
    lv_obj_set_style_text_align(v.hero, LV_TEXT_ALIGN_CENTER, 0);
    std::snprintf(t, sizeof(t), "%lu s", ulong(s.cycle_elapsed_ms / 1000));
    text(v.hero, t);
    hidden(v.hero_time, true);
    hidden(v.hero_divider, true);
  }
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
  color(v.hero, s.cycle_state == core::CycleState::kPreinfusion
                    ? theme::kRampLow
                    : theme::kAccent);
  color(v.hero_time, s.cycle_state == core::CycleState::kPreinfusion
                         ? theme::kRampLow
                         : theme::kAccent);
  text(lv_obj_get_child(v.stop, 0), "arrêter");
  disable(v.stop, false);
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
lv_obj_t *diagnostic_icon(lv_obj_t *parent) {
  lv_obj_t *icon = lv_obj_create(parent);
  lv_obj_remove_style_all(icon);
  lv_obj_set_size(icon, 24, 24);

  lv_obj_t *body = lv_obj_create(icon);
  lv_obj_remove_style_all(body);
  lv_obj_set_size(body, 20, 20);
  lv_obj_set_pos(body, 2, 2);
  lv_obj_set_style_border_width(body, 2, 0);
  lv_obj_set_style_border_color(body, theme::kText, 0);
  lv_obj_set_style_radius(body, 4, 0);

  for (int y : {6, 11, 16}) {
    lv_obj_t *line = lv_obj_create(body);
    lv_obj_remove_style_all(line);
    lv_obj_set_size(line, y == 11 ? 10 : 7, 2);
    lv_obj_set_pos(line, 5, y);
    lv_obj_set_style_bg_color(line, theme::kText, 0);
    lv_obj_set_style_bg_opa(line, LV_OPA_COVER, 0);
    lv_obj_set_style_radius(line, 1, 0);
  }
  return icon;
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
  // Le bandeau n'emploie plus de coordonnées pour ses valeurs : les deux
  // extrémités ont une largeur réservée et les télémétries occupent le solde.
  // Les séparateurs font partie de ces calculs, afin qu'aucune valeur ne les
  // recouvre lorsque son texte s'allonge.
  lv_obj_t *topbar = lv_obj_create(p);
  lv_obj_remove_style_all(topbar);
  lv_obj_set_pos(topbar, theme::kMargin, 24);
  lv_obj_set_size(topbar, theme::kScreenWidth - 2 * theme::kMargin + 27,
                  kTopbarHeight);
  lv_obj_remove_flag(topbar, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_set_flex_flow(topbar, LV_FLEX_FLOW_ROW);
  lv_obj_set_flex_align(topbar, LV_FLEX_ALIGN_START, LV_FLEX_ALIGN_CENTER,
                        LV_FLEX_ALIGN_CENTER);

  // 24 icône + espacement + 82 horloge (80 px d'avance mesurée, +2 de
  // marge) + espacement + séparateur + espacement + 104 version +
  // espacement + séparateur = 260 px.
  lv_obj_t *left = topbar_group(topbar, 260);
  v.diagnostic = diagnostic_icon(left);
  spacer(left, kTopbarGap);
  dyn(left, &v.clock, "", theme::kFontStatus, theme::kTextDim, 0, 0);
  lv_obj_set_width(v.clock, 82);
  lv_label_set_long_mode(v.clock, LV_LABEL_LONG_CLIP);
  spacer(left, kTopbarGap);
  topbar_rule(left);
  spacer(left, kTopbarGap);
  char profile[40];
  std::snprintf(profile, sizeof(profile), "v%u.%u.%02u",
                common::kFirmwareVersionMajor, common::kFirmwareVersionMinor,
                common::kFirmwareVersionPatch);
  lv_obj_t *ignore = nullptr;
  lab(left, &ignore, profile, theme::kFontStatus, theme::kText, 0, 0);
  lv_label_set_long_mode(ignore, LV_LABEL_LONG_CLIP);
  lv_obj_set_width(ignore, 104);
  spacer(left, kTopbarGap);
  topbar_rule(left);

  // La température reste à droite, la pression juste avant. La cellule du
  // poids absorbe tout l'espace restant et son contenu disparaît sans balance.
  lv_obj_t *sensors = topbar_group(topbar, 1, true);
  lv_obj_set_flex_align(sensors, LV_FLEX_ALIGN_END, LV_FLEX_ALIGN_CENTER,
                        LV_FLEX_ALIGN_CENTER);
  v.weight_group = topbar_group(sensors, 1, true);
  lv_obj_set_flex_align(v.weight_group, LV_FLEX_ALIGN_END, LV_FLEX_ALIGN_CENTER,
                        LV_FLEX_ALIGN_CENTER);
  v.weight_content = topbar_group(v.weight_group, LV_SIZE_CONTENT);
  dyn(v.weight_content, &v.weight, "", theme::kFontStatus, theme::kTextDim, 0, 0);
  spacer(v.weight_content, kTopbarGap);
  topbar_rule(v.weight_content);
  spacer(v.weight_content, kTopbarGap);
  dyn(sensors, &v.pressure, "-", theme::kFontStatus, theme::kTextDim, 0, 0);
  lv_obj_set_width(v.pressure, 110);
  lv_label_set_long_mode(v.pressure, LV_LABEL_LONG_CLIP);
  lv_obj_set_style_text_align(v.pressure, LV_TEXT_ALIGN_RIGHT, 0);
  spacer(sensors, kTopbarGap);
  topbar_rule(sensors);
  spacer(sensors, kTopbarGap);
  dyn(sensors, &v.temperature, "-", theme::kFontStatus, theme::kTextDim, 0, 0);
  lv_obj_set_width(v.temperature, 110);
  lv_label_set_long_mode(v.temperature, LV_LABEL_LONG_CLIP);
  lv_obj_set_style_text_align(v.temperature, LV_TEXT_ALIGN_RIGHT, 0);
  // L'icône de diagnostic et l'heure ouvrent les diagnostics.
  for (lv_obj_t *status : {v.diagnostic, v.clock}) {
    lv_obj_add_flag(status, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_add_event_cb(status, note, LV_EVENT_PRESSED, nullptr);
    lv_obj_add_event_cb(status, show_diagnostics, LV_EVENT_CLICKED, nullptr);
  }
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
  dyn(v.cycle, &v.phase, "", theme::kFontStatus, theme::kAccent, 0, 24);
  lv_obj_set_width(v.phase, 800);
  lv_obj_set_style_text_align(v.phase, LV_TEXT_ALIGN_CENTER, 0);
  dyn(v.cycle, &v.hero, "", theme::kFontHeroBrew, theme::kAccent, 0, 74);
  // Deux cellules héro fixes de 399 px, séparées par une barre verticale de
  // 2 px exactement au centre. Hors du mode poids, la première reprend toute
  // la largeur et la barre est masquée.
  lv_obj_set_width(v.hero, 800);
  lv_obj_set_style_text_align(v.hero, LV_TEXT_ALIGN_CENTER, 0);
  dyn(v.cycle, &v.hero_time, "", theme::kFontHeroBrew, theme::kAccent, 401,
      74);
  lv_obj_set_width(v.hero_time, 399);
  lv_obj_set_style_text_align(v.hero_time, LV_TEXT_ALIGN_CENTER, 0);
  hidden(v.hero_time, true);
  v.hero_divider = box(v.cycle, 399, 74, 2, 104, theme::kTextFaint, 0);
  hidden(v.hero_divider, true);
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
      navbar(v.settings, "réglages", &ignore, back_settings, false, true);
  dyn(settings_bar, &v.index, "1/4", theme::kFontButton, theme::kText, 0, 0);
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
    lv_obj_align(value, LV_ALIGN_TOP_LEFT, 16, 36);
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
  const char *names[] = {"pression", "chaudière", "débit",
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
    if (i == 3) {
      lv_obj_t *pump_tile = lv_obj_create(v.diag);
      lv_obj_remove_style_all(pump_tile);
      lv_obj_set_size(pump_tile, 240, 96);
      lv_obj_set_pos(pump_tile, x, y);
      lv_obj_add_flag(pump_tile, LV_OBJ_FLAG_CLICKABLE);
      lv_obj_add_event_cb(pump_tile, show_dimmer_menu, LV_EVENT_CLICKED, nullptr);
    }
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
  base(v.confirm);
  lv_obj_set_style_bg_color(v.confirm, theme::kBg, 0);
  lv_obj_set_style_bg_opa(v.confirm, LV_OPA_70, 0);
  lv_obj_set_style_border_width(v.confirm, 0, 0);
  lv_obj_set_style_outline_width(v.confirm, 0, 0);
  lv_obj_set_style_shadow_width(v.confirm, 0, 0);
  lv_obj_set_style_width(v.confirm, 0, LV_PART_SCROLLBAR);
  lv_obj_remove_flag(v.confirm, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_add_flag(v.confirm, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_t *confirm_card = box(v.confirm, 136, 116, 528, 248,
                               theme::kBgRaised, theme::kRadius);
  box(confirm_card, 0, 0, 4, 248, theme::kAccent, 0);
  dyn(confirm_card, &v.confirm_title, "", theme::kFontSecondary, theme::kAccent,
      28, 24);
  dyn(confirm_card, &v.confirm_body, "", theme::kFontLabel, theme::kTextDim, 28,
      92);
  lv_obj_t *c = button(confirm_card, 28, 138, 216, 80, "annuler");
  lv_obj_t *o =
      button(confirm_card, 276, 138, 216, 80, "valider", Role::Primary);
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
  v.dimmer_menu = lv_obj_create(p);
  base(v.dimmer_menu);
  dyn(v.dimmer_menu, &v.dimmer_menu_title, "dimmerlink", theme::kFontSecondary,
      theme::kAccent, 32, 64);
  dyn(v.dimmer_menu, &v.dimmer_menu_body, "", theme::kFontLabel, theme::kTextDim,
      32, 126);
  v.dimmer_reset = button(v.dimmer_menu, 32, 208, 352, 88, "reset dimmerlink",
                          Role::Destructive);
  v.dimmer_recalibrate = button(v.dimmer_menu, 416, 208, 352, 88, "calibrer");
  v.dimmer_close = button(v.dimmer_menu, 224, 328, 352, 88, "fermer");
  lv_obj_add_event_cb(v.dimmer_reset, run_dimmer_action, LV_EVENT_CLICKED,
                      reinterpret_cast<void *>(static_cast<uintptr_t>(core::Action::kResetDimmer)));
  lv_obj_add_event_cb(v.dimmer_recalibrate, run_dimmer_action, LV_EVENT_CLICKED,
                      reinterpret_cast<void *>(static_cast<uintptr_t>(core::Action::kRecalibrateDimmer)));
  lv_obj_add_event_cb(v.dimmer_close, close_dimmer_menu, LV_EVENT_CLICKED, nullptr);
  hidden(v.dimmer_menu, true);
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
  bool boiler = s.boiler_temperature_valid && s.boiler_temperature_freshness == core::Freshness::kFresh;
  bool brew_temperature_ready = c.heating_enabled && boiler && s.brew_temperature_ready &&
      s.heating_power_capable && s.heating_freshness == core::Freshness::kFresh &&
      std::fabs(s.boiler_temperature_c - c.brew_temperature_c) <= core::kBrewTemperatureToleranceC;
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
  if (boiler)
    fmt(t, sizeof(t), s.boiler_temperature_c, "°");
  else
    std::snprintf(t, sizeof(t), "-");
  text(v.temperature, t);
  const lv_color_t temperature_color =
      !c.heating_enabled ? theme::kTextFaint
      : !boiler ? theme::kTextDim
      : s.boiler_temperature_c > c.brew_temperature_c + core::kBrewTemperatureToleranceC ? theme::kFault
      : brew_temperature_ready ? theme::kSuccess
      : s.boiler_temperature_c >= c.brew_temperature_c - 5.0f ? theme::kThermalNear
      : theme::kThermal;
  color(v.temperature, temperature_color);
  lv_obj_set_style_text_opa(v.temperature,
                            LV_OPA_COVER,
                            0);
  if (scale) {
    fmt(t, sizeof(t), s.weight_g, " g");
    text(v.weight, t);
  } else
    text(v.weight, "");
  hidden(v.weight_content, !scale);
  color(v.weight, scale ? theme::kText : theme::kTextFaint);
  // L'heure n'est affichée qu'après la synchronisation NTP. Le poids visible
  // suffit à signaler la présence d'une balance qui fournit des mesures.
  hidden(v.clock, !s.time_known);
  if (s.time_known) {
#if defined(UI_SIM)
    if (g_sim_clock_override[0] != '\0') {
      text(v.clock, g_sim_clock_override);
    } else {
#endif
    std::time_t now = std::time(nullptr);
    std::tm local{};
    localtime_r(&now, &local);
    std::strftime(t, sizeof(t), "%H:%M", &local);
    text(v.clock, t);
#if defined(UI_SIM)
    }
#endif
  }
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
  text(v.warning, s.lockout ? "verrou de sécurité · couper puis rallumer la machine" :
                  !s.sensors_alive ? "module interne injoignable · wifi disponible pour récupération" :
                  (!s.dimmer_ready || !s.dimmer_valid) && s.sensors_alive
                      ? "dimmer en calibration"
                      : !c.heating_enabled ? "chauffe désactivée · purge disponible"
                      : !s.heating_power_capable ? "chauffage indisponible"
                      : !brew_temperature_ready ? "chaudière en chauffe"
                      : "");
  recover_stuck_dimmer_calibration(s);
  disable(v.brew_button, s.lockout || !s.sensors_alive || !s.dimmer_ready || !s.dimmer_valid ||
             !brew_temperature_ready);
  disable(v.minus, scale ? c.target_weight_g <= 10 : c.target_time_s <= 5);
  disable(v.plus, scale ? c.target_weight_g >= 100 : c.target_time_s >= 60);
  cycle(s, c);
  if (!lv_obj_has_flag(v.diag, LV_OBJ_FLAG_HIDDEN)) {
    bool flow = s.flow_valid && present(s.flow_freshness),
         states[] = {press,
                     boiler,
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
    } else {
      text(v.diag_val[0], "-");
    }
    if (boiler) {
      fmt(t, sizeof(t), s.boiler_temperature_c, "°");
      text(v.diag_val[1], t);
    } else {
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
  if (s.boot_time_syncing)
    fullscreen(true, "synchronisation heure", "connexion wifi...");
  else if (s.flash_active)
    fullscreen(true, "mise à jour", "ne pas couper la machine");
  else if (s.radio_mode == core::RadioMode::kWifi) {
    const core::HFCaptureInfo capture = core::get_hf_capture_info();
    char recording[64];
    if (capture.status == core::HFCaptureStatus::kComplete) {
      const auto duration_s = static_cast<unsigned>(
          (capture.ended_at_us - capture.started_at_us + 500000) / 1000000);
      std::snprintf(recording, sizeof(recording), "un enregistrement de %u s disponible", duration_s);
    } else if (capture.status == core::HFCaptureStatus::kActive) {
      std::snprintf(recording, sizeof(recording), "enregistrement en cours");
    } else {
      std::snprintf(recording, sizeof(recording), "aucun enregistrement disponible");
    }
    if (s.radio_transition)
      std::snprintf(t, sizeof(t), "activation du réseau\n%s", recording);
    else if (s.ipv4_address)
      std::snprintf(t, sizeof(t), "adresse ip · %u.%u.%u.%u\n%s",
                    static_cast<unsigned>(s.ipv4_address & 255),
                    static_cast<unsigned>((s.ipv4_address >> 8) & 255),
                    static_cast<unsigned>((s.ipv4_address >> 16) & 255),
                    static_cast<unsigned>((s.ipv4_address >> 24) & 255), recording);
    else
      std::snprintf(t, sizeof(t), "configuration wifi ou association en cours\n%s", recording);
    fullscreen(true, "Mode wifi", t);
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
      std::strcmp(scenario, "settings2") == 0 ||
      std::strcmp(scenario, "settings3") == 0) {
    page = static_cast<uint8_t>(std::strcmp(scenario, "settings3") == 0   ? 3
                                : std::strcmp(scenario, "settings2") == 0 ? 2
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
    page = 3;
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

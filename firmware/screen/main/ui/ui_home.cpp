#include "ui_home.h"
#include "common/version.hpp"
#include "core/core.h"
#include "ota_local.h"
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
constexpr int kHeatBarWidth = 100;
// ui_font_28 a une hauteur de ligne légèrement supérieure à 32 px : ces deux
// pixels empêchent le parent Flex de rogner les descendantes (notamment le g).
constexpr int kTopbarHeight = 52;
enum class Role : uint8_t { Secondary, Primary, Destructive, Disabled };
enum class DiagnosticState : uint8_t { Disabled, Valid, Warn, Error };
struct DiagnosticTile {
  lv_obj_t *dot{};
  lv_obj_t *title{};
  lv_obj_t *value{};
  lv_obj_t *detail{};
};
enum class Edit : uint8_t {
  None,
  Weight,
  Time,
  BrewPressure,
  BrewTemperature,
  BrewPreheatTime,
  FillingTime,
  FillingPressure,
  FillingPump,
  PreTime,
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
  lv_obj_t *pressure{}, *temperature{}, *heat_track{}, *heat_fill{},
      *weight{}, *weight_group{}, *weight_content{}, *diagnostic{},
      *clock{}, *target{},
      *detail{}, *warning{}, *minus{}, *plus{}, *tap{}, *brew_button{}, *brew{},
      *purge{}, *settings_button{}, *cycle{}, *phase{}, *hero{},
      *hero_time{}, *hero_divider{},
      *cycle_detail{}, *progress{}, *stop{}, *settings{}, *tab[4]{},
      *tile[6]{}, *tile_name[6]{}, *tile_value[6]{}, *diag{},
      *diag_version_value{}, *diag_version_detail{}, *keypad{}, *key_title{},
      *key_value{}, *key_unit{}, *key_error{}, *key_ok{}, *key_comma{},
      *choice{}, *choice_title{}, *choice_button[4]{}, *confirm{},
      *confirm_title{}, *confirm_body{}, *full{}, *full_title{}, *full_body{},
      *wifi_exit{}, *dimmer_menu{}, *dimmer_menu_title{}, *dimmer_menu_body{},
      *dimmer_reset{}, *dimmer_recalibrate{}, *dimmer_close{}, *dim{},
      *heating_menu{}, *heating_menu_title{}, *heating_menu_body{},
      *heating_enable{}, *heating_disable{}, *heating_close{},
      *valve_menu{}, *valve_menu_title{}, *valve_menu_body{},
      *valve_menu_hint{}, *valve_open{}, *valve_close{}, *valve_back{},
      *standby{}, *standby_title{}, *standby_body{};
  DiagnosticTile diag_tile[9]{};
} v;
// Écran d'infusion : frise des phases, six tuiles de résumé, puis la tuile de
// fonctionnement et « arrêter » pendant l'écoulement, « fermer » ensuite.
// Voir docs/ecran-infusion.md.
constexpr size_t kSegments = core::kShotSegmentCount;
struct BrewTile {
  lv_obj_t *title{}, *aside{}, *value{}, *detail{};
};
struct BrewView {
  lv_obj_t *root{}, *phase{}, *started{}, *reserve{}, *live{}, *stop{}, *close{};
  lv_obj_t *segment[kSegments]{}, *segment_label[kSegments]{}, *number[kSegments]{};
  BrewTile tile[6]{};
  lv_obj_t *live_value[4]{};
  // Géométrie déjà appliquée : LVGL invalide à chaque appel, même identique.
  int32_t segment_x[kSegments]{}, segment_w[kSegments]{}, number_x[kSegments]{};
  int32_t reserve_x = -1;
  int8_t outlined = -1;
} bv;
// L'écran d'infusion ajoute une trentaine de libellés dynamiques.
constexpr size_t N = 160, L = 128;
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
lv_color_t diagnostic_color(DiagnosticState state) {
  switch (state) {
    case DiagnosticState::Valid: return theme::kSuccess;
    case DiagnosticState::Warn: return theme::kAccent;
    case DiagnosticState::Error: return theme::kFault;
    case DiagnosticState::Disabled: return theme::kTextFaint;
  }
  return theme::kTextFaint;
}
DiagnosticTile diagnostic_tile(lv_obj_t *parent, int x, int y,
                               const char *title) {
  DiagnosticTile tile;
  tile.dot = box(parent, x, y + 4, 10, 10, theme::kTextFaint, 5);
  lab(parent, &tile.title, title, theme::kFontLabel, theme::kTextDim, x + 18,
      y);
  dyn(parent, &tile.value, "-", theme::kFontButton, theme::kText, x, y + 28);
  dyn(parent, &tile.detail, "", theme::kFontLabel, theme::kTextFaint, x,
      y + 62);
  return tile;
}
void diagnostic_tile_set(DiagnosticTile &tile, DiagnosticState state,
                         const char *value, const char *detail) {
  lv_obj_set_style_bg_color(tile.dot, diagnostic_color(state), 0);
  text(tile.value, value);
  text(tile.detail, detail);
}
DiagnosticState diagnostic_measure_state(bool valid, core::Freshness freshness) {
  if (!valid || freshness == core::Freshness::kMissing)
    return DiagnosticState::Error;
  return freshness == core::Freshness::kStale ? DiagnosticState::Warn
                                               : DiagnosticState::Valid;
}
const char *diagnostic_measure_detail(DiagnosticState state) {
  return state == DiagnosticState::Valid ? "valide"
       : state == DiagnosticState::Warn ? "périmé"
                                         : "absent";
}
void format_error_count(char *out, size_t size, uint64_t value) {
  if (value < 1000) {
    std::snprintf(out, size, "%llu erreur%s",
                  static_cast<unsigned long long>(value), value == 1 ? "" : "s");
  } else if (value < 10000) {
    const uint64_t tenths = (value + 50) / 100;
    std::snprintf(out, size, "%llu,%llu K erreurs",
                  static_cast<unsigned long long>(tenths / 10),
                  static_cast<unsigned long long>(tenths % 10));
  } else if (value < 1000000) {
    std::snprintf(out, size, "%llu K erreurs",
                  static_cast<unsigned long long>((value + 500) / 1000));
  } else if (value < 10000000) {
    const uint64_t tenths = (value + 50000) / 100000;
    std::snprintf(out, size, "%llu,%llu M erreurs",
                  static_cast<unsigned long long>(tenths / 10),
                  static_cast<unsigned long long>(tenths % 10));
  } else {
    std::snprintf(out, size, "%llu M erreurs",
                  static_cast<unsigned long long>((value + 500000) / 1000000));
  }
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
  // Format the magnitude separately so the sign is always the ASCII '-'.
  // This keeps LVGL text independent of locale or Unicode minus characters.
  const bool negative = std::signbit(f);
  const int written = std::snprintf(out, n, "%s%.1f%s", negative ? "-" : "",
                                    static_cast<double>(std::fabs(f)), s);
  if (written < 0 || static_cast<size_t>(written) >= n)
    return;
  for (char *p = out; *p; ++p)
    if (*p == '.')
      *p = ',';
}
bool present(core::Freshness f) { return f != core::Freshness::kMissing; }
// Dernier refus d'ouverture, affiché à la place de l'état jusqu'au prochain appui.
const char *valve_menu_error = nullptr;
// Quitter l'écran de maintenance referme toujours la vanne : elle ne reste
// ouverte que sous les yeux de l'utilisateur.
void hide_valve_menu() {
  if (lv_obj_has_flag(v.valve_menu, LV_OBJ_FLAG_HIDDEN))
    return;
  hidden(v.valve_menu, true);
  if (core::get_snapshot().maintenance_valve_open)
    static_cast<void>(core::perform_action({core::Action::kCloseMaintenanceValve}));
}
void close_all() {
  hide_valve_menu();
  hidden(v.settings, true);
  hidden(v.diag, true);
  hidden(v.keypad, true);
  hidden(v.choice, true);
  hidden(v.dimmer_menu, true);
  hidden(v.heating_menu, true);
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
void close_heating_menu(lv_event_t *) { hidden(v.heating_menu, true); }
void run_heating_config(lv_event_t *event) {
  const bool enabled = reinterpret_cast<uintptr_t>(lv_event_get_user_data(event)) != 0;
  auto config = core::get_config();
  config.heating_enabled = enabled;
  const core::ConfigResult result = core::put_config(config);
  if (result.status == core::ConfigStatus::kOk) {
    hidden(v.heating_menu, true);
    return;
  }
  text(v.heating_menu_body,
       result.status == core::ConfigStatus::kBusy
           ? "modification refusée pendant un cycle"
           : "configuration refusée");
}
void show_heating_menu(lv_event_t *) {
  const bool enabled = core::get_config().heating_enabled;
  text(v.heating_menu_body,
       enabled ? "chauffage actuellement activé"
               : "chauffage actuellement désactivé");
  disable(v.heating_enable, enabled);
  disable(v.heating_disable, !enabled);
  hidden(v.heating_menu, false);
}
void close_valve_menu(lv_event_t *) { hide_valve_menu(); }
void run_valve_action(lv_event_t *event) {
  auto action = static_cast<core::Action>(reinterpret_cast<uintptr_t>(lv_event_get_user_data(event)));
  const core::ActionResult result = core::perform_action({action});
  valve_menu_error =
      result.status == core::ActionStatus::kOk ? nullptr
      : result.status == core::ActionStatus::kBusLost ? "module capteurs injoignable"
      : result.status == core::ActionStatus::kLocked ? "verrou de sécurité actif"
      : result.status == core::ActionStatus::kCycleActive ? "ouverture refusée pendant un cycle"
                                                          : "ouverture indisponible";
}
void show_valve_menu(lv_event_t *) {
  valve_menu_error = nullptr;
  hidden(v.valve_menu, false);
}
// Rafraîchi à chaque tick tant que le menu est visible : compte à rebours et
// conditions d'ouverture suivent l'état réel des capteurs.
void render_valve_menu(const core::Snapshot &s, const core::Config &c) {
  if (lv_obj_has_flag(v.valve_menu, LV_OBJ_FLAG_HIDDEN))
    return;
  char t[64];
  const bool open = s.maintenance_valve_open;
  if (valve_menu_error)
    std::snprintf(t, sizeof(t), "%s", valve_menu_error);
  else if (!s.sensors_alive)
    std::snprintf(t, sizeof(t), "module capteurs injoignable");
  else if (s.lockout)
    std::snprintf(t, sizeof(t), "verrou de sécurité actif");
  else if (!s.maintenance_valve_capable)
    std::snprintf(t, sizeof(t), "module capteurs à mettre à jour");
  else if (open)
    std::snprintf(t, sizeof(t), "vanne ouverte · fermeture dans %u s",
                  static_cast<unsigned>((s.lease_remaining_ms + 999) / 1000));
  else if (c.heating_enabled)
    std::snprintf(t, sizeof(t), "désactiver le chauffage avant d'ouvrir");
  else
    std::snprintf(t, sizeof(t), "vanne fermée · pompe arrêtée");
  text(v.valve_menu_body, t);
  text(lv_obj_get_child(v.valve_open, 0), open ? "relancer 30 s" : "ouvrir 30 s");
  disable(v.valve_open, !s.sensors_alive || s.lockout ||
                            !s.maintenance_valve_capable || c.heating_enabled);
  disable(v.valve_close, !open);
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
  lv_obj_set_style_pad_all(bar, 0, 0);
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
  case Edit::BrewPreheatTime:
  case Edit::FillingPressure:
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
  case Edit::BrewPreheatTime:
    t = "précharge chauffe";
    u = "s";
    break;
  case Edit::FillingTime:
    t = "durée remplissage";
    u = "s";
    break;
  case Edit::FillingPressure:
    t = "pression remplissage";
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
  case Edit::BrewPreheatTime:
    n = c.brew_preheat_time_s;
    decimals = 1;
    break;
  case Edit::FillingTime:
    n = c.filling_time_s;
    break;
  case Edit::FillingPressure:
    n = c.filling_pressure_bar;
    decimals = 1;
    break;
  case Edit::FillingPump:
    n = c.filling_pump_pct;
    break;
  case Edit::PreTime:
    n = c.preinfusion_time_s;
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
    return *n >= core::kMinimumBrewTemperatureC && *n <= core::kMaximumBrewTemperatureC &&
           std::fabs(*n * 2 - std::round(*n * 2)) < .01f;
  case Edit::BrewPreheatTime:
    return *n >= 0 && *n <= core::kMaximumBrewPreheatTimeS &&
           std::fabs(*n * 2 - std::round(*n * 2)) < .01f;
  case Edit::FillingTime:
    return *n >= 1 && *n <= 20 && std::floor(*n) == *n;
  case Edit::FillingPressure:
    return *n >= .3f && *n <= 2.0f &&
           std::fabs(*n * 10 - std::round(*n * 10)) < .01f;
  case Edit::FillingPump:
    return *n >= 20 && *n <= 100 && std::floor(*n) == *n &&
           static_cast<unsigned>(*n) % 5 == 0;
  case Edit::PreTime:
    return *n >= 0 && *n <= 20 && std::floor(*n) == *n;
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
  case Edit::BrewPreheatTime:
    c.brew_preheat_time_s = n;
    break;
  case Edit::FillingTime:
    c.filling_time_s = n;
    break;
  case Edit::FillingPressure:
    c.filling_pressure_bar = n;
    break;
  case Edit::FillingPump:
    c.filling_pump_pct = n;
    break;
  case Edit::PreTime:
    c.preinfusion_time_s = n;
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
void select_settings_page(lv_event_t *e) {
  page = static_cast<uint8_t>(reinterpret_cast<uintptr_t>(lv_event_get_user_data(e)));
  render_settings();
}
void show_settings(lv_event_t *) {
  page = 0;
  render_settings();
  close_all();
  hidden(v.settings, false);
}
// Boutons du choix de pré-infusion : temps (bit 0), poids (bit 2). Le bit 1,
// l'ancienne sortie par pression, n'est plus proposé.
uint8_t preinfusion_choice_bit(unsigned i) {
  return static_cast<uint8_t>(i == 0 ? core::PreinfusionMode::kTime : core::PreinfusionMode::kWeight);
}
void show_choice(Choice q);
void choose(lv_event_t *e) {
  unsigned i = reinterpret_cast<uintptr_t>(lv_event_get_user_data(e));
  auto c = core::get_config();
  if (choosing == Choice::Preinfusion) {
    const uint8_t bit = preinfusion_choice_bit(i);
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
  const char *n[] = {"temps", "poids", "", "aucune", "temps",
                     "poids", "chute pression"};
  unsigned count = q == Choice::Preinfusion ? 2 : 4;
  for (unsigned i = 0; i < 4; ++i) {
    hidden(v.choice_button[i], i >= count);
    if (i < count) {
      text(lv_obj_get_child(v.choice_button[i], 0),
           q == Choice::Preinfusion ? n[i] : n[i + 3]);
      auto c = core::get_config();
      bool sel = q == Choice::Preinfusion
                     ? (static_cast<uint8_t>(c.preinfusion_mode) & preinfusion_choice_bit(i)) != 0
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
      Edit a[] = {Edit::FillingPressure, Edit::None, Edit::FillingTime,
                  Edit::PreTime, Edit::BrewTemperature, Edit::None};
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
    else if (i == 4)
      show_edit(Edit::BrewPreheatTime);
  }
}
const char *preinfusion_mode_text(core::PreinfusionMode mode, char *buffer, size_t size) {
  if (mode == core::PreinfusionMode::kNone) {
    std::snprintf(buffer, size, "aucune");
    return buffer;
  }
  bool first = true;
  buffer[0] = '\0';
  const char *names[] = {"temps", "poids"};
  for (unsigned i = 0; i < 2; ++i) {
    if ((static_cast<uint8_t>(mode) & preinfusion_choice_bit(i)) == 0) continue;
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
  char x[6][40]{};
  for (unsigned i = 0; i < 4; ++i) {
    lv_obj_t *label = lv_obj_get_child(v.tab[i], 0);
    const bool selected = i == page;
    lv_obj_set_style_bg_color(v.tab[i], selected ? theme::kSurfaceAccent : theme::kSurface, 0);
    lv_obj_set_style_text_color(label, selected ? theme::kAccent : theme::kText, 0);
    lv_obj_remove_state(v.tab[i], LV_STATE_PRESSED);
  }
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
    fmt(x[0], sizeof(x[0]), c.filling_pressure_bar, " bar");
    preinfusion_mode_text(c.preinfusion_mode, x[1], sizeof(x[1]));
    std::snprintf(x[2], 40, "%u s", c.filling_time_s);
    std::snprintf(x[3], 40, "%u s", c.preinfusion_time_s);
    fmt(x[4], sizeof(x[4]), c.brew_temperature_c, " °C");
    const char *n[] = {"pression remplissage", "critères pré-inf.", "durée remplissage",
                       "échéance pré-inf.", "cible chaudière", ""};
    for (unsigned i = 0; i < 6; ++i) tile(i, n[i], x[i]);
    hidden(v.tile[5], true);
    hidden(v.tile_name[5], true);
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
    fmt(x[4], sizeof(x[4]), c.brew_preheat_time_s, " s");
    const char *n[] = {"réinitialiser réseau", "calibrations", "réinitialiser LCD",
                       "veille", "précharge chauffe", ""};
    const char *val[] = {"effacer", "depuis /config", "redémarrer",
                         "automatique", x[4], ""};
    for (unsigned i = 0; i < 6; ++i)
      tile(i, n[i], val[i],
           i == 0 ? Role::Destructive : Role::Secondary);
    for (unsigned i : {5u}) {
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
  return s.cycle_state == core::CycleState::kThermalPreheat ||
         s.cycle_state == core::CycleState::kFilling ||
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

// --- Écran d'infusion ----------------------------------------------------
constexpr int kFriseWidth = 736;
constexpr int kFriseHeight = 26;
constexpr int kTileWidth = 234, kTileHeight = 76, kTileGap = 16;
struct SegmentStyle {
  const char *names[3];
  lv_color_t fill;
  bool dark_text;
};
const SegmentStyle &segment_style(size_t i) {
  // Libellés du plus long au plus court : le premier qui tient est affiché.
  static const SegmentStyle styles[kSegments] = {
      {{"PRÉ-CHAUFFE", "CHAUFFE", "C"}, theme::kFault, false},
      {{"REMPLISSAGE", "REMPL.", "R"}, theme::kThermal, false},
      {{"PRÉ-INFUSION", "PRÉ-INF.", "PI"}, theme::kSurfaceHigh, false},
      {{"MONTÉE", "MONT.", "M"}, theme::kRampLow, false},
      {{"INFUSION", "INF.", "I"}, theme::kAccent, true},
      {{"GOUTTES", "GTTE", "G"}, theme::kTextFaint, false},
  };
  return styles[i];
}
const lv_color_t kSegmentText = lv_color_hex(0xEEEEEE);
int32_t text_width(const char *t, const lv_font_t *f) {
  lv_point_t size{};
  lv_text_get_size(&size, t, f, 0, 0, LV_COORD_MAX, LV_TEXT_FLAG_NONE);
  return size.x;
}
// Comme fmt(), avec un nombre de décimales choisi et « - » pour l'inconnu :
// les polices Inter embarquées s'arrêtent à Latin-1, sans tiret long.
void fmt_digits(char *out, size_t n, float f, int digits, const char *suffix) {
  if (!std::isfinite(f)) {
    std::snprintf(out, n, "-");
    return;
  }
  const bool negative = f < 0.0f && std::fabs(f) >= 0.5f * std::pow(10.0f, -digits);
  std::snprintf(out, n, "%s%.*f%s", negative ? "-" : "", digits,
                static_cast<double>(std::fabs(f)), suffix);
  for (char *p = out; *p; ++p)
    if (*p == '.')
      *p = ',';
}
void set_geometry(lv_obj_t *o, int32_t &cached_x, int32_t &cached_w, int32_t x,
                  int32_t w) {
  if (cached_x == x && cached_w == w)
    return;
  cached_x = x;
  cached_w = w;
  lv_obj_set_pos(o, x, 42);
  lv_obj_set_width(o, w);
}
BrewTile brew_tile(lv_obj_t *p, int col, int row, const char *title,
                   bool main_value) {
  const int x = theme::kMargin + col * (kTileWidth + kTileGap);
  const int y = 112 + row * (kTileHeight + 12);
  lv_obj_t *t = box(p, x, y, kTileWidth, kTileHeight, theme::kBgRaised,
                    theme::kRadius);
  lv_obj_set_style_pad_all(t, 0, 0);
  BrewTile tile;
  dyn(t, &tile.title, title, theme::kFontLabel, theme::kTextFaint, 16, 8);
  // Valeur secondaire alignée à droite du titre (temps total, débit), ou
  // posée après la valeur (températures) : voir render_brew().
  dyn(t, &tile.aside, "", theme::kFontLabel, theme::kTextDim, 16, 8);
  lv_obj_set_width(tile.aside, kTileWidth - 32);
  lv_obj_set_style_text_align(tile.aside, LV_TEXT_ALIGN_RIGHT, 0);
  dyn(t, &tile.value, "-", theme::kFontUnit,
      main_value ? theme::kRampFull : theme::kText, 16, 30);
  dyn(t, &tile.detail, "", theme::kFontLabel, theme::kTextDim, 16, 44);
  return tile;
}
void build_brew(lv_obj_t *p) {
  bv.root = lv_obj_create(p);
  lv_obj_remove_style_all(bv.root);
  lv_obj_set_size(bv.root, 800, 400);
  lv_obj_set_pos(bv.root, 0, 80);
  lv_obj_remove_flag(bv.root, LV_OBJ_FLAG_SCROLLABLE);
  dyn(bv.root, &bv.phase, "", theme::kFontLabel, theme::kAccent,
      theme::kMargin, 4);
  dyn(bv.root, &bv.started, "", theme::kFontLabel, theme::kTextFaint, 468, 4);
  lv_obj_set_width(bv.started, 300);
  lv_obj_set_style_text_align(bv.started, LV_TEXT_ALIGN_RIGHT, 0);
  lv_obj_t *frise = lv_obj_create(bv.root);
  lv_obj_remove_style_all(frise);
  lv_obj_set_size(frise, kFriseWidth, 72);
  lv_obj_set_pos(frise, theme::kMargin, 0);
  lv_obj_remove_flag(frise, LV_OBJ_FLAG_SCROLLABLE);
  // Réserve des gouttes : contour seul, à la fin de l'échelle prévue.
  bv.reserve = box(frise, 0, 42, 1, kFriseHeight, theme::kBg, 4);
  lv_obj_set_style_bg_opa(bv.reserve, LV_OPA_TRANSP, 0);
  lv_obj_set_style_border_width(bv.reserve, 2, 0);
  lv_obj_set_style_border_color(bv.reserve, theme::kTextFaint, 0);
  lv_obj_set_style_border_opa(bv.reserve, LV_OPA_60, 0);
  hidden(bv.reserve, true);
  for (size_t i = 0; i < kSegments; ++i) {
    const SegmentStyle &style = segment_style(i);
    bv.segment[i] = box(frise, 0, 42, 2, kFriseHeight, style.fill, 4);
    lv_obj_set_style_outline_color(bv.segment[i], theme::kText, 0);
    lv_obj_set_style_outline_pad(bv.segment[i], 0, 0);
    dyn(bv.segment[i], &bv.segment_label[i], "", theme::kFontLabel,
        style.dark_text ? theme::kBg : kSegmentText, 0, 0);
    lv_obj_center(bv.segment_label[i]);
    dyn(frise, &bv.number[i], "", theme::kFontButton, theme::kText, 0, 0);
    hidden(bv.segment[i], true);
    hidden(bv.number[i], true);
    bv.segment_x[i] = bv.segment_w[i] = bv.number_x[i] = -1;
  }
  // Chiffres sous la frise : un objet frise plus haut porterait les deux ; ils
  // sont placés à y 74 par rapport à bv.root.
  lv_obj_set_height(frise, 112);
  for (size_t i = 0; i < kSegments; ++i)
    lv_obj_set_y(bv.number[i], 74);
  const char *titles[6] = {"poids",      "temps infusion",  "moyenne en tasse",
                           "temps total", "débit infusion", "baisse thermique"};
  for (int i = 0; i < 6; ++i)
    bv.tile[i] = brew_tile(bv.root, i % 3, i / 3, titles[i], i % 3 == 0);
  // Tuile de fonctionnement : quatre colonnes, séparées par des filets
  // indépendants (une bordure de colonne déborderait sur ses enfants).
  bv.live = box(bv.root, theme::kMargin, 296, 536, theme::kButtonHeight,
                theme::kBgRaised, theme::kRadius);
  lv_obj_set_style_pad_all(bv.live, 0, 0);
  const char *live_titles[4] = {"chauffe", "pompe", "débit balance",
                                "débit pompe"};
  for (int i = 0; i < 4; ++i) {
    lv_obj_t *title = nullptr;
    // 134 px par colonne : la police 26 garde « 1,25 ml/s » dans sa colonne.
    lab(bv.live, &title, live_titles[i], theme::kFontLabel, theme::kTextFaint,
        12 + i * 134, 14);
    dyn(bv.live, &bv.live_value[i], "-", theme::kFontButton, theme::kText,
        12 + i * 134, 42);
    if (i > 0)
      box(bv.live, i * 134, 16, 1, 56, theme::kHairline, 0);
  }
  bv.stop = button(bv.root, 584, 296, 184, theme::kButtonHeight, "arrêter",
                   Role::Primary);
  bind(bv.stop);
  lv_obj_add_event_cb(bv.stop, stop, LV_EVENT_CLICKED, nullptr);
  bv.close = button(bv.root, 224, 296, 352, theme::kButtonHeight, "fermer");
  // Grisé pendant les 20 s de capture qui suivent l'arrêt : fond de tuile et
  // texte pâle, sans le recolor gris à 50 % du thème par défaut.
  constexpr auto disabled = static_cast<lv_style_selector_t>(
      static_cast<uint32_t>(LV_PART_MAIN) | static_cast<uint32_t>(LV_STATE_DISABLED));
  lv_obj_set_style_bg_color(bv.close, theme::kBgRaised, disabled);
  lv_obj_set_style_recolor_opa(bv.close, LV_OPA_TRANSP, disabled);
  lv_obj_set_style_text_color(lv_obj_get_child(bv.close, 0), theme::kTextFaint, disabled);
  bind(bv.close);
  lv_obj_add_event_cb(bv.close, stop, LV_EVENT_CLICKED, nullptr);
  hidden(bv.close, true);
  hidden(bv.root, true);
}
const char *brew_phase_text(const core::Snapshot &s, bool done) {
  const core::ShotSummary &shot = s.shot;
  if (done)
    return s.capture_cooldown ? (shot.drip_done ? "écoulement" : "dernières gouttes")
                              : "terminé";
  switch (s.cycle_state) {
  case core::CycleState::kThermalPreheat: return "pré-chauffe";
  case core::CycleState::kFilling: return "remplissage";
  case core::CycleState::kPreinfusion: return "pré-infusion";
  case core::CycleState::kRampdown: return "rampe";
  default:
    return shot.has(core::ShotSegment::kInfusion) ? "infusion"
                                                  : "infusion · montée en pression";
  }
}
void render_frise(const core::ShotSummary &shot, bool live) {
  const float axis = std::max(shot.axis_s, 1.0f);
  const float k = kFriseWidth / axis;
  char t[24];
  int8_t current = -1;
  for (size_t i = 0; i < kSegments; ++i) {
    const auto segment = static_cast<core::ShotSegment>(i);
    if (!shot.has(segment)) {
      hidden(bv.segment[i], true);
      hidden(bv.number[i], true);
      continue;
    }
    const int32_t x = std::clamp<int32_t>(
        std::lround(shot.segment_start_s[i] * k), 0, kFriseWidth - 2);
    const int32_t end = std::clamp<int32_t>(
        std::lround(shot.segment_end_s[i] * k), 0, kFriseWidth);
    const int32_t w = std::max<int32_t>(2, std::min<int32_t>(end, kFriseWidth) - x - 2);
    set_geometry(bv.segment[i], bv.segment_x[i], bv.segment_w[i], x, w);
    hidden(bv.segment[i], false);
    const SegmentStyle &style = segment_style(i);
    const char *name = "";
    for (const char *candidate : style.names)
      if (text_width(candidate, theme::kFontLabel) + 8 <= w) {
        name = candidate;
        break;
      }
    text(bv.segment_label[i], name);
    if (segment == core::ShotSegment::kDrip) {
      char gain[16];
      fmt_digits(gain, sizeof(gain), shot.drip_gain_g, 1, " g");
      std::snprintf(t, sizeof(t), "+%s", gain);
    } else {
      fmt_digits(t, sizeof(t), shot.duration_s(segment), 1, "");
    }
    const int32_t number_w = text_width(t, theme::kFontButton);
    // Les gouttes s'alignent sur la fin de la frise ; les autres chiffres sur
    // le début de leur segment, s'ils tiennent dans sa largeur.
    const bool drip = segment == core::ShotSegment::kDrip;
    const bool show = drip ? shot.duration_s(segment) > 0.3f : number_w + 6 <= w;
    if (show) {
      text(bv.number[i], t);
      const int32_t nx = drip ? std::max<int32_t>(0, x + w - number_w) : x;
      if (bv.number_x[i] != nx) {
        bv.number_x[i] = nx;
        lv_obj_set_x(bv.number[i], nx);
      }
    }
    hidden(bv.number[i], !show);
    if (live && !shot.drip_done)
      current = static_cast<int8_t>(i);
  }
  if (current != bv.outlined) {
    if (bv.outlined >= 0)
      lv_obj_set_style_outline_width(bv.segment[bv.outlined], 0, 0);
    if (current >= 0)
      lv_obj_set_style_outline_width(bv.segment[current], 2, 0);
    bv.outlined = current;
  }
  const bool reserve = !shot.pump_stopped();
  if (reserve) {
    const int32_t rx = std::clamp<int32_t>(
        std::lround((axis - core::ShotSummaryBuilder::kDripReserveS) * k), 0,
        kFriseWidth - 2);
    if (rx != bv.reserve_x) {
      bv.reserve_x = rx;
      lv_obj_set_x(bv.reserve, rx);
      lv_obj_set_width(bv.reserve, kFriseWidth - rx);
    }
  }
  hidden(bv.reserve, !reserve);
}
void tile_detail_after_value(BrewTile &tile) {
  // Après la valeur, à 10 px, sur la ligne de base de la police 32.
  const int32_t x = 16 + text_width(lv_label_get_text(tile.value), theme::kFontUnit) + 10;
  if (lv_obj_get_x(tile.detail) != x)
    lv_obj_set_x(tile.detail, x);
}
void render_brew(const core::Snapshot &s, const core::Config &c, bool done) {
  const core::ShotSummary &shot = s.shot;
  char t[48];
  text(bv.phase, brew_phase_text(s, done));
  if (s.shot_start_unix_s > 0) {
    std::time_t start = static_cast<std::time_t>(s.shot_start_unix_s);
    std::tm local{};
    localtime_r(&start, &local);
    std::strftime(t, sizeof(t), "%d/%m · %H:%M", &local);
    text(bv.started, t);
  } else {
    text(bv.started, "");
  }
  render_frise(shot, active(s) || s.capture_cooldown);

  fmt_digits(t, sizeof(t), shot.cup_weight_g, 1, " g");
  text(bv.tile[0].value, t);
  fmt_digits(t, sizeof(t), shot.infusion_s, 1, " s");
  text(bv.tile[1].value, t);
  fmt_digits(t, sizeof(t), shot.cup_mean_c, 1, "°");
  text(bv.tile[2].value, t);
  if (std::isfinite(shot.cup_mean_c))
    std::snprintf(t, sizeof(t), "cible %.0f°", static_cast<double>(c.brew_temperature_c));
  else
    t[0] = '\0';
  text(bv.tile[2].detail, t);
  tile_detail_after_value(bv.tile[2]);
  const float total = shot.pump_stopped() ? shot.pump_stop_s : shot.elapsed_s;
  fmt_digits(t, sizeof(t), total, 1, " s");
  text(bv.tile[3].value, t);
  if (std::isfinite(shot.pump_start_s)) {
    char pump[16];
    fmt_digits(pump, sizeof(pump), total - shot.pump_start_s, 1, "");
    std::snprintf(t, sizeof(t), "pompe %s", pump);
    text(bv.tile[3].aside, t);
  } else {
    text(bv.tile[3].aside, "");
  }
  fmt_digits(t, sizeof(t), shot.infusion_cup_g_s, 2, " g/s");
  text(bv.tile[4].value, t);
  if (std::isfinite(shot.infusion_ml_s))
    fmt_digits(t, sizeof(t), shot.infusion_ml_s, 2, " ml/s");
  else
    t[0] = '\0';
  text(bv.tile[4].aside, t);
  if (std::isfinite(shot.min_temperature_c)) {
    fmt_digits(t, sizeof(t), -std::max(0.0f, shot.start_temperature_c - shot.min_temperature_c), 1, "°");
    text(bv.tile[5].value, t);
    char low[12], high[12];
    fmt_digits(low, sizeof(low), shot.min_temperature_c, 1, "");
    fmt_digits(high, sizeof(high), shot.max_temperature_c, 1, "°");
    std::snprintf(t, sizeof(t), "%s-%s", low, high);
    text(bv.tile[5].detail, t);
  } else {
    text(bv.tile[5].value, "-");
    text(bv.tile[5].detail, "");
  }
  tile_detail_after_value(bv.tile[5]);

  const bool flowing = !done;
  hidden(bv.live, !flowing);
  hidden(bv.stop, !flowing);
  hidden(bv.close, flowing);
  if (flowing) {
    std::snprintf(t, sizeof(t), "%.0f %%", static_cast<double>(s.heating_power_pct));
    text(bv.live_value[0], t);
    std::snprintf(t, sizeof(t), "%u %%", s.dimmer_pct);
    text(bv.live_value[1], t);
    fmt_digits(t, sizeof(t), shot.cup_rate_g_s, 2, " g/s");
    text(bv.live_value[2], t);
    fmt_digits(t, sizeof(t), s.flow_ml_s, 2, " ml/s");
    text(bv.live_value[3], t);
  } else {
    if (s.capture_cooldown)
      std::snprintf(t, sizeof(t), "fermer · %lu s",
                    ulong((s.capture_cooldown_remaining_ms + 999) / 1000));
    else
      std::snprintf(t, sizeof(t), "fermer");
    text(lv_obj_get_child(bv.close, 0), t);
    disable(bv.close, s.capture_cooldown);
  }
}
void cycle(const core::Snapshot &s, const core::Config &c) {
  bool a = active(s), done = s.cycle_state == core::CycleState::kFinished;
  cycle_visible(a || done);
  // Une infusion a son propre écran ; la purge garde l'écran de cycle.
  const bool brew_view = s.shot.active && s.cycle_state != core::CycleState::kPurge && (a || done);
  hidden(bv.root, !brew_view);
  if (brew_view) {
    hidden(v.cycle, true);
    render_brew(s, c, done);
    return;
  }
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
                : s.cycle_state == core::CycleState::kThermalPreheat
                    ? "précharge thermique"
                : s.cycle_state == core::CycleState::kFilling
                    ? "remplissage"
                : s.cycle_state == core::CycleState::kPreinfusion
                    ? "pré-infusion"
                : s.cycle_state == core::CycleState::kRampdown ? "rampe"
                                                               : "infusion");
  color(v.phase, s.cycle_state == core::CycleState::kThermalPreheat
                     ? theme::kThermal
                     : s.cycle_state == core::CycleState::kPreinfusion
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
      s.cycle_state == core::CycleState::kThermalPreheat ? theme::kThermal
      : s.cycle_state == core::CycleState::kPreinfusion ? theme::kRampLow
      : s.cycle_state == core::CycleState::kRampdown  ? theme::kRampFull
                                                      : theme::kAccent,
      0);
  color(v.hero, s.cycle_state == core::CycleState::kThermalPreheat
                    ? theme::kThermal
                    : s.cycle_state == core::CycleState::kPreinfusion
                    ? theme::kRampLow
                    : theme::kAccent);
  color(v.hero_time, s.cycle_state == core::CycleState::kThermalPreheat
                         ? theme::kThermal
                         : s.cycle_state == core::CycleState::kPreinfusion
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
  const bool wifi = std::strcmp(t, "Mode wifi") == 0;
  lv_obj_set_y(v.full_title, wifi ? 96 : 154);
  lv_obj_set_y(v.full_body, wifi ? 162 : 220);
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
  lv_obj_t *temperature_column = topbar_group(sensors, 110);
  lv_obj_set_flex_flow(temperature_column, LV_FLEX_FLOW_COLUMN);
  lv_obj_set_flex_align(temperature_column, LV_FLEX_ALIGN_CENTER,
                        LV_FLEX_ALIGN_END, LV_FLEX_ALIGN_CENTER);
  lv_obj_set_style_pad_row(temperature_column, 1, 0);
  dyn(temperature_column, &v.temperature, "-", theme::kFontStatus,
      theme::kTextDim, 0, 0);
  lv_obj_set_width(v.temperature, 110);
  lv_label_set_long_mode(v.temperature, LV_LABEL_LONG_CLIP);
  lv_obj_set_style_text_align(v.temperature, LV_TEXT_ALIGN_RIGHT, 0);
  lv_obj_t *heat_bar = lv_obj_create(temperature_column);
  lv_obj_remove_style_all(heat_bar);
  lv_obj_set_size(heat_bar, kHeatBarWidth, 4);
  lv_obj_remove_flag(heat_bar, LV_OBJ_FLAG_SCROLLABLE);
  v.heat_track =
      box(heat_bar, 0, 0, kHeatBarWidth, 4, theme::kBgRaised, 2);
  v.heat_fill = box(heat_bar, 0, 0, 0, 4, theme::kTextFaint, 2);
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
  build_brew(p);
  v.settings = lv_obj_create(p);
  base(v.settings);
  lv_obj_t *settings_bar =
      navbar(v.settings, "réglages", &ignore, back_settings, false, true);
  for (unsigned i = 0; i < 4; ++i) {
    char number[2] = {static_cast<char>('1' + i), '\0'};
    v.tab[i] = button(settings_bar, 536 + static_cast<int>(i) * 64, 12, 56, 64,
                      number);
    lv_obj_add_event_cb(v.tab[i], select_settings_page, LV_EVENT_CLICKED,
                        reinterpret_cast<void *>(uintptr_t(i)));
  }
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
                         "pompe",    "vanne",       "chauffage",
                         "bus can",  "balance",     "versions"};
  for (unsigned i = 0; i < 9; ++i) {
    int x = 32 + (i % 3) * 248, y = 104 + (i / 3) * 112;
    v.diag_tile[i] = diagnostic_tile(v.diag, x, y, names[i]);
    if (i == 8) {
      // La case versions est la seule à porter deux couples valeur/détail.
      lv_obj_set_width(v.diag_tile[i].value, 112);
      dyn(v.diag, &v.diag_version_value, "-", theme::kFontButton, theme::kText,
          x + 120, y + 28);
      dyn(v.diag, &v.diag_version_detail, "sensors", theme::kFontLabel,
          theme::kTextFaint, x + 120, y + 62);
    }
    if (i == 3) {
      lv_obj_t *pump_tile = lv_obj_create(v.diag);
      lv_obj_remove_style_all(pump_tile);
      lv_obj_set_size(pump_tile, 240, 96);
      lv_obj_set_pos(pump_tile, x, y);
      lv_obj_add_flag(pump_tile, LV_OBJ_FLAG_CLICKABLE);
      lv_obj_add_event_cb(pump_tile, show_dimmer_menu, LV_EVENT_CLICKED, nullptr);
    }
    if (i == 4) {
      lv_obj_t *valve_tile = lv_obj_create(v.diag);
      lv_obj_remove_style_all(valve_tile);
      lv_obj_set_size(valve_tile, 240, 96);
      lv_obj_set_pos(valve_tile, x, y);
      lv_obj_add_flag(valve_tile, LV_OBJ_FLAG_CLICKABLE);
      lv_obj_add_event_cb(valve_tile, show_valve_menu, LV_EVENT_CLICKED, nullptr);
    }
    if (i == 5) {
      lv_obj_t *heating_tile = lv_obj_create(v.diag);
      lv_obj_remove_style_all(heating_tile);
      lv_obj_set_size(heating_tile, 240, 96);
      lv_obj_set_pos(heating_tile, x, y);
      lv_obj_add_flag(heating_tile, LV_OBJ_FLAG_CLICKABLE);
      lv_obj_add_event_cb(heating_tile, show_heating_menu, LV_EVENT_CLICKED,
                          nullptr);
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
  v.heating_menu = lv_obj_create(p);
  base(v.heating_menu);
  dyn(v.heating_menu, &v.heating_menu_title, "chauffage",
      theme::kFontSecondary, theme::kAccent, 32, 64);
  dyn(v.heating_menu, &v.heating_menu_body, "", theme::kFontLabel,
      theme::kTextDim, 32, 126);
  v.heating_enable = button(v.heating_menu, 32, 208, 352, 88, "activer",
                            Role::Primary);
  v.heating_disable =
      button(v.heating_menu, 416, 208, 352, 88, "désactiver");
  v.heating_close = button(v.heating_menu, 224, 328, 352, 88, "fermer");
  lv_obj_add_event_cb(v.heating_enable, run_heating_config, LV_EVENT_CLICKED,
                      reinterpret_cast<void *>(uintptr_t(1)));
  lv_obj_add_event_cb(v.heating_disable, run_heating_config, LV_EVENT_CLICKED,
                      reinterpret_cast<void *>(uintptr_t(0)));
  lv_obj_add_event_cb(v.heating_close, close_heating_menu, LV_EVENT_CLICKED,
                      nullptr);
  hidden(v.heating_menu, true);
  v.valve_menu = lv_obj_create(p);
  base(v.valve_menu);
  dyn(v.valve_menu, &v.valve_menu_title, "vanne · maintenance",
      theme::kFontSecondary, theme::kAccent, 32, 64);
  dyn(v.valve_menu, &v.valve_menu_body, "", theme::kFontLabel,
      theme::kTextDim, 32, 126);
  dyn(v.valve_menu, &v.valve_menu_hint,
      "pompe arrêtée · ouvrir la buse vapeur pour laisser entrer l'air",
      theme::kFontLabel, theme::kTextFaint, 32, 156);
  v.valve_open = button(v.valve_menu, 32, 208, 352, 88, "ouvrir 30 s",
                        Role::Primary);
  bind(v.valve_open);
  v.valve_close = button(v.valve_menu, 416, 208, 352, 88, "fermer la vanne");
  v.valve_back = button(v.valve_menu, 224, 328, 352, 88, "retour");
  lv_obj_add_event_cb(v.valve_open, run_valve_action, LV_EVENT_CLICKED,
                      reinterpret_cast<void *>(static_cast<uintptr_t>(core::Action::kOpenMaintenanceValve)));
  lv_obj_add_event_cb(v.valve_close, run_valve_action, LV_EVENT_CLICKED,
                      reinterpret_cast<void *>(static_cast<uintptr_t>(core::Action::kCloseMaintenanceValve)));
  lv_obj_add_event_cb(v.valve_back, close_valve_menu, LV_EVENT_CLICKED, nullptr);
  hidden(v.valve_menu, true);
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
  char t[192];
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
            : theme::ramp_color(s.pressure_bar, 9));
  // Sous 50 °C utilisateur, la température affichée est la lecture sonde, sans
  // offset : machine froide, elle se compare directement à l'ambiante. Elle
  // plafonne à 50 tant que la valeur utilisateur n'a pas rejoint 50, pour que
  // l'affichage ne recule jamais pendant la montée.
  constexpr float kAmbientDisplayLimitC = 50.0f;
  const bool ambient = boiler && s.boiler_temperature_c < kAmbientDisplayLimitC;
  if (boiler)
    fmt(t, sizeof(t),
        ambient ? std::min(s.boiler_sensor_temperature_c, kAmbientDisplayLimitC)
                : s.boiler_temperature_c,
        "°");
  else
    std::snprintf(t, sizeof(t), "-");
  text(v.temperature, t);
  const lv_color_t temperature_color =
      ambient ? theme::kAmbient
      : !c.heating_enabled ? theme::kTextFaint
      : !boiler ? theme::kTextDim
      : s.boiler_temperature_c > c.brew_temperature_c + core::kBrewTemperatureToleranceC ? theme::kFault
      : brew_temperature_ready ? theme::kSuccess
      : s.boiler_temperature_c >= c.brew_temperature_c - 5.0f ? theme::kThermalNear
      : theme::kThermal;
  color(v.temperature, temperature_color);
  lv_obj_set_style_text_opa(v.temperature,
                            LV_OPA_COVER,
                            0);
  const bool heat_available = s.heating_power_capable &&
                              s.heating_freshness == core::Freshness::kFresh;
  const float heat_pct = heat_available
                             ? std::clamp(s.heating_power_pct, 0.0f, 100.0f)
                             : 0.0f;
  const lv_color_t heat_colors[] = {theme::kTextFaint, theme::kTextDim,
                                    theme::kThermalNear, theme::kSuccess};
  const unsigned heat_quarter =
      std::min(static_cast<unsigned>(heat_pct / 25.0f), 3u);
  lv_obj_set_width(
      v.heat_fill,
      static_cast<int>(std::lround(heat_pct * kHeatBarWidth / 100.0f)));
  lv_obj_set_style_bg_color(v.heat_fill, heat_colors[heat_quarter], 0);
  lv_obj_set_style_bg_color(
      v.heat_track, heat_available ? theme::kSurfaceHigh : theme::kBgRaised, 0);
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
  render_valve_menu(s, c);
  if (!lv_obj_has_flag(v.diag, LV_OBJ_FLAG_HIDDEN)) {
    const DiagnosticState pressure_state =
        diagnostic_measure_state(s.pressure_valid, s.pressure_freshness);
    if (pressure_state != DiagnosticState::Error) {
      fmt(t, sizeof(t), s.pressure_bar, " bar");
    } else {
      std::snprintf(t, sizeof(t), "-");
    }
    diagnostic_tile_set(v.diag_tile[0], pressure_state, t,
                        diagnostic_measure_detail(pressure_state));

    const DiagnosticState boiler_state = diagnostic_measure_state(
        s.boiler_temperature_valid, s.boiler_temperature_freshness);
    if (boiler_state != DiagnosticState::Error) {
      fmt(t, sizeof(t), s.boiler_temperature_c, "°");
    } else {
      std::snprintf(t, sizeof(t), "-");
    }
    diagnostic_tile_set(v.diag_tile[1], boiler_state, t,
                        diagnostic_measure_detail(boiler_state));

    const DiagnosticState flow_state =
        diagnostic_measure_state(s.flow_valid, s.flow_freshness);
    if (flow_state != DiagnosticState::Error)
      fmt(t, sizeof(t), s.flow_ml_s, " ml/s");
    else
      std::snprintf(t, sizeof(t), "-");
    diagnostic_tile_set(v.diag_tile[2], flow_state, t,
                        diagnostic_measure_detail(flow_state));

    const DiagnosticState pump_state =
        s.dimmer_error_active ? DiagnosticState::Error
        : !s.sensors_alive || !s.dimmer_valid ? DiagnosticState::Error
        : !s.dimmer_ready ? DiagnosticState::Warn
                          : DiagnosticState::Valid;
    std::snprintf(t, sizeof(t), "%u %%", s.dimmer_pct);
    diagnostic_tile_set(v.diag_tile[3], pump_state, t,
                        s.dimmer_error_active ? "erreur"
                        : pump_state == DiagnosticState::Valid ? "valide"
                        : pump_state == DiagnosticState::Warn ? "calibration"
                                                              : "absent");

    const DiagnosticState valve_state = diagnostic_measure_state(
        s.sensors_alive, s.actuators_freshness);
    diagnostic_tile_set(v.diag_tile[4], valve_state,
                        s.valve_open ? "ouverte" : "fermée",
                        s.maintenance_valve_open && valve_state != DiagnosticState::Error
                            ? "maintenance"
                            : diagnostic_measure_detail(valve_state));

    const DiagnosticState heating_state = !c.heating_enabled
        ? DiagnosticState::Disabled
        : diagnostic_measure_state(s.sensors_alive && s.heating_power_capable,
                                   s.heating_freshness);
    if (c.heating_enabled)
      fmt(t, sizeof(t), s.heating_power_accepted_pct, " %");
    else
      std::snprintf(t, sizeof(t), "OFF");
    diagnostic_tile_set(v.diag_tile[5], heating_state, t,
                        heating_state == DiagnosticState::Disabled
                            ? "désactivé"
                            : diagnostic_measure_detail(heating_state));

    const uint64_t can_errors =
        static_cast<uint64_t>(s.sensors_twai_rx_errors) +
        static_cast<uint64_t>(s.sensors_twai_tx_errors) +
        static_cast<uint64_t>(s.sensors_twai_bus_errors);
    format_error_count(t, sizeof(t), can_errors);
    diagnostic_tile_set(v.diag_tile[6],
                        s.sensors_alive ? DiagnosticState::Valid
                                        : DiagnosticState::Error,
                        s.sensors_alive ? "OK" : "ERREUR", t);

    if (scale) {
      fmt(t, sizeof(t), s.weight_g, " g");
    } else {
      std::snprintf(t, sizeof(t), "-");
    }
    diagnostic_tile_set(v.diag_tile[7],
                        scale ? DiagnosticState::Valid : DiagnosticState::Error,
                        t, scale ? "présente" : "absente");

    std::snprintf(t, sizeof(t), "%u.%u.%u", s.screen_version_major,
                  s.screen_version_minor, s.screen_version_patch);
    diagnostic_tile_set(v.diag_tile[8],
                        s.sensors_alive ? DiagnosticState::Valid
                                        : DiagnosticState::Warn,
                        t, "screen");
    std::snprintf(t, sizeof(t), "%u.%u.%u", s.sensors_version_major,
                  s.sensors_version_minor, s.sensors_version_patch);
    text(v.diag_version_value, t);
    text(v.diag_version_detail, "sensors");
  }
  if (s.boot_time_syncing)
    fullscreen(true, "synchronisation heure", "connexion wifi...");
  else if (s.flash_active) {
    const char *target = s.flash_target == core::FlashTarget::kScreen ? "écran" :
                         s.flash_target == core::FlashTarget::kSensors ? "capteurs" : "firmware";
    std::snprintf(t, sizeof(t), "mise à jour · %s\nne pas couper la machine", target);
    fullscreen(true, "mise à jour", t);
  }
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
      std::snprintf(t, sizeof(t), "activation du réseau\n%s\nversion écran · %u.%u.%u", recording,
                    s.screen_version_major, s.screen_version_minor, s.screen_version_patch);
    else if (s.ipv4_address)
      std::snprintf(t, sizeof(t), "adresse ip · %u.%u.%u.%u\n%s\nversion écran · %u.%u.%u",
                    static_cast<unsigned>(s.ipv4_address & 255),
                    static_cast<unsigned>((s.ipv4_address >> 8) & 255),
                    static_cast<unsigned>((s.ipv4_address >> 16) & 255),
                    static_cast<unsigned>((s.ipv4_address >> 24) & 255), recording,
                    s.screen_version_major, s.screen_version_minor, s.screen_version_patch);
    else
      std::snprintf(t, sizeof(t), "configuration wifi ou association en cours\n%s\nversion écran · %u.%u.%u",
                    recording, s.screen_version_major, s.screen_version_minor,
                    s.screen_version_patch);
    if (ota_local::pending_verify()) {
      std::snprintf(t, sizeof(t), "veuillez confirmer le firmware v%u.%u.%u\nPOST /firmware/confirm",
                    s.screen_version_major, s.screen_version_minor,
                    s.screen_version_patch);
    }
    fullscreen(true, "Mode wifi", t);
  } else if (boot)
    fullscreen(true, "coffeeflow", "démarrage");
  else
    fullscreen(false, "", "");
  hidden(v.wifi_exit, s.flash_active || s.radio_mode != core::RadioMode::kWifi);
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
  } else if (std::strcmp(scenario, "diagnostic") == 0 ||
             std::strcmp(scenario, "diagnostic-errors") == 0 ||
             std::strcmp(scenario, "diagnostic-states") == 0) {
    close_all();
    hidden(v.diag, false);
  } else if (std::strcmp(scenario, "heating-menu-on") == 0 ||
             std::strcmp(scenario, "heating-menu-off") == 0) {
    close_all();
    hidden(v.diag, false);
    show_heating_menu(nullptr);
  } else if (std::strcmp(scenario, "valve-menu") == 0 ||
             std::strcmp(scenario, "valve-menu-open") == 0 ||
             std::strcmp(scenario, "valve-menu-heating") == 0) {
    close_all();
    hidden(v.diag, false);
    show_valve_menu(nullptr);
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

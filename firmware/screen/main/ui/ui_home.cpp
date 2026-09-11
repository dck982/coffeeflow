#include "ui_home.h"

#include <cstdio>
#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <cstring>

#include "common/version.hpp"
#include "core/core.h"
#include "esp_attr.h"
#include "esp_timer.h"
#include "lvgl.h"

#include "ui_theme.h"

namespace ui::home {
namespace {

struct View {
  lv_obj_t *pressure, *temperature, *weight, *presence, *target, *target_unit, *target_detail, *brew, *warning;
  lv_obj_t *minus, *plus, *target_tap, *brew_button, *purge_button, *settings_button;
  lv_obj_t *cycle, *cycle_phase, *cycle_hero, *cycle_detail, *cycle_progress, *cycle_stop;
  lv_obj_t *diagnostic, *full, *full_title, *full_body, *standby_title, *standby_body;
  lv_obj_t *settings, *settings_value, *settings_status, *wifi_exit, *wifi_body;
  lv_obj_t *confirm, *confirm_title, *confirm_body;
  lv_obj_t *keypad, *keypad_input;
  lv_obj_t *dim, *standby;
  lv_obj_t* diag_rows[9]{};
  lv_obj_t* diag_dots[9]{};
  lv_obj_t* settings_buttons[6]{};
};
View g_view{};

// lv_label_set_text() copie chaque chaîne dans le tas LVGL (SRAM interne).
// Les valeurs CAN qui changent régulièrement ne doivent pas fragmenter ce
// tas, déjà très contraint par le LCD RGB et les radios. Ces buffers ont une
// durée de vie égale à l'écran et résident donc explicitement en PSRAM.
constexpr size_t kDynamicLabelCount = 48;
constexpr size_t kDynamicTextLength = 128;
struct TextBinding { lv_obj_t* label; char text[kDynamicTextLength]; };
EXT_RAM_BSS_ATTR TextBinding g_text_bindings[kDynamicLabelCount]{};
size_t g_text_binding_count = 0;
int64_t g_last_activity_us = 0;

void note_activity_cb(lv_event_t*) { g_last_activity_us = esp_timer_get_time(); }

TextBinding* binding_for(lv_obj_t* label) {
  for (size_t i = 0; i < g_text_binding_count; ++i)
    if (g_text_bindings[i].label == label) return &g_text_bindings[i];
  return nullptr;
}

void set_text(lv_obj_t* object, const char* text) {
  TextBinding* binding = binding_for(object);
  if (binding == nullptr) return;
  if (std::strcmp(binding->text, text) == 0) return;
  // L'ancienne surface doit être invalidée avant d'écraser son buffer.
  lv_obj_invalidate(object);
  std::snprintf(binding->text, sizeof(binding->text), "%s", text);
  lv_label_set_text_static(object, binding->text);
}

void set_color(lv_obj_t* object, lv_color_t color) {
  if (!lv_color_eq(lv_obj_get_style_text_color(object, LV_PART_MAIN), color))
    lv_obj_set_style_text_color(object, color, 0);
}

void set_hidden(lv_obj_t* object, bool hidden) {
  if (object == nullptr) return;
  if (hidden) lv_obj_add_flag(object, LV_OBJ_FLAG_HIDDEN);
  else lv_obj_remove_flag(object, LV_OBJ_FLAG_HIDDEN);
}

void label(lv_obj_t* parent, lv_obj_t** out, const char* text, const lv_font_t* font,
           lv_color_t color, int x, int y) {
  *out = lv_label_create(parent);
  lv_obj_set_style_text_font(*out, font, 0);
  lv_obj_set_style_text_color(*out, color, 0);
  lv_label_set_text(*out, text);
  lv_obj_set_pos(*out, x, y);
}

void dynamic_label(lv_obj_t* parent, lv_obj_t** out, const char* text, const lv_font_t* font,
                   lv_color_t color, int x, int y) {
  label(parent, out, text, font, color, x, y);
  if (g_text_binding_count == kDynamicLabelCount) return;
  TextBinding& binding = g_text_bindings[g_text_binding_count++];
  binding.label = *out;
  std::snprintf(binding.text, sizeof(binding.text), "%s", text);
  lv_label_set_text_static(*out, binding.text);
}

void bind_button_caption(lv_obj_t* button) {
  lv_obj_t* caption = lv_obj_get_child(button, 0);
  if (g_text_binding_count == kDynamicLabelCount) return;
  TextBinding& binding = g_text_bindings[g_text_binding_count++];
  binding.label = caption;
  std::snprintf(binding.text, sizeof(binding.text), "%s", lv_label_get_text(caption));
  lv_label_set_text_static(caption, binding.text);
}

lv_obj_t* outline_button(lv_obj_t* parent, int x, int y, int width, const char* text,
                         bool primary = false, bool round = false) {
  constexpr lv_style_selector_t kPressed = static_cast<lv_style_selector_t>(
      static_cast<uint32_t>(LV_PART_MAIN) | static_cast<uint32_t>(LV_STATE_PRESSED));
  lv_obj_t* button = lv_button_create(parent);
  lv_obj_add_event_cb(button, note_activity_cb, LV_EVENT_PRESSED, nullptr);
  lv_obj_set_size(button, width, theme::kButtonHeight);
  lv_obj_set_pos(button, x, y);
  lv_obj_set_style_bg_opa(button, LV_OPA_TRANSP, LV_PART_MAIN);
  lv_obj_set_style_border_color(button, primary ? theme::kAccent : theme::kHairline, LV_PART_MAIN);
  lv_obj_set_style_border_width(button, 2, LV_PART_MAIN);
  lv_obj_set_style_radius(button, theme::kRadius, LV_PART_MAIN);
  lv_obj_set_style_bg_color(button, theme::kAccent, kPressed);
  lv_obj_set_style_bg_opa(button, static_cast<lv_opa_t>(36), kPressed);
  lv_obj_set_style_border_color(button, theme::kAccent, kPressed);
  lv_obj_t* caption = lv_label_create(button);
  lv_obj_set_style_text_font(caption, round ? theme::kFontSecondary : theme::kFontButton, 0);
  lv_obj_set_style_text_color(caption, primary ? theme::kAccent : theme::kText, 0);
  lv_obj_set_style_text_color(caption, theme::kAccent, kPressed);
  lv_label_set_text(caption, text);
  lv_obj_center(caption);
  return button;
}

void hairline(lv_obj_t* parent, int x, int y, int width) {
  lv_obj_t* rule = lv_obj_create(parent);
  lv_obj_set_size(rule, width, 1);
  lv_obj_set_pos(rule, x, y);
  lv_obj_set_style_bg_color(rule, theme::kHairline, 0);
  lv_obj_set_style_border_width(rule, 0, 0);
  lv_obj_set_style_pad_all(rule, 0, 0);
}

void format_decimal(char* out, size_t out_len, float value, const char* suffix) {
  std::snprintf(out, out_len, "%.1f%s", static_cast<double>(value), suffix);
  for (char* c = out; *c != '\0'; ++c) if (*c == '.') *c = ',';
}

bool present(core::Freshness freshness) { return freshness != core::Freshness::kMissing; }

void show_diagnostic(lv_event_t*) { lv_obj_remove_flag(g_view.diagnostic, LV_OBJ_FLAG_HIDDEN); }
void hide_diagnostic(lv_event_t*) { lv_obj_add_flag(g_view.diagnostic, LV_OBJ_FLAG_HIDDEN); }

// Toutes les modifications passent par put_config(): l'UI ne connaît ni NVS
// ni les bornes de stockage.  Le coeur rend l'écriture transactionnelle et
// refuse aussi une modification pendant un cycle.
bool g_scale_present = false;
enum class Confirmation : uint8_t { kNone, kWifi, kForgetNetwork };
Confirmation g_confirmation = Confirmation::kNone;
uint8_t g_settings_page = 0;
void change_target(int direction) {
  core::Config candidate = core::get_config();
  if (g_scale_present) candidate.target_weight_g += .5f * direction;
  else candidate.target_time_s = static_cast<uint16_t>(static_cast<int>(candidate.target_time_s) + direction);
  (void)core::put_config(candidate);
}
void target_minus_cb(lv_event_t*) { change_target(-1); }
void target_plus_cb(lv_event_t*) { change_target(1); }
void hide_keypad(lv_event_t*) { lv_obj_add_flag(g_view.keypad, LV_OBJ_FLAG_HIDDEN); }
void keypad_ready_cb(lv_event_t*) {
  char input[24];
  std::snprintf(input, sizeof(input), "%s", lv_textarea_get_text(g_view.keypad_input));
  for (char* p = input; *p != '\0'; ++p) if (*p == ',') *p = '.';
  char* end = nullptr;
  const float value = std::strtof(input, &end);
  if (end == input || *end != '\0') return;
  core::Config candidate = core::get_config();
  if (g_scale_present) candidate.target_weight_g = value;
  else candidate.target_time_s = static_cast<uint16_t>(value);
  if (core::put_config(candidate).status == core::ConfigStatus::kOk)
    lv_obj_add_flag(g_view.keypad, LV_OBJ_FLAG_HIDDEN);
}
void show_keypad(lv_event_t*) {
  char value[24]; const core::Config c = core::get_config();
  if (g_scale_present) std::snprintf(value, sizeof(value), "%.1f", static_cast<double>(c.target_weight_g));
  else std::snprintf(value, sizeof(value), "%u", static_cast<unsigned>(c.target_time_s));
  lv_textarea_set_text(g_view.keypad_input, value);
  lv_obj_remove_flag(g_view.keypad, LV_OBJ_FLAG_HIDDEN);
}

void settings_render() {
  const core::Config c = core::get_config();
  char value[96];
  std::snprintf(value, sizeof(value), "réglages %u/3 · un appui avance la valeur", static_cast<unsigned>(g_settings_page + 1));
  set_text(g_view.settings_value, value);
  set_text(g_view.settings_status, "");
  const char* captions[6]{};
  char dynamic[6][48]{};
  if (g_settings_page == 0) {
    std::snprintf(dynamic[0], sizeof(dynamic[0]), "poids %.1f g", static_cast<double>(c.target_weight_g));
    std::snprintf(dynamic[1], sizeof(dynamic[1]), "temps %u s", static_cast<unsigned>(c.target_time_s));
    std::snprintf(dynamic[2], sizeof(dynamic[2]), "pré-inf. %s", c.preinfusion_mode == core::PreinfusionMode::kTime ? "temps" : "pression");
    std::snprintf(dynamic[3], sizeof(dynamic[3]), "durée pré-inf. %u s", static_cast<unsigned>(c.preinfusion_time_s));
    std::snprintf(dynamic[4], sizeof(dynamic[4]), "seuil %.1f bar", static_cast<double>(c.preinfusion_pressure_bar));
    std::snprintf(dynamic[5], sizeof(dynamic[5]), "pompe pré-inf. %u %%", static_cast<unsigned>(c.preinfusion_pump_pct));
  } else if (g_settings_page == 1) {
    const char* modes[] = {"aucune", "temps", "poids", "chute pression"};
    std::snprintf(dynamic[0], sizeof(dynamic[0]), "rampe %s", modes[static_cast<unsigned>(c.rampdown_mode)]);
    std::snprintf(dynamic[1], sizeof(dynamic[1]), "avance temps %.1f s", static_cast<double>(c.rampdown_lead_time_s));
    std::snprintf(dynamic[2], sizeof(dynamic[2]), "avance poids %.1f g", static_cast<double>(c.rampdown_lead_weight_g));
    std::snprintf(dynamic[3], sizeof(dynamic[3]), "chute %.1f bar", static_cast<double>(c.rampdown_pressure_drop_bar));
    std::snprintf(dynamic[4], sizeof(dynamic[4]), "pompe infusion %u %%", static_cast<unsigned>(c.brew_pump_pct));
    std::snprintf(dynamic[5], sizeof(dynamic[5]), "pompe purge %u %%", static_cast<unsigned>(c.purge_pump_pct));
  } else {
    std::snprintf(dynamic[0], sizeof(dynamic[0]), "purge max %u s", static_cast<unsigned>(c.purge_max_s));
    captions[1] = "mode wifi"; captions[2] = "réinitialiser réseau";
    captions[3] = "page 1"; captions[4] = "page 2"; captions[5] = "page 3";
  }
  for (size_t i = 0; i < 6; ++i) {
    if (captions[i] == nullptr) captions[i] = dynamic[i];
    set_text(lv_obj_get_child(g_view.settings_buttons[i], 0), captions[i]);
  }
}
void show_settings(lv_event_t*) { g_settings_page = 0; settings_render(); lv_obj_remove_flag(g_view.settings, LV_OBJ_FLAG_HIDDEN); }
void hide_settings(lv_event_t*) { lv_obj_add_flag(g_view.settings, LV_OBJ_FLAG_HIDDEN); }
void next_settings_page_cb(lv_event_t*) { g_settings_page = static_cast<uint8_t>((g_settings_page + 1) % 3); settings_render(); }
float step_wrap(float value, float min, float max, float step) { return value + step > max + .001f ? min : value + step; }
uint16_t step_wrap(uint16_t value, uint16_t min, uint16_t max, uint16_t step) { return value + step > max ? min : static_cast<uint16_t>(value + step); }
uint8_t step_wrap(uint8_t value, uint8_t min, uint8_t max, uint8_t step) { return value + step > max ? min : static_cast<uint8_t>(value + step); }
void show_confirmation(Confirmation confirmation) {
  g_confirmation = confirmation;
  set_text(g_view.confirm_title, confirmation == Confirmation::kWifi ? "mode wifi" : "réinitialiser le réseau");
  set_text(g_view.confirm_body, confirmation == Confirmation::kWifi
      ? "quitter le mode machine et activer le réseau ?"
      : "effacer les identifiants wifi enregistrés ?");
  lv_obj_remove_flag(g_view.confirm, LV_OBJ_FLAG_HIDDEN);
}
void hide_confirmation(lv_event_t*) { g_confirmation = Confirmation::kNone; lv_obj_add_flag(g_view.confirm, LV_OBJ_FLAG_HIDDEN); }
void accept_confirmation(lv_event_t*) {
  if (g_confirmation == Confirmation::kWifi) (void)core::request_radio_mode(core::RadioMode::kWifi);
  else if (g_confirmation == Confirmation::kForgetNetwork) core::forget_network();
  g_confirmation = Confirmation::kNone;
  lv_obj_add_flag(g_view.confirm, LV_OBJ_FLAG_HIDDEN);
}
void enter_wifi_cb(lv_event_t*) {
  show_confirmation(Confirmation::kWifi);
}
void exit_wifi_cb(lv_event_t*) {
  (void)core::request_radio_mode(core::RadioMode::kMachine);
  g_confirmation = Confirmation::kNone;
  lv_obj_add_flag(g_view.confirm, LV_OBJ_FLAG_HIDDEN);
  hide_settings(nullptr);
}
void forget_network_cb(lv_event_t*) {
  show_confirmation(Confirmation::kForgetNetwork);
}

void settings_button_cb(lv_event_t* event) {
  const size_t slot = static_cast<size_t>(reinterpret_cast<uintptr_t>(lv_event_get_user_data(event)));
  if (g_settings_page == 2) {
    if (slot == 0) { core::Config c = core::get_config(); c.purge_max_s = step_wrap(c.purge_max_s, 5, 60, 5); (void)core::put_config(c); }
    else if (slot == 1) { enter_wifi_cb(event); return; }
    else if (slot == 2) { forget_network_cb(event); return; }
    else { g_settings_page = static_cast<uint8_t>(slot - 3); }
    settings_render(); return;
  }
  core::Config c = core::get_config();
  if (g_settings_page == 0) {
    if (slot == 0) c.target_weight_g = step_wrap(c.target_weight_g, 10, 100, .5f);
    else if (slot == 1) c.target_time_s = step_wrap(c.target_time_s, 5, 60, 1);
    else if (slot == 2) c.preinfusion_mode = c.preinfusion_mode == core::PreinfusionMode::kTime ? core::PreinfusionMode::kPressure : core::PreinfusionMode::kTime;
    else if (slot == 3) c.preinfusion_time_s = step_wrap(c.preinfusion_time_s, 0, 20, 1);
    else if (slot == 4) c.preinfusion_pressure_bar = step_wrap(c.preinfusion_pressure_bar, 1, 9, .5f);
    else c.preinfusion_pump_pct = step_wrap(c.preinfusion_pump_pct, 0, 100, 5);
  } else {
    if (slot == 0) c.rampdown_mode = static_cast<core::RampdownMode>((static_cast<unsigned>(c.rampdown_mode) + 1) % 4);
    else if (slot == 1) c.rampdown_lead_time_s = step_wrap(c.rampdown_lead_time_s, 0, 15, .5f);
    else if (slot == 2) c.rampdown_lead_weight_g = step_wrap(c.rampdown_lead_weight_g, 0, 20, .5f);
    else if (slot == 3) c.rampdown_pressure_drop_bar = step_wrap(c.rampdown_pressure_drop_bar, .5f, 4, .5f);
    else if (slot == 4) c.brew_pump_pct = step_wrap(c.brew_pump_pct, 20, 100, 5);
    else c.purge_pump_pct = step_wrap(c.purge_pump_pct, 20, 100, 5);
  }
  (void)core::put_config(c);
  settings_render();
}

// La purge est explicitement un homme-mort : PRESSED démarre, RELEASED et
// PRESS_LOST arrêtent. Le plafond temporel appartient à Machine, pas à LVGL.
void purge_press_cb(lv_event_t*) { (void)core::perform_action({core::Action::kPurgePress}); }
void purge_release_cb(lv_event_t*) { (void)core::perform_action({core::Action::kPurgeRelease}); }
void brew_cb(lv_event_t*) { (void)core::perform_action({core::Action::kStartBrew}); }
void cycle_stop_cb(lv_event_t*) {
  const core::Snapshot s = core::get_snapshot();
  (void)core::perform_action({s.cycle_state == core::CycleState::kFinished ? core::Action::kDismissSummary : core::Action::kStopBrew});
}

bool cycle_active(const core::Snapshot& s) {
  return s.cycle_state == core::CycleState::kPreinfusion || s.cycle_state == core::CycleState::kBrew ||
         s.cycle_state == core::CycleState::kRampdown || s.cycle_state == core::CycleState::kPurge;
}

void set_cycle_visible(bool visible) {
  set_hidden(g_view.cycle, !visible);
  set_hidden(g_view.minus, visible); set_hidden(g_view.plus, visible); set_hidden(g_view.target, visible);
  set_hidden(g_view.target_unit, visible); set_hidden(g_view.target_tap, visible); set_hidden(g_view.target_detail, visible);
  set_hidden(g_view.warning, visible); set_hidden(g_view.brew_button, visible); set_hidden(g_view.purge_button, visible);
  set_hidden(g_view.settings_button, visible);
}

float cycle_progress(const core::Snapshot& s, const core::Config& c) {
  if (s.cycle_state == core::CycleState::kPreinfusion) {
    if (c.preinfusion_mode == core::PreinfusionMode::kTime && c.preinfusion_time_s > 0)
      return static_cast<float>(s.cycle_phase_elapsed_ms) / (c.preinfusion_time_s * 1000.0f);
    return c.preinfusion_pressure_bar > 0 ? s.pressure_bar / c.preinfusion_pressure_bar : 0.0f;
  }
  if (s.cycle_state == core::CycleState::kPurge)
    return c.purge_max_s > 0 ? static_cast<float>(s.cycle_elapsed_ms) / (c.purge_max_s * 1000.0f) : 0.0f;
  if (s.cycle_weight_goal && c.target_weight_g > 0)
    return (s.weight_g - s.cycle_start_weight_g) / c.target_weight_g;
  return c.target_time_s > 0 ? static_cast<float>(s.cycle_elapsed_ms) / (c.target_time_s * 1000.0f) : 0.0f;
}

void render_cycle(const core::Snapshot& s, const core::Config& c) {
  const bool active = cycle_active(s);
  const bool finished = s.cycle_state == core::CycleState::kFinished;
  set_cycle_visible(active || finished);
  if (!active && !finished) return;
  const bool purge = s.cycle_state == core::CycleState::kPurge;
  const bool weight_goal = s.cycle_weight_goal && !purge;
  const float shot_weight = s.weight_g - s.cycle_start_weight_g;
  char text[96];
  if (finished) {
    std::snprintf(text, sizeof(text), "terminé"); set_text(g_view.cycle_phase, text);
    if (s.last_shot_available) {
      std::snprintf(text, sizeof(text), "%.1f g", static_cast<double>(s.last_shot_weight_g));
      set_text(g_view.cycle_hero, text);
      std::snprintf(text, sizeof(text), "%lu s · %.1f ml/s", static_cast<unsigned long>(s.last_shot_duration_ms / 1000), static_cast<double>(s.last_shot_flow_ml_s));
      set_text(g_view.cycle_detail, text);
    } else { set_text(g_view.cycle_hero, "-"); set_text(g_view.cycle_detail, "purge terminée"); }
    lv_obj_set_width(g_view.cycle_progress, 420); set_color(g_view.cycle_hero, theme::kRampFull);
    set_text(lv_obj_get_child(g_view.cycle_stop, 0), "fermer");
    return;
  }
  const char* phase = purge ? "purge" : s.cycle_state == core::CycleState::kPreinfusion ? "pré-infusion" :
                      s.cycle_state == core::CycleState::kRampdown ? "rampe" : "infusion";
  set_text(g_view.cycle_phase, phase);
  if (weight_goal) std::snprintf(text, sizeof(text), "%.1f g", static_cast<double>(shot_weight));
  else std::snprintf(text, sizeof(text), "%lu s", static_cast<unsigned long>(s.cycle_elapsed_ms / 1000));
  set_text(g_view.cycle_hero, text);
  std::snprintf(text, sizeof(text), "%.1f bar · %.1f ml/s · pompe %u %%", static_cast<double>(s.pressure_bar),
                static_cast<double>(s.flow_ml_s), static_cast<unsigned>(s.dimmer_pct));
  set_text(g_view.cycle_detail, text);
  const float progress = std::clamp(cycle_progress(s, c), 0.0f, 1.0f);
  lv_obj_set_width(g_view.cycle_progress, static_cast<int>(420 * progress));
  lv_obj_set_style_bg_color(g_view.cycle_progress,
      s.cycle_state == core::CycleState::kPreinfusion ? theme::kRampLow :
      s.cycle_state == core::CycleState::kRampdown ? theme::kRampFull : theme::kAccent, 0);
  set_color(g_view.cycle_hero, theme::kAccent);
  set_text(lv_obj_get_child(g_view.cycle_stop, 0), "arrêter");
}

void set_fullscreen(bool visible, const char* title, const char* body) {
  if (!visible) { lv_obj_add_flag(g_view.full, LV_OBJ_FLAG_HIDDEN); return; }
  set_text(g_view.full_title, title);
  set_text(g_view.full_body, body);
  lv_obj_remove_flag(g_view.full, LV_OBJ_FLAG_HIDDEN);
}

void update_idle_overlay(const core::Snapshot& snapshot) {
  const bool active = snapshot.flash_active || snapshot.lockout ||
                      snapshot.cycle_state == core::CycleState::kPreinfusion ||
                      snapshot.cycle_state == core::CycleState::kBrew ||
                      snapshot.cycle_state == core::CycleState::kRampdown ||
                      snapshot.cycle_state == core::CycleState::kPurge;
  if (active) { g_last_activity_us = esp_timer_get_time(); lv_obj_add_flag(g_view.dim, LV_OBJ_FLAG_HIDDEN); return; }
  const uint64_t idle_s = static_cast<uint64_t>((esp_timer_get_time() - g_last_activity_us) / 1000000);
  const core::Config config = core::get_config();
  if (idle_s < config.dim_after_s) { lv_obj_add_flag(g_view.dim, LV_OBJ_FLAG_HIDDEN); return; }
  lv_obj_remove_flag(g_view.dim, LV_OBJ_FLAG_HIDDEN);
  const bool standby = idle_s >= config.standby_after_s;
  lv_obj_set_style_bg_opa(g_view.dim, standby ? LV_OPA_90 : LV_OPA_50, 0);
  if (standby) {
    // Déplacement local, très lent : pas de transition plein écran sur le
    // framebuffer RGB. Le bloc fait une boucle en deux minutes.
    const int phase = static_cast<int>((idle_s - config.standby_after_s) % 120);
    const int offset = phase < 60 ? phase - 30 : 90 - phase;
    lv_obj_set_pos(g_view.standby, 270 + offset, 194 + offset / 3);
    // Respiration très lente, limitée aux deux labels : aucune animation plein écran.
    const int breath = phase < 30 ? phase : (phase < 90 ? 60 - phase : phase - 120);
    const lv_opa_t opa = static_cast<lv_opa_t>(190 + std::abs(breath) * 2);
    lv_obj_set_style_text_opa(g_view.standby_title, opa, 0);
    lv_obj_set_style_text_opa(g_view.standby_body, static_cast<lv_opa_t>(opa * 3 / 4), 0);
    lv_obj_remove_flag(g_view.standby, LV_OBJ_FLAG_HIDDEN);
  }
  else lv_obj_add_flag(g_view.standby, LV_OBJ_FLAG_HIDDEN);
}

}  // namespace

void create(lv_obj_t* parent) {
  g_text_binding_count = 0;
  g_last_activity_us = esp_timer_get_time();
  char profile[40];
  std::snprintf(profile, sizeof(profile), "espresso · v%u.%u.%u", common::kFirmwareVersionMajor,
                common::kFirmwareVersionMinor, common::kFirmwareVersionPatch);
  lv_obj_t* ignored = nullptr;
  label(parent, &ignored, profile, theme::kFontStatus, theme::kText, theme::kMargin, 24);
  // Le héros Inter restreint n'embarque pas U+2014 : le tiret ASCII est le
  // repli déjà retenu au sous-lot 2 pour ne jamais afficher un carré.
  dynamic_label(parent, &g_view.pressure, "-  ·", theme::kFontStatus, theme::kTextDim, 410, 24);
  dynamic_label(parent, &g_view.temperature, "-  ·", theme::kFontStatus, theme::kTextDim, 525, 24);
  dynamic_label(parent, &g_view.weight, "-  ·", theme::kFontStatus, theme::kTextDim, 620, 24);
  dynamic_label(parent, &g_view.presence, "-  -", theme::kFontLabel, theme::kTextFaint, 710, 31);
  // La zone de présence déborde volontairement sous le filet : c'est le
  // raccourci diagnostic décrit dans ui.md, indisponible durant L2/L1.
  lv_obj_t* presence_tap = lv_obj_create(parent);
  lv_obj_remove_style_all(presence_tap);
  lv_obj_set_size(presence_tap, 120, theme::kButtonHeight);
  lv_obj_set_pos(presence_tap, 680, 8);
  lv_obj_set_style_bg_opa(presence_tap, LV_OPA_TRANSP, 0);
  lv_obj_set_style_border_width(presence_tap, 0, 0);
  lv_obj_add_flag(presence_tap, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_add_event_cb(presence_tap, note_activity_cb, LV_EVENT_PRESSED, nullptr);
  lv_obj_add_event_cb(presence_tap, show_diagnostic, LV_EVENT_CLICKED, nullptr);
  hairline(parent, theme::kMargin, 82, theme::kScreenWidth - 2 * theme::kMargin);

  constexpr int kControlY = 150;
  g_view.minus = outline_button(parent, 172, kControlY, theme::kButtonHeight, "-", false, true);
  g_view.plus = outline_button(parent, 540, kControlY, theme::kButtonHeight, "+", false, true);
  lv_obj_add_event_cb(g_view.minus, target_minus_cb, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_event_cb(g_view.plus, target_plus_cb, LV_EVENT_CLICKED, nullptr);
  // Valeur et unité forment une seule chaîne centrée. Deux labels fixés à des
  // coordonnées différentes créaient un trou visuel (notamment « 31    s »).
  dynamic_label(parent, &g_view.target, "-", theme::kFontHeroRest, theme::kTextFaint, 0, 155);
  lv_obj_set_width(g_view.target, theme::kScreenWidth);
  lv_obj_set_style_text_align(g_view.target, LV_TEXT_ALIGN_CENTER, 0);
  dynamic_label(parent, &g_view.target_unit, "", theme::kFontUnit, theme::kTextDim, 484, 194);
  g_view.target_tap = lv_obj_create(parent);
  lv_obj_remove_style_all(g_view.target_tap);
  lv_obj_set_size(g_view.target_tap, 260, 120);
  lv_obj_set_pos(g_view.target_tap, 270, 135);
  lv_obj_set_style_bg_opa(g_view.target_tap, LV_OPA_TRANSP, 0);
  lv_obj_set_style_border_width(g_view.target_tap, 0, 0);
  lv_obj_add_flag(g_view.target_tap, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_add_event_cb(g_view.target_tap, note_activity_cb, LV_EVENT_PRESSED, nullptr);
  lv_obj_add_event_cb(g_view.target_tap, show_keypad, LV_EVENT_CLICKED, nullptr);
  dynamic_label(parent, &g_view.target_detail, "", theme::kFontLabel, theme::kTextFaint, 190, 268);
  dynamic_label(parent, &g_view.warning, "", theme::kFontLabel, theme::kFault, 190, 296);
  constexpr int kButtonsY = 368;
  g_view.brew_button = outline_button(parent, 32, kButtonsY, 280, "infuser", true);
  // Seul le caption du bouton varie avec le mode poids/temps.
  lv_obj_t* brew_caption = lv_obj_get_child(g_view.brew_button, 0);
  if (g_text_binding_count < kDynamicLabelCount) {
    TextBinding& binding = g_text_bindings[g_text_binding_count++];
    binding.label = brew_caption;
    std::snprintf(binding.text, sizeof(binding.text), "%s", "infuser");
    lv_label_set_text_static(brew_caption, binding.text);
  }
  g_view.brew = brew_caption;
  lv_obj_add_event_cb(g_view.brew_button, brew_cb, LV_EVENT_CLICKED, nullptr);
  g_view.purge_button = outline_button(parent, 336, kButtonsY, 200, "purge");
  lv_obj_add_event_cb(g_view.purge_button, purge_press_cb, LV_EVENT_PRESSED, nullptr);
  lv_obj_add_event_cb(g_view.purge_button, purge_release_cb, LV_EVENT_RELEASED, nullptr);
  lv_obj_add_event_cb(g_view.purge_button, purge_release_cb, LV_EVENT_PRESS_LOST, nullptr);
  g_view.settings_button = outline_button(parent, 560, kButtonsY, 208, "réglages");
  lv_obj_add_event_cb(g_view.settings_button, show_settings, LV_EVENT_CLICKED, nullptr);

  // L1 est construit une fois, puis seulement masqué ou mis à jour : aucune
  // recréation d'écran pendant un cycle, ce qui garde les écritures RGB locales.
  g_view.cycle = lv_obj_create(parent);
  lv_obj_remove_style_all(g_view.cycle);
  lv_obj_set_size(g_view.cycle, theme::kScreenWidth, 370);
  lv_obj_set_pos(g_view.cycle, 0, 96);
  dynamic_label(g_view.cycle, &g_view.cycle_phase, "", theme::kFontLabel, theme::kTextDim, 0, 24);
  lv_obj_set_width(g_view.cycle_phase, theme::kScreenWidth);
  lv_obj_set_style_text_align(g_view.cycle_phase, LV_TEXT_ALIGN_CENTER, 0);
  dynamic_label(g_view.cycle, &g_view.cycle_hero, "", theme::kFontHeroBrew, theme::kAccent, 0, 54);
  lv_obj_set_width(g_view.cycle_hero, theme::kScreenWidth);
  lv_obj_set_style_text_align(g_view.cycle_hero, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_t* progress_track = lv_obj_create(g_view.cycle);
  lv_obj_set_size(progress_track, 420, 2); lv_obj_set_pos(progress_track, 190, 186);
  lv_obj_set_style_bg_color(progress_track, theme::kHairline, 0); lv_obj_set_style_border_width(progress_track, 0, 0);
  lv_obj_set_style_pad_all(progress_track, 0, 0);
  g_view.cycle_progress = lv_obj_create(progress_track);
  lv_obj_set_size(g_view.cycle_progress, 0, 2); lv_obj_set_pos(g_view.cycle_progress, 0, 0);
  lv_obj_set_style_bg_color(g_view.cycle_progress, theme::kAccent, 0); lv_obj_set_style_border_width(g_view.cycle_progress, 0, 0);
  lv_obj_set_style_pad_all(g_view.cycle_progress, 0, 0);
  dynamic_label(g_view.cycle, &g_view.cycle_detail, "", theme::kFontSecondary, theme::kTextDim, 0, 208);
  lv_obj_set_width(g_view.cycle_detail, theme::kScreenWidth);
  lv_obj_set_style_text_align(g_view.cycle_detail, LV_TEXT_ALIGN_CENTER, 0);
  g_view.cycle_stop = outline_button(g_view.cycle, 240, 272, 320, "arrêter", true);
  bind_button_caption(g_view.cycle_stop);
  lv_obj_add_event_cb(g_view.cycle_stop, cycle_stop_cb, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_flag(g_view.cycle, LV_OBJ_FLAG_HIDDEN);

  g_view.diagnostic = lv_obj_create(parent);
  lv_obj_set_size(g_view.diagnostic, theme::kScreenWidth, 390);
  lv_obj_set_pos(g_view.diagnostic, 0, 90);
  lv_obj_set_style_bg_color(g_view.diagnostic, theme::kBgRaised, 0);
  lv_obj_set_style_border_width(g_view.diagnostic, 0, 0);
  lv_obj_set_style_radius(g_view.diagnostic, 0, 0);
  lv_obj_set_style_pad_all(g_view.diagnostic, 0, 0);
  lv_obj_t* diagnostic_accent = lv_obj_create(g_view.diagnostic);
  lv_obj_set_size(diagnostic_accent, theme::kScreenWidth, 3); lv_obj_set_pos(diagnostic_accent, 0, 0);
  lv_obj_set_style_bg_color(diagnostic_accent, theme::kAccent, 0); lv_obj_set_style_border_width(diagnostic_accent, 0, 0);
  label(g_view.diagnostic, &ignored, "diagnostic", theme::kFontButton, theme::kText, 32, 20);
  static constexpr const char* kNames[] = {"pression", "température", "débit", "pompe", "vanne",
                                            "balance", "bus can", "réseau", "versions"};
  for (size_t i = 0; i < 9; ++i) {
    g_view.diag_dots[i] = lv_obj_create(g_view.diagnostic);
    lv_obj_set_size(g_view.diag_dots[i], 10, 10); lv_obj_set_pos(g_view.diag_dots[i], 32 + (i % 3) * 250, 77 + static_cast<int>(i / 3) * 76);
    lv_obj_set_style_radius(g_view.diag_dots[i], LV_RADIUS_CIRCLE, 0); lv_obj_set_style_border_width(g_view.diag_dots[i], 0, 0);
    dynamic_label(g_view.diagnostic, &g_view.diag_rows[i], kNames[i], theme::kFontLabel, theme::kTextDim,
                  48 + (i % 3) * 250, 72 + static_cast<int>(i / 3) * 76);
  }
  lv_obj_t* close = outline_button(g_view.diagnostic, 568, 280, 200, "fermer");
  lv_obj_add_event_cb(close, hide_diagnostic, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_flag(g_view.diagnostic, LV_OBJ_FLAG_HIDDEN);

  // Feuille L3 des réglages. Les lignes sont de grandes cibles : ce premier
  // passage expose les réglages qui ont un effet immédiat; les valeurs sont
  // persistées par core::put_config() à chaque appui.
  g_view.settings = lv_obj_create(parent);
  lv_obj_set_size(g_view.settings, theme::kScreenWidth, 390);
  lv_obj_set_pos(g_view.settings, 0, 90);
  lv_obj_set_style_bg_color(g_view.settings, theme::kBgRaised, 0);
  lv_obj_set_style_border_width(g_view.settings, 0, 0);
  lv_obj_set_style_radius(g_view.settings, 0, 0);
  lv_obj_set_style_pad_all(g_view.settings, 0, 0);
  lv_obj_t* settings_accent = lv_obj_create(g_view.settings);
  lv_obj_set_size(settings_accent, theme::kScreenWidth, 3); lv_obj_set_pos(settings_accent, 0, 0);
  lv_obj_set_style_bg_color(settings_accent, theme::kAccent, 0); lv_obj_set_style_border_width(settings_accent, 0, 0);
  label(g_view.settings, &ignored, "réglages", theme::kFontButton, theme::kText, 32, 18);
  dynamic_label(g_view.settings, &g_view.settings_value, "", theme::kFontLabel, theme::kTextDim, 32, 58);
  dynamic_label(g_view.settings, &g_view.settings_status, "", theme::kFontLabel, theme::kFault, 32, 82);
  lv_obj_t* next_settings = outline_button(g_view.settings, 400, 8, 144, "suite");
  lv_obj_add_event_cb(next_settings, next_settings_page_cb, LV_EVENT_CLICKED, nullptr);
  lv_obj_t* close_settings = outline_button(g_view.settings, 568, 8, 200, "fermer");
  static constexpr int kSettingsX[] = {32, 276, 520, 32, 276, 520};
  static constexpr int kSettingsY[] = {120, 120, 120, 224, 224, 224};
  static constexpr int kSettingsW[] = {220, 220, 248, 220, 220, 248};
  for (size_t i = 0; i < 6; ++i) {
    g_view.settings_buttons[i] = outline_button(g_view.settings, kSettingsX[i], kSettingsY[i], kSettingsW[i], "");
    lv_obj_set_style_text_font(lv_obj_get_child(g_view.settings_buttons[i], 0), theme::kFontLabel, 0);
    bind_button_caption(g_view.settings_buttons[i]);
    lv_obj_add_event_cb(g_view.settings_buttons[i], settings_button_cb, LV_EVENT_CLICKED,
                        reinterpret_cast<void*>(static_cast<uintptr_t>(i)));
  }
  lv_obj_add_event_cb(close_settings, hide_settings, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_flag(g_view.settings, LV_OBJ_FLAG_HIDDEN);

  // Confirmation locale, au-dessus de la feuille : pas de texte d'erreur
  // posé entre les lignes, et elle disparaît systématiquement au retour du
  // mode Wi-Fi.
  g_view.confirm = lv_obj_create(parent);
  lv_obj_set_size(g_view.confirm, 520, 250);
  lv_obj_set_pos(g_view.confirm, 140, 115);
  lv_obj_set_style_bg_color(g_view.confirm, theme::kBgRaised, 0);
  lv_obj_set_style_border_color(g_view.confirm, theme::kAccent, 0);
  lv_obj_set_style_border_width(g_view.confirm, 2, 0);
  lv_obj_set_style_radius(g_view.confirm, theme::kRadius, 0);
  lv_obj_set_style_pad_all(g_view.confirm, 0, 0);
  dynamic_label(g_view.confirm, &g_view.confirm_title, "", theme::kFontSecondary, theme::kAccent, 28, 24);
  dynamic_label(g_view.confirm, &g_view.confirm_body, "", theme::kFontLabel, theme::kTextDim, 28, 92);
  lv_obj_set_size(g_view.confirm_body, 464, 28);
  lv_label_set_long_mode(g_view.confirm_body, LV_LABEL_LONG_CLIP);
  lv_obj_t* cancel_confirm = outline_button(g_view.confirm, 28, 138, 216, "annuler");
  lv_obj_t* accept_confirm = outline_button(g_view.confirm, 276, 138, 216, "valider", true);
  lv_obj_add_event_cb(cancel_confirm, hide_confirmation, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_event_cb(accept_confirm, accept_confirmation, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_flag(g_view.confirm, LV_OBJ_FLAG_HIDDEN);

  // Saisie directe des grandes corrections de cible. La valeur passe ensuite
  // par la même validation transactionnelle du coeur que les pas +/-.
  g_view.keypad = lv_obj_create(parent);
  lv_obj_set_size(g_view.keypad, theme::kScreenWidth, 390);
  lv_obj_set_pos(g_view.keypad, 0, 90);
  lv_obj_set_style_bg_color(g_view.keypad, theme::kBgRaised, 0);
  lv_obj_set_style_border_width(g_view.keypad, 0, 0);
  lv_obj_set_style_radius(g_view.keypad, 0, 0);
  lv_obj_set_style_pad_all(g_view.keypad, 16, 0);
  lv_obj_t* keypad_accent = lv_obj_create(g_view.keypad);
  lv_obj_set_size(keypad_accent, theme::kScreenWidth, 3); lv_obj_set_pos(keypad_accent, -16, 0);
  lv_obj_set_style_bg_color(keypad_accent, theme::kAccent, 0); lv_obj_set_style_border_width(keypad_accent, 0, 0);
  g_view.keypad_input = lv_textarea_create(g_view.keypad);
  lv_obj_set_size(g_view.keypad_input, 360, 56);
  lv_obj_set_pos(g_view.keypad_input, 32, 12);
  lv_obj_set_style_text_font(g_view.keypad_input, theme::kFontSecondary, 0);
  lv_obj_t* keyboard = lv_keyboard_create(g_view.keypad);
  lv_keyboard_set_textarea(keyboard, g_view.keypad_input);
  lv_keyboard_set_mode(keyboard, LV_KEYBOARD_MODE_NUMBER);
  lv_obj_set_size(keyboard, 768, 292);
  lv_obj_set_pos(keyboard, 16, 82);
  lv_obj_add_event_cb(keyboard, keypad_ready_cb, LV_EVENT_READY, nullptr);
  lv_obj_add_event_cb(keyboard, hide_keypad, LV_EVENT_CANCEL, nullptr);
  lv_obj_add_flag(g_view.keypad, LV_OBJ_FLAG_HIDDEN);

  g_view.full = lv_obj_create(parent);
  lv_obj_set_size(g_view.full, theme::kScreenWidth, theme::kScreenHeight);
  lv_obj_set_pos(g_view.full, 0, 0);
  lv_obj_set_style_bg_color(g_view.full, theme::kBg, 0);
  lv_obj_set_style_border_width(g_view.full, 0, 0);
  lv_obj_set_style_radius(g_view.full, 0, 0);
  dynamic_label(g_view.full, &g_view.full_title, "coffeeflow", theme::kFontSecondary, theme::kAccent, 32, 154);
  dynamic_label(g_view.full, &g_view.full_body, "", theme::kFontButton, theme::kTextDim, 32, 220);
  g_view.wifi_exit = outline_button(g_view.full, 32, 300, 320, "quitter le mode wifi");
  lv_obj_add_event_cb(g_view.wifi_exit, exit_wifi_cb, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_flag(g_view.wifi_exit, LV_OBJ_FLAG_HIDDEN);
  lv_obj_add_flag(g_view.full, LV_OBJ_FLAG_HIDDEN);

  // Calque L4 : il intercepte le premier doigt, ne le propage jamais au
  // contrôle dessous et réveille donc sans lancer d'action.
  g_view.dim = lv_obj_create(parent);
  lv_obj_set_size(g_view.dim, theme::kScreenWidth, theme::kScreenHeight);
  lv_obj_set_pos(g_view.dim, 0, 0);
  lv_obj_set_style_bg_color(g_view.dim, theme::kBg, 0);
  lv_obj_set_style_border_width(g_view.dim, 0, 0);
  lv_obj_set_style_radius(g_view.dim, 0, 0);
  lv_obj_set_style_pad_all(g_view.dim, 0, 0);
  lv_obj_add_flag(g_view.dim, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_add_event_cb(g_view.dim, note_activity_cb, LV_EVENT_PRESSED, nullptr);
  g_view.standby = lv_obj_create(g_view.dim);
  lv_obj_set_size(g_view.standby, 260, 92);
  lv_obj_set_pos(g_view.standby, 270, 194);
  lv_obj_set_style_bg_opa(g_view.standby, LV_OPA_TRANSP, 0);
  lv_obj_set_style_border_width(g_view.standby, 0, 0);
  lv_obj_set_style_pad_all(g_view.standby, 0, 0);
  label(g_view.standby, &g_view.standby_title, "coffeeflow", theme::kFontSecondary, theme::kRampLow, 42, 8);
  label(g_view.standby, &g_view.standby_body, "au repos", theme::kFontLabel, theme::kTextFaint, 92, 58);
  lv_obj_add_flag(g_view.standby, LV_OBJ_FLAG_HIDDEN);
  lv_obj_add_flag(g_view.dim, LV_OBJ_FLAG_HIDDEN);
}

void refresh(const core::Snapshot& s, bool show_boot) {
  char text[120];
  g_scale_present = s.scale_present;
  const bool pressure = s.pressure_valid && present(s.pressure_freshness);
  if (pressure) format_decimal(text, sizeof(text), s.pressure_bar, " bar  ·"); else std::snprintf(text, sizeof(text), "-  ·");
  set_text(g_view.pressure, text);
  const core::Config config = core::get_config();
  const float pressure_target = s.cycle_state == core::CycleState::kPreinfusion ? config.preinfusion_pressure_bar : 9.0f;
  set_color(g_view.pressure, !pressure ? theme::kTextFaint :
            s.pressure_freshness == core::Freshness::kStale ? theme::kTextDim : theme::ramp_color(s.pressure_bar, pressure_target));
  if (pressure) format_decimal(text, sizeof(text), s.temperature_c, "°  ·"); else std::snprintf(text, sizeof(text), "-  ·");
  set_text(g_view.temperature, text);
  set_color(g_view.temperature, !pressure ? theme::kTextFaint :
            s.pressure_freshness == core::Freshness::kStale ? theme::kTextDim : theme::kText);
  if (s.scale_present) format_decimal(text, sizeof(text), s.weight_g, " g  ·"); else std::snprintf(text, sizeof(text), "-  ·");
  set_text(g_view.weight, text);
  set_color(g_view.weight, s.scale_present ? theme::kText : theme::kTextFaint);
  std::snprintf(text, sizeof(text), "%s  %s", s.scale_present ? "bal" : "-", s.sensors_alive ? "can" : "-");
  set_text(g_view.presence, text);
  set_color(g_view.presence, s.sensors_alive ? theme::kText : theme::kTextFaint);

  if (s.scale_present) {
    format_decimal(text, sizeof(text), config.target_weight_g, " g");
    set_text(g_view.target, text); set_text(g_view.target_unit, "");
    std::snprintf(text, sizeof(text), "cible · pré-infusion %u s · rampe sur chute de pression", static_cast<unsigned>(config.preinfusion_time_s));
    char brew[40]; std::snprintf(brew, sizeof(brew), "infuser · %.0f g", static_cast<double>(config.target_weight_g)); set_text(g_view.brew, brew);
  } else {
    std::snprintf(text, sizeof(text), "%u s", static_cast<unsigned>(config.target_time_s));
    set_text(g_view.target, text); set_text(g_view.target_unit, "");
    std::snprintf(text, sizeof(text), "cible temps · balance absente");
    char brew[40]; std::snprintf(brew, sizeof(brew), "infuser · %u s", static_cast<unsigned>(config.target_time_s)); set_text(g_view.brew, brew);
  }
  set_text(g_view.target_detail, text);
  set_text(g_view.warning, (!s.dimmer_ready || !s.dimmer_valid) && s.sensors_alive ? "dimmer en calibration - vérifier le secteur" : "");
  render_cycle(s, config);

  // Ne jamais allouer/renouveler les chaînes de la feuille tant qu'elle est
  // cachée. LVGL stocke le texte de label dans son tas interne : remplir ces
  // neuf lignes à chaque tick consommait et fragmentait inutilement la SRAM.
  if (!lv_obj_has_flag(g_view.diagnostic, LV_OBJ_FLAG_HIDDEN)) {
    const bool flow = s.flow_valid && present(s.flow_freshness);
    char rows[9][96];
    if (pressure) { format_decimal(rows[0], sizeof(rows[0]), s.pressure_bar, " bar · valide"); format_decimal(rows[1], sizeof(rows[1]), s.temperature_c, "° · valide"); }
    else { std::snprintf(rows[0], sizeof(rows[0]), "pression · absent"); std::snprintf(rows[1], sizeof(rows[1]), "température · absent"); }
    if (flow) format_decimal(rows[2], sizeof(rows[2]), s.flow_ml_s, " ml/s · valide"); else std::snprintf(rows[2], sizeof(rows[2]), "débit · absent");
    std::snprintf(rows[3], sizeof(rows[3]), "pompe · %u %%", static_cast<unsigned>(s.dimmer_pct));
    std::snprintf(rows[4], sizeof(rows[4]), "vanne · %s", s.valve_open ? "ouverte" : "fermée");
    if (s.scale_present) format_decimal(rows[5], sizeof(rows[5]), s.weight_g, " g · connectée"); else std::snprintf(rows[5], sizeof(rows[5]), "balance · absente");
    std::snprintf(rows[6], sizeof(rows[6]), "bus can · %s", s.sensors_alive ? "valide" : "absent");
    std::snprintf(rows[7], sizeof(rows[7]), "réseau · %s", s.radio_mode == core::RadioMode::kWifi && s.ipv4_address != 0 ? "wifi · connecté" : "machine · ble");
    std::snprintf(rows[8], sizeof(rows[8]), "versions · écran %u.%u.%u", s.screen_version_major, s.screen_version_minor, s.screen_version_patch);
    for (size_t i = 0; i < 9; ++i) set_text(g_view.diag_rows[i], rows[i]);
    const bool states[] = {pressure, pressure, flow, s.dimmer_valid, s.valve_open, s.scale_present,
                           s.sensors_alive, s.radio_mode == core::RadioMode::kWifi && s.ipv4_address != 0, true};
    for (size_t i = 0; i < 9; ++i) {
      const bool fault = (i == 3 && s.dimmer_error_active) || (i == 6 && !s.sensors_alive);
      lv_obj_set_style_bg_color(g_view.diag_dots[i], fault ? theme::kFault : states[i] ? theme::kAccent : theme::kTextFaint, 0);
    }
  }

  if (s.lockout) set_fullscreen(true, "verrou de sécurité", "couper la machine à l'interrupteur principal, puis la rallumer");
  else if (!s.sensors_alive && !show_boot) set_fullscreen(true, "module interne injoignable", "les commandes de pompe et de vanne sont coupées");
  else if (s.flash_active) set_fullscreen(true, "mise à jour", "ne pas couper la machine");
  else if (s.radio_mode == core::RadioMode::kWifi) {
    if (s.radio_transition) std::snprintf(text, sizeof(text), "activation du réseau");
    else if (s.ipv4_address != 0) std::snprintf(text, sizeof(text), "adresse ip · %u.%u.%u.%u",
        static_cast<unsigned>((s.ipv4_address >> 24) & 0xff), static_cast<unsigned>((s.ipv4_address >> 16) & 0xff),
        static_cast<unsigned>((s.ipv4_address >> 8) & 0xff), static_cast<unsigned>(s.ipv4_address & 0xff));
    else std::snprintf(text, sizeof(text), "configuration wifi ou association en cours");
    set_fullscreen(true, "wifi mode", text);
    lv_obj_remove_flag(g_view.wifi_exit, LV_OBJ_FLAG_HIDDEN);
  }
  else if (show_boot) set_fullscreen(true, "coffeeflow", "démarrage");
  else { set_fullscreen(false, "", ""); lv_obj_add_flag(g_view.wifi_exit, LV_OBJ_FLAG_HIDDEN); }
  update_idle_overlay(s);
}

}  // namespace ui::home

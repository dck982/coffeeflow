#include "ui_home.h"

#include <cstdio>
#include <cstring>

#include "common/version.hpp"
#include "core/core.h"
#include "esp_attr.h"
#include "lvgl.h"

#include "ui_theme.h"

namespace ui::home {
namespace {

struct View {
  lv_obj_t *pressure, *temperature, *weight, *presence, *target, *target_unit, *target_detail, *brew, *warning;
  lv_obj_t *diagnostic, *full, *full_title, *full_body;
  lv_obj_t* diag_rows[9]{};
};
View g_view{};

// lv_label_set_text() copie chaque chaîne dans le tas LVGL (SRAM interne).
// Les valeurs CAN qui changent régulièrement ne doivent pas fragmenter ce
// tas, déjà très contraint par le LCD RGB et les radios. Ces buffers ont une
// durée de vie égale à l'écran et résident donc explicitement en PSRAM.
constexpr size_t kDynamicLabelCount = 20;
constexpr size_t kDynamicTextLength = 128;
struct TextBinding { lv_obj_t* label; char text[kDynamicTextLength]; };
EXT_RAM_BSS_ATTR TextBinding g_text_bindings[kDynamicLabelCount]{};
size_t g_text_binding_count = 0;

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

lv_obj_t* outline_button(lv_obj_t* parent, int x, int y, int width, const char* text,
                         bool primary = false, bool round = false) {
  constexpr lv_style_selector_t kPressed = static_cast<lv_style_selector_t>(
      static_cast<uint32_t>(LV_PART_MAIN) | static_cast<uint32_t>(LV_STATE_PRESSED));
  lv_obj_t* button = lv_button_create(parent);
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

void wifi_button_cb(lv_event_t*) {
  const core::RadioMode next = core::radio_mode() == core::RadioMode::kWifi
                                   ? core::RadioMode::kMachine : core::RadioMode::kWifi;
  (void)core::request_radio_mode(next);
}

void set_fullscreen(bool visible, const char* title, const char* body) {
  if (!visible) { lv_obj_add_flag(g_view.full, LV_OBJ_FLAG_HIDDEN); return; }
  set_text(g_view.full_title, title);
  set_text(g_view.full_body, body);
  lv_obj_remove_flag(g_view.full, LV_OBJ_FLAG_HIDDEN);
}

}  // namespace

void create(lv_obj_t* parent) {
  g_text_binding_count = 0;
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
  hairline(parent, theme::kMargin, 82, theme::kScreenWidth - 2 * theme::kMargin);

  lv_obj_t* wifi = outline_button(parent, 32, 96, 112, "wifi");
  lv_obj_add_event_cb(wifi, wifi_button_cb, LV_EVENT_CLICKED, nullptr);
  constexpr int kControlY = 150;
  outline_button(parent, 172, kControlY, theme::kButtonHeight, "-", false, true);
  outline_button(parent, 540, kControlY, theme::kButtonHeight, "+", false, true);
  dynamic_label(parent, &g_view.target, "-", theme::kFontHeroRest, theme::kTextFaint, 302, 155);
  dynamic_label(parent, &g_view.target_unit, "", theme::kFontUnit, theme::kTextDim, 484, 194);
  dynamic_label(parent, &g_view.target_detail, "", theme::kFontLabel, theme::kTextFaint, 190, 268);
  dynamic_label(parent, &g_view.warning, "", theme::kFontLabel, theme::kFault, 190, 296);
  constexpr int kButtonsY = 368;
  lv_obj_t* brew_button = outline_button(parent, 32, kButtonsY, 280, "infuser");
  // Seul le caption du bouton varie avec le mode poids/temps.
  lv_obj_t* brew_caption = lv_obj_get_child(brew_button, 0);
  if (g_text_binding_count < kDynamicLabelCount) {
    TextBinding& binding = g_text_bindings[g_text_binding_count++];
    binding.label = brew_caption;
    std::snprintf(binding.text, sizeof(binding.text), "%s", "infuser");
    lv_label_set_text_static(brew_caption, binding.text);
  }
  g_view.brew = brew_caption;
  outline_button(parent, 336, kButtonsY, 200, "purge");
  lv_obj_t* settings = outline_button(parent, 560, kButtonsY, 208, "réglages");
  // Le sous-lot 4 remplacera cette entrée par les réglages effectifs.
  lv_obj_add_event_cb(settings, show_diagnostic, LV_EVENT_CLICKED, nullptr);

  g_view.diagnostic = lv_obj_create(parent);
  lv_obj_set_size(g_view.diagnostic, theme::kScreenWidth, 390);
  lv_obj_set_pos(g_view.diagnostic, 0, 90);
  lv_obj_set_style_bg_color(g_view.diagnostic, theme::kBgRaised, 0);
  lv_obj_set_style_border_width(g_view.diagnostic, 0, 0);
  lv_obj_set_style_radius(g_view.diagnostic, 0, 0);
  lv_obj_set_style_pad_all(g_view.diagnostic, 0, 0);
  label(g_view.diagnostic, &ignored, "diagnostic", theme::kFontButton, theme::kText, 32, 20);
  static constexpr const char* kNames[] = {"pression", "température", "débit", "pompe", "vanne",
                                            "balance", "bus can", "réseau", "versions"};
  for (size_t i = 0; i < 9; ++i)
    dynamic_label(g_view.diagnostic, &g_view.diag_rows[i], kNames[i], theme::kFontLabel, theme::kTextDim,
                  32 + (i % 3) * 250, 72 + static_cast<int>(i / 3) * 76);
  lv_obj_t* close = outline_button(g_view.diagnostic, 568, 280, 200, "fermer");
  lv_obj_add_event_cb(close, hide_diagnostic, LV_EVENT_CLICKED, nullptr);
  lv_obj_add_flag(g_view.diagnostic, LV_OBJ_FLAG_HIDDEN);

  g_view.full = lv_obj_create(parent);
  lv_obj_set_size(g_view.full, theme::kScreenWidth, theme::kScreenHeight);
  lv_obj_set_pos(g_view.full, 0, 0);
  lv_obj_set_style_bg_color(g_view.full, theme::kBg, 0);
  lv_obj_set_style_border_width(g_view.full, 0, 0);
  lv_obj_set_style_radius(g_view.full, 0, 0);
  dynamic_label(g_view.full, &g_view.full_title, "coffeeflow", theme::kFontSecondary, theme::kAccent, 32, 154);
  dynamic_label(g_view.full, &g_view.full_body, "", theme::kFontButton, theme::kTextDim, 32, 220);
  lv_obj_add_flag(g_view.full, LV_OBJ_FLAG_HIDDEN);
}

void refresh(const core::Snapshot& s, bool show_boot) {
  char text[120];
  const bool pressure = s.pressure_valid && present(s.pressure_freshness);
  if (pressure) format_decimal(text, sizeof(text), s.pressure_bar, " bar  ·"); else std::snprintf(text, sizeof(text), "-  ·");
  set_text(g_view.pressure, text);
  set_color(g_view.pressure, !pressure ? theme::kTextFaint :
            s.pressure_freshness == core::Freshness::kStale ? theme::kTextDim : theme::kText);
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

  const core::Config config = core::get_config();
  if (s.scale_present) {
    format_decimal(text, sizeof(text), config.target_weight_g, "");
    set_text(g_view.target, text); set_text(g_view.target_unit, "g");
    std::snprintf(text, sizeof(text), "cible · pré-infusion %u s · rampe sur chute de pression", static_cast<unsigned>(config.preinfusion_time_s));
    char brew[40]; std::snprintf(brew, sizeof(brew), "infuser · %.0f g", static_cast<double>(config.target_weight_g)); set_text(g_view.brew, brew);
  } else {
    std::snprintf(text, sizeof(text), "%u", static_cast<unsigned>(config.target_time_s));
    set_text(g_view.target, text); set_text(g_view.target_unit, "s");
    std::snprintf(text, sizeof(text), "cible temps · balance absente");
    char brew[40]; std::snprintf(brew, sizeof(brew), "infuser · %u s", static_cast<unsigned>(config.target_time_s)); set_text(g_view.brew, brew);
  }
  set_text(g_view.target_detail, text);
  set_text(g_view.warning, (!s.dimmer_ready || !s.dimmer_valid) && s.sensors_alive ? "dimmer en calibration - vérifier le secteur" : "");

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
  }

  if (s.lockout) set_fullscreen(true, "verrou de sécurité", "couper la machine à l'interrupteur principal, puis la rallumer");
  else if (!s.sensors_alive && !show_boot) set_fullscreen(true, "module interne injoignable", "les commandes de pompe et de vanne sont coupées");
  else if (s.flash_active) set_fullscreen(true, "mise à jour", "ne pas couper la machine");
  else if (show_boot) set_fullscreen(true, "coffeeflow", "démarrage");
  else set_fullscreen(false, "", "");
}

}  // namespace ui::home

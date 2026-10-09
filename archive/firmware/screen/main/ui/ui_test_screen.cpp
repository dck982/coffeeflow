#include "ui_test_screen.h"

#include <cstring>

#include "common/version.hpp"
#include "core/core.h"
#include "ui_theme.h"

namespace ui::test_screen {
namespace {

const lv_color_t kPalette[] = {
    theme::kBg,       theme::kBgRaised, theme::kHairline, theme::kText,
    theme::kTextDim,  theme::kTextFaint, theme::kAccent,  theme::kFault,
};
constexpr const char* kPaletteNames[] = {
    "bg", "raised", "filet", "texte", "dim", "absent", "ambre", "faute",
};
constexpr size_t kPaletteCount = sizeof(kPalette) / sizeof(kPalette[0]);

lv_obj_t* g_status = nullptr;
lv_obj_t* g_wifi_label = nullptr;
size_t g_touch_count = 0;

void style_label(lv_obj_t* label, const lv_font_t* font, lv_color_t color) {
  lv_obj_set_style_text_font(label, font, 0);
  lv_obj_set_style_text_color(label, color, 0);
}

void touch_cb(lv_event_t*) {
  ++g_touch_count;
  lv_indev_t* indev = lv_indev_get_act();
  lv_point_t point{};
  if (indev != nullptr) lv_indev_get_point(indev, &point);
  lv_obj_set_style_bg_color(lv_screen_active(), kPalette[g_touch_count % kPaletteCount], 0);
  lv_label_set_text_fmt(g_status, "tactile ok  ·  %d, %d", static_cast<int>(point.x), static_cast<int>(point.y));
}

void set_label_if_changed(lv_obj_t* label, const char* text) {
  if (std::strcmp(lv_label_get_text(label), text) != 0) lv_label_set_text(label, text);
}

void refresh_radio_status(lv_timer_t*) {
  const core::Snapshot snapshot = core::get_snapshot();
  if (snapshot.radio_transition) {
    set_label_if_changed(g_status, "changement de mode radio...");
    set_label_if_changed(g_wifi_label, "patiente...");
    return;
  }
  if (snapshot.radio_mode != core::RadioMode::kWifi) {
    set_label_if_changed(g_status, "mode machine · ble actif");
    set_label_if_changed(g_wifi_label, "entrer en mode wifi");
    return;
  }
  switch (static_cast<core::NetworkState>(snapshot.network_state)) {
    case core::NetworkState::kApProvisioning:
      set_label_if_changed(g_status, "wifi · configuration 192.168.4.1");
      break;
    case core::NetworkState::kStaConnecting:
      set_label_if_changed(g_status, "wifi · connexion...");
      break;
    case core::NetworkState::kStaConnected:
      set_label_if_changed(g_status, "wifi · connecté · http prêt");
      break;
    case core::NetworkState::kStaDisconnected:
      set_label_if_changed(g_status, "wifi · réseau non associé");
      break;
    case core::NetworkState::kOff:
      set_label_if_changed(g_status, "wifi · démarrage...");
      break;
  }
  set_label_if_changed(g_wifi_label, "quitter le mode wifi");
}

void wifi_button_cb(lv_event_t*) {
  const core::Snapshot snapshot = core::get_snapshot();
  const core::RadioMode next = snapshot.radio_mode == core::RadioMode::kWifi
                                   ? core::RadioMode::kMachine
                                   : core::RadioMode::kWifi;
  if (!core::request_radio_mode(next)) set_label_if_changed(g_status, "changement de mode refusé");
}

lv_obj_t* add_outline_button(lv_obj_t* parent, int x, int y, int width) {
  lv_obj_t* button = lv_button_create(parent);
  lv_obj_set_size(button, width, theme::kTouchMin);
  lv_obj_set_pos(button, x, y);
  lv_obj_set_style_bg_opa(button, LV_OPA_TRANSP, LV_PART_MAIN);
  lv_obj_set_style_border_color(button, theme::kAccent, LV_PART_MAIN);
  lv_obj_set_style_border_width(button, 2, LV_PART_MAIN);
  lv_obj_set_style_radius(button, theme::kRadius, LV_PART_MAIN);
  return button;
}

void add_swatch(lv_obj_t* parent, size_t index) {
  constexpr int kWidth = 82;
  constexpr int kHeight = 70;
  constexpr int kGap = 12;
  lv_obj_t* swatch = lv_obj_create(parent);
  lv_obj_set_size(swatch, kWidth, kHeight);
  lv_obj_set_pos(swatch, theme::kMargin + static_cast<int>(index) * (kWidth + kGap), 120);
  lv_obj_set_style_bg_color(swatch, kPalette[index], 0);
  lv_obj_set_style_border_color(swatch, theme::kTextDim, 0);
  lv_obj_set_style_border_width(swatch, 1, 0);
  lv_obj_set_style_radius(swatch, theme::kRadius, 0);
  lv_obj_set_style_pad_all(swatch, 4, 0);
  lv_obj_t* name = lv_label_create(swatch);
  style_label(name, theme::kFontLabel, index == 3 ? theme::kBg : theme::kText);
  lv_label_set_text(name, kPaletteNames[index]);
  lv_obj_center(name);
}

void add_type_sample(lv_obj_t* parent, const char* text, const lv_font_t* font, int x, int y) {
  lv_obj_t* sample = lv_label_create(parent);
  style_label(sample, font, theme::kText);
  lv_label_set_text(sample, text);
  lv_obj_set_pos(sample, x, y);
}

}  // namespace

void build() {
  lv_obj_t* screen = lv_screen_active();
  lv_obj_clean(screen);
  lv_obj_set_style_bg_color(screen, theme::kBg, 0);
  lv_obj_set_style_bg_opa(screen, LV_OPA_COVER, 0);
  lv_obj_add_flag(screen, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_add_event_cb(screen, touch_cb, LV_EVENT_PRESSED, nullptr);

  lv_obj_t* title = lv_label_create(screen);
  style_label(title, theme::kFontButton, theme::kAccent);
  lv_label_set_text_fmt(title, "coffeeflow v%u.%u.%u · écran & jetons",
                        common::kFirmwareVersionMajor, common::kFirmwareVersionMinor,
                        common::kFirmwareVersionPatch);
  lv_obj_set_pos(title, theme::kMargin, 28);

  lv_obj_t* rule = lv_obj_create(screen);
  lv_obj_set_size(rule, theme::kScreenWidth - 2 * theme::kMargin, 2);
  lv_obj_set_pos(rule, theme::kMargin, 82);
  lv_obj_set_style_bg_color(rule, theme::kHairline, 0);
  lv_obj_set_style_border_width(rule, 0, 0);
  lv_obj_set_style_radius(rule, 0, 0);

  for (size_t i = 0; i < kPaletteCount; ++i) add_swatch(screen, i);

  // Les sept rôles typographiques sont tous visibles avant d'introduire la
  // moindre donnée machine.
  add_type_sample(screen, "18  étiquette", theme::kFontLabel, 32, 218);
  add_type_sample(screen, "26  bouton", theme::kFontButton, 32, 245);
  add_type_sample(screen, "28  bandeau", theme::kFontStatus, 32, 280);
  add_type_sample(screen, "40  secondaire", theme::kFontSecondary, 32, 318);
  add_type_sample(screen, "36.0 g", theme::kFontHeroRest, 398, 195);
  add_type_sample(screen, "21.6 s", theme::kFontHeroBrew, 398, 265);
  add_type_sample(screen, "32  unité", theme::kFontUnit, 398, 386);

  g_status = lv_label_create(screen);
  style_label(g_status, theme::kFontButton, theme::kTextDim);
  lv_label_set_text(g_status, "mode machine · ble actif");
  lv_obj_set_pos(g_status, theme::kMargin, 424);

  lv_obj_t* wifi = add_outline_button(screen, 538, 392, 230);
  g_wifi_label = lv_label_create(wifi);
  style_label(g_wifi_label, theme::kFontLabel, theme::kAccent);
  lv_label_set_text(g_wifi_label, "entrer en mode wifi");
  lv_obj_center(g_wifi_label);
  lv_obj_add_event_cb(wifi, wifi_button_cb, LV_EVENT_CLICKED, nullptr);
  lv_timer_create(refresh_radio_status, 250, nullptr);
}

}  // namespace ui::test_screen

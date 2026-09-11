#include "ui_home.h"

#include <cstdio>

#include "common/version.hpp"
#include "core/core.h"
#include "lvgl.h"

#include "ui_theme.h"

namespace ui::home {
namespace {

void label(lv_obj_t* parent, const char* text, const lv_font_t* font, lv_color_t color,
           int x, int y) {
  lv_obj_t* object = lv_label_create(parent);
  lv_obj_set_style_text_font(object, font, 0);
  lv_obj_set_style_text_color(object, color, 0);
  lv_label_set_text(object, text);
  lv_obj_set_pos(object, x, y);
}

lv_obj_t* outline_button(lv_obj_t* parent, int x, int y, int width, const char* text,
                         bool primary = false, bool round = false) {
  constexpr lv_style_selector_t kPressed =
      static_cast<lv_style_selector_t>(static_cast<uint32_t>(LV_PART_MAIN) |
                                       static_cast<uint32_t>(LV_STATE_PRESSED));
  lv_obj_t* button = lv_button_create(parent);
  lv_obj_set_size(button, width, theme::kButtonHeight);
  lv_obj_set_pos(button, x, y);
  lv_obj_set_style_bg_opa(button, LV_OPA_TRANSP, LV_PART_MAIN);
  lv_obj_set_style_border_color(button, primary ? theme::kAccent : theme::kHairline, LV_PART_MAIN);
  lv_obj_set_style_border_width(button, 2, LV_PART_MAIN);
  lv_obj_set_style_radius(button, theme::kRadius, LV_PART_MAIN);
  // Retour tactile défini par ui.md, sans encore attacher d'action métier.
  lv_obj_set_style_bg_color(button, theme::kAccent, kPressed);
  lv_obj_set_style_bg_opa(button, static_cast<lv_opa_t>(36), kPressed);  // 14 %
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
  lv_obj_set_style_radius(rule, 0, 0);
  lv_obj_set_style_pad_all(rule, 0, 0);
}

// Accès de transition imposé par le plan jusqu'à la destination Wi-Fi du
// sous-lot 4. L'UI ne manipule aucune pile radio : elle ne fait qu'une demande
// au coeur, qui arrête BLE avant de démarrer Wi-Fi/httpd.
void wifi_button_cb(lv_event_t*) {
  const core::RadioMode next = core::radio_mode() == core::RadioMode::kWifi
                                   ? core::RadioMode::kMachine
                                   : core::RadioMode::kWifi;
  (void)core::request_radio_mode(next);
}

}  // namespace

void create_static(lv_obj_t* parent) {
  // Cadre 1 de docs/ui-mockup.html, aux coordonnées normatives de ui.md.
  char profile[40];
  std::snprintf(profile, sizeof(profile), "espresso · v%u.%u.%u", common::kFirmwareVersionMajor,
                common::kFirmwareVersionMinor, common::kFirmwareVersionPatch);
  label(parent, profile, theme::kFontStatus, theme::kText, theme::kMargin, 24);
  label(parent, "92,4°  ·  0,0 g  ·", theme::kFontStatus, theme::kTextDim, 470, 24);
  // Les pictogrammes définitifs arriveront avec la police d'icônes ; ces
  // marqueurs ASCII conservent ici l'espacement du groupe de présence.
  label(parent, "bal  can", theme::kFontLabel, theme::kText, 694, 31);
  hairline(parent, theme::kMargin, 82, theme::kScreenWidth - 2 * theme::kMargin);

  // Ce petit accès de service reste visible tant que l'écran Wi-Fi final
  // n'existe pas. Sa cible reste 88 px de haut, même si son dessin est discret.
  lv_obj_t* wifi = outline_button(parent, 32, 96, 112, "wifi");
  lv_obj_add_event_cb(wifi, wifi_button_cb, LV_EVENT_CLICKED, nullptr);

  constexpr int kControlY = 150;
  // Le jeu de glyphes héros est volontairement restreint : tiret ASCII, pas
  // signe moins Unicode U+2212 qui serait rendu comme un carré.
  outline_button(parent, 172, kControlY, theme::kButtonHeight, "-", false, true);
  outline_button(parent, 540, kControlY, theme::kButtonHeight, "+", false, true);
  label(parent, "36,0", theme::kFontHeroRest, theme::kText, 302, 155);
  label(parent, "g", theme::kFontUnit, theme::kTextDim, 484, 194);
  label(parent, "cible · pré-infusion 6 s · rampe sur chute de pression",
        theme::kFontLabel, theme::kTextFaint, 190, 268);

  constexpr int kButtonsY = 368;
  outline_button(parent, 32, kButtonsY, 280, "infuser · 36 g", true);
  outline_button(parent, 336, kButtonsY, 200, "purge");
  outline_button(parent, 560, kButtonsY, 208, "réglages");
}

}  // namespace ui::home

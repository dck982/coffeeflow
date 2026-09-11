// Jetons visuels du lot 10. Ce fichier ne dépend que de LVGL : les écrans
// l'emploient, mais ne connaissent ni CAN, ni le coeur machine.
#pragma once

#include "lvgl.h"

namespace ui::theme {

extern "C" {
extern const lv_font_t ui_font_18;
extern const lv_font_t ui_font_26;
extern const lv_font_t ui_font_28;
extern const lv_font_t ui_font_32;
extern const lv_font_t ui_font_40;
extern const lv_font_t ui_font_76;
extern const lv_font_t ui_font_104;
}

inline const lv_color_t kBg = lv_color_hex(0x141110);
inline const lv_color_t kBgRaised = lv_color_hex(0x1E1A18);
inline const lv_color_t kHairline = lv_color_hex(0x332C28);
inline const lv_color_t kText = lv_color_hex(0xF2EBE3);
inline const lv_color_t kTextDim = lv_color_hex(0xA2968C);
inline const lv_color_t kTextFaint = lv_color_hex(0x6B615A);
inline const lv_color_t kAccent = lv_color_hex(0xD98324);
inline const lv_color_t kFault = lv_color_hex(0xB9412F);
inline const lv_color_t kRampLow = lv_color_hex(0x8C5A22);
inline const lv_color_t kRampFull = lv_color_hex(0xF2B25C);

inline constexpr int kScreenWidth = 800;
inline constexpr int kScreenHeight = 480;
inline constexpr int kMargin = 32;
inline constexpr int kTouchMin = 80;
inline constexpr int kButtonHeight = 88;
inline constexpr int kRadius = 8;

// Inter est générée avec lv_font_conv. Les corps héros ne portent que les
// glyphes utiles au café, ce qui garde l'image OTA compacte.
inline const lv_font_t* const kFontLabel = &ui_font_18;
inline const lv_font_t* const kFontButton = &ui_font_26;
inline const lv_font_t* const kFontStatus = &ui_font_28;
inline const lv_font_t* const kFontUnit = &ui_font_32;
inline const lv_font_t* const kFontSecondary = &ui_font_40;
inline const lv_font_t* const kFontHeroRest = &ui_font_76;
inline const lv_font_t* const kFontHeroBrew = &ui_font_104;

}  // namespace ui::theme

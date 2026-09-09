// Écran de service — lot 2 (docs/plan-phase6.md) : dalle RGB 800x480 + GT911
// + esp_lvgl_port, et un affichage volontairement laid (version, état CAN,
// état réseau, IP, cinq derniers événements du cœur, coordonnées tactiles).
// Absorbé par ui/ au lot 10 (docs/plan-phase6.md).
#pragma once

namespace service_screen {

// Alimente la dalle et le tactile via board::panel_power_on(), installe le
// panneau RGB (esp_lcd, bounce buffer activé), le tactile GT911 et
// esp_lvgl_port, puis construit l'écran de service. À appeler une seule fois
// depuis app_main, après board::ch422g_init()/select_can(). L'init LCD
// s'exécute sur le cœur 1 même si l'appelant est sur le cœur 0
// (docs/screen-issue.md).
void init();

}  // namespace service_screen

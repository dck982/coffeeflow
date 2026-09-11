// Racine LVGL du lot 10. Les niveaux ultérieurs s'ajoutent au même écran.
#pragma once

namespace ui::root {

// À appeler sous le verrou esp_lvgl_port et depuis l'unique tâche LVGL.
void build();

}  // namespace ui::root

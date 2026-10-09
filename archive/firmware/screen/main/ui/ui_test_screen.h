// Miroir matériel du sous-lot 10.1 : aucun accès au coeur machine.
#pragma once

namespace ui::test_screen {

// À appeler sous le verrou esp_lvgl_port et depuis l'unique tâche LVGL.
void build();

}  // namespace ui::test_screen

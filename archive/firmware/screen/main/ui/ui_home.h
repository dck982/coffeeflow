// Repos L0/L1 du sous-lot 10.2. Cette étape ne dépend volontairement pas du
// coeur : elle reproduit le premier cadre de la maquette sur données figées.
#pragma once

struct _lv_obj_t;
typedef struct _lv_obj_t lv_obj_t;

namespace core {
struct Snapshot;
}

namespace ui::home {

void create(lv_obj_t* parent);
void refresh(const core::Snapshot& snapshot, bool show_boot);

#ifdef UI_SIM
// Sélection déterministe des vues pour les captures hôte; absent du firmware.
void snapshot_scenario(const char* scenario);
#endif

}  // namespace ui::home

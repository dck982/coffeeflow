// Le cœur machine — LA façade que net_http et ui/ incluent côté machine
// (docs/plan-phase6.md, "L'idée qui structure tout : le cœur machine").
//
// Trois faces, posées ici comme des structures/interfaces vides : c'est le
// contrat que les lots suivants remplissent, pas une implémentation.
//   - Sorties      (lot 3, core/snapshot.h/.cpp)  : un instantané cohérent
//     de tout ce que la machine sait.
//   - Configuration (lot 4, core/config.cpp)       : lecture, écriture
//     partielle validée tout ou rien, valeurs par défaut, bornes.
//   - Actions       (lot 9, core/machine.cpp)      : ce que la machine sait
//     faire, chacune acceptée ou refusée avec un motif.
// Une quatrième face transverse, le flux d'événements (core/events.cpp),
// arrive avec les lots ci-dessus.
//
// Règle de dépendance (docs/plan-phase6.md) : core/core.h est la SEULE chose
// que net_http et ui/ incluent du côté machine. Un fichier d'UI ou de HTTP
// qui inclut can_link.h franchit la frontière.
//
// Aucune logique n'est implémentée ici pour les faces sorties/configuration/
// actions — voir les lots 3, 4 et 9. La face événements (transverse) est en
// revanche déjà réelle depuis le lot 2 : voir core/events.h.
#pragma once

#include "core/events.h"

namespace core {

// --- Sorties (lot 3) ---------------------------------------------------
// Instantané étendu de ui_model_t (ui.md), pris sous verrou. Champ par
// champ, validité et âge : posés au lot 3.
struct Snapshot {};

// --- Configuration (lot 4) ----------------------------------------------
// Lecture de l'intégralité du réglable, écriture partielle validée tout ou
// rien. Schéma et bornes : firmware.md / ui.md.
struct Config {};

// --- Actions (lot 9) -----------------------------------------------------
// Chaque action répond acceptée ou refusée avec un motif. Table complète :
// docs/plan-phase6.md, section "Actions".
struct ActionResult {};

}  // namespace core

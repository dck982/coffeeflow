#pragma once

#include <cstdint>

namespace core {

// Échéancier d'un échantillonnage périodique appelé depuis une boucle plus
// rapide dont le réveil est quantifié (tick FreeRTOS) et dont l'heure de
// lecture varie avec le travail qui précède. Les échéances restent sur la
// grille de départ : poser « maintenant + période » ferait glisser chaque
// échantillon manqué de quelques microsecondes d'un tour de boucle entier.
struct SampleSchedule {
  int64_t period_us;
  // Avance tolérée sur l'échéance, un demi-tour de boucle : l'échantillon est
  // pris au tour le plus proche de l'échéance plutôt qu'au premier qui la suit.
  int64_t early_us;

  bool due(int64_t now_us, int64_t deadline_us) const { return now_us >= deadline_us - early_us; }

  // Échéance suivante, une période après la précédente. Si elle serait déjà
  // due (retard d'au moins une période moins l'avance), la grille repart de
  // maintenant plutôt que d'enchaîner des échantillons en rafale.
  int64_t next(int64_t now_us, int64_t deadline_us) const {
    const int64_t following = deadline_us + period_us;
    return due(now_us, following) ? now_us + period_us : following;
  }
};

}  // namespace core

#include <cassert>
#include <cstdint>
#include "core/sample_schedule.h"

namespace {

constexpr core::SampleSchedule kSchedule{100000, 25000};

// Boucle de 50 ms réveillée sur la grille des ticks ; l'heure lue varie de
// 0 à 3 ms selon le travail qui précède, comme dans la tâche de télémétrie.
int64_t loop_now_us(int pass) { return pass * 50000LL + (pass * 7919 % 4) * 1000; }

}  // namespace

int main() {
  // Tours pairs d'un échantillon à l'autre, même quand le travail raccourcit :
  // l'ancienne échéance « now + 100 ms » glissait alors d'un tour (150 ms).
  int64_t deadline = loop_now_us(0);
  int64_t previous = -1;
  int samples = 0;
  for (int pass = 0; pass < 400; ++pass) {
    const int64_t now = loop_now_us(pass);
    if (!kSchedule.due(now, deadline)) continue;
    deadline = kSchedule.next(now, deadline);
    if (previous >= 0) {
      const int64_t interval = now - previous;
      assert(interval >= 97000 && interval <= 103000);
    }
    previous = now;
    ++samples;
  }
  assert(samples == 200);

  // Une échéance lue juste avant son heure est prise dans ce tour.
  assert(kSchedule.due(99990, 100000));
  assert(!kSchedule.due(74000, 100000));
  assert(kSchedule.next(99990, 100000) == 200000);

  // Un tour de boucle en retard garde la grille : la moyenne reste à 100 ms.
  assert(kSchedule.next(150000, 100000) == 200000);

  // Au-delà d'une période de retard, la grille repart de maintenant au lieu
  // de rattraper en rafale.
  assert(kSchedule.next(180000, 100000) == 280000);
  assert(kSchedule.next(420000, 100000) == 520000);
  return 0;
}

// Politique de scan BLE partagée par la balance et la sonde SCACE. Sans
// dépendance NimBLE, pour être testée sur l'hôte.
#pragma once

#include <cstdint>

namespace ble_scan_policy {

// La sonde reste des mois au tiroir : tant que la balance est connectée, la
// chercher ne doit pas occuper la radio en continu. Un scan d'une seconde
// toutes les dix la trouve en moins de dix secondes après son allumage.
inline constexpr int64_t kProbeScanWindowUs = 1000 * 1000;
inline constexpr int64_t kProbeScanPeriodUs = 10 * 1000 * 1000;

enum class Scan : uint8_t { kOff, kContinuous, kProbeWindow };

struct Inputs {
  bool scale_wanted = false;   // balance ni connectée ni en cours de connexion
  bool probe_wanted = false;   // scace.enabled et sonde ni connectée ni en cours de connexion
  bool connecting = false;     // une connexion est demandée : NimBLE n'en accepte qu'une à la fois
  bool cycle_active = false;   // infusion ou purge en cours
};

// La balance garde le scan continu d'avant la sonde : elle conditionne
// l'arrêt au poids. Sa recherche trouve aussi la sonde, si elle est voulue.
// Pendant un cycle, la sonde seule ne justifie aucun scan : elle doit être
// connectée avant l'infusion.
constexpr Scan decide(const Inputs& in) {
  if (in.connecting) return Scan::kOff;
  if (in.scale_wanted) return Scan::kContinuous;
  if (in.probe_wanted && !in.cycle_active) return Scan::kProbeWindow;
  return Scan::kOff;
}

// Une fenêtre de sonde part au plus tôt une période après le début de la
// précédente. 0 signifie qu'aucune fenêtre n'a encore été ouverte.
constexpr bool probe_window_due(int64_t now_us, int64_t last_window_start_us) {
  return last_window_start_us == 0 || now_us - last_window_start_us >= kProbeScanPeriodUs;
}

}  // namespace ble_scan_policy

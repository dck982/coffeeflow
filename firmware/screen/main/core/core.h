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
// La face sorties est réelle depuis le lot 3. Configuration et actions restent
// des emplacements réservés pour les lots 4 et 9.
#pragma once

#include <cstdint>

#include "core/events.h"

namespace core {

// --- Sorties (lot 3) ---------------------------------------------------
enum class Freshness : uint8_t { kFresh, kStale, kMissing };

// Instantané étendu de ui_model_t (ui.md), pris sous verrou puis complété sur
// une copie. Aucun consommateur ne lit l'état interne champ par champ.
struct Snapshot {
  float pressure_bar = 0.0f;
  float temperature_c = 0.0f;
  float flow_ml_s = 0.0f;
  float volume_ml = 0.0f;
  uint32_t flow_pulse_count = 0;
  uint32_t flow_last_edge_age_ms = 0;

  bool sensors_alive = false;
  bool pressure_valid = false;
  bool flow_valid = false;
  bool valve_open = false;
  uint8_t dimmer_pct = 0;
  bool dimmer_ready = false;
  bool dimmer_valid = false;
  bool dimmer_error_active = false;
  bool lockout = false;
  uint16_t lease_remaining_ms = 0;
  uint16_t continuous_on_ms = 0;

  Freshness pressure_freshness = Freshness::kMissing;
  Freshness flow_freshness = Freshness::kMissing;
  Freshness actuators_freshness = Freshness::kMissing;
  uint32_t pressure_age_ms = 0;
  uint32_t flow_age_ms = 0;
  uint32_t actuators_age_ms = 0;

  uint8_t sensors_version_major = 0;
  uint8_t sensors_version_minor = 0;
  uint8_t sensors_version_patch = 0;
  uint32_t sensors_uptime_s = 0;
  uint16_t sensors_twai_rx_errors = 0;
  uint16_t sensors_twai_tx_errors = 0;
  uint32_t sensors_twai_bus_errors = 0;
};

enum class TelemetryProfile : uint8_t { kIdle, kActive, kSuspended };

void init();
void start_telemetry_task();
void set_telemetry_profile(TelemetryProfile profile);
Snapshot get_snapshot();

// Adaptateurs de protocole, appelés exclusivement par can_link après que la
// trame a été attribuée au nœud sensors.
void on_status_pressure(const uint8_t* data, uint8_t len);
void on_status_flow(const uint8_t* data, uint8_t len);
void on_status_actuators(const uint8_t* data, uint8_t len);
void on_pong(const uint8_t* data, uint8_t len);
void on_log(const uint8_t* data, uint8_t len);

// --- Configuration (lot 4) ----------------------------------------------
// Lecture de l'intégralité du réglable, écriture partielle validée tout ou
// rien. Schéma et bornes : firmware.md / ui.md.
struct Config {};

// --- Actions (lot 9) -----------------------------------------------------
// Chaque action répond acceptée ou refusée avec un motif. Table complète :
// docs/plan-phase6.md, section "Actions".
struct ActionResult {};

}  // namespace core

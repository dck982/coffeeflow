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
// Faces réelles : sorties (lot 3), configuration (lot 4), actions de banc
// (lot 5, commande brute). Infusion et purge restent le lot 9.
#pragma once

#include <cstdint>

#include "core/config.h"
#include "core/events.h"

namespace core {

// --- Sorties (lot 3) ---------------------------------------------------
enum class Freshness : uint8_t { kFresh, kStale, kMissing };
enum class FlashTarget : uint8_t { kNone, kScreen, kSensors };
enum class RadioMode : uint8_t { kOff, kMachine, kWifi };
enum class CycleState : uint8_t { kIdle, kPreinfusion, kBrew, kRampdown, kFinished, kPurge };

// Instantané étendu de ui_model_t (ui.md), pris sous verrou puis complété sur
// une copie. Aucun consommateur ne lit l'état interne champ par champ.
struct Snapshot {
  float pressure_bar = 0.0f;
  float temperature_c = 0.0f;
  float flow_ml_s = 0.0f;
  float volume_ml = 0.0f;
  uint32_t flow_pulse_count = 0;
  uint32_t flow_last_edge_age_ms = 0;

  float weight_g = 0.0f;
  bool scale_connected = false;
  bool scale_present = false;
  uint32_t scale_age_ms = 0;

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

  uint8_t network_state = 0;  // NetworkState, évite une dépendance d'ordre dans Snapshot
  uint32_t ipv4_address = 0;  // ordre réseau, 0 = aucune adresse
  RadioMode radio_mode = RadioMode::kOff;
  bool radio_transition = false;
  uint32_t internal_heap_free = 0;
  uint32_t internal_heap_largest = 0;
  uint32_t internal_heap_minimum = 0;
  bool time_known = false;
  int64_t wall_time_unix_s = 0;

  uint32_t pressure_raw = 0;
  uint16_t temperature_raw = 0;

  uint8_t screen_version_major = 0;
  uint8_t screen_version_minor = 0;
  uint8_t screen_version_patch = 0;
  uint32_t screen_uptime_s = 0;
  bool flash_active = false;
  FlashTarget flash_target = FlashTarget::kNone;
  uint32_t flash_bytes_done = 0;
  uint32_t flash_bytes_total = 0;
  CycleState cycle_state = CycleState::kIdle;
  uint32_t cycle_elapsed_ms = 0;
  uint32_t cycle_phase_elapsed_ms = 0;
  float cycle_start_weight_g = 0.0f;
  bool cycle_weight_goal = false;
  bool last_shot_available = false;
  float last_shot_weight_g = 0.0f;
  uint32_t last_shot_duration_ms = 0;
  float last_shot_flow_ml_s = 0.0f;
  int64_t last_shot_unix_s = 0;
};

enum class TelemetryProfile : uint8_t { kIdle, kActive, kSuspended };
enum class NetworkState : uint8_t { kOff, kApProvisioning, kStaConnecting, kStaConnected, kStaDisconnected };

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

// BLE Acaia, appelé exclusivement par ble_scale. La définition de présence
// reste au coeur : connectée et une pesée reçue depuis moins de deux secondes.
void update_scale_connection(bool connected);
void update_scale_weight(float weight_g);

void update_network_status(NetworkState state, uint32_t ipv4_address);
void mark_wall_time_known(int64_t unix_s);

// Transition radio asynchrone, toujours effectuée sur le cœur 0. Au bring-up,
// le défaut kOff permet de mesurer la SRAM avant de charger une radio.
bool request_radio_mode(RadioMode mode);
RadioMode radio_mode();

using ForgetNetworkCallback = void (*)();
void register_forget_network_callback(ForgetNetworkCallback callback);
void forget_network();

ConfigResult put_config(const Config& candidate);

void note_http_auth_refused();
void note_config_rejected(const char* field);

// Commande de banc strictement bornée : l'écran de service est client, seul
// le cœur émet SET. La machine d'infusion complète reste le lot 9.
enum class DiagnosticStatus : uint8_t { kOk, kBusLost, kDimmerNotReady, kLocked };
DiagnosticStatus set_diagnostic_purge(bool enabled, uint8_t pump_pct);

enum class Action : uint8_t {
  kSetActuators,
  kStartBrew,
  kStopBrew,
  kPurgePress,
  kPurgeRelease,
  kTare,
  kDismissSummary,
  kStartFlash,
};

enum class ActionStatus : uint8_t {
  kOk,
  kUnavailable,
  kBusLost,
  kLocked,
  kDimmerNotReady,
  kCycleActive,
  kNoCycle,
  kInvalidValue,
};

struct ActionCommand {
  Action action = Action::kSetActuators;
  bool ssr = false;
  uint8_t dimmer = 0;
  uint16_t ttl_ms = 0;
};

struct ActionResult {
  ActionStatus status = ActionStatus::kOk;
  const char* reason = "ok";
};

ActionResult perform_action(const ActionCommand& command);

bool begin_flash(FlashTarget target, uint32_t total);
void update_flash_progress(uint32_t done);
void finish_flash();

}  // namespace core

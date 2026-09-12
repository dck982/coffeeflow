#include "core/core.h"

int64_t g_sim_time_us = 0;

namespace core {
namespace {
Config g_config{};
RadioMode g_radio_mode = RadioMode::kMachine;
}

Config get_config() { return g_config; }
ConfigResult put_config(const Config& candidate) { g_config = candidate; ++g_config.revision; return {}; }
Snapshot get_snapshot() { return {}; }
bool request_radio_mode(RadioMode mode) { g_radio_mode = mode; return true; }
RadioMode radio_mode() { return g_radio_mode; }
void forget_network() {}
ActionResult perform_action(const ActionCommand&) { return {}; }
}  // namespace core

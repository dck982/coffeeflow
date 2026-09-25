#include "core/core.h"
#include "ota_local.h"
#include "service_screen.h"

int64_t g_sim_time_us = 0;

namespace core {
namespace {
Config g_config{};
RadioMode g_radio_mode = RadioMode::kMachine;
}

Config get_config() { return g_config; }
ConfigResult put_config(const Config& candidate) { g_config = candidate; ++g_config.revision; return {}; }
Snapshot get_snapshot() { return {}; }
HFCaptureInfo get_hf_capture_info() { return {}; }
bool get_hf_capture_sample(uint16_t, HFSample *) { return false; }
bool request_radio_mode(RadioMode mode) { g_radio_mode = mode; return true; }
RadioMode radio_mode() { return g_radio_mode; }
void forget_network() {}
ActionResult perform_action(const ActionCommand&) { return {}; }
}  // namespace core

namespace service_screen {
bool restart_lcd() { return true; }
}  // namespace service_screen

namespace ota_local {
bool pending_verify() { return false; }
}  // namespace ota_local

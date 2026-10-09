#include <cstdio>
#include <cstdlib>

#include "ble_scan_policy.h"

namespace {

int g_failures = 0;

#define CHECK(condition)                                                   \
  do {                                                                     \
    if (!(condition)) {                                                    \
      std::fprintf(stderr, "%s:%d: CHECK(%s)\n", __FILE__, __LINE__, #condition); \
      ++g_failures;                                                        \
    }                                                                      \
  } while (0)

using ble_scan_policy::Inputs;
using ble_scan_policy::Scan;
using ble_scan_policy::decide;
using ble_scan_policy::probe_window_due;

void test_nothing_wanted_stops_scan() {
  CHECK(decide(Inputs{}) == Scan::kOff);
}

void test_scale_keeps_continuous_scan() {
  Inputs in;
  in.scale_wanted = true;
  CHECK(decide(in) == Scan::kContinuous);
  in.probe_wanted = true;
  CHECK(decide(in) == Scan::kContinuous);
  in.cycle_active = true;
  CHECK(decide(in) == Scan::kContinuous);
}

void test_probe_alone_uses_windows() {
  Inputs in;
  in.probe_wanted = true;
  CHECK(decide(in) == Scan::kProbeWindow);
}

void test_probe_alone_never_scans_during_cycle() {
  Inputs in;
  in.probe_wanted = true;
  in.cycle_active = true;
  CHECK(decide(in) == Scan::kOff);
}

void test_connection_attempt_stops_scan() {
  Inputs in;
  in.scale_wanted = true;
  in.probe_wanted = true;
  in.connecting = true;
  CHECK(decide(in) == Scan::kOff);
}

void test_probe_window_period() {
  constexpr int64_t start = 5'000'000;
  CHECK(probe_window_due(start, 0));
  CHECK(!probe_window_due(start + ble_scan_policy::kProbeScanPeriodUs - 1, start));
  CHECK(probe_window_due(start + ble_scan_policy::kProbeScanPeriodUs, start));
}

}  // namespace

int main() {
  test_nothing_wanted_stops_scan();
  test_scale_keeps_continuous_scan();
  test_probe_alone_uses_windows();
  test_probe_alone_never_scans_during_cycle();
  test_connection_attempt_stops_scan();
  test_probe_window_period();
  if (g_failures != 0) {
    std::fprintf(stderr, "%d ble scan policy check(s) failed\n", g_failures);
    return EXIT_FAILURE;
  }
  std::puts("ble scan policy tests passed");
  return EXIT_SUCCESS;
}

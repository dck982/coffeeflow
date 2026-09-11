#include <cassert>
#include <cstdio>

#include "core/machine.h"

using core::machine::Config;
using core::machine::Input;
using core::machine::Machine;
using core::machine::PreinfusionMode;
using core::machine::RampdownMode;
using core::machine::State;
using core::machine::StopReason;

Config config() {
  return {36, 28, PreinfusionMode::kTime, 6, 4, 30, RampdownMode::kNone,
          3, 4, 1, 100, 100, 20};
}

int main() {
  Machine machine;
  Input input{0, false, 0};
  Config c = config();
  assert(machine.start(1000, c, input));
  assert(machine.state() == State::kPreinfusion);
  assert(machine.tick(6999, input).dimmer == 30);
  assert(machine.tick(7000, input).dimmer == 100);
  assert(machine.state() == State::kBrew);
  assert(machine.tick(28999, input).ssr);
  assert(!machine.tick(29000, input).ssr);
  assert(machine.stop_reason() == StopReason::kTargetTime);

  Machine weighted;
  input = {10, true, 0};
  assert(weighted.start(1000, c, input));
  assert(weighted.weight_goal());
  input.weight_g = 46;
  assert(!weighted.tick(2000, input).ssr);
  assert(weighted.stop_reason() == StopReason::kTargetWeight);

  Machine ramp_weight;
  c.rampdown_mode = RampdownMode::kWeight;
  c.preinfusion_time_s = 0;
  input = {10, true, 0};
  assert(ramp_weight.start(1000, c, input));
  input.weight_g = 42;
  assert(ramp_weight.tick(2000, input).ssr);
  assert(ramp_weight.state() == State::kRampdown);
  input.weight_g = 46;
  assert(!ramp_weight.tick(2100, input).ssr);
  assert(ramp_weight.stop_reason() == StopReason::kTargetWeight);
  c = config();

  Machine lost;
  input = {10, true, 0};
  assert(lost.start(1000, c, input));
  input.scale_present = false;
  assert(!lost.tick(1100, input).ssr);
  assert(lost.stop_reason() == StopReason::kScaleLost);

  Machine purge;
  assert(purge.purge_press(1000, c));
  assert(purge.tick(2000, input).ssr);
  assert(purge.purge_release(2000));
  assert(!purge.tick(2001, input).ssr);
  assert(purge.stop_reason() == StopReason::kPurgeReleased);
  assert(purge.purge_press(3000, c));
  assert(!purge.tick(23000, input).ssr);
  assert(purge.stop_reason() == StopReason::kPurgeTimeout);

  std::puts("machine tests passed");
}

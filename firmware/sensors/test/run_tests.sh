#!/bin/sh
set -eu
cd "$(dirname "$0")"
for test_source in test_heating_pwm.cpp test_dimmer_zero_guard.cpp; do
  BIN="$(mktemp -t coffeeflow_sensors_tests)"
  c++ -std=c++17 -Wall -Wextra -Werror -I ../main "$test_source" -o "$BIN"
  "$BIN"
  rm -f "$BIN"
done

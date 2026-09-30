#!/bin/sh
set -eu

cd "$(dirname "$0")"
BIN="$(mktemp -t coffeeflow_machine_tests)"
c++ -std=c++17 -Wall -Wextra -Werror -I ../main ../main/core/machine.cpp test_machine.cpp -o "$BIN"
"$BIN"
THERMAL_BIN="$(mktemp -t coffeeflow_thermal_tests)"
c++ -std=c++17 -Wall -Wextra -Werror -I ../main test_thermal.cpp -o "$THERMAL_BIN"
"$THERMAL_BIN"
SCHEDULE_BIN="$(mktemp -t coffeeflow_schedule_tests)"
c++ -std=c++17 -Wall -Wextra -Werror -I ../main test_sample_schedule.cpp -o "$SCHEDULE_BIN"
"$SCHEDULE_BIN"
rm -f "$SCHEDULE_BIN"
rm -f "$THERMAL_BIN"
rm -f "$BIN"

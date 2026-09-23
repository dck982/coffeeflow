#!/bin/sh
set -eu
cd "$(dirname "$0")"
BIN="$(mktemp -t coffeeflow_pwm_tests)"
c++ -std=c++17 -Wall -Wextra -Werror -I ../main test_heating_pwm.cpp -o "$BIN"
"$BIN"
rm -f "$BIN"

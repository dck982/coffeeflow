#!/bin/sh
set -eu

cd "$(dirname "$0")"
BIN="$(mktemp -t coffeeflow_machine_tests)"
c++ -std=c++17 -Wall -Wextra -Werror -I ../main ../main/core/machine.cpp test_machine.cpp -o "$BIN"
"$BIN"
rm -f "$BIN"

#!/bin/sh
# Tests hôte de firmware/common/. Aucune dépendance ESP-IDF : tourne sur le
# Mac, sans matériel. Voir docs/firmware-implementation.md, phase 0.
set -eu

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

python3 "$ROOT/codegen/gen_log_codes.py"

BIN="$(mktemp -t common_tests)"
c++ -std=c++17 -Wall -Wextra -Werror \
  -I "$ROOT/include" -I "$ROOT/generated" \
  "$ROOT/src/crc.cpp" test_common.cpp \
  -o "$BIN"
"$BIN"
rm -f "$BIN"

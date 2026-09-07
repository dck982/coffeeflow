#!/bin/sh
# Tests hôte de l'outil Mac (coffeetool). Aucune dépendance matérielle : voir
# docs/firmware-implementation.md, phase 1. Régénère d'abord log_codes.py
# depuis la même source YAML que le firmware, comme common/test/run_tests.sh.
set -eu

cd "$(dirname "$0")"
TOOLS_ROOT="$(cd .. && pwd)"
FIRMWARE_ROOT="$(cd ../.. && pwd)"

python3 "$FIRMWARE_ROOT/common/codegen/gen_log_codes.py"

PYTHONPATH="$TOOLS_ROOT:${PYTHONPATH:-}" python3 -m pytest "$TOOLS_ROOT/test" "$@"

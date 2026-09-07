"""Table de codes LOG — chargée depuis la source unique générée, voir
firmware/common/codegen/log_codes.yaml (docs/firmware.md, message 0x30).
Ne pas dupliquer cette table ici : elle dériverait de celle du firmware.
"""

import pathlib
import sys

_GENERATED_DIR = pathlib.Path(__file__).resolve().parents[2] / "common" / "generated"
if str(_GENERATED_DIR) not in sys.path:
    sys.path.insert(0, str(_GENERATED_DIR))

from log_codes import LOG_CODES, LOG_SEVERITIES  # noqa: E402  (import après sys.path)


def log_code_name(code: int) -> str:
    return LOG_CODES.get(code, f"UNKNOWN({code})")


def log_severity_name(severity: int) -> str:
    return LOG_SEVERITIES.get(severity, f"unknown({severity})")

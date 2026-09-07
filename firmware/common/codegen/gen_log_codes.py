#!/usr/bin/env python3
"""Génère log_codes.hpp et log_codes.py depuis log_codes.yaml.

Source unique de la table de codes LOG (docs/firmware.md, message 0x30) :
un seul endroit à éditer, deux consommateurs (C++ embarqué, outil Mac en
Python) qui ne peuvent pas dériver l'un de l'autre.

Parseur volontairement minimal (pas de dépendance à PyYAML) : le format
d'entrée est une liste à plat de mappings à trois clés fixes
(name / severity / comment), pas du YAML général.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent
SOURCE = ROOT / "log_codes.yaml"
OUT_DIR = ROOT.parent / "generated"

SEVERITIES = ["debug", "info", "warn", "error"]


def parse(text: str):
    entries = []
    current = None
    for raw_line in text.splitlines():
        line = raw_line.split("#", 1)[0].rstrip()
        if not line.strip():
            continue
        m = re.match(r"^- name:\s*(\S+)\s*$", line)
        if m:
            if current is not None:
                entries.append(current)
            current = {"name": m.group(1)}
            continue
        m = re.match(r"^\s+(severity|comment):\s*(.+?)\s*$", line)
        if m and current is not None:
            key, value = m.group(1), m.group(2)
            current[key] = value
            continue
        raise ValueError(f"ligne non reconnue dans {SOURCE.name}: {raw_line!r}")
    if current is not None:
        entries.append(current)

    for e in entries:
        if "severity" not in e or "comment" not in e:
            raise ValueError(f"entrée incomplète: {e}")
        if e["severity"] not in SEVERITIES:
            raise ValueError(f"sévérité inconnue pour {e['name']}: {e['severity']}")
    if len(entries) > 256:
        raise ValueError("plus de 256 codes LOG, le champ code est un octet")
    return entries


def render_hpp(entries) -> str:
    lines = [
        "// GÉNÉRÉ — ne pas éditer à la main.",
        "// Source : firmware/common/codegen/log_codes.yaml",
        "// Régénérer : python3 firmware/common/codegen/gen_log_codes.py",
        "#pragma once",
        "",
        "#include <cstdint>",
        "",
        "namespace common {",
        "",
        "enum class LogCode : uint8_t {",
    ]
    for i, e in enumerate(entries):
        lines.append(f"  k{to_camel(e['name'])} = {i},")
    lines.append("};")
    lines.append("")
    lines.append("enum class LogSeverity : uint8_t {")
    for i, s in enumerate(SEVERITIES):
        lines.append(f"  k{s.capitalize()} = {i},")
    lines.append("};")
    lines.append("")
    lines.append("inline const char* log_code_name(LogCode code) {")
    lines.append("  switch (code) {")
    for e in entries:
        lines.append(f"    case LogCode::k{to_camel(e['name'])}: return \"{e['name']}\";")
    lines.append("  }")
    lines.append('  return "UNKNOWN";')
    lines.append("}")
    lines.append("")
    lines.append("}  // namespace common")
    lines.append("")
    return "\n".join(lines)


def render_py(entries) -> str:
    lines = [
        "# GÉNÉRÉ — ne pas éditer à la main.",
        "# Source : firmware/common/codegen/log_codes.yaml",
        "# Régénérer : python3 firmware/common/codegen/gen_log_codes.py",
        "",
        "LOG_CODES = {",
    ]
    for i, e in enumerate(entries):
        lines.append(f"    {i}: {e['name']!r},")
    lines.append("}")
    lines.append("")
    lines.append("LOG_SEVERITIES = {")
    for i, s in enumerate(SEVERITIES):
        lines.append(f"    {i}: {s!r},")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def to_camel(name: str) -> str:
    return "".join(part.capitalize() for part in name.split("_"))


def main():
    entries = parse(SOURCE.read_text(encoding="utf-8"))
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "log_codes.hpp").write_text(render_hpp(entries), encoding="utf-8")
    (OUT_DIR / "log_codes.py").write_text(render_py(entries), encoding="utf-8")
    print(f"{len(entries)} codes LOG générés dans {OUT_DIR}")


if __name__ == "__main__":
    sys.exit(main())

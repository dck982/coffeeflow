#!/usr/bin/env python3
"""List C/C++ string literals in the screen UI containing code points above U+00FF."""

from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[1] / "screen" / "main" / "ui"
TOKEN = re.compile(
    r'//[^\n]*|/\*[\s\S]*?\*/|(?:u8|u|U|L)?"(?:\\.|[^"\\])*"|'
    r"'(?:\\.|[^'\\])*'"
)
STRING = re.compile(r'^(?:u8|u|U|L)?"')


def main() -> int:
    found = False
    for path in sorted((*ROOT.rglob("*.cpp"), *ROOT.rglob("*.h"))):
        if "ui_fonts" in path.parts:
            continue
        source = path.read_text(encoding="utf-8")
        for token in TOKEN.finditer(source):
            value = token.group()
            if not STRING.match(value):
                continue
            unusual = sorted({ch for ch in value if ord(ch) > 255})
            if unusual:
                line = source.count("\n", 0, token.start()) + 1
                chars = " ".join(f"{ch!r} (U+{ord(ch):04X})" for ch in unusual)
                print(f"{path.relative_to(ROOT.parent.parent.parent)}:{line}: {chars}: {value}")
                found = True
    if not found:
        print("Aucune chaîne UI contenant un caractère supérieur à U+00FF.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

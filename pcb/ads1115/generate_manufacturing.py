#!/usr/bin/env python3
"""Generate JLCPCB files from the saved board. Requires KiCad 10's kicad-cli.

Run: python3 generate_manufacturing.py
No Python dependencies. Does not modify the source PCB or schematic.
"""
import argparse
import csv
import hashlib
import json
import math
import re
import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BOARD = ROOT / "screen_sensors_draft.kicad_pcb"
# The PCB's LED reference is POWER; the schematic/source BOM still calls it D1.
REFERENCE_ALIASES = {"D1": "POWER"}
# JLCPCB model correction, relative to the saved KiCad footprint rotation.
ROTATION_OFFSETS = {"U1": 270.0}
README = """# JLCPCB manufacturing files

Generated from the saved screen_sensors_draft.kicad_pcb.
Regenerate with `python3 generate_manufacturing.py` after saving design changes.
See export_manifest.json for source hashes, board dimensions and placement rules.

Upload screen_sensors_jlcpcb.zip (fabrication + BOM + placement). If the PCB
uploader does not process the assembly files, upload bom.csv and positions.csv
separately at the assembly step. screen_sensors_gerbers.zip is fabrication only.
Choose top-side assembly, including through-hole assembly for J1, J2 and J3.
Suggested board options: two layers, 1.6 mm FR-4, 1 oz copper, green/white.

JST XH connector centres use pin-row midpoints, not footprint pin-one origins
or housing centres. U1 has a 270-degree JLCPCB rotation offset. Coordinates
are absolute millimetres with negative Y, matching the fabrication outputs.
Verify all placement/polarity in the JLCPCB preview after every upload.

J1 pin order is 3V3, GND, SCL, SDA; cross SDA/SCL in the custom cable.
R1 is 0.1%; R3 and R4 are basic 1% parts. Verify matched parts and stock.
SHT40 U2 requires no board washing; keep its sensing opening uncontaminated.
The LED is POWER on the PCB but D1 in the schematic; the exporter maps this
reference explicitly. Its two known schematic-parity warnings are retained in
drc.json. Other parity differences or physical DRC findings stop the export.
"""


def parse_sexpr(source):
    """Parse KiCad's nested S-expressions without third-party dependencies."""
    tokens = re.findall(r'"(?:\\.|[^"\\])*"|[()]|[^\s()]+', source)
    stack = [[]]
    for token in tokens:
        if token == "(":
            item = []
            stack[-1].append(item)
            stack.append(item)
        elif token == ")":
            if len(stack) == 1:
                raise ValueError("Unbalanced PCB expression")
            stack.pop()
        else:
            stack[-1].append(json.loads(token) if token.startswith('"') else token)
    if len(stack) != 1 or len(stack[0]) != 1:
        raise ValueError("Invalid PCB expression")
    return stack[0][0]


def children(node, name):
    return [item for item in node if isinstance(item, list) and item and item[0] == name]


def first(node, name):
    return children(node, name)[0]


def write_csv(path, header, rows):
    with path.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.writer(stream)
        writer.writerow(header)
        writer.writerows(rows)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kicad-cli", help="Path to KiCad's kicad-cli executable")
    parser.add_argument("--output", type=Path, default=ROOT / "manufacturing")
    args = parser.parse_args()
    cli = args.kicad_cli or shutil.which("kicad-cli")
    if not cli:
        mac_cli = Path("/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli")
        cli = str(mac_cli) if mac_cli.is_file() else None
    if not cli:
        parser.error("Install KiCad 10 or supply --kicad-cli")

    def run(*arguments):
        subprocess.run([cli, *map(str, arguments)], check=True, cwd=ROOT)

    tree = parse_sexpr(BOARD.read_text())
    footprints = {}
    for fp in children(tree, "footprint"):
        properties = {p[1]: p[2] for p in children(fp, "property")}
        ref = properties["Reference"]
        if ref in footprints:
            raise ValueError(f"Duplicate PCB reference: {ref}")
        footprints[ref] = (fp, properties)
    with (ROOT / "screen_sensors_draft_bom.csv").open(newline="") as stream:
        source_rows = list(csv.DictReader(stream))
    bom, positions, fitted = [], [], set()
    for row in source_rows:
        if row["DNP"].lower() == "true":
            continue
        ref = REFERENCE_ALIASES.get(row["Reference"], row["Reference"])
        if ref in fitted:
            raise ValueError(f"Duplicate BOM reference: {ref}")
        fp, props = footprints[ref]
        if first(fp, "layer")[1] != "F.Cu":
            raise ValueError(f"{ref}: exporter expects top-side assembly")
        if children(fp, "dnp") and first(fp, "dnp")[1] == "yes":
            raise ValueError(f"{ref}: PCB marked DNP but source BOM includes it")
        for field, expected in (("Value", row["Value"]), ("LCSC", row["LCSC"])):
            if props.get(field) != expected:
                raise ValueError(f"{ref}: PCB {field} differs from source BOM")
        if fp[1] != row["Footprint"] or not re.fullmatch(r"C\d+", row["LCSC"]):
            raise ValueError(f"{ref}: invalid sourcing or footprint mismatch")
        at = first(fp, "at")
        x, y = map(float, at[1:3])
        angle = float(at[3]) if len(at) > 3 else 0.0
        if fp[1].startswith("Connector_JST:JST_XH_"):
            pads = children(fp, "pad")
            if not pads or any(p[2] != "thru_hole" for p in pads):
                raise ValueError(f"{ref}: unexpected XH pad type")
            pad_positions = [list(map(float, first(p, "at")[1:3])) for p in pads]
            cx = sum(p[0] for p in pad_positions) / len(pad_positions)
            cy = sum(p[1] for p in pad_positions) / len(pad_positions)
            radians = math.radians(angle)
            x += cx * math.cos(radians) + cy * math.sin(radians)
            y += -cx * math.sin(radians) + cy * math.cos(radians)
        rotation = (angle + ROTATION_OFFSETS.get(ref, 0)) % 360
        bom.append([row["Value"], ref, fp[1].split(":", 1)[-1], row["LCSC"]])
        positions.append([ref, f"{x:.6f}", f"{-y:.6f}", f"{rotation:.6f}", "Top"])
        fitted.add(ref)
    # Exclude mounting holes; catch newly added electrical components omitted by the BOM.
    electrical = {ref for ref, (fp, _) in footprints.items()
                  if any(p[1] for p in children(fp, "pad"))
                  and not (children(fp, "dnp") and first(fp, "dnp")[1] == "yes")}
    if fitted != electrical:
        raise ValueError(f"BOM/PCB reference mismatch: {fitted ^ electrical}")
    outline = [s for kind in ("gr_rect", "gr_line") for s in children(tree, kind)
               if first(s, "layer")[1] == "Edge.Cuts"]
    points = [tuple(map(float, first(s, endpoint)[1:3]))
              for s in outline for endpoint in ("start", "end")]
    if len(outline) == 4 and all(s[0] == "gr_line" for s in outline):
        if any(points.count(p) != 2 for p in points) or any(
                points[i][0] != points[i + 1][0] and points[i][1] != points[i + 1][1]
                for i in range(0, len(points), 2)):
            raise ValueError("Expected a closed rectangular outline")
    elif len(outline) != 1 or outline[0][0] != "gr_rect":
        raise ValueError("Expected a rectangular outline; update dimension handling")
    size = [max(p[i] for p in points) - min(p[i] for p in points) for i in (0, 1)]
    source_paths = [BOARD, ROOT / "screen_sensors_draft.kicad_sch", ROOT / "screen_sensors_draft_bom.csv"]
    source_hashes = {p.name: sha(p) for p in source_paths}

    with tempfile.TemporaryDirectory(prefix="coffeeflow-manufacturing-") as temporary:
        stage = Path(temporary)
        gerbers = stage / "gerbers"
        gerbers.mkdir()
        run("pcb", "drc", "--refill-zones", "--schematic-parity", "--severity-all",
            "--format", "json", "--output", stage / "drc.json", BOARD)
        drc = json.loads((stage / "drc.json").read_text())
        if drc["violations"] or drc["unconnected_items"]:
            raise ValueError("Physical DRC findings; fix the PCB before exporting")
        for item in drc["schematic_parity"]:
            known_missing = item["type"] == "missing_footprint" and item["description"] == "Missing footprint D1 (RED / POWER)"
            known_extra = item["type"] == "extra_footprint" and [i["description"] for i in item["items"]] == ["Footprint POWER"]
            if not (known_missing or known_extra):
                raise ValueError(f"Unexpected schematic parity finding: {item['description']}")
        run("pcb", "export", "gerbers", "--layers",
            "F.Cu,B.Cu,F.Mask,B.Mask,F.SilkS,B.SilkS,F.Paste,Edge.Cuts",
            "--check-zones", "--subtract-soldermask", "--output", str(gerbers) + "/", BOARD)
        run("pcb", "export", "drill", "--format", "excellon", "--drill-origin", "absolute",
            "--excellon-zeros-format", "decimal", "--excellon-units", "mm",
            "--excellon-oval-format", "alternate", "--excellon-separate-th",
            "--output", str(gerbers) + "/", BOARD)
        write_csv(stage / "bom.csv", ["Comment", "Designator", "Footprint", "LCSC Part #"], bom)
        write_csv(stage / "positions.csv", ["Designator", "Mid X", "Mid Y", "Rotation", "Layer"], sorted(positions))
        (stage / "README.md").write_text(README)
        if source_hashes != {p.name: sha(p) for p in source_paths}:
            raise ValueError("Source changed during export; save and rerun")
        files = sorted(gerbers.iterdir())
        if len(files) != 11:
            raise ValueError(f"Expected 11 fabrication files, found {len(files)}")
        manifest = {"source_board": BOARD.name, "board_sha256": source_hashes[BOARD.name],
                    "source_sha256": source_hashes, "size_mm": size, "components": len(fitted),
                    "files": {p.name: sha(p) for p in files},
                    "assembly_files": {name: sha(stage / name) for name in ("bom.csv", "positions.csv")},
                    "placement_revision": "JST XH pin-row midpoints; U1 relative rotation offset +270 degrees",
                    "kicad_version": subprocess.check_output([cli, "version"], text=True).strip()}
        (stage / "export_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        for name, extras in (("screen_sensors_gerbers.zip", []),
                             ("screen_sensors_jlcpcb.zip", [stage / n for n in ("bom.csv", "positions.csv", "README.md")])):
            with zipfile.ZipFile(stage / name, "w", zipfile.ZIP_DEFLATED) as archive:
                for path in files + extras:
                    archive.write(path, path.name)
            with zipfile.ZipFile(stage / name) as archive:
                if archive.testzip():
                    raise ValueError(f"Archive integrity check failed: {name}")
        args.output.mkdir(parents=True, exist_ok=True)
        if (args.output / "gerbers").exists():
            shutil.rmtree(args.output / "gerbers")
        shutil.copytree(stage, args.output, dirs_exist_ok=True)
    print(f"Exported {len(fitted)} components, {size[0]:g} × {size[1]:g} mm to {args.output}")


if __name__ == "__main__":
    main()

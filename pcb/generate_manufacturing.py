#!/usr/bin/env python3
"""Generate JLCPCB files from the saved board. Requires KiCad 10's kicad-cli.

Run: uv run generate_manufacturing.py path/to/project
Supports two-layer rectangular boards with top-side assembly.
Requires <board>_bom.csv with Reference, Value, Footprint, DNP, and LCSC columns.
Optional project-local customize-manufacturing.py supplies settings and hooks.
No Python dependencies. Does not modify the source PCB or schematic.
"""
import argparse
import csv
import hashlib
import json
import importlib.util
import re
import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path
from types import SimpleNamespace

README = """# JLCPCB manufacturing files

Generated from the saved {board}.
Regenerate with `uv run ../generate_manufacturing.py .` from the project directory.
See export_manifest.json for source hashes, board dimensions and placement rules.

Upload {name}_jlcpcb.zip (fabrication + BOM + placement). If the PCB
uploader does not process the assembly files, upload bom.csv and positions.csv
separately at the assembly step. {name}_gerbers.zip is fabrication only.
Coordinates are absolute millimetres with negative Y, matching fabrication outputs.
Verify all placement/polarity in the JLCPCB preview after every upload.

{notes}
"""


def load_customization(root):
    """Load optional project-local Python settings and hooks."""
    path = root / "customize-manufacturing.py"
    if not path.is_file():
        return SimpleNamespace()
    spec = importlib.util.spec_from_file_location("manufacturing_customization", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


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
    parser.add_argument("project", type=Path, nargs="?", default=Path.cwd(),
                        help="Project directory containing one PCB, or a .kicad_pcb file (default: cwd)")
    parser.add_argument("--kicad-cli", help="Path to KiCad's kicad-cli executable")
    parser.add_argument("--output", type=Path, help="Output directory (default: project/manufacturing)")
    args = parser.parse_args()
    project = args.project.resolve()
    if project.is_dir():
        boards = sorted(project.glob("*.kicad_pcb"))
        if len(boards) != 1:
            parser.error("Project directory must contain exactly one PCB; pass a .kicad_pcb file explicitly")
        board = boards[0]
    elif project.is_file() and project.suffix == ".kicad_pcb":
        board = project
    else:
        parser.error(f"Not a project directory or PCB file: {project}")
    root, name = board.parent, board.stem
    schematic = board.with_suffix(".kicad_sch")
    source_bom = root / f"{name}_bom.csv"
    for path in (schematic, source_bom):
        if not path.is_file():
            parser.error(f"Required source file missing: {path}")
    args.output = (args.output or root / "manufacturing").resolve()
    # Replacing outputs must never remove the source project or one of its parents.
    if args.output == root or args.output in root.parents or args.output == root / "gerbers":
        parser.error("Output must be separate from the source project directory")
    custom = load_customization(root)
    reference_aliases = getattr(custom, "REFERENCE_ALIASES", {})
    assembly_reference_aliases = getattr(custom, "ASSEMBLY_REFERENCE_ALIASES", {})
    rotation_offsets = getattr(custom, "ROTATION_OFFSETS", {})
    placement_revision = getattr(custom, "PLACEMENT_REVISION", "Saved KiCad footprint origins and rotations")
    customize_placement = getattr(custom, "customize_placement", lambda ref, fp, x, y, angle: (x, y, angle))
    allow_parity = getattr(custom, "allow_schematic_parity", lambda item: False)
    cli = args.kicad_cli or shutil.which("kicad-cli")
    if not cli:
        mac_cli = Path("/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli")
        cli = str(mac_cli) if mac_cli.is_file() else None
    if not cli:
        parser.error("Install KiCad 10 or supply --kicad-cli")

    def run(*arguments):
        subprocess.run([cli, *map(str, arguments)], check=True, cwd=root)

    tree = parse_sexpr(board.read_text())
    copper_layers = [layer for layer in first(tree, "layers")[1:]
                     if isinstance(layer, list) and str(layer[1]).endswith(".Cu")]
    if len(copper_layers) != 2:
        raise ValueError("Exporter currently supports two-layer boards")
    footprints = {}
    for fp in children(tree, "footprint"):
        properties = {p[1]: p[2] for p in children(fp, "property")}
        ref = properties["Reference"]
        if ref in footprints:
            raise ValueError(f"Duplicate PCB reference: {ref}")
        footprints[ref] = (fp, properties)
    with source_bom.open(newline="") as stream:
        source_rows = list(csv.DictReader(stream))
    bom, positions, fitted = {}, [], set()
    assembly_refs = set()
    for row in source_rows:
        if row["DNP"].lower() == "true":
            continue
        ref = reference_aliases.get(row["Reference"], row["Reference"])
        assembly_ref = assembly_reference_aliases.get(ref, ref)
        if not re.fullmatch(r"[A-Za-z0-9_-]+", assembly_ref) or assembly_ref in assembly_refs:
            raise ValueError(f"{ref}: invalid or duplicate assembly reference {assembly_ref!r}")
        assembly_refs.add(assembly_ref)
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
        x, y, angle = customize_placement(ref, fp, x, y, angle)
        rotation = (angle + rotation_offsets.get(ref, 0)) % 360
        code = row["LCSC"]
        footprint_name = fp[1].split(":", 1)[-1]
        if code not in bom:
            bom[code] = [row.get("MPN") or row["Value"], [], footprint_name, code]
        elif bom[code][2] != footprint_name:
            raise ValueError(f"{code}: one sourced part has multiple footprints; review BOM")
        bom[code][1].append(assembly_ref)
        positions.append([assembly_ref, f"{x:.6f}", f"{-y:.6f}", f"{rotation:.6f}", "Top"])
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
    source_paths = [board, schematic, source_bom]
    if (root / "customize-manufacturing.py").is_file():
        source_paths.append(root / "customize-manufacturing.py")
    source_hashes = {p.name: sha(p) for p in source_paths}

    with tempfile.TemporaryDirectory(prefix="coffeeflow-manufacturing-") as temporary:
        stage = Path(temporary)
        gerbers = stage / "gerbers"
        gerbers.mkdir()
        run("pcb", "drc", "--refill-zones", "--schematic-parity", "--severity-all",
            "--format", "json", "--output", stage / "drc.json", board)
        drc = json.loads((stage / "drc.json").read_text())
        if drc["violations"] or drc["unconnected_items"]:
            raise ValueError("Physical DRC findings; fix the PCB before exporting")
        for item in drc["schematic_parity"]:
            if not allow_parity(item):
                raise ValueError(f"Unexpected schematic parity finding: {item['description']}")
        run("pcb", "export", "gerbers", "--layers",
            "F.Cu,B.Cu,F.Mask,B.Mask,F.SilkS,B.SilkS,F.Paste,Edge.Cuts",
            "--check-zones", "--subtract-soldermask", "--output", str(gerbers) + "/", board)
        run("pcb", "export", "drill", "--format", "excellon", "--drill-origin", "absolute",
            "--excellon-zeros-format", "decimal", "--excellon-units", "mm",
            "--excellon-oval-format", "alternate", "--excellon-separate-th",
            "--output", str(gerbers) + "/", board)
        grouped_bom = [[comment, ",".join(refs), footprint, code]
                       for comment, refs, footprint, code in bom.values()]
        write_csv(stage / "bom.csv", ["Comment", "Designator", "Footprint", "LCSC Part #"], grouped_bom)
        write_csv(stage / "positions.csv", ["Designator", "Mid X", "Mid Y", "Rotation", "Layer"], sorted(positions))
        (stage / "README.md").write_text(README.format(board=board.name, name=name,
                                                    notes=getattr(custom, "README_NOTES", "")))
        if source_hashes != {p.name: sha(p) for p in source_paths}:
            raise ValueError("Source changed during export; save and rerun")
        files = sorted(gerbers.iterdir())
        enabled_layers = {layer[1] for layer in first(tree, "layers")[1:]
                          if isinstance(layer, list)}
        gerber_names = {
            "F.Cu": "F_Cu.gtl", "B.Cu": "B_Cu.gbl",
            "F.Mask": "F_Mask.gts", "B.Mask": "B_Mask.gbs",
            "F.SilkS": "F_Silkscreen.gto", "B.SilkS": "B_Silkscreen.gbo",
            "F.Paste": "F_Paste.gtp", "Edge.Cuts": "Edge_Cuts.gm1",
        }
        expected_files = {f"{name}-{suffix}" for layer, suffix in gerber_names.items()
                          if layer in enabled_layers}
        expected_files.update({f"{name}-PTH.drl", f"{name}-NPTH.drl", f"{name}-job.gbrjob"})
        if {path.name for path in files} != expected_files:
            raise ValueError(f"Unexpected fabrication file set: "
                             f"{ {path.name for path in files} ^ expected_files }")
        manifest = {"source_board": board.name, "board_sha256": source_hashes[board.name],
                    "source_sha256": source_hashes, "size_mm": size, "components": len(fitted),
                    "files": {p.name: sha(p) for p in files},
                    "assembly_files": {name: sha(stage / name) for name in ("bom.csv", "positions.csv")},
                    "placement_revision": placement_revision,
                    "kicad_version": subprocess.check_output([cli, "version"], text=True).strip()}
        (stage / "export_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        for name, extras in ((f"{name}_gerbers.zip", []),
                             (f"{name}_jlcpcb.zip", [stage / n for n in ("bom.csv", "positions.csv", "README.md")])):
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

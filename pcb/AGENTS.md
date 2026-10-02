# Datasheet lookup

For component specifications and circuit design decisions, consult the local
`../docs/datasheets/` directory first (the project's `docs/datasheets/` directory).
Search the internet only when the relevant datasheet or information is unavailable
locally; prefer the component manufacturer's documentation.

# KiCad workflow

These directories contain KiCad PCB projects. Keep each project's `.kicad_pro`,
`.kicad_sch`, and `.kicad_pcb` basenames aligned. Update references in text
exports, documentation, and generation scripts when renaming them.

## Command-line exports

Prefer `kicad-cli` for exporting saved designs without opening an editor.
On this Mac it is installed outside PATH at:

```sh
/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli
```

Check `kicad-cli version` and the relevant subcommand's `--help` before assuming
options or capabilities. KiCad 10.0.6 was installed when this workflow was checked.
From `screen_sensors/`, regenerate the schematic PDF with:

```sh
/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli sch export pdf \
  --output screen_sensors.pdf screen_sensors.kicad_sch
```

This exports the saved schematic. Save changes in the editor first if they should
appear in the export. Verify the resulting PDF opens and has the expected pages.
The shared `generate_manufacturing.py` lives in this `pcb/` directory. Run:

```sh
uv run generate_manufacturing.py screen_sensors
# Or, from inside a project:
uv run ../generate_manufacturing.py .
```

Pass a project directory containing exactly one board, or an explicit PCB file.
The matching schematic and `<board>_bom.csv` must exist. Outputs default to the
project's `manufacturing/` directory; `--output` overrides it. The current exporter
supports two-layer rectangular boards with top-side assembly and requires BOM
columns `Reference`, `Value`, `Footprint`, `DNP`, and `LCSC`. Physical DRC findings
and unapproved schematic parity differences stop exports.

An optional project-local `customize-manufacturing.py` is loaded automatically.
It can define `REFERENCE_ALIASES`, `ROTATION_OFFSETS`, `PLACEMENT_REVISION`, and
`README_NOTES`, plus these hooks:

- `customize_placement(ref, footprint, x, y, angle)` returns `(x, y, angle)` in
  KiCad's millimetres/degrees before rotation offsets and Y inversion. The
  footprint argument is its parsed S-expression list.
- `allow_schematic_parity(item)` approves a specific DRC parity JSON item;
  without a hook, all parity findings stop the export.

Keep component-specific conventions in this local file. The screen-sensors
customization adjusts JST XH centres and U1 rotation. Its LED reference is POWER
in both schematic and PCB, so it needs no reference alias or parity exception. These rules must not leak into
other projects. The manifest hashes the customization file alongside design sources.

## Python and IPC API

Use `uv run` with PEP 723 inline dependencies for standalone Python scripts;
avoid installing packages into system Python. For a released IPC client:

```python
# /// script
# dependencies = ["kicad-python"]
# ///
from kipy import KiCad

kicad = KiCad()
print(kicad.get_version())
# Check kicad.check_version() before relying on version-specific API features.
```

Run with `uv run path/to/script.py`. The PyPI name is `kicad-python`, and the
import name is `kipy`. IPC normally needs a running KiCad editor with its API
server enabled in Preferences > Plugins. Check the connected version and target
document before making changes. Use one connection at a time.

The sibling `kicad-python/` clone is a separate library repository and is useful
for reading implementation, examples, and docstrings. Do not assume its current
branch matches the installed KiCad. Its `Schematic` class currently marks
schematic IPC support as requiring KiCad 11; its README also says headless
`kicad-cli api-server` requires KiCad 11. Use CLI schematic exports on KiCad 10.
For using the clone as a dependency, follow its build instructions and initialize
its KiCad submodule; generated protobuf modules and version compatibility matter.

The older `pcbnew` SWIG bindings are for the PCB editor, not schematic PDF
exports. They are deprecated; prefer IPC for new editor automation. `pcbnew`
uses KiCad's bundled native modules and cannot be treated as an ordinary PyPI
package for an arbitrary `uv` Python interpreter.

References:
- https://dev-docs.kicad.org/en/apis-and-binding/ipc-api/index.html
- https://dev-docs.kicad.org/en/apis-and-binding/pcbnew/index.html
- `kicad-python/README.md` and `kicad-python/COMPILING.md`

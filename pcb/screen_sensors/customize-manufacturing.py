"""Screen-sensors assembly conventions used by ../generate_manufacturing.py."""
import math

ROTATION_OFFSETS = {"U1": 270.0}
PLACEMENT_REVISION = "JST XH pin-row midpoints; U1 relative rotation offset +270 degrees"
README_NOTES = """Choose top-side assembly, including through-hole assembly for J1, J2 and J3.
Suggested board options: two layers, 1.6 mm FR-4, 1 oz copper, green/white.

JST XH connector centres use pin-row midpoints, not footprint pin-one origins
or housing centres. U1 has a 270-degree JLCPCB rotation offset. Coordinates
are absolute millimetres with negative Y, matching the fabrication outputs.
Verify all placement/polarity in the JLCPCB preview after every upload.

J1 pin order is 3V3, GND, SCL, SDA; cross SDA/SCL in the custom cable.
R1 is 0.1%; R3 and R4 are basic 1% parts. Verify matched parts and stock.
SHT40 U2 requires no board washing; keep its sensing opening uncontaminated.
The LED reference is POWER in both schematic and PCB.
Schematic parity differences or physical DRC findings stop the export.
"""


def customize_placement(ref, footprint, x, y, angle):
    """Return placement coordinates; angle offsets are applied by the exporter."""
    if not footprint[1].startswith("Connector_JST:JST_XH_"):
        return x, y, angle
    pads = [item for item in footprint if isinstance(item, list) and item[0] == "pad"]
    if not pads or any(p[2] != "thru_hole" for p in pads):
        raise ValueError(f"{ref}: unexpected XH pad type")
    pad_positions = [next(item for item in p if isinstance(item, list) and item[0] == "at")
                     for p in pads]
    cx = sum(float(p[1]) for p in pad_positions) / len(pads)
    cy = sum(float(p[2]) for p in pad_positions) / len(pads)
    radians = math.radians(angle)
    return (x + cx * math.cos(radians) + cy * math.sin(radians),
            y - cx * math.sin(radians) + cy * math.cos(radians), angle)

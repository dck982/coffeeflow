"""Placement corrections for the sensors board's JLCPCB assembly files.

The PCB keeps its schematic references (J1-J12), so the schematic parity check
needs no exception. The XIAO (U1) stays in the schematic only; JLCPCB solders
its two sockets J11/J12.
"""
import math

from generate_manufacturing import children, first

# Match numbered pads in the selected supplier CAD footprints to KiCad.
# Q2/Q3, U6 and J11/J12 were confirmed by the user in the JLCPCB preview. U4,
# U5 and the XH/Grove connectors follow the supplier CAD footprints, as checked
# in the previous preview. D2 needs no correction: the LCSC footprint suggested
# +180, but the preview showed the LED reversed with it.
ROTATION_OFFSETS = {
    'U4': 270.0, 'U5': 180.0, 'U6': 270.0,
    'Q2': 180.0, 'Q3': 180.0,
    'J1': 180.0, 'J3': 180.0, 'J8': 180.0,
    'J4': -90.0, 'J5': -90.0,
    'J11': 270.0, 'J12': 270.0,
}
PLACEMENT_REVISION = (
    'v8: references J1-J12 without aliases; XIAO sockets J11/J12 and U6 at +270 '
    'degrees and Q2/Q3 at +180, confirmed in the JLCPCB preview; D2 without '
    'correction; supplier CAD orientation for U4/U5, C158012 and C722737'
)

# A rotation correction holds only for the part and footprint it was made for.
PLACEMENT_PARTS = {
    'U4': ('C38695', 'Package_SO:SOIC-8_3.9x4.9mm_P1.27mm'),
    'U5': ('C99269', 'Package_TO_SOT_SMD:SOT-563'),
    'U6': ('C37593', 'Package_SO:TSSOP-10_3x3mm_P0.5mm'),
    'Q2': ('C15127', 'Package_TO_SOT_SMD:SOT-23'),
    'Q3': ('C8545', 'Package_TO_SOT_SMD:SOT-23'),
    'D2': ('C2297', 'LED_SMD:LED_0805_2012Metric'),
}

# Through-hole connectors are placed at their pin-row midpoint, the origin of
# the supplier footprints, instead of KiCad's pin-1 origin.
CONNECTOR_PARTS = {
    'J1': 'C158012', 'J3': 'C158012', 'J8': 'C158012',
    'J6': 'C144394', 'J7': 'C144394', 'J10': 'C144394',
    'J4': 'C722737', 'J5': 'C722737',
    'J11': 'C22438157', 'J12': 'C22438157',
}
CONNECTOR_FOOTPRINTS = (
    'Connector_JST:JST_XH_',
    'Sensors_Local:CONN-TH_HY-4A',
    'Sensors_Local:XIAO_Socket_1x07_P2.54mm_Vertical',
)


def customize_placement(ref, footprint, x, y, angle):
    props = {p[1]: p[2] for p in children(footprint, 'property')}
    if ref in PLACEMENT_PARTS:
        if (props.get('LCSC'), footprint[1]) != PLACEMENT_PARTS[ref]:
            raise ValueError(f'{ref}: selected part/footprint changed; review placement correction')
    if not footprint[1].startswith(CONNECTOR_FOOTPRINTS):
        if ref in CONNECTOR_PARTS:
            raise ValueError(f'{ref}: connector footprint changed; review placement correction')
        return x, y, angle
    if props.get('LCSC') != CONNECTOR_PARTS.get(ref):
        raise ValueError(f'{ref}: connector sourcing changed; review placement correction')
    pads = [p for p in children(footprint, 'pad') if p[1]]
    if not pads or any(p[2] != 'thru_hole' for p in pads):
        raise ValueError(f'{ref}: unexpected connector pad type')
    cx = sum(float(first(p, 'at')[1]) for p in pads) / len(pads)
    cy = sum(float(first(p, 'at')[2]) for p in pads) / len(pads)
    radians = math.radians(angle)
    # KiCad's local Y points down; apply the saved board angle before the
    # exporter inverts Y. Supplier rotation offsets must not rotate this shift.
    return (x + cx * math.cos(radians) + cy * math.sin(radians),
            y - cx * math.sin(radians) + cy * math.cos(radians), angle)


README_NOTES = """Select through-hole assembly for J1, J3-J8 and J10-J12.
J11/J12 are the female 1x7 sockets of the XIAO ESP32-S3 (U1), which is not
assembled: it plugs in by hand. Deselect J11/J12 at order time to solder the
sockets yourself.
XH, Grove and socket coordinates use numbered pin-row midpoints, following
the screen_sensors XH convention.
Rotation corrections: U4 +270, U5 +180, U6 +270, Q2 and Q3 +180,
J1/J3/J8 (C158012) +180, J4/J5 (C722737 Grove) -90, J11/J12 (C22438157) +270.
D2 has none. The 2026-10-11 JLCPCB preview confirmed all placements. The Grove
supplier footprint uses 1.0 mm drills.
Other parts retain saved KiCad origins/rotations. Verify placement and
polarity in the JLCPCB preview after every upload.
"""

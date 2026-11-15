"""Placement corrections for the scale board's JLCPCB assembly files.

JLCPCB places each part's supplier (EasyEDA) footprint at the CPL point. The
corrections below turn and shift it onto the KiCad pads of the same numbers.
The XIAO (U2) stays in the schematic only; JLCPCB solders its sockets J3/J4.
"""
import math

from generate_manufacturing import children

# Rotation added to the saved KiCad angle, in degrees.
# U1: the supplier TSSOP-24 has its pins along the top and bottom edges, pin 1
# bottom-left; KiCad's runs them down the sides, pin 1 top-left. Same quarter
# turn as the TSSOP-10 U6 on combined, confirmed in that board's preview.
# J1/J2: the supplier XH-4AW draws the housing on the -Y side of the pin row,
# Scale_Local on the +Y side, as KiCad's S4B-XH-A.
# J3/J4: same socket and footprint as J11/J12 on combined.
ROTATION_OFFSETS = {
    'U1': 270.0,
    'J1': 180.0, 'J2': 180.0,
    'J3': 270.0, 'J4': 270.0,
}

# Supplier footprint origin in KiCad footprint coordinates (mm, Y down):
# the pin-row midpoint for the connectors and sockets.
ORIGIN_SHIFTS = {
    'J1': (3.75, 0.0), 'J2': (3.75, 0.0),
    'J3': (0.0, 7.62), 'J4': (0.0, 7.62),
}

PLACEMENT_REVISION = (
    'v1: U1 and the XH-4AW fitted to the supplier footprints; sockets and LED '
    'as on combined'
)

# A correction holds only for the part and footprint it was computed for.
PLACEMENT_PARTS = {
    'U1': ('C27919', 'Package_SO:TSSOP-24_4.4x7.8mm_P0.65mm'),
    'J1': ('C21273', 'Scale_Local:XH-4AW_1x04_P2.50mm_Horizontal'),
    'J2': ('C21273', 'Scale_Local:XH-4AW_1x04_P2.50mm_Horizontal'),
    'J3': ('C27985232', 'Scale_Local:XIAO_Socket_1x07_P2.54mm_Vertical'),
    'J4': ('C27985232', 'Scale_Local:XIAO_Socket_1x07_P2.54mm_Vertical'),
    # Same LED as D4 on combined: no correction, the preview showed it reversed
    # with the +180 that the fit suggests.
    'D1': ('C2286', 'LED_SMD:LED_0603_1608Metric'),
}


def customize_placement(ref, footprint, x, y, angle):
    props = {p[1]: p[2] for p in children(footprint, 'property')}
    if ref in PLACEMENT_PARTS and (props.get('LCSC'), footprint[1]) != PLACEMENT_PARTS[ref]:
        raise ValueError(f'{ref}: selected part/footprint changed; review placement correction')
    dx, dy = ORIGIN_SHIFTS.get(ref, (0.0, 0.0))
    radians = math.radians(angle)
    # KiCad's local Y points down; apply the saved board angle before the
    # exporter inverts Y. The rotation offset must not rotate this shift.
    return (x + dx * math.cos(radians) + dy * math.sin(radians),
            y - dx * math.sin(radians) + dy * math.cos(radians), angle)


README_NOTES = """Select through-hole assembly for J1-J4.
J3/J4 are the female 1x7 sockets of the XIAO ESP32-S3 (U2), which is not
assembled: it plugs in by hand. H1-H3 are unplated M2 holes.
Rotation corrections: U1 +270; J1, J2 (C21273) +180; J3, J4 +270.
Origin shifts: XH connectors and XIAO sockets at their pin-row midpoint.
Check in the JLCPCB preview: pin 1 of U1, the LED polarity, and that the
J1/J2 housings face away from the board centre (J2 opening north, J1 south).
"""

"""Placement corrections for the scale board's JLCPCB assembly files.

JLCPCB places each part's supplier (EasyEDA) footprint at the CPL point. The
corrections below turn and shift it onto the KiCad pads of the same numbers.
"""
import math

from generate_manufacturing import children

# Rotation added to the saved KiCad angle, in degrees.
# U1: the supplier SOP-16 has its pins along the top and bottom edges, pin 1
# bottom-left; KiCad's SOIC-16 runs them down the sides, pin 1 top-left. Same
# quarter turn as the SOIC-8 U4 on combined, confirmed in that board's preview.
# J1/J2: none. The supplier DB125 footprint has pin 1 on the right, Scale_Local
# on the left, but the wire entry on the same +Y side (with its own 3D model, the
# same STEP as Scale_Local's). The block is symmetric, so the pins land on the
# holes either way; the preview with +180 showed the entries facing U1.
ROTATION_OFFSETS = {
    'U1': 270.0,
}

# Supplier footprint origin in KiCad footprint coordinates (mm, Y down):
# the pin-row midpoint, which is also the body centre.
ORIGIN_SHIFTS = {
    'J1': (3.81, 0.0), 'J2': (3.81, 0.0),
}

PLACEMENT_REVISION = (
    'v2: NAU7802 (SOIC-16) and DB125 terminal blocks (entry checked in preview); LED as on combined'
)

# A correction holds only for the part and footprint it was computed for.
PLACEMENT_PARTS = {
    'U1': ('C2614351', 'Package_SO:SOIC-16_3.9x9.9mm_P1.27mm'),
    'J1': ('C918122', 'Scale_Local:TerminalBlock_Dorabo_DB125-2.54-4P_1x04_P2.54mm_Horizontal'),
    'J2': ('C918122', 'Scale_Local:TerminalBlock_Dorabo_DB125-2.54-4P_1x04_P2.54mm_Horizontal'),
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


README_NOTES = """Select through-hole assembly for J1 and J2 (DB125-2.54-4P terminal blocks).
H1 and H2 are unplated M2 holes.
Rotation corrections: U1 +270; J1, J2 (C918122) none (pin 1 mirrored, symmetric part).
Origin shifts: J1, J2 at their pin-row midpoint.
Check in the JLCPCB preview: pin 1 of U1 (NAU7802), the LED polarity, and
that the J1/J2 wire entries face the board edges, away from U1.
"""

"""Placement corrections for the combined board's JLCPCB assembly files.

JLCPCB places each part's supplier (EasyEDA) footprint at the CPL point. The
corrections below turn and shift it onto the KiCad pads of the same numbers.
They were computed by fitting the supplier pads to the KiCad pads; the method
reproduces every correction confirmed in the sensors board's preview. The
XIAO (U7) stays in the schematic only; JLCPCB solders its sockets J11/J12.
"""
import math

from generate_manufacturing import children

# Rotation added to the saved KiCad angle, in degrees.
# U4, U6, Q5, Q6, the XH connectors and J11/J12 match the sensors board,
# whose preview confirmed them. U5 takes +270, not the sensors +180: the
# supplier SC-89-6 has pins 1-3 along its bottom edge (pin 1 bottom-left),
# KiCad's SOT-563 down its left edge (pin 1 top-left), a quarter turn apart;
# the combined preview showed pin 1 a quarter turn off with +180. LEDs D1, D4 and D5 (KENTO, as the sensors D2)
# take none: the fit suggests +180, but the sensors preview showed the LED
# reversed with it.
ROTATION_OFFSETS = {
    'U1': 270.0,
    'Q2': 270.0,
    'Q1': 180.0, 'Q5': 180.0, 'Q6': 180.0,
    'U4': 270.0, 'U5': 270.0, 'U6': 270.0,
    # C144395 is drawn turned half a turn from C144394: pin 1 on the right and
    # the body deeper on the other side. The four pins land on the holes either
    # way; without this, the housing and pin 1 come out reversed.
    'J1': 180.0, 'J9': 180.0,
    'J11': 270.0, 'J12': 270.0,
}

# Supplier footprint origin in KiCad footprint coordinates (mm, Y down).
# Through-hole connectors use their pin-row midpoint, as on the sensors board.
# The supplier DPAK puts its tab 7.49 mm from the leads instead of 6.3 mm;
# the shift splits that difference, 0.4 mm on the leads and 0.8 mm on the tab.
ORIGIN_SHIFTS = {
    'J10': (2.5, 0.0),
    'J1': (3.75, 0.0), 'J9': (3.75, 0.0),
    'J11': (0.0, 7.62), 'J12': (0.0, 7.62),
    'Q2': (-1.692, 0.0),
}

PLACEMENT_REVISION = (
    'v1: supplier footprints fitted to KiCad pads; sensors corrections for the '
    'low-voltage parts; LEDs without correction'
)

# A correction holds only for the part and footprint it was computed for.
PLACEMENT_PARTS = {
    'U1': ('C41809', 'Combined_Local:SMDIP-6_W9.53mm_Slot'),
    'Q2': ('C23427', 'Package_TO_SOT_SMD:TO-252-2'),
    'Q1': ('C2150', 'Package_TO_SOT_SMD:SOT-23'),
    'Q5': ('C15127', 'Package_TO_SOT_SMD:SOT-23'),
    'Q6': ('C8545', 'Package_TO_SOT_SMD:SOT-23'),
    'U4': ('C38695', 'Package_SO:SOIC-8_3.9x4.9mm_P1.27mm'),
    'U5': ('C99269', 'Package_TO_SOT_SMD:SOT-563'),
    'U6': ('C468683', 'Package_SO:TSSOP-10_3x3mm_P0.5mm'),
    'J10': ('C144394', 'Connector_JST:JST_XH_B3B-XH-A_1x03_P2.50mm_Vertical'),
    # Pin-row midpoint like J10; +180, unlike it (see ROTATION_OFFSETS).
    'J1': ('C144395', 'Connector_JST:JST_XH_B4B-XH-A_1x04_P2.50mm_Vertical'),
    'J9': ('C144395', 'Connector_JST:JST_XH_B4B-XH-A_1x04_P2.50mm_Vertical'),
    'J11': ('C22438157', 'Combined_Local:XIAO_Socket_1x07_P2.54mm_Vertical'),
    'J12': ('C22438157', 'Combined_Local:XIAO_Socket_1x07_P2.54mm_Vertical'),
    # Checked without correction. U8 and U9 have the pads of the former U3
    # (same part, footprint without the slot).
    'U8': ('C2912100', 'Package_DIP:SMDIP-4_W9.53mm'),
    'U9': ('C2912100', 'Package_DIP:SMDIP-4_W9.53mm'),
    'J2': ('C577799', 'Combined_Local:CONN-TH_P3.96_HT396R-3.96-2P'),
    'J3': ('C577799', 'Combined_Local:CONN-TH_P3.96_HT396R-3.96-2P'),
    # LCSC footprint, origin and orientation unchanged, as for J2/J3.
    'J7': ('C577788', 'Combined_Local:CONN-TH_P3.96_HT396V-3.96-2P'),
    'J8': ('C577788', 'Combined_Local:CONN-TH_P3.96_HT396V-3.96-2P'),
    'J6': ('C577789', 'Combined_Local:CONN-TH_P3.96_HT396V-3.96-3P'),
    'D1': ('C2286', 'LED_SMD:LED_0603_1608Metric'),
    'D4': ('C2286', 'LED_SMD:LED_0603_1608Metric'),
    'D5': ('C2286', 'LED_SMD:LED_0603_1608Metric'),
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


README_NOTES = """Select through-hole assembly for J1-J3 and J6-J12.
J11/J12 are the female 1x7 sockets of the XIAO ESP32-S3 (U7), which is not
assembled: it plugs in by hand.
Rotation corrections: U1, Q2 +270; Q1, Q5, Q6 +180;
U4 +270, U5 +270, U6 +270; J1 and J9 (C144395) +180;
J11/J12 +270.
Origin shifts: XH connectors and XIAO sockets at their pin-row midpoint;
Q2 1.69 mm towards the leads. U8, U9, J2, J3, J6, J7, J8 and the LEDs
D1/D4/D5 have none.
U10 and L1 have no computed correction yet. Check
them, pin 1 of U1, U8, U9 and Q2 and the LED polarity in the JLCPCB
preview: these corrections are computed, not yet confirmed there.
"""

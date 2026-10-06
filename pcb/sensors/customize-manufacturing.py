"""Permit the sensors board's intentional reference labels and mounting holes.

Validate electrical connectivity independently because KiCad's parity check
reports renamed footprints as missing/extra and cannot compare their pins.
"""
import json
import math
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path

from generate_manufacturing import children, first, parse_sexpr

# Only upload names are changed; saved PCB references and parity use the labels.
ASSEMBLY_REFERENCE_ALIASES = json.loads(
    (Path(__file__).resolve().parent / 'sensors_sourcing.json').read_text()
)['AssemblyReferenceAliases']

# Match numbered pads in the selected supplier CAD footprints to KiCad.
# C3/Q2/Q3/D1 corrections were confirmed by the user in the JLCPCB preview.
ROTATION_OFFSETS = {
    'U1': 0.0, 'U2': 180.0, 'U4': 270.0, 'U5': 180.0,
    'C3': 180.0, '5V': 180.0, 'CAN': 180.0, 'BOILER': 180.0,
    'Q2': 180.0, 'Q3': 180.0,
    'D1': -90.0,
    'I2C A': -90.0, 'I2C B': -90.0,
}
PLACEMENT_REVISION = (
    'v6: dual I2C A/B Grove pin-row midpoints; C722737 -90 degrees from supplier pads; '
    'supplier CAD orientation for U1/U2/U4/U5 and C158012; '
    'C3/Q2/Q3 +180 and D1 -90 degrees confirmed in JLCPCB preview'
)

PLACEMENT_PARTS = {
    'U1': ('C2980299', 'RF_Module:ESP32-S2-MINI-1U'),
    'U2': ('C6186', 'Package_TO_SOT_SMD:SOT-223-3_TabPin2'),
    'U4': ('C38695', 'Package_SO:SOIC-8_3.9x4.9mm_P1.27mm'),
    'U5': ('C99269', 'Package_TO_SOT_SMD:SOT-563'),
    'C3': ('C7198', 'Capacitor_Tantalum_SMD:CP_EIA-3528-21_Kemet-B'),
    'Q2': ('C15127', 'Package_TO_SOT_SMD:SOT-23'),
    'Q3': ('C8545', 'Package_TO_SOT_SMD:SOT-23'),
    'D1': ('C1855726', 'Package_TO_SOT_SMD:SOT-323_SC-70'),
}


def customize_placement(ref, footprint, x, y, angle):
    props = {p[1]: p[2] for p in children(footprint, 'property')}
    if ref in PLACEMENT_PARTS:
        if (props.get('LCSC'), footprint[1]) != PLACEMENT_PARTS[ref]:
            raise ValueError(f'{ref}: selected part/footprint changed; review placement correction')
    is_xh = footprint[1].startswith('Connector_JST:JST_XH_')
    is_grove = footprint[1] == 'Sensors_Local:CONN-TH_HY-4A'
    if not (is_xh or is_grove):
        return x, y, angle
    expected_codes = {'5V': 'C158012', 'CAN': 'C158012', 'BOILER': 'C158012',
                      'FLOW': 'C144394', 'VALVE': 'C144394',
                      'I2C A': 'C722737', 'I2C B': 'C722737'}
    if props.get('LCSC') != expected_codes.get(ref):
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

# Source BOM already uses PCB references; do not apply aliases to its rows.
SCHEMATIC_LABELS = {
    'J1': '5V', 'J3': 'CAN', 'J4': 'I2C B', 'J5': 'I2C A', 'J6': 'FLOW',
    'J7': 'VALVE', 'J8': 'BOILER', 'SW1': 'BOOT', 'SW2': 'RESET',
    'D2': 'STATUS',
}
MOUNTING_HOLES = {
    'MH1': '5f3a6be9-fd01-4b0a-8dbf-d7bd29fae49f',
    'MH2': '75a41b75-de31-4c40-9fb4-07345aa75279',
    'MH3': 'eeb76f1a-84e7-46ff-bcca-6126c934556c',
    'MH4': '2ae7bc33-c01a-4580-a4e8-e975873ec087',
}
README_NOTES = """PCB references intentionally use connector/function labels:
J1=5V, J3=CAN, J4=I2C B, J5=I2C A, J6=FLOW, J7=VALVE, J8=BOILER,
SW1=BOOT, SW2=RESET, D2=STATUS. Upload names I2CA/I2CB correspond to
PCB labels I2C A/I2C B; spaces are omitted consistently in BOM and placement.
MH1-MH4 are mechanical holes excluded from assembly.
Electrical net membership is checked independently of these reference aliases.
Select through-hole assembly for 5V, CAN, I2C A, I2C B, FLOW, VALVE and BOILER.
XH and Grove connector coordinates use numbered pin-row midpoints, following
the screen_sensors XH convention. U2 has +180, U4 +270, U5 +180 degrees;
U1 needs no correction. C3, Q2 and Q3 have user-confirmed +180-degree corrections.
D1 has the user-confirmed -90-degree correction.
C158012 two-pin connectors have +180 degrees to match supplier pad numbering.
C722737 Grove I2C A/B have -90 degrees to match supplier pad numbering.
Their supplier footprint uses 1.0 mm drills.
Other parts retain saved KiCad origins/rotations. Verify the new file in the
JLCPCB preview.
"""


def _approved_parity():
    root = Path(__file__).resolve().parent
    board = parse_sexpr((root / 'sensors.kicad_pcb').read_text())
    symbols = {}
    for path in root.glob('*.kicad_sch'):
        for symbol in children(parse_sexpr(path.read_text()), 'symbol'):
            props = {p[1]: p[2] for p in children(symbol, 'property')}
            symbols[props['Reference']] = (first(symbol, 'uuid')[1], props)
    footprints = {next(p[2] for p in children(fp, 'property') if p[1] == 'Reference'): fp
                  for fp in children(board, 'footprint')}
    missing, extra = set(), set()
    for sch_ref, pcb_ref in SCHEMATIC_LABELS.items():
        fp = footprints[pcb_ref]
        uid, symbol = symbols[sch_ref]
        props = {p[1]: p[2] for p in children(fp, 'property')}
        if (first(fp, 'path')[1].split('/')[-1] != uid
                or fp[1] != symbol['Footprint']
                or any(props.get(k) != symbol.get(k) for k in ('Value', 'LCSC', 'MPN'))):
            raise ValueError(f'{pcb_ref}: renamed footprint no longer matches {sch_ref}')
        missing.add(f'Missing footprint {sch_ref} ({symbol["Value"]})')
        extra.add((f'Footprint {pcb_ref}', first(fp, 'uuid')[1]))
    for ref, uid in MOUNTING_HOLES.items():
        fp = footprints[ref]
        if (first(fp, 'uuid')[1] != uid
                or not fp[1].startswith('MountingHole:')
                or any(p[1] or children(p, 'net') for p in children(fp, 'pad'))):
            raise ValueError(f'{ref}: expected mechanical mounting hole')
        extra.add((f'Footprint {ref}', uid))

    cli = '/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli'
    with tempfile.TemporaryDirectory(prefix='sensors-net-parity-') as temporary:
        netlist = Path(temporary) / 'netlist.xml'
        subprocess.run([cli, 'sch', 'export', 'netlist', '--format', 'kicadxml',
                        '--output', str(netlist), str(root / 'sensors.kicad_sch')], check=True)
        schematic_nets = {
            frozenset((SCHEMATIC_LABELS.get(n.attrib['ref'], n.attrib['ref']), n.attrib['pin'])
                      for n in net.findall('node'))
            for net in ET.parse(netlist).findall('./nets/net')
        }
    board_nets = defaultdict(set)
    for ref, fp in footprints.items():
        for pad in children(fp, 'pad'):
            if pad[1] and children(pad, 'net'):
                board_nets[first(pad, 'net')[-1]].add((ref, pad[1]))
    # Single-pin nets include intentional no-connects; they have no interconnect.
    expected = {net for net in schematic_nets if len(net) > 1}
    actual = {frozenset(net) for net in board_nets.values() if len(net) > 1}
    if actual != expected:
        raise ValueError('PCB/schematic electrical connectivity differs after reference mapping')
    return missing, extra


_MISSING, _EXTRA = _approved_parity()


def allow_schematic_parity(item):
    if item.get('severity') != 'warning':
        return False
    if item.get('type') == 'missing_footprint':
        return item.get('description') in _MISSING and item.get('items') == []
    if item.get('type') == 'extra_footprint' and item.get('description') == 'Extra footprint':
        details = item.get('items', [])
        return len(details) == 1 and (details[0].get('description'), details[0].get('uuid')) in _EXTRA
    return False

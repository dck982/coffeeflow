# /// script
# dependencies = []
# ///
"""Regenerate the ps_upstream schematic and source BOM; the PCB is not modified.
Run with: uv run generate_design.py
Regeneration overwrites schematic edits.
"""
import copy, csv, math
from pathlib import Path
from sexpr import parse, dump, key, get, node, Sym, effects, uid

HERE = Path(__file__).resolve().parent
KICAD = Path('/Applications/KiCad/KiCad.app/Contents/SharedSupport')
LIBS = {'PS_Upstream': HERE / 'PS_Upstream.kicad_sym'}
FOOTPRINTS = {'PS_Upstream_Local': HERE / 'PS_Upstream_Local.pretty'}
PROJECT = 'ps_upstream'
ROOT = uid('root')

cache = {}
def library(libid):
    lib, name = libid.split(':')
    if lib not in cache:
        path = LIBS.get(lib, KICAD / 'symbols' / (lib + '.kicad_sym'))
        cache[lib] = {a[1]: a for a in parse(path.read_text()) if key(a) == 'symbol'}
    a = copy.deepcopy(cache[lib][name])
    a[1] = libid
    return a

items, libs, bom = [], {}, []
count = 0
def ident():
    global count
    count += 1
    return uid('item/' + str(count))

def label(name, p, angle):
    items.append(node('label', name, node('at', *p, angle),
                      effects(1.27, 'right bottom' if angle == 180 else 'left bottom'),
                      node('uuid', ident())))

def text(t, x, y, size=1.27):
    items.append(node('text', t, node('at', x, y, 0), effects(size, 'left top'), node('uuid', ident())))

def part(libid, ref, value, x, y, nets, footprint, angle=0, lcsc='', mpn='', desc='', note=''):
    a = library(libid)
    libs[libid] = a
    lib, name = footprint.split(':')
    base = FOOTPRINTS.get(lib, KICAD / 'footprints' / (lib + '.pretty'))
    if not (base / (name + '.kicad_mod')).exists():
        raise ValueError('Missing footprint ' + footprint)
    defaults = {p[1]: p[2] for p in a if key(p) == 'property'}
    in_bom = not ref.startswith('H')
    s = node('symbol', node('lib_id', libid), node('at', x, y, angle), node('unit', 1),
             node('exclude_from_sim', Sym('no')), node('in_bom', Sym('yes' if in_bom else 'no')),
             node('on_board', Sym('yes')), node('dnp', Sym('no')), node('uuid', uid(ref)))
    # A rotated two-pin part takes its names above and below its body.
    turned = angle in (90, 270)
    rx, ry, vx, vy = (x, y - 3.81, x, y + 3.81) if turned else (x + 3.81, y - 6.35, x + 3.81, y + 6.35)
    fields = [('Reference', ref, rx, ry, False), ('Value', value, vx, vy, False),
              ('Footprint', footprint, x, y, True), ('Datasheet', defaults.get('Datasheet', ''), x, y, True),
              ('Description', desc or defaults.get('Description', ''), x, y, True)]
    if in_bom:
        fields += [('MPN', mpn, x, y, True), ('LCSC', lcsc, x, y, True), ('Note', note, x, y, True)]
    for k, v, px, py, hidden in fields:
        p = node('property', k, v, node('at', px, py, 90 if turned else 0), effects(1.27, None if turned else 'left'))
        if hidden:
            p.append(node('hide', Sym('yes')))
        s.append(p)
    pins = [p for sub in a if key(sub) == 'symbol' for p in sub if key(p) == 'pin']
    for pin in pins:
        num = get(pin, 'number')[1]
        at = get(pin, 'at')
        th = math.radians(angle)
        px, py = at[1], at[2]
        p = (round(x + px * math.cos(th) - py * math.sin(th), 4), round(y - px * math.sin(th) - py * math.cos(th), 4))
        s.append(node('pin', num, node('uuid', uid(ref + '/' + num))))
        net = nets.get(num)
        if net is None:
            items.append(node('no_connect', node('at', *p), node('uuid', ident())))
            continue
        direction = (int(at[3]) + angle) % 360
        vec = {0: (-1, 0), 90: (0, 1), 180: (1, 0), 270: (0, -1)}[direction]
        q = (round(p[0] + vec[0] * 5.08, 4), round(p[1] + vec[1] * 5.08, 4))
        items.append(node('wire', node('pts', node('xy', *p), node('xy', *q)),
                          node('stroke', node('width', 0), node('type', Sym('default'))), node('uuid', ident())))
        label(net, q, 180 if vec[0] < 0 else 0)
    s.append(node('instances', node('project', PROJECT, node('path', '/' + ROOT, node('reference', ref), node('unit', 1)))))
    items.append(s)
    if in_bom:
        bom.append({'Reference': ref, 'Value': value, 'Footprint': footprint, 'DNP': '0', 'LCSC': lcsc,
                    'MPN': mpn, 'Description': desc or defaults.get('Description', ''), 'Note': note})

text('Power supply upstream: mains in, inrush NTC on L, mains out to the 24 V supply', 25.4, 20.32, 2)
part('PS_Upstream:Conn_L_N_Pin2Slot', 'J1', 'MAINS_IN', 50.8, 76.2, {'1': 'L_IN', '3': 'N'},
     'PS_Upstream_Local:HT396R-3.96-3P_Pin2Slot', lcsc='C577800', mpn='HT396R-3.96-3P',
     desc='Mains input, Kefa HT396R right-angle pluggable header, 3.96 mm',
     note='1 L, 2 not connected (slot), 3 N')
part('Device:Thermistor_NTC', 'RT1', '10R NTC', 101.6, 66.04, {'1': 'L_IN', '2': 'L'},
     'PS_Upstream_Local:NTC_TDK_S236_P5.00', angle=90, lcsc='C125458', mpn='B57236S0100M000',
     desc='Inrush current limiter, TDK S236, 10 ohm, 3.5 A, 300 uF tested at 230 V',
     note='In series with L')
part('PS_Upstream:Conn_L_N_Pin2Slot', 'J2', 'MAINS_OUT', 152.4, 76.2, {'1': 'L', '3': 'N'},
     'PS_Upstream_Local:VH-3A_Pin2Slot', lcsc='C20547', mpn='VH-3A',
     desc='Mains output to the 24 V supply, JST VH-compatible vertical header, 3.96 mm',
     note='1 L, 2 not connected (slot), 3 N')
for ref, x in [('H1', 50.8), ('H2', 76.2)]:
    part('Mechanical:MountingHole', ref, 'M3 NYLON', x, 116.84, {}, 'PS_Upstream_Local:MountingHole_3.2mm_M3_Nylon')
text('J1 and J2: pin 1 = L, pin 3 = N. Pin 2 is not connected: it drops into a\n'
     'non-plated slot between L and N on the PCB.\n'
     'RT1 limits the inrush current of the 24 V supply input capacitors (81 uF):\n'
     '325 V / 10 ohm = 33 A peak. The supply has its own T2A fuse.\n'
     'H1 and H2: M3 holes between L and N, for nylon screws only.', 25.4, 132.08)

sheet = node('kicad_sch', node('version', 20260306), node('generator', Sym('eeschema')),
             node('generator_version', '10.0'), node('uuid', ROOT), node('paper', 'A4'),
             node('title_block', node('title', 'Power supply upstream'), node('date', '2026-10-26'),
                  node('rev', 'A'), node('company', 'CoffeeFlow'),
                  node('comment', 1, 'Mains in, inrush NTC, mains out to the 24 V supply')),
             node('lib_symbols', *libs.values()), *items,
             node('sheet_instances', node('path', '/', node('page', '1'))), node('embedded_fonts', Sym('no')))
# One top-level item per line keeps generated diffs readable.
(HERE / 'ps_upstream.kicad_sch').write_text('(' + dump(sheet[0]) + '\n' + '\n'.join(dump(i) for i in sheet[1:]) + '\n)\n')
with (HERE / 'ps_upstream_bom.csv').open('w', newline='') as f:
    w = csv.DictWriter(f, fieldnames=list(bom[0]))
    w.writeheader()
    w.writerows(bom)

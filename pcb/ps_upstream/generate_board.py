# /// script
# dependencies = []
# ///
"""Place and route the ps_upstream board from the schematic.
Run with: uv run generate_board.py [--force]
The board is overwritten only if it has no tracks yet, or with --force: once
the layout is edited by hand, it is the source and this script is history.

Coordinates in mm. x runs along the board, from the mains input J1 to the
output J2; y = 0 is the axis between L (y < 0) and N (y > 0). The pins of both
connectors sit 3.96 mm on each side of it, with pin 2 in a slot on the axis.
"""
import copy, math, sys
from pathlib import Path
from sexpr import parse, dump, key, get, node, Sym, effects, uid

HERE = Path(__file__).resolve().parent
BOARD = HERE / 'ps_upstream.kicad_pcb'
SCH = HERE / 'ps_upstream.kicad_sch'
OX, OY = 100.0, 100.0           # board origin on the sheet: left edge, L-N axis

PITCH = 3.96                    # L and N pins at -PITCH and +PITCH
TRACK = 2.0                     # mains track width
EDGE = 3.0                      # copper to board edge
HEAD = 3.1                      # courtyard radius of the nylon screw head (6.0 mm) + 0.1
GAP = 0.05                      # between neighbouring courtyards

# Courtyard extents along x once placed, from each footprint's own courtyard.
J1_BACK = 1.2                   # HT396R: courtyard behind the pin row, over the slot
J1_FRONT = 9.6                  # HT396R: mating face in front of the pin row (Kefa drawing)
RT1_HALF = 3.0                  # S236 disc, 6 mm courtyard across its thickness
J2_BACK = 4.80                  # VH-3A: courtyard on the side away from the latch ramp
J2_FRONT = 3.40                 # VH-3A: courtyard on the latch ramp side

x_j1 = J1_FRONT                 # mating face flush with the left edge
x_h1 = x_j1 + J1_BACK + GAP + HEAD
x_rt1 = x_h1 + HEAD + GAP + RT1_HALF
x_h2 = x_rt1 + RT1_HALF + GAP + HEAD
x_j2 = x_h2 + HEAD + GAP + J2_BACK
y_rt1 = -PITCH                  # NTC leads across the L track, 5 mm apart
RT1_IN, RT1_OUT = y_rt1 + 2.5, y_rt1 - 2.5
LENGTH = round(max(x_j2 + J2_FRONT, x_j2 + 1.4 + EDGE) + 0.04, 1)
TOP = round(RT1_OUT - 1.0 - EDGE - 0.04, 1)       # RT1 pad radius 1.0
BOTTOM = round(PITCH + 1.4 + EDGE + 0.14, 1)      # J2 pad radius 1.4

if '--force' not in sys.argv and BOARD.exists() and '(segment' in BOARD.read_text():
    sys.exit('ps_upstream.kicad_pcb already has tracks; rerun with --force to overwrite it')

sch = parse(SCH.read_text())
symbols = {}
for s in sch:
    if key(s) != 'symbol':
        continue
    props = {p[1]: p[2] for p in s if key(p) == 'property'}
    symbols[props['Reference']] = (s, props)

# Net of each symbol pin, from the generated schematic: one wire from the pin
# to a local label.
labels = {tuple(get(a, 'at')[1:3]): a[1] for a in sch if key(a) == 'label'}
wires = [[tuple(p[1:3]) for p in get(w, 'pts')[1:]] for w in sch if key(w) == 'wire']
def pin_nets(ref):
    s, props = symbols[ref]
    libsym = next(a for a in get(sch, 'lib_symbols')[1:] if a[1] == get(s, 'lib_id')[1])
    x, y, angle = get(s, 'at')[1:4]
    th = math.radians(angle)
    nets = {}
    for sub in libsym:
        if key(sub) != 'symbol':
            continue
        for pin in sub:
            if key(pin) != 'pin':
                continue
            px, py = get(pin, 'at')[1:3]
            p = (round(x + px * math.cos(th) - py * math.sin(th), 4), round(y - px * math.sin(th) - py * math.cos(th), 4))
            for a, b in wires:
                end = b if a == p else a if b == p else None
                if end in labels:
                    nets[get(pin, 'number')[1]] = ('/' + labels[end], get(pin, 'name')[1], Sym(pin[1]))
    return nets

def place(ref, x, y, angle, silk_value=False):
    s, props = symbols[ref]
    lib, name = props['Footprint'].split(':')
    fp = parse((HERE / (lib + '.pretty') / (name + '.kicad_mod')).read_text())
    nets = pin_nets(ref)
    body = [Sym('footprint'), props['Footprint'], node('layer', 'F.Cu'), node('uuid', uid('fp/' + ref)),
            node('at', round(OX + x, 4), round(OY + y, 4), angle)]
    for k in ['Reference', 'Value', 'Datasheet', 'Description', 'MPN', 'LCSC', 'Note']:
        if k not in props:
            continue
        hidden = k not in ('Reference', 'Value')
        # References and values stay on the fabrication layer: the silkscreen
        # carries only the board texts placed below, clear of pads and holes.
        p = node('property', k, props[k], node('at', 0, 0, angle), node('layer', 'F.Fab'))
        if hidden:
            p.append(node('hide', Sym('yes')))
        p += [node('uuid', uid('fp/' + ref + '/' + k)), node('effects', node('font', node('size', 0.8, 0.8), node('thickness', 0.12)))]
        body.append(p)
    body += [node('path', '/' + str(get(s, 'uuid')[1])), node('sheetname', '/'), node('sheetfile', 'ps_upstream.kicad_sch')]
    for item in fp[2:]:
        k = key(item)
        if k in ('version', 'generator', 'generator_version', 'layer', 'property', 'embedded_fonts'):
            continue
        item = copy.deepcopy(item)
        if k == 'pad':
            at = get(item, 'at')
            local = at[3] if len(at) > 3 else 0
            at[1:] = [at[1], at[2], (local + angle) % 360]
            if item[1] in nets:
                net, function, kind = nets[item[1]]
                uuid_at = next(i for i, e in enumerate(item) if key(e) == 'uuid')
                item[uuid_at:uuid_at] = [node('net', net), node('pinfunction', function), node('pintype', kind)]
        if k == 'fp_text':
            get(item, 'at')[1:] = [*get(item, 'at')[1:3], angle]
        body.append(item)
    body.append(node('embedded_fonts', Sym('no')))
    return body

def segment(net, *points):
    out = []
    for (x1, y1), (x2, y2) in zip(points, points[1:]):
        out.append(node('segment', node('start', round(OX + x1, 4), round(OY + y1, 4)),
                        node('end', round(OX + x2, 4), round(OY + y2, 4)), node('width', TRACK),
                        node('layer', 'F.Cu'), node('net', net),
                        node('uuid', uid('seg/' + net + '/' + str(x1) + ',' + str(y1)))))
    return out

def silk(text, x, y, size=1.0, angle=0):
    return node('gr_text', text, node('at', round(OX + x, 4), round(OY + y, 4), angle), node('layer', 'F.SilkS'),
                node('uuid', uid('text/' + text + '/' + str(x) + ',' + str(y))),
                node('effects', node('font', node('size', size, size), node('thickness', 0.15))))

layers = [(0, 'F.Cu', 'signal'), (2, 'B.Cu', 'signal'), (9, 'F.Adhes', 'user', 'F.Adhesive'),
          (11, 'B.Adhes', 'user', 'B.Adhesive'), (13, 'F.Paste', 'user'), (15, 'B.Paste', 'user'),
          (5, 'F.SilkS', 'user', 'F.Silkscreen'), (7, 'B.SilkS', 'user', 'B.Silkscreen'),
          (1, 'F.Mask', 'user'), (3, 'B.Mask', 'user'), (17, 'Dwgs.User', 'user', 'User.Drawings'),
          (19, 'Cmts.User', 'user', 'User.Comments'), (21, 'Eco1.User', 'user', 'User.Eco1'),
          (23, 'Eco2.User', 'user', 'User.Eco2'), (25, 'Edge.Cuts', 'user'), (27, 'Margin', 'user'),
          (31, 'F.CrtYd', 'user', 'F.Courtyard'), (29, 'B.CrtYd', 'user', 'B.Courtyard'),
          (35, 'F.Fab', 'user'), (33, 'B.Fab', 'user')]
board = [Sym('kicad_pcb'), node('version', 20260206), node('generator', 'pcbnew'), node('generator_version', '10.0'),
         node('general', node('thickness', 1.6), node('legacy_teardrops', Sym('no'))), node('paper', 'A4'),
         node('layers', *[[l[0], l[1], Sym(l[2]), *l[3:]] for l in layers]),
         node('setup', node('pad_to_mask_clearance', 0), node('allow_soldermask_bridges_in_footprints', Sym('no')))]
board += [place('J1', x_j1, 0, 270), place('RT1', x_rt1, y_rt1, 90), place('J2', x_j2, 0, 270),
          place('H1', x_h1, 0, 0), place('H2', x_h2, 0, 0)]
board.append(node('gr_rect', node('start', OX, round(OY + TOP, 4)), node('end', round(OX + LENGTH, 4), round(OY + BOTTOM, 4)),
                  node('stroke', node('width', 0.1), node('type', Sym('solid'))), node('fill', Sym('no')),
                  node('layer', 'Edge.Cuts'), node('uuid', uid('outline'))))
# L_IN leaves J1 along the L axis, clears H1, then rises 45 degrees to the
# RT1 lead nearest the axis. L leaves the outer lead along the top edge and
# drops 45 degrees onto J2. N runs straight between the two connectors.
rise = RT1_IN - (-PITCH)
board += segment('/L_IN', (x_j1, -PITCH), (x_rt1 - 1.0 - rise, -PITCH), (x_rt1 - 1.0, RT1_IN), (x_rt1, RT1_IN))
drop = -PITCH - RT1_OUT
board += segment('/L', (x_rt1, RT1_OUT), (x_j2 - drop, RT1_OUT), (x_j2, -PITCH))
board += segment('/N', (x_j1, PITCH), (x_j2, PITCH))
text_y = round((PITCH + 1.0 + BOTTOM) / 2, 2)
board += [silk('PS_UPSTREAM - M3 NYLON ONLY', (x_j1 + x_j2) / 2, text_y, 0.8)]
for x in (1.5, x_j2):
    board += [silk('L', x, -7.5, 1.0), silk('N', x, 7.2, 1.0)]
board.append(silk('230VAC 0.5A', 7.0, -9.3, 0.8))
# The bare board reaches JLCPCB's review as copper, silkscreen and drill
# files only: this note goes on the silkscreen, under each connector body,
# so a slot without a pad does not read as a missing plated hole.
board += [silk('NO PIN', x_j1 - 6.0, 0, 0.8), silk('NO PIN', x_j2 - J2_BACK + 1.25, 0, 0.8, 90)]
board.append(node('embedded_fonts', Sym('no')))
BOARD.write_text('(' + dump(board[0]) + '\n' + '\n'.join(dump(i) for i in board[1:]) + '\n)\n')
print(f'board {LENGTH} x {BOTTOM - TOP:.1f} mm; J1 {x_j1:.2f}, H1 {x_h1:.2f}, RT1 {x_rt1:.2f}, H2 {x_h2:.2f}, J2 {x_j2:.2f}')

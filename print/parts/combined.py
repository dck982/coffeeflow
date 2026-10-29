from nurb import *

# Edge.Cuts of pcb/combined/combined.kicad_pcb, in KiCad millimetres (Y down).
_BOARD_OUTLINE_KICAD = [
    (80.5, 47.0), (80.5, 98.8), (55.9, 98.8), (55.9, 134.6),
    (106.0, 134.6), (106.0, 82.0), (180.7, 82.0), (180.7, 59.0),
    (156.5, 59.0), (156.5, 47.0),
]
_WEST = min(x for x, _ in _BOARD_OUTLINE_KICAD)
_SOUTH = max(y for _, y in _BOARD_OUTLINE_KICAD)

# M2 mounting holes H1-H6, KiCad centres.
_HOLES_KICAD = [
    (91.0, 132.1), (154.0, 49.5), (83.0, 49.5),
    (58.4, 132.2), (58.4, 101.4), (120.9, 79.5),
]

# Tall components: KiCad X range, Y range, height above the board's top face.
# Measured on the 3D models placed as on the board (not on the LCSC footprint
# outlines, drawn too large for the HT396) and checked against the Kefa and
# JST drawings in docs/jlcpcb.
_COMPONENTS_KICAD = {
    'J1': ((72.45, 84.85), (128.25, 134.00), 7.0),     # XH 4, 24 V and CAN
    'J9': ((92.55, 104.95), (110.80, 116.55), 7.0),    # XH 4, pump
    'J10': ((95.05, 104.95), (118.75, 124.50), 7.0),   # XH 3, pressure
    'J6': ((81.20, 88.60), (102.16, 114.04), 10.1),    # HT396V 3, flowmeter
    'J7': ((62.36, 70.28), (126.90, 134.30), 10.1),    # HT396V 2, heater SSR
    'J8': ((97.36, 105.28), (126.90, 134.30), 10.1),   # HT396V 2, boiler SSR
    'J2': ((170.55, 180.65), (72.64, 80.56), 7.4),     # HT396R 2, mains in
    'J3': ((170.55, 180.65), (60.30, 68.22), 7.4),     # HT396R 2, valve
}

# XIAO ESP32-S3 on its sockets: 17.8 mm wide across the pin rows, 21 mm long
# plus the 1.5 mm USB-C overhang to the south (silkscreen outline on the board).
_XIAO_KICAD = ((82.08, 99.86), (68.47, 90.93))


def _to_part(x, y):
    """KiCad millimetres to part coordinates: south-west corner at the origin, Y north."""
    return x - _WEST, _SOUTH - y


def _outline_box(xs, ys, z0, height, wall):
    """Four walls around a component footprint, standing from z0."""
    (x0, y1), (x1, y0) = _to_part(xs[0], ys[0]), _to_part(xs[1], ys[1])
    width, depth = x1 - x0, y1 - y0
    centre = Pos((x0 + x1) / 2, (y0 + y1) / 2, z0)
    outer = Box(width, depth, height, align=(Align.CENTER, Align.CENTER, Align.MIN))
    inner = Box(width - 2 * wall, depth - 2 * wall, height,
                align=(Align.CENTER, Align.CENTER, Align.MIN))
    return centre * (outer - inner)


@part
def combined(board_thickness=1.6, hole_width=2.2, component_wall=1.0,
             xiao_total_height=15.0):
    """Stand-in for the combined board to design its enclosure around: exact
    outline, M2 holes, and the tall components as four-walled boxes.

    board_thickness: thickness of the PCB
    hole_width: diameter of the six mounting holes
    component_wall: wall thickness of the component boxes
    xiao_total_height: XIAO top above the PCB underside, board and socket included
    """
    outline = [_to_part(x, y) for x, y in _BOARD_OUTLINE_KICAD]
    board = extrude(Polygon(*outline, align=None), amount=board_thickness)
    for x, y in _HOLES_KICAD:
        board -= Pos(*_to_part(x, y), 0) * Cylinder(
            hole_width / 2, board_thickness, align=(Align.CENTER, Align.CENTER, Align.MIN))
    for xs, ys, height in _COMPONENTS_KICAD.values():
        board += _outline_box(xs, ys, board_thickness, height, component_wall)
    board += _outline_box(*_XIAO_KICAD, board_thickness,
                          xiao_total_height - board_thickness, component_wall)
    return board

from nurb import *

from system import (
    AMIN, CMIN, INSERT_M2, add_heat_insert_against, cable_tie_tower, scale_frame, scale_layout,
)

CHAMFER = 1.0
# How far the pilot holes and cutters overshoot a face so the cut is clean.
OVERSHOOT = 1.0


def block(x0, x1, y0, y1, z0, z1):
    """Axis-aligned box from its six faces, in the machine frame."""
    return Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, z1 - z0, align=AMIN)


def _vertical_corners(solid):
    """The vertical edges of a box-like solid, which get the 1 mm corner chamfer."""
    return solid.edges().filter_by(Axis.Z)


def _floor_plate(layout, draft):
    """Floor slab plus the tongue under the cable saddle, one outline, chamfered
    on every convex edge except the bed face, the concave tongue junctions and
    the tongue's top edges, which the saddle post covers."""
    b = layout.base
    outline = [
        (b.x0, b.y0), (b.saddle_x0, b.y0), (b.saddle_x0, b.saddle_y0),
        (b.saddle_x1, b.saddle_y0), (b.saddle_x1, b.y0), (b.x1, b.y0),
        (b.x1, b.y1), (b.x0, b.y1),
    ]
    plate = extrude(Polygon(*outline, align=None), b.floor_top)
    if draft:
        return plate
    concave = concave_edges(plate)

    def exposed_edge(edge):
        box = edge.bounding_box()
        vertical = box.max.Z - box.min.Z > 0.5
        on_tongue_top = not vertical and box.min.Y < b.y0 - 0.01
        return box.max.Z > b.floor_top / 2.0 and edge not in concave and not on_tongue_top

    exposed = plate.edges().filter_by(exposed_edge)
    return polish(plate, exposed, CHAMFER)


def _cable_saddle(layout, notch_width, draft):
    """Post with a groove across its top, along Y, for the bus cable, and a
    window through it along X under the groove for a cable tie, which goes
    round the cable at right angles to it."""
    b = layout.base
    chamfer = CHAMFER
    outline = [
        (b.saddle_x0 + chamfer, b.saddle_y0), (b.saddle_x1 - chamfer, b.saddle_y0),
        (b.saddle_x1, b.saddle_y0 + chamfer), (b.saddle_x1, b.saddle_y1),
        (b.saddle_x0, b.saddle_y1), (b.saddle_x0, b.saddle_y0 + chamfer),
    ]
    post = extrude(Polygon(*outline, align=None), b.saddle_top)
    if draft:
        post = block(b.saddle_x0, b.saddle_x1, b.saddle_y0, b.saddle_y1, 0.0, b.saddle_top)

    centre_x = (b.saddle_x0 + b.saddle_x1) / 2.0
    groove = block(
        centre_x - notch_width / 2.0, centre_x + notch_width / 2.0,
        b.saddle_y0 - OVERSHOOT, b.saddle_y1 + OVERSHOOT,
        b.saddle_top - b.saddle_notch_depth, b.saddle_top + OVERSHOOT,
    )
    centre_y = (b.saddle_y0 + b.saddle_y1) / 2.0
    tie_window = block(
        b.saddle_x0 - OVERSHOOT, b.saddle_x1 + OVERSHOOT,
        centre_y - b.saddle_tie_width / 2.0, centre_y + b.saddle_tie_width / 2.0,
        b.saddle_tie_bottom, b.saddle_tie_bottom + b.saddle_tie_height,
    )
    return post - groove - tie_window


def _pad(layout):
    """Raised pad under the cell's fixed end."""
    b, c = layout.base, layout.cell
    return block(b.pad_x0, c.x1 + 2.0, c.y0 - 3.0, c.y1 + 3.0, b.floor_top - 0.5, c.z0)


def _countersinks(layout):
    """Cutters for the two M4 countersunk screws, entering from the bed face:
    head 0.3 mm below flush, 90 degree cone, clearance hole above it."""
    c = layout.cell
    head = measured("vis_fraisee_m4_tete")
    clearance = 4.5
    sunk = 0.3
    cone_height = (head - clearance) / 2.0
    cutters = []
    for x in c.fixed_xs:
        y = layout.axis[1]
        shaft = Pos(x, y, -OVERSHOOT) * Cylinder(clearance / 2.0, c.z0 + 2 * OVERSHOOT, align=CMIN)
        well = Pos(x, y, -OVERSHOOT) * Cylinder(head / 2.0, sunk + OVERSHOOT, align=CMIN)
        cone = Pos(x, y, sunk) * Cone(head / 2.0, clearance / 2.0, cone_height, align=CMIN)
        cutters += [shaft, well, cone]
    return cutters


def _screw_holes(layout):
    """Cutters for the three M3 screws: clearance through the floor and a
    hex pocket in the floor's top that holds the nut while the screw is
    driven from under the plate."""
    b = layout.base
    clearance = measured("vis_passage")
    nut_across_flats = measured("scale_ecrou_plats") + 0.3
    hex_face = RegularPolygon(nut_across_flats / 2.0, 6, major_radius=False)
    pocket_bottom = b.floor_top - b.nut_pocket_depth
    cutters = []
    for x, y in b.screw_holes:
        shaft = Pos(x, y, -OVERSHOOT) * Cylinder(clearance / 2.0, b.floor_top + 2 * OVERSHOOT, align=CMIN)
        pocket = Pos(x, y, pocket_bottom) * extrude(hex_face, b.nut_pocket_depth + OVERSHOOT)
        cutters += [shaft, pocket]
    return cutters


def _bridge(layout, side_gap, post_thickness):
    """Upward stop: two posts beside the cell and a beam over it. The cell
    slides in lengthwise from +X, under the beam."""
    b, c = layout.base, layout.cell
    x0, x1 = b.bridge_x0, b.bridge_x1
    beam_bottom = c.z1 + b.bridge_gap
    beam_top = beam_bottom + b.bridge_thickness
    south = (c.y0 - side_gap - post_thickness, c.y0 - side_gap)
    north = (c.y1 + side_gap, c.y1 + side_gap + post_thickness)
    posts = [block(x0, x1, y0, y1, b.floor_top - 0.5, beam_top) for y0, y1 in (south, north)]
    beam = block(x0, x1, south[0], north[1], beam_bottom, beam_top)
    return posts[0] + posts[1] + beam


def _pcb_box_outer(layout, draft):
    """Outer shell of the PCB box, vertical corners chamfered."""
    x = layout.box
    outer = block(
        x.inner_x0 - x.wall, x.inner_x1 + x.wall, x.inner_y0 - x.wall, x.inner_y1 + x.wall,
        x.floor_top, x.wall_top,
    )
    return outer if draft else polish(outer, _vertical_corners(outer), CHAMFER)


def _wire_notches(layout):
    """U-shaped notch through each X-end wall, round at the bottom."""
    x = layout.box
    radius = x.notch_width / 2.0
    centre_z = x.notch_bottom + radius
    x0, x1 = x.inner_x0 - x.wall - OVERSHOOT, x.inner_x1 + x.wall + OVERSHOOT
    slot = block(x0, x1, x.notch_y - radius, x.notch_y + radius, centre_z, x.wall_top + OVERSHOOT)
    round_bottom = Pos(x0, x.notch_y, centre_z) * Rot(0, 90, 0) * Cylinder(radius, x1 - x0, align=(Align.CENTER, Align.CENTER, Align.MIN))
    return slot + round_bottom


def _pcb_box(layout, ledge_depth, ledge_length, rib_outside, draft):
    """Open-top box: walls, wire notches, two M2 insert bosses under the PCB's
    -Y edge, a ledge under its +Y edge and a rib under each short edge, below
    the terminal blocks."""
    x = layout.box
    outer = _pcb_box_outer(layout, draft)
    cavity = block(x.inner_x0, x.inner_x1, x.inner_y0, x.inner_y1, x.floor_top, x.wall_top + OVERSHOOT)
    walls = outer - cavity - _wire_notches(layout)

    boss_height = x.pcb_underside - x.floor_top
    for hole_x, hole_y in x.pcb_holes:
        walls = add_heat_insert_against(
            walls, outer, hole_x, hole_y, (hole_x, x.inner_y0), x.floor_top, boss_height, INSERT_M2
        )

    pcb_middle_x = (x.pcb_x0 + x.pcb_x1) / 2.0
    ledge = block(
        pcb_middle_x - ledge_length / 2.0, pcb_middle_x + ledge_length / 2.0,
        x.inner_y1 - ledge_depth, x.inner_y1 + x.wall / 2.0,
        x.floor_top - 0.5, x.pcb_underside,
    )
    # The terminal pads start 2.5 mm in from each short edge: the rib only
    # reaches ledge_depth under the board.
    pcb_middle_y = (x.pcb_y0 + x.pcb_y1) / 2.0
    y0, y1 = pcb_middle_y - ledge_length / 2.0, pcb_middle_y + ledge_length / 2.0
    ribs = [
        block(x.pcb_x0 - rib_outside, x.pcb_x0 + ledge_depth, y0, y1, x.floor_top - 0.5, x.pcb_underside),
        block(x.pcb_x1 - ledge_depth, x.pcb_x1 + rib_outside, y0, y1, x.floor_top - 0.5, x.pcb_underside),
    ]
    return walls + ledge + ribs[0] + ribs[1]


def _tie_towers(layout):
    """Two cable tie arches that hold the cell's wires along the corridor
    between the PCB box and the cell, on their way to J2."""
    b = layout.base
    towers = [cable_tie_tower(x, b.tie_tower_y, b.tie_tower_height + 0.5, b.floor_top - 0.5)
              for x in b.tie_tower_xs]
    return towers[0] + towers[1]


@part
def scale_base(
    cell_side_gap=0.4,
    bridge_post_thickness=3.0,
    ledge_depth=1.2,
    ledge_length=10.0,
    rib_outside=2.0,
    saddle_groove_width=5.0,
    draft=False,
):
    """Base of the drip tray scale: floor on the base plate, pad and upward stop
    for the load cell, the PCB box and the cable saddle. Prints upright.

    cell_side_gap: clearance between the cell's sides and the bridge posts
    bridge_post_thickness: thickness of each bridge post along Y
    ledge_depth: how far the PCB's rest ledge and the two ribs reach under the board
    ledge_length: length of the ledge and of each rib, centred on the PCB
    rib_outside: how far each rib under a terminal block extends past the board's edge
    saddle_groove_width: width of the groove across the cable saddle's top
    """
    layout = scale_layout()
    body = _floor_plate(layout, draft)
    body += _cable_saddle(layout, saddle_groove_width, draft)
    body += _pad(layout)
    body += _bridge(layout, cell_side_gap, bridge_post_thickness)
    body += _pcb_box(layout, ledge_depth, ledge_length, rib_outside, draft)
    body += _tie_towers(layout)
    for cutter in _countersinks(layout) + _screw_holes(layout):
        body -= cutter
    return scale_frame(body)

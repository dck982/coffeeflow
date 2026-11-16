from nurb import *

from system import AMIN, CMIN, INSERT_M2, add_heat_insert_against, scale_frame, scale_layout

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
    the tongue's top edges, which the saddle post and its buttress cover."""
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


def _cable_saddle(layout, buttress_reach, notch_width, draft):
    """Post with a groove across its top, along Y, and a 45 degree buttress
    toward +Y that stands on the tongue and the floor."""
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

    # Triangle in the YZ plane: vertical against the post, 45 degree slope.
    # It starts inside the post and below the floor top so it fuses cleanly.
    root_y, root_z = b.saddle_y1 - 0.5, b.floor_top - 0.5
    reach_y = b.saddle_y1 + buttress_reach + 0.5
    triangle = Polygon(
        (root_y, root_z), (reach_y, root_z), (root_y, root_z + reach_y - root_y),
        align=None,
    )
    buttress = Pos(b.saddle_x0, 0, 0) * extrude(Plane.YZ * triangle, b.saddle_x1 - b.saddle_x0)

    centre_x = (b.saddle_x0 + b.saddle_x1) / 2.0
    groove = block(
        centre_x - notch_width / 2.0, centre_x + notch_width / 2.0,
        b.saddle_y0 - OVERSHOOT, b.saddle_y1 + OVERSHOOT,
        b.saddle_top - b.saddle_notch_depth, b.saddle_top + OVERSHOOT,
    )
    return post + buttress - groove


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


def _pilot_holes(layout):
    """Cutters for the three M3 self-tapping pilots, through the floor."""
    diameter = measured("vis_autoforeuse")
    height = layout.base.floor_top + 2 * OVERSHOOT
    return [
        Pos(x, y, -OVERSHOOT) * Cylinder(diameter / 2.0, height, align=CMIN)
        for x, y in layout.base.spigots
    ]


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


def _pcb_box(layout, ledge_depth, ledge_length, draft):
    """Open-top box: walls, wire notches, two M2 insert bosses under the PCB's
    +Y edge and a ledge under its -Y edge."""
    x = layout.box
    outer = _pcb_box_outer(layout, draft)
    cavity = block(x.inner_x0, x.inner_x1, x.inner_y0, x.inner_y1, x.floor_top, x.wall_top + OVERSHOOT)
    walls = outer - cavity - _wire_notches(layout)

    boss_height = x.pcb_underside - x.floor_top
    for hole_x, hole_y in x.pcb_holes:
        walls = add_heat_insert_against(
            walls, outer, hole_x, hole_y, (hole_x, x.inner_y1), x.floor_top, boss_height, INSERT_M2
        )

    pcb_middle = (x.pcb_x0 + x.pcb_x1) / 2.0
    ledge = block(
        pcb_middle - ledge_length / 2.0, pcb_middle + ledge_length / 2.0,
        x.inner_y0 - x.wall / 2.0, x.inner_y0 + ledge_depth,
        x.floor_top, x.pcb_underside,
    )
    return walls + ledge


@part
def scale_base(
    cell_side_gap=0.4,
    bridge_post_thickness=3.0,
    ledge_depth=1.2,
    ledge_length=10.0,
    saddle_groove_width=5.0,
    buttress_reach=4.0,
    draft=False,
):
    """Base of the drip tray scale: floor on the base plate, pad and upward stop
    for the load cell, the PCB box and the cable saddle. Prints upright.

    cell_side_gap: clearance between the cell's sides and the bridge posts
    bridge_post_thickness: thickness of each bridge post along Y
    ledge_depth: how far the PCB's rest ledge sticks out from the -Y wall
    ledge_length: length of that ledge along X, centred on the PCB
    saddle_groove_width: width of the groove across the cable saddle's top
    buttress_reach: how far the saddle's 45 degree buttress extends toward +Y
    """
    layout = scale_layout()
    body = _floor_plate(layout, draft)
    body += _cable_saddle(layout, buttress_reach, saddle_groove_width, draft)
    body += _pad(layout)
    body += _bridge(layout, cell_side_gap, bridge_post_thickness)
    body += _pcb_box(layout, ledge_depth, ledge_length, draft)
    for cutter in _countersinks(layout) + _pilot_holes(layout):
        body -= cutter
    return scale_frame(body)

import math

from nurb import *

from system import CMIN, magnet_well_cutter, scale_flip, scale_frame, scale_layout, scale_circle, scale_seat

# Clearances around the drain slots, mm.
SLOT_RING_CLEARANCE = 1.0      # from the drip ring's outer face
SLOT_EDGE_BORDER = 3.0         # solid border to the plate's edge
SLOT_FINGER_CLEARANCE = 2.5    # from the finger holes
SCAN_STEP = 0.1                # sampling step along a row before the ends are refined


def _ring_radii(layout):
    """(inner, outer) radius of the drip ring.

    The chimney's outer face is the kerb grown by the tube wall (its grip
    only shrinks it by a tenth); the ring follows it at a constant gap."""
    tray, chimney, grille = layout.tray, layout.chimney, layout.grille_inner
    inner = tray.window_diameter / 2.0 + tray.kerb_wall + chimney.wall + grille.drip_ring_clearance
    return inner, inner + grille.drip_ring_wall


def _slot_room(grille, corner_chamfer, slot_width, slot_web):
    """Predicate on a slot's centre line point (x, y), relative to the axis, and
    the side ("east-west" or "north-south") whose rows may use it."""
    half_x, half_y = grille.width / 2.0, grille.length / 2.0
    _, ring_radius = _ring_radii(scale_layout())
    ring_reach = ring_radius + SLOT_RING_CLEARANCE + slot_width / 2.0
    border = SLOT_EDGE_BORDER + slot_width / 2.0
    finger_x, finger_y = _finger_hole_offsets(grille)
    finger_reach = grille.finger_hole_diameter / 2.0 + SLOT_FINGER_CLEARANCE + slot_width / 2.0
    # The diagonal from each plate corner splits the east-west rows from the
    # north-south ones; each keeps half a web from it.
    mitre = half_y - half_x
    mitre_margin = (slot_width + slot_web) / 2.0 * math.sqrt(2.0)

    def allowed(x, y, side):
        ax, ay = abs(x), abs(y)
        if ax > half_x - border or ay > half_y - border:
            return False
        if ax + ay > half_x + half_y - corner_chamfer - border * math.sqrt(2.0):
            return False
        if side == "east-west" and ay - ax > mitre - mitre_margin:
            return False
        if side == "north-south" and ay - ax < mitre + mitre_margin:
            return False
        if (ax - finger_x) ** 2 + (ay - finger_y) ** 2 < finger_reach ** 2:
            return False
        return x * x + y * y > ring_reach * ring_reach

    return allowed


def _runs(free, start, stop):
    """Intervals of [start, stop] where `free(t)` holds, ends refined by bisection."""
    def edge(inside, outside):
        for _ in range(30):
            middle = (inside + outside) / 2.0
            inside, outside = (middle, outside) if free(middle) else (inside, middle)
        return inside

    steps = int((stop - start) / SCAN_STEP)
    samples = [start + k * (stop - start) / steps for k in range(steps + 1)]
    runs, run_start, previous = [], None, None
    for t in samples:
        if free(t) and run_start is None:
            run_start = t if previous is None else edge(t, previous)
        elif not free(t) and run_start is not None:
            runs.append((run_start, edge(previous, t)))
            run_start = None
        previous = t
    if run_start is not None:
        runs.append((run_start, stop))
    return runs


def _split(run, slot_width, slot_web, max_slot_length):
    """A run of centre line cut into slots no longer than `max_slot_length`
    overall, with a web between them. Returns (centre, overall length) pairs."""
    first, last = run
    overall = last - first + slot_width
    count = math.ceil((overall + slot_web) / (max_slot_length + slot_web))
    length = (overall - (count - 1) * slot_web) / count
    return [(first - slot_width / 2.0 + length / 2.0 + k * (length + slot_web), length) for k in range(count)]


def _slot_layout(grille, corner_chamfer, slot_width, slot_web, max_slot_length):
    """Rows of slots parallel to each side, from the border inward, as many as
    reach past the drip ring. Returns (x, y, length, along_y) per slot, relative
    to the axis; the layout is mirror symmetric in X and in Y."""
    allowed = _slot_room(grille, corner_chamfer, slot_width, slot_web)
    pitch = slot_width + slot_web
    slots = []
    for side, half_across, half_along in (("east-west", grille.width / 2.0, grille.length / 2.0),
                                          ("north-south", grille.length / 2.0, grille.width / 2.0)):
        along_y = side == "east-west"
        offset = half_across - SLOT_EDGE_BORDER - slot_width / 2.0
        while offset > 0.0:
            def free(t, offset=offset):
                return allowed(offset, t, side) if along_y else allowed(t, offset, side)
            for run in _runs(free, -half_along, half_along):
                if run[1] - run[0] < 2.0 * slot_width:
                    continue
                for centre, length in _split(run, slot_width, slot_web, max_slot_length):
                    for sign in (-1.0, 1.0):
                        x, y = (sign * offset, centre) if along_y else (centre, sign * offset)
                        slots.append((x, y, length, along_y))
            offset -= pitch
    if not slots:
        reject(f"slot_width {slot_width} leaves no room for a slot between the drip ring and the border",
               param="slot_width")
    return slots


def _slot_cutters(axis, z_bottom, height, slot_width, slots):
    cutters = None
    for x, y, length, along_y in slots:
        slot = extrude(SlotOverall(length, slot_width), amount=height)
        slot = Pos(x, y, 0) * Rot(0, 0, 90.0 if along_y else 0.0) * slot
        cutters = slot if cutters is None else cutters + slot
    return Pos(axis[0], axis[1], z_bottom) * cutters


def _finger_hole_offsets(grille):
    """(x, y) distance of the finger holes' centres from the axis, so each hole
    keeps `finger_hole_border` of solid to both plate edges of its corner."""
    inset = grille.finger_hole_border + grille.finger_hole_diameter / 2.0
    return grille.width / 2.0 - inset, grille.length / 2.0 - inset


def _plate(axis, width, length, z0, thickness, corner_chamfer, edge_chamfer, draft):
    """The rectangular plate, its underside rim chamfered; the top face stays flat for the bed."""
    plate = Pos(axis[0], axis[1], z0) * Box(width, length, thickness, align=(Align.CENTER, Align.CENTER, Align.MIN))
    plate = chamfer(plate.edges().filter_by(Axis.Z), corner_chamfer)
    if draft:
        return plate
    underside_rim = plate.edges().filter_by(
        lambda edge: edge.bounding_box().max.Z < z0 + 0.01)
    return polish(plate, underside_rim, edge_chamfer)


def _drip_ring(layout, underside):
    """Ring under the plate, around the chimney's top, so water running
    under the plate falls outside the chimney."""
    grille = layout.grille_inner
    inner, outer = _ring_radii(layout)
    outline = scale_circle(outer) - scale_circle(inner)
    return Pos(0, 0, grille.drip_ring_bottom) * extrude(outline, amount=underside - grille.drip_ring_bottom)


@part
def scale_grille_inner(slot_width=3.0, slot_web=2.5, max_slot_length=25.0,
                       corner_chamfer=4.0, underside_chamfer=1.0, push_out_hole=2.5, draft=False):
    """
    Cup grille that sits on the tower's magnets; solid over the chimney, rows of drain slots around it.

    slot_width: width of each drain slot
    slot_web: solid strip between two slots, side by side or end to end
    max_slot_length: longest slot before it is split in two with a web between
    corner_chamfer: size of the chamfer on the plate's four vertical corners
    underside_chamfer: chamfer along the plate's underside outline
    push_out_hole: hole on each magnet's axis from the plate's top, to push the magnet out with a pin
    """
    layout = scale_layout()
    axis_x, axis_y = layout.axis
    seat, tower, grille = layout.seat, layout.tower, layout.grille_inner
    underside = grille.top - grille.thickness

    body = _plate(layout.axis, grille.width, grille.length, underside, grille.thickness,
                  corner_chamfer, underside_chamfer, draft)

    # Seat boss under the plate, with the strong magnets flush in its lower face.
    boss = scale_seat(tower.top_seat_z, seat.height, up=False)
    boss_bottom = tower.top_seat_z - seat.height
    magnet_height = measured("aimant_hauteur")
    for well_x, well_y in tower.strong_magnets:
        boss -= magnet_well_cutter(
            well_x, well_y, boss_bottom - measured("aimant_puit_fond"), height=magnet_height)
    body += boss
    # A pin pushed down the hole from the plate's top gets each magnet out.
    for well_x, well_y in tower.strong_magnets:
        body -= Pos(well_x, well_y, boss_bottom + magnet_height - 0.01) * Cylinder(
            push_out_hole / 2.0, grille.top - boss_bottom - magnet_height + 1.0, align=CMIN)
    body += _drip_ring(layout, underside)

    # Drain slots and finger holes cut through the plate only.
    slots = _slot_layout(grille, corner_chamfer, slot_width, slot_web, max_slot_length)
    body -= _slot_cutters(layout.axis, underside - 0.01, grille.thickness + 0.02, slot_width, slots)
    finger_x, finger_y = _finger_hole_offsets(grille)
    for sign in (-1.0, 1.0):
        body -= Pos(axis_x + sign * finger_x, axis_y - sign * finger_y, underside - 0.01) * Cylinder(
            grille.finger_hole_diameter / 2.0, grille.thickness + 0.02, align=(Align.CENTER, Align.CENTER, Align.MIN))

    return scale_flip(scale_frame(body))

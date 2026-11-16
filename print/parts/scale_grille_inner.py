import math

from nurb import *

from system import magnet_well_cutter, scale_flip, scale_frame, scale_layout, scale_seat

# Clearances around the drain slots, mm.
SLOT_RING_CLEARANCE = 1.0      # from the drip ring's outer face
SLOT_EDGE_BORDER = 3.0         # solid border to the plate's edge
SLOT_FINGER_CLEARANCE = 2.5    # from the finger holes


def _slot_row(grille, slot_width):
    """Where the single row of drain slots sits on each side of the axis.

    Returns (distance of the slot's centre line from the axis, overall slot
    length). The row lies between the drip ring and the plate's border, and
    each slot stops short of the finger hole that the symmetric pattern puts
    beside its end."""
    ring_outer = grille.drip_ring_inner_diameter / 2.0 + grille.drip_ring_wall
    nearest = ring_outer + SLOT_RING_CLEARANCE + slot_width / 2.0
    farthest = grille.size / 2.0 - SLOT_EDGE_BORDER - slot_width / 2.0
    if nearest > farthest:
        reject(f"slot_width {slot_width} leaves no room for the slots between the drip ring and the "
               f"border: the centre line would have to lie between {nearest:.2f} and {farthest:.2f} mm "
               "from the axis", param="slot_width")
    row_offset = (nearest + farthest) / 2.0
    finger_reach = grille.finger_hole_diameter / 2.0 + SLOT_FINGER_CLEARANCE + slot_width / 2.0
    half_run = math.sqrt(finger_reach ** 2 - (grille.finger_hole_offset - row_offset) ** 2)
    return row_offset, 2.0 * half_run + slot_width


def _slot_cutters(axis, z_bottom, height, slot_width, row_offset, slot_length):
    """The row of slots on the four sides of the tower axis."""
    cutters = None
    for turn in range(4):
        slot = extrude(SlotOverall(slot_length, slot_width), amount=height)
        slot = Rot(0, 0, 90.0 * turn) * Pos(0, row_offset, 0) * slot
        cutters = slot if cutters is None else cutters + slot
    return Pos(axis[0], axis[1], z_bottom) * cutters


def _plate(axis, size, z0, thickness, corner_chamfer, edge_chamfer, draft):
    """The square plate, its underside rim chamfered; the top face stays flat for the bed."""
    plate = Pos(axis[0], axis[1], z0) * Box(size, size, thickness, align=(Align.CENTER, Align.CENTER, Align.MIN))
    plate = chamfer(plate.edges().filter_by(Axis.Z), corner_chamfer)
    if draft:
        return plate
    underside_rim = plate.edges().filter_by(
        lambda edge: edge.bounding_box().max.Z < z0 + 0.01)
    return polish(plate, underside_rim, edge_chamfer)


@part
def scale_grille_inner(slot_width=3.0, corner_chamfer=4.0, underside_chamfer=1.0, draft=False):
    """
    Cup grille that sits on the tower's magnets; solid in the middle, drain slots around it.

    slot_width: width of each drain slot
    corner_chamfer: size of the chamfer on the plate's four vertical corners
    underside_chamfer: chamfer along the plate's underside outline
    """
    layout = scale_layout()
    axis_x, axis_y = layout.axis
    seat, tower, grille = layout.seat, layout.tower, layout.grille_inner
    underside = grille.top - grille.thickness

    body = _plate(layout.axis, grille.size, underside, grille.thickness,
                  corner_chamfer, underside_chamfer, draft)

    # Seat boss under the plate, with the strong magnets flush in its lower face.
    boss = scale_seat(tower.top_seat_z, seat.height, up=False)
    boss_bottom = tower.top_seat_z - seat.height
    magnet_height = measured("aimant_hauteur")
    for well_x, well_y in tower.strong_magnets:
        boss -= magnet_well_cutter(
            well_x, well_y, boss_bottom - measured("aimant_puit_fond"), height=magnet_height)
    body += boss

    # Drip ring, so water running under the plate falls outside the chimney.
    ring_outer = grille.drip_ring_inner_diameter / 2.0 + grille.drip_ring_wall
    ring_height = underside - grille.drip_ring_bottom
    ring = Pos(axis_x, axis_y, grille.drip_ring_bottom) * (
        Cylinder(ring_outer, ring_height, align=(Align.CENTER, Align.CENTER, Align.MIN))
        - Cylinder(grille.drip_ring_inner_diameter / 2.0, ring_height, align=(Align.CENTER, Align.CENTER, Align.MIN)))
    body += ring

    # Drain slots and finger holes cut through the plate only.
    row_offset, slot_length = _slot_row(grille, slot_width)
    body -= _slot_cutters(layout.axis, underside - 0.01, grille.thickness + 0.02,
                          slot_width, row_offset, slot_length)
    offset = grille.finger_hole_offset
    for sign in (-1.0, 1.0):
        body -= Pos(axis_x + sign * offset, axis_y - sign * offset, underside - 0.01) * Cylinder(
            grille.finger_hole_diameter / 2.0, grille.thickness + 0.02, align=(Align.CENTER, Align.CENTER, Align.MIN))

    return scale_flip(scale_frame(body))

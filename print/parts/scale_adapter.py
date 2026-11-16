from nurb import *

from system import AMIN, small_magnet_well_cutter, scale_frame, scale_layout, scale_seat

CONE_HALF_ANGLE_RISE = 1.0  # a 90 degree countersink widens by 1 mm per mm of height


def _countersunk_hole(x, y, z_bottom, z_head_top, z_cone_top, clearance_diameter, head_diameter):
    """Clearance hole from `z_bottom` up to the cone that seats a countersunk head.

    The head's top (head_diameter wide) sits at `z_head_top`; the cone keeps
    widening up to `z_cone_top` so the cutter leaves the part cleanly."""
    clearance_radius = clearance_diameter / 2.0
    cone_bottom_z = z_head_top - (head_diameter / 2.0 - clearance_radius) * CONE_HALF_ANGLE_RISE
    cone_top_radius = head_diameter / 2.0 + (z_cone_top - z_head_top) * CONE_HALF_ANGLE_RISE
    shaft = Pos(x, y, z_bottom) * Cylinder(clearance_radius, cone_bottom_z - z_bottom + 0.01, align=(Align.CENTER, Align.CENTER, Align.MIN))
    cone = Pos(x, y, cone_bottom_z) * Cone(clearance_radius, cone_top_radius, z_cone_top - cone_bottom_z, align=(Align.CENTER, Align.CENTER, Align.MIN))
    return shaft + cone


def _through_hole(x, y, z_bottom, z_top, diameter):
    return Pos(x, y, z_bottom) * Cylinder(diameter / 2.0, z_top - z_bottom, align=(Align.CENTER, Align.CENTER, Align.MIN))


def _polish_edges(body, bottom, seat_z):
    """Edges that take the chamfer: the vertical corners where the cylinder
    meets the flat, and the round rim of the top shelf (the straight edge along
    the flat stays sharp: the seat's corner is only 0.5 mm from it)."""
    def keep(edge):
        box = edge.bounding_box()
        if box.min.Z < bottom + 0.01:
            return False
        vertical = box.max.Z - box.min.Z > 1.0
        on_shelf_rim = (abs(box.min.Z - seat_z) < 0.01 and abs(box.max.Z - seat_z) < 0.01
                        and edge.geom_type == GeomType.CIRCLE)
        return vertical or on_shelf_rim
    return body.edges().filter_by(keep)


@part
def scale_adapter(
    slot_clearance=0.3,
    tip_clearance=0.2,
    clamp_hole_diameter=4.5,
    stop_hole_diameter=3.0,
    head_recess=0.2,
    magnet_recess=0.2,
    draft=False,
):
    """
    Block clamped on the loaded end of the load cell; carries the seat for the tower.

    slot_clearance: total play across the cell's width in the slot
    tip_clearance: gap between the cell's free tip and the slot's end face
    clamp_hole_diameter: clearance hole for the M4 countersunk clamp screw
    stop_hole_diameter: hole that gives the hex key access to the stop screw
    head_recess: how far the clamp screw's head sits below the boss's top face
    magnet_recess: how far each magnet sits below the boss's top face
    """
    layout = scale_layout()
    axis_x, axis_y = layout.axis
    cell, adapter, seat = layout.cell, layout.adapter, layout.seat

    body = Pos(axis_x, axis_y, adapter.bottom) * Cylinder(
        adapter.radius, adapter.seat_z - adapter.bottom, align=(Align.CENTER, Align.CENTER, Align.MIN))
    body -= Pos(adapter.flat_x, axis_y, adapter.bottom) * Box(
        adapter.radius, 2.0 * adapter.radius, adapter.seat_z - adapter.bottom, align=(Align.MIN, Align.CENTER, Align.MIN))

    if not draft:
        body = polish(body, _polish_edges(body, adapter.bottom, adapter.seat_z), 1.0)

    body += scale_seat(adapter.seat_z, seat.height)

    # Slot for the cell, open at the bed and through the flat; its ceiling rests on the cell's top.
    slot_x0 = cell.x0 - tip_clearance
    slot_width = cell.section + slot_clearance
    slot_below = 1.0
    body -= Pos(slot_x0, axis_y - slot_width / 2.0, adapter.bottom - slot_below) * Box(
        adapter.flat_x + adapter.radius - slot_x0, slot_width, cell.z1 - adapter.bottom + slot_below, align=AMIN)

    # Clamp screw, from above into the slot, its head just under the boss's top face.
    head_diameter = measured("vis_fraisee_m4_tete")
    body -= _countersunk_hole(
        cell.clamp_x, axis_y, cell.z1 - 0.01, adapter.top - head_recess, adapter.top,
        clamp_hole_diameter, head_diameter)

    # Hex key access to the stop screw in the cell's tip hole.
    body -= _through_hole(cell.stop_x, axis_y, cell.z1 - 0.01, adapter.top, stop_hole_diameter)

    # Weak magnets, flush minus `magnet_recess`.
    well_depth = measured("aimant_petit_hauteur") + magnet_recess
    for well_x, well_y in adapter.weak_magnets:
        body -= small_magnet_well_cutter(
            well_x, well_y, adapter.top - well_depth - measured("aimant_puit_fond"), height=well_depth)

    return scale_frame(body)

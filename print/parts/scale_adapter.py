from nurb import *

from system import AMIN, CMIN, push_out_hole_cutter, small_magnet_well_cutter, scale_flip, scale_frame, scale_layout

CONE_HALF_ANGLE_RISE = 1.0  # a 90 degree countersink widens by 1 mm per mm of height


def _countersunk_hole(x, y, z_bottom, z_head_top, z_cone_top, clearance_diameter, head_diameter):
    """Clearance hole from `z_bottom` up to the cone that seats a countersunk head.

    The head's top (head_diameter wide) sits at `z_head_top`; the cone keeps
    widening up to `z_cone_top` so the cutter leaves the part cleanly."""
    clearance_radius = clearance_diameter / 2.0
    cone_bottom_z = z_head_top - (head_diameter / 2.0 - clearance_radius) * CONE_HALF_ANGLE_RISE
    cone_top_radius = head_diameter / 2.0 + (z_cone_top - z_head_top) * CONE_HALF_ANGLE_RISE
    shaft = Pos(x, y, z_bottom) * Cylinder(clearance_radius, cone_bottom_z - z_bottom + 0.01, align=CMIN)
    cone = Pos(x, y, cone_bottom_z) * Cone(clearance_radius, cone_top_radius, z_cone_top - cone_bottom_z, align=CMIN)
    return shaft + cone


def _body(layout, draft):
    """Cylinder on the tower axis with a flat on its +X side. Chamfered on the
    vertical corners at the flat and round the bottom; the top face, on which
    the tower stands and which prints on the bed, stays sharp."""
    adapter = layout.adapter
    axis_x, axis_y = layout.axis
    height = adapter.top - adapter.bottom
    body = Pos(axis_x, axis_y, adapter.bottom) * Cylinder(adapter.radius, height, align=CMIN)
    body -= Pos(adapter.flat_x, axis_y, adapter.bottom) * Box(
        adapter.radius, 2.0 * adapter.radius, height, align=(Align.MIN, Align.CENTER, Align.MIN))
    if draft:
        return body

    def exposed(edge):
        box = edge.bounding_box()
        vertical = box.max.Z - box.min.Z > 1.0
        return vertical or box.max.Z < adapter.bottom + 0.01
    return polish(body, body.edges().filter_by(exposed), 1.0)


@part
def scale_adapter(
    slot_clearance=0.3,
    tip_clearance=0.2,
    clamp_hole_diameter=4.5,
    head_recess=0.2,
    magnet_recess=0.2,
    dowel_hole_chamfer=0.0,
    push_out_hole=2.5,
    draft=False,
):
    """
    Block clamped on the loaded end of the load cell; the tower stands on its flat top.
    Prints upside down, top face on the bed.

    slot_clearance: total play across the cell's width in the slot
    tip_clearance: gap between the cell's free tip and the slot's end face
    clamp_hole_diameter: clearance hole for the M4 countersunk clamp screw
    head_recess: how far the clamp screw's head sits below the top face
    magnet_recess: how far each magnet sits below the top face
    dowel_hole_chamfer: 45 degree chamfer on the key hole's rim, if the tower's dowel cannot tip out; 0 for none
    push_out_hole: hole on each magnet's axis from the bottom, to push the magnet out with a pin
    """
    layout = scale_layout()
    axis_y = layout.axis[1]
    cell, adapter = layout.cell, layout.adapter
    body = _body(layout, draft)

    # Slot for the cell, open at the bottom and through the flat; its end face
    # locates the adapter along X, its ceiling rests on the cell's top.
    slot_x0 = cell.x0 - tip_clearance
    slot_width = cell.section + slot_clearance
    slot_below = 1.0
    body -= Pos(slot_x0, axis_y - slot_width / 2.0, adapter.bottom - slot_below) * Box(
        adapter.flat_x + adapter.radius - slot_x0, slot_width, cell.z1 - adapter.bottom + slot_below, align=AMIN)

    # Clamp screw, from above into the cell, its head just under the top face.
    body -= _countersunk_hole(
        cell.clamp_x, axis_y, cell.z1 - 0.01, adapter.top - head_recess, adapter.top,
        clamp_hole_diameter, measured("vis_fraisee_m4_tete"))

    # Hex key access to the stop screw in the cell's tip hole; the tower's dowel
    # drops into it, 1 mm deep, which lets the tower tip off the magnets.
    key_radius = adapter.key_hole / 2.0
    body -= Pos(cell.stop_x, axis_y, cell.z1 - 0.01) * Cylinder(
        key_radius, adapter.top - cell.z1 + 0.02, align=CMIN)
    if dowel_hole_chamfer > 0.0:
        body -= Pos(cell.stop_x, axis_y, adapter.top - dowel_hole_chamfer) * Cone(
            key_radius, key_radius + dowel_hole_chamfer + 0.01, dowel_hole_chamfer + 0.01, align=CMIN)

    # Weak magnets, flush minus `magnet_recess`. A pin pushed up the hole under
    # each one, with the adapter off the cell, gets the magnet back out. The
    # floor over the cell is too thin for the whole 45 degree taper: it stops at
    # the cell's top, and the hole goes on at that width through the slot's flank.
    well_depth = measured("aimant_petit_hauteur") + magnet_recess
    for well_x, well_y in adapter.weak_magnets:
        body -= small_magnet_well_cutter(
            well_x, well_y, adapter.top - well_depth - measured("aimant_puit_fond"), height=well_depth)
        body -= push_out_hole_cutter(
            well_x, well_y, adapter.top - well_depth, adapter.bottom, push_out_hole,
            measured("aimant_petit_diametre"), taper_end_z=cell.z1)

    return scale_flip(scale_frame(body))

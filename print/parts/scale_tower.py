from nurb import *

from system import CMIN, magnet_well_cutter, small_magnet_well_cutter, scale_frame, scale_layout, scale_seat

LIGHT_JOINT_GAP = scale_layout().adapter.weak_magnet_gap


def _top_rim(body, top):
    """The outer circular edge of the top face (the only edge that gets a chamfer)."""
    return body.edges().filter_by(
        lambda edge: edge.geom_type == GeomType.CIRCLE
        and abs(edge.bounding_box().min.Z - top) < 0.01
        and edge.bounding_box().size.X > 2.0 * scale_layout().tower.radius - 0.1)


@part
def scale_tower(weak_magnet_gap=LIGHT_JOINT_GAP, adapter_magnet_recess=0.2, draft=False):
    """
    Column that carries the cup grille's load down to the adapter through the tray window.
    Its flat bottom stands on the adapter's flat top; a dowel pressed into it sets its place.

    weak_magnet_gap: distance between the paired magnets of the lower joint; raise it to weaken the joint
    adapter_magnet_recess: how far the adapter's magnets sit below its top face (same value as the adapter's)
    """
    layout = scale_layout()
    axis_x, axis_y = layout.axis
    adapter, dowel, seat, tower = layout.adapter, layout.dowel, layout.seat, layout.tower
    floor_gap = measured("aimant_puit_fond")

    body = Pos(axis_x, axis_y, tower.bottom) * Cylinder(tower.radius, tower.top - tower.bottom, align=CMIN)
    if not draft:
        body = polish(body, _top_rim(body, tower.top), 1.0)

    # The upper socket takes the grille's boss.
    socket_height = seat.height + seat.socket_clearance
    body -= scale_seat(tower.top_seat_z, socket_height, up=False)

    # Lower magnets: wells open at the bottom face, the magnet pushed to the
    # well's end, so the air under it makes up the joint's gap with the
    # adapter's recessed magnets.
    weak_well_depth = measured("aimant_petit_hauteur") + weak_magnet_gap - adapter_magnet_recess
    for well_x, well_y in adapter.weak_magnets:
        body -= small_magnet_well_cutter(well_x, well_y, tower.bottom - floor_gap, height=weak_well_depth)

    # Dowel hole: as deep as the dowel's tower end, so a dowel pushed home stands `proud`.
    body -= Pos(dowel.x, dowel.y, tower.bottom) * Cylinder(dowel.tower_hole / 2.0, dowel.in_tower, align=CMIN)

    # Upper magnets sit flush in the floor of the upper socket, wells going down.
    strong_height = measured("aimant_hauteur")
    socket_floor_z = tower.top_seat_z - socket_height
    for well_x, well_y in tower.strong_magnets:
        body -= magnet_well_cutter(well_x, well_y, socket_floor_z - strong_height - floor_gap, height=strong_height)

    return scale_frame(body)

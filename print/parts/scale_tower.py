from nurb import *

from system import magnet_well_cutter, small_magnet_well_cutter, scale_frame, scale_layout, scale_seat

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

    weak_magnet_gap: distance between the paired magnets of the lower joint; raise it to weaken the joint
    adapter_magnet_recess: how far the adapter's magnets sit below its boss's top face (same value as the adapter's)
    """
    layout = scale_layout()
    axis_x, axis_y = layout.axis
    adapter, seat, tower = layout.adapter, layout.seat, layout.tower
    socket_height = seat.height + seat.socket_clearance
    fond = measured("aimant_puit_fond")

    body = Pos(axis_x, axis_y, tower.bottom) * Cylinder(
        tower.radius, tower.top - tower.bottom, align=(Align.CENTER, Align.CENTER, Align.MIN))
    if not draft:
        body = polish(body, _top_rim(body, tower.top), 1.0)

    # Sockets: the lower one takes the adapter's boss, the upper one the grille's.
    body -= scale_seat(adapter.seat_z, socket_height)
    body -= scale_seat(tower.top_seat_z, socket_height, up=False)

    # Lower magnets sit at the top of wells in the socket's ceiling. The adapter's magnets are
    # recessed 0.2 and the socket leaves its clearance, so the well depth makes up the rest of the gap.
    small_height = measured("aimant_petit_hauteur")
    ceiling_z = adapter.seat_z + socket_height
    weak_well_depth = small_height + weak_magnet_gap - adapter_magnet_recess - seat.socket_clearance
    for well_x, well_y in adapter.weak_magnets:
        body -= small_magnet_well_cutter(well_x, well_y, ceiling_z - fond, height=weak_well_depth)

    # Upper magnets sit flush in the floor of the upper socket, wells going down.
    strong_height = measured("aimant_hauteur")
    floor_z = tower.top_seat_z - socket_height
    for well_x, well_y in tower.strong_magnets:
        body -= magnet_well_cutter(well_x, well_y, floor_z - strong_height - fond, height=strong_height)

    return scale_frame(body)

from nurb import *

from system import CMIN, scale_layout

SIDE_WALL = 1.5


@part
def scale_bumper(grip=0.2, edge_chamfer=0.8, draft=False):
    """
    TPU cap pushed over the tray's rear boss; it stops the tray against the back of the mouth.

    grip: how much smaller the cap's bore is than the boss, so it holds by squeezing
    edge_chamfer: chamfer on the outer rim of the open end
    """
    tray = scale_layout().tray
    bore = tray.bumper_boss_diameter - grip
    if bore >= tray.bumper_boss_diameter:
        reject(f"grip {grip} leaves the cap loose on the Ø{tray.bumper_boss_diameter} boss: "
               "raise it above 0", param="grip")
    outer_radius = bore / 2.0 + SIDE_WALL
    height = tray.bumper_cap + tray.bumper_boss_length
    body = Cylinder(outer_radius, height, align=CMIN)
    body -= Pos(0, 0, tray.bumper_cap) * Cylinder(bore / 2.0, tray.bumper_boss_length, align=CMIN)
    if draft:
        return body
    # The outer rim of the open end; the bed face and the bore stay sharp.
    rim = body.edges().filter_by(
        lambda edge: edge.geom_type == GeomType.CIRCLE
        and edge.bounding_box().min.Z > height - 1e-3
        and edge.radius > outer_radius - 1e-3)
    return polish(body, rim, edge_chamfer)

from nurb import *

from system import AMIN, CMIN, INSERT_M3
from parts.boitier_pid import bb_overall, corbels

def _cadre(x0, y0, x1, y1, ep, z0, h):
    """Rectangular frame, thickness `ep` on every side, from z0 up by h."""
    outer = Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, h, align=AMIN)
    inner = Pos(x0 + ep, y0 + ep, z0 - 0.5) * Box(
        x1 - x0 - 2.0 * ep, y1 - y0 - 2.0 * ep, h + 1.0, align=AMIN
    )
    return outer - inner

@part
def couvercle_pid(
    wall=1.68,
    floor=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.2,
    gouttiere_profondeur=3.0,
    draft=False,
):
    """Couvercle plat de `boitier_pid`, avec un rebord intérieur qui s'appuie
    contre la face intérieure des murs pour l'ajustage.

    wall: épaisseur de la plaque et des murs de boitier_pid
    floor: épaisseur du fond
    gouttiere_jeu: jeu entre le rebord et la face intérieure du mur
    gouttiere_epaisseur: épaisseur du rebord (3 périmètres)
    gouttiere_profondeur: profondeur du rebord dans la cavité
    """
    bb = bb_overall()

    body = Pos(bb.min.X, bb.min.Y, 0) * Box(
        bb.size.X, bb.size.Y, floor, align=AMIN
    )

    jeu = gouttiere_jeu
    ep = gouttiere_epaisseur
    prof = gouttiere_profondeur
    body = body + _cadre(
        bb.min.X + wall + jeu, bb.min.Y + wall + jeu, 
        bb.max.X - wall - jeu, bb.max.Y - wall - jeu, 
        ep, floor, prof
    )

    margin = 0.5
    vis_diametre = 3.0
    for cx, cy, _d, pad in corbels(wall):
        cutter = Pos(pad.min.X - margin, pad.min.Y - margin, floor) * Box(
            pad.size.X + margin*2, pad.size.Y + margin*2, prof + 1.0, align=AMIN
        )
        body = body - cutter
        body = body - Pos(cx, cy, -0.5) * Cylinder(
            vis_diametre / 2.0, wall + 1.0, align=CMIN
        )

    mid_x = bb.center().X
    mirror_x = Plane(origin=(mid_x, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
    body = mirror(body, about=mirror_x)

    if draft:
        return body
    # Name what must stay sharp, then let `polish` chamfer whatever the kernel takes.
    # A bare `chamfer(...)` is all or nothing: one edge that cannot land loses the lot.
    keep = body.edges().filter_by(Axis.Z).filter_by(
        lambda e: (e.bounding_box().min.Y < 0.05) or (bb.max.Y-e.bounding_box().max.Y < 0.05)
    )
    return polish(body, keep, 1.0) if keep else body

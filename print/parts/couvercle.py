from nurb import *

from system import dims, outer_corners


@part
def couvercle(
    longueur=62.0,
    largeur=30.0,
    hauteur=20.0,
    epaisseur_paroi=1.6,
    jeu_couvercle=0.3,
    draft=False,
):
    """Lid that nests into the Wago junction box and screws down on the cable-channel side.

    Printed top-down: the outer face sits on the bed, the inner skirt grows in +Z.
    Screw heads sit on the lid; the two M3 holes line up with the box bosses.

    longueur: inner length, must match the box
    largeur: inner width, must match the box
    hauteur: box height, unused for the solid, kept so the sliders stay in lockstep
    epaisseur_paroi: box wall, used to place the skirt and the screw holes
    jeu_couvercle: per-side clearance between skirt and inner wall
    """
    d = dims(longueur, largeur, hauteur, epaisseur_paroi, jeu_couvercle)
    amin = (Align.MIN, Align.MIN, Align.MIN)
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)

    # Print frame: notch wall at y=0 so a 180° X rotation seats it on the box
    # without flipping the left-hand exits.
    plate = Box(d.outer_x, d.outer_y, d.lid_th, align=amin)

    sx = d.inner_x - 2.0 * d.jeu
    sy = d.inner_y - 2.0 * d.jeu
    ox = d.wall + d.jeu
    oy = d.wall + d.jeu
    # Skirt only on the two walls without wire notches. A full perimeter left
    # 0.12 mm films where the exit slots met the long-wall jupe.
    wago_skirt = Pos(ox, oy + sy - d.jupe_th, d.lid_th) * Box(
        sx, d.jupe_th, d.jupe_h, align=amin
    )
    end_skirt = Pos(ox + sx - d.jupe_th, oy, d.lid_th) * Box(
        d.jupe_th, sy, d.jupe_h, align=amin
    )
    body = plate + wago_skirt + end_skirt

    for x, y_box in d.piliers:
        y = d.outer_y - y_box
        body = body - Pos(x, y, -0.5) * Cylinder(
            d.vis_pass / 2.0, d.lid_th + 0.8, align=cmin
        )

    if draft:
        return body
    bed = body.bounding_box().min.Z
    return polish(body, outer_corners(body, d.outer_x, d.outer_y, bed), 1.0)

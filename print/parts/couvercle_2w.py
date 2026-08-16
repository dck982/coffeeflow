from nurb import *

from system import dims, lid_hold_murets, outer_corners


@part
def couvercle_2w(
    longueur=30.0,
    largeur=32.0,
    hauteur=25.0,
    epaisseur_paroi=1.6,
    jeu_couvercle=0.3,
    draft=False,
):
    """Lid for the two-Wago box. One centred M3 hole over the box boss.

    Printed top-down: the outer face sits on the bed, the inner skirt grows in +Z.
    Screw head sits on the lid; the hole lines up with the centred box boss.
    Hold-down murets face the box dividers; a front bar stops the Wagos.
    Back skirt is split around the square M3 boss so the lid can nest.

    longueur: inner length, must match the box
    largeur: inner width, must match the box
    hauteur: box height, unused for the solid, kept so the sliders stay in lockstep
    epaisseur_paroi: box wall, used to place the skirt and the screw hole
    jeu_couvercle: per-side clearance between skirt and inner wall
    """
    d = dims(longueur, largeur, hauteur, epaisseur_paroi, jeu_couvercle, n=2)
    amin = (Align.MIN, Align.MIN, Align.MIN)
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)

    # Print frame: slotted wall at y=0 so a 180° X rotation seats it on the box
    # without flipping the left-hand end.
    plate = Box(d.outer_x, d.outer_y, d.lid_th, align=amin)

    sx = d.inner_x - 2.0 * d.jeu
    sy = d.inner_y - 2.0 * d.jeu
    ox = d.wall + d.jeu
    oy = d.wall + d.jeu
    by = oy + sy - d.jupe_th
    # Skirt on the three walls without wire slots. The slotted wall stays clear
    # so the three wires per Wago are not pinched by a jupe. Back skirt is cut
    # open over the square M3 boss (flush with the rim); a continuous jupe there
    # hit the pillar and the lid would not close.
    cx0, _cy0, csx, _csy = d.carre
    # Open exactly over the square so the back skirt butts the lid murets cleanly.
    gap0 = cx0
    gap1 = cx0 + csx
    back_left = Pos(ox, by, d.lid_th) * Box(
        max(gap0 - ox, 0.0), d.jupe_th, d.jupe_h, align=amin
    )
    back_right = Pos(gap1, by, d.lid_th) * Box(
        max(ox + sx - gap1, 0.0), d.jupe_th, d.jupe_h, align=amin
    )
    end_a = Pos(ox, oy, d.lid_th) * Box(d.jupe_th, sy, d.jupe_h, align=amin)
    end_b = Pos(ox + sx - d.jupe_th, oy, d.lid_th) * Box(
        d.jupe_th, sy, d.jupe_h, align=amin
    )
    body = plate + back_left + back_right + end_a + end_b + lid_hold_murets(d)

    for x, y_box in d.piliers:
        y = d.outer_y - y_box
        body = body - Pos(x, y, -0.5) * Cylinder(
            d.vis_pass / 2.0, d.lid_th + 0.8, align=cmin
        )

    if draft:
        return body
    bed = body.bounding_box().min.Z
    return polish(body, outer_corners(body, d.outer_x, d.outer_y, bed), 1.0)

from nurb import *

from system import dims, outer_corners, u_cutter


@part
def boitier(
    longueur=62.0,
    largeur=30.0,
    hauteur=20.0,
    epaisseur_paroi=1.6,
    jeu_couvercle=0.3,
    draft=False,
):
    """Junction box for three Wago 221-423 on an espresso chassis.

    longueur: inner length, split into three equal Wago zones
    largeur: inner width, Wago bays plus the cable channel
    hauteur: outer height including the floor
    epaisseur_paroi: outer wall and floor thickness
    jeu_couvercle: per-side clearance for the lid skirt
    """
    d = dims(longueur, largeur, hauteur, epaisseur_paroi, jeu_couvercle)
    amin = (Align.MIN, Align.MIN, Align.MIN)
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)

    body = Box(d.outer_x, d.outer_y, d.hauteur, align=amin)
    cavity = Pos(d.wall, d.wall, d.floor) * Box(
        d.inner_x, d.inner_y, d.hauteur, align=amin
    )
    body = body - cavity

    for x in d.div_x:
        body = body + Pos(x, d.wall, d.floor) * Box(
            d.muret_ep, d.muret_l, d.muret_h, align=amin
        )

    body = body + Pos(d.wall, d.rail_y0, d.floor) * Box(
        d.inner_x, d.rail_ep, d.rail_h, align=amin
    )

    for x, y in d.picots:
        body = body + Pos(x, y, d.floor) * Cylinder(d.picot_r, d.picot_h, align=cmin)

    boss_r = d.puit_r + 1.4
    for x, y in d.puits:
        if d.boss_h > 0.05:
            body = body + Pos(x, y, d.floor) * Cylinder(boss_r, d.boss_h, align=cmin)
        well_z = d.puit_fond
        body = body - Pos(x, y, well_z) * Cylinder(
            d.puit_r, d.aimant_h + 0.2, align=cmin
        )

    for x, y in d.piliers:
        body = body + Pos(x, y, d.floor) * Cylinder(
            d.pilier_r, d.hauteur - d.floor, align=cmin
        )
        body = body - Pos(x, y, d.floor) * Cylinder(
            d.vis_trou / 2.0, d.hauteur - d.floor + 0.4, align=cmin
        )

    notch = u_cutter(d.encoche, d.encoche, d.wall + 4.0)
    z_notch = d.hauteur - d.encoche
    for x in d.entries:
        body = body - Pos(x, d.outer_y - d.wall / 2.0, z_notch) * notch
    end_notch = Rot(0, 0, 90) * notch
    for y in d.exits:
        body = body - Pos(d.wall / 2.0, y, z_notch) * end_notch

    if draft:
        return body
    bed = body.bounding_box().min.Z
    return polish(body, outer_corners(body, d.outer_x, d.outer_y, bed), 1.0)

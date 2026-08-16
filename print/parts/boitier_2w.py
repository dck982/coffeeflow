from nurb import *

from system import dims, outer_corners, u_cutter


@part
def boitier_2w(
    longueur=25.4,
    largeur=32.0,
    hauteur=25.0,
    epaisseur_paroi=1.6,
    jeu_couvercle=0.3,
    draft=False,
):
    """Junction box for two standing Wago 221-423 at the ends. Square M3 block, one magnet.

    longueur: inner length; middle gap is the square boss (min 6.5 mm)
    largeur: inner width, Wago depth plus the magnet channel
    hauteur: outer height including the floor
    epaisseur_paroi: outer wall and floor thickness
    jeu_couvercle: per-side clearance for the lid skirt
    """
    d = dims(longueur, largeur, hauteur, epaisseur_paroi, jeu_couvercle, n=2)
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

    for y in d.rail_y:
        body = body + Pos(d.wall, y, d.floor) * Box(
            d.inner_x, d.rail_ep, d.rail_h, align=amin
        )

    # Raised channel floor so the 3 mm magnet sits on 0.6 mm.
    body = body + Pos(d.wall, d.channel_y0, d.floor) * Box(
        d.inner_x, d.canal, d.boss_h, align=amin
    )
    for x, y in d.puits:
        body = body - Pos(x, y, d.puit_fond) * Cylinder(
            d.puit_r, d.well_h, align=cmin
        )

    # Square M3 block fills the middle gap, bonded to both murets and the back wall.
    x0, y0, sx, sy = d.carre
    body = body + Pos(x0, y0, d.floor) * Box(
        sx, sy, d.hauteur - d.floor, align=amin
    )
    for x, y in d.piliers:
        body = body - Pos(x, y, d.floor) * Cylinder(
            d.vis_trou / 2.0, d.hauteur - d.floor + 0.4, align=cmin
        )

    notch = u_cutter(d.fente_w, d.fente_h, d.wall + 4.0)
    z_notch = d.fente_bottom
    for x in d.fentes:
        body = body - Pos(x, d.outer_y - d.wall / 2.0, z_notch) * notch

    if draft:
        return body
    bed = body.bounding_box().min.Z
    return polish(body, outer_corners(body, d.outer_x, d.outer_y, bed), 1.0)

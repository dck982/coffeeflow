from nurb import *

from system import dims, m3_nut_trap, outer_corners, u_cutter


@part
def boitier_2w(
    longueur=30.0,
    largeur=32.0,
    hauteur=25.0,
    epaisseur_paroi=1.6,
    jeu_couvercle=0.3,
    seuil_canal=4.0,
    draft=False,
):
    """Junction box for two standing Wago 221-423 at the ends. Square M3 nut trap.

    longueur: inner length; middle gap is the square boss (min 11 mm for the nut)
    largeur: inner width, Wago depth plus the magnet channel
    hauteur: outer height including the floor
    epaisseur_paroi: outer wall and floor thickness
    jeu_couvercle: per-side clearance for the lid skirt
    seuil_canal: raised magnet-channel floor; 2 mm above the rails holds the Wagos
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

    # Raised channel floor (default 4 mm = 2 mm lip above the rails). Magnet
    # still sits on 0.6 mm; well depth tracks the threshold. 4w stays at 3 mm.
    well_h = d.floor + seuil_canal - d.puit_fond + 0.2
    body = body + Pos(d.wall, d.channel_y0, d.floor) * Box(
        d.inner_x, d.canal, seuil_canal, align=amin
    )
    for x, y in d.puits:
        body = body - Pos(x, y, d.puit_fond) * Cylinder(
            d.puit_r, well_h, align=cmin
        )

    # Square fills the middle gap, bonded to both murets and the back wall.
    # Captive M3 nut from the bed: tip of an M3×10 through the 1.6 mm lid reaches
    # z = hauteur + lid - 10; seat the nut so the tip clears its bottom.
    x0, y0, sx, sy = d.carre
    body = body + Pos(x0, y0, d.floor) * Box(
        sx, sy, d.hauteur - d.floor, align=amin
    )
    tip_z = d.hauteur + d.lid_th - d.vis_longueur
    shoulder = tip_z + d.ecrou_ep + 0.5
    for x, y in d.piliers:
        body = body - Pos(x, y, 0) * m3_nut_trap(
            d.vis_pass,
            d.ecrou_plats,
            d.ecrou_ep,
            shoulder,
            d.hauteur + 1.0,
        )

    notch = u_cutter(d.fente_w, d.fente_h, d.wall + 4.0)
    z_notch = d.fente_bottom
    for x in d.fentes:
        body = body - Pos(x, d.outer_y - d.wall / 2.0, z_notch) * notch

    if draft:
        return body
    bed = body.bounding_box().min.Z
    return polish(body, outer_corners(body, d.outer_x, d.outer_y, bed), 1.0)

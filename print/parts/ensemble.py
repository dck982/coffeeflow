from nurb import *

from system import dims


@assembly
def ensemble(ouvert=0.0, longueur=62.0, largeur=30.0, hauteur=20.0, epaisseur_paroi=1.6):
    """Box and lid together, with Wagos and the espresso chassis as context.

    ouvert: how far the lid is lifted, in mm, to look inside
    longueur: inner length, passed to both parts
    largeur: inner width, passed to both parts
    hauteur: box outer height
    epaisseur_paroi: wall thickness, passed to both parts
    """
    d = dims(float(longueur), float(largeur), float(hauteur), float(epaisseur_paroi))
    lift = float(ouvert)
    kw = dict(
        longueur=float(longueur),
        largeur=float(largeur),
        hauteur=float(hauteur),
        epaisseur_paroi=float(epaisseur_paroi),
    )
    box = use("boitier", **kw)
    lid = (
        Pos(0, d.outer_y, d.hauteur + d.lid_th + lift)
        * Rot(180, 0, 0)
        * use("couvercle", **kw)
    )

    wagos = []
    for i, x in enumerate(d.bays_x):
        w = Pos(x, d.bay_cy, d.floor + d.picot_h) * Box(
            d.wago_x, d.wago_y, d.wago_z, align=(Align.CENTER, Align.CENTER, Align.MIN)
        )
        wagos.append(obstacle(w, f"Wago 221-423 #{i + 1}"))

    chassis = obstacle(
        Pos(d.outer_x / 2.0, d.outer_y / 2.0, 0)
        * Box(90.0, 50.0, 2.0, align=(Align.CENTER, Align.CENTER, Align.MAX)),
        "espresso chassis",
    )
    return box, lid, chassis, *wagos

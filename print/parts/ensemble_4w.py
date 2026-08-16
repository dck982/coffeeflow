from nurb import *

from system import dims


@assembly
def ensemble_4w(
    ouvert=1.0,
    longueur=37.0,
    largeur=32.0,
    hauteur=25.0,
    epaisseur_paroi=1.6,
):
    """Four-Wago box and lid, with standing Wagos and the espresso chassis.

    ouvert: 0 closed, 1 lid lifted 40 mm so the bays are visible
    longueur: inner length, passed to both parts
    largeur: inner width, passed to both parts
    hauteur: box outer height
    epaisseur_paroi: wall thickness, passed to both parts
    """
    d = dims(
        float(longueur),
        float(largeur),
        float(hauteur),
        float(epaisseur_paroi),
        n=4,
    )
    lift = float(ouvert) * 40.0
    kw = dict(
        longueur=float(longueur),
        largeur=float(largeur),
        hauteur=float(hauteur),
        epaisseur_paroi=float(epaisseur_paroi),
    )
    box = use("boitier_4w", **kw)
    lid = (
        Pos(0, d.outer_y, d.hauteur + d.lid_th + lift)
        * Rot(180, 0, 0)
        * use("couvercle_4w", **kw)
    )

    wagos = []
    for i, x in enumerate(d.bays_x):
        w = Pos(x, d.bay_cy, d.floor + d.rail_h) * Box(
            d.wago_x, d.wago_y, d.wago_z, align=(Align.CENTER, Align.CENTER, Align.MIN)
        )
        wagos.append(obstacle(w, f"Wago 221-423 #{i + 1}"))

    chassis = obstacle(
        Pos(d.outer_x / 2.0, d.outer_y / 2.0, 0)
        * Box(90.0, 50.0, 2.0, align=(Align.CENTER, Align.CENTER, Align.MAX)),
        "espresso chassis",
    )
    return box, lid, chassis, *wagos

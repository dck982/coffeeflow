from nurb import *

from system import outer_corners


@part
def ac_box(
    longueur=75.5,
    largeur=115.0,
    hauteur=30.0,
    epaisseur_paroi=1.6,
    epaisseur_fond=1.6,
    encoche_largeur=10.0,
    encoche_hauteur=10.0,
    draft=False,
):
    """Plateau ouvert pour l'Atom Control. Cotes hors-tout, parois comprises.

    longueur: outer size in X, walls included
    largeur: outer size in Y, walls included
    hauteur: outer height, floor included
    epaisseur_paroi: thickness of the four side walls
    epaisseur_fond: thickness of the floor
    encoche_largeur: cable notch width in X, on the far-Y wall at max X
    encoche_hauteur: how far the cable notch cuts down from the rim
    """
    amin = (Align.MIN, Align.MIN, Align.MIN)

    body = Box(longueur, largeur, hauteur, align=amin)
    cavity = Pos(epaisseur_paroi, epaisseur_paroi, epaisseur_fond) * Box(
        longueur - 2.0 * epaisseur_paroi,
        largeur - 2.0 * epaisseur_paroi,
        hauteur,
        align=amin,
    )
    body = body - cavity

    # Cable notch: far-Y wall, right-hand end (max X), open at the rim. Once the
    # box is flipped 180° about Y it lands on the bed, which is the point.
    # Held back by one wall thickness so the long side wall stays untouched:
    # the notch stops flush with that wall's inner face.
    body = body - Pos(
        longueur - epaisseur_paroi - encoche_largeur,
        largeur - epaisseur_paroi - 1.0,
        hauteur - encoche_hauteur,
    ) * Box(
        encoche_largeur,
        epaisseur_paroi + 2.0,
        encoche_hauteur + 1.0,
        align=amin,
    )

    if draft:
        return body
    bed = body.bounding_box().min.Z
    return polish(body, outer_corners(body, longueur, largeur, bed), 1.0)

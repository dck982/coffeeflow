from nurb import *

from system import outer_corners


@part
def boitier_ac(
    longueur=90.0,
    largeur=50.0,
    hauteur=10.0,
    epaisseur_paroi=1.6,
    draft=False,
):
    """Bac AC : dalle 90 × 50 mm hors-tout, murs 1,6 mm vers l'intérieur.

    longueur: cote externe en X (le grand côté)
    largeur: cote externe en Y (le petit côté)
    hauteur: hauteur hors-tout depuis le lit
    epaisseur_paroi: épaisseur du fond et des murs, vers l'intérieur
    """
    wall = epaisseur_paroi
    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    if longueur < 2.0 * wall + 4.0:
        reject(
            f"longueur {longueur} leaves under 4 mm of cavity: raise it above "
            f"{2.0 * wall + 4.0:.1f}",
            param="longueur",
        )
    if largeur < 2.0 * wall + 4.0:
        reject(
            f"largeur {largeur} leaves under 4 mm of cavity: raise it above "
            f"{2.0 * wall + 4.0:.1f}",
            param="largeur",
        )
    if hauteur < wall + 2.0:
        reject(
            f"hauteur {hauteur} leaves under 2 mm of wall above a {wall} mm floor: "
            f"raise it above {wall + 2.0:.1f}",
            param="hauteur",
        )

    amin = (Align.MIN, Align.MIN, Align.MIN)
    body = Box(longueur, largeur, hauteur, align=amin)
    body = body - (
        Pos(wall, wall, wall)
        * Box(longueur - 2.0 * wall, largeur - 2.0 * wall, hauteur, align=amin)
    )

    if draft:
        return body

    bed = body.bounding_box().min.Z
    return polish(body, outer_corners(body, longueur, largeur, bed), 1.0)

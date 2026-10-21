from nurb import *

from system import couvercle_corps, polish_couvercle, AMIN
from parts.boitier_dc import dc_contour, corbels

@part
def couvercle_dc(
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.0,
    gouttiere_profondeur=3.0,
    draft=False,
):
    """Couvercle plat de `boitier_dc`, avec un rebord intérieur qui s'appuie
    contre la face intérieure des murs.

    epaisseur_paroi: épaisseur des murs de boitier_dc
    epaisseur_fond: épaisseur de la plaque
    gouttiere_jeu: jeu entre le rebord et la face intérieure du mur
    gouttiere_epaisseur: épaisseur du rebord
    gouttiere_profondeur: profondeur du rebord dans la cavité
    """
    if gouttiere_jeu < 0.1:
        reject(f"gouttiere_jeu {gouttiere_jeu} is under 0.1 mm: raise it", param="gouttiere_jeu")
    if gouttiere_epaisseur < 0.8:
        reject(
            f"gouttiere_epaisseur {gouttiere_epaisseur} is under 0.8 mm: raise it",
            param="gouttiere_epaisseur",
        )

    pts, bb = dc_contour(epaisseur_paroi)
    openings = [
        (bb.min.X,bb.min.Y+35,15,20),
        (bb.max.X-10,bb.min.Y+45,10,10),
        (bb.min.X+22,bb.min.Y+10,5,5)
    ]
    extra_v_rims = [
        (bb.center().X+5,bb.center().Y-5,30),
        (bb.center().X,bb.center().Y-5,30)
    ]
    body = couvercle_corps(
        pts, corbels(epaisseur_paroi), epaisseur_paroi, epaisseur_fond,
        gouttiere_jeu, gouttiere_epaisseur, gouttiere_profondeur,
        openings, extra_v_rims
    )

    if draft:
        return body
    return polish_couvercle(body, epaisseur_fond, gouttiere_profondeur)

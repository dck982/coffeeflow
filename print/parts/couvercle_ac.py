from nurb import *

from system import AMIN, couvercle_corps, polish_couvercle
from parts.boitier_ac import ac_top_contour, corbels


@part
def couvercle_ac(
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.0,
    gouttiere_profondeur=3.0,
    appui_pcb_hauteur=6.45,
    appui_pcb_largeur=3.0,
    draft=False,
):
    """Couvercle plat de `boitier_ac`, avec un rebord intérieur qui s'appuie
    contre la face intérieure des murs et un appui qui presse le PCB.

    epaisseur_paroi: épaisseur des murs de boitier_ac
    epaisseur_fond: épaisseur de la plaque
    gouttiere_jeu: jeu entre le rebord et la face intérieure du mur
    gouttiere_epaisseur: épaisseur du rebord
    gouttiere_profondeur: profondeur du rebord dans la cavité
    appui_pcb_hauteur: hauteur de l'appui qui presse le PCB du module sud, depuis la plaque
    appui_pcb_largeur: largeur (en Y) de l'appui contre le mur nord
    """
    wall = epaisseur_paroi
    floor = epaisseur_fond
    if gouttiere_jeu < 0.1:
        reject(f"gouttiere_jeu {gouttiere_jeu} is under 0.1 mm: raise it", param="gouttiere_jeu")
    if gouttiere_epaisseur < 0.8:
        reject(
            f"gouttiere_epaisseur {gouttiere_epaisseur} is under 0.8 mm: raise it",
            param="gouttiere_epaisseur",
        )
    if appui_pcb_hauteur <= 0.0:
        reject(
            f"appui_pcb_hauteur {appui_pcb_hauteur} is not positive: raise it",
            param="appui_pcb_hauteur",
        )

    pts = ac_top_contour()
    west_x, east_x = pts[0][0], pts[1][0]
    north_y = pts[2][1]

    body = couvercle_corps(
        pts, corbels(wall), wall, floor,
        gouttiere_jeu, gouttiere_epaisseur, gouttiere_profondeur,
    )

    # Middle third of the north rim presses the PCB; added after the mirror,
    # which leaves the centred third where it is.
    third = (east_x - west_x) / 3.0
    press_y1 = north_y - (wall + gouttiere_jeu)
    body = body + Pos(west_x + third, press_y1 - appui_pcb_largeur, floor) * Box(
        third, appui_pcb_largeur, appui_pcb_hauteur, align=AMIN
    )

    if draft:
        return body
    return polish_couvercle(body, floor, gouttiere_profondeur)

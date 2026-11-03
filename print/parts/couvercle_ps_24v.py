from nurb import *

from system import AMIN, CMIN, couvercle_corps, polish_couvercle
from parts.boitier_ps_24v import reperes


def _cadre_alim(d, wall, floor, rebord_x, hauteur_murets):
    """Murets hanging from the plate around the top of the supply: north and
    south against its ends, from the west rim, east along its open side.
    Press-fit, no gap."""
    x0, x1 = rebord_x, d["alim_x1"]
    y0, y1 = d["alim_y0"], d["alim_y1"]
    murets = (
        (x0, y0 - wall, x1 + wall - x0, wall),
        (x0, y1, x1 + wall - x0, wall),
        (x1, y0 - wall, wall, y1 - y0 + 2 * wall),
    )
    return [Pos(x, y, floor) * Box(dx, dy, hauteur_murets, align=AMIN)
            for x, y, dx, dy in murets]


def _pions(d, floor, air_couvercle, cote_appui, diametre_pions, longueur_pions):
    """A square pad pressing on the supply's upper flank at each M3 hole, 3.5 mm
    from its north-east and south-east corners, and a pin into the hole."""
    coin = measured("alim24_trou_coin")
    x = d["alim_x1"] - coin
    solids = []
    for y in (d["alim_y0"] + coin, d["alim_y1"] - coin):
        solids.append(Pos(x, y, floor) * Box(cote_appui, cote_appui, air_couvercle, align=CMIN))
        solids.append(Pos(x, y, floor + air_couvercle)
                      * Cylinder(diametre_pions / 2.0, longueur_pions, align=CMIN))
    return solids


@part
def couvercle_ps_24v(
    epaisseur_fond=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.0,
    gouttiere_profondeur=3.0,
    hauteur_murets=4.0,
    cote_appui=5.0,
    diametre_pions=2.4,
    longueur_pions=2.0,
    draft=False,
):
    """Couvercle de boitier_ps_24v, imprimé à l'envers, miroir en X. Un cadre
    de murets et deux pions tiennent l'alimentation par le haut.

    epaisseur_fond: épaisseur de la plaque
    gouttiere_jeu: jeu entre le rebord et la face intérieure des murs
    gouttiere_epaisseur: épaisseur du rebord
    gouttiere_profondeur: profondeur du rebord dans le boîtier
    hauteur_murets: murets autour du haut de l'alimentation, nord, sud et est
    cote_appui: côté des appuis carrés posés sur le flanc de l'alimentation
    diametre_pions: pions dans les trous M3 taraudés, sous le diamètre du filet
    longueur_pions: dépassement des pions sous les appuis
    """
    d, contour, trous, wall = reperes()
    floor = epaisseur_fond
    jeu = gouttiere_jeu
    air = d["hauteur"] - d["alim_z1"]
    if jeu < 0.1:
        reject(f"gouttiere_jeu {jeu} is under 0.1 mm: raise it", param="gouttiere_jeu")
    if gouttiere_epaisseur < 0.8:
        reject(f"gouttiere_epaisseur {gouttiere_epaisseur} is under 0.8 mm: raise it",
               param="gouttiere_epaisseur")
    if diametre_pions > 2.5:
        reject(f"diametre_pions {diametre_pions} does not enter an M3 tapped hole "
               f"(about 2.5 mm across the thread): lower it", param="diametre_pions")
    if hauteur_murets <= air:
        reject(f"hauteur_murets {hauteur_murets} does not reach the supply, "
               f"{air:.1f} mm under the lid: raise it", param="hauteur_murets")

    # The west rim would bite 0.3 mm into the supply's aluminium plate.
    rim_gap = (0.0, d["alim_y0"], jeu + gouttiere_epaisseur + 0.5, d["alim_y1"])
    # Everything is modelled in the box frame and mirrored by couvercle_corps,
    # so the supply holders are mirrored before being added.
    xs = [x for x, _ in contour]
    mid_x = (min(xs) + max(xs)) / 2.0
    miroir = Plane(origin=(mid_x, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))

    body = couvercle_corps(
        contour, trous, wall, floor, jeu, gouttiere_epaisseur, gouttiere_profondeur,
        rim_gaps=(rim_gap,),
    )
    for solid in (_cadre_alim(d, wall, floor, jeu + gouttiere_epaisseur, hauteur_murets)
                  + _pions(d, floor, air, cote_appui, diametre_pions, longueur_pions)):
        body = body + mirror(solid, about=miroir)
    if draft:
        return body
    return polish_couvercle(body, floor, gouttiere_profondeur)

from nurb import *

# Assembly of wagox5 in wago_slide. Rebuilds when either sibling changes.


@assembly
def wago_montage(
    largeur_borne=30.0,
    insertion=0.0,
    epaisseur_plancher=2.0,
    epaisseur_plaque=4.0,
    recul_serre_cable=5.0,
):
    """Le logement glissé dans les U de la glissière. insertion 0 = assis, 1 = sorti de 30 mm.

    largeur_borne: 30 = 221-415, 18.8 = 221-423 — passé aux deux pièces
    insertion: 0 le logement est enfoncé contre la plaque, 1 il est sorti vers l'avant
    epaisseur_plancher: fond des U, le logement pose dessus
    epaisseur_plaque: plaque de la glissière, pour caler le fond du logement
    recul_serre_cable: distance plaque → fentes du serre-câble, passé à la glissière (1 à 10 mm)
    """
    if insertion < 0.0:
        reject("insertion cannot be negative", param="insertion")
    if insertion > 1.5:
        reject("insertion 1 is fully out; above 1.5 it flies off", param="insertion")

    w = float(largeur_borne)
    y0 = float(epaisseur_plancher)
    z0 = float(epaisseur_plaque)
    recul = float(recul_serre_cable)
    slide = use(
        "wago_slide",
        largeur_borne=w,
        epaisseur_plancher=y0,
        epaisseur_plaque=z0,
        recul_serre_cable=recul,
    )
    holder = use("wagox5", largeur_borne=w)

    # wagox5 is modelled standing on its base (+Z). In the slide the base lies
    # on the shelf (print +Y) and the wings slide along +Z toward the plaque.
    # Rot(-90,0,0): holder +Z -> +Y, holder +Y -> -Z (back against the plaque).
    epaisseur = measured("wago_epaisseur")
    jeu = 0.3
    jupe = 1.2
    log_y = 2.0 * epaisseur + jeu + 2.0 * jupe
    seated_z = z0 + log_y / 2.0
    holder = (
        Pos(0, y0, seated_z + float(insertion) * 30.0)
        * Rot(-90, 0, 0)
        * holder
    )
    return slide, holder

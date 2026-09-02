from nurb import *


@assembly
def ensemble_boitiers(
    hauteur=10.0,
    epaisseur_paroi=1.6,
    decrochage=11.0,
):
    """DC et AC en place, même repère que l'ancien boitier_int.

    hauteur: hauteur hors-tout, passée aux deux pièces
    epaisseur_paroi: murs et fond, passés aux deux pièces
    decrochage: DC nord +11 mm, AC sud-ouest reculé d'autant sur la largeur du DC
    """
    h = float(hauteur)
    w = float(epaisseur_paroi)
    d = float(decrochage)
    return (
        use("boitier_dc", hauteur=h, epaisseur_paroi=w, prolongement_nord=d),
        use("boitier_ac", hauteur=h, epaisseur_paroi=w, decrochage=d),
    )

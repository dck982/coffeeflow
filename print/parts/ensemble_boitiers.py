from nurb import *


@assembly
def ensemble_boitiers(
    hauteur_ac=30.0,
    epaisseur_paroi=1.6,
):
    """DC et AC en place, même repère que l'ancien boitier_int.

    hauteur_ac: murs de l'AC ; le DC reste à 10 mm
    epaisseur_paroi: murs et fond, passés aux deux pièces
    """
    w = float(epaisseur_paroi)
    return (
        use("boitier_dc", epaisseur_paroi=w),
        use("boitier_ac", hauteur=float(hauteur_ac), epaisseur_paroi=w),
    )

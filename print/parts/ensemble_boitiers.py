from nurb import *


@assembly
def ensemble_boitiers(
    hauteur=10.0,
    epaisseur_paroi=1.6,
    prolongement_est=5.0,
    decrochage=5.0,
):
    """DC et AC en place, même repère que l'ancien boitier_int.

    hauteur: hauteur hors-tout, passée aux deux pièces
    epaisseur_paroi: murs et fond, passés aux deux pièces
    prolongement_est: face est du DC, et départ du palier AC
    decrochage: palier sud de l'AC, à l'est du DC
    """
    h = float(hauteur)
    w = float(epaisseur_paroi)
    est = float(prolongement_est)
    d = float(decrochage)
    return (
        use("boitier_dc", hauteur=h, epaisseur_paroi=w, prolongement_est=est),
        use(
            "boitier_ac",
            hauteur=h,
            epaisseur_paroi=w,
            prolongement_est=est,
            decrochage=d,
        ),
    )

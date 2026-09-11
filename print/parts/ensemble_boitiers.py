from nurb import *

from system import AMIN
from parts.boitier_dc import canpal_bb 

def can_pal_plane(dc, z0, w):
    bb_cp = canpal_bb(z0, w)

    module_w = measured("can_pal_width")
    module_x = (bb_cp.min.X + bb_cp.max.X)/2.0 - module_w/2.0
    module_l = measured("can_pal_length")
    return Pos(module_x, bb_cp.min.Y + w, bb_cp.max.Z) * Box(module_w, module_l, 1.0, align=AMIN)

@assembly
def ensemble_boitiers(
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6
):
    """DC et AC en place, même repère que l'ancien boitier_int.

    epaisseur_paroi: murs, passés aux deux pièces
    epaisseur_fond: epaisseur du fond
    """
    w = float(epaisseur_paroi)
    dc = use("boitier_dc", epaisseur_paroi=w)
    ac = use("boitier_ac", epaisseur_paroi=w)

    can_pal = obstacle(can_pal_plane(dc, epaisseur_fond, w), name="Adafruit CAN Pal")

    return (dc, ac, can_pal)

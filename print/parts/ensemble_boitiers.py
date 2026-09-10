from nurb import *

@assembly
def ensemble_boitiers(
    epaisseur_paroi=1.68,
):
    """DC et AC en place, même repère que l'ancien boitier_int.

    epaisseur_paroi: murs et fond, passés aux deux pièces
    """
    w = float(epaisseur_paroi)
    dc = use("boitier_dc", epaisseur_paroi=w)
    ac = use("boitier_ac", epaisseur_paroi=w)
    # boitier_ac's own outer contour lands its west face a few mm past
    # `aile_x` (wall-thickness margin baked into its shape). Slide the whole
    # solid east so that face sits flush against boitier_dc's east wall
    # instead of overlapping it: recompute the shift from AC's own bounding
    # box so this stays correct if boitier_ac's margin ever changes.
    aile_x = measured("boitier_int_aile_x")
    ac_shift = aile_x - ac.bounding_box().min.X
    ac = Pos(ac_shift, 0, 0) * ac
    return (dc, ac)

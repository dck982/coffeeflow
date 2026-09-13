from nurb import *

from system import AMIN
from parts.boitier_dc import canpal_bb, magnet_spacer_pins as dc_spacer_pins
from parts.boitier_ac import magnet_spacer_pins as ac_spacer_pins


def can_pal_plane(dc, z0, w):
    bb_cp = canpal_bb(z0, w)

    module_w = measured("can_pal_width")
    module_x = (bb_cp.min.X + bb_cp.max.X)/2.0 - module_w / 2.0
    module_l = measured("can_pal_length")
    return Pos(module_x, bb_cp.min.Y + w, bb_cp.max.Z) * Box(module_w, module_l, 1.0, align=AMIN)


def _seat_spacer(spacer, z0, pz):
    """Drop a bed-printed spacer under the boxes: XY already in box frame."""
    h = spacer.bounding_box().size.Z
    return Pos(0, 0, -h + z0 + pz) * spacer


@assembly
def ensemble_boitiers(
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    open=10.0,
):
    """DC et AC en place, même repère que l'ancien boitier_int.

    epaisseur_paroi: murs, passés aux deux pièces et au couvercle
    epaisseur_fond: epaisseur du fond
    open: décalage Z du couvercle en mm, 0 fermé, 10 pour voir dans le bac
    """
    w = float(epaisseur_paroi)
    z0 = float(epaisseur_fond)
    open = float(open)

    if open < 0.0:
        reject(f"open {open} is negative: raise it to 0 or more", param="open")

    dc = use("boitier_dc", epaisseur_paroi=w)
    ac = use("boitier_ac", epaisseur_paroi=w)

    spacer_dc = _seat_spacer(
        use("magnet_spacer_dc", wall=w), z0, dc_spacer_pins(w)[0][2]
    )
    spacer_ac = _seat_spacer(
        use("magnet_spacer_ac", wall=w), z0, ac_spacer_pins(w)[0][2]
    )

    # Print orientation is plate on the bed, rim +Z, already mirrored in X so
    # a north-south flip lands west on west. Rotate 180 about Y to hang the
    # rim into the cavity, then lift by the box height plus `open`.
    lid = use("couvercle_acdc", epaisseur_paroi=w, epaisseur_fond=z0)
    mid_x = measured("boitier_int_x") / 2.0
    z_top = dc.bounding_box().max.Z
    lid = (
        Pos(0, 0, z_top + z0 + open)
        * Pos(mid_x, 0, 0)
        * Rot(0, 180, 0)
        * Pos(-mid_x, 0, 0)
        * lid
    )

    can_pal = obstacle(can_pal_plane(dc, z0, w), name="Adafruit CAN Pal")

    return (dc, ac, spacer_dc, spacer_ac, lid, can_pal)

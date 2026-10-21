from nurb import *

from system import AMIN
from parts.boitier_ac import magnet_spacer_pins as ac_spacer_pins


def _seat_spacer(spacer, z0, pz):
    """Drop a bed-printed spacer under the boxes: XY already in box frame."""
    h = spacer.bounding_box().size.Z
    return Pos(0, 0, -h + z0 + pz) * spacer


def _flip_lid(lid, box, z0, open):
    """Print orientation is plate on the bed, rim +Z, already mirrored in X so
    a north-south flip lands west on west. Rotate 180 about Y to hang the rim
    into the cavity, then lift by the box height plus `open`."""
    mid_x = lid.bounding_box().center().X
    return (
        Pos(0, 0, box.bounding_box().max.Z + z0 + open)
        * Pos(mid_x, 0, 0)
        * Rot(0, 180, 0)
        * Pos(-mid_x, 0, 0)
        * lid
    )

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

    dc = Pos(0,0,-4) * use("boitier_dc", epaisseur_paroi=w)
    ac = use("boitier_ac", epaisseur_paroi=w)

    spacer_ac = _seat_spacer(
        use("magnet_spacer_ac", wall=w), z0, ac_spacer_pins(w)[0][2]
    )

    dc_lid = _flip_lid(use("couvercle_dc", epaisseur_paroi=w, epaisseur_fond=z0), dc, z0, open)
    ac_lid = _flip_lid(use("couvercle_ac", epaisseur_paroi=w, epaisseur_fond=z0), ac, z0, open)

    return (dc, ac, spacer_ac, dc_lid, ac_lid)

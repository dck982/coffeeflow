from nurb import *

from parts.boitier_pid import bb_overall


@assembly
def ensemble_boitier_pid(open=10.0):
    """Boîtier PID et son couvercle, même repère.

    open: décalage Z du couvercle en mm, 0 fermé, 10 pour voir dans le bac
    """
    open = float(open)
    if open < 0.0:
        reject(f"open {open} is negative: raise it to 0 or more", param="open")

    box = use("boitier_pid")
    lid = use("couvercle_pid")

    # Print orientation is plate on the bed, rim +Z, already mirrored in X so
    # a north-south flip lands west on west. Rotate 180 about Y to hang the
    # rim into the cavity, then lift by the box rim plus `open`.
    # The cavity rim is `hauteur` from the bed (40). bbox.max.Z is 41.6 because
    # the north channel wall stands `hauteur` on `epaisseur_fond`.
    mid_x = bb_overall().size.X / 2.0
    z_top = box.bounding_box().size.Z
    plate = 1.6
    lid = (
        Pos(0, 0, z_top + plate + open)
        * Pos(mid_x, 0, 0)
        * Rot(0, 180, 0)
        * Pos(-mid_x, 0, 0)
        * lid
    )
    return (box, lid)

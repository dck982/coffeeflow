from nurb import *

from system import AMIN
from parts.boitier_ac import bb_overall as ac_overall


def _flip_lid(lid, plate_thickness, top, open):
    """Print orientation is plate on the bed, rim +Z, mirrored in X. Rotate 180
    about Y around its own X centre to hang the rim into the boxes, then lift
    the plate onto the wall tops plus `open`."""
    mid_x = lid.bounding_box().center().X
    return (
        Pos(0, 0, top + plate_thickness + open)
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
    """boitier_dc sur la face de la machine, boitier_ac sur la plaque métal
    4 mm plus haut, la carte combined vissée sur les deux, et le couvercle.

    epaisseur_paroi: murs, passés aux deux boîtiers et au couvercle
    epaisseur_fond: épaisseur du fond des deux boîtiers et de la plaque du couvercle
    open: décalage Z du couvercle en mm, 0 fermé, 10 pour voir dedans
    """
    w = float(epaisseur_paroi)
    floor = float(epaisseur_fond)
    open = float(open)
    if open < 0.0:
        reject(f"open {open} is negative: raise it to 0 or more", param="open")

    plate = measured("plaque_machine_epaisseur")
    dc = Pos(0, 0, -plate) * use("boitier_dc", epaisseur_paroi=w, epaisseur_fond=floor)
    ac = use("boitier_ac", epaisseur_paroi=w, epaisseur_fond=floor)
    top = ac.bounding_box().max.Z
    lid = _flip_lid(
        use("couvercle_acdc", epaisseur_paroi=w, epaisseur_fond=floor), floor, top, open)
    board = Pos(w, w, floor + measured("combined_dessous_pcb")) * use("combined")

    # The metal plate under boitier_ac: 35 mm from its north edge, wider than both boxes.
    ac_bb = ac_overall(w)
    plate_length = measured("combined_ac_profondeur")
    metal = Pos(-10, ac_bb.max.Y - plate_length, -plate) * Box(
        ac_bb.max.X + 20, plate_length, plate, align=AMIN)

    return (
        dc, ac, lid,
        obstacle(board, "carte combined : contour, trous et composants hauts en boîtes"),
        obstacle(metal, "plaque métal de la machine, 35 mm × 4 mm, largeur indicative"),
    )

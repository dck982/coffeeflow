from nurb import *
import copy

@assembly
def ensemble_boitier_pid(open=10.0, push=0.0):
    """Boîtier PID et son couvercle, même repère.

    open: décalage Z du couvercle en mm, 0 fermé, 10 pour voir dans le bac
    push: décalage Z du cache et de la plaque
    """
    open = float(open)
    if open < 0.0:
        reject(f"open {open} is negative: raise it to 0 or more", param="open")

    box = use("boitier_pid")
    lid = use("couvercle_pid")
    cache = use("cache_pid")
    plaque = use("cache_pid",plaque=1)
    goujon_a = use("goujon_pid")
    goujon_b = copy.copy(goujon_a)

    # Print orientation is plate on the bed, rim +Z, already mirrored in X so
    # a north-south flip lands west on west. Rotate 180 about Y to hang the
    # rim into the cavity, then lift by the box rim plus `open`.
    # The cavity rim is `hauteur` from the bed (40). bbox.max.Z is 41.6 because
    # the north channel wall stands `hauteur` on `epaisseur_fond`.
    box_bb = box.bounding_box()
    mid_x = box_bb.size.X / 2.0
    z_top = box.bounding_box().size.Z
    plate = 1.6
    lid = (
        Pos(0, 0, z_top + plate + open)
        * Pos(mid_x, 0, 0)
        * Rot(0, 180, 0)
        * Pos(-mid_x, 0, 0)
        * lid
    )
    machine_plate = plaque.bounding_box().size.Z
    opening_dy = measured("boitier_pid_ouverture_hauteur")
    opening_y = measured("boitier_pid_ouverture_y")
    cache_bb = cache.bounding_box()
    cache = (
        Rot(180,0,0)
        * Pos(-(cache_bb.size.X-box_bb.size.X)/2,
            -cache_bb.size.Y/2 - opening_y - opening_dy/2,
            machine_plate+push*2)
        * cache
    )

    plaque_bb = plaque.bounding_box()
    plaque = Pos(
        -(plaque_bb.size.X-box_bb.size.X)/2,
        opening_y,
        -plaque_bb.size.Z - push) * plaque

    opening_dx = measured("boitier_pid_ouverture_largeur")
    goujon_w = measured("boitier_pid_goujon_w")
    goujon_x0 = measured("boitier_pid_goujon_x")
    goujon_z = measured("cache_pid_goujon_z")
    goujon_y = opening_y + opening_dy/2 - goujon_w/2
    goujon_coords = (
        (box_bb.center().X+goujon_x0-goujon_w,goujon_y,-goujon_z-machine_plate),
        (box_bb.center().X-goujon_x0,goujon_y,-goujon_z-machine_plate)
    )

    goujon_a = Pos(goujon_coords[0][0],goujon_coords[0][1],goujon_coords[0][2])*goujon_a
    goujon_b = Pos(goujon_coords[1][0],goujon_coords[1][1],goujon_coords[0][2])*goujon_b

    return (box, lid, cache, goujon_a, goujon_b, plaque)

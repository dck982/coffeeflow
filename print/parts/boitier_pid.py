from nurb import *

from system import (
    AMIN, CMIN, 
    INSERT_M3,
    surplomb, 
    magnet_well_cutter, 
    bbox, 
    add_corbel
)

def bb_overall():
    return bbox(0, 0, measured("boitier_pid_largeur"), measured("boitier_pid_longueur"))

def corbel_dimensions():
    corbel_diameter = INSERT_M3.diametre_percage
    corbel_wall = INSERT_M3.epaisseur_paroi_min
    return (
        # plat
        corbel_diameter + corbel_wall,
        # along
        corbel_diameter + 2 * corbel_wall
    )

def corbels(wall):
    """Hole (cx, cy, d) and XY bbox of the pad, same origins as `add_corbel`."""
    (corbel_plat, corbel_along) = corbel_dimensions()
    corbel_diameter = INSERT_M3.diametre_percage
    corbel_wall = INSERT_M3.epaisseur_paroi_min
    corbel_mid = corbel_wall + corbel_diameter / 2
    corbel_plat = corbel_diameter + corbel_wall
    corbel_along = corbel_diameter + 2 * corbel_wall
    bb = bb_overall()
    return [
        (
            bb.max.X - wall - corbel_plat/2,
            bb.min.Y + corbel_along/2,
            3,
            bbox(bb.max.X - wall - corbel_plat, bb.min.Y, corbel_plat, corbel_along),
        ),
        (
            bb.min.X + wall + corbel_plat/2,
            bb.max.Y - corbel_mid - wall,
            3,
            bbox(bb.min.X + wall, bb.max.Y - wall - corbel_along, corbel_plat, corbel_along),
        ),
    ]    

def _block(x0,y0,z0,dx,dy,dz):
    return Pos(x0,y0,z0)*Box(dx,dy,dz,align=AMIN)

@part
def boitier_pid(
    wall=1.68, 
    floor=1.6, 
    hauteur=27.0,
    wago_surplomb=6.0,
    hauteur_hw399=5.0,
    ouvertures_sud_x=15.0,
    ouvertures_sud_dx=8.0,
    draft=False):
    """Boîtier PID avec ouverture en façade

    wall: épaisseur des murs
    floor: épaisseur du plancher
    hauteur: hauteur Z du boitier
    wago_surplomb: longueur du surplomb WAGO
    hauteur_hw399: hauteur en-dessus du floor pour le HW399
    ouvertures_sud_x: distance au bord pour chacune des ouvertures
    ouvertures_sud_dx: largeur de ouvertures
    """

    bb = bb_overall()
    body = _block(0,0,0,
        bb.max.X,
        bb.max.Y,
        floor)
    inner_west_x = wall
    inner_east_x = bb.max.X - wall
    inner_north_y = bb.max.Y - wall
    inner_south_y = wall

    # Murs nord, ouest, est. Sud = passage cables
    body += _block(
        0,0,floor,
        wall,inner_north_y,hauteur
    )
    body += _block(
        inner_east_x,0,floor,
        wall,inner_north_y,hauteur
    )
    body += _block(
        0,inner_north_y,floor,
        bb.max.X,wall,hauteur
    )

    # Mur sud
    body += _block(
        0,bb.min.Y,floor,
        bb.size.X,wall,hauteur
    )
    body -= _block(
        wall + ouvertures_sud_x, bb.min.Y, floor+hauteur/2,
        ouvertures_sud_dx, wall, hauteur/2
    )
    body -= _block(
        inner_east_x - ouvertures_sud_x - ouvertures_sud_dx, bb.min.Y, floor+hauteur/2,
        ouvertures_sud_dx, wall, hauteur/2
    )

    # Deux carrés ouverts dans le fond pour emboiter le cache externe
    opening_dx = measured("boitier_pid_ouverture_largeur")
    opening_dy = measured("boitier_pid_ouverture_hauteur")
    opening_y = measured("boitier_pid_ouverture_y")
    goujon_w = measured("boitier_pid_goujon_w")
    goujon_x0 = measured("boitier_pid_goujon_x")
    goujon_y = opening_y + opening_dy/2 - goujon_w/2
    goujon_coords = (
        (body.bounding_box().center().X-goujon_x0,goujon_y),
        (body.bounding_box().center().X+goujon_x0-goujon_w,goujon_y)
    )
    for (gx, gy) in goujon_coords:
        body -= _block(gx,gy,0,goujon_w,goujon_w,floor)

    # body -=_block(
    #     # Centered in X
    #     body.bounding_box().center().X-opening_dx/2,
    #     measured("boitier_pid_ouverture_y"),
    #     0,
    #     opening_dx,
    #     opening_dy,
    #     floor
    #     )

    # Compartiment pour WAGO 221-412 sur la tranche, au nord-est
    # Utilisé pour retour V_out de l'optocoupleur
    # Mur sud, seuil ouest, surplomb nord
    wago_seuil_dz = 1.0
    wago_prof = measured("wago_profondeur")-0.1
    wago_hauteur = measured("wago_epaisseur")-0.15
    wago_412_largeur = measured("wago_412_largeur")
    surplomb_w = 1.0
    wago_north = _block(
        inner_east_x-wago_prof-wall,inner_north_y-wago_hauteur-wall,floor,
        wago_prof+wall,wall,wago_412_largeur+surplomb_w
    )
    wago_north += surplomb(
        inner_north_y-wago_hauteur,inner_east_x-wago_surplomb,wago_412_largeur+floor,
        surplomb_w,wago_surplomb,plane=Plane.YZ,inverse=True)
    # seuil
    wago_north += _block(
        inner_east_x-wago_prof-wall,inner_north_y-wago_hauteur-wall,floor,
        wall,wago_hauteur+wall,wago_seuil_dz
    )
    wago_north_bb = wago_north.bounding_box()
    body += wago_north

    # Compartiment pour trois WAGO 221-423 sur leur tranche au sud du premier compartiment
    wago_423_largeur = measured("wago_423_largeur")
    wago_3x_423_west_x = inner_east_x-wago_hauteur*3-wall
    wago_423_south_y = wago_north_bb.min.Y-wago_prof-wall
    wago_423_dy = wago_prof+wall
    wago_423_dz = wago_423_largeur+surplomb_w
    # Mur ouest
    body += _block(
        wago_3x_423_west_x,wago_423_south_y,floor,
        wall,wago_423_dy,wago_423_dz
    )
    # Mur nord
    body += _block(
        wago_3x_423_west_x,wago_north_bb.min.Y,floor,
        wago_hauteur*3+wall,wall,wago_423_dz
    )
    # seuil
    body += _block(
        wago_3x_423_west_x,wago_423_south_y,floor,
        wago_hauteur*3+wall,wall,wago_seuil_dz
    )
    # surplombs
    body += surplomb(
        wago_3x_423_west_x+wall,wago_north_bb.min.Y,wago_423_largeur+floor,
        surplomb_w,wago_surplomb,inverse=True
    )
    body += surplomb(
        inner_east_x,wago_north_bb.min.Y-wago_surplomb,wago_423_largeur+floor,
        surplomb_w,wago_surplomb
    )
    body += surplomb(
        wago_north_bb.min.Y,round(wago_3x_423_west_x+wall+wago_hauteur*1.5+wago_surplomb/2),wago_423_largeur+floor,
        surplomb_w,wago_surplomb,plane=Plane.YZ
    )

    # HW399: snap-in
    # un muret au nord et sud pour le bloquer
    # quatre piliers aux coins pour le soutenir
    # deux snap-ins sur les cotes
    hw399_north_y = inner_north_y - measured("hw399_marge") - wall
    hw399_dx = measured("hw399_largeur")+0.2
    hw399_dy = measured("hw399_longueur")+0.2
    hw399_pcb = measured("hw399_pcb_width")
    hw399_south_y = hw399_north_y - hw399_dy
    plot_width = 4.0
    # Murs est
    body += _block(
        inner_west_x+hw399_dx,hw399_north_y-plot_width,floor,
        wall,plot_width+wall,hauteur_hw399+hw399_pcb
    )
    body += _block(
        inner_west_x+hw399_dx,hw399_south_y-wall,floor,
        wall,plot_width+wall,hauteur_hw399+hw399_pcb
    )
    # Mur nords et sud
    body += _block(
        inner_west_x,hw399_north_y,floor,
        hw399_dx+wall,wall,hauteur_hw399+hw399_pcb
    )
    body += _block(
        inner_west_x,hw399_south_y-wall,floor,
        hw399_dx+wall,wall,hauteur_hw399+hw399_pcb
    )
    # Plots
    for (x,y) in (
        (inner_west_x,hw399_north_y-plot_width),
        (inner_west_x+hw399_dx-plot_width,hw399_north_y-plot_width),
        (inner_west_x,hw399_south_y),
        (inner_west_x+hw399_dx-plot_width,hw399_south_y),
    ):
        body += _block(x,y,floor,plot_width,plot_width,hauteur_hw399)

    # Snap-in: wall + surplomb
    snap_in_x = inner_west_x+hw399_dx
    snap_in_z = hauteur_hw399+hw399_pcb+0.5
    body += _block(
        snap_in_x,hw399_south_y+plot_width+wall,floor,
        wall,hw399_dy-2*plot_width-2*wall,snap_in_z+surplomb_w
    )
    body += surplomb(
        snap_in_x,hw399_south_y+hw399_dy/2-wago_surplomb/2,snap_in_z+floor,
        surplomb_w,wago_surplomb
    )

    # Magnets
    well_dx = measured("aimant_diametre") + measured("aimant_puit_press_fit")+wall
    pad_dz = measured("aimant_hauteur")+measured("aimant_puit_fond")
    body += _block(
        inner_west_x,inner_north_y-well_dx,0,
        well_dx,well_dx,pad_dz)
    body -= magnet_well_cutter(inner_west_x+well_dx/2-wall/2, inner_north_y-well_dx/2+wall/2, 0)
    body += _block(
        inner_east_x-well_dx,inner_south_y-wall,0,
        well_dx,well_dx+wall,pad_dz)
    body -= magnet_well_cutter(inner_east_x-well_dx/2+wall/2, inner_south_y+well_dx/2-wall/2, 0)

    # Corbels
    corbels_data = corbels(wall)
    (corbel_plat, corbel_along) = corbel_dimensions()
    body = add_corbel(body, 
        corbels_data[0][0] + corbel_plat/2, corbels_data[0][1] - corbel_along/2, hauteur+floor, 
        insert=INSERT_M3, reverse=True)
    body = add_corbel(body, 
        corbels_data[1][0] - corbel_plat/2, corbels_data[1][1] - corbel_along/2, hauteur+floor, 
        insert=INSERT_M3)

    if draft:
        return body
    # Name what must stay sharp, then let `polish` chamfer whatever the kernel takes.
    # A bare `chamfer(...)` is all or nothing: one edge that cannot land loses the lot.
    keep = body.edges().filter_by(Axis.Z).filter_by(
        lambda e: (e.bounding_box().min.Y < 0.05) or (bb.max.Y-e.bounding_box().max.Y < 0.05)
    )
    return polish(body, keep, 1.0) if keep else body

from nurb import *

from system import (
    INSERT_M25,
    INSERT_M3,
    CMIN, AMIN,
    _fuse_one,
    add_wall,
    add_corbel,
    add_well,
    offset_in,
    add_heat_insert,
    bbox,
    puit_couche,
    surplomb_xz,
)

ANGLE_OPENING_W = 7.0
ANGLE_OPENING_Z0 = 10.0

# Overall dimensions
def bb_overall(wall):
    dx = 3*wall + measured("pcb_sensors_dx") + measured("wago_2411_largeur")
    west_x = measured("boitier_int_marche_x")
    dy = 2*wall+measured("pcb_sensors_dy")+measured("pcb_sensors_space_y")
    south_y = measured("boitier_int_aile_y")+measured("magnet_spacer_ac_dans_boitier")-dy
    return bbox(west_x,south_y,dx,dy)

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
    bb = bb_overall(wall)
    return [
        (
            bb.min.X + wall + corbel_plat/2,
            bb.min.Y + wall + corbel_along/2,
            INSERT_M3.diametre_vis,
            bbox(bb.min.X + wall, bb.min.Y + wall, corbel_plat, corbel_along),
        ),
        (
            bb.max.X - wall - corbel_plat/2,
            bb.max.Y - wall - corbel_along/2 - ANGLE_OPENING_W,
            INSERT_M3.diametre_vis,
            bbox(bb.max.X - wall - corbel_plat, bb.max.Y - wall - ANGLE_OPENING_W - corbel_along, corbel_plat, corbel_along),
        ),

    ]    

def dc_contour(wall):
    bb = bb_overall(wall)
    return ([
        (bb.min.X, bb.min.Y),
        (bb.max.X, bb.min.Y),
        (bb.max.X, bb.max.Y),
        (bb.min.X, bb.max.Y),
    ], bb)

def add_east_wago(body,outer,inner_east_x,inner_south_y,floor,wall,surplomb_len,height):
    x0 = inner_east_x-measured("wago_2411_largeur")-wall
    dy = measured("wago_2411_longueur")
    dz = measured("wago_2411_epaisseur")*2
    surplomb_w = 1.0
    body = add_wall(
        body, 
        outer, 
        x0,
        inner_south_y,
        wall,
        dy,
        floor,
        dz+surplomb_w
    )
    # surplomb catch
    body = body + surplomb_xz(
        x0+wall,
        inner_south_y+surplomb_len,
        dz,
        surplomb_w, 
        surplomb_len, 
        floor, 
        inverse=True)
    # opening
    catch = 1.0
    body -= Pos(
        inner_east_x-measured("wago_2411_largeur")+catch,
        inner_south_y-wall,
        floor+catch) * Box(
            measured("wago_2411_largeur")-2*catch,
            wall,
            height-catch,
            align=AMIN
        )
    return body

def add_pcb(body,outer,x0,y0,floor,wall):
    z0 = measured("pcb_sensors_space_z")
    # heat insert for M2.5 PCB hole
    body = add_heat_insert(
        body, 
        outer, 
        x0+measured("pcb_sensors_hole_cx"),
        y0+measured("pcb_sensors_hole_cy"),
        floor,
        z0,
        INSERT_M25
        )
    # bar to hold the PCB up north
    bar_w = 1.0
    y1 = y0 + measured("pcb_sensors_dy")
    body = add_wall(
        body,outer,
        x0,y1-bar_w,
        measured("pcb_sensors_dx"),bar_w,
        floor+z0+measured("pcb_sensors_depth"),bar_w)
    # and bar to support
    body = add_wall(body,outer,x0,y1-bar_w*3,measured("pcb_sensors_dx"),bar_w*3,floor,z0)

    return body

@part
def boitier_dc(
    hauteur=25.0,
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    wago_surplomb=6.0,
    draft=False,
):
    """Boîtier DC : partie ouest, face est à x = 50, nord à y = 95.

    hauteur: hauteur hors-tout depuis le lit (murs compris)
    epaisseur_paroi: épaisseur des murs, vers l'intérieur
    epaisseur_fond: épaisseur du fond vers le haut
    wago_surplomb: longueur du surplomb WAGO
    """
    wall = epaisseur_paroi
    floor = epaisseur_fond
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")

    if wall < 1.26:
        reject(
            f"epaisseur_paroi {wall} is under 1.26 mm: raise it",
            param="epaisseur_paroi",
        )
    if hauteur < floor + 2.0:
        reject(
            f"hauteur {hauteur} leaves under 2 mm of wall above a {wall} mm floor: "
            f"raise it above {wall + 2.0:.1f}",
            param="hauteur",
        )

    well_stack = measured("aimant_puit_fond")+measured("aimant_hauteur")
    if hauteur < well_stack:
        reject(
            f"hauteur {hauteur} is under the magnet well ({well_stack + 0.4:.1f} mm): "
            "raise it",
            param="hauteur",
        )

    # outer and inner polygon
    outer_pts, bb = dc_contour(wall)
    inner_pts = offset_in(outer_pts, wall)

    # compute inner angles coords
    inner_west_x = bb.min.X + wall
    inner_east_x = bb.max.X - wall
    inner_south_y = bb.min.Y + wall
    inner_north_y = bb.max.Y - wall

    # Empty box
    outer = Pos(bb.min.X,bb.min.Y,0) * Box(bb.size.X,bb.size.Y,hauteur,align=AMIN)
    cavity = Pos(0, 0, floor) * extrude(
        Polygon(*inner_pts, align=None), hauteur + 0.2
    )
    body = outer - cavity

    # two wago 2411 stacked on the right
    body = add_east_wago(body,outer,inner_east_x,inner_south_y,floor,wall,wago_surplomb,hauteur)

    # PCB sensors
    body = add_pcb(body,outer,inner_west_x,inner_south_y+measured("pcb_sensors_space_y"),floor,wall)

    # Opening north west
    body -= Pos(inner_east_x,inner_north_y-ANGLE_OPENING_W,ANGLE_OPENING_Z0)*Box(
        wall,
        ANGLE_OPENING_W,
        hauteur-ANGLE_OPENING_Z0,
        align=AMIN
    )
    body -= Pos(inner_east_x-ANGLE_OPENING_W+wall,inner_north_y,ANGLE_OPENING_Z0)*Box(
        ANGLE_OPENING_W,
        wall,
        hauteur-ANGLE_OPENING_Z0,
        align=AMIN
    )

    # Magnet well
    dx = inner_east_x-inner_west_x
    dy = inner_north_y-inner_south_y
    body = add_well(body,outer,inner_west_x+dx/2,inner_south_y+dy/3)
    body = add_well(body,outer,inner_west_x+dx/2,inner_south_y+dy*2/3)

    # Corbels
    corbels_data = corbels(wall)
    (corbel_plat, corbel_along) = corbel_dimensions()
    body = add_corbel(body, 
        corbels_data[0][0] - corbel_plat/2, corbels_data[0][1] - corbel_along/2, hauteur, 
        insert=INSERT_M3, reverse=False)
    body = add_corbel(body, 
        corbels_data[1][0] + corbel_plat/2, corbels_data[1][1] - corbel_along/2, hauteur, 
        insert=INSERT_M3, reverse=True)

    body = _fuse_one(body)

    if draft:
        return body

    def keep(edge):
        bb = bb_overall(wall)
        c = edge.center()
        if c.X<bb.max.X and c.X>bb.min.X:
            return False
        return True

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

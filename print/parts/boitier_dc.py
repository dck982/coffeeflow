from nurb import *

from system import (
    INSERT_M2,
    INSERT_M25,
    INSERT_M3,
    CMIN, AMIN,
    _fuse_one,
    add_well,
    add_wall,
    add_corbel,
    offset_in,
    add_heat_insert,
    bbox,
    puit_couche,
)

from parts.boitier_ac import ssr_dc_opening, dimmer_dc_opening

# Overall dimensions
def bb_overall():
    return bbox(0, 0, measured("boitier_int_aile_x"), measured("boitier_int_y"))

# Dimensions of the top area after the marche
def bb_top():
    return bbox(
        measured("boitier_int_marche_x"), 
        measured("boitier_int_marche_y"),
        measured("boitier_int_aile_x")-measured("boitier_int_marche_x"),
        measured("boitier_int_y")-measured("boitier_int_marche_y")
        )

def bb_bottom():
    return bbox(0,0,measured("boitier_int_marche_x"),measured("boitier_int_marche_y"))

def corbels(wall):
    corbel_diameter = INSERT_M3.diametre_percage
    corbel_wall = INSERT_M3.epaisseur_paroi_min
    corbel_mid = corbel_wall + corbel_diameter/2
    corbel_half = (corbel_diameter + corbel_wall)/2
    chanfrein = measured("boitier_int_chanfrein")
    return [
        (
            bb_top().min.X + wall + corbel_half,
            bb_top().min.Y - wall + corbel_mid,
            3
        ),
        (
            measured("boitier_int_chanfrein") + corbel_mid,
            bb_overall().min.Y + wall + corbel_half,
            3
        )
    ]

def dc_contour():
    bb = bb_overall()
    bbtop = bb_top()
    chanfrein = measured("boitier_int_chanfrein")
    return ([
        (bb.min.X, chanfrein),
        (chanfrein, bb.min.Y),
        (bb.max.X, bb.min.Y),
        (bb.max.X, bb.max.Y),
        (bbtop.min.X, bb.max.Y),
        (bbtop.min.X, bbtop.min.Y),
        (bb.min.X, bbtop.min.Y),
    ], bb)

def _add_xiao_pin(body, outer, cx, cy, z0, height):
    # 3mm wide pin to support the M2 hole
    pin_sw = Pos(cx, cy, z0) * Cylinder(
        1.5, height, align=CMIN
    )
    return body + pin_sw.intersect(outer)

# XIAO elements: two heat inserts, two pins, a thin separation wall
def _xiao_area(body, outer, east_x, south_y, z0):
    xiao_len = measured("xiao_board_len") 
    xiao_width = measured("xiao_board_width")
    xiao_height = measured("xiao_board_height")
    xiao_z0 = 5.0

    # two M2 heat inserts at the top
    hole_dx = measured("xiao_board_hole_dx")
    hole_dy = measured("xiao_board_hole_dy")
    hole_east_x = east_x - hole_dx
    hole_west_x = east_x - xiao_width + hole_dx
    hole_y = south_y + xiao_len - hole_dy
    body = add_heat_insert(body, outer, 
        hole_east_x, hole_y, z0, xiao_z0, INSERT_M2, ring_factor=1.25)
    body = add_heat_insert(body, outer, 
        hole_west_x, hole_y, z0, xiao_z0, INSERT_M2)

    # two pillars lower
    pillars_dy = measured("xiao_board_pillars_dy")
    pin_y = hole_y - pillars_dy
    body = _add_xiao_pin(body, outer, hole_west_x, pin_y, z0, xiao_z0)
    body = _add_xiao_pin(body, outer, hole_east_x, pin_y, z0, xiao_z0)

    # a separation wall on the west, thinnest possible
    thin_wall = 1.26
    body = add_wall(body, outer, 
        east_x - xiao_width - thin_wall, pin_y+3, 
        thin_wall, hole_y-pin_y-3-INSERT_M2.encombrement/2, 
        z0, xiao_height)

    # add a magnet well under the XIAO module
    body = add_well(body, outer, east_x - xiao_width/2, south_y + xiao_len - 20.0)

    return body

def canpal_bb(wall, z0):
    bbtop = bb_top()
    wago_bb = bb_wago_north_west(bbtop, wall)
    # leave room for the SLNT wire that hooks through the top terminal block
    canpal_dy = measured("can_pal_length") + wall
    return bbox(
        wago_bb.max.X, 
        bbtop.max.Y-wall-canpal_dy-measured("can_pal_slnt_room"),
        bbtop.max.X-wall-wago_bb.max.X,
        canpal_dy,
        z0=z0,h=5.0
        )

# CAN Pal in the NE corner
def _canpal_area(body, outer, z0, wall):

    bb = canpal_bb(wall, z0)

    module_w = measured("can_pal_width")
    module_l = measured("can_pal_length")
    # place the module 5mm above floor (heat inserts need 4)
    module_z0 = bb.size.Z 
    h2e = measured("can_pal_hole_to_edge")
    
    # center the module
    center_x = bb.center().X
    west_edge_x = center_x - module_w/2.0
    east_edge_x = center_x + module_w/2.0

    # then place the two M2.5 inserts
    # First in the NE corner
    # Left hole is same distance from left edge
    # Add two walls direction south to support the
    # module over 50% of its length
    for cx in (east_edge_x-h2e,west_edge_x+h2e):
        cy = bb.max.Y - h2e
        body = add_heat_insert(body, outer,
            cx, cy, z0, module_z0, INSERT_M25
            )
        # half insert
        hi = INSERT_M25.encombrement/2
        body = add_wall(body, outer, 
            cx-wall/2, cy-hi-module_l/2,
            wall, module_l/2, z0, module_z0)
    
    # Add a perpendicular wall to stop the module on the south
    # Make it use only the center 1/3 of the width, to not
    # block the cable coming from the AC box
    pcb_h = measured("can_pal_pcb_height")
    stop_bar_x0 = west_edge_x + module_w/3
    body = add_wall(body, outer,
        stop_bar_x0, bb.min.Y,
        module_w/3, wall, z0, module_z0 + pcb_h)
    # Then a little overhang to hold the PCB
    body = add_wall(body, outer,
        stop_bar_x0, bb.min.Y,
        module_w/3, wall+1.0, z0 + module_z0 + pcb_h, 1.0)

    return body

def _surplomb_hook_pts(xy, z_mid, surplomb_w, inverse=False):
    multiplier = -1.0 if inverse else 1.0
    return [
        (xy, z_mid - surplomb_w),
        (xy - surplomb_w * multiplier, z_mid),
        (xy - surplomb_w * multiplier, z_mid + surplomb_w),
        (xy, z_mid + surplomb_w),
    ]

def _surplomb_xz(x, y, z, sw, slen, z0, inverse=False):
    hook_pts = _surplomb_hook_pts(x, z, sw, inverse=inverse) 
    hook_y0 = y - slen
    return (
        Pos(0, hook_y0, z0)
        * extrude(
            Plane.XZ * Polygon(*hook_pts, align=None),
            slen * (-1.0 if inverse else 1.0),
        )
    )

def _bb_wago_nw(bb, area_dx, wall):
    # In Y: a wago and a stop wall
    area_dy = measured("wago_profondeur") + wall
    # In X: a wago and a side wall
    area_dx = area_dx + wall
    # Get inner bounds (add/subtract wall)
    return bbox(bb.min.X + wall, bb.max.Y - wall - area_dy, area_dx, area_dy)

# Definition of a compartment for wago connectors laying on their side, in a NW corner
def _wago_nw(body, outer, container_bb, area_dx, area_dz, wago_raise, surplomb_len, surplomb_w, z0, wall, count=1):
    bb = _bb_wago_nw(container_bb, area_dx, wall)
    
    area_dy = measured("wago_profondeur")
    area_dz = area_dz+wago_raise

    # Add a wall on the east side to press the WAGO
    body = add_wall(body, outer,
        bb.max.X - wall, bb.min.Y,
        wall, area_dy+wall,
        z0, area_dz + surplomb_w)

    # Add a parallel wall in the middle to raise the WAGO
    wx = bb.min.X
    for widx in range(count):
        dwx = (area_dx - wall) / (count+1)
        wx += dwx
        body = add_wall(body, outer,
            wx, bb.min.Y+wall,
            wall, area_dy,
            z0, wago_raise)

    # Add a perpendicular wall to stop the WAGO from sliding out
    catch_height = 1.0
    body = add_wall(body, outer,
        bb.min.X, bb.min.Y,
        area_dx, wall,
        z0, wago_raise + catch_height)

    # Surplomb (catch): 1mm return from the muret toward the WAGO
    # with its vertical face starting at the Wago top and its 45° lead-in
    # starting 1mm below.    
    body = body + _surplomb_xz(
        bb.max.X - wall, 
        bb.max.Y,
        area_dz, 
        surplomb_w, 
        surplomb_len, 
        z0).intersect(outer)

    return body

# A compartment for a 221-412 wago connector in the marche corner
def _wago_south_west(body, outer, container_bb, wago_raise, surplomb_len, surplomb_w, z0, wall):
    # A 221-412 on its side
    area_dx = measured("wago_epaisseur")*2
    dz_412 = measured("wago_412_largeur")
    dz_423 = measured("wago_423_largeur")
    body = body + _wago_nw(body, outer, 
        container_bb, area_dx, dz_412, 
        wago_raise, surplomb_len, surplomb_w, z0, wall, count=2)
    # add a surplomb on the left for the 423
    body = body + _surplomb_xz(
        container_bb.min.X+wall, 
        container_bb.max.Y-wall, 
        dz_423+wago_raise, 
        surplomb_w, 
        surplomb_len, 
        z0, 
        inverse=True)
    return body

# For ensemble_boitiers
def bb_wago_north_west(container_bb, wall):
    return _bb_wago_nw(container_bb, measured("wago_epaisseur"), wall)

# A compartment for a 221-423 wago connector in the NW corner
def _wago_north_west(body, outer, container_bb, wago_raise, surplomb_len, surplomb_w, z0, wall):
    # A 221-423 on its side
    area_dx = measured("wago_epaisseur")
    area_dz = measured("wago_423_largeur")
    return _wago_nw(body, outer, container_bb, area_dx, area_dz, wago_raise, surplomb_len, surplomb_w, z0, wall)

def _add_vertical_magnet(body, hauteur):
    bb = bb_overall()
    # Small magnet on the north wall, axis lying (into the box). Mouth on the
    # interior; outer face keeps `aimant_puit_fond`. Teardrop roof via
    # `puit_couche` (local +Y = world +Z). pont=1: default 2 flattens a Ø5.2.
    d = measured("aimant_petit_diametre") + measured("aimant_puit_press_fit")
    h = measured("aimant_petit_hauteur")
    fond = measured("aimant_puit_fond")
    ring_radius = d / 2 + measured("aimant_puit_mur")
    cx = bb.min.X + bb.size.X * 0.6
    cz = hauteur - 5
    pad = Rot(90, 0, 0) * Pos(cx, cz, -bb.max.Y) * Cylinder(ring_radius, h, align=CMIN)
    mouth = Plane(
        origin=(cx, bb.max.Y - fond - h, cz),
        x_dir=(-1, 0, 0),
        z_dir=(0, 1, 0),
    )
    return body + pad - mouth * puit_couche(d / 2, h, pont=1.0)

@part
def boitier_dc(
    hauteur=27.0,
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    wago_raise=3.0,
    wago_surplomb=6.0,
    draft=False,
):
    """Boîtier DC : partie ouest, face est à x = 50, nord à y = 95.

    hauteur: hauteur hors-tout depuis le lit (murs compris)
    epaisseur_paroi: épaisseur des murs, vers l'intérieur
    epaisseur_fond: épaisseur du fond vers le haut
    wago_raise: de combien monter les logements WAGO
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
    if wago_raise < 3:
        reject(
            f"wago_raise {wago_raise} is under the magnet well height 3mm: raise it",
            param="wago_raise"
        )

    # outer and inner polygon
    outer_pts, bb = dc_contour()
    inner_pts = offset_in(outer_pts, wall)

    # compute inner angles coords
    inner_west_x = bb.min.X + wall
    inner_west_marche_x = bb_top().min.X + wall
    inner_north_marche_y = bb_top().min.Y - wall
    inner_east_x = bb.max.X - wall
    inner_south_y = bb.min.Y + wall
    inner_north_y = bb.max.Y - wall

    # we need to fit 
    # - a Unit CAN on its side
    # - a 1.6mm wire channel 
    # - a thin wall (1.26mm)
    # - the XIAO 
    # in the upper width (marche)
    if inner_west_marche_x + measured("unit_can_height") + 1.6 + 1.26 + measured("xiao_board_width") > inner_east_x:
        reject(
            f"pas la place pour Unit CAN, cable, mur fin et XIAO en X dans la partie supérieure du boitier",
            param="epaisseur_paroi"
        )

    # we need to fit in Y
    # - a XIAO board
    # - a 221-415 WAGO
    if inner_south_y + measured("xiao_board_len") + measured("wago_415_largeur") > inner_north_y:
        reject(
            f"pas la place pour XIAO et WAGO en Y dans le boitier",
            param="epaisseur_paroi"
        )

    # Empty box
    outer = extrude(Polygon(*outer_pts, align=None), hauteur)
    cavity = Pos(0, 0, floor) * extrude(
        Polygon(*inner_pts, align=None), hauteur + 0.2
    )
    body = outer - cavity

    # Pillars / heat inserts for XIAO ESP32, tuck in the south east corner
    body = _xiao_area(body, outer, inner_east_x, inner_south_y, floor)

    # Wago south east: one Wago 221-423 for 5V + one Wago 221-412 for GND connection
    wago_surplomb_w = 1.0
    body = _wago_south_west(body, outer, 
        bb_bottom(),
        wago_raise, wago_surplomb, wago_surplomb_w, 
        floor, wall)

    # Wago north east: one Wago 221-423 for 5V connection
    body = _wago_north_west(body, outer, 
        bb_top(),
        wago_raise, wago_surplomb, wago_surplomb_w, 
        floor, wall)

    # CAN Pal
    body = _canpal_area(body, outer, floor, wall)

    # Vertical magnet
    body = _add_vertical_magnet(body, hauteur)

    # Two M3 corbel heat inserts: same recipe as boitier_ps's wall corbels,
    # an overhang from the wall's inner face near the rim (not a tower from
    # the floor) to spend minimum material. Thin at z_corbel_45, full
    # `corbel_plat` thick from z_corbel to the rim; the bore drills down
    # `corbel_profondeur` from the rim.
    chanfrein = measured("boitier_int_chanfrein")
    body = add_corbel(body, inner_west_marche_x, inner_north_marche_y, hauteur, insert=INSERT_M3)
    body = add_corbel(body, chanfrein, inner_south_y, hauteur, insert=INSERT_M3, plane=Plane.YZ)

    # Chamfer wall (0, 20) → (20, 0): open the low-X half, leftmost
    # corner to the midpoint. Floor stays.
    s2 = 2.0 ** 0.5
    tx, ty = 1.0 / s2, -1.0 / s2
    nx, ny = 1.0 / s2, 1.0 / s2
    ax, ay = 0.0, chanfrein
    mx, my = chanfrein / 2.0, chanfrein / 2.0
    past = 1.0
    inn = wall + 2.0
    margin = 0
    body = body - (
        Pos(0, 0, floor)
        * extrude(
            Polygon(        
                (ax - past * tx - margin * nx, ay - past * ty - margin * ny),
                (mx + margin * tx - margin * nx, my + margin * ty - margin * ny),
                (mx + margin * tx + inn * nx, my + margin * ty + inn * ny),
                (ax - past * tx + inn * nx, ay - past * ty + inn * ny),
                align=None,
            ),
            hauteur + 0.2,
        )
    )

    # East-face slots matching AC west: dimmer (north module) and SSR (south).
    # Y from y_max so the north faces stay aligned; Z from AC (open to the top).
    (do_y0, do_z0, do_dy, do_dz) = dimmer_dc_opening(wall, floor, hauteur)
    body = body - Pos(inner_east_x, do_y0, do_z0) * Box(
        wall,
        do_dy,
        do_dz,
        align=AMIN,
    )
    (ss_y0, ss_z0, ss_dy, ss_dz) = ssr_dc_opening(wall, floor, hauteur)
    body = body - Pos(inner_east_x, ss_y0, ss_z0) * Box(
        wall,
        ss_dy,
        ss_dz,
        align=AMIN,
    )

    body = _fuse_one(body)
    if draft:
        return body

    def keep(edge):
        c = edge.center()
        if c.Y < chanfrein:
            return True
        if c.X < inner_west_x:
            return True
        if (c.X < inner_west_marche_x) and (c.Y>inner_north_y):
            return True
        return False

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

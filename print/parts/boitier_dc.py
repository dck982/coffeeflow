from nurb import *

from system import (
    INSERT_M2,
    INSERT_M25,
    CMIN, AMIN,
    _fuse_one,
    add_well,
    offset_in,
    ouvertures_modules,
    add_heat_insert
)

def _contour(chanfrein):
    aile_x = measured("boitier_int_aile_x")
    y_max = measured("boitier_int_y")
    marche_x = measured("boitier_int_marche_x")
    marche_y = measured("boitier_int_marche_y") + 3.0
    return ([
        (0.0, chanfrein),
        (chanfrein, 0.0),
        (aile_x, 0.0),
        (aile_x, y_max),
        (marche_x, y_max),
        (marche_x, marche_y),
        (0.0, marche_y),
    ],
    0, aile_x, 0, y_max, marche_x, marche_y)

def _add_xiao_pin(body, outer, cx, cy, z0, height):
    # 3mm wide pin to support the M2 hole
    pin_sw = Pos(cx, cy, z0) * Cylinder(
        1.5, height, align=CMIN
    )
    return body + pin_sw.intersect(outer)

# add a horizontal wall
def _add_wall(body, outer, x, y, dx, dy, z0, height):
    xiao_west_box = Pos(x, y, z0) * Box(
        dx,
        dy,
        height,
        align=AMIN,
    )
    return body + xiao_west_box.intersect(outer)

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
    body = _add_wall(body, outer, 
        east_x - xiao_width - thin_wall, pin_y+3, 
        thin_wall, hole_y-pin_y-3-INSERT_M2.encombrement/2, 
        z0, xiao_height)

    # add a magnet well under the XIAO module
    body = add_well(body, outer, east_x - xiao_width/2, south_y + xiao_len - 20.0)

    return body

# CAN Pal in the NE corner
def _canpal_area(body, outer, east_x, north_y, z0, wall, west_x):
    module_w = measured("can_pal_width")
    module_l = measured("can_pal_length")
    # place the module 5mm above floor (heat inserts need 4)
    module_z0 = 5.0 
    h2e = measured("can_pal_hole_to_edge")

    # leave room for the SLNT wire that hooks through the top terminal block
    top_y = north_y - measured("can_pal_slnt_room")
    
    # center the module
    center_x = round(east_x - west_x / 2.0)*1.0
    west_edge_x = center_x - module_w/2.0
    east_edge_x = center_x + module_w/2.0

    # then place the two M2.5 inserts
    # First in the NE corner
    # Left hole is same distance from left edge
    # Add two walls direction south to support the
    # module over 50% of its length
    for cx in (east_edge_x-h2e,west_edge_x+h2e):
        cy = top_y - h2e
        body = add_heat_insert(body, outer,
            cx, cy, z0, module_z0, INSERT_M25
            )
        # half insert
        hi = INSERT_M25.encombrement/2
        body = _add_wall(body, outer, 
            cx-wall/2, cy-hi-module_l/2,
            wall, module_l/2, z0, module_z0)
    
    # Add a perpendicular wall to stop the module on the south
    pcb_h = measured("can_pal_pcb_height")
    body = _add_wall(body, outer,
        west_edge_x, top_y-module_l-wall, 
        module_w, wall, z0, module_z0 + pcb_h)
    # Then a little overhang to hold the PCB
    body = _add_wall(body, outer,
        west_edge_x, top_y-module_l-wall, 
        module_w, wall+1.0, z0 + module_z0 + pcb_h, 1.0)

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

# Definition of a compartment for wago connectors laying on their side, in a NW corner
def _wago_nw(body, outer, west_x, north_y, area_dx, area_dz, wago_raise, surplomb_len, surplomb_w, z0, wall, count=1):
    area_dy = measured("wago_profondeur")
    area_dz = area_dz+wago_raise

    # Add a wall on the east side to press the WAGO
    east_wall_x = west_x + area_dx
    body = _add_wall(body, outer,
        east_wall_x, north_y-area_dy,
        wall, area_dy,
        z0, area_dz + surplomb_w)

    # Add a parallel wall in the middle to raise the WAGO
    wx = west_x
    for widx in range(count):
        dwx = (area_dx - wall) / (count+1)
        wx += dwx
        body = _add_wall(body, outer,
            wx, north_y - area_dy,
            wall, area_dy,
            z0, wago_raise)

    # Add a perpendicular wall to stop the WAGO from sliding out
    catch_height = 1.0
    body = _add_wall(body, outer,
        west_x, north_y - area_dy - wall,
        area_dx, wall,
        z0, wago_raise + catch_height)

    # Surplomb (catch): 1mm return from the muret toward the WAGO
    # with its vertical face starting at the Wago top and its 45° lead-in
    # starting 1mm below.    
    body = body + _surplomb_xz(east_wall_x, north_y, area_dz, surplomb_w, surplomb_len, z0).intersect(outer)

    return body

# A compartment for a 221-412 wago connector in the marche corner
def _wago_south_west(body, outer, west_x, north_y, wago_raise, surplomb_len, surplomb_w, z0, wall):
    # A 221-412 on its side
    area_dx = measured("wago_epaisseur")*2
    dz_412 = measured("wago_412_largeur")
    dz_423 = measured("wago_423_largeur")
    body = body + _wago_nw(body, outer, 
        west_x, north_y, area_dx, dz_412, 
        wago_raise, surplomb_len, surplomb_w, z0, wall, count=2)
    # add a surplomb on the left for the 423
    body = body + _surplomb_xz(west_x, north_y, dz_423, surplomb_w, surplomb_len, z0, inverse=True)
    return body

# A compartment for a 221-423 wago connector in the NW corner
def _wago_north_west(body, outer, west_x, north_y, wago_raise, surplomb_len, surplomb_w, z0, wall):
    # A 221-423 on its side
    area_dx = measured("wago_epaisseur")
    area_dz = measured("wago_423_largeur")
    return (
        _wago_nw(body, outer, west_x, north_y, area_dx, area_dz, wago_raise, surplomb_len, surplomb_w, z0, wall),
        west_x + area_dx + wall
    )

@part
def boitier_dc(
    hauteur=27.0,
    chanfrein=15.0,
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    wago_raise=3.0,
    wago_surplomb=6.0,
    draft=False,
):
    """Boîtier DC : partie ouest, face est à x = 50, nord à y = 95.

    hauteur: hauteur hors-tout depuis le lit (murs compris)
    chanfrein: angle coupé en sud-ouest
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
    outer_pts, west_x, east_x, south_y, north_y, west_marche_x, north_marche_y = _contour(chanfrein)    
    inner_pts = offset_in(outer_pts, wall)

    # compute inner angles coords
    inner_west_x = west_x + wall
    inner_west_marche_x = west_marche_x + wall
    inner_north_marche_y = north_marche_y - wall
    inner_east_x = east_x - wall
    inner_south_y = south_y + wall
    inner_north_y = north_y - wall

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
        inner_west_x, inner_north_marche_y, 
        wago_raise, wago_surplomb, wago_surplomb_w, 
        floor, wall)

    # Wago north east: one Wago 221-423 for 5V connection
    body, wago_x = _wago_north_west(body, outer, 
        inner_west_marche_x, inner_north_y, 
        wago_raise, wago_surplomb, wago_surplomb_w, 
        floor, wall)

    # CAN Pal
    body = _canpal_area(body, outer, inner_east_x, inner_north_y, floor, wall, wago_x)


    # Two M2.5 corbel heat inserts: same recipe as boitier_ps's wall corbels,
    # an overhang from the wall's inner face near the rim (not a tower from
    # the floor) to spend minimum material. Thin at z_corbel_45, full
    # `corbel_plat` thick from z_corbel to the rim; the bore drills down
    # `corbel_profondeur` from the rim.
    corbel_diametre = INSERT_M25.diametre_percage
    corbel_profondeur = INSERT_M25.profondeur_min
    corbel_paroi = INSERT_M25.epaisseur_paroi_min
    corbel_r = corbel_diametre / 2.0
    corbel_plat = corbel_diametre + corbel_paroi
    corbel_along = INSERT_M25.diametre_percage + 2*corbel_paroi
    corbel_half = corbel_along / 2.0
    z_corbel = hauteur - corbel_profondeur
    z_corbel_45 = z_corbel - corbel_plat
    ov = 0.4
    corbel_pts = [
        (ov, z_corbel_45),
        (0.0, z_corbel_45),
        (-corbel_plat, z_corbel),
        (-corbel_plat, hauteur),
        (ov, hauteur),
    ]

    corbel1_x = inner_west_marche_x
    corbel1_y = inner_north_marche_y + 10.0
    body = body + (
        Pos(corbel1_x, corbel1_y+corbel_along, 0)
        * extrude(Plane.XZ * Polygon(*[(-a,b) for a,b in corbel_pts], align=None), corbel_along)
    )
    body = body - (
        Pos(corbel1_x + corbel_plat/2, corbel1_y + corbel_paroi + corbel_r, z_corbel)
        * Cylinder(corbel_r, corbel_profondeur + 0.1, align=CMIN)
    )

    corbel2_x = chanfrein
    corbel2_y = inner_south_y
    corbel2_pts = [(-a,b) for a,b in corbel_pts]
    body = body + (
        Pos(corbel2_x, corbel2_y, 0)
        * extrude(Plane.YZ * Polygon(*corbel2_pts, align=None), corbel_along)
    )
    body = body - (
        Pos(corbel2_x + corbel_paroi + corbel_r, corbel2_y + corbel_plat/2, z_corbel)
        * Cylinder(corbel_r, corbel_profondeur + 0.1, align=CMIN)
    )

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
    dimmer, ssr = ouvertures_modules(north_y, wall)
    dimmer_y0, dimmer_y1, dimmer_z = dimmer
    ssr_y0, ssr_y1, ssr_z = ssr
    ssr_z = max(ssr_z, floor + measured("wago_epaisseur"))
    body = body - Pos(east_x - wall - margin, dimmer_y0, dimmer_z) * Box(
        wall + 2 * margin,
        dimmer_y1 - dimmer_y0,
        hauteur + margin - dimmer_z,
        align=AMIN,
    )
    body = body - Pos(east_x - wall - margin, ssr_y0, ssr_z) * Box(
        wall + 2 * margin,
        ssr_y1 - ssr_y0,
        hauteur + margin - ssr_z,
        align=AMIN,
    )

    body = _fuse_one(body)
    if draft:
        return body

    conc = {
        (round(e.center().X, 2), round(e.center().Y, 2), round(e.center().Z, 2))
        for e in concave_edges(body)
    }
    chanfrein_fente = 0.6 

    def keep(edge):
        c = edge.center()
        if (round(c.X, 2), round(c.Y, 2), round(c.Z, 2)) in conc:
            return False
        if c.Y < chanfrein:
            return True
        if c.X < west_x+wall:
            return True
        if (c.X < west_marche_x + wall) and (c.Y>north_y-wall):
            return True
        if (c.X > east_x - wall) and (c.Y < north_y-wall):
            return True
        return False

    def fente_keep(edge):
        bb = edge.bounding_box()
        dx = bb.max.X - bb.min.X
        dy = bb.max.Y - bb.min.Y
        dz = bb.max.Z - bb.min.Z
        if (dx * dx + dy * dy + dz * dz) ** 0.5 < 2.0:
            return False
        if dz < 4.0:
            return False
        return in_fente_est(bb)

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

from nurb import *

from system import (
    bbox,
    INSERT_M3, 
    _fuse_one, 
    add_well, 
    add_wall,
    add_corbel,
    offset_in, 
    AMIN, CMIN
    )

def bb_overall():
    return bbox(
        measured("boitier_int_aile_x"),
        measured("boitier_int_aile_y"),
        measured("boitier_int_x")-measured("boitier_int_aile_x"),
        measured("boitier_int_y")-measured("boitier_int_aile_y")
    )

def bb_encoche():
    return bbox(
        measured("boitier_int_screw_x"),
        measured("boitier_int_y")-measured("boitier_int_screw_dy"),
        measured("boitier_int_screw_dx"),
        measured("boitier_int_screw_dy"),
        z0=0.0,
        h=measured("boitier_int_screw_dz")
    )

# contour at z0: with the encoche
def ac_contour():
    bb = bb_overall()
    screw = bb_encoche()
    return ([
            (bb.min.X, bb.min.Y),
            (bb.max.X, bb.min.Y),
            (bb.max.X, bb.max.Y),
            (screw.max.X, screw.max.Y),
            (screw.max.X, screw.min.Y),
            (screw.min.X, screw.min.Y),
            (screw.min.X, screw.max.Y),
            (bb.min.X, bb.max.Y),
        ], bb, screw
    )

# contour at hauteur: without the encoche
def ac_top_contour():
    bb = bb_overall()
    return [
            (bb.min.X, bb.min.Y),
            (bb.max.X, bb.min.Y),
            (bb.max.X, bb.max.Y),
            (bb.min.X, bb.max.Y),
        ]

def corbels(wall):
    corbel_diameter = INSERT_M3.diametre_percage
    corbel_wall = INSERT_M3.epaisseur_paroi_min
    corbel_mid = corbel_wall + corbel_diameter/2
    corbel_half = (corbel_diameter + corbel_wall)/2
    chanfrein = measured("boitier_int_chanfrein")
    bb = bb_overall()
    cx = (bb.max.X-wall)+(bb.min.X+wall)
    return [
        (
            (bb.min.X+bb.max.X)/2 + corbel_mid,
            bb.min.Y + wall + corbel_half,
            3
        ),        
    ]

def _gousset_section(run):
    """Right triangle: wall, slab underside, 45° hypotenuse. Origin on the wall, below the slab."""
    return make_face(
        Curve()
        + [
            Line((0.0, 0.0), (0.0, run)),
            Line((0.0, run), (run, run)),
            Line((run, run), (0.0, 0.0)),
        ]
    )

def _gousset_x(x_wall, y0, y1, z_slab_bot, run, toward_plus_x):
    """Solid 45° support from a wall in X, under the slab, spanning y0..y1."""
    z0 = z_slab_bot - run
    span = y1 - y0
    if toward_plus_x:
        plane = Plane(origin=(x_wall, y1, z0), x_dir=(1, 0, 0), z_dir=(0, -1, 0))
    else:
        plane = Plane(origin=(x_wall, y0, z0), x_dir=(-1, 0, 0), z_dir=(0, 1, 0))
    return extrude(plane * _gousset_section(run), span)


def _gousset_y(y_wall, x0, x1, z_slab_bot, run, toward_plus_y):
    """Solid 45° support from a wall in Y, under the slab, spanning x0..x1."""
    z0 = z_slab_bot - run
    span = x1 - x0
    sy = 1.0 if toward_plus_y else -1.0
    plane = Plane(origin=(x0, y_wall, z0), x_dir=(0, sy, 0), z_dir=(1, 0, 0))
    return extrude(plane * _gousset_section(run), span)

def _build_encoche(body, encoche, north_y, hauteur, z0, w):
    # Drop the notch walls above the encoche in Z
    inner_west_x = encoche.min.X - w
    inner_east_x = encoche.max.X + w
    inner_south_y = encoche.min.Y - w
    slab_bottom_z = encoche.max.Z
    slab_top_z = slab_bottom_z + z0
    body = body - (
            Pos(inner_west_x, inner_south_y, slab_top_z) * 
            Box(
                inner_east_x-inner_west_x,
                north_y-inner_south_y,
                hauteur-slab_top_z,
                align=AMIN)
    )
    # Slab between the E/W walls, top flush with the wall tops.
    body = body + (
        Pos(inner_west_x, inner_south_y, slab_bottom_z) *
        Box(inner_east_x-inner_west_x, north_y-inner_south_y+w, z0, align=AMIN)
    )
    # Close the top wall
    body = body + (
        Pos(inner_west_x, north_y, slab_top_z) *
        Box(inner_east_x-inner_west_x, w, hauteur-slab_top_z, align=AMIN)
    )
    # Use goussets to reduce overhang below slab
    gousset_run = 2.0
    body = body + _gousset_x(encoche.min.X, inner_south_y, north_y+w, slab_bottom_z, gousset_run, True)
    body = body + _gousset_x(encoche.max.X, inner_south_y, north_y+w, slab_bottom_z, gousset_run, False)
    body = body + _gousset_y(encoche.min.Y, inner_west_x, inner_east_x, slab_bottom_z, gousset_run, True)

    return body

def dimmer_bb(wall, z0):
    bb = bb_overall()
    dimmer_width = measured("dimmer_width")
    dimmer_z0 = measured("boitier_int_screw_dz") + z0
    south_y = bb.max.Y-wall-dimmer_width
    # the dimmer is facing down, terminal block goes east of encoche
    x1 = bb_encoche().max.X+wall+measured("dimmer_terminal_length")
    x0 = x1 - measured("dimmer_length")
    return bbox(x0, south_y, x1-x0, dimmer_width, dimmer_z0)

def dimmer_ac_opening(wall, z0, hauteur):
    obb = bb_overall()
    bb = dimmer_bb(wall, z0)
    ac_z0 = bb.min.Z - measured("dimmer_ac_terminal_depth")
    return (bb.min.Y, ac_z0, bb.max.Y-bb.min.Y, hauteur-ac_z0)

def dimmer_dc_opening(wall, z0, hauteur):
    obb = bb_overall()
    bb = dimmer_bb(wall, z0)
    dc_z0 = bb.min.Z - measured("dimmer_dc_terminal_depth")
    return (bb.min.Y, dc_z0, bb.max.Y-bb.min.Y, hauteur-dc_z0)

def _add_dimmer_tower(body, outer, x, y0, dx, dy, z0, dz, tz):
    body = add_wall(body, outer, x, y0, dx, dy, z0, dz)
    # south tower
    body = add_wall(body, outer, x, y0-dx, dx, dx, z0, dz+tz)
    return body

def _dimmer_area(body, outer, tour_y, z0, wall):
    bb = dimmer_bb(wall, z0)
    # Y legs
    encoche = bb_encoche()
    dimmer_z0 = encoche.max.Z
    ylen = bb.max.Y - bb.min.Y
    body = _add_dimmer_tower(body, outer, encoche.min.X - wall, bb.min.Y, wall, ylen, z0, dimmer_z0, tour_y)
    body = _add_dimmer_tower(body, outer, encoche.max.X, bb.min.Y, wall, ylen, z0, dimmer_z0, tour_y)

    # room for XH connector 4mm
    left_x = bb.min.X-wall + 4.0
    body = _add_dimmer_tower(body, outer, left_x, bb.min.Y, wall, ylen, z0, dimmer_z0, tour_y)

    return body

def _cut_opening(body, x0, wall, openings):
    return body - (
        Pos(x0, openings[0], openings[1]) *
        Box(wall, openings[2], openings[3], align=AMIN)
    )

def _dimmer_openings(body, z0, wall, hauteur):
    obb = bb_overall()
    body = _cut_opening(body, obb.max.X-wall, wall, dimmer_ac_opening(wall, z0, hauteur))
    body = _cut_opening(body, obb.min.X, wall, dimmer_dc_opening(wall, z0, hauteur))

    return body

def ssr_bb(wall, z0):
    bb = bb_overall()
    # press fit the SSR against the dimmer towers
    ssr_depth = measured("ssr_depth")
    y0 = dimmer_bb(wall, z0).min.Y-wall-ssr_depth-wall

    # 1mm east of the west side
    return bbox(bb.min.X + 1.0, y0, measured("ssr_length"), ssr_depth + wall)

def ssr_ac_opening(wall, z0, hauteur):
    obb = bb_overall()
    bb = ssr_bb(wall, z0)
    ac_term_delta_y = measured("ssr_ac_terminal_shift")
    ac_z0 = measured("ssr_ac_terminal_distance")+z0
    ac_term_dy = measured("ssr_ac_terminal_depth")
    return (
        bb.max.Y - ac_term_delta_y - ac_term_dy, 
        measured("ssr_ac_terminal_distance")+z0, 
        ac_term_dy, 
        hauteur-ac_z0)

def ssr_dc_opening(wall, z0, hauteur):
    obb = bb_overall()
    bb = ssr_bb(wall, z0)
    dc_z0 = measured("ssr_dc_terminal_distance")+z0
    return (
        bb.min.Y, 
        dc_z0, 
        bb.max.Y-bb.min.Y, 
        hauteur-dc_z0)

def _ssr_area(body, outer, wall_dz, z0, wall):
    bb = ssr_bb(wall, z0)
    term_len = measured("ssr_terminal_length")

    # horizontal support traverse
    x0 = bb.min.X + 10.0
    x1 = bb.max.X - term_len/2
    body = add_wall(body, outer, x0, bb.min.Y, x1-x0, wall, z0, wall_dz)

    # perpendicular stop bar
    stop_dz = 3.0
    body = add_wall(body, outer, bb.max.X, bb.min.Y, wall, bb.max.Y-bb.min.Y, z0, stop_dz)

    return body

def _ssr_openings(body, z0, wall, hauteur):
    obb = bb_overall()
    bb = ssr_bb(wall, z0)
    body = _cut_opening(body, obb.max.X-wall, wall, ssr_ac_opening(wall, z0, hauteur))
    body = _cut_opening(body, obb.min.X, wall, ssr_dc_opening(wall, z0, hauteur))

    return body

@part
def boitier_ac(
    hauteur=27.0,
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    hauteur_muret_ssr=10.0,
    tour_y=3.0,
    draft=False,
):
    """Boîtier AC : partie est, murs 30 mm, plateforme vis et barre d'appui.

    hauteur: hauteur hors-tout depuis le lit (murs compris, linteau nord inclus)
    epaisseur_paroi: épaisseur du fond et des murs, vers l'intérieur
    epaisseur_fond: épaisseur du fond vers le haut
    hauteur_muret_ssr: hauteur du muret de soutien pour le module SSR
    tour_y: longueur des tours en Y, collées au sud des murets (X reste l'épaisseur de paroi)
    """
    wall = epaisseur_paroi
    z0 = epaisseur_fond    

    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    if hauteur < z0 + 2.0:
        reject(
            f"hauteur {hauteur} leaves under 2 mm of wall above a {wall} mm floor: "
            f"raise it above {wall + 2.0:.1f}",
            param="hauteur",
        )

    (
        outer_pts_outer, bb, encoche
    )  = ac_contour()
    # Build cavity from the shifted outer envelope as well: it keeps the
    # wall thickness consistent and avoids degenerate ultra-thin east walls
    # that can crash the polish/border analysis in nurb.
    inner_pts = offset_in(outer_pts_outer, wall)
    inner_west_x = bb.min.X + wall
    inner_east_x = bb.max.X - wall
    inner_north_y = bb.max.Y - wall
    inner_south_y = bb.min.Y + wall
    inner_west_encoche_x = encoche.min.X - wall
    inner_east_encoche_x = encoche.max.X + wall
    inner_south_encoche_y = encoche.min.Y - wall

    outer = extrude(Polygon(*outer_pts_outer, align=None), hauteur)
    cavity = Pos(0, 0, z0) * extrude(
        Polygon(*inner_pts, align=None), hauteur + 0.2
    )
    body = outer - cavity

    body = _build_encoche(body, encoche, inner_north_y, hauteur, z0, wall)
    
    # The support legs for the dimmer area
    body = _dimmer_area(body, outer, tour_y, z0, wall)
    body = _dimmer_openings(body, z0, wall, hauteur)

    # Two magnet wells
    body = add_well(
        body, 
        outer,
        inner_west_x+(inner_east_x-inner_west_x)*0.3, 
        inner_north_y-6.5)
    body = add_well(
        body, 
        outer,
        inner_east_x-8,
        inner_north_y-6.5)

    # The stop walls for the SSR
    body = _ssr_area(body, outer, hauteur_muret_ssr, z0, wall)
    body = _ssr_openings(body, z0, wall, hauteur)

    # Corbel
    body = add_corbel(body,(inner_east_x + inner_west_x)/2.0,inner_south_y,hauteur,plane=Plane.YZ)

    body = _fuse_one(body)
    if draft:
        return body

    def in_fente(bb):
        # Keep both jamb edges (inner + outer) after east-slot X shifts.
        on_est = (
            bb.max.X > x_east_fente - wall - margin - 0.2
            and bb.min.X < x_east_fente + margin + 0.2
        )
        my = 0.5 * (bb.min.Y + bb.max.Y)
        se_mid = 0.5 * (y_se0 + y_se1)
        ne_mid = 0.5 * (y_ne0 + y_ne1)
        se = (
            abs(my - se_mid) <= 0.5 * fente_sud_est + 0.8
            and bb.min.Z > fente_sud_est_z - 0.5
        )
        ne = (
            abs(my - ne_mid) <= 0.5 * fente_nord_est + 0.8
            and bb.min.Z > fente_nord_est_z - 0.5
        )
        return on_est and (se or ne)

    def in_fente_ouest(bb):
        on_ouest = (
            bb.max.X > x_outer_west - margin - 0.2
            and bb.min.X < aile_x + margin + 0.2
        )
        my = 0.5 * (bb.min.Y + bb.max.Y)
        mid = 0.5 * (ssr_y0 + ssr_y1)
        y_tol = 0.5 * (ssr_y1 - ssr_y0) + 0.8
        return (
            on_ouest
            and abs(my - mid) <= y_tol
            and bb.min.Z > ssr_z - 0.5
        )    

    def keep(edge):
        c = edge.center()
        if c.X > inner_east_x:
            return True
        if (c.X < inner_west_x) and (c.Y > inner_south_y) and (c.Y < inner_north_y):
            return True
        return False

    def fente_keep(edge):
        bb = edge.bounding_box()
        dx = bb.max.X - bb.min.X
        dy = bb.max.Y - bb.min.Y
        dz = bb.max.Z - bb.min.Z
        if (dx * dx + dy * dy + dz * dz) ** 0.5 < 2.0:
            return False
        if dz < 8.0:
            return False
        return in_fente(bb) or in_fente_ouest(bb)

    body = polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)
    # body = polish(body, body.edges().filter_by(fente_keep), chanfrein_fente)
    return body

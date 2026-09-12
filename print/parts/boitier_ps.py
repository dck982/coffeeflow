from nurb import *

from system import (
    INSERT_M3,
    AMIN,
    _fuse_one,
    add_well,
    add_wall,
    add_corbel,
    add_hook,
    bbox,
    surplomb,
    surplomb_xz,
)

# Layout wall/floor used by bb_overall — not a constructor param. Keep in
# step with the epaisseur_paroi / epaisseur_fond defaults (1.6).
_WALL = 1.6
_SURPLOMB_W = 1.0
_REGLETTE_PSU_Z = 2.0
_PUIT_X = 10.0
_PUIT_Y_SUD = 30.0
_PUIT_Y_NORD = 60.0
_INSERT_SUD_DEPUIS_EST = 10.0
_HOOK_JEU = 1.2
_HOOK_LARGEUR = 3.0
_HOOK_BORDS = 5.6
_HOOK_Z = 5.0
_OV = 0.4


def bb_overall():
    """Inner cavity, origin at the inner south-west corner."""
    w = _WALL
    inner_x = measured("recom_rac05_x") + measured("recom_rac05_rayon_min")
    inner_y = (
        measured("helu145_rayon_min")
        + measured("boitier_ps_wago_entre_murets")
        + w
        + measured("recom_rac05_y")
        + w
        + 2 * measured("wago_epaisseur_corps")
    )
    return bbox(0, 0, inner_x, inner_y)


def bb_wago_south():
    """221-415 bay, east-aligned, south of the battery wall."""
    bb = bb_overall()
    dx = measured("boitier_ps_wago_largeur")
    dy = measured("boitier_ps_wago_entre_murets")
    return bbox(bb.max.X - dx, measured("helu145_rayon_min"), dx, dy)


def bb_psu():
    """RECOM RAC05-05SK pocket, east-aligned, between the two Wago bays."""
    bb = bb_overall()
    w = _WALL
    dx = measured("recom_rac05_x")
    dy = measured("recom_rac05_y")
    dz = measured("recom_rac05_z")
    return bbox(bb.max.X - dx, bb_wago_south().max.Y + w, dx, dy, h=dz)


def bb_wago_north():
    """Two 221-423 on edge, east-aligned, north of the PSU wall."""
    bb = bb_overall()
    w = _WALL
    dx = measured("wago_profondeur")
    dy = 2 * measured("wago_epaisseur_corps")
    return bbox(bb.max.X - dx, bb_psu().max.Y + w, dx, dy)


def corbels(wall):
    """Hole (cx, cy, d) and XY bbox of the pad, same origins as `add_corbel`."""
    along = INSERT_M3.diametre_percage + 2 * INSERT_M3.epaisseur_paroi_min
    plat = INSERT_M3.diametre_percage + INSERT_M3.epaisseur_paroi_min
    r = INSERT_M3.diametre_percage / 2.0
    half = along / 2.0
    bb = bb_overall()
    x_sud = bb.max.X - _INSERT_SUD_DEPUIS_EST
    return [
        (r, _PUIT_Y_NORD, 3, bbox(0.0, _PUIT_Y_NORD - half, plat, along)),
        (
            x_sud - wall - r,
            r,
            3,
            bbox(x_sud - along, 0.0, along, plat),
        ),
    ]


def _cut_opening(body, x, y, dx, dy, z0, h):
    return body - (
        Pos(x, y, z0) * Box(dx, dy, h, align=AMIN)
    )


@part
def boitier_ps(
    hauteur=40.0,
    epaisseur_paroi=1.6,
    epaisseur_fond=1.6,
    wago_raise=15.0,
    wago_surplomb=8.0,
    largeur_passage_cable=5.0,
    draft=False,
):
    """Boîtier alim : RAC05-05SK, 221-415 sud et deux 221-423 nord.

    hauteur: murs de pourtour, depuis le lit
    epaisseur_paroi: murs, 1,6 mm
    epaisseur_fond: fond, 1,6 mm
    wago_raise: hauteur des rails sous les Wago
    wago_surplomb: longueur du retour, 8 mm, les deux logements
    largeur_passage_cable: canal au nord, un fond et un mur
    """
    wall = epaisseur_paroi
    z0 = epaisseur_fond
    bb = bb_overall()
    south = bb_wago_south()
    psu = bb_psu()
    north = bb_wago_north()
    inner_x = bb.size.X
    inner_y = bb.size.Y

    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    if hauteur < z0 + 8.0:
        reject(
            f"hauteur {hauteur} leaves under 8 mm of wall above the floor: "
            f"raise it above {z0 + 8.0:.1f}",
            param="hauteur",
        )
    if wago_raise < 1.0:
        reject(
            f"wago_raise {wago_raise} is under 1 mm: raise it",
            param="wago_raise",
        )
    if wago_surplomb < 2.0:
        reject(
            f"wago_surplomb {wago_surplomb} is under 2 mm: raise it",
            param="wago_surplomb",
        )

    muret_z = measured("boitier_ps_wago_muret_z")
    rail_ecart = measured("boitier_ps_wago_rail_ecart")
    ouv_z = measured("boitier_ps_ouverture_z")
    ouv_bas = measured("boitier_ps_ouverture_bas")
    ouv_ouest = measured("boitier_ps_ouverture_gauche")
    ouv_ouest_n = measured("boitier_ps_ouverture_gauche_haut")
    hook_x = measured("boitier_ps_anti_tirage_x")
    psu_z = measured("recom_rac05_z")

    outer = Pos(-wall, -wall, 0) * Box(
        inner_x + 2.0 * wall, inner_y + 2.0 * wall, hauteur, align=AMIN
    )
    body = outer - Pos(0, 0, z0) * Box(inner_x, inner_y, hauteur, align=AMIN)

    # Separator walls: battery (south Wago / PSU) and PSU north (PSU / north Wago).
    x_l = south.min.X - wall
    body = add_wall(
        body, outer,
        x_l, psu.min.Y - wall,
        inner_x + wall - x_l, wall,
        z0, muret_z,
    )
    body = add_wall(
        body, outer,
        psu.min.X - _OV, psu.max.Y,
        psu.size.X + wall + _OV, wall,
        z0, psu_z,
    )

    # PSU west stop, south Wago side wall + stop, north Wago west stop.
    body = add_wall(
        body, outer,
        psu.min.X - wall, psu.min.Y,
        wall, psu.size.Y,
        z0, _REGLETTE_PSU_Z,
    )
    body = add_wall(
        body, outer,
        x_l, south.min.Y - wall,
        inner_x + 2.0 * wall - south.min.X, wall,
        z0, wago_raise,
    )
    body = add_wall(
        body, outer,
        x_l, south.min.Y - wall,
        wall, psu.min.Y - (south.min.Y - wall),
        z0, muret_z,
    )
    body = add_wall(
        body, outer,
        north.min.X - wall, north.min.Y,
        wall, north.size.Y,
        z0, wago_raise,
    )

    # Rails: 5 mm off the side walls so those walls can flex.
    body = add_wall(
        body, outer,
        south.min.X + rail_ecart, south.min.Y,
        wall, south.size.Y,
        z0, wago_raise,
    )
    body = add_wall(
        body, outer,
        bb.max.X - rail_ecart - wall, south.min.Y,
        wall, south.size.Y,
        z0, wago_raise,
    )
    body = add_wall(
        body, outer,
        north.min.X, north.min.Y + rail_ecart,
        north.size.X + wall, wall,
        z0, wago_raise,
    )
    body = add_wall(
        body, outer,
        north.min.X, bb.max.Y - rail_ecart - wall,
        north.size.X + wall, wall,
        z0, wago_raise,
    )

    # Two catches: south bay from the battery wall, north bay from the outer north wall.
    z_catch_s = z0 + muret_z - _SURPLOMB_W
    body = body + surplomb_xz(
        south.min.X, psu.min.Y, z_catch_s, _SURPLOMB_W, wall + wago_surplomb,
        0.0, inverse=True, overlap=_OV,
    )
    z_catch_n = z0 + wago_raise + measured("wago_423_largeur")
    body = body + surplomb(
        bb.max.Y, bb.max.X - wago_surplomb, z_catch_n, _SURPLOMB_W,
        wago_surplomb + wall,
        plane=Plane.YZ, overlap=_OV,
    )

    for cx, cy in (
        (south.center().X, south.center().Y),
        (_PUIT_X, _PUIT_Y_SUD),
        (_PUIT_X, _PUIT_Y_NORD),
    ):
        body = add_well(body, outer, cx, cy)

    z_ouv = z0 + ouv_z
    body = add_hook(
        body, hook_x, 0.0, z_ouv, ns=True, toward_plus=True, wall=wall,
        jeu=_HOOK_JEU, largeur=_HOOK_LARGEUR, bords=_HOOK_BORDS, hauteur_u=_HOOK_Z,
    )
    body = add_hook(
        body, hook_x, inner_x, z_ouv, ns=False, toward_plus=False, wall=wall,
        jeu=_HOOK_JEU, largeur=_HOOK_LARGEUR, bords=_HOOK_BORDS, hauteur_u=_HOOK_Z,
    )
    body = add_hook(
        body, hook_x, inner_y, z_ouv, ns=True, toward_plus=False, wall=wall,
        jeu=_HOOK_JEU, largeur=_HOOK_LARGEUR, bords=_HOOK_BORDS, hauteur_u=_HOOK_Z,
    )

    along = INSERT_M3.diametre_percage + 2 * INSERT_M3.epaisseur_paroi_min
    body = add_corbel(
        body, 0.0, _PUIT_Y_NORD - along / 2.0, hauteur, flush=True,
    )
    body = add_corbel(
        body, inner_x - _INSERT_SUD_DEPUIS_EST - along, 0.0, hauteur,
        plane=Plane.YZ, flush=True,
    )

    # Openings last, through the outer walls. Height overshoots the rim.
    margin = 0.5
    ouv_x0 = inner_x - ouv_bas
    y_nw_min = inner_y - ouv_ouest_n
    body = _cut_opening(
        body, ouv_x0, -wall - margin, ouv_bas, wall + 2.0 * margin, z_ouv, hauteur,
    )
    body = _cut_opening(
        body, -wall - margin, 0.0, wall + 2.0 * margin, ouv_ouest, z_ouv, hauteur,
    )
    body = _cut_opening(
        body, -wall - margin, y_nw_min, wall + 2.0 * margin, ouv_ouest_n, z_ouv, hauteur,
    )

    # North cable channel: floor + north wall, closed by the lid. Unpolished.
    body = body + (
        Pos(-wall, inner_y + wall, 0)
        * Box(inner_x + 2 * wall, largeur_passage_cable + wall, z0, align=AMIN)
    )
    body = body + (
        Pos(-wall, inner_y + wall + largeur_passage_cable, z0)
        * Box(inner_x + 2 * wall, wall, hauteur, align=AMIN)
    )

    body = _fuse_one(body)
    if draft:
        return body

    bed = body.bounding_box().min.Z
    x_min = -wall
    x_max = inner_x + wall
    y_min = -wall
    y_max = inner_y + wall
    chanfrein_ouv = 0.6
    at_u0 = z_ouv
    at_u1 = at_u0 + _HOOK_Z
    at_span = _HOOK_BORDS
    at_x0 = hook_x
    at_y0 = hook_x

    def in_south_ouv(ebb):
        return (
            ebb.min.X > ouv_x0 - 0.5
            and ebb.max.X < inner_x + 0.5
            and ebb.min.Y > y_min - 0.5
            and ebb.max.Y < 0.5
            and ebb.min.Z > z_ouv - 0.5
        )

    def in_west_sud(ebb):
        return (
            ebb.min.X > x_min - 0.5
            and ebb.max.X < 0.5
            and ebb.min.Y > -0.5
            and ebb.max.Y < ouv_ouest + 0.5
            and ebb.min.Z > z_ouv - 0.5
        )

    def in_west_nord(ebb):
        return (
            ebb.min.X > x_min - 0.5
            and ebb.max.X < 0.5
            and ebb.min.Y > y_nw_min - 0.5
            and ebb.max.Y < inner_y + 0.5
            and ebb.min.Z > z_ouv - 0.5
        )

    def opening_keep(edge):
        ebb = edge.bounding_box()
        dx = ebb.max.X - ebb.min.X
        dy = ebb.max.Y - ebb.min.Y
        dz = ebb.max.Z - ebb.min.Z
        if (dx * dx + dy * dy + dz * dz) ** 0.5 < 2.0:
            return False
        return in_south_ouv(ebb) or in_west_sud(ebb) or in_west_nord(ebb)

    def pont_passage(edge):
        ebb = edge.bounding_box()
        dx = ebb.max.X - ebb.min.X
        dy = ebb.max.Y - ebb.min.Y
        dz = ebb.max.Z - ebb.min.Z
        span = (dx * dx + dy * dy + dz * dz) ** 0.5
        if abs(span - _HOOK_LARGEUR) > 0.4:
            return False
        if dz > 0.4:
            return False
        zmid = 0.5 * (ebb.min.Z + ebb.max.Z)
        if abs(zmid - at_u0) > 0.3 and abs(zmid - at_u1) > 0.3:
            return False
        mx = 0.5 * (ebb.min.X + ebb.max.X)
        my = 0.5 * (ebb.min.Y + ebb.max.Y)
        mid = at_x0 + 0.5 * at_span
        y_mid = at_y0 + 0.5 * at_span
        sud = abs(my - _HOOK_JEU) < 0.3 and abs(mx - mid) < 2.0
        est = abs(mx - (inner_x - _HOOK_JEU)) < 0.3 and abs(my - y_mid) < 2.0
        nord = abs(my - (inner_y - _HOOK_JEU)) < 0.3 and abs(mx - mid) < 2.0
        return sud or est or nord

    def box_keep(edge):
        ebb = edge.bounding_box()
        if ebb.min.Z < bed - 0.05:
            return False
        mx = 0.5 * (ebb.min.X + ebb.max.X)
        my = 0.5 * (ebb.min.Y + ebb.max.Y)
        on_x = abs(mx - x_min) < 0.4 or abs(mx - x_max) < 0.4
        on_y = abs(my - y_min) < 0.4 or abs(my - y_max) < 0.4
        return on_x and on_y

    body = polish(body, body.edges().filter_by(opening_keep), chanfrein_ouv)
    body = polish(body, body.edges().filter_by(pont_passage), 0.4)
    return polish(body, body.edges().filter_by(Axis.Z).filter_by(box_keep), 1.0)

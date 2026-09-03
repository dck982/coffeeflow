from nurb import *

from system import MARGE_PUIT, _fuse_one, add_well, offset_in


def _contour():
    aile_x = measured("boitier_int_aile_x")
    y_max = measured("boitier_int_y")
    chanfrein = measured("boitier_int_chanfrein")
    marche_x = measured("boitier_int_marche_x")
    marche_y = measured("boitier_int_marche_y") + 3.0
    return [
        (0.0, chanfrein),
        (chanfrein, 0.0),
        (aile_x, 0.0),
        (aile_x, y_max),
        (marche_x, y_max),
        (marche_x, marche_y),
        (0.0, marche_y),
    ]


@part
def boitier_dc(
    hauteur=25.0,
    epaisseur_paroi=1.6,
    puit_diametre=8.2,
    puit_peau=0.6,
    marge_puit=MARGE_PUIT,
    draft=False,
):
    """Boîtier DC : partie ouest, face est à x = 50, nord à y = 95.

    hauteur: hauteur hors-tout depuis le lit (murs compris)
    epaisseur_paroi: épaisseur du fond et des murs, vers l'intérieur
    puit_diametre: diamètre intérieur du puits d'aimant Ø8×3
    puit_peau: plastique sous l'aimant
    marge_puit: plastique autour du puits (doctrine 1,6 mm)
    """
    wall = epaisseur_paroi
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")
    puit_bas_x = measured("boitier_int_puit_bas_x")
    puit_bas_y = measured("boitier_int_puit_bas_y")

    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    if hauteur < wall + 2.0:
        reject(
            f"hauteur {hauteur} leaves under 2 mm of wall above a {wall} mm floor: "
            f"raise it above {wall + 2.0:.1f}",
            param="hauteur",
        )
    if puit_diametre < aimant_d + 0.1:
        reject(
            f"puit_diametre {puit_diametre} is too tight for an {aimant_d} mm magnet",
            param="puit_diametre",
        )
    if puit_peau < 0.4:
        reject(
            f"puit_peau {puit_peau} would knife-edge the well floor: raise it above 0.4",
            param="puit_peau",
        )
    if marge_puit < 1.2:
        reject(
            f"marge_puit {marge_puit} is under 1.2 mm: raise it",
            param="marge_puit",
        )
    well_stack = puit_peau + aimant_h
    if hauteur < well_stack + 0.4:
        reject(
            f"hauteur {hauteur} is under the magnet well ({well_stack + 0.4:.1f} mm): "
            "raise it",
            param="hauteur",
        )

    outer_pts = _contour()
    inner_pts = offset_in(outer_pts, wall)

    outer = extrude(Polygon(*outer_pts, align=None), hauteur)
    cavity = Pos(0, 0, wall) * extrude(
        Polygon(*inner_pts, align=None), hauteur + 0.2
    )
    body = outer - cavity

    body = add_well(
        body,
        outer,
        puit_bas_x,
        puit_bas_y - 6.0,
        puit_diametre,
        puit_peau,
        aimant_h,
        marge_puit,
    )

    # Second magnet well: 30mm from the north face, along the west wall.
    # In the NW zone the west face is at x=marche_x (12), not x=0.
    y_n = measured("boitier_int_y")
    puit2_cy = y_n - 26.5  # pad fused into the NW Wago sill
    pad_r = puit_diametre / 2.0 + marge_puit
    marche_x_val = measured("boitier_int_marche_x")
    puit2_cx = marche_x_val + pad_r  # pad fused into the marche west wall
    body = add_well(
        body,
        outer,
        puit2_cx,
        puit2_cy,
        puit_diametre,
        puit_peau,
        aimant_h,
        marge_puit,
    )

    # Two M2.5 heat-insert housings in the top NE corner.
    insert_depth = 5.0
    insert_hole_d = measured("insert_m25_exterieur")
    hole_r = insert_hole_d / 2.0
    ring_outer_r = hole_r + wall
    z_insert0 = wall
    x_face_clear = 1.5
    x_e = measured("boitier_int_aile_x")
    y_n2 = measured("boitier_int_y")
    x_inner_e = x_e - wall
    y_inner_n = y_n2 - wall
    insert1_cx = x_inner_e - x_face_clear - hole_r + 1.5
    insert1_cy = y_inner_n - x_face_clear - hole_r + 1.5
    insert2_cx = insert1_cx - 16.0
    insert2_cy = insert1_cy
    cyl_amin = (Align.CENTER, Align.CENTER, Align.MIN)
    _amin = (Align.MIN, Align.MIN, Align.MIN)
    for cx, cy in ((insert1_cx, insert1_cy), (insert2_cx, insert2_cy)):
        outer_pad = Pos(cx, cy, z_insert0) * Cylinder(
            ring_outer_r, insert_depth, align=cyl_amin
        )
        inner_void = Pos(cx, cy, z_insert0) * Cylinder(
            hole_r, insert_depth, align=cyl_amin
        )
        ring = outer_pad - inner_void
        clipped = ring.intersect(outer)
        if clipped is not None:
            body = body + clipped
        body = body - inner_void

    # Two support walls running 10mm south from each insert annulus.
    support_run = 10.0
    for cx in (insert1_cx, insert2_cx):
        y_top = insert1_cy - ring_outer_r
        y_bot = y_top - support_run
        body = body + Pos(cx - wall / 2.0, y_bot, z_insert0) * Box(
            wall, y_top - y_bot, insert_depth, align=_amin
        )

    # Four M2.5 heat-insert housings for the second module.
    # Coordinates from the SE corner of the module (x=aile_x, y=0).
    mod2_inserts = [
        (x_e - 3.9, 25.6),   # SE
        (x_e - 3.9, 60.6),   # NE
        (x_e - 23.9, 60.6),  # NW
        (x_e - 23.9, 25.6),  # SW
    ]
    for cx, cy in mod2_inserts:
        outer_pad = Pos(cx, cy, z_insert0) * Cylinder(
            ring_outer_r, insert_depth, align=cyl_amin
        )
        inner_void = Pos(cx, cy, z_insert0) * Cylinder(
            hole_r, insert_depth, align=cyl_amin
        )
        ring = outer_pad - inner_void
        clipped = ring.intersect(outer)
        if clipped is not None:
            body = body + clipped
        body = body - inner_void

    # Wago 221-423 bay in the NW corner.
    # In the NW zone (y > marche_y), the west face is at x=marche_x, not x=0.
    # Raised 2mm above the floor on a platform.
    marche_x = measured("boitier_int_marche_x")
    wago_depth = 8.4
    wago_span = 18.6
    wago_z = 18.8
    wago_raise = 2.0  # platform height above the floor
    overlap = 0.4
    x_inner_w_nw = marche_x + wall  # inner west face in NW zone
    y_inner_n_w = y_n - wall
    muret_x0 = x_inner_w_nw + wago_depth
    muret_y0 = y_inner_n_w - wago_span
    z_wago_floor = wall + wago_raise  # raised floor level

    # Platform: solid block filling the bay footprint, from the real floor
    # up to the raised floor level.
    platform_box = Pos(marche_x - overlap, muret_y0, -overlap) * Box(
        wago_depth + wall + 2 * overlap, wago_span + wall + overlap,
        z_wago_floor + overlap, align=_amin
    )
    body = body + platform_box.intersect(outer)

    # East muret, from the real floor, height = wago_z + wago_raise
    muret_box = Pos(muret_x0, muret_y0, -overlap) * Box(
        wall, wago_span + wall + overlap, wago_z + wago_raise + wall + overlap, align=_amin
    )
    body = body + muret_box.intersect(outer)

    # Surplomb (catch): 1mm return from the muret toward the west wall,
    # at the top of the muret, 45° below. 8mm long in Y, placed against
    # the north inner face (so the Wago can be tilted in from the south).
    surplomb = 1.0
    surplomb_len = 8.0
    z_top_w = z_wago_floor + wago_z
    z_catch = z_top_w - surplomb
    z_45 = z_catch - surplomb
    hook_pts = [
        (muret_x0 + overlap, z_45),
        (muret_x0 - surplomb, z_catch),
        (muret_x0 - surplomb, z_top_w),
        (muret_x0 + overlap, z_top_w),
    ]
    hook_y0 = y_inner_n_w - surplomb_len
    hook_solid = (
        Pos(0, hook_y0, 0)
        * extrude(
            Plane.XZ * Polygon(*hook_pts, align=None),
            surplomb_len + wall + overlap,
        )
    )
    body = body + hook_solid.intersect(outer)

    # Entry sill: 1mm above the raised floor, closing the south end.
    seuil_z = 1.0
    seuil_y0 = muret_y0
    seuil_box = Pos(marche_x - overlap, seuil_y0 - wall, -overlap) * Box(
        wago_depth + wall + 2 * overlap, wall, seuil_z + wago_raise + wall + overlap, align=_amin
    )
    body = body + seuil_box.intersect(outer)

    # Second Wago bay: angle of the marche (SW zone, against west wall x=0).
    # Two terminals (3-way + 2-way), muret depth 16.8mm, span 18.6mm in Y,
    # muret height 13.2mm. Two surplombs: one at muret z=13.2 (small terminal),
    # one against the west wall at z=18.8 (large terminal).
    wago2_depth = 16.8
    wago2_span = 18.6
    wago2_z = 13.2       # muret height (small terminal)
    wago2_z_big = 18.8   # surplomb height for the large terminal
    x_inner_w_sw = wall  # inner west face in SW zone
    marche_y_val = measured("boitier_int_marche_y") + 3.0
    y_inner_marche = marche_y_val - wall  # inner south face of the marche wall
    muret2_x0 = x_inner_w_sw + wago2_depth
    muret2_y0 = y_inner_marche - wago2_span

    # East muret
    muret2_box = Pos(muret2_x0, muret2_y0, -overlap) * Box(
        wall, wago2_span + wall + overlap, wago2_z + wall + overlap, align=_amin
    )
    body = body + muret2_box.intersect(outer)

    # Surplomb 1: on the muret (east side), at z = wall + wago2_z,
    # 8mm long against the marche wall (north end of the bay).
    z_top_w2 = wall + wago2_z
    z_catch2 = z_top_w2 - surplomb
    z_45_2 = z_catch2 - surplomb
    hook2_pts = [
        (muret2_x0 + overlap, z_45_2),
        (muret2_x0 - surplomb, z_catch2),
        (muret2_x0 - surplomb, z_top_w2),
        (muret2_x0 + overlap, z_top_w2),
    ]
    hook2_y0 = y_inner_marche - surplomb_len
    hook2_solid = (
        Pos(0, hook2_y0, 0)
        * extrude(
            Plane.XZ * Polygon(*hook2_pts, align=None),
            surplomb_len + wall + overlap,
        )
    )
    body = body + hook2_solid.intersect(outer)

    # Surplomb 2: against the west wall (x=0 side) at z = wall + 18.8,
    # for the larger terminal. Hook runs along Y, profile in YZ plane.
    z_top_w2b = wall + wago2_z_big
    z_catch2b = z_top_w2b - surplomb
    z_45_2b = z_catch2b - surplomb
    hook2b_pts = [
        (x_inner_w_sw - overlap, z_45_2b),
        (x_inner_w_sw + surplomb, z_catch2b),
        (x_inner_w_sw + surplomb, z_top_w2b),
        (x_inner_w_sw - overlap, z_top_w2b),
    ]
    hook2b_solid = (
        Pos(0, hook2_y0, 0)
        * extrude(
            Plane.XZ * Polygon(*hook2b_pts, align=None),
            surplomb_len + wall + overlap,
        )
    )
    body = body + hook2b_solid.intersect(outer)

    # Entry sill for second Wago bay
    seuil2_y0 = muret2_y0
    seuil2_box = Pos(-overlap, seuil2_y0 - wall, -overlap) * Box(
        wago2_depth + wall + 2 * overlap, wall, wall + seuil_z + overlap, align=_amin
    )
    body = body + seuil2_box.intersect(outer)

    # Chamfer wall (0, 20) → (20, 0): open the low-X half, leftmost
    # corner to the midpoint. Floor stays.
    chanfrein = measured("boitier_int_chanfrein")
    s2 = 2.0 ** 0.5
    tx, ty = 1.0 / s2, -1.0 / s2
    nx, ny = 1.0 / s2, 1.0 / s2
    ax, ay = 0.0, chanfrein
    mx, my = chanfrein / 2.0, chanfrein / 2.0
    past = 1.0
    inn = wall + 2.0
    margin = 0.5
    body = body - (
        Pos(0, 0, wall)
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

    body = _fuse_one(body)
    if draft:
        return body

    conc = {
        (round(e.center().X, 2), round(e.center().Y, 2), round(e.center().Z, 2))
        for e in concave_edges(body)
    }

    def keep(edge):
        c = edge.center()
        return (round(c.X, 2), round(c.Y, 2), round(c.Z, 2)) not in conc

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

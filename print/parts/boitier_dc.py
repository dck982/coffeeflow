from nurb import *

from system import MARGE_PUIT, _fuse_one, add_well, offset_in, ouvertures_modules


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
    hauteur=24.0,
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

    # Move the SW well northwest on a 45° line and fuse it to the sill with
    # the same proven recipe as the NW well: bore edge 0.2mm under the sill,
    # hence the full 1.6mm crown plus 0.2mm of overlap.
    wago_span = 18.6
    pad_r = puit_diametre / 2.0 + marge_puit
    puit_bas_cy0 = puit_bas_y - 6.0
    seuil2_south_y = (
        measured("boitier_int_marche_y") + 3.0 - wall - wago_span - wall
    )
    puit_bas_target_cy = seuil2_south_y - puit_diametre / 2.0 + 0.2
    puit_bas_shift = puit_bas_target_cy - puit_bas_cy0
    puit_bas_cx = puit_bas_x - puit_bas_shift
    puit_bas_cy = puit_bas_target_cy
    body = add_well(
        body,
        outer,
        puit_bas_cx,
        puit_bas_cy,
        puit_diametre,
        puit_peau,
        aimant_h,
        marge_puit,
    )

    # Second magnet well: west wall of the marche, fused to the NW Wago
    # sill. Centre so the well circle meets the sill's south face, then
    # 0.2 mm into the sill so the pad does not leave a sliver at the west.
    y_n = measured("boitier_int_y")
    seuil_y_sud = y_n - 2.0 * wall - wago_span
    puit2_cy = seuil_y_sud - puit_diametre / 2.0 + 0.2
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
    insert1_cx = x_inner_e - x_face_clear - hole_r + 1.5 - 2.0
    insert1_cy = y_inner_n - x_face_clear - hole_r + 1.5 - 2.5
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

    # Three M2.5 heat-insert housings for the second module.
    # Coordinates from the SE corner of the module (x=aile_x, y=0).
    mod2_inserts = [
        (x_e - 3.9, 25.6),   # SE
        (x_e - 3.9, 60.6),   # NE
        (x_e - 23.9, 60.6),  # NW
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

    # SW board support: solid Ø3mm pin instead of an insert housing, to clear
    # the component beneath the board while the other three screws retain it.
    pin_sw_cx = x_e - 23.9
    pin_sw_cy = 25.6
    pin_sw = Pos(pin_sw_cx, pin_sw_cy, z_insert0) * Cylinder(
        1.5, insert_depth, align=cyl_amin
    )
    body = body + pin_sw.intersect(outer)

    # Wago 221-423 bay in the NW corner.
    # In the NW zone (y > marche_y), the west face is at x=marche_x, not x=0.
    # Raised 2mm above the floor on a platform.
    marche_x = measured("boitier_int_marche_x")
    wago_depth = 8.4
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

    # East muret, from the real floor, 1mm above the Wago so the catch
    # descends onto its top instead of ending against its side.
    surplomb = 1.0
    muret_box = Pos(muret_x0, muret_y0, -overlap) * Box(
        wall,
        wago_span + wall + overlap,
        wago_z + wago_raise + wall + overlap + surplomb,
        align=_amin,
    )
    body = body + muret_box.intersect(outer)

    # Surplomb (catch): 1mm return from the muret toward the west wall,
    # with its vertical face starting at the Wago top and its 45° lead-in
    # starting 1mm below. 8mm long in Y, against the north inner face.
    surplomb_len = 8.0
    z_top_w = z_wago_floor + wago_z + surplomb
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

    # Re-open the full north Ø8.2 pocket after its sill/platform fusion.
    puit2_cutter = Pos(puit2_cx, puit2_cy, puit_peau) * Cylinder(
        puit_diametre / 2.0,
        hauteur,
        align=cyl_amin,
    )
    body = body - puit2_cutter

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
    z_wago2_floor = wall + wago_raise

    # Same raised platform as the NW bay: 2mm above the inner floor.
    platform2_box = Pos(-overlap, muret2_y0, -overlap) * Box(
        wago2_depth + wall + 2 * overlap,
        wago2_span + wall + overlap,
        z_wago2_floor + overlap,
        align=_amin,
    )
    body = body + platform2_box.intersect(outer)

    # East muret: 1mm above the small Wago so its catch clips over the top.
    muret2_box = Pos(muret2_x0, muret2_y0, -overlap) * Box(
        wall,
        wago2_span + wall + overlap,
        wago2_z + wago_raise + wall + overlap + surplomb,
        align=_amin,
    )
    body = body + muret2_box.intersect(outer)

    # Surplomb 1: on the muret (east side), its vertical face starts at
    # the raised Wago top; 8mm long against the north end of the bay.
    z_top_w2 = z_wago2_floor + wago2_z + surplomb
    z_catch2 = z_top_w2 - surplomb
    z_45_2 = z_catch2 - surplomb
    hook2_pts = [
        (muret2_x0 + overlap, z_45_2),
        (muret2_x0 - surplomb, z_catch2),
        (muret2_x0 - surplomb, z_top_w2),
        (muret2_x0 + overlap, z_top_w2),
    ]
    hook2_east_y0 = y_inner_marche - surplomb_len
    hook2_solid = (
        Pos(0, hook2_east_y0, 0)
        * extrude(
            Plane.XZ * Polygon(*hook2_pts, align=None),
            surplomb_len + wall + overlap,
        )
    )
    body = body + hook2_solid.intersect(outer)

    # Surplomb 2: against the west wall for the larger terminal. Its vertical
    # face starts at the raised Wago top; hook runs along Y.
    z_top_w2b = z_wago2_floor + wago2_z_big + surplomb
    z_catch2b = z_top_w2b - surplomb
    z_45_2b = z_catch2b - surplomb
    hook2b_pts = [
        (x_inner_w_sw - overlap, z_45_2b),
        (x_inner_w_sw + surplomb, z_catch2b),
        (x_inner_w_sw + surplomb, z_top_w2b),
        (x_inner_w_sw - overlap, z_top_w2b),
    ]
    # The mirrored west profile extrudes toward -Y, unlike the east profile.
    # Start beyond the north wall so its exposed 8mm aligns with the east catch.
    hook2_west_y0 = y_inner_marche + wall + overlap
    hook2b_solid = (
        Pos(0, hook2_west_y0, 0)
        * extrude(
            Plane.XZ * Polygon(*hook2b_pts, align=None),
            surplomb_len + wall + overlap,
        )
    )
    body = body + hook2b_solid.intersect(outer)

    # Entry sill for second Wago bay
    seuil2_y0 = muret2_y0
    seuil2_box = Pos(-overlap, seuil2_y0 - wall, -overlap) * Box(
        wago2_depth + wall + 2 * overlap,
        wall,
        wall + wago_raise + seuil_z + overlap,
        align=_amin,
    )
    body = body + seuil2_box.intersect(outer)

    # Re-open the full Ø8.2 pocket after fusing the sill, which otherwise
    # intrudes 0.2mm into the bore. Preserve the 0.6mm magnet skin below.
    puit_bas_cutter = Pos(puit_bas_cx, puit_bas_cy, puit_peau) * Cylinder(
        puit_diametre / 2.0,
        hauteur,
        align=cyl_amin,
    )
    body = body - puit_bas_cutter

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

    # East-face slots matching AC west: dimmer (north module) and SSR (south).
    # Y from y_max so the north faces stay aligned; Z from AC (open to the top).
    dimmer, ssr = ouvertures_modules(y_n, wall)
    dimmer_y0, dimmer_y1, dimmer_z = dimmer
    ssr_y0, ssr_y1, ssr_z = ssr
    body = body - Pos(x_e - wall - margin, dimmer_y0, dimmer_z) * Box(
        wall + 2 * margin,
        dimmer_y1 - dimmer_y0,
        hauteur + margin - dimmer_z,
        align=_amin,
    )
    body = body - Pos(x_e - wall - margin, ssr_y0, ssr_z) * Box(
        wall + 2 * margin,
        ssr_y1 - ssr_y0,
        hauteur + margin - ssr_z,
        align=_amin,
    )

    body = _fuse_one(body)
    if draft:
        return body

    conc = {
        (round(e.center().X, 2), round(e.center().Y, 2), round(e.center().Z, 2))
        for e in concave_edges(body)
    }
    chanfrein_fente = 0.6

    def in_angle_ne(bb):
        """Inner NE corner only (x ≈ 48.4, y ≈ 93.4), not the outer (50, 95)."""
        mx = 0.5 * (bb.min.X + bb.max.X)
        my = 0.5 * (bb.min.Y + bb.max.Y)
        return abs(mx - x_inner_e) < 1.2 and abs(my - y_inner_n) < 1.2

    def in_muret_ouest_bas(bb):
        """South end of the west insert support wall: left it unchamfered."""
        mx = 0.5 * (bb.min.X + bb.max.X)
        my = 0.5 * (bb.min.Y + bb.max.Y)
        return abs(mx - insert2_cx) < 1.0 and abs(my - y_bot) < 1.0

    def in_fente_est(bb):
        on_est = (
            bb.max.X > x_e - wall - margin - 0.2
            and bb.min.X < x_e + margin + 0.2
        )
        if not on_est:
            return False
        if in_angle_ne(bb):
            return False
        my = 0.5 * (bb.min.Y + bb.max.Y)
        dimmer_mid = 0.5 * (dimmer_y0 + dimmer_y1)
        ssr_mid = 0.5 * (ssr_y0 + ssr_y1)
        dimmer_hit = (
            abs(my - dimmer_mid) <= 0.5 * (dimmer_y1 - dimmer_y0) + 0.8
            and bb.min.Z > dimmer_z - 0.5
        )
        ssr_hit = (
            abs(my - ssr_mid) <= 0.5 * (ssr_y1 - ssr_y0) + 0.8
            and bb.min.Z > ssr_z - 0.5
        )
        return dimmer_hit or ssr_hit

    def keep(edge):
        c = edge.center()
        if (round(c.X, 2), round(c.Y, 2), round(c.Z, 2)) in conc:
            return False
        bb = edge.bounding_box()
        if in_angle_ne(bb):
            return False
        if in_fente_est(bb):
            return False
        if in_muret_ouest_bas(bb):
            return False
        return True

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

    body = polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)
    return polish(body, body.edges().filter_by(fente_keep), chanfrein_fente)

from nurb import *

from system import MARGE_PUIT, _fuse_one, add_well, anti_tirage_ns, offset_in

_AMIN = (Align.MIN, Align.MIN, Align.MIN)


def _bb(x0, y0, z0, x1, y1, z1):
    return Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, z1 - z0, align=_AMIN)


def _contour(vide_haut, degagement_vis):
    x_max = measured("boitier_int_x")
    y_max = measured("boitier_int_y")
    aile_x = measured("boitier_int_aile_x")
    aile_y = measured("boitier_int_aile_y")
    gout_x = measured("boitier_int_gouttiere_x")
    gout_haut = y_max - vide_haut
    encoche_est = gout_x + degagement_vis
    return [
        (aile_x, aile_y),
        (x_max, aile_y),
        (x_max, y_max),
        (encoche_est, y_max),
        (encoche_est, gout_haut),
        (gout_x, gout_haut),
        (gout_x, y_max),
        (aile_x, y_max),
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


def _rebord_sud(x0, x1, y_sud, z_top, run, rebord):
    """45° south extension then a vertical catch. y_sud is the wall's south face."""
    z0 = z_top - run
    z1 = z_top + rebord
    return Pos(x0, 0, 0) * extrude(
        Plane.YZ
        * Polygon(
            (y_sud, z0),
            (y_sud, z1),
            (y_sud - run, z1),
            (y_sud - run, z_top),
            align=None,
        ),
        x1 - x0,
    )


def _butee_triangle(x0, x1, y_sud, y_nord, z_bot, z_top):
    """Right triangle on the north wall: 45° underside, flat top. Empty below."""
    return Pos(x0, 0, 0) * extrude(
        Plane.YZ
        * Polygon(
            (y_nord, z_bot),
            (y_nord, z_top),
            (y_sud, z_top),
            align=None,
        ),
        x1 - x0,
    )


@part
def boitier_ac(
    hauteur=30.0,
    epaisseur_paroi=1.6,
    gouttiere_vide_haut=10.0,
    degagement_vis=10.0,
    hauteur_vis=16.6,
    appui_y=18.0,
    muret_depuis_ouest=20.0,
    puit_depuis_cote=10.0,
    puit_depuis_nord=10.0,
    puit_diametre=8.2,
    puit_peau=0.6,
    marge_puit=MARGE_PUIT,
    fente_cables=5.0,
    fente_z=10.0,
    anneau_z=18.0,
    anneau_depuis_est=10.0,
    rebord=2.0,
    butee_depuis_ouest=7.0,
    draft=False,
):
    """Boîtier AC : partie est, murs 30 mm, plateforme vis et barre d'appui.

    hauteur: hauteur hors-tout depuis le lit (murs compris, linteau nord inclus)
    epaisseur_paroi: épaisseur du fond et des murs, vers l'intérieur
    gouttiere_vide_haut: profondeur en Y du logement vis (mur sud du logement à 10 mm)
    degagement_vis: écart en X entre les murs est et ouest autour de la vis
    hauteur_vis: haut des murs vis / muret / dalle (16,6 = 15 mm de vide + 1,6 mm de dalle)
    appui_y: longueur en Y des murs est et ouest (tête + jambes)
    muret_depuis_ouest: distance de la face ouest extérieure à la barre de soutien
    puit_depuis_cote: distance du centre de chaque puits à sa face latérale (ouest / est)
    puit_depuis_nord: distance du centre des deux puits à la face nord extérieure
    puit_diametre: diamètre intérieur du puits d'aimant Ø8×3
    puit_peau: plastique sous l'aimant
    marge_puit: plastique autour du puits (doctrine 1,6 mm)
    fente_cables: largeur en Y des deux fentes sur la face est
    fente_z: hauteur des fentes, les derniers millimètres sous le sommet
    anneau_z: bas de l'anneau anti-tirage sud (z = 18, dans la pente vers la plateforme)
    anneau_depuis_est: distance du bord est intérieur à l'anneau, vers l'ouest
    rebord: cran vertical au sud des jambes et du muret, 45° dessous
    butee_depuis_ouest: face est de la butée-triangle, depuis la face ouest
    """
    wall = epaisseur_paroi
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")
    x_max = measured("boitier_int_x")
    y_max = measured("boitier_int_y")
    aile_x = measured("boitier_int_aile_x")
    aile_y = measured("boitier_int_aile_y")
    gout_x = measured("boitier_int_gouttiere_x")
    vis_x = measured("boitier_ac_vis_x")
    vis_y = measured("boitier_ac_vis_y")
    vis_z = measured("boitier_ac_vis_z")

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
    if gouttiere_vide_haut < 0.5:
        reject(
            f"gouttiere_vide_haut {gouttiere_vide_haut} collapses the screw "
            "notch into y_max: raise it",
            param="gouttiere_vide_haut",
        )
    gout_haut = y_max - gouttiere_vide_haut
    if gout_haut <= aile_y + 2.0 * wall:
        reject(
            f"gouttiere_vide_haut {gouttiere_vide_haut} leaves no east bay "
            f"above y={aile_y}: lower it",
            param="gouttiere_vide_haut",
        )
    if degagement_vis < vis_x + 0.4:
        reject(
            f"degagement_vis {degagement_vis} is too tight for a {vis_x} mm "
            "screw (may be 5.1): raise it above 5.4",
            param="degagement_vis",
        )
    encoche_est = gout_x + degagement_vis
    if encoche_est >= x_max - wall:
        reject(
            f"degagement_vis {degagement_vis} eats the north-east return "
            f"(east edge at x={encoche_est:.1f}): lower it",
            param="degagement_vis",
        )
    if aile_x >= gout_x - wall:
        reject(
            f"west face at x={aile_x} collides with the screw notch at "
            f"x={gout_x}: the split must stay west of the notch",
        )
    if hauteur_vis < vis_z + wall:
        reject(
            f"hauteur_vis {hauteur_vis} leaves under {vis_z} mm of screw "
            f"pocket (slab is {wall} mm): raise it above {vis_z + wall:.1f}",
            param="hauteur_vis",
        )
    lintel_z = hauteur_vis
    if hauteur < lintel_z:
        reject(
            f"hauteur {hauteur} is under the north lintel "
            f"({lintel_z:.1f} mm): raise it",
            param="hauteur",
        )
    if appui_y <= gouttiere_vide_haut + wall:
        reject(
            f"appui_y {appui_y} is not longer than the screw head "
            f"({gouttiere_vide_haut + wall:.1f} mm): raise it",
            param="appui_y",
        )
    plat_y0 = y_max - appui_y
    if plat_y0 < aile_y + wall:
        reject(
            f"appui_y {appui_y} hits the south wall: lower it",
            param="appui_y",
        )
    if rebord < 0.8:
        reject(
            f"rebord {rebord} is under 0.8 mm: raise it",
            param="rebord",
        )
    if hauteur_vis + rebord > hauteur:
        reject(
            f"rebord {rebord} plus hauteur_vis {hauteur_vis} exceeds "
            f"hauteur {hauteur}: lower it",
            param="rebord",
        )
    if plat_y0 - wall < aile_y:
        reject(
            f"rebord would hit the south wall: raise appui_y or lower "
            "epaisseur_paroi",
            param="appui_y",
        )
    if muret_depuis_ouest < wall:
        reject(
            f"muret_depuis_ouest {muret_depuis_ouest} is under one wall: raise it",
            param="muret_depuis_ouest",
        )
    muret_x = aile_x + muret_depuis_ouest
    if muret_x + wall >= gout_x - wall:
        reject(
            f"muret_depuis_ouest {muret_depuis_ouest} runs into the screw "
            "platform: lower it",
            param="muret_depuis_ouest",
        )
    if butee_depuis_ouest < wall:
        reject(
            f"butee_depuis_ouest {butee_depuis_ouest} is under one wall: raise it",
            param="butee_depuis_ouest",
        )
    butee_x1 = aile_x + butee_depuis_ouest
    butee_x0 = butee_x1 - wall
    if butee_x0 < aile_x + wall:
        reject(
            f"butee_depuis_ouest {butee_depuis_ouest} sits in the west wall: "
            "raise it",
            param="butee_depuis_ouest",
        )
    if butee_x1 + 2.0 > muret_x:
        reject(
            f"butee_depuis_ouest {butee_depuis_ouest} runs into the support bar: "
            "lower it",
            param="butee_depuis_ouest",
        )
    butee_z1 = hauteur_vis + rebord
    butee_z0 = butee_z1 - appui_y
    if butee_z0 < 0.0:
        reject(
            f"appui_y {appui_y} is taller than the stop top "
            f"({butee_z1:.1f} mm): lower it",
            param="appui_y",
        )
    if butee_z1 > hauteur:
        reject(
            f"butee top {butee_z1:.1f} exceeds hauteur {hauteur}: lower rebord",
            param="rebord",
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
    pad_r = puit_diametre / 2.0 + marge_puit
    puit_cx = aile_x + puit_depuis_cote
    puit_est_cx = x_max - puit_depuis_cote
    puit_cy = y_max - puit_depuis_nord
    if puit_depuis_cote < wall + pad_r:
        reject(
            f"puit_depuis_cote {puit_depuis_cote} puts a well pad into "
            f"a side wall: raise it above {wall + pad_r:.1f}",
            param="puit_depuis_cote",
        )
    if puit_depuis_nord < wall + pad_r:
        reject(
            f"puit_depuis_nord {puit_depuis_nord} puts the well pad into "
            f"the north wall: raise it above {wall + pad_r:.1f}",
            param="puit_depuis_nord",
        )
    if puit_cx + pad_r > muret_x:
        reject(
            f"puit_depuis_cote {puit_depuis_cote} runs the west well into the "
            "support bar: lower it",
            param="puit_depuis_cote",
        )
    plat_x1 = encoche_est + wall
    if puit_est_cx - pad_r < plat_x1:
        reject(
            f"puit_depuis_cote {puit_depuis_cote} runs the east well into the "
            "screw platform: lower it",
            param="puit_depuis_cote",
        )
    if puit_est_cx - puit_cx < puit_diametre + 2.0 * marge_puit:
        reject(
            f"puit_depuis_cote {puit_depuis_cote} makes the two wells overlap: "
            "lower it",
            param="puit_depuis_cote",
        )
    z_hyp_puit = butee_z0 + (y_max - puit_cy)
    if well_stack + 0.4 > z_hyp_puit:
        reject(
            f"butee 45° underside at the well is {z_hyp_puit:.1f} mm, "
            f"under the magnet stack {well_stack:.1f} mm: raise the stop or "
            "move the well",
            param="butee_depuis_ouest",
        )
    if fente_cables < 2.0:
        reject(
            f"fente_cables {fente_cables} is under 2 mm: raise it",
            param="fente_cables",
        )
    if fente_z < 2.0:
        reject(
            f"fente_z {fente_z} is under 2 mm: raise it",
            param="fente_z",
        )
    z_fente = hauteur - fente_z
    if z_fente < wall + 2.0:
        reject(
            f"fente_z {fente_z} leaves the slots too close to the floor: "
            "lower it",
            param="fente_z",
        )
    y_se0 = aile_y + wall
    y_se1 = y_se0 + fente_cables
    y_ne1 = y_max - wall
    y_ne0 = y_ne1 - fente_cables
    if y_se1 + 2.0 > y_ne0:
        reject(
            f"fente_cables {fente_cables} makes the two east slots overlap: "
            "lower it",
            param="fente_cables",
        )
    at_jeu = 1.2
    at_largeur = 3.0
    at_bords = 5.6
    at_h = 5.0
    at_out = at_jeu + wall
    if anneau_z - at_out < wall + 0.4:
        reject(
            f"anneau_z {anneau_z} plus the 45° ramps hit the floor: raise it",
            param="anneau_z",
        )
    if anneau_z + at_h + at_out > hauteur:
        reject(
            f"anneau_z {anneau_z} plus the 45° ramps stick out above "
            f"hauteur {hauteur}: lower it",
            param="anneau_z",
        )
    if anneau_depuis_est < 0.0:
        reject(
            f"anneau_depuis_est {anneau_depuis_est} is negative: raise it",
            param="anneau_depuis_est",
        )
    x_inner = x_max - wall
    x_anneau1 = x_inner - anneau_depuis_est
    x_anneau0 = x_anneau1 - at_bords
    if x_anneau0 < encoche_est + wall + 2.0:
        reject(
            f"anneau_depuis_est {anneau_depuis_est} runs the ring into the "
            "screw platform: raise it",
            param="anneau_depuis_est",
        )

    outer_pts = _contour(gouttiere_vide_haut, degagement_vis)
    inner_pts = offset_in(outer_pts, wall)

    outer = extrude(Polygon(*outer_pts, align=None), hauteur)
    cavity = Pos(0, 0, wall) * extrude(
        Polygon(*inner_pts, align=None), hauteur + 0.2
    )
    body = outer - cavity

    body = add_well(
        body, outer, puit_cx, puit_cy, puit_diametre, puit_peau, aimant_h, marge_puit
    )
    body = add_well(
        body,
        outer,
        puit_est_cx,
        puit_cy,
        puit_diametre,
        puit_peau,
        aimant_h,
        marge_puit,
    )

    # Drop the notch walls from `hauteur` down to `hauteur_vis`.
    plat_x0 = gout_x - wall
    plat_x1 = encoche_est + wall
    body = body - _bb(
        plat_x0,
        gout_haut - wall,
        hauteur_vis,
        plat_x1,
        y_max + 1.0,
        hauteur + 1.0,
    )

    # Slab between the E/W walls, top flush with the wall tops.
    # Underside at 15 mm so the screw pocket is 15 mm effective.
    # Solid 45° gussets under it, from the three housing walls up to the underside.
    slab_z0 = hauteur_vis - wall
    body = body + _bb(gout_x, gout_haut, slab_z0, encoche_est, y_max, hauteur_vis)
    body = body + _gousset_x(gout_x, gout_haut, y_max, slab_z0, wall, True)
    body = body + _gousset_x(encoche_est, gout_haut, y_max, slab_z0, wall, False)
    body = body + _gousset_y(gout_haut, gout_x, encoche_est, slab_z0, wall, True)

    # Legs: E/W walls continue south to 18 mm total Y. No closing south wall.
    body = body + _bb(plat_x0, plat_y0, 0.0, plat_x0 + wall, gout_haut, hauteur_vis)
    body = body + _bb(
        plat_x1 - wall, plat_y0, 0.0, plat_x1, gout_haut, hauteur_vis
    )

    # Same bar as the vis legs: 16.6 mm tall, 18 mm in Y, 20 mm from west face.
    body = body + _bb(
        muret_x, plat_y0, 0.0, muret_x + wall, y_max, hauteur_vis
    )

    # 2 mm catch at the south of each rest: 45° out, then vertical.
    body = body + _rebord_sud(
        plat_x0, plat_x0 + wall, plat_y0, hauteur_vis, wall, rebord
    )
    body = body + _rebord_sud(
        plat_x1 - wall, plat_x1, plat_y0, hauteur_vis, wall, rebord
    )
    body = body + _rebord_sud(
        muret_x, muret_x + wall, plat_y0, hauteur_vis, wall, rebord
    )

    # West stop: east face at 7 mm, 18×18 45° triangle on the north wall.
    # No south hook. Underside leaves the magnet well clear.
    body = body + _butee_triangle(
        butee_x0, butee_x1, plat_y0, y_max, butee_z0, butee_z1
    )

    # North face is open only at the housing interior. From the slab top
    # the north wall runs the full X width (lintel sitting on the slab).
    body = body + _bb(
        plat_x0, y_max - wall, lintel_z, plat_x1, y_max, hauteur
    )

    # Two 5 mm cable slots on the east face, last 10 mm in Z, at the
    # inner north and south corners (N/S walls stay continuous to x_max).
    margin = 0.5
    body = body - _bb(
        x_max - wall - margin,
        y_se0,
        z_fente,
        x_max + margin,
        y_se1,
        hauteur + margin,
    )
    body = body - _bb(
        x_max - wall - margin,
        y_ne0,
        z_fente,
        x_max + margin,
        y_ne1,
        hauteur + margin,
    )

    # Cable-tie U on the inner south only, 10 mm west of the east inner
    # face, at z = 18. North ring conflicts with the module.
    at_kw = dict(
        wall=wall,
        jeu=at_jeu,
        largeur=at_largeur,
        bords=at_bords,
        hauteur_u=at_h,
    )
    body = body + anti_tirage_ns(
        x_anneau0, y_se0, anneau_z, True, **at_kw
    )

    body = _fuse_one(body)
    if draft:
        return body

    conc = {
        (round(e.center().X, 2), round(e.center().Y, 2), round(e.center().Z, 2))
        for e in concave_edges(body)
    }
    vis_x0 = plat_x0 - 1.0
    vis_x1 = plat_x1 + 1.0
    vis_y0 = plat_y0 - wall
    muret_x0 = muret_x - 0.5
    muret_x1 = muret_x + wall + 0.5
    butee_skip_x0 = butee_x0 - 0.5
    butee_skip_x1 = butee_x1 + 0.5
    chanfrein_fente = 0.6
    at_u1 = anneau_z + at_h
    at_mid = x_anneau0 + 0.5 * at_bords

    def in_fente(bb):
        on_est = bb.max.X > x_max - wall - 0.5
        se = bb.min.Y > y_se0 - 0.5 and bb.max.Y < y_se1 + 0.5
        ne = bb.min.Y > y_ne0 - 0.5 and bb.max.Y < y_ne1 + 0.5
        return on_est and (se or ne) and bb.min.Z > z_fente - 0.5

    def keep(edge):
        c = edge.center()
        if (round(c.X, 2), round(c.Y, 2), round(c.Z, 2)) in conc:
            return False
        # No polish on the opening, platform or support bar: the north-wall
        # Z edges of the cutout were a 1 mm chamfer the user does not want.
        if vis_x0 <= c.X <= vis_x1 and c.Y >= vis_y0:
            return False
        if muret_x0 <= c.X <= muret_x1 and c.Y >= vis_y0:
            return False
        if butee_skip_x0 <= c.X <= butee_skip_x1 and c.Y >= vis_y0:
            return False
        if in_fente(edge.bounding_box()):
            return False
        return True

    def fente_keep(edge):
        bb = edge.bounding_box()
        dx = bb.max.X - bb.min.X
        dy = bb.max.Y - bb.min.Y
        dz = bb.max.Z - bb.min.Z
        if (dx * dx + dy * dy + dz * dz) ** 0.5 < 2.0:
            return False
        if dz < 8.0:
            return False
        return in_fente(bb)

    def pont_passage(edge):
        bb = edge.bounding_box()
        dx = bb.max.X - bb.min.X
        dy = bb.max.Y - bb.min.Y
        dz = bb.max.Z - bb.min.Z
        span = (dx * dx + dy * dy + dz * dz) ** 0.5
        if abs(span - at_largeur) > 0.4:
            return False
        if dz > 0.4:
            return False
        zmid = 0.5 * (bb.min.Z + bb.max.Z)
        if abs(zmid - anneau_z) > 0.3 and abs(zmid - at_u1) > 0.3:
            return False
        mx = 0.5 * (bb.min.X + bb.max.X)
        my = 0.5 * (bb.min.Y + bb.max.Y)
        if abs(mx - at_mid) > 2.0:
            return False
        sud = abs(my - (y_se0 + at_jeu)) < 0.3
        return sud

    body = polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)
    body = polish(body, body.edges().filter_by(fente_keep), chanfrein_fente)
    return polish(body, body.edges().filter_by(pont_passage), 0.4)

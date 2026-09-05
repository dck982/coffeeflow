from nurb import *

from system import INSERT_M3, MARGE_PUIT, _fuse_one, add_well, offset_in, ouvertures_modules

_AMIN = (Align.MIN, Align.MIN, Align.MIN)


def _bb(x0, y0, z0, x1, y1, z1):
    return Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, z1 - z0, align=_AMIN)


def _contour(vide_haut, degagement_vis, west_x=None, east_x=None, south_y=None):
    x_max = measured("boitier_int_x")
    y_max = measured("boitier_int_y")
    aile_x = measured("boitier_int_aile_x")
    aile_y = measured("boitier_int_aile_y")
    gout_x = measured("boitier_int_gouttiere_x")
    gout_haut = y_max - vide_haut
    encoche_est = gout_x + degagement_vis
    if west_x is None:
        west_x = aile_x
    if east_x is None:
        east_x = x_max
    if south_y is None:
        south_y = aile_y
    return [
        (west_x, south_y),
        (east_x, south_y),
        (east_x, y_max),
        (encoche_est, y_max),
        (encoche_est, gout_haut),
        (gout_x, gout_haut),
        (gout_x, y_max),
        (west_x, y_max),
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


def _tour_sud(x0, y_sud, wall, tour_y, z_top):
    """Tower at the south end of a wall: wall in X, tour_y in Y, bed to z_top."""
    return _bb(x0, y_sud - tour_y, 0.0, x0 + wall, y_sud, z_top)


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
    hauteur=27.0,
    epaisseur_paroi=1.6,
    gouttiere_vide_haut=10.0,
    degagement_vis=10.0,
    hauteur_vis=16.6,
    appui_y=18.0,
    muret_depuis_ouest=23.0,
    puit_depuis_cote=10.0,
    puit_depuis_nord=10.0,
    puit_diametre=8.2,
    puit_peau=0.6,
    marge_puit=MARGE_PUIT,
    fente_nord_est=15.0,
    fente_nord_est_z=10.0,
    fente_sud_est=10.0,
    fente_sud_est_z=5.6,
    fente_sud_ouest_z=7.0,
    fente_nord_ouest_z=13.6,
    rebord=4.0,
    tour_y=3.0,
    butee_depuis_ouest=8.0,
    butee_y=10.0,
    traverse_depuis_crochet_sud=8.0,
    traverse_decalage_x=3.0,
    insert_sud_decalage_x=0.0,
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
    fente_nord_est: largeur en Y de la fente nord sur la face est
    fente_nord_est_z: bas de la fente nord-est, ouverte du sommet jusqu'à ce Z
    fente_sud_est: largeur en Y de la fente sud sur la face est
    fente_sud_est_z: bas de la fente sud-est (4 mm au-dessus du fond)
    fente_sud_ouest_z: bas de la fente sud-ouest (écart tours–traverse), ouverte du sommet jusqu'à ce Z
    fente_nord_ouest_z: bas de la fente nord-ouest (dimmer), ouverte du sommet jusqu'à ce Z
    rebord: dépassement des tours sud au-dessus du muret / plateforme
    tour_y: longueur des tours en Y, collées au sud des murets (X reste l'épaisseur de paroi)
    butee_depuis_ouest: face est de la butée-triangle, depuis la face ouest
    butee_y: hauteur/longueur en Y de la butée à 45°, indépendante de appui_y
    traverse_depuis_crochet_sud: face nord de la traverse, depuis la face sud des tours
    traverse_decalage_x: décalage de toute la traverse vers l'ouest (X diminue) ; le côté
        est de la traverse est en plus décalé de 10 mm vers l'est, ce qui l'élargit
    insert_sud_decalage_x: décalage du heat insert M3 (face sud) depuis le centre de
        cette face ; 0 le centre
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
    if tour_y < 0.8:
        reject(
            f"tour_y {tour_y} is under 0.8 mm: raise it",
            param="tour_y",
        )
    if plat_y0 - tour_y < aile_y:
        reject(
            f"tour_y {tour_y} hits the south wall: lower it or raise appui_y",
            param="tour_y",
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
    if butee_y < 2.0:
        reject(
            f"butee_y {butee_y} is under 2 mm: raise it",
            param="butee_y",
        )
    butee_y_sud = y_max - butee_y
    if butee_y_sud < aile_y:
        reject(
            f"butee_y {butee_y} runs the stop south of y={aile_y}: lower it",
            param="butee_y",
        )
    butee_z1 = hauteur_vis
    butee_z0 = butee_z1 - butee_y
    if butee_z0 < 0.0:
        reject(
            f"butee_y {butee_y} is taller than the stop top "
            f"({butee_z1:.1f} mm): lower it",
            param="butee_y",
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
    tour_y_sud = plat_y0 - tour_y
    trav_y_nord = tour_y_sud - traverse_depuis_crochet_sud
    trav_y_sud = trav_y_nord - wall
    trav_x0 = muret_x - traverse_decalage_x
    trav_x1 = plat_x1 - traverse_decalage_x
    if traverse_depuis_crochet_sud < 0.0:
        reject(
            f"traverse_depuis_crochet_sud {traverse_depuis_crochet_sud} is negative: "
            "raise it",
            param="traverse_depuis_crochet_sud",
        )
    if trav_y_sud < aile_y:
        reject(
            f"traverse_depuis_crochet_sud {traverse_depuis_crochet_sud} puts the "
            f"traverse south of y={aile_y}: lower it",
            param="traverse_depuis_crochet_sud",
        )
    if traverse_decalage_x < 0.0:
        reject(
            f"traverse_decalage_x {traverse_decalage_x} is negative: raise it",
            param="traverse_decalage_x",
        )
    if trav_x0 < aile_x + wall:
        reject(
            f"traverse_decalage_x {traverse_decalage_x} runs the traverse into "
            f"the west wall: lower it",
            param="traverse_decalage_x",
        )
    if fente_nord_est < 2.0:
        reject(
            f"fente_nord_est {fente_nord_est} is under 2 mm: raise it",
            param="fente_nord_est",
        )
    if fente_sud_est < 2.0:
        reject(
            f"fente_sud_est {fente_sud_est} is under 2 mm: raise it",
            param="fente_sud_est",
        )
    if fente_nord_est_z < wall:
        reject(
            f"fente_nord_est_z {fente_nord_est_z} cuts the floor: raise it "
            f"above {wall:.1f}",
            param="fente_nord_est_z",
        )
    if fente_nord_est_z >= hauteur:
        reject(
            f"fente_nord_est_z {fente_nord_est_z} is not below hauteur "
            f"{hauteur}: lower it",
            param="fente_nord_est_z",
        )
    if fente_sud_est_z < wall:
        reject(
            f"fente_sud_est_z {fente_sud_est_z} cuts the floor: raise it "
            f"above {wall:.1f}",
            param="fente_sud_est_z",
        )
    if fente_sud_est_z >= hauteur:
        reject(
            f"fente_sud_est_z {fente_sud_est_z} is not below hauteur "
            f"{hauteur}: lower it",
            param="fente_sud_est_z",
        )
    if fente_sud_ouest_z < wall:
        reject(
            f"fente_sud_ouest_z {fente_sud_ouest_z} cuts the floor: raise it above "
            f"{wall:.1f}",
            param="fente_sud_ouest_z",
        )
    if fente_sud_ouest_z >= hauteur:
        reject(
            f"fente_sud_ouest_z {fente_sud_ouest_z} is not below hauteur {hauteur}: "
            "lower it",
            param="fente_sud_ouest_z",
        )
    if fente_nord_ouest_z < wall:
        reject(
            f"fente_nord_ouest_z {fente_nord_ouest_z} cuts the floor: raise it above "
            f"{wall:.1f}",
            param="fente_nord_ouest_z",
        )
    if fente_nord_ouest_z >= hauteur:
        reject(
            f"fente_nord_ouest_z {fente_nord_ouest_z} is not below hauteur {hauteur}: "
            "lower it",
            param="fente_nord_ouest_z",
        )
    y_se0 = aile_y + wall
    y_se1 = y_se0 + fente_sud_est
    y_ne1 = y_max - wall
    y_ne0 = y_ne1 - fente_nord_est
    if y_se1 + 2.0 > y_ne0:
        reject(
            f"fente_nord_est {fente_nord_est} and fente_sud_est {fente_sud_est} "
            "make the two east slots overlap: lower one",
            param="fente_nord_est",
        )

    # Outer envelope edits (west/east/south) must not move the interior.
    # We therefore build the cavity from the original contour, while the
    # outer solid uses shifted outer-wall coordinates.
    ac_west_shift = 5.0
    ac_east_shift = 1.5
    ac_south_shift = 4.0
    x_east_fente = x_max - 1.0
    outer_pts_outer = _contour(
        gouttiere_vide_haut,
        degagement_vis,
        west_x=aile_x - ac_west_shift,
        east_x=x_max - ac_east_shift,
        south_y=aile_y - ac_south_shift,
    )
    # Build cavity from the shifted outer envelope as well: it keeps the
    # wall thickness consistent and avoids degenerate ultra-thin east walls
    # that can crash the polish/border analysis in nurb.
    inner_pts = offset_in(outer_pts_outer, wall)

    outer = extrude(Polygon(*outer_pts_outer, align=None), hauteur)
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

    x_outer_west = aile_x - ac_west_shift

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

    # Mini-tower at the south of each rest: wall in X, tour_y in Y, bed to 2 mm above.
    tour_z = hauteur_vis + rebord
    body = body + _tour_sud(plat_x0, plat_y0, wall, tour_y, tour_z)
    body = body + _tour_sud(plat_x1 - wall, plat_y0, wall, tour_y, tour_z)
    body = body + _tour_sud(muret_x, plat_y0, wall, tour_y, tour_z)

    # M3 corbel heat insert, centred on the south wall: same overhang recipe
    # as boitier_ps/boitier_dc, full thickness only over the top
    # insert_profondeur so it costs little material below the rim.
    east_x_outer = x_max - ac_east_shift
    ins_diametre = INSERT_M3.diametre_percage
    ins_profondeur = INSERT_M3.profondeur_min
    ins_r = ins_diametre / 2.0
    ins_plat = ins_diametre + wall
    ins_along = ins_diametre + 2.0 * wall
    ins_half = ins_along / 2.0
    z_ins = hauteur - ins_profondeur
    z_ins_45 = z_ins - ins_plat
    ov = 0.4
    south_pts = [
        (-ov, z_ins_45),
        (0.0, z_ins_45),
        (ins_plat, z_ins),
        (ins_plat, hauteur),
        (-ov, hauteur),
    ]
    y_south_face = aile_y - ac_south_shift + wall
    insert_sud_x = 0.5 * (x_outer_west + east_x_outer) + insert_sud_decalage_x
    if insert_sud_x - ins_half < x_outer_west + wall:
        reject(
            f"insert_sud_decalage_x {insert_sud_decalage_x} runs the south "
            "insert into the west wall: raise it",
            param="insert_sud_decalage_x",
        )
    if insert_sud_x + ins_half > east_x_outer - wall:
        reject(
            f"insert_sud_decalage_x {insert_sud_decalage_x} runs the south "
            "insert into the east wall: lower it",
            param="insert_sud_decalage_x",
        )
    body = body + (
        Pos(insert_sud_x - ins_half, y_south_face, 0)
        * extrude(Plane.YZ * Polygon(*south_pts, align=None), ins_along)
    )
    body = body - (
        Pos(insert_sud_x, y_south_face + ins_r, z_ins)
        * Cylinder(ins_r, ins_profondeur + 0.1, align=(Align.CENTER, Align.CENTER, Align.MIN))
    )

    # Traverse: same length, shifted west by traverse_decalage_x. Same Z as
    # the murets (hauteur_vis), not the south towers.
    body = body + _bb(
        trav_x0, trav_y_sud, 0.0, trav_x1, trav_y_nord, hauteur_vis
    )

    # Stop wall 10 mm east of the traverse's east edge (module mounted
    # upside down, connector clears the west stop and needs a stop of its
    # own past the traverse): 3 mm tall from the floor (z=1,6 to z=4,6),
    # 1,6 mm thick to the east, 10 mm in Y, south side flush with the
    # traverse's south side.
    butee_arret_x0 = trav_x1 + 11.0
    body = body + _bb(
        butee_arret_x0,
        trav_y_sud,
        wall,
        butee_arret_x0 + wall,
        trav_y_sud + 10.0,
        wall + 3.0,
    )

    # West stop: east face at 7 mm, 18×18 45° triangle on the north wall.
    # No south hook. Underside leaves the magnet well clear.
    body = body + _butee_triangle(
        butee_x0, butee_x1, butee_y_sud, y_max, butee_z0, butee_z1
    )

    # North face is open only at the housing interior. From the slab top
    # the north wall runs the full X width (lintel sitting on the slab).
    body = body + _bb(
        plat_x0, y_max - wall, lintel_z, plat_x1, y_max, hauteur
    )

    # Cable slots on the east face: north last 10 mm in Z, south down to
    # 4 mm above the floor. N/S walls stay so those faces run to the corner.
    margin = 0.5

    # Top opening on the west face: dimmer (north module), same recipe as DC east.
    # From the triangle top upward; north edge one wall inside y_max.
    dimmer, ssr = ouvertures_modules(
        y_max,
        wall,
        appui_y,
        tour_y,
        traverse_depuis_crochet_sud,
        fente_nord_ouest_z,
        fente_sud_ouest_z,
    )
    dimmer_y0, dimmer_y1, dimmer_z = dimmer
    ssr_y0, ssr_y1, ssr_z = ssr
    body = body - _bb(
        x_outer_west - margin,
        dimmer_y0,
        dimmer_z,
        aile_x + margin,
        dimmer_y1,
        hauteur + margin,
    )

    # West slot facing the SSR gap (tower south … traverse north).
    body = body - _bb(
        x_outer_west - margin,
        ssr_y0,
        ssr_z,
        aile_x + margin,
        ssr_y1,
        hauteur + margin,
    )

    body = body - _bb(
        x_east_fente - wall - margin,
        y_se0,
        fente_sud_est_z,
        x_east_fente + margin,
        y_se1,
        hauteur + margin,
    )
    body = body - _bb(
        x_east_fente - wall - margin,
        y_ne0,
        fente_nord_est_z,
        x_east_fente + margin,
        y_ne1,
        hauteur + margin,
    )

    body = _fuse_one(body)
    if draft:
        return body

    # Some wall-step edits can create ultra-thin or near-degenerate edges.
    # nurb's concave-edge classifier may recurse in those cases; we still
    # want the part to build, so fall back to "no concave-edge filter".
    try:
        conc = {
            (
                round(e.center().X, 2),
                round(e.center().Y, 2),
                round(e.center().Z, 2),
            )
            for e in concave_edges(body)
        }
    except RecursionError:
        conc = set()
    vis_x0 = plat_x0 - 1.0
    vis_x1 = plat_x1 + 1.0
    vis_y0 = plat_y0 - tour_y
    muret_x0 = muret_x - 0.5
    muret_x1 = muret_x + wall + 0.5
    butee_skip_x0 = butee_x0 - 0.5
    butee_skip_x1 = butee_x1 + 0.5
    chanfrein_fente = 0.6

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

    def in_angle_nw_outer(bb):
        """Outer NW corner (x = x_outer_west, y = y_max): left square so it
        reads as one continuous face with boitier_dc's outer NE corner
        across the seam in `ensemble_boitiers`."""
        mx = 0.5 * (bb.min.X + bb.max.X)
        my = 0.5 * (bb.min.Y + bb.max.Y)
        return abs(mx - x_outer_west) < 1.2 and abs(my - y_max) < 1.2

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
        if in_fente(edge.bounding_box()) or in_fente_ouest(edge.bounding_box()):
            return False
        if in_angle_nw_outer(edge.bounding_box()):
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
        return in_fente(bb) or in_fente_ouest(bb)

    body = polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)
    body = polish(body, body.edges().filter_by(fente_keep), chanfrein_fente)
    return body

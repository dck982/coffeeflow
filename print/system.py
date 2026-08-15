"""Shared layout for the Wago derivation box and its lid."""

from types import SimpleNamespace

from nurb import Axis, measured, reject


def dims(longueur, largeur, hauteur, epaisseur_paroi, jeu_couvercle=0.3):
    """World-frame coordinates for the box. Origin at the outer min corner.

    Inner cavity is `longueur` x `largeur`. The notched long wall is at y = outer_y
    (opposite the Wago bays). The three exit notches are on x = 0.
    """
    wall = epaisseur_paroi
    floor = epaisseur_paroi
    inner_x = longueur
    inner_y = largeur
    outer_x = inner_x + 2.0 * wall
    outer_y = inner_y + 2.0 * wall

    wago_x = measured("wago_largeur")
    wago_y = measured("wago_profondeur")
    wago_z = measured("wago_hauteur")
    wago_pas = measured("wago_pas")
    muret_ep = measured("muret_epaisseur")
    muret_l = measured("muret_longueur")
    muret_h = measured("muret_hauteur")
    rail_ep = measured("rail_epaisseur")
    rail_h = measured("rail_hauteur")
    picot_d = measured("picot_diametre")
    picot_h = measured("picot_hauteur")
    encoche = measured("encoche")
    puit_d = measured("puit_diametre")
    puit_fond = measured("puit_fond")
    aimant_h = measured("aimant_hauteur")
    vis_trou = measured("vis_autoforeuse")
    vis_pass = measured("vis_passage")
    pilier_d = measured("pilier_diametre")
    lid_th = measured("couvercle_epaisseur")
    jupe_h = measured("jupe_hauteur")
    jupe_th = measured("jupe_epaisseur")
    jeu_wago = measured("jeu_wago")

    n = 3
    leftover = inner_x - n * 20.0 - (n - 1) * muret_ep
    if leftover < -0.05:
        reject(
            f"longueur {longueur} cannot fit three 20 mm Wago zones and two "
            f"{muret_ep} mm murets: raise longueur above {n * 20.0 + (n - 1) * muret_ep}",
            param="longueur",
        )
    slack = max(leftover, 0.0) / 2.0
    zone = 20.0 if leftover >= -0.05 else (inner_x - (n - 1) * muret_ep) / n

    if zone < wago_x + jeu_wago:
        reject(
            f"each Wago zone is {zone:.2f} mm, under the {wago_x} mm connector "
            f"plus {jeu_wago} mm clearance: raise longueur",
            param="longueur",
        )
    if muret_l + 0.5 > inner_y:
        reject(
            f"muret length {muret_l} leaves no cable channel in largeur {largeur}: "
            f"raise largeur above {muret_l + 8.0}",
            param="largeur",
        )

    # Divider X (inner), then world.
    div_inner = [slack + zone + i * (zone + muret_ep) for i in range(n - 1)]
    bays_inner = [slack + zone / 2.0 + i * (zone + muret_ep) for i in range(n)]
    bays_x = [wall + x for x in bays_inner]
    div_x = [wall + x for x in div_inner]

    # Rail sits at the back of the 20 mm bays so the channel stays 10 mm.
    rail_y0 = wall + muret_l - rail_ep
    rail_y1 = wall + muret_l
    channel_y0 = rail_y1
    channel_y1 = wall + inner_y
    channel = channel_y1 - channel_y0
    bay_cy = wall + muret_l / 2.0

    if channel < encoche * 3 + 0.8 * 2:
        reject(
            f"cable channel is {channel:.2f} mm, too narrow for three {encoche} mm "
            f"exit notches: raise largeur",
            param="largeur",
        )

    # Two entry notches per bay, on the outer terminals.
    entries = []
    for cx in bays_x:
        entries.append(cx - wago_pas)
        entries.append(cx + wago_pas)

    # Three exits, equally spaced in the channel strip of the -X wall.
    exits = [
        channel_y0 + channel * (i + 0.5) / 3.0
        for i in range(3)
    ]

    # Four standoffs per bay, around the centre, clear of the magnet well.
    picot_r = picot_d / 2.0
    puit_r = puit_d / 2.0
    spread = 5.5
    picots = []
    for cx in bays_x:
        for dx, dy in ((-spread, -spread), (spread, -spread), (-spread, spread), (spread, spread)):
            picots.append((cx + dx, bay_cy + dy))

    # Magnet wells in the two end bays. Boss rises so 3 mm of magnet sits on 0.6 mm floor.
    boss_h = puit_fond + aimant_h - floor
    if boss_h < 0.4:
        boss_h = 0.4
    puits = [(bays_x[0], bay_cy), (bays_x[-1], bay_cy)]

    # M3 bosses in the channel, on the two divider X, fused into the notched long
    # wall so the cylinder does not leave a tangent cusp on the inner face.
    pilier_r = pilier_d / 2.0
    pilier_y = channel_y1 - pilier_r + 1.2
    piliers = [(x + muret_ep / 2.0, pilier_y) for x in div_x]

    return SimpleNamespace(
        wall=wall,
        floor=floor,
        inner_x=inner_x,
        inner_y=inner_y,
        outer_x=outer_x,
        outer_y=outer_y,
        hauteur=hauteur,
        zone=zone,
        slack=slack,
        muret_ep=muret_ep,
        muret_l=muret_l,
        muret_h=muret_h,
        div_x=div_x,
        bays_x=bays_x,
        bay_cy=bay_cy,
        rail_ep=rail_ep,
        rail_h=rail_h,
        rail_y0=rail_y0,
        rail_y1=rail_y1,
        channel_y0=channel_y0,
        channel_y1=channel_y1,
        channel=channel,
        picot_r=picot_r,
        picot_h=picot_h,
        picots=picots,
        puit_r=puit_r,
        puit_fond=puit_fond,
        aimant_h=aimant_h,
        boss_h=boss_h,
        puits=puits,
        encoche=encoche,
        entries=entries,
        exits=exits,
        vis_trou=vis_trou,
        vis_pass=vis_pass,
        pilier_r=pilier_r,
        piliers=piliers,
        lid_th=lid_th,
        jupe_h=jupe_h,
        jupe_th=jupe_th,
        jeu=jeu_couvercle,
        wago_x=wago_x,
        wago_y=wago_y,
        wago_z=wago_z,
        n=n,
    )


def u_cutter(width, depth, length):
    """U-notch prism. Through +Y, rounded bottom at local z=0, opens +Z, centred on X."""
    from nurb import Align, Box, Cylinder, Pos, Rot

    r = width / 2.0
    rod = Rot(90, 0, 0) * Cylinder(r, length)
    rod = Pos(0, 0, r) * rod
    extra = max(depth - r, 0.0) + 2.0
    blade = Pos(0, 0, r) * Box(
        width, length, extra, align=(Align.CENTER, Align.CENTER, Align.MIN)
    )
    return rod + blade


def outer_corners(body, outer_x, outer_y, bed):
    """Vertical edges on the four outer corners, above the bed. Inner junctions stay sharp."""

    def keep(edge):
        bb = edge.bounding_box()
        if bb.min.Z < bed - 0.05:
            return False
        mx = 0.5 * (bb.min.X + bb.max.X)
        my = 0.5 * (bb.min.Y + bb.max.Y)
        on_x = mx < 0.4 or mx > outer_x - 0.4
        on_y = my < 0.4 or my > outer_y - 0.4
        return on_x and on_y

    return body.edges().filter_by(Axis.Z).filter_by(keep)

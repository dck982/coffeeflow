"""Shared layout for the Wago derivation box and its lid."""

from types import SimpleNamespace

from nurb import Axis, measured, reject


def dims(longueur, largeur, hauteur, epaisseur_paroi, jeu_couvercle=0.3):
    """Four standing Wago 221-423. Origin at the outer min corner.

    Back wall at y = 0: each Wago is pressed against it. Slotted wall at
    y = outer_y: four open-top slots, so a pre-wired connector drops in from +Z
    and its three wires slide into the slot facing it. Magnets sit in the
    empty run between the Wago fronts and that wall, not under the bays.
    """
    wall = epaisseur_paroi
    floor = epaisseur_paroi
    inner_x = longueur
    inner_y = largeur
    outer_x = inner_x + 2.0 * wall
    outer_y = inner_y + 2.0 * wall

    wago_x = measured("wago_hauteur")
    wago_y = measured("wago_profondeur")
    wago_z = measured("wago_largeur")
    jeu_slide = measured("jeu_glissement")
    muret_ep = measured("muret_epaisseur")
    canal_min = measured("canal_aimants")
    rail_ep = measured("rail_porteur_epaisseur")
    rail_h = measured("rail_porteur_hauteur")
    # Half of the standing Wago, counted above the rails it sits on
    # (user: 9 mm above the rails; 18.7/2 = 9.35 mm).
    muret_h = rail_h + wago_z / 2.0
    fente_w = measured("encoche")
    fente_h = measured("fente_hauteur")
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

    n = 4
    pilier_r = pilier_d / 2.0
    # Four identical bays: leftover length is split across all four, never
    # parked at the ends (that made the outer compartments look twice as wide).
    wago_block = n * (wago_x + jeu_slide) + (n - 1) * muret_ep
    leftover = inner_x - wago_block
    if leftover < -0.05:
        reject(
            f"longueur {longueur} cannot fit four identical "
            f"{wago_x + jeu_slide:.2f} mm bays and three {muret_ep} mm murets: "
            f"raise longueur above {wago_block}",
            param="longueur",
        )
    bay = (inner_x - (n - 1) * muret_ep) / n

    canal = inner_y - wago_y
    if canal < canal_min - 0.05:
        reject(
            f"largeur {largeur} leaves a {canal:.2f} mm magnet channel, "
            f"under the {canal_min} mm minimum: raise largeur above "
            f"{wago_y + canal_min}",
            param="largeur",
        )

    inner_h = hauteur - floor
    need_h = rail_h + wago_z + jeu_wago
    if inner_h < need_h - 0.05:
        reject(
            f"hauteur {hauteur} leaves {inner_h:.2f} mm inside, under the "
            f"{need_h:.2f} mm for {rail_h:.0f} mm rails plus a standing Wago: "
            f"raise hauteur",
            param="hauteur",
        )
    if fente_h > inner_h - 1.0:
        reject(
            f"fente_hauteur {fente_h} would reach the floor in hauteur "
            f"{hauteur}: raise hauteur",
            param="hauteur",
        )

    x0 = wall
    bays_x = [x0 + bay / 2.0 + i * (bay + muret_ep) for i in range(n)]
    div_x = [x0 + bay + i * (bay + muret_ep) for i in range(n - 1)]

    wago_y0 = wall
    wago_y1 = wall + wago_y
    bay_cy = wall + wago_y / 2.0

    rail_inset = 4.0
    rail_y = [wago_y0 + rail_inset, wago_y1 - rail_inset - rail_ep]

    channel_y0 = wago_y1
    channel_y1 = wall + inner_y
    puit_r = puit_d / 2.0
    # Wells sit on the Wago side of the channel so the cut does not undercut
    # the M3 bosses fused into the slotted wall on the outer muret lines.
    puits_y = channel_y0 + puit_r + 0.84
    puits = [(bays_x[0], puits_y), (bays_x[-1], puits_y)]

    # Channel floor is the 3 mm retaining lip. Rails sit 1 mm below it.
    boss_h = measured("seuil_canal")
    well_h = floor + boss_h - puit_fond + 0.2

    pilier_y = channel_y1 - pilier_r + 1.2
    piliers = [
        (div_x[0] + muret_ep / 2.0, pilier_y),
        (div_x[-1] + muret_ep / 2.0, pilier_y),
    ]
    dx = piliers[0][0] - puits[0][0]
    dy = piliers[0][1] - puits[0][1]
    if (dx * dx + dy * dy) ** 0.5 < puit_r + pilier_r + 0.4:
        reject(
            f"magnet well undercuts the M3 boss in the {canal:.2f} mm "
            f"channel: raise largeur",
            param="largeur",
        )

    return SimpleNamespace(
        wall=wall,
        floor=floor,
        inner_x=inner_x,
        inner_y=inner_y,
        outer_x=outer_x,
        outer_y=outer_y,
        hauteur=hauteur,
        n=n,
        bay=bay,
        muret_ep=muret_ep,
        muret_l=wago_y,
        muret_h=muret_h,
        div_x=div_x,
        bays_x=bays_x,
        bay_cy=bay_cy,
        wago_y0=wago_y0,
        wago_y1=wago_y1,
        canal=canal,
        rail_ep=rail_ep,
        rail_h=rail_h,
        rail_y=rail_y,
        channel_y0=channel_y0,
        channel_y1=channel_y1,
        puit_r=puit_r,
        puit_fond=puit_fond,
        aimant_h=aimant_h,
        boss_h=boss_h,
        well_h=well_h,
        puits=puits,
        fente_w=fente_w,
        fente_h=fente_h,
        fentes=list(bays_x),
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

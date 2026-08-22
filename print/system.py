"""Shared geometry: Wago derivation box layout, and the printable thread."""

import math
from types import SimpleNamespace

from nurb import (
    Align,
    Axis,
    Box,
    Cone,
    Cylinder,
    Helix,
    Plane,
    Polygon,
    Pos,
    RegularPolygon,
    extrude,
    measured,
    reject,
    sweep,
)

# Mouth lead so a bed-flush nut trap still cuts the floor cleanly (same idea as
# nurb.holes.counterbore).
_NUT_LEAD = 1.0
_NUT_SEATED = (Align.CENTER, Align.CENTER, Align.MIN)


def dims(longueur, largeur, hauteur, epaisseur_paroi, jeu_couvercle=0.3, n=4):
    """Standing Wago 221-423 layout. Origin at the outer min corner.

    Back wall at y = 0: each Wago is pressed against it. Slotted wall at
    y = outer_y: open-top slots so a pre-wired connector drops in from +Z
    and its three wires slide into the slot facing it. Magnets sit in the
    empty run between the Wago fronts and that wall, not under the bays.

    n=4: four identical bays, leftover split across all four; two M3 bosses
    on the outer muret lines; two magnet wells in the end bays.
    n=2: two end bays against the short walls; middle gap shrinks to a square
    M3 block (side = gap) against the back wall between the murets; one magnet
    well centred in the channel in front of that block.
    """
    if n not in (2, 4):
        reject(f"n must be 2 or 4, got {n}", param="longueur")

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
    # Slot floor sits 5 mm above the Wago base (floor + rails), so the three
    # wires do not have to kink down into a shallow notch.
    fente_dessus_rail = measured("fente_dessus_rail")
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

    pilier_r = pilier_d / 2.0
    bay_fit = wago_x + jeu_slide

    if n == 4:
        # Four identical bays: leftover length is split across all four, never
        # parked at the ends (that made the outer compartments look twice as wide).
        wago_block = n * bay_fit + (n - 1) * muret_ep
        leftover = inner_x - wago_block
        if leftover < -0.05:
            reject(
                f"longueur {longueur} cannot fit four identical "
                f"{bay_fit:.2f} mm bays and three {muret_ep} mm murets: "
                f"raise longueur above {wago_block}",
                param="longueur",
            )
        bay = (inner_x - (n - 1) * muret_ep) / n
        x0 = wall
        bays_x = [x0 + bay / 2.0 + i * (bay + muret_ep) for i in range(n)]
        div_x = [x0 + bay + i * (bay + muret_ep) for i in range(n - 1)]
    else:
        # End bays fixed at bay_fit. Middle gap is the square M3 block (nut trap):
        # as narrow as carre_ecrou_min, filled solid so it bonds to both murets
        # and the back. 4w still uses pilier_d for its round bosses.
        n_muret = 2
        wago_block = n * bay_fit + n_muret * muret_ep
        leftover = inner_x - wago_block
        carre_min = measured("carre_ecrou_min")
        if leftover < carre_min - 0.05:
            reject(
                f"longueur {longueur} leaves only {leftover:.2f} mm between the "
                f"two end bays, under the {carre_min} mm square for an M3 nut: "
                f"raise longueur above {wago_block + carre_min}",
                param="longueur",
            )
        bay = bay_fit
        x0 = wall
        bays_x = [x0 + bay / 2.0, x0 + inner_x - bay / 2.0]
        div_x = [x0 + bay, x0 + inner_x - bay - muret_ep]

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

    fente_bottom = floor + rail_h + fente_dessus_rail
    fente_h = hauteur - fente_bottom
    if fente_h < 6.0:
        reject(
            f"hauteur {hauteur} leaves only {fente_h:.2f} mm of slot above the "
            f"{fente_dessus_rail} mm wire clearance: raise hauteur",
            param="hauteur",
        )

    wago_y0 = wall
    wago_y1 = wall + wago_y
    bay_cy = wall + wago_y / 2.0

    rail_inset = 4.0
    rail_y = [wago_y0 + rail_inset, wago_y1 - rail_inset - rail_ep]

    channel_y0 = wago_y1
    channel_y1 = wall + inner_y
    puit_r = puit_d / 2.0
    # Wells sit on the Wago side of the channel so the cut does not undercut
    # the M3 bosses fused into the slotted wall (4w) or the square block (2w).
    puits_y = channel_y0 + puit_r + 0.84
    pilier_y = channel_y1 - pilier_r + 1.2

    # Channel floor is the 3 mm retaining lip. Rails sit 1 mm below it.
    boss_h = measured("seuil_canal")
    well_h = floor + boss_h - puit_fond + 0.2

    # Optional square boss for n=2. None for the cylindrical 4w pillars.
    carre = None

    if n == 4:
        puits = [(bays_x[0], puits_y), (bays_x[-1], puits_y)]
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
    else:
        # Square fills the middle gap, sits on the back wall between the murets.
        # Screw on the square centre. Magnet centred in the channel in front —
        # same X, different Y, so both read as centred without stacking.
        cx = wall + inner_x / 2.0
        side = leftover
        carre_x0 = div_x[0] + muret_ep
        carre_y0 = wago_y0
        carre = (carre_x0, carre_y0, side, side)
        piliers = [(cx, carre_y0 + side / 2.0)]
        puits = [(cx, puits_y)]
        # Square front face must clear the magnet well.
        square_front = carre_y0 + side
        if puits_y - puit_r < square_front + 0.4:
            reject(
                f"centred magnet well collides with the {side:.2f} mm square "
                f"M3 block: raise largeur",
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
        fente_bottom=fente_bottom,
        fentes=list(bays_x),
        vis_trou=vis_trou,
        vis_pass=vis_pass,
        vis_longueur=measured("vis_longueur"),
        ecrou_plats=measured("ecrou_m3_plats"),
        ecrou_ep=measured("ecrou_m3_epaisseur"),
        pilier_r=pilier_r,
        piliers=piliers,
        carre=carre,
        lid_th=lid_th,
        jupe_h=jupe_h,
        jupe_th=jupe_th,
        jeu=jeu_couvercle,
        jeu_wago=jeu_wago,
        wago_x=wago_x,
        wago_y=wago_y,
        wago_z=wago_z,
    )


def m3_nut_trap(shaft_dia, nut_af, nut_th, shoulder_z, depth, layer=1.0, slack=0.2):
    """Captive M3 hex nut from the bed, clearance hole to the rim.

    Mouth on the chassis face: hex prism up to `shoulder_z` (nut top bears there),
    two sacrificial bridge slots like `counterbore`, then a shaft of `shaft_dia`
    through `depth`. Size the hex on across-flats + slack; rotate so a flat faces
    +Y for a cleaner socket. `depth` may overshoot the rim.
    """
    import math

    if shaft_dia <= 0 or nut_af <= 0 or nut_th <= 0 or layer <= 0:
        raise ValueError("m3_nut_trap needs positive dimensions")
    if shoulder_z < nut_th:
        reject(
            f"nut shoulder at {shoulder_z:.2f} mm is under the {nut_th} mm nut: "
            f"raise the box or shorten the screw",
        )
    if depth < shoulder_z + 2 * layer:
        reject(
            f"depth {depth:.2f} leaves no shaft above the nut seat at "
            f"{shoulder_z:.2f} mm plus {2 * layer:.2f} mm of bridges",
        )

    af = nut_af + slack
    hex_face = RegularPolygon(af / 2.0, 6, major_radius=False, rotation=30)
    hex_prism = Pos(0, 0, -_NUT_LEAD) * extrude(hex_face, shoulder_z + _NUT_LEAD)
    # Circumscribed circle of the pocket, for clipping the bridge slots.
    ac = af / math.cos(math.radians(30))
    inside = Pos(0, 0, shoulder_z) * Cylinder(ac / 2.0, 2 * layer, align=_NUT_SEATED)
    first = (
        Pos(0, 0, shoulder_z) * Box(ac, shaft_dia, layer, align=_NUT_SEATED) & inside
    )
    second = (
        Pos(0, 0, shoulder_z + layer)
        * Box(shaft_dia, ac, layer, align=_NUT_SEATED)
        & inside
    )
    shaft = Pos(0, 0, -_NUT_LEAD) * Cylinder(
        shaft_dia / 2.0, depth + _NUT_LEAD, align=_NUT_SEATED
    )
    return hex_prism + first + second + shaft


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


def lid_hold_murets(d):
    """Lid ribs aligned with the box bay dividers, plus a front stop bar.

    Same idea as commercial Wago lids: short murets (jupe height) on the divider
    lines so the lid separates the connectors, and a bar just past the Wago front
    (depth + jeu_wago) that blocks them and braces into the end skirts.
    On the 2w box the front bar is one segment per end bay so the middle stays open
    over the square boss. Lid frame: slotted wall at y=0.
    """
    from nurb import Align, Box, Pos

    amin = (Align.MIN, Align.MIN, Align.MIN)
    ox = d.wall + d.jeu
    sx = d.inner_x - 2.0 * d.jeu
    # Back skirt inner face (lid Y); murets butt into it like the reference frame.
    y_back = ox + (d.inner_y - 2.0 * d.jeu) - d.jupe_th
    # Front stop: inner face at wago_y1 + jeu_wago so the 18.6 mm body fits.
    front_box = d.wago_y1 + d.jeu_wago
    y_front = d.outer_y - front_box - d.muret_ep
    # Murets run from the front bar to the back skirt (no 0.3 mm gap).
    y_muret = y_front
    muret_len = y_back - y_muret
    if muret_len < 1.0:
        from nurb import reject

        reject(
            f"lid hold murets only {muret_len:.2f} mm long: check largeur / jupe",
            param="largeur",
        )

    body = None
    for x in d.div_x:
        rib = Pos(x, y_muret, d.lid_th) * Box(
            d.muret_ep, muret_len, d.jupe_h, align=amin
        )
        body = rib if body is None else body + rib

    if d.n == 2:
        # One cell per end bay: bar runs from the end skirt to that bay's muret.
        spans = [
            (ox, d.div_x[0] + d.muret_ep - ox),
            (d.div_x[1], ox + sx - d.div_x[1]),
        ]
    else:
        spans = [(ox, sx)]

    for x0, w in spans:
        if w < 0.5:
            continue
        bar = Pos(x0, y_front, d.lid_th) * Box(
            w, d.muret_ep, d.jupe_h, align=amin
        )
        body = body + bar
    return body


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


_CMIN = (Align.CENTER, Align.CENTER, Align.MIN)


def _fuse_one(shape):
    """Boolean-union every solid in a compound. `+` sometimes leaves a compound."""
    solids = list(shape.solids())
    if not solids:
        return shape
    body = solids[0]
    for s in solids[1:]:
        body = body.fuse(s)
    return body


def barreau_filete(
    major_dia,
    pitch,
    depth,
    height,
    jeu_radial=0.0,
    start=0.8,
    alpha=35.0,
    collar_h=None,
    chanfrein_tete=0.0,
):
    """Printable sawtooth thread, male bar or female cutter.

    Profile taken off a cable gland found online: a sawtooth rather than an ISO
    60° V. A pure 45° axial flank reads ~51° after helix/Frenet, so axial
    `alpha` 35° is what lands the underside under 45°. Always fused to one
    solid before return.

    `start` is the plain run below the first turn, `collar_h` the full-major
    cylinder at the bottom (defaults to start + 0.3). Used as a cutter for a
    female thread, pass collar_h=0: a full-major collar there counterbores the
    bore mouth and leaves the first thread ridge printing on air.
    """
    if collar_h is None:
        collar_h = start + 0.3
    r_maj = major_dia / 2.0 + jeu_radial
    r_min = r_maj - depth
    if r_min < 1.0:
        reject(
            f"thread minor radius {r_min:.2f} mm is under 1 mm: "
            "lower depth or raise major_dia",
        )
    if height <= start + pitch:
        reject(
            f"thread height {height} mm is too short for a {start} mm plain "
            f"run plus one {pitch} mm turn: raise it",
        )
    dz = depth / math.tan(math.radians(alpha))
    bite = 0.15
    # Crest flat = whatever the pitch has left once the 35° underside and a 45°
    # return flank are paid for. A 0.12 mm crest is thinner than one 0.42 mm
    # bead and fires min_wall; spending the slack on the flat is free.
    flat = max(0.12, pitch - dz - (depth + bite))
    if pitch + 0.05 < dz + flat + 0.2:
        reject(
            f"pitch {pitch} mm cannot fit a {alpha}° flank (dz {dz:.2f}) plus "
            f"crest flat: lower depth or raise pitch",
        )
    # Sawtooth: gentle underside (printable), short crest flat, steeper return.
    z_crest = dz + flat
    pts = [
        (r_min - bite, 0.0),
        (r_maj, dz),
        (r_maj, z_crest),
        (r_min - bite, pitch),
    ]
    face = Plane.XZ * Polygon(*pts, align=None)
    helix_h = height - start
    path = Helix(pitch=pitch, height=helix_h, radius=r_min, center=(0, 0, start))
    crest = sweep(face, path=path, is_frenet=True)
    core = Cylinder(r_min, height + 0.2, align=_CMIN)
    # Fuse the helix onto the core FIRST. `collar.fuse(core).fuse(crest)`
    # silently returns the crest alone (measured: 94mm3 instead of 1179) —
    # OCCT loses the operands when a coaxial cylinder pair meets a swept
    # helix. Core + crest, then the collar, is stable.
    body = _fuse_one(core + crest)
    if collar_h > 0.0:
        body = _fuse_one(body + Cylinder(r_maj, collar_h, align=_CMIN))
    trim = Pos(0, 0, height) * Box(50, 50, pitch + 4.0, align=_CMIN)
    body = _fuse_one(body - trim)
    if chanfrein_tete > 0.0:
        # Bolt-tip chamfer. Without it the trim plane knifes the last turn
        # mid-tooth and leaves a 0.37 mm section (min_wall) at the tip.
        z0 = height - chanfrein_tete
        band = Pos(0, 0, z0) * Cylinder(r_maj + 1.0, chanfrein_tete, align=_CMIN)
        cone = Pos(0, 0, z0) * Cone(
            r_min + chanfrein_tete, r_min, chanfrein_tete, align=_CMIN
        )
        body = _fuse_one(body - (band - cone))
    return body

from nurb import *

from system import MARGE_PUIT, entretoise_m2, puit_debout


def _intersect(a1, a2, b1, b2):
    ax, ay = a1
    bx, by = a2[0] - a1[0], a2[1] - a1[1]
    cx, cy = b1
    dx, dy = b2[0] - b1[0], b2[1] - b1[1]
    det = bx * dy - by * dx
    if abs(det) < 1e-12:
        return a2
    t = ((cx - ax) * dy - (cy - ay) * dx) / det
    return (ax + t * bx, ay + t * by)


def _inward(p0, p1):
    dx, dy = p1[0] - p0[0], p1[1] - p0[1]
    length = (dx * dx + dy * dy) ** 0.5
    return (-dy / length, dx / length)


def _offset_in(pts, d):
    """Inward offset of a CCW polygon: intersect consecutive offset edges."""
    n = len(pts)
    shifted = []
    for i in range(n):
        p0, p1 = pts[i], pts[(i + 1) % n]
        nx, ny = _inward(p0, p1)
        shifted.append(
            ((p0[0] + nx * d, p0[1] + ny * d), (p1[0] + nx * d, p1[1] + ny * d))
        )
    out = []
    for i in range(n):
        a1, a2 = shifted[i - 1]
        b1, b2 = shifted[i]
        out.append(_intersect(a1, a2, b1, b2))
    return out


def _fuse_one(shape):
    solids = list(shape.solids())
    if len(solids) <= 1:
        return shape
    body = solids[0]
    for s in solids[1:]:
        body = body.fuse(s)
    return body


def _add_well(body, outer, cx, cy, diametre, fond, aimant_h, marge):
    pad, cutter = puit_debout(cx, cy, diametre, fond, aimant_h, marge)
    clipped = pad.intersect(outer)
    if clipped is not None:
        body = body + clipped
    return body - cutter


def _contour(wall, gouttiere_depuis_x, gout_bas, gout_haut):
    x_max = measured("boitier_int_x")
    y_max = measured("boitier_int_y")
    aile_x = measured("boitier_int_aile_x")
    aile_y = measured("boitier_int_aile_y")
    chanfrein = measured("boitier_int_chanfrein")
    marche_x = measured("boitier_int_marche_x")
    marche_y = measured("boitier_int_marche_y")
    return [
        (0.0, chanfrein),
        (chanfrein, 0.0),
        (aile_x, 0.0),
        (aile_x, aile_y),
        (gouttiere_depuis_x, aile_y),
        (gouttiere_depuis_x, gout_bas),
        (x_max, gout_bas),
        (x_max, gout_haut),
        (gouttiere_depuis_x, gout_haut),
        (gouttiere_depuis_x, y_max),
        (marche_x, y_max),
        (marche_x, marche_y),
        (0.0, marche_y),
    ]


@part
def boitier_int(
    hauteur=10.0,
    epaisseur_paroi=1.6,
    puit_diametre=8.2,
    puit_peau=0.6,
    marge_puit=MARGE_PUIT,
    hauteur_plateforme=3.6,
    muret_wago=4.0,
    gouttiere_depuis_x=85.0,
    gouttiere_largeur=10.0,
    gouttiere_vide_haut=10.0,
    capteur_recul=11.0,
    capteur_entraxe_y=20.0,
    capteur_entraxe_x=30.0,
    module_bas_y=25.0,
    module_bas_entraxe_x=20.0,
    module_bas_entraxe_y=35.0,
    module_bas_decalage_x=-1.0,
    draft=False,
):
    """Fond du boîtier intérieur. Section haute jusqu'à 85 mm, gouttière 10 mm ensuite.

    hauteur: hauteur hors-tout depuis le lit (murs compris)
    epaisseur_paroi: épaisseur du fond et des murs, vers l'intérieur
    puit_diametre: diamètre intérieur du puits d'aimant Ø8×3 (8,2 : le disque glisse jusqu'à la face 0,6 mm)
    puit_peau: plastique sous l'aimant (même cote que puit_fond)
    marge_puit: plastique autour du puits (doctrine 1,6 mm)
    hauteur_plateforme: assise Wago et épaulement des entretoises ; 3,6 = fond 0,6 + aimant 3, à fleur
    muret_wago: muret devant le logement Wago (le mur droit est à `hauteur`)
    gouttiere_depuis_x: tout ce qui est à droite de cette cote est la gouttière
    gouttiere_largeur: largeur interne de la gouttière en Y, murs non compris
    gouttiere_vide_haut: vide entre le bord haut (Y max) et le haut de la gouttière
    capteur_recul: centre de la première entretoise, à droite du mur Wago
    capteur_entraxe_y: écart Y des deux entretoises collées au mur du haut
    capteur_entraxe_x: la troisième entretoise est plus loin en X, à mi-Y
    module_bas_y: première entretoise du module bas, au-dessus de l'angle (50, 0)
    module_bas_entraxe_x: les deux de gauche sont plus à gauche
    module_bas_entraxe_y: les deux du haut sont plus haut
    module_bas_decalage_x: décalage X des quatre entretoises du module bas (négatif = vers -X)
    """
    wall = epaisseur_paroi
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")
    x_max = measured("boitier_int_x")
    y_max = measured("boitier_int_y")
    aile_y = measured("boitier_int_aile_y")
    aile_x = measured("boitier_int_aile_x")
    marche_x = measured("boitier_int_marche_x")
    wago_x = measured("wago_largeur")
    wago_y = measured("wago_profondeur")
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
    if hauteur_plateforme < well_stack:
        reject(
            f"hauteur_plateforme {hauteur_plateforme} is under the magnet stack "
            f"({well_stack:.1f} mm): raise it so the Wago sits flush",
            param="hauteur_plateforme",
        )
    if muret_wago < hauteur_plateforme + 0.4:
        reject(
            f"muret_wago {muret_wago} leaves under 0.4 mm of lip above the "
            f"{hauteur_plateforme:.1f} mm platform: raise it",
            param="muret_wago",
        )
    if gouttiere_depuis_x < marche_x + wago_x + wall + 40.0:
        reject(
            f"gouttiere_depuis_x {gouttiere_depuis_x} leaves no room for the "
            "Wago bay and the temperature module: raise it",
            param="gouttiere_depuis_x",
        )
    if gouttiere_depuis_x >= x_max - 2.0 * wall - puit_diametre:
        reject(
            f"gouttiere_depuis_x {gouttiere_depuis_x} leaves no gutter before "
            f"x={x_max}: lower it",
            param="gouttiere_depuis_x",
        )
    if gouttiere_largeur < 8.0:
        reject(
            f"gouttiere_largeur {gouttiere_largeur} is under 8 mm: raise it",
            param="gouttiere_largeur",
        )
    if gouttiere_vide_haut < 0.5:
        reject(
            f"gouttiere_vide_haut {gouttiere_vide_haut} collapses the top "
            "edge of the gutter into y_max: raise it",
            param="gouttiere_vide_haut",
        )
    gout_outer = gouttiere_largeur + 2.0 * wall
    if gouttiere_vide_haut + gout_outer > y_max - aile_y:
        reject(
            f"gouttiere_vide_haut {gouttiere_vide_haut} plus the "
            f"{gout_outer:.1f} mm gutter drops below y={aile_y}: lower it",
            param="gouttiere_vide_haut",
        )
    gout_haut = y_max - gouttiere_vide_haut
    gout_bas = gout_haut - gout_outer
    if hauteur < well_stack + 0.4:
        reject(
            f"hauteur {hauteur} is under the magnet well ({well_stack + 0.4:.1f} mm): "
            "raise it",
            param="hauteur",
        )

    outer_pts = _contour(wall, gouttiere_depuis_x, gout_bas, gout_haut)
    inner_pts = _offset_in(outer_pts, wall)
    amin = (Align.MIN, Align.MIN, Align.MIN)

    outer = extrude(Polygon(*outer_pts, align=None), hauteur)
    cavity = Pos(0, 0, wall) * extrude(
        Polygon(*inner_pts, align=None), hauteur + 0.2
    )
    body = outer - cavity

    # Wago 221-423 bay, top-left of the high section. Inner left / top walls
    # of the box close two sides; a full-height divider and a 4 mm front lip
    # close the others. Platform is magnet-stack height so the Wago sits
    # flush with the disc.
    plat_x0 = marche_x + wall
    plat_x1 = plat_x0 + wago_x
    plat_y1 = y_max - wall
    plat_y0 = plat_y1 - wago_y
    platform = Pos(plat_x0, plat_y0, 0) * Box(
        wago_x, wago_y, hauteur_plateforme, align=amin
    )
    divider = Pos(plat_x1, plat_y0 - wall, 0) * Box(
        wall, y_max - (plat_y0 - wall), hauteur, align=amin
    )
    front = Pos(plat_x0, plat_y0 - wall, 0) * Box(
        wago_x + wall, wall, muret_wago, align=amin
    )
    body = body + platform + divider + front

    puit_r = puit_diametre / 2.0
    wells = [
        (puit_bas_x, puit_bas_y),
        (plat_x0 + puit_r + marge_puit, plat_y1 - puit_r - marge_puit),
        (
            gouttiere_depuis_x + (x_max - gouttiere_depuis_x) / 2.0,
            gout_bas + wall + gouttiere_largeur / 2.0,
        ),
    ]
    for cx, cy in wells:
        body = _add_well(
            body, outer, cx, cy, puit_diametre, puit_peau, aimant_h, marge_puit
        )

    # Three M2 spacers for the ~20 × 40 mm temperature module, to the right
    # of the Wago divider, first two against the top inner wall. Four more
    # for the module in the lower bay, from the (aile_x, 0) corner: first
    # against that vertical inner face (2 mm to centre, Ø4 so it does not
    # enter the 1.6 mm wall), 25 mm up, then a 20 × 35 mm rectangle.
    e_r = 2.0
    e1x = plat_x1 + wall + capteur_recul
    e1y = y_max - wall - e_r
    b1x = aile_x - wall - e_r + module_bas_decalage_x
    b1y = module_bas_y
    posts = [
        (e1x, e1y),
        (e1x, e1y - capteur_entraxe_y),
        (e1x + capteur_entraxe_x, e1y - capteur_entraxe_y / 2.0),
        (b1x, b1y),
        (b1x - module_bas_entraxe_x, b1y),
        (b1x, b1y + module_bas_entraxe_y),
        (b1x - module_bas_entraxe_x, b1y + module_bas_entraxe_y),
    ]
    for px, py in posts:
        body = body + Pos(px, py, 0) * entretoise_m2(hauteur=hauteur_plateforme)

    # Gutter exit: drop the end wall at x_max. Floor and the two long walls stay.
    margin = 0.5
    body = body - (
        Pos(x_max - wall - margin, gout_bas + wall, wall)
        * Box(wall + 2.0 * margin, gouttiere_largeur, hauteur + 0.2, align=amin)
    )

    # Chamfer wall (0, 20) → (20, 0): open the low-X half, leftmost
    # corner to the midpoint. Floor stays. Extra inward bite so the cut
    # clears the inner corner at the left wall (a tight inn left 0.27 mm).
    chanfrein = measured("boitier_int_chanfrein")
    s2 = 2.0 ** 0.5
    tx, ty = 1.0 / s2, -1.0 / s2
    nx, ny = 1.0 / s2, 1.0 / s2
    ax, ay = 0.0, chanfrein
    mx, my = chanfrein / 2.0, chanfrein / 2.0
    past = 1.0
    inn = wall + 2.0
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

    # Polish the box first... no: spacers are already fused. A 1 mm chamfer
    # on the Ø2 pin would eat it, so keep only convex verticals that are
    # not on the posts (centres within the Ø4 base).
    conc = {
        (round(e.center().X, 2), round(e.center().Y, 2), round(e.center().Z, 2))
        for e in concave_edges(body)
    }

    def keep(edge):
        c = edge.center()
        if (round(c.X, 2), round(c.Y, 2), round(c.Z, 2)) in conc:
            return False
        for px, py in posts:
            if (c.X - px) ** 2 + (c.Y - py) ** 2 < (e_r + 0.6) ** 2:
                return False
        return True

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

"""Shared geometry: printable thread, magnet wells, contour offset, cable-tie U."""

import math

from nurb import (
    Align,
    Axis,
    Box,
    CenterArc,
    Cone,
    Curve,
    Cylinder,
    Helix,
    Line,
    Plane,
    Polygon,
    Pos,
    extrude,
    make_face,
    reject,
    sweep,
)


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


def offset_in(pts, d):
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


def puit_couche_toit(rayon, pont=2.0):
    """Combien le toit de `puit_couche` monte au-dessus de l'axe du puits.

    C'est la cote dont le plafond au-dessus du puits doit se pousser : le toit
    remplace le sommet du cercle, il est donc plus haut que `rayon`.
    """
    return rayon * math.sqrt(2.0) - pont / 2.0


def puit_couche(rayon, profondeur, pont=2.0, chanfrein=0.5, debord=0.1):
    """Cutter d'un puits d'aimant dont l'axe est COUCHÉ sur le lit.

    Repère local : l'axe est +Z, le puits court de z=0 (la bouche, dans le plan
    de la face percée) à z=profondeur, et **le local +Y est le haut de
    l'impression**. À placer avec un `Plane` dont on donne le `x_dir`, sinon le
    toit part de travers.

    Un alésage rond imprimé couché finit en porte-à-faux : la tangente à 45°
    tombe à rayon/sqrt(2) au-dessus de l'axe, et tout ce qui est au-dessus
    (six couches sur un Ø8,2 à 0,2 mm) s'affaisse dans le trou — d'où l'ovale et
    le méplat au sommet. On remplace donc le haut du cercle par une tente à 45°
    tangente à l'alésage, tronquée par un pont plat de `pont` : plus rien ne
    dépasse 45° et la dernière portée est un pont banal. Le sommet monte à
    `puit_couche_toit(rayon, pont)` au-dessus de l'axe.

    La bouche prend un chanfrein `chanfrein` x 45° pour que l'aimant entre
    droit ; `chanfrein=0` le supprime, ce que demande un puits trop près d'une
    paroi pour lui laisser la place. Le cutter dépasse de `debord` en arrière de
    la face pour la couper proprement.
    """
    if pont <= 0.0:
        reject(f"pont {pont} doit être positif")
    if chanfrein < 0.0 or chanfrein >= profondeur:
        reject(f"chanfrein {chanfrein} ne tient pas dans un puits de {profondeur}")
    tangente = rayon / math.sqrt(2.0)
    toit = puit_couche_toit(rayon, pont)
    # Le pont doit rester au-dessus du sommet de l'alésage, sinon il le rabote et
    # l'aimant se coince entre deux plats au lieu de se poser dans un rond.
    if toit <= rayon:
        reject(
            f"pont {pont} raboterait un alésage de rayon {rayon} : "
            f"le réduire sous {2.0 * rayon * (math.sqrt(2.0) - 1.0):.2f}"
        )

    # Section dessinée d'un trait, pas unie de deux solides : la tente est
    # tangente à l'alésage, et deux volumes qui se touchent sans se traverser
    # laissent à OCCT une poignée de facettes sub-millimétriques par puits.
    # L'arc part de 135°, fait le tour par le bas et ressort à 45° ; les deux
    # pans de 45° montent de là au pont plat.
    demi_pont = pont / 2.0
    section = make_face(
        Curve()
        + [
            CenterArc((0.0, 0.0), rayon, 135.0, 270.0),
            Line((tangente, tangente), (demi_pont, toit)),
            Line((demi_pont, toit), (-demi_pont, toit)),
            Line((-demi_pont, toit), (-tangente, tangente)),
        ]
    )

    # Le fût à la cote commence après le chanfrein d'entrée, qui évase la section
    # elle-même à 45° pour que l'aimant entre droit ; `debord` prolonge dehors
    # pour couper la face proprement.
    cutter = extrude(Plane.XY.offset(chanfrein) * section, profondeur - chanfrein)
    if chanfrein > 0.0:
        bouche = cutter.faces().filter_by(Plane.XY).sort_by(Axis.Z)[0]
        cutter = _fuse_one(cutter + extrude(bouche, chanfrein, taper=-45))
    dehors = cutter.faces().filter_by(Plane.XY).sort_by(Axis.Z)[0]
    return _fuse_one(cutter + extrude(dehors, debord))


# Standing magnet well: 1.6 mm of plastic around the bore (four 0.4 mm
# perimeters on the A1 Mini). Same as the project walls. No lateral load
# on these discs — they pull through the 0.6 mm floor — so 2 mm was spare.
MARGE_PUIT = 1.6

_CMIN = (Align.CENTER, Align.CENTER, Align.MIN)
_AMIN = (Align.MIN, Align.MIN, Align.MIN)


def puit_debout(cx, cy, diametre, fond, aimant_h, marge=MARGE_PUIT):
    """Pad + cutter of a standing Ø8×3 well. Pad is `fond + aimant_h` tall.

    Place the pad first, clip it to the outer solid if the well sits in a
    wall, then subtract the cutter. The cutter overshoots the pad by 0.1 mm
    so the pocket has no ceiling.
    """
    r = diametre / 2.0
    pad_h = fond + aimant_h
    pad = Pos(cx, cy, 0) * Cylinder(r + marge, pad_h, align=_CMIN)
    cutter = Pos(cx, cy, fond) * Cylinder(r, aimant_h + 0.1, align=_CMIN)
    return pad, cutter


def add_well(body, outer, cx, cy, diametre, fond, aimant_h, marge=MARGE_PUIT):
    """Fuse a standing magnet pad into `body` and cut the pocket, clipped to `outer`."""
    pad, cutter = puit_debout(cx, cy, diametre, fond, aimant_h, marge)
    clipped = pad.intersect(outer)
    if clipped is not None:
        body = body + clipped
    return body - cutter


def entretoise_m2(
    hauteur=3.0,
    diametre_base=4.0,
    diametre_pion=2.0,
    hauteur_pion=2.0,
):
    """Entretoise M2 réutilisable : épaulement Ø4 à `hauteur`, pion Ø2 dessus.

    Le pion se loge dans un trou M2. C'est un nubby de positionnement, pas
    un plot chargé : 2 mm de haut sur Ø2, trop court pour le `pin` de nurb.
    Imprimée avec la pièce qui l'appelle.
    """
    if diametre_base < diametre_pion + 1.2:
        reject(
            f"diametre_base {diametre_base} leaves under 0.6 mm of shoulder "
            f"around a {diametre_pion} mm pin: raise it",
        )
    if hauteur < 1.2:
        reject(f"hauteur {hauteur} is under 1.2 mm: raise it")
    if diametre_pion < 2.0:
        reject(
            f"diametre_pion {diametre_pion} is under 2 mm: the nozzle will "
            "smear it. Raise it or drop the pin",
        )
    if hauteur_pion < 0.8:
        reject(f"hauteur_pion {hauteur_pion} is under 0.8 mm: raise it")
    base = Cylinder(diametre_base / 2.0, hauteur, align=_CMIN)
    pion = Pos(0, 0, hauteur) * Cylinder(
        diametre_pion / 2.0, hauteur_pion, align=_CMIN
    )
    return _fuse_one(base + pion)


def _fuse_one(shape):
    """Boolean-union every solid in a compound. `+` sometimes leaves a compound."""
    solids = list(shape.solids())
    if not solids:
        return shape
    body = solids[0]
    for s in solids[1:]:
        body = body.fuse(s)
    return body


def anti_tirage_ns(
    x0,
    y_inner,
    z0,
    toward_plus_y,
    wall=1.6,
    jeu=1.2,
    largeur=3.0,
    bords=5.6,
    hauteur_u=5.0,
):
    """Cable-tie U on a north or south inner wall, extruded in X.

    `z0` is the bottom of the 5 mm slot (where the tie sits). 45° ramps
    above and below, 3 mm centre open top and bottom so the tie comes
    out vertically. Fuse the result onto the box.
    """
    overlap = 0.4
    margin = 0.5
    at_out = jeu + wall
    at_leg = 0.5 * (bords - largeur)
    at_u0 = z0
    at_u1 = z0 + hauteur_u
    at_bot = at_u0 - at_out
    at_top = at_u1 + at_out
    span = bords
    if toward_plus_y:
        body = Pos(x0, 0, 0) * extrude(
            Plane.YZ
            * Polygon(
                (y_inner - overlap, at_bot),
                (y_inner, at_bot),
                (y_inner + at_out, at_u0),
                (y_inner + at_out, at_u1),
                (y_inner, at_top),
                (y_inner - overlap, at_top),
                align=None,
            ),
            span,
        )
        y_cut = y_inner
        y_sz = at_out + margin
        y_jeu = jeu
    else:
        body = Pos(x0 + span, 0, 0) * extrude(
            Plane.YZ
            * Polygon(
                (y_inner + overlap, at_bot),
                (y_inner, at_bot),
                (y_inner - at_out, at_u0),
                (y_inner - at_out, at_u1),
                (y_inner, at_top),
                (y_inner + overlap, at_top),
                align=None,
            ),
            span,
        )
        y_cut = y_inner - at_out
        y_sz = at_out
        y_jeu = jeu
        y_inner_gap = y_inner - jeu
    body = body - (
        Pos(x0 + at_leg, y_cut, at_bot - margin)
        * Box(largeur, y_sz, at_out + margin, align=_AMIN)
    )
    if toward_plus_y:
        body = body - (
            Pos(x0 + at_leg, y_inner, at_u0)
            * Box(largeur, y_jeu, hauteur_u, align=_AMIN)
        )
    else:
        body = body - (
            Pos(x0 + at_leg, y_inner_gap, at_u0)
            * Box(largeur, y_jeu, hauteur_u, align=_AMIN)
        )
    body = body - (
        Pos(x0 + at_leg, y_cut, at_u1)
        * Box(largeur, y_sz, at_out + margin, align=_AMIN)
    )
    return _fuse_one(body)


def anti_tirage_ew(
    y0,
    x_inner,
    z0,
    toward_plus_x,
    wall=1.6,
    jeu=1.2,
    largeur=3.0,
    bords=5.6,
    hauteur_u=5.0,
):
    """Cable-tie U on an east or west inner wall, extruded in Y.

    Same recipe as `anti_tirage_ns`. `toward_plus_x` True is a west wall
    (inward +X); False is an east wall (inward −X).
    """
    overlap = 0.4
    margin = 0.5
    at_out = jeu + wall
    at_leg = 0.5 * (bords - largeur)
    at_u0 = z0
    at_u1 = z0 + hauteur_u
    at_bot = at_u0 - at_out
    at_top = at_u1 + at_out
    span = bords
    if toward_plus_x:
        body = Pos(0, y0, 0) * extrude(
            Plane.XZ
            * Polygon(
                (x_inner - overlap, at_bot),
                (x_inner, at_bot),
                (x_inner + at_out, at_u0),
                (x_inner + at_out, at_u1),
                (x_inner, at_top),
                (x_inner - overlap, at_top),
                align=None,
            ),
            span,
        )
        x_cut = x_inner
        x_sz = at_out + margin
        x_gap = x_inner
    else:
        body = Pos(0, y0, 0) * extrude(
            Plane.XZ
            * Polygon(
                (x_inner + overlap, at_bot),
                (x_inner, at_bot),
                (x_inner - at_out, at_u0),
                (x_inner - at_out, at_u1),
                (x_inner, at_top),
                (x_inner + overlap, at_top),
                align=None,
            ),
            span,
        )
        x_cut = x_inner - at_out
        x_sz = at_out
        x_gap = x_inner - jeu
    body = body - (
        Pos(x_cut, y0 + at_leg, at_bot - margin)
        * Box(x_sz, largeur, at_out + margin, align=_AMIN)
    )
    body = body - (
        Pos(x_gap, y0 + at_leg, at_u0)
        * Box(jeu, largeur, hauteur_u, align=_AMIN)
    )
    body = body - (
        Pos(x_cut, y0 + at_leg, at_u1)
        * Box(x_sz, largeur, at_out + margin, align=_AMIN)
    )
    return _fuse_one(body)


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

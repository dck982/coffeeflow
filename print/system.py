"""Shared geometry: printable thread, magnet wells, contour offset, cable-tie U."""

import math
from collections import namedtuple

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
    measured,
    polish,
    reject,
    sweep,
)

def bbox(x0, y0, w, l, z0=0.0, h=1.0):
    return (
        Pos(x0, y0, z0) * Box(w,l,h,align=AMIN)
        ).bounding_box()

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


# Heat-insert bores. `encombrement` is the outer diameter of the housing
# (perçage + 2x l'épaisseur de paroi mini), for a circular pad or annulus
# around the bore; a part using a square/rectangular pad instead can still
# size it off `encombrement` as the footprint's side.
HeatInsert = namedtuple(
    "HeatInsert",
    "diametre_percage epaisseur_paroi_min profondeur_min encombrement",
)

INSERT_M3 = HeatInsert(
    diametre_percage=4.0,
    epaisseur_paroi_min=1.6,
    profondeur_min=5.0,
    encombrement=7.2,
)
INSERT_M25 = HeatInsert(
    diametre_percage=3.9,
    epaisseur_paroi_min=1.5,
    profondeur_min=4.5,
    encombrement=6.9,
)
INSERT_M2 = HeatInsert(
    diametre_percage=3.2,
    epaisseur_paroi_min=1.2,
    profondeur_min=3.5,
    encombrement=5.6,
)

CMIN = (Align.CENTER, Align.CENTER, Align.MIN)
AMIN = (Align.MIN, Align.MIN, Align.MIN)

def add_heat_insert(body, outer, cx, cy, z0, height, insert_def, ring_factor=1.0):
    outer_pad = Pos(cx, cy, z0) * Cylinder(
        insert_def.encombrement*ring_factor/2, height, align=CMIN
    )
    inner_void = Pos(cx, cy, z0) * Cylinder(
        insert_def.diametre_percage/2, height, align=CMIN
    )
    ring = outer_pad - inner_void
    clipped = ring.intersect(outer)
    if clipped is not None:
        body = body + clipped
    return body - inner_void

def add_wall(body, outer, x, y, dx, dy, z0, height):
    wall = Pos(x, y, z0) * Box(
        dx,
        dy,
        height,
        align=AMIN,
    )
    return body + wall.intersect(outer)

def magnet_well_cutter(cx, cy, z0, diameter=0, height=0):
    dz = height if height>0 else measured("aimant_hauteur")
    dd = diameter if diameter>0 else measured("aimant_diametre")
    return Pos(cx, cy, z0 + measured("aimant_puit_fond")) * Cylinder(
        (dd+measured("aimant_puit_press_fit")) / 2.0,
        dz, align=CMIN
    )

def small_magnet_well_cutter(cx, cy, z0, height=0):
    dz = height if height>0 else measured("aimant_petit_hauteur")
    return magnet_well_cutter(cx, cy, z0, height=dz, diameter=measured("aimant_petit_diametre"))

def add_well(body, outer, cx, cy, z0=0, h=None):
    """Fuse a standing magnet pad into `body` and cut the pocket, clipped to `outer`."""

    # first a pad
    d = measured("aimant_diametre") + measured("aimant_puit_press_fit")
    if h is None:
        h = measured("aimant_hauteur")
    pad = Pos(cx, cy, z0) * Cylinder(d/2 + measured("aimant_puit_mur"), h, align=CMIN)
    cutter = magnet_well_cutter(cx, cy, z0, diameter=d, height=h)
    clipped = pad.intersect(outer)
    if clipped is not None:
        body = body + clipped
    return body - cutter

def add_corbel(body, x, y, hauteur, insert=INSERT_M3, plane=Plane.XZ, reverse=False, flush=False):
    corbel_diametre = insert.diametre_percage
    corbel_profondeur = insert.profondeur_min
    corbel_paroi = insert.epaisseur_paroi_min
    corbel_r = corbel_diametre / 2.0
    corbel_plat = corbel_diametre + corbel_paroi
    corbel_along = insert.diametre_percage + 2*corbel_paroi
    z_corbel = hauteur - corbel_profondeur
    z_corbel_45 = z_corbel - corbel_plat
    ov = 0.4
    multiplier = -1.0 if reverse else 1.0
    corbel_pts = [
        (-ov * multiplier, z_corbel_45),
        (0.0, z_corbel_45),
        (corbel_plat * multiplier, z_corbel),
        (corbel_plat * multiplier, hauteur),
        (-ov * multiplier, hauteur),
    ]

    if plane==Plane.XZ:
        plat_x = x
        plat_y = y+corbel_along
        hole_x = x + (corbel_r if flush else corbel_plat/2) * multiplier
        hole_y = y + corbel_paroi + corbel_r
    else:
        plat_x = x
        plat_y = y
        hole_x = x + corbel_paroi + corbel_r
        hole_y = y + (corbel_r if flush else corbel_plat/2)
    body = body + (
        Pos(plat_x, plat_y, 0)
        * extrude(plane * Polygon(*corbel_pts, align=None), corbel_along)
    )
    body = body - (
        Pos(hole_x, hole_y, z_corbel)
        * Cylinder(corbel_r, corbel_profondeur + 0.1, align=CMIN)
    )
    return body

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
    base = Cylinder(diametre_base / 2.0, hauteur, align=CMIN)
    pion = Pos(0, 0, hauteur) * Cylinder(
        diametre_pion / 2.0, hauteur_pion, align=CMIN
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


def surplomb_hook_pts(xy, z_mid, surplomb_w, inverse=False, overlap=0.0):
    """Section of a 45° Wago catch in the wall's plane.

    `xy` is the inner face of the wall. The catch returns `surplomb_w` toward
    the bay; `inverse` flips that toward +X / +Y. `overlap` bites into the
    wall so the boolean fuses. `z_mid` is the bottom of the vertical face.
    """
    multiplier = -1.0 if inverse else 1.0
    wall_xy = xy + overlap * multiplier
    return [
        (wall_xy, z_mid - surplomb_w),
        (xy - surplomb_w * multiplier, z_mid),
        (xy - surplomb_w * multiplier, z_mid + surplomb_w),
        (wall_xy, z_mid + surplomb_w),
    ]


def surplomb(xy, along0, z_mid, sw, slen, *, plane=Plane.XZ, inverse=False, z0=0.0, overlap=0.0):
    """Wago catch solid. `along0` + `slen` are the extrusion start and signed length."""
    pts = surplomb_hook_pts(xy, z_mid, sw, inverse=inverse, overlap=overlap)
    face = plane * Polygon(*pts, align=None)
    if plane == Plane.XZ:
        return Pos(0, along0, z0) * extrude(face, slen)
    return Pos(along0, 0, z0) * extrude(face, slen)


def surplomb_xz(x, y, z, sw, slen, z0, inverse=False, overlap=0.0):
    """Catch ending at `y`, `slen` along Y (sign from `inverse`). Used by boitier_dc."""
    along0 = y - slen
    signed = slen * (-1.0 if inverse else 1.0)
    return surplomb(
        x, along0, z, sw, signed,
        plane=Plane.XZ, inverse=inverse, z0=z0, overlap=overlap,
    )


def add_hook(
    body,
    a0,
    inner,
    z0,
    *,
    ns=True,
    toward_plus=True,
    wall=1.6,
    jeu=1.2,
    largeur=3.0,
    bords=5.6,
    hauteur_u=5.0,
):
    """Cable-tie U on an inner wall. `ns` True is north/south (extruded in X)."""
    kw = dict(wall=wall, jeu=jeu, largeur=largeur, bords=bords, hauteur_u=hauteur_u)
    if ns:
        return body + anti_tirage_ns(a0, inner, z0, toward_plus, **kw)
    return body + anti_tirage_ew(a0, inner, z0, toward_plus, **kw)


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
        * Box(largeur, y_sz, at_out + margin, align=AMIN)
    )
    if toward_plus_y:
        body = body - (
            Pos(x0 + at_leg, y_inner, at_u0)
            * Box(largeur, y_jeu, hauteur_u, align=AMIN)
        )
    else:
        body = body - (
            Pos(x0 + at_leg, y_inner_gap, at_u0)
            * Box(largeur, y_jeu, hauteur_u, align=AMIN)
        )
    body = body - (
        Pos(x0 + at_leg, y_cut, at_u1)
        * Box(largeur, y_sz, at_out + margin, align=AMIN)
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
        * Box(x_sz, largeur, at_out + margin, align=AMIN)
    )
    body = body - (
        Pos(x_gap, y0 + at_leg, at_u0)
        * Box(jeu, largeur, hauteur_u, align=AMIN)
    )
    body = body - (
        Pos(x_cut, y0 + at_leg, at_u1)
        * Box(x_sz, largeur, at_out + margin, align=AMIN)
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
    core = Cylinder(r_min, height + 0.2, align=CMIN)
    # Fuse the helix onto the core FIRST. `collar.fuse(core).fuse(crest)`
    # silently returns the crest alone (measured: 94mm3 instead of 1179) —
    # OCCT loses the operands when a coaxial cylinder pair meets a swept
    # helix. Core + crest, then the collar, is stable.
    body = _fuse_one(core + crest)
    if collar_h > 0.0:
        body = _fuse_one(body + Cylinder(r_maj, collar_h, align=CMIN))
    trim = Pos(0, 0, height) * Box(50, 50, pitch + 4.0, align=CMIN)
    body = _fuse_one(body - trim)
    if chanfrein_tete > 0.0:
        # Bolt-tip chamfer. Without it the trim plane knifes the last turn
        # mid-tooth and leaves a 0.37 mm section (min_wall) at the tip.
        z0 = height - chanfrein_tete
        band = Pos(0, 0, z0) * Cylinder(r_maj + 1.0, chanfrein_tete, align=CMIN)
        cone = Pos(0, 0, z0) * Cone(
            r_min + chanfrein_tete, r_min, chanfrein_tete, align=CMIN
        )
        body = _fuse_one(body - (band - cone))
    return body


def passe_cable_body(
    diametre_bride,
    epaisseur_bride,
    diametre_fut,
    diametre_passage,
    entraxe_passages,
    pas_filet,
    profondeur_filet,
    epaisseur_a_traverser,
    draft,
):
    """The full passe_cable solid, shared by `passe_cable` and the split
    `passe_cable_demi_a` / `passe_cable_demi_b` halves, which cut this same
    body along the Y=0 plane (the plane through both cable-bore axes)."""
    ep_ecrou = measured("ecrou_m16_epaisseur")
    amorce = measured("passe_cable_amorce")
    cable = measured("cable_od_passe")
    trou = measured("passe_cable_trou")

    if diametre_bride < diametre_fut + 2.4:
        reject(
            f"diametre_bride {diametre_bride} leaves under 1.2 mm of flange "
            f"around the Ø{diametre_fut} barrel: raise it",
            param="diametre_bride",
        )
    if epaisseur_bride < 1.2:
        reject(
            f"epaisseur_bride {epaisseur_bride} is under 1.2 mm: raise it",
            param="epaisseur_bride",
        )
    if diametre_fut > trou - 0.2:
        reject(
            f"diametre_fut {diametre_fut} will not pass the Ø{trou} hole "
            f"with 0.2 mm snug: lower it",
            param="diametre_fut",
        )
    if diametre_passage < cable + 0.2:
        reject(
            f"diametre_passage {diametre_passage} is tighter than a {cable} mm "
            f"cable plus 0.2 mm snug: raise it",
            param="diametre_passage",
        )
    web = entraxe_passages - diametre_passage
    if web < 1.2:
        reject(
            f"entraxe_passages {entraxe_passages} leaves only {web:.2f} mm "
            f"between the bores: raise it",
            param="entraxe_passages",
        )
    bore_extent = entraxe_passages / 2.0 + diametre_passage / 2.0
    if diametre_fut / 2.0 - profondeur_filet - bore_extent < 1.0:
        reject(
            f"cable bores leave under 1 mm to the thread root: lower "
            f"entraxe_passages or profondeur_filet",
            param="entraxe_passages",
        )
    if pas_filet < profondeur_filet + 0.3:
        reject(
            f"pas_filet {pas_filet} is too short for depth "
            f"{profondeur_filet}: raise it",
            param="pas_filet",
        )

    h_col = epaisseur_a_traverser
    h_filet = ep_ecrou + amorce

    body = Cylinder(diametre_bride / 2.0, epaisseur_bride, align=CMIN)
    body = body + Pos(0, 0, epaisseur_bride) * Cylinder(
        diametre_fut / 2.0, h_col, align=CMIN
    )
    filet = barreau_filete(
        diametre_fut,
        pas_filet,
        profondeur_filet,
        h_filet,
        chanfrein_tete=profondeur_filet + 0.3,
    )
    body = _fuse_one(
        body + Pos(0, 0, epaisseur_bride + h_col - 0.2) * filet
    )

    # Flange rim chamfer BEFORE the cable bores: chamfering the rim on a body
    # that already carries the two through bores makes OCCT rebuild the solid
    # without them (measured: 2124mm3 polished against 1709mm3 draft, the two
    # Ø5.2 bores silently healed shut). Bores last is the same geometry and
    # survives.
    if not draft:

        def keep(edge):
            ebb = edge.bounding_box()
            if ebb.min.Z < epaisseur_bride - 0.2:
                return False
            if ebb.max.Z > epaisseur_bride + 0.2:
                return False
            span = max(ebb.max.X - ebb.min.X, ebb.max.Y - ebb.min.Y)
            return span > diametre_bride - 1.5

        body = polish(body, body.edges().filter_by(keep), 1.0)

    for sign in (-1.0, 1.0):
        body = body - Pos(sign * entraxe_passages / 2.0, 0, -0.5) * Cylinder(
            diametre_passage / 2.0,
            epaisseur_bride + h_col + h_filet + 1.0,
            align=CMIN,
        )

    return body


def passe_cable_demi(body, garder):
    """Cut a passe_cable body in half along Y=0, the plane through both
    cable-bore axes: `garder` picks which side survives ("a" keeps Y>=0,
    "b" keeps Y<=0). The thread is a single helix, not symmetric under a
    180° turn, so the two halves are genuinely different solids — always
    print and use them as a matched pair, never two of the same one."""
    bb = body.bounding_box()
    big = 2.0 * max(
        bb.max.X - bb.min.X, bb.max.Y - bb.min.Y, bb.max.Z - bb.min.Z
    ) + 20.0
    mid_z = (bb.min.Z + bb.max.Z) / 2.0
    y_align = Align.MAX if garder == "a" else Align.MIN
    cutter = Pos(0, 0, mid_z) * Box(
        big, big, big, align=(Align.CENTER, y_align, Align.CENTER)
    )
    body = body - cutter
    solids = list(body.solids())
    if not solids:
        reject(f"splitting side {garder!r} removed everything: check parameters")
    return max(solids, key=lambda s: s.volume)


def back_face_layout(
    wedge_width=126.1,
    seat_height=5.0,
    backing=77.80,
    tilt=45.0,
    wedge_thickness=19.60,
    channel_fit=0.4,
    rail_width=2.0,
    back_opening_from_left=25.0,
    back_opening_below_top=40.0,
    second_opening_offset_x=20.0,
):
    """screen_base's back face: its own size, and its two openings' centres,
    in screen_base's frame. Shared with screen_assembly, which places an
    obstacle plate (the espresso machine's rear panel) against this same
    face and needs its size and opening centres without recomputing them —
    call with the same keyword values passed to screen_base, or the two
    drift apart."""
    t = math.radians(tilt)
    s, c = math.sin(t), math.cos(t)
    channel_half = wedge_width / 2 + channel_fit / 2
    outer_half = channel_half + rail_width
    width = 2 * outer_half
    seat_y = wedge_thickness * s
    north_height = seat_height + backing * s
    north_y = seat_y + backing * c
    back_opening_x = -outer_half + back_opening_from_left
    back_opening_z = north_height - back_opening_below_top
    second_opening_x = back_opening_x + second_opening_offset_x
    second_opening_z = back_opening_z
    return dict(
        width=width,
        outer_half=outer_half,
        north_y=north_y,
        north_height=north_height,
        back_opening_x=back_opening_x,
        back_opening_z=back_opening_z,
        second_opening_x=second_opening_x,
        second_opening_z=second_opening_z,
    )

"""Shared geometry: printable thread, magnet wells, outer-corner polish."""

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

"""Shared geometry: printable thread, magnet wells, contour offset, cable-tie U."""

import math

def digmesa_layout(centre_x=52.0, centre_y=29.0, hauteur_pieds=5.0,
                   surelevation_berceau=17.0):
    """Interfaces communes support/chapeau : coordonnées machine, mm.

    Valeurs de conception, distinctes des mesures du capteur. Le berceau est
    posé à Z=6 + hauteur_pieds. Sa propre surélévation porte le Digmesa plus
    haut sans modifier le support ; la butée du chapeau porte sur la
    collerette, jamais les fils.
    """
    assise = (hauteur_pieds + 6.0 + surelevation_berceau
              + measured("digmesa_pin_longueur") + 1.0)
    return dict(x=centre_x, y=centre_y, berceau_z=hauteur_pieds + 6.0, assise=assise,
                toit=assise + measured("digmesa_hauteur_connecte") + 3.0,
                retenue=assise + measured("digmesa_corps_hauteur"),
                arriere=centre_y - 24.0, vis_x=(centre_x-19.0, centre_x+19.0),
                vis_z=38)
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
    concave_edges,
    RegularPolygon,
    extrude,
    make_face,
    measured,
    mirror,
    polish,
    reject,
    sweep,
)


# Lead so a bed-flush nut trap cuts the outside face cleanly.
_NUT_LEAD = 1.0
_NUT_SEATED = (Align.CENTER, Align.CENTER, Align.MIN)


def m3_nut_trap(shaft_dia, nut_af, nut_th, shoulder_z, depth, layer=1.0,
                slack=0.2, bridge_roof=True):
    """Captive M3 hex nut from the entry face, with a printable bridged roof.

    The nut enters through the hexagonal mouth and bears at ``shoulder_z``.
    With ``bridge_roof=True`` (the legacy default), two crossed sacrificial
    bridge layers separate it from the clearance shaft. Set it false when the
    support material below the nut carries the transition directly. The hex is
    sized from across-flats plus print slack and rotated so a flat, rather than
    a point, faces the print direction.
    """
    if shaft_dia <= 0 or nut_af <= 0 or nut_th <= 0 or layer <= 0:
        raise ValueError("m3_nut_trap needs positive dimensions")
    if shoulder_z < nut_th:
        reject(
            f"nut shoulder at {shoulder_z:.2f} mm is under the {nut_th} mm nut: "
            "raise the support or shorten the screw",
        )
    bridge_depth = 2 * layer if bridge_roof else 0.0
    if depth < shoulder_z + bridge_depth:
        reject(
            f"depth {depth:.2f} leaves no shaft above the nut seat at "
            f"{shoulder_z:.2f} mm plus {bridge_depth:.2f} mm of bridges",
        )

    af = nut_af + slack
    hex_face = RegularPolygon(af / 2.0, 6, major_radius=False, rotation=30)
    hex_prism = Pos(0, 0, -_NUT_LEAD) * extrude(hex_face, shoulder_z + _NUT_LEAD)
    across_corners = af / math.cos(math.radians(30))
    shaft = Pos(0, 0, -_NUT_LEAD) * Cylinder(
        shaft_dia / 2.0, depth + _NUT_LEAD, align=_NUT_SEATED
    )
    if not bridge_roof:
        return hex_prism + shaft
    bridge_zone = Pos(0, 0, shoulder_z) * Cylinder(
        across_corners / 2.0, 2 * layer, align=_NUT_SEATED
    )
    first = Pos(0, 0, shoulder_z) * Box(
        across_corners, shaft_dia, layer, align=_NUT_SEATED
    ) & bridge_zone
    second = Pos(0, 0, shoulder_z + layer) * Box(
        shaft_dia, across_corners, layer, align=_NUT_SEATED
    ) & bridge_zone
    return hex_prism + first + second + shaft

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


def puit_couche(rayon, profondeur, pont=2.0, debord=0.1):
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

    La bouche est nette, à la cote sur toute la profondeur, pour que l'aimant
    porte sur tout le fût. Le cutter dépasse de `debord` en arrière de la face
    pour la couper proprement.
    """
    if pont <= 0.0:
        reject(f"pont {pont} doit être positif")
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

    # `debord` prolonge dehors pour couper la face proprement.
    cutter = extrude(section, profondeur)
    dehors = cutter.faces().filter_by(Plane.XY).sort_by(Axis.Z)[0]
    return _fuse_one(cutter + extrude(dehors, debord))


# Heat-insert bores. `diametre_vis` is the clearance hole (screw + 0.2 mm) to
# drill in the part facing the insert. `encombrement` is the outer diameter of the housing
# (perçage + 2x l'épaisseur de paroi mini), for a circular pad or annulus
# around the bore; a part using a square/rectangular pad instead can still
# size it off `encombrement` as the footprint's side.
HeatInsert = namedtuple(
    "HeatInsert",
    "diametre_percage diametre_vis epaisseur_paroi_min profondeur_min encombrement",
)

INSERT_M3 = HeatInsert(
    diametre_percage=4.0,
    diametre_vis=3.2,
    epaisseur_paroi_min=1.6,
    profondeur_min=5.0,
    encombrement=7.2,
)
INSERT_M25 = HeatInsert(
    diametre_percage=3.9,
    diametre_vis=2.7,
    epaisseur_paroi_min=1.5,
    profondeur_min=4.5,
    encombrement=6.9,
)
INSERT_M2 = HeatInsert(
    diametre_percage=3.2,
    diametre_vis=2.2,
    epaisseur_paroi_min=1.2,
    profondeur_min=3.6,  # user 2026-10-29: insertion length of the M2 inserts in stock
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
    if outer is not None:
        clipped = ring.intersect(outer)
        if clipped is not None:
            body = body + clipped
    else:
        body += ring
    return body - inner_void

def add_heat_insert_against(body, outer, cx, cy, toward, z0, height, insert_def):
    """`add_heat_insert` by a wall or in a corner, with the pad filled out to
    `toward`, a point on the wall face or the corner: a round pad a hair off a
    wall leaves a crevice too thin to print. Along a single wall the fill is
    as wide as the pad."""
    body = add_heat_insert(body, outer, cx, cy, z0, height, insert_def)
    half = insert_def.encombrement / 2

    def span(c, t):
        lo, hi = sorted((c, t))
        return (c - half, c + half) if hi - lo < half else (lo, hi)

    (x0, x1), (y0, y1) = span(cx, toward[0]), span(cy, toward[1])
    fill = Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, height, align=AMIN)
    bore = Pos(cx, cy, z0) * Cylinder(insert_def.diametre_percage / 2, height, align=CMIN)
    return body + (fill - bore)


def cable_tie_tower(x, centre_y, height, floor):
    """Arch for a cable tie: two 2 x 4 posts 3 mm apart along X, joined by a
    2 mm beam on top. The tie goes through the arch along Y and around the
    beam; the wires lie on the beam along X. `x` is the west edge."""
    post, gap = 2.0, 3.0
    y0 = centre_y - post

    def block(x0, dx, z0, dz):
        return Pos(x0, y0, z0) * Box(dx, 2 * post, dz, align=AMIN)

    return (block(x, post, floor, height)
            + block(x + post + gap, post, floor, height)
            + block(x, 2 * post + gap, floor + height - post, post))


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
    if outer is not None:
        clipped = pad.intersect(outer)
        if clipped is not None:
            body = body + clipped
    else:
        body += pad
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
        plat_y = y + corbel_along
        hole_x = x + (corbel_r if flush else corbel_plat/2) * multiplier
        hole_y = y + corbel_paroi + corbel_r
    else:
        plat_x = x
        plat_y = y
        hole_x = x + (corbel_paroi + corbel_r)
        hole_y = y + (corbel_r if flush else corbel_plat/2)*multiplier
    body = body + (
        Pos(plat_x, plat_y, 0)
        * extrude(plane * Polygon(*corbel_pts, align=None), corbel_along * multiplier)
    )
    body = body - (
        Pos(hole_x, hole_y, z_corbel)
        * Cylinder(corbel_r, corbel_profondeur + 0.1, align=CMIN)
    )
    return body

CorbelSite = namedtuple("CorbelSite", "x y plane reverse insert hole")


def corbel_site(side, face, start, insert=INSERT_M3):
    """A lid corbel hanging from the inner face of one wall.

    side: the wall it hangs from, "west", "east", "south" or "north".
    face: X (west/east) or Y (south/north) of that wall's inner face.
    start: low end of the corbel along the wall.
    `hole` is (cx, cy, screw diameter, pad bbox), what `couvercle_corps` takes.
    """
    plat = insert.diametre_percage + insert.epaisseur_paroi_min
    along = insert.diametre_percage + 2 * insert.epaisseur_paroi_min
    if side in ("west", "east"):
        reverse = side == "east"
        pad = bbox(face - plat if reverse else face, start, plat, along)
        plane, x, y = Plane.XZ, face, start
    elif side in ("south", "north"):
        reverse = side == "north"
        pad = bbox(start, face - plat if reverse else face, along, plat)
        plane, x, y = Plane.YZ, start, face
    else:
        raise ValueError(f"corbel side {side!r}: west, east, south or north")
    c = pad.center()
    return CorbelSite(x, y, plane, reverse, insert, (c.X, c.Y, insert.diametre_vis, pad))


def add_corbel_site(body, site, hauteur):
    return add_corbel(body, site.x, site.y, hauteur, insert=site.insert,
                      plane=site.plane, reverse=site.reverse)


# Gap between the board edge and the inner face of the box walls, all round:
# enough for the board to slide in by hand.
JEU_CARTE = 1.0


def bord_carte(wall):
    """Distance from the outer face of a box to the board edge: the wall
    plus the slide gap. The board's south-west corner sits at (bord, bord)."""
    return wall + JEU_CARTE


def carte_rect(name, wall):
    """A `combined_*` rectangle [x0, y0, x1, y1], from the board frame to the box frame."""
    x0, y0, x1, y1 = measured(name)
    m = bord_carte(wall)
    return (x0 + m, y0 + m, x1 + m, y1 + m)


# Side of each HT396V header where its HT396K plug's latch sticks out: the
# side without the rounded contact tubes, from the 3D model and the footprint
# rotation on combined.kicad_pcb (confirmed by David). J7 is turned 180° on
# the board so its latch, like J8's, comes out through the south wall.
VERROUS_HT396 = {"combined_j6": "west", "combined_j7": "south", "combined_j8": "south"}


def carte_plug_rect(name, wall):
    """Footprint of the HT396K plug on header `name`, box frame: the header
    grown on its latch side by the plug's extra depth."""
    x0, y0, x1, y1 = carte_rect(name, wall)
    extra = measured("ht396k_profondeur") - measured("ht396v_profondeur")
    side = VERROUS_HT396[name]
    if side == "west":
        x0 -= extra
    elif side == "east":
        x1 += extra
    elif side == "south":
        y0 -= extra
    else:
        y1 += extra
    return (x0, y0, x1, y1)


def carte_trous(wall):
    """Centres of the board's M2 holes H1 to H6, in the box frame."""
    m = bord_carte(wall)
    return [(x + m, y + m) for x, y in measured("combined_trous")]


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
    jeu=1.4,
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
    jeu=1.4,
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
    jeu=1.5,
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
    back_opening_below_top=40.72,
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
    # The flat top over the wedge's own thickness, flush with the machine's top.
    top_height = north_height + wedge_thickness * c
    back_opening_x = -outer_half + back_opening_from_left
    back_opening_z = top_height - back_opening_below_top
    second_opening_x = back_opening_x + second_opening_offset_x
    second_opening_z = back_opening_z
    return dict(
        width=width,
        outer_half=outer_half,
        north_y=north_y,
        north_height=north_height,
        top_height=top_height,
        back_opening_x=back_opening_x,
        back_opening_z=back_opening_z,
        second_opening_x=second_opening_x,
        second_opening_z=second_opening_z,
    )


def couvercle_corps(pts, corbels, wall, floor, jeu, rim_width, rim_depth, openings=(), extra_rims=(),
                    rim_gaps=()):
    """Flat lid for a box outline: plate, inner rim, screw holes, mirrored in X.

    The rim sits `wall + jeu` inside `pts` and is cut `corbel_jeu` around each
    corbel pad. `openings` are (x0, y0, dx, dy) cut through plate and rim;
    `extra_rims` are (x0, y0, x1, y1) ribs as deep as the rim; `rim_gaps` are
    (x0, y0, x1, y1) cut through the rim only, where it would hit a part
    inside the box. The result is
    mirrored about the outline's X centre so that flipping the lid about a
    north-south axis lands it on the box.
    """
    corbel_jeu = 1.3
    plate = extrude(Polygon(*pts, align=None), floor)
    outer = offset_in(pts, wall + jeu)
    inner = offset_in(pts, wall + jeu + rim_width)
    rim = Pos(0, 0, floor) * (
        extrude(Polygon(*outer, align=None), rim_depth)
        - extrude(Polygon(*inner, align=None), rim_depth)
    )
    body = plate + rim

    for cx, cy, diameter, pad in corbels:
        body = body - Pos(cx, cy, 0) * Cylinder(diameter / 2, floor, align=CMIN)
        body = body - Pos(
            pad.min.X - corbel_jeu, pad.min.Y - corbel_jeu, floor
        ) * Box(
            pad.size.X + 2 * corbel_jeu,
            pad.size.Y + 2 * corbel_jeu,
            rim_depth,
            align=AMIN,
        )
    for o_x0,o_y0,o_w,o_h in openings:
        body -= Pos(o_x0,o_y0,0)*Box(o_w,o_h,floor+rim_depth,align=AMIN)
    for x0, y0, x1, y1 in extra_rims:
        body += Pos(x0, y0, floor) * Box(x1 - x0, y1 - y0, rim_depth, align=AMIN)
    for x0, y0, x1, y1 in rim_gaps:
        body -= Pos(x0, y0, floor) * Box(x1 - x0, y1 - y0, rim_depth, align=AMIN)

    xs = [x for x, _ in pts]
    mid_x = (min(xs) + max(xs)) / 2.0
    mirror_x = Plane(origin=(mid_x, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
    return mirror(body, about=mirror_x)


def polish_couvercle(body, wall, rim_depth):
    """Chamfer the vertical edges of a lid, keeping those at the rim tip sharp
    for fit, and the inside corners of openings, where a chamfer only fills."""
    bed = body.bounding_box().min.Z

    def keep(edge):
        bb = edge.bounding_box()
        return bb.min.Z <= bed + 0.05 or bb.min.Z >= wall + rim_depth - 0.05

    concave = concave_edges(body)
    edges = [e for e in body.edges().filter_by(Axis.Z).filter_by(keep)
             if not any(e.is_same(c) for c in concave)]
    return polish(body, edges, 0.6)


# --- Balance du drip tray : repère et niveaux partagés par les pièces scale_* ---
#
# Toutes les pièces scale_* sont modélisées dans le repère machine : X depuis la
# face intérieure de la paroi gauche de la tôle (vue par l'utilisateur), Y depuis
# la face de la machine vers l'utilisateur (la bouche est en Y négatif), Z depuis
# le dessus du fond de la tôle. Ce repère est gauche : une pièce dessinée telle
# quelle sortirait en miroir. Chaque pièce se termine donc par `scale_frame`,
# qui inverse Y : dans le viewer, la machine est au nord (Y positif),
# l'utilisateur au sud et la paroi gauche à l'ouest. Une pièce qui s'imprime
# retournée passe ensuite par `scale_flip`, et l'assemblage applique
# `scale_flip` une seconde fois pour la remettre en place.

from types import SimpleNamespace

from nurb import Rectangle, Rot, ThreePointArc, loft


def _scale_plate_holes(axis):
    """Centres of the 19 holes of the base plate: centre, 6 on r = 20 from
    -45°, 12 on r = 35 every 30°. Angles turn from +X toward the machine."""
    holes = [axis]
    for radius, count, first in ((20.0, 6, -45.0), (35.0, 12, 0.0)):
        for k in range(count):
            angle = math.radians(first + k * 360.0 / count)
            holes.append((axis[0] + radius * math.cos(angle),
                          axis[1] - radius * math.sin(angle)))
    return holes


def scale_layout():
    """Positions and levels shared by the scale_* parts, machine frame, in mm.

    Measured values come from measurements.toml; the rest are the design
    levels of docs/tray-with-scale.md, « Design decisions ». A part reads what it mates
    with from here, so two parts cannot drift apart."""
    plate_width = measured("scale_tole_largeur")
    axis = tuple(measured("scale_tole_trou_centre"))
    section = measured("scale_cellule_section")
    cell_holes = measured("scale_cellule_trous")
    cell_length = measured("scale_cellule_longueur")
    centre_x = plate_width / 2.0
    holes = _scale_plate_holes(axis)

    # Cell along X, fixed end toward +X, tower axis midway between the two
    # holes of the loaded end.
    cell_fixed_tip = axis[0] + (cell_holes[2] + cell_holes[3]) / 2.0
    # The cell sits 1 mm above the base's floor, on the pad under its fixed end.
    floor_top = 2.0
    cell_bottom = floor_top + 1.0
    cell = SimpleNamespace(
        x0=cell_fixed_tip - cell_length, x1=cell_fixed_tip,
        y0=axis[1] - section / 2.0, y1=axis[1] + section / 2.0,
        z0=cell_bottom, z1=cell_bottom + section,
        section=section,
        stop_x=cell_fixed_tip - cell_holes[3],    # set screw, tip hole
        clamp_x=cell_fixed_tip - cell_holes[2],   # adapter screw
        fixed_xs=(cell_fixed_tip - cell_holes[1], cell_fixed_tip - cell_holes[0]),
    )

    base = SimpleNamespace(
        x0=84.0, x1=177.0, y0=6.0, y1=73.0,
        floor_top=floor_top,
        pad_x0=cell.x1 - 20.0,            # raised pad under the fixed end
        # M3 x 10 from under the plate, through a scale_washer whose collar
        # centres it in the plate's Ø8 hole, into a 4 mm nylon-insert locknut
        # half sunk in the floor. None is under the cell: nut and screw end
        # stand 2.5 mm above the floor, and the cell is 1 mm over it.
        screw_holes=(holes[3], holes[8], holes[1]),   # (102.18, 34.68), (127.31, 36.5), (111.14, 68.14)
        nut_pocket_depth=2.0,
        # The floor is too thin for the nut's pocket: a boss around each nut
        # keeps 2 mm of plastic under it.
        nut_boss_top=floor_top + 2.0, nut_boss_diameter=10.0,
        washer_collar_diameter=7.7,
        washer_collar_depth=1.8,
        bridge_x0=116.0, bridge_x1=119.0, bridge_gap=0.6, bridge_thickness=3.0,
        # Cable saddle east of the PCB box (east face X 163.4, lid skirt
        # 164.8): the bus cable comes off it straight south along the box's
        # east face to the J1 notch, instead of into the box's north wall.
        saddle_x0=166.0, saddle_x1=174.0, saddle_y0=0.5, saddle_y1=6.5,
        saddle_top=14.0, saddle_notch_depth=1.0,
        # Cable tie window through the saddle along X, under the groove.
        saddle_tie_width=3.0, saddle_tie_height=1.5, saddle_tie_bottom=10.0,
        # Two cable tie towers for the cell's wires, in the corridor between
        # the PCB box (Y 32.5) and the cell (Y 47.65), clear of the nuts at
        # X 98.9..105.5 and 124..130.6. The west one ends at the box's west
        # face (114.9), where the wires turn north to the J2 notch.
        # West edge of each tower.
        tie_tower_xs=(107.5, 146.0), tie_tower_y=40.0, tie_tower_height=6.0,
    )

    # PCB box, long axis along X: J2 (cell) toward -X, J1 (bus) toward +X,
    # on the side where the bus cable comes in. Screw holes along -Y (the
    # machine side), rest ledge along +Y.
    pcb_width, pcb_length = measured("scale_pcb")
    box_wall, wire_room, pcb_play = 1.6, 10.0, 0.3
    inner_x0, inner_y0 = 116.5, 11.0
    inner_x1 = inner_x0 + pcb_length + 2 * wire_room + 2 * pcb_play
    inner_y1 = inner_y0 + pcb_width + 2 * pcb_play
    pcb_x0, pcb_y0 = inner_x0 + wire_room + pcb_play, inner_y0 + pcb_play
    box = SimpleNamespace(
        wall=box_wall,
        inner_x0=inner_x0, inner_x1=inner_x1, inner_y0=inner_y0, inner_y1=inner_y1,
        floor_top=base.floor_top,
        pcb_x0=pcb_x0, pcb_x1=pcb_x0 + pcb_length, pcb_y0=pcb_y0, pcb_y1=pcb_y0 + pcb_width,
        pcb_underside=base.floor_top + 4.0,
        pcb_holes=((pcb_x0 + 2.5, pcb_y0 + 2.5), (pcb_x0 + pcb_length - 2.5, pcb_y0 + 2.5)),
        wall_top=base.floor_top + 15.0,
        notch_y=(inner_y0 + inner_y1) / 2.0, notch_width=5.0, notch_bottom=base.floor_top + 7.0,
        lid_play=0.2, lid_skirt=1.2, lid_plate=1.6, lid_skirt_bottom=base.floor_top + 0.3,
    )

    seat = SimpleNamespace(base=24.0, height=3.0, flank_degrees=30.0, socket_clearance=0.5)

    adapter = SimpleNamespace(
        radius=18.0, flat_x=109.5,
        bottom=cell.z1 - 5.0, seat_z=cell.z1 + 2.5, top=cell.z1 + 2.5 + seat.height,
        weak_magnets=((axis[0], axis[1] - 6.4), (axis[0], axis[1] + 6.4)),
        weak_magnet_gap=1.0,
    )

    # The cup grille is a rectangle, longer along Y like the chimney's lens, so
    # it only drops into its cut-out the right way round or turned half a turn,
    # and both are the same grille. Turned a quarter, its 94 meets a 90 opening.
    grille_inner = SimpleNamespace(width=86.0, length=94.0, top=50.0, thickness=2.5,
                                   # Radial gap between the chimney's outer face and the ring.
                                   drip_ring_clearance=2.5, drip_ring_wall=1.6,
                                   drip_ring_bottom=43.0,
                                   finger_hole_diameter=16.0, finger_hole_border=4.0)
    cutout_gap = 2.0
    grille_outer = SimpleNamespace(top=48.5, thickness=2.5,
                                   cutout_width=grille_inner.width + 2.0 * cutout_gap,
                                   cutout_length=grille_inner.length + 2.0 * cutout_gap,
                                   purge_diameter=16.0, purge_y=-8.0,
                                   pull_hole=20.0, apron_bottom=20.0, play=0.3)
    grille_inner_underside = grille_inner.top - grille_inner.thickness

    tower = SimpleNamespace(
        radius=18.0,
        bottom=adapter.seat_z + seat.socket_clearance,
        top=grille_inner_underside - 0.5,
        top_seat_z=grille_inner_underside,   # base plane of the grille's seat
        strong_magnets=((axis[0] - 5.0, axis[1]), (axis[0] + 5.0, axis[1])),
        strong_magnet_gap=0.4,
    )

    tray_wall = 2.5
    tray = SimpleNamespace(
        wall=tray_wall, floor=2.5,
        x0=centre_x - 87.0, x1=centre_x + 87.0,
        y_rear=-23.0, y_front=139.5,
        raised_underside=25.0, well_underside=2.0,
        # The tray prints standing on its front wall, so its floor only changes
        # level along ramps. Plateau over the scale: flat back from plateau_y,
        # then 45 degrees down to the well. West of plateau_x0 nothing stands
        # under the tray: the floor climbs gently from the well to the mouth.
        plateau_x0=72.0, plateau_y=89.0,
        slope_y_front=112.0, slope_y_rear=4.0, mouth_underside=13.5,
        rim_top=grille_outer.top,
        rail_top=grille_outer.top - grille_outer.thickness, rail_width=3.0,
        window_diameter=42.0, window_tip_radius=2.0, kerb_wall=2.0, kerb_top=35.5,
        # Web under the kerb's +Y tip, which the chimney's sleeve straddles.
        kerb_web_width=10.0,
        water_max=43.5,
        shoulder_underside=measured("scale_tole_paroi_hauteur"), shoulder_thickness=2.0,
        shoulder_x0=-measured("scale_tole_epaisseur") - 2.5,
        shoulder_x1=plate_width + measured("scale_tole_epaisseur") + 2.5,
        shoulder_y0=2.0,
        side_play=0.5,
        # Two pegs stand out of the sloping rear wall, along -Y. The west one
        # meets the machine's rubber bumper; the east one carries the TPU cap
        # that replaces the bumper removed for the cable.
        bumper_west=(centre_x - measured("scale_taquet_x"), measured("scale_taquet_z")),
        bumper=(centre_x + measured("scale_taquet_x") + 14.0, measured("scale_taquet_z")),
        bumper_boss_diameter=10.0, bumper_boss_length=6.5, bumper_cap=1.5,
    )

    chimney = SimpleNamespace(top=45.0, wall=1.6,
                              bottom=tray.raised_underside + tray.floor)

    return SimpleNamespace(
        plate_width=plate_width, centre_x=centre_x, axis=axis, holes=holes,
        plate_depth=measured("scale_tole_profondeur"),
        plate_thickness=measured("scale_tole_epaisseur"),
        hole_diameter=measured("scale_tole_trou_diametre"),
        cell=cell, base=base, box=box, seat=seat, adapter=adapter, tower=tower,
        tray=tray, chimney=chimney, grille_inner=grille_inner, grille_outer=grille_outer,
    )


def scale_seat(z_base, height, up=True):
    """Square truncated pyramid of the tower joints, centred on the tower axis.

    Its base (seat.base wide) lies at `z_base` and it tapers along +Z (`up`)
    or -Z. A boss uses seat.height; the socket that receives it uses the same
    base plane and seat.height + seat.socket_clearance, so the flanks coincide
    and the socket's end stays clear of the boss's end."""
    layout = scale_layout()
    seat = layout.seat
    narrow = seat.base - 2.0 * height * math.tan(math.radians(seat.flank_degrees))
    z_end = z_base + height if up else z_base - height
    sections = [
        Pos(*layout.axis, z_base) * Rectangle(seat.base, seat.base),
        Pos(*layout.axis, z_end) * Rectangle(narrow, narrow),
    ]
    return loft(sections if up else sections[::-1], ruled=True)


def scale_lens(radius, tip_radius):
    """Outline of the tray window, its kerb and the chimney: a circle on the
    tower axis drawn out to a rounded point toward +Y and toward -Y, with
    flanks at 45 degrees. On a tray printed standing on its front wall, the
    +Y point lets the kerb start on the floor instead of in the air, and the
    -Y point roofs the window. Offsetting both radii by the same amount gives
    a parallel outline. Returns a face in the XY plane, machine frame."""
    axis = scale_layout().axis
    reach = (radius - tip_radius) * math.sqrt(2.0)

    def on_circle(offset_y, circle_radius, degrees):
        angle = math.radians(degrees)
        return (axis[0] + circle_radius * math.cos(angle),
                axis[1] + offset_y + circle_radius * math.sin(angle))

    def arc(offset_y, circle_radius, start):
        return ThreePointArc(*(on_circle(offset_y, circle_radius, start + step)
                               for step in (0.0, 45.0, 90.0)))

    # Counter-clockwise from the east flank: body arc, +Y tip, body arc, -Y tip.
    arcs = [arc(0.0, radius, -45.0), arc(reach, tip_radius, 45.0),
            arc(0.0, radius, 135.0), arc(-reach, tip_radius, 225.0)]
    flanks = [Line(arcs[k] @ 1, arcs[(k + 1) % 4] @ 0) for k in range(4)]
    return make_face([edge for pair in zip(arcs, flanks) for edge in pair])


def scale_frame(solid):
    """Last step of every scale_* part: the machine frame is left-handed, so
    reversing Y turns the model into the real, right-handed object."""
    return mirror(solid, Plane.XZ)


def scale_flip(solid):
    """Turn a part over about the machine X axis, for one that prints top face
    down. Applying it twice gives the part back, which is how the assembly
    puts a flipped part in place."""
    return Rot(180, 0, 0) * solid

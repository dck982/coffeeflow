from nurb import *

from system import (
    AMIN,
    INSERT_M2,
    _fuse_one,
    add_corbel_site,
    add_heat_insert_against,
    add_well,
    bbox,
    bord_carte,
    carte_rect,
    carte_trous,
    corbel_site,
    offset_in,
)

# Tongues of the floor that key boitier_ac into the north muret of boitier_dc:
# width, and centres as a fraction of the opening onto boitier_dc.
LANGUETTE_LARGEUR = 6.0
_LANGUETTE_FRACTIONS = (0.25, 0.75)
# Magnet wells, board frame: one on the low-voltage west side; the other is
# placed in the PCB notch, east of the screw housing, under no copper at all.
_PUIT_OUEST = (35.0, 78.0)
# Gap between a connector and its opening in the east wall.
_JEU_OUVERTURE = 0.5


def bb_overall(wall):
    """Upper band of the board plus the slide gap and one wall all round, box frame."""
    x0 = measured("combined_ac_x")
    y0 = measured("combined_ac_y")
    m = bord_carte(wall)
    return bbox(
        x0, y0,
        measured("combined_largeur") + 2 * m - x0,
        measured("combined_ac_profondeur") + 2 * m,
    )


def bb_encoche(wall):
    """Inside of the screw housing: its west wall stands the slide gap east of
    the PCB notch edge."""
    cote = measured("logement_vis_cote")
    north = bb_overall(wall).max.Y
    return bbox(
        measured("combined_encoche_x") + 2 * bord_carte(wall), north - cote, cote, cote,
        h=measured("logement_vis_hauteur"),
    )


def ouverture_dc(wall):
    """X range of the south side left open onto boitier_dc: its inside."""
    return measured("combined_ac_x") + wall, measured("combined_dc_largeur") + 2 * bord_carte(wall) - wall


def languettes(wall):
    """(x0, x1) of the floor tongues that drop into the boitier_dc muret."""
    x0, x1 = ouverture_dc(wall)
    return [
        (x0 + f * (x1 - x0) - LANGUETTE_LARGEUR / 2, x0 + f * (x1 - x0) + LANGUETTE_LARGEUR / 2)
        for f in _LANGUETTE_FRACTIONS
    ]


def ac_contour(wall):
    """Outline at the floor, with the screw housing cut in from the north."""
    bb = bb_overall(wall)
    screw = bb_encoche(wall)
    return [
        (bb.min.X, bb.min.Y),
        (bb.max.X, bb.min.Y),
        (bb.max.X, bb.max.Y),
        (screw.max.X, bb.max.Y),
        (screw.max.X, screw.min.Y),
        (screw.min.X, screw.min.Y),
        (screw.min.X, bb.max.Y),
        (bb.min.X, bb.max.Y),
    ]


def corbel_sites(wall):
    """M2 lid corbels, each beside a mounting hole so the board's M2 screw
    stays reachable, plus one on the east wall in the PCB notch. The board
    slides in tilted, north edge first, under the two north ones; the south
    one fits a notch in the board edge (`combined_entailles`)."""
    bb = bb_overall(wall)
    m = bord_carte(wall)
    return [
        # H3, north wall, east of the hole
        corbel_site("north", bb.max.Y - wall, 29.6 + m, INSERT_M2),
        # H2, north wall, west of the hole
        corbel_site("north", bb.max.Y - wall, 88.4 + m, INSERT_M2),
        # H6, south wall, east of the hole
        corbel_site("south", bb.min.Y + wall, 68.5 + m, INSERT_M2),
        # east wall, north of J3, over the notch
        corbel_site("east", bb.max.X - wall, 77.0 + m, INSERT_M2),
    ]


def corbels(wall):
    """Hole (cx, cy, d, pad bbox) of each corbel, for the lid."""
    return [site.hole for site in corbel_sites(wall)]


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


def _build_encoche(body, encoche, north_y, hauteur, z0, w):
    """Roof the screw housing with a slab, and close the north wall above it."""
    inner_west_x = encoche.min.X - w
    inner_east_x = encoche.max.X + w
    inner_south_y = encoche.min.Y - w
    slab_bottom_z = encoche.max.Z
    slab_top_z = slab_bottom_z + z0
    # The housing walls stop at the slab: above it is the box's cavity.
    body = body - (
        Pos(inner_west_x, inner_south_y, slab_top_z)
        * Box(inner_east_x - inner_west_x, north_y - inner_south_y, hauteur - slab_top_z, align=AMIN)
    )
    body = body + (
        Pos(inner_west_x, inner_south_y, slab_bottom_z)
        * Box(inner_east_x - inner_west_x, north_y - inner_south_y + w, z0, align=AMIN)
    )
    body = body + (
        Pos(inner_west_x, north_y, slab_top_z)
        * Box(inner_east_x - inner_west_x, w, hauteur - slab_top_z, align=AMIN)
    )
    # Goussets shorten the slab's overhang.
    run = 2.0
    body = body + _gousset_x(encoche.min.X, inner_south_y, north_y + w, slab_bottom_z, run, True)
    body = body + _gousset_x(encoche.max.X, inner_south_y, north_y + w, slab_bottom_z, run, False)
    body = body + _gousset_y(encoche.min.Y, inner_west_x, inner_east_x, slab_bottom_z, run, True)
    return body


def _puit_encoche(wall):
    """Centre of the well east of the screw housing: its pad bites into the
    north wall, so no slot is left between them."""
    bb = bb_overall(wall)
    pad_radius = (measured("aimant_diametre") + measured("aimant_puit_press_fit")) / 2 \
        + measured("aimant_puit_mur")
    return (
        (bb_encoche(wall).max.X + wall + bb.max.X - wall) / 2,
        bb.max.Y - wall - pad_radius + 0.5,
    )


def _inserts(body, outer, wall, z0):
    """M2 inserts under H2, H3 and H6, filled out to their wall or corner."""
    bb = bb_overall(wall)
    height = measured("combined_dessous_pcb")
    _, h2, h3, _, _, h6 = carte_trous(wall)
    north = bb.max.Y - wall
    body = add_heat_insert_against(
        body, outer, *h2, (bb_encoche(wall).min.X - wall, north), z0, height, INSERT_M2)
    body = add_heat_insert_against(
        body, outer, *h3, (bb.min.X + wall, north), z0, height, INSERT_M2)
    return add_heat_insert_against(body, outer, *h6, (h6[0], bb.min.Y + wall), z0, height, INSERT_M2)


def east_openings(wall, z0, hauteur):
    """J2 and J3 slots in the east wall, (y0, y1, z0, z1): from the PCB top to the rim."""
    pcb_top = z0 + measured("combined_dessous_pcb") + measured("combined_epaisseur")
    slots = []
    for name in ("combined_j2", "combined_j3"):
        _, y0, _, y1 = carte_rect(name, wall)
        slots.append((y0 - _JEU_OUVERTURE, y1 + _JEU_OUVERTURE, pcb_top, hauteur))
    return slots


@part
def boitier_ac(
    hauteur=26.0,
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    draft=False,
):
    """Boîtier AC : bande nord de la carte combined, posé sur la plaque métal.

    hauteur: hauteur des murs depuis le lit
    epaisseur_paroi: épaisseur des murs, vers l'intérieur
    epaisseur_fond: épaisseur du fond
    """
    wall = epaisseur_paroi
    z0 = epaisseur_fond

    if wall < 1.2:
        reject(f"epaisseur_paroi {wall} is under 1.2 mm: raise it", param="epaisseur_paroi")
    if measured("combined_dessous_pcb") < INSERT_M2.profondeur_min:
        reject(
            f"combined_dessous_pcb leaves under the {INSERT_M2.profondeur_min} mm "
            "an M2 insert needs under the PCB: raise it",
        )
    xiao_top = z0 + measured("combined_dessous_pcb") + measured("combined_hauteur_xiao")
    if hauteur < xiao_top:
        reject(
            f"hauteur {hauteur} puts the lid into the XIAO, whose top is at "
            f"{xiao_top:.1f} mm: raise it",
            param="hauteur",
        )

    bb = bb_overall(wall)
    encoche = bb_encoche(wall)
    pts = ac_contour(wall)

    outer = extrude(Polygon(*pts, align=None), hauteur)
    cavity = Pos(0, 0, z0) * extrude(Polygon(*offset_in(pts, wall), align=None), hauteur)
    body = outer - cavity
    body = _build_encoche(body, encoche, bb.max.Y - wall, hauteur, z0, wall)

    # South side open onto boitier_dc above the floor.
    x0, x1 = ouverture_dc(wall)
    body = body - Pos(x0, bb.min.Y, z0) * Box(x1 - x0, wall, hauteur - z0, align=AMIN)
    # Floor tongues keyed into the boitier_dc muret.
    for tx0, tx1 in languettes(wall):
        body = body + Pos(tx0, bb.min.Y - wall, 0) * Box(tx1 - tx0, wall, z0, align=AMIN)

    # J2 / J3 plugs come in from the east.
    for y0, y1, sz0, sz1 in east_openings(wall, z0, hauteur):
        body = body - Pos(bb.max.X - wall, y0, sz0) * Box(wall, y1 - y0, sz1 - sz0, align=AMIN)

    body = _inserts(body, outer, wall, z0)

    # Magnet wells.
    body = add_well(body, outer, _PUIT_OUEST[0] + bord_carte(wall), _PUIT_OUEST[1] + bord_carte(wall))
    body = add_well(body, outer, *_puit_encoche(wall))

    for site in corbel_sites(wall):
        body = add_corbel_site(body, site, hauteur)

    body = _fuse_one(body)
    if draft:
        return body

    # Outer vertical corners, except the south-west one that meets boitier_dc.
    corners = [(bb.max.X, bb.min.Y), (bb.max.X, bb.max.Y), (bb.min.X, bb.max.Y)]

    def keep(edge):
        c = edge.center()
        return any(abs(c.X - x) < 0.01 and abs(c.Y - y) < 0.01 for x, y in corners)

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

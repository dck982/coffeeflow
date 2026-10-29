from nurb import *

from system import VERROUS_HT396, carte_plug_rect, carte_rect, couvercle_corps, polish_couvercle
from parts import boitier_ac, boitier_dc

# Connectors plugged from above, through the lid. The HT396 openings take
# the plug, wider than its header on the latch side.
_CONNECTEURS = ("combined_j7", "combined_j1", "combined_j8", "combined_j10",
                "combined_j9", "combined_j6")
# J8, J10 and J9 share one rectangular opening out to the south and east
# edges, as wide as J8, the corner connector (first of the group): the lid
# goes on with the plugs in, the flexible XH wires of J10 and J9 bending east.
_REGROUPES = (("combined_j8", "combined_j10", "combined_j9"),)
# J6 sits mid-board: a passage east into the J8-J10-J9 opening lets its cable
# slide out sideways, so the lid goes on with the plug in.
_PASSAGES = (("combined_j6", "combined_j8"),)
# An opening closer than this to the lid edge runs out through it; two
# openings closer than this to each other are joined.
_BORD_MIN = 4.0
_ENTRE_MIN = 1.5


def acdc_contour(wall):
    """Outline of both boxes side by side: boitier_dc, and boitier_ac north of it."""
    ac = boitier_ac.bb_overall(wall)
    dc = boitier_dc.dc_contour(wall)
    return dc[:3] + [
        (ac.max.X, ac.min.Y),
        (ac.max.X, ac.max.Y),
        (ac.min.X, ac.max.Y),
    ] + dc[4:]


def _overlap(a0, a1, b0, b1):
    return a0 < b1 and b0 < a1


def _join_close(rects):
    """Stretch an opening up to its neighbour when the lid between them would be a sliver."""
    for a in rects:
        for b in rects:
            if _overlap(a[0], a[2], b[0], b[2]) and 0 < b[1] - a[3] < _ENTRE_MIN:
                a[3] = b[1]
            if _overlap(a[1], a[3], b[1], b[3]) and 0 < b[0] - a[2] < _ENTRE_MIN:
                a[2] = b[0]
    return rects


def _hole(name, wall, jeu):
    """[x0, y0, x1, y1] of the lid opening over one connector, before edges and joins."""
    x0, y0, x1, y1 = carte_plug_rect(name, wall) if name in VERROUS_HT396 else carte_rect(name, wall)
    return [x0 - jeu, y0 - jeu, x1 + jeu, y1 + jeu]


def _j6_rims(wall, jeu, gouttiere_jeu, rim_width):
    """Ribs along the north and west edges of the J6 hole, up to the rim of
    the 24 V wing: they stiffen the narrow neck of lid north of J6."""
    hole_x0, hole_y0, hole_x1, hole_y1 = _hole("combined_j6", wall, jeu)
    _, wing_north = boitier_dc.dc_contour(wall)[-1]
    rim_face = wing_north - wall - gouttiere_jeu - rim_width
    return [
        (hole_x0 - rim_width, hole_y1, hole_x1, rim_face),
        (hole_x0 - rim_width, hole_y0, hole_x0, rim_face),
    ]


def _group(rects):
    """[x0, y0, x1, y1] covering the rects in Y and to the east, from the
    west edge of the first one."""
    x0s, y0s, x1s, y1s = zip(*rects)
    return [x0s[0], min(y0s), max(x1s), max(y1s)]


def _passage(west, east):
    """Opening across the lid between two openings side by side, over their common Y."""
    return [west[2], max(west[1], east[1]), east[0], min(west[3], east[3])]


def openings(wall, jeu):
    """Lid openings (x0, y0, dx, dy) over the vertical connectors."""
    east, _ = boitier_dc.dc_contour(wall)[1]
    rects = {}
    for name in _CONNECTEURS:
        rect = _hole(name, wall, jeu)
        # Out through the south and east edges of boitier_dc.
        if rect[1] < _BORD_MIN:
            rect[1] = min(rect[1], -1.0)
        if east - rect[2] < _BORD_MIN:
            rect[2] = max(rect[2], east + 1.0)
        rects[name] = rect
    for group in _REGROUPES:
        shared = _group([rects[n] for n in group])
        for n in group:
            rects[n] = shared
    unique = list({id(r): r for r in rects.values()}.values())
    joined = _join_close(unique)
    joined += [_passage(rects[west], rects[east]) for west, east in _PASSAGES]
    return [(x0, y0, x1 - x0, y1 - y0) for x0, y0, x1, y1 in joined]


@part
def couvercle_acdc(
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.0,
    gouttiere_profondeur=3.0,
    jeu_connecteur=1.0,
    draft=False,
):
    """Couvercle commun de boitier_ac et boitier_dc, qui tient les deux
    ensemble. Imprimé à l'envers, miroir en X.

    epaisseur_paroi: épaisseur des murs des deux boîtiers
    epaisseur_fond: épaisseur de la plaque
    gouttiere_jeu: jeu entre le rebord et la face intérieure des murs
    gouttiere_epaisseur: épaisseur du rebord
    gouttiere_profondeur: profondeur du rebord dans les boîtiers
    jeu_connecteur: marge autour de chaque connecteur (ou plug HT396) dans les ouvertures
    """
    wall = epaisseur_paroi
    if gouttiere_jeu < 0.1:
        reject(f"gouttiere_jeu {gouttiere_jeu} is under 0.1 mm: raise it", param="gouttiere_jeu")
    if gouttiere_epaisseur < 0.8:
        reject(
            f"gouttiere_epaisseur {gouttiere_epaisseur} is under 0.8 mm: raise it",
            param="gouttiere_epaisseur",
        )
    if jeu_connecteur < 0.0:
        reject(f"jeu_connecteur {jeu_connecteur} is negative: raise it", param="jeu_connecteur")

    body = couvercle_corps(
        acdc_contour(wall),
        boitier_ac.corbels(wall) + boitier_dc.corbels(wall),
        wall, epaisseur_fond,
        gouttiere_jeu, gouttiere_epaisseur, gouttiere_profondeur,
        openings(wall, jeu_connecteur),
        _j6_rims(wall, jeu_connecteur, gouttiere_jeu, gouttiere_epaisseur),
    )
    if draft:
        return body
    return polish_couvercle(body, epaisseur_fond, gouttiere_profondeur)

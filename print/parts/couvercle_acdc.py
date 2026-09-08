import math

from nurb import *

from system import INSERT_M25, INSERT_M3, offset_in

_AMIN = (Align.MIN, Align.MIN, Align.MIN)
_CMIN = (Align.CENTER, Align.CENTER, Align.MIN)


def _contour_dc():
    """boitier_dc's own outer contour. Must track `boitier_dc._contour`."""
    y_max = measured("boitier_int_y")
    aile_x = measured("boitier_int_aile_x")
    chanfrein = measured("boitier_int_chanfrein")
    marche_x = measured("boitier_int_marche_x")
    marche_y = measured("boitier_int_marche_y") + 3.0
    return [
        (0.0, chanfrein),
        (chanfrein, 0.0),
        (aile_x, 0.0),
        (aile_x, y_max),
        (marche_x, y_max),
        (marche_x, marche_y),
        (0.0, marche_y),
    ]


def _contour_ac(ac_west_shift, ac_east_shift, ac_south_shift):
    """boitier_ac's own outer contour, in the ensemble frame (shifted east
    so its west face lands flush on boitier_dc's east wall). Must track
    `boitier_ac._contour`.

    Plain rectangle: the north-face screw-notch step (`encoche_est` /
    `gout_x`) only cuts boitier_ac's wall up to `hauteur_vis` — above that
    its lintel is deliberately continuous (`boitier_ac.md`), so both the
    plate and the rim stay solid there too (2026-09-06, user: a notched
    contour here punches a hole in the couvercle over solid material)."""
    x_max = measured("boitier_int_x")
    y_max = measured("boitier_int_y")
    aile_x = measured("boitier_int_aile_x")
    aile_y = measured("boitier_int_aile_y")

    ac_shift = ac_west_shift
    south_y = aile_y - ac_south_shift
    east_x = x_max - ac_east_shift + ac_shift

    return [
        (aile_x, south_y),
        (east_x, south_y),
        (east_x, y_max),
        (aile_x, y_max),
    ]


def _contour_union(dc_pts, ac_pts):
    """The two boxes' contours, unfolded into one polygon along the shared
    edge (aile_x, south_y)-(aile_x, y_max), for the flat plate only. The
    rims stay on `dc_pts` / `ac_pts` separately: see the module docstring.

    `dc_pts[3]` is `(aile_x, y_max)`, the same point `ac_pts` ends on;
    `ac_pts[0]` is `(aile_x, south_y)`, where `dc_pts[2]-dc_pts[3]` would
    have passed through. Splice `ac_pts` in and drop `dc_pts[3]`."""
    return [dc_pts[0], dc_pts[1], dc_pts[2]] + ac_pts + [dc_pts[4], dc_pts[5], dc_pts[6]]


def _ring(pts_out, pts_in, z0, h):
    a = Pos(0, 0, z0) * extrude(Polygon(*pts_out, align=None), h)
    b = Pos(0, 0, z0 - 0.5) * extrude(Polygon(*pts_in, align=None), h + 1.0)
    return a - b


def _dc_east_opening(body, rib, y, wall, margin, prof):
    """Create a 10x10 opening for cables in the cover, along the east side of the DC boitier"""
    opening_sz = 10
    x = measured("boitier_int_aile_x")-opening_sz
    return (
        body - Pos(x, y, 0) * Box(opening_sz, opening_sz, wall, align=_AMIN),
        rib - Pos(x-margin, y-margin, wall) * Box(opening_sz+margin*2, opening_sz+margin*2, prof, align=_AMIN)
    )

def _pcb_press(body, rib_west, rib_east, y0, y1, appui_pcb_z, wall):
    """PCB press on the the middle 1/3 of a horizontal rib"""
    rib_len_third = (rib_east - rib_west) / 3.0
    appui_x0 = rib_west + rib_len_third
    appui_x1 = appui_x0 + rib_len_third
    return body + Pos(appui_x0, y0, wall) * Box(
        appui_x1 - appui_x0, y1 - y0, appui_pcb_z, align=_AMIN
    )


@part
def couvercle_acdc(
    epaisseur_paroi=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.2,
    gouttiere_profondeur=3.0,
    ac_west_shift=5.0,
    ac_east_shift=1.5,
    ac_south_shift=4.0,
    ac_appui_pcb_hauteur=6.0,
    dc_appui_xiao_hauteur=6.0,
    vis_m25_diametre=2.9,
    vis_m3_diametre=None,
    draft=False,
):
    """Couvercle unique pour `ensemble_boitiers` (boitier_dc + boitier_ac).

    La plaque a exactement le contour extérieur des deux boîtes réunies
    (elle ne déborde pas). Le rebord intérieur, lui, est calculé séparément
    pour chaque boîte, à partir de son propre contour : boitier_dc et
    boitier_ac gardent chacun leur propre boucle fermée, visibles
    séparément, qui ne se rejoignent jamais. À la jonction (face est de
    boitier_dc contre face ouest de boitier_ac, collées sans jeu), les deux
    boucles passent l'une à côté de l'autre sans se toucher : les coller
    aurait mis le rebord en conflit avec les faces à cet endroit. Sur le
    chanfrein sud-ouest de boitier_dc, dont la moitié ouest est une
    ouverture (rien ne monte pour soutenir un rebord), la réglette reste
    telle quelle : elle ne sert à rien à cet endroit mais ne gêne pas non
    plus, et la couper laissait un défaut (esquille, lu comme un
    "chanfrein négatif") plus gênant que la réglette elle-même. Coupé aux
    trois logements de heat insert qui affleurent le sommet, avec un trou
    de vis dans chacun : deux M2.5 sur boitier_dc, un M3 sur boitier_ac.

    epaisseur_paroi: épaisseur de la plaque et des murs des deux boîtes
    gouttiere_jeu: jeu entre le rebord et la face intérieure d'un mur
    gouttiere_epaisseur: épaisseur du rebord
    gouttiere_profondeur: profondeur du rebord dans la cavité
    ac_west_shift: décalage de boitier_ac appliqué par ensemble_boitiers,
        doit rester égal à `boitier_int_aile_x − ac_west_shift` interne à
        boitier_ac
    ac_east_shift: retrait de la face est de boitier_ac, doit rester égal à
        sa valeur interne
    ac_south_shift: retrait de la face sud de boitier_ac, doit rester égal
        à sa valeur interne
    ac_appui_pcb_hauteur: profondeur du rebord nord de boitier_ac sur son
        tiers central en X (aile_x + (east_x−aile_x)/3 à aile_x +
        (east_x−aile_x)*2/3) : presse le PCB du module sud par le dessus.
        Valeur empirique, indépendante des cotes internes de boitier_ac
        (muret_depuis_ouest, tour_y, rebord...).
    dc_appui_pcb_hauteur: profondeur du rebord sud de boitier_dc pour tenir
        le module XIAO
    vis_m25_diametre: passage des deux vis M2.5 (boitier_dc)
    vis_m3_diametre: passage de la vis M3 (boitier_ac) ; par défaut la cote
        mesurée `vis_passage`
    """
    wall = epaisseur_paroi
    jeu = gouttiere_jeu
    ep = gouttiere_epaisseur
    prof = gouttiere_profondeur
    margin = 0.5
    if vis_m3_diametre is None:
        vis_m3_diametre = measured("vis_passage")

    if wall < 1.0:
        reject(f"epaisseur_paroi {wall} is under 1 mm: raise it", param="epaisseur_paroi")
    if jeu < 0.1:
        reject(f"gouttiere_jeu {jeu} is under 0.1 mm: raise it", param="gouttiere_jeu")
    if ep < 0.8:
        reject(
            f"gouttiere_epaisseur {ep} is under 0.8 mm: raise it",
            param="gouttiere_epaisseur",
        )
    if prof < 1.5:
        reject(
            f"gouttiere_profondeur {prof} is under 1.5 mm: raise it",
            param="gouttiere_profondeur",
        )
    if vis_m25_diametre < 2.0:
        reject(
            f"vis_m25_diametre {vis_m25_diametre} is under 2 mm: raise it",
            param="vis_m25_diametre",
        )
    if vis_m3_diametre < 2.0:
        reject(
            f"vis_m3_diametre {vis_m3_diametre} is under 2 mm: raise it",
            param="vis_m3_diametre",
        )

    marche_x = measured("boitier_int_marche_x")
    chanfrein_cb = measured("boitier_int_chanfrein")

    dc_pts = _contour_dc()
    ac_pts = _contour_ac(ac_west_shift, ac_east_shift, ac_south_shift)

    # Plate matches the two boxes' outer footprint exactly: it must not
    # overhang.
    plate_pts = _contour_union(dc_pts, ac_pts)
    body = extrude(Polygon(*plate_pts, align=None), wall)

    # Each box's rim comes from its OWN contour, offset inward by
    # `epaisseur_paroi + gouttiere_jeu` to clear the wall's inner (cavity)
    # face — `offset_in` measures from the wall's OUTER face, one `wall`
    # short of the cavity. The two loops are built and clipped separately
    # so they never touch at the seam (see the module docstring).
    rib_dc = _ring(
        offset_in(dc_pts, wall + jeu), offset_in(dc_pts, wall + jeu + ep), wall, prof
    )
    rib_ac = _ring(
        offset_in(ac_pts, wall + jeu), offset_in(ac_pts, wall + jeu + ep), wall, prof
    )

    # boitier_dc's SW diagonal chamfer is open on its west half (no wall
    # there to register a rim against): the rim strip along that stretch
    # hangs unsupported. Tried cutting it away, but the cut's own edge met
    # the adjacent straight ribs at a shallow angle and left a sliver
    # (min_wall warning, and read as a stray negative chamfer in the
    # viewer). The strip does nothing there either way, so it stays
    # (user 2026-09-05): harmless, and the clean rib end is worth more
    # than removing an unused sliver of plastic.
    s2 = math.sqrt(2.0)
    tan_x, tan_y = 1.0 / s2, -1.0 / s2
    nrm_x, nrm_y = 1.0 / s2, 1.0 / s2

    # Openings for sensors
    body, rib_dc = _dc_east_opening(body, rib_dc, 30, wall, margin*2, prof)
    body, rib_dc = _dc_east_opening(body, rib_dc, 70, wall, margin*2, prof)

    # Three heat-insert corbels, all flush with the rim: clear the rim only
    # over each pad's footprint, same recipe (and same defaults) as the
    # parts themselves.
    d25 = INSERT_M25.diametre_percage
    plat_25 = d25 + wall
    along_25 = d25 + 2.0 * wall
    half_25 = along_25 / 2.0
    r_25 = d25 / 2.0

    # Corbel 1 (boitier_dc, upper west face at x = marche_x + wall).
    corbel1_x = marche_x + wall
    corbel1_y = 60.6
    rib_dc = rib_dc - Pos(corbel1_x - margin, corbel1_y - half_25, wall - 0.5) * Box(
        plat_25 + margin, along_25, prof + 1.0, align=_AMIN
    )
    vis1 = (corbel1_x + r_25, corbel1_y)

    # Corbel 2 (boitier_dc, SW diagonal chamfer, east/solid half). Same
    # local frame as boitier_dc: x_dir along the inward normal, z_dir along
    # the wall's tangent. `edge_clear = 2.0` must match boitier_dc's value
    # (1.0 leaves a floating sliver there once the pad is M2.5-sized).
    mid_x, mid_y = chanfrein_cb / 2.0, chanfrein_cb / 2.0
    cut_margin = 0.5
    edge_clear = 2.0
    offset_from_mid = cut_margin + edge_clear + half_25
    wall_face_x = mid_x + offset_from_mid * tan_x + wall * nrm_x
    wall_face_y = mid_y + offset_from_mid * tan_y + wall * nrm_y
    plane2 = Plane(
        origin=(
            wall_face_x - half_25 * tan_x,
            wall_face_y - half_25 * tan_y,
            wall - 0.5,
        ),
        x_dir=(nrm_x, nrm_y, 0.0),
        z_dir=(tan_x, tan_y, 0.0),
    )
    keepout2_pts = [
        (-margin, 0.0),
        (plat_25 + margin, 0.0),
        (plat_25 + margin, prof + 1.0),
        (-margin, prof + 1.0),
    ]
    rib_dc = rib_dc - extrude(plane2 * Polygon(*keepout2_pts, align=None), along_25)
    vis2 = (wall_face_x + r_25 * nrm_x, wall_face_y + r_25 * nrm_y)

    # Corbel 3 (boitier_ac, south wall, centred by default: insert_sud_decalage_x = 0).
    d3 = INSERT_M3.diametre_percage
    plat_3 = d3 + wall
    along_3 = d3 + 2.0 * wall
    half_3 = along_3 / 2.0
    r_3 = d3 / 2.0
    aile_x = measured("boitier_int_aile_x")
    south_y = ac_pts[0][1]
    x_outer_west = aile_x
    east_x_outer = measured("boitier_int_x") - ac_east_shift + ac_west_shift
    corbel3_x = 0.5 * (x_outer_west + east_x_outer)
    rib_ac = rib_ac - Pos(
        corbel3_x - half_3, south_y - margin, wall - 0.5
    ) * Box(along_3, plat_3 + margin, prof + 1.0, align=_AMIN)
    vis3 = (corbel3_x, south_y + wall + r_3)

    # PCB press AC: the north rim segment (boitier_ac) runs deeper than
    # `gouttiere_profondeur` over the box's middle third in X, to press down
    # on the south module's PCB from above. 
    if ac_appui_pcb_hauteur <= 0.0:
        reject(
            f"ac_appui_pcb_hauteur {ac_appui_pcb_hauteur} is not positive: raise it",
            param="ac_appui_pcb_hauteur",
        )
    y_max = ac_pts[2][1]
    appui_y1 = y_max - (wall + jeu)
    body = _pcb_press(body, ac_pts[0][0], ac_pts[2][0], appui_y1 - ep, appui_y1, ac_appui_pcb_hauteur, wall)

    # PCB press DC on south rib to hold the XIAO module
    if dc_appui_xiao_hauteur <= 0.0:
        reject(
            f"dc_appui_xiao_hauteur {dc_appui_xiao_hauteur} is not positive: raise it",
            param="dc_appui_xiao_hauteur",
        )
    appui_y0 = wall + jeu
    body = _pcb_press(body, dc_pts[1][0], dc_pts[2][0], appui_y0, appui_y0 + ep, dc_appui_xiao_hauteur, wall)

    body = body + rib_dc + rib_ac

    for cx, cy in (vis1, vis2):
        body = body - Pos(cx, cy, -0.5) * Cylinder(
            vis_m25_diametre / 2.0, wall + 1.0, align=_CMIN
        )
    body = body - Pos(vis3[0], vis3[1], -0.5) * Cylinder(
        vis_m3_diametre / 2.0, wall + 1.0, align=_CMIN
    )

    # Placing the lid means flipping it over (about a north-south axis, the
    # long way): that mirrors X. Everything above is modelled in the direct
    # ensemble_boitiers frame; mirror the whole part in X here so it lands
    # correctly once flipped onto the boxes (same fix as couvercle_ps).
    east_x = ac_pts[1][0]
    mid_x = east_x / 2.0
    mirror_x = Plane(origin=(mid_x, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
    body = mirror(body, about=mirror_x)

    if draft:
        return body

    bed = body.bounding_box().min.Z

    def keep(edge):
        bb = edge.bounding_box()
        return bb.min.Z <= bed + 0.05 or bb.min.Z >= wall + prof - 0.05

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 0.6)

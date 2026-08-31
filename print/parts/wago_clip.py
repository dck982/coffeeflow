from nurb import *

# Keep in phase with wagox5 defaults: two Wagos, same jeu / walls / wings.
N_BORNES = 2
JEU_WAGO = 0.3
EPAISSEUR_MUR_WAGO = 1.2
EPAISSEUR_JUPE_WAGO = 1.2
EXTENSION_WAGO = 9.0
BASE_WAGO = 2.0
PORTEE_RENFORT_WAGO = 3.5
JEU_RENFORT = 0.5

# Zip-tie pass on the plaque extension. Slot centres ±10.3, canal 4 mm in Y.
FENTE_COLLIER_X = 2.0
CANAL_COLLIER_Y = 4.0
ENTRAXE_COLLIER = 20.6
X_FENTE_INT = 9.3
X_FENTE_EXT = 11.3
EVIDEMENT_COLLIER = 1.4
LANGUETTE_COLLIER = 0.4
CONGE_COLLIER = 0.2


def _taille_logement(largeur_borne):
    """Outer footprint of wagox5 at the same largeur_borne, matching its defaults."""
    epaisseur = measured("wago_epaisseur")
    inner_x = largeur_borne + JEU_WAGO
    inner_y = N_BORNES * epaisseur + JEU_WAGO
    body_x = inner_x + 2.0 * EPAISSEUR_MUR_WAGO
    body_y = inner_y + 2.0 * EPAISSEUR_JUPE_WAGO
    base_x = body_x + 2.0 * EXTENSION_WAGO
    return base_x, body_x, body_y


@part
def wago_clip(
    largeur_borne=30.0,
    jeu_glissiere=0.4,
    epaisseur_plancher=2.0,
    epaisseur_plaque=4.0,
    epaisseur_u=2.0,
    epaisseur_retour=1.8,
    jeu_fente=0.2,
    largeur_languette=14.0,
    largeur_fente=1.0,
    depassement_languette=2.0,
    hauteur_levre=1.0,
    hauteur_plaque=23.2,
    recul_serre_cable=5.0,
    largeur_serre_cable=38.0,
    draft=False,
):
    """Plaque à deux U : le fond de wagox5 y glisse, languette devant, aimants 8x3 dans la plaque.

    largeur_borne: même slider que wagox5 (30 = 221-415, 18.8 = 221-423)
    jeu_glissiere: extra total sur la largeur pour que les ailes coulissent
    epaisseur_plancher: fond des U, sous les ailes
    epaisseur_plaque: plaque contre le châssis, assez pour l'aimant 3 mm plus 0,6 mm de peau
    epaisseur_u: paroi extérieure de chaque U
    epaisseur_retour: retour de chaque U, au-dessus de l'aile
    jeu_fente: extra de hauteur dans la fente, au-dessus des 2 mm d'aile
    largeur_languette: languette de maintien, entre les deux fentes
    largeur_fente: fentes qui séparent la languette du plancher
    depassement_languette: la languette dépasse devant le plancher
    hauteur_levre: lèvre d'accroche au bout de la languette
    hauteur_plaque: hauteur de la plaque (sur le mur, une fois collée)
    recul_serre_cable: distance en Y entre la plaque et les fentes (1 à 10 mm ; plus grand = plus de place pour des câbles rigides)
    largeur_serre_cable: largeur de cette plaque, centrée en X
    """
    puit_d = measured("puit_diametre")
    puit_r = puit_d / 2.0
    puit_fond = measured("puit_fond")
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")

    log_x, body_x, log_y = _taille_logement(largeur_borne)
    cav_x = log_x + jeu_glissiere
    cav_y = log_y + jeu_glissiere
    y_sol = epaisseur_plancher
    y_fente = y_sol + BASE_WAGO + jeu_fente
    y_u = y_fente + epaisseur_retour

    if jeu_glissiere < 0.2:
        reject(
            f"jeu_glissiere {jeu_glissiere} est sous 0.2 mm : ça coince. Remonte au-dessus de 0.2",
            param="jeu_glissiere",
        )
    if epaisseur_plaque < puit_fond + aimant_h:
        reject(
            f"epaisseur_plaque {epaisseur_plaque} ne loge pas un aimant de {aimant_h} mm "
            f"plus {puit_fond} mm de peau. Monte au-dessus de {puit_fond + aimant_h:.1f}",
            param="epaisseur_plaque",
        )
    if puit_d < aimant_d + 0.1:
        reject(
            f"puit_diametre {puit_d} is too tight for an {aimant_d} mm magnet",
            param="epaisseur_plaque",
        )
    if epaisseur_u < 1.2:
        reject(
            f"epaisseur_u {epaisseur_u} is under 1.2 mm: raise it",
            param="epaisseur_u",
        )
    if epaisseur_retour < 1.2:
        reject(
            f"epaisseur_retour {epaisseur_retour} is under 1.2 mm: raise it",
            param="epaisseur_retour",
        )
    if jeu_fente < 0.1:
        reject(
            f"jeu_fente {jeu_fente} est sous 0.1 mm : l'aile coince. Remonte au-dessus de 0.1",
            param="jeu_fente",
        )
    if largeur_languette < 8.0:
        reject(
            f"largeur_languette {largeur_languette} is under 8 mm: raise it",
            param="largeur_languette",
        )
    if largeur_fente < 0.8:
        reject(
            f"largeur_fente {largeur_fente} is under 0.8 mm: the slit will close. Raise it",
            param="largeur_fente",
        )
    if hauteur_levre < 0.6:
        reject(
            f"hauteur_levre {hauteur_levre} is under 0.6 mm: it will not catch. Raise it",
            param="hauteur_levre",
        )
    if y_u + 1.0 > hauteur_plaque:
        reject(
            f"hauteur_plaque {hauteur_plaque} n'est pas plus haute que les U "
            f"({y_u:.1f} mm). Monte au-dessus de {y_u + 1.0:.1f}",
            param="hauteur_plaque",
        )
    if recul_serre_cable < 1.0:
        reject(
            f"recul_serre_cable {recul_serre_cable} is under 1 mm: the pad "
            f"has no neck. Raise it above 1",
            param="recul_serre_cable",
        )
    if recul_serre_cable > 10.0:
        reject(
            f"recul_serre_cable {recul_serre_cable} is above 10 mm: the pad "
            f"gets floppy. Drop it to 10 or under",
            param="recul_serre_cable",
        )
    t_plaque = epaisseur_plaque
    upper_run = t_plaque - LANGUETTE_COLLIER
    lower_run = t_plaque - EVIDEMENT_COLLIER
    x_ramp_top = X_FENTE_EXT + upper_run
    if lower_run < 0.5:
        reject(
            f"epaisseur_plaque {t_plaque} is too thin for the underside ramp. "
            f"Raise it above {EVIDEMENT_COLLIER + CONGE_COLLIER + 0.5:.1f}",
            param="epaisseur_plaque",
        )
    if largeur_serre_cable < 2.0 * x_ramp_top + 2.0:
        reject(
            f"largeur_serre_cable {largeur_serre_cable} is too narrow for the "
            f"zip-tie ramps (need {2.0 * x_ramp_top + 2.0:.1f}). Raise it",
            param="largeur_serre_cable",
        )

    cmin = (Align.CENTER, Align.MIN, Align.MIN)
    zcyl = (Align.CENTER, Align.CENTER, Align.MIN)

    x_outer = cav_x / 2.0 + epaisseur_u
    if 2.0 * x_outer + 0.5 < largeur_serre_cable:
        reject(
            f"largeur_borne {largeur_borne} makes the plaque narrower than "
            f"largeur_serre_cable {largeur_serre_cable}. Lower largeur_serre_cable",
            param="largeur_serre_cable",
        )
    x_wall_in = cav_x / 2.0
    # Goussets sit on the first 3.5 mm of wing; the U return starts after that.
    x_retour = body_x / 2.0 + PORTEE_RENFORT_WAGO + JEU_RENFORT
    if x_retour >= x_wall_in - 1.5:
        reject(
            f"largeur_borne {largeur_borne} leaves under 1.5 mm of U return. "
            f"Raise extension on wagox5 or drop jeu_glissiere",
            param="largeur_borne",
        )

    z_front = epaisseur_plaque + cav_y
    z_tongue = z_front + depassement_languette
    # Hinge at the plaque; slits run through the tip so the tongue
    # is free of the two U's all the way to the top.
    z_slit0 = epaisseur_plaque + 2.0
    slit_z = z_tongue - z_slit0 + 0.2

    plaque = Box(2.0 * x_outer, hauteur_plaque, epaisseur_plaque, align=cmin)
    shelf = Box(2.0 * x_outer, y_sol, z_tongue, align=cmin)

    us = None
    for sign in (-1.0, 1.0):
        wall = Pos(sign * (x_wall_in + epaisseur_u / 2.0), 0, 0) * Box(
            epaisseur_u, y_u, z_front, align=cmin
        )
        retour_w = x_wall_in - x_retour
        retour = Pos(sign * (x_retour + retour_w / 2.0), y_fente, 0) * Box(
            retour_w, epaisseur_retour, z_front, align=cmin
        )
        u = wall + retour
        us = u if us is None else us + u

    # Lip: vertical catch facing the plaque, 45° lead-in from the front.
    z_catch = z_front + 0.3
    z_lip_flat = z_catch + 0.7
    lip_pts = [
        (y_sol, z_catch),
        (y_sol + hauteur_levre, z_catch),
        (y_sol + hauteur_levre, z_lip_flat),
        (y_sol, z_tongue),
    ]
    lip = Pos(-largeur_languette / 2.0, 0, 0) * extrude(
        Plane.YZ * Polygon(*lip_pts, align=None), largeur_languette
    )

    body = plaque + shelf + us + lip

    for sign in (-1.0, 1.0):
        x_slit = sign * (largeur_languette / 2.0 + largeur_fente / 2.0)
        slit = Pos(x_slit, -0.1, z_slit0) * Box(
            largeur_fente, y_sol + 0.2, slit_z, align=cmin
        )
        body = body - slit

    y_well_min = y_u + puit_r + 1.0
    y_well_max = hauteur_plaque - puit_r - 1.0
    if y_well_min > y_well_max:
        reject(
            f"hauteur_plaque {hauteur_plaque} is too short for a Ø{puit_d} well "
            f"above the U. Raise it above {y_u + puit_d + 2.0:.1f}",
            param="hauteur_plaque",
        )
    y_well = 0.5 * (y_well_min + y_well_max)
    x_well = x_outer - puit_r - 2.0
    if x_well < puit_r + 2.0:
        reject(
            f"largeur_borne {largeur_borne} is too narrow for two Ø{puit_d} wells "
            f"in the plaque. Raise it",
            param="largeur_borne",
        )
    well_h = epaisseur_plaque - puit_fond + 0.1
    for sign in (-1.0, 1.0):
        body = body - Pos(sign * x_well, y_well, puit_fond) * Cylinder(
            puit_r, well_h, align=zcyl
        )

    y_pad0 = hauteur_plaque - 0.2
    y_canal_min = hauteur_plaque + recul_serre_cable
    y_pad1 = y_canal_min + CANAL_COLLIER_Y + 3.0
    # Plane.XZ extrude runs -Y, so start at the +Y end of the canal.
    y_canal_cut = y_canal_min + CANAL_COLLIER_Y
    pad = Pos(0, y_pad0, 0) * Box(
        largeur_serre_cable, y_pad1 - y_pad0, t_plaque, align=cmin
    )
    body = body + pad

    x_ev = X_FENTE_INT - lower_run
    z_inner = t_plaque + 0.05
    for sign in (-1.0, 1.0):
        body = body - Pos(sign * (ENTRAXE_COLLIER / 2.0), y_canal_min, -0.1) * Box(
            FENTE_COLLIER_X, CANAL_COLLIER_Y, t_plaque + 0.2, align=cmin
        )
    void_under = [
        (-X_FENTE_INT, -0.1),
        (X_FENTE_INT, -0.1),
        (X_FENTE_INT, z_inner),
        (x_ev, EVIDEMENT_COLLIER),
        (-x_ev, EVIDEMENT_COLLIER),
        (-X_FENTE_INT, z_inner),
    ]
    body = body - (
        Pos(0, y_canal_cut, 0)
        * extrude(Plane.XZ * Polygon(*void_under, align=None), CANAL_COLLIER_Y)
    )
    for sign in (-1.0, 1.0):
        xs = sign * X_FENTE_EXT
        xr = sign * x_ramp_top
        xc = xr - sign * CONGE_COLLIER
        pts = (
            [
                (xs, LANGUETTE_COLLIER),
                (xc, t_plaque - CONGE_COLLIER),
                (xr, t_plaque),
                (xr, t_plaque + 0.2),
                (xs, t_plaque + 0.2),
            ]
            if sign > 0
            else [
                (xs, LANGUETTE_COLLIER),
                (xs, t_plaque + 0.2),
                (xr, t_plaque + 0.2),
                (xr, t_plaque),
                (xc, t_plaque - CONGE_COLLIER),
            ]
        )
        body = body - (
            Pos(0, y_canal_cut, 0)
            * extrude(Plane.XZ * Polygon(*pts, align=None), CANAL_COLLIER_Y)
        )

    if draft:
        return body

    hx = x_outer
    hy0 = 0.0
    hy_pad = y_pad1
    x_pad = largeur_serre_cable / 2.0

    def u_and_plaque_outline(edge):
        bb = edge.bounding_box()
        cx = abs((bb.min.X + bb.max.X) / 2.0)
        cy = (bb.min.Y + bb.max.Y) / 2.0
        cz = (bb.min.Z + bb.max.Z) / 2.0
        dx = bb.max.X - bb.min.X
        dy = bb.max.Y - bb.min.Y
        dz = bb.max.Z - bb.min.Z
        on_x = abs(cx - hx) < 0.4
        on_bed_y = abs(cy - hy0) < 0.4
        at_front = abs(cz - z_front) < 0.4
        on_pad = abs(cx - x_pad) < 0.4 and abs(cy - hy_pad) < 0.4
        # Long Z edges: plaque corners and the pad's outer corners. Not the U top.
        if dx < 0.3 and dy < 0.3 and dz > 1.5:
            return (on_x and on_bed_y) or on_pad
        # Front outer corners of each U, rising from the shelf.
        if at_front and dz < 0.3 and on_x and dy > 1.0:
            return True
        return False

    keep = body.edges().filter_by(u_and_plaque_outline)
    keep = keep - concave_edges(body)
    return polish(body, keep, 1.0)

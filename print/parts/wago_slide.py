from nurb import *

# Keep in phase with wagox5 defaults: two Wagos, same jeu / walls / wings.
N_BORNES = 2
JEU_WAGO = -0.2
EPAISSEUR_BORNE_WAGO = 8.0
EPAISSEUR_MUR_WAGO = 1.2
EPAISSEUR_JUPE_WAGO = 1.2
EXTENSION_WAGO = 9.0
BASE_WAGO = 2.0
PORTEE_RENFORT_WAGO = 1.6
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
    inner_x = largeur_borne + JEU_WAGO
    inner_y = N_BORNES * EPAISSEUR_BORNE_WAGO + JEU_WAGO
    body_x = inner_x + 2.0 * EPAISSEUR_MUR_WAGO
    body_y = inner_y + 2.0 * EPAISSEUR_JUPE_WAGO
    base_x = body_x + 2.0 * EXTENSION_WAGO
    return base_x, body_x, body_y


@part
def wago_slide(
    largeur_borne=30.0,
    jeu_glissiere=0.4,
    epaisseur_plancher=2.0,
    epaisseur_plaque=4.0,
    epaisseur_u=2.0,
    epaisseur_retour=1.8,
    jeu_fente=0.2,
    avance_devant=10.0,
    retrait_slot=5.0,
    hauteur_reglette=1.0,
    rebord_ailette=2.0,
    hauteur_plaque=23.2,
    recul_serre_cable=5.0,
    largeur_serre_cable=38.0,
    draft=False,
):
    """Plaque à deux U en T : toit 10 mm devant les bornes, slot en retrait, réglette au bord.

    largeur_borne: même slider que wagox5 (30 = 221-415, 18.8 = 221-423)
    jeu_glissiere: extra total sur la largeur pour que les ailes coulissent
    epaisseur_plancher: fond des U, sous les ailes
    epaisseur_plaque: plaque contre le châssis, assez pour l'aimant 3 mm plus 0,6 mm de peau
    epaisseur_u: paroi extérieure de chaque U
    epaisseur_retour: retour de chaque U, au-dessus de l'aile
    jeu_fente: extra de hauteur dans la fente, au-dessus des 2 mm d'aile
    avance_devant: toit en Z devant les bornes, au moins 10 mm (murs en Y compris)
    retrait_slot: le retour en X (le slot) s'arrête d'autant avant le bord du toit
    hauteur_reglette: débord au bord du toit, goutte pendante et cran à plat
    rebord_ailette: petit mur en Y au-delà du retour, transforme le L en T
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
    y_mur = y_u + rebord_ailette

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
    if avance_devant < 10.0:
        reject(
            f"avance_devant {avance_devant} est sous 10 mm : le toit ne dépasse plus "
            f"assez des bornes. Monte au-dessus de 10",
            param="avance_devant",
        )
    if retrait_slot < 2.0:
        reject(
            f"retrait_slot {retrait_slot} est sous 2 mm : plus d'angle d'entrée. "
            f"Monte au-dessus de 2",
            param="retrait_slot",
        )
    if retrait_slot > avance_devant - epaisseur_retour:
        reject(
            f"retrait_slot {retrait_slot} mange le logement assis "
            f"(toit {avance_devant} mm, rampe 45° {epaisseur_retour} mm). "
            f"Descends sous {avance_devant - epaisseur_retour:.1f}",
            param="retrait_slot",
        )
    if hauteur_reglette < 0.6:
        reject(
            f"hauteur_reglette {hauteur_reglette} is under 0.6 mm: it will not catch. "
            f"Raise it",
            param="hauteur_reglette",
        )
    if hauteur_reglette > y_fente - y_sol - 0.4:
        reject(
            f"hauteur_reglette {hauteur_reglette} bouche le slot "
            f"({y_fente - y_sol:.1f} mm). Descends sous {y_fente - y_sol - 0.4:.1f}",
            param="hauteur_reglette",
        )
    if 2.0 * hauteur_reglette >= retrait_slot:
        reject(
            f"hauteur_reglette {hauteur_reglette} empiète sur le slot "
            f"(rampe 2× = {2.0 * hauteur_reglette:.1f}, retrait {retrait_slot}). "
            f"Descends sous {retrait_slot / 2.0:.1f}",
            param="hauteur_reglette",
        )
    if rebord_ailette < 1.2:
        reject(
            f"rebord_ailette {rebord_ailette} is under 1.2 mm: raise it",
            param="rebord_ailette",
        )
    if y_mur + 1.0 > hauteur_plaque:
        reject(
            f"hauteur_plaque {hauteur_plaque} n'est pas plus haute que les U "
            f"({y_mur:.1f} mm). Monte au-dessus de {y_mur + 1.0:.1f}",
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
    # Goussets sit on the first 1.6 mm of wing; the U return starts after that.
    x_retour = body_x / 2.0 + PORTEE_RENFORT_WAGO + JEU_RENFORT
    if x_retour >= x_wall_in - 1.5:
        reject(
            f"largeur_borne {largeur_borne} leaves under 1.5 mm of U return. "
            f"Raise extension on wagox5 or drop jeu_glissiere",
            param="largeur_borne",
        )

    z_front = epaisseur_plaque + cav_y
    z_end = z_front + avance_devant
    z_retour = z_end - retrait_slot
    ramp_retour = epaisseur_retour

    plaque = Box(2.0 * x_outer, hauteur_plaque, epaisseur_plaque, align=cmin)
    shelf = Box(2.0 * x_outer, y_sol, z_end, align=cmin)

    # 1 mm at both corners of the T-stem tip (same as polish). On the
    # default 2 mm wall that meets in a ridge along Z.
    tip = min(1.0, epaisseur_u / 2.0, rebord_ailette)
    y_tip = y_mur - tip
    peaked = epaisseur_u - 2.0 * tip < 0.05

    us = None
    for sign in (-1.0, 1.0):
        x_left = min(sign * x_wall_in, sign * x_outer)
        x_right = max(sign * x_wall_in, sign * x_outer)
        if peaked:
            wall_pts = [
                (x_left, 0),
                (x_right, 0),
                (x_right, y_tip),
                ((x_left + x_right) / 2.0, y_mur),
                (x_left, y_tip),
            ]
        else:
            wall_pts = [
                (x_left, 0),
                (x_right, 0),
                (x_right, y_tip),
                (x_right - tip, y_mur),
                (x_left + tip, y_mur),
                (x_left, y_tip),
            ]
        wall = extrude(Plane.XY * Polygon(*wall_pts, align=None), z_end)
        retour_w = x_wall_in - x_retour
        # Slot ceiling ends 5 mm before the roof; 45° at the mouth so the
        # wing can enter at an angle.
        retour_pts = [
            (y_fente, 0),
            (y_u, 0),
            (y_u, z_retour),
            (y_fente, z_retour - ramp_retour),
        ]
        x0 = x_retour if sign > 0 else -x_wall_in
        retour = Pos(x0, 0, 0) * extrude(
            Plane.YZ * Polygon(*retour_pts, align=None), retour_w
        )
        u = wall + retour
        us = u if us is None else us + u

    # Full-width drip / catch at the roof edge. 45° toward the housing
    # (stop if you stay flat, duck to pass), 1 mm flat at the drip so
    # the tip is not a knife (min_wall 0.6).
    h_lip = hauteur_reglette
    lip_pts = [
        (y_sol, z_end - 2.0 * h_lip),
        (y_sol + h_lip, z_end - h_lip),
        (y_sol + h_lip, z_end),
        (y_sol, z_end),
    ]
    lip = Pos(-x_outer, 0, 0) * extrude(
        Plane.YZ * Polygon(*lip_pts, align=None), 2.0 * x_outer
    )

    body = plaque + shelf + us + lip

    y_well_min = y_mur + puit_r + 1.0
    y_well_max = hauteur_plaque - puit_r - 1.0
    if y_well_min > y_well_max:
        reject(
            f"hauteur_plaque {hauteur_plaque} is too short for a Ø{puit_d} well "
            f"above the U. Raise it above {y_mur + puit_d + 2.0:.1f}",
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
        at_front = abs(cz - z_end) < 0.4
        on_pad = abs(cx - x_pad) < 0.4 and abs(cy - hy_pad) < 0.4
        # Long Z edges: plaque corners and the pad's outer corners. Not the
        # T-stem ridge (already 45°) and not the 1.8 mm return top.
        if dx < 0.3 and dy < 0.3 and dz > 1.5:
            return (on_x and on_bed_y) or on_pad
        # Front outer corners of each U, rising from the shelf.
        if at_front and dz < 0.3 and on_x and dy > 1.0:
            return True
        return False

    keep = body.edges().filter_by(u_and_plaque_outline)
    keep = keep - concave_edges(body)
    return polish(body, keep, 1.0)

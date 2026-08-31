from nurb import *

from system import puit_couche, puit_couche_toit

# Clip outline from wago3.stl, section x = -37.51 mm, 0.08 mm simplification.
# Frame: metal face at x=0, wall front at x=epaisseur_face, +Y up the wall,
# +Z = width = print up. DIN hooks stripped; back closed on x=0.
# y=0 is the lowest point of the lower hood.
_CLIP = [
    (1.600, 46.760),
    (10.280, 51.780),
    (11.010, 51.880),
    (11.610, 51.470),
    (14.000, 47.340),
    (12.530, 46.490),
    (10.530, 49.950),
    (3.600, 45.950),
    (3.610, 34.640),
    (3.910, 34.050),
    (4.520, 33.780),
    (7.310, 34.360),
    (8.280, 34.320),
    (9.220, 34.070),
    (10.350, 33.460),
    (11.070, 32.810),
    (12.500, 30.540),
    (13.190, 30.940),
    (12.990, 31.280),
    (13.420, 31.530),
    (14.060, 30.430),
    (11.460, 28.930),
    (9.940, 31.510),
    (9.350, 32.090),
    (8.620, 32.480),
    (7.820, 32.660),
    (7.200, 32.650),
    (5.110, 32.130),
    (4.040, 32.110),
    (3.760, 31.980),
    (3.600, 31.610),
    (3.600, 20.650),
    (3.750, 20.190),
    (4.040, 19.890),
    (4.420, 19.730),
    (4.910, 19.770),
    (10.340, 22.880),
    (11.010, 22.950),
    (11.610, 22.540),
    (14.000, 18.410),
    (12.530, 17.560),
    (10.530, 21.020),
    (3.600, 17.020),
    (3.600, 5.780),
    (3.860, 5.170),
    (4.450, 4.860),
    (7.310, 5.430),
    (8.280, 5.390),
    (9.220, 5.140),
    (10.350, 4.530),
    (11.070, 3.880),
    (12.500, 1.610),
    (13.190, 2.010),
    (12.990, 2.350),
    (13.420, 2.600),
    (14.060, 1.500),
    (11.460, 0.000),
    (9.940, 2.580),
    (9.350, 3.160),
    (8.620, 3.550),
    (7.410, 3.740),
    (5.110, 3.200),
    (3.870, 3.130),
    (3.670, 2.950),
    (3.600, 2.660),
]

# Wago-cavity centres in Y, one per module (wall gap behind each clip).
_PUITS_Y = (40.295, 11.400)


def _profil(epaisseur_face):
    """Closed XY outline: measured clips + a flat back of `epaisseur_face`."""
    dx = epaisseur_face - 3.6
    pts = [(p[0] + dx, p[1]) for p in _CLIP]
    first = pts[0]
    last = pts[-1]
    return [(0.0, first[1]), (0.0, last[1]), *reversed(pts)]


@part
def wago_snap(
    largeur=15.0,
    epaisseur_face=3.6,
    draft=False,
):
    """Deux pinces Wago 221, face plate aimantée. Coupon à 15 mm de large.

    largeur: longueur en Z, l'axe d'extrusion (15 = coupon, 18.8 = 221-413, 30 = 221-415)
    epaisseur_face: mur contre le métal ; 3.6 = peau 0.6 + aimant 3
    """
    puit_d = measured("puit_diametre_canal")
    puit_r = puit_d / 2.0
    peau = measured("puit_fond")
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")

    if epaisseur_face < peau + aimant_h:
        reject(
            f"epaisseur_face {epaisseur_face} does not fit a {aimant_h} mm magnet "
            f"behind {peau} mm of skin. Raise it above {peau + aimant_h:.1f}",
            param="epaisseur_face",
        )
    toit = puit_couche_toit(puit_r)
    if largeur < 2.0 * toit + 2.0:
        reject(
            f"largeur {largeur} is too short for a Ø{puit_d} well with a 45° roof "
            f"(needs {2.0 * toit + 2.0:.1f}). Raise it",
            param="largeur",
        )
    if puit_d < aimant_d + 0.1:
        reject(
            f"puit_diametre {puit_d} is too tight for an {aimant_d} mm magnet",
            param="epaisseur_face",
        )
    if epaisseur_face < 2.0:
        reject(
            f"epaisseur_face {epaisseur_face} is under 2 mm: raise it",
            param="epaisseur_face",
        )

    body = extrude(Polygon(*_profil(epaisseur_face), align=None), largeur)
    profondeur = epaisseur_face - peau
    z_puit = largeur / 2.0
    for y in _PUITS_Y:
        body = body - (
            Plane(
                origin=(epaisseur_face, y, z_puit),
                z_dir=(-1.0, 0.0, 0.0),
                x_dir=(0.0, -1.0, 0.0),
            )
            * puit_couche(puit_r, profondeur, chanfrein=0.0)
        )

    if draft:
        return body

    bed = body.bounding_box().min.Z
    back = body.bounding_box().min.X
    wall_front = epaisseur_face + 0.2

    def keep(edge):
        bb = edge.bounding_box()
        if bb.min.Z < bed + 0.05:
            return False
        if bb.min.X < back + 0.05:
            return False
        if bb.max.X > wall_front:
            return False
        return True

    keep_edges = body.edges().filter_by(keep) - concave_edges(body)
    return polish(body, keep_edges, 1.0)

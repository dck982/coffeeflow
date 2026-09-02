from nurb import *

from system import MARGE_PUIT, _fuse_one, add_well, offset_in


def _contour(prolongement_est):
    aile_x = measured("boitier_int_aile_x")
    aile_y = measured("boitier_int_aile_y")
    chanfrein = measured("boitier_int_chanfrein")
    marche_x = measured("boitier_int_marche_x")
    marche_y = measured("boitier_int_marche_y")
    east = aile_x + prolongement_est
    return [
        (0.0, chanfrein),
        (chanfrein, 0.0),
        (east, 0.0),
        (east, aile_y),
        (marche_x, aile_y),
        (marche_x, marche_y),
        (0.0, marche_y),
    ]


@part
def boitier_dc(
    hauteur=10.0,
    epaisseur_paroi=1.6,
    prolongement_est=5.0,
    puit_diametre=8.2,
    puit_peau=0.6,
    marge_puit=MARGE_PUIT,
    draft=False,
):
    """Boîtier DC : partie basse du bac intérieur, face est +5 mm, coin plein.

    hauteur: hauteur hors-tout depuis le lit (murs compris)
    epaisseur_paroi: épaisseur du fond et des murs, vers l'intérieur
    prolongement_est: extra sur la face est, pour le composant qui dépasse en +Y
    puit_diametre: diamètre intérieur du puits d'aimant Ø8×3
    puit_peau: plastique sous l'aimant
    marge_puit: plastique autour du puits (doctrine 1,6 mm)
    """
    wall = epaisseur_paroi
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")
    puit_bas_x = measured("boitier_int_puit_bas_x")
    puit_bas_y = measured("boitier_int_puit_bas_y")

    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    if hauteur < wall + 2.0:
        reject(
            f"hauteur {hauteur} leaves under 2 mm of wall above a {wall} mm floor: "
            f"raise it above {wall + 2.0:.1f}",
            param="hauteur",
        )
    if prolongement_est < 0.0:
        reject(
            f"prolongement_est {prolongement_est} is negative: raise it",
            param="prolongement_est",
        )
    if puit_diametre < aimant_d + 0.1:
        reject(
            f"puit_diametre {puit_diametre} is too tight for an {aimant_d} mm magnet",
            param="puit_diametre",
        )
    if puit_peau < 0.4:
        reject(
            f"puit_peau {puit_peau} would knife-edge the well floor: raise it above 0.4",
            param="puit_peau",
        )
    if marge_puit < 1.2:
        reject(
            f"marge_puit {marge_puit} is under 1.2 mm: raise it",
            param="marge_puit",
        )
    well_stack = puit_peau + aimant_h
    if hauteur < well_stack + 0.4:
        reject(
            f"hauteur {hauteur} is under the magnet well ({well_stack + 0.4:.1f} mm): "
            "raise it",
            param="hauteur",
        )

    outer_pts = _contour(prolongement_est)
    inner_pts = offset_in(outer_pts, wall)
    amin = (Align.MIN, Align.MIN, Align.MIN)

    outer = extrude(Polygon(*outer_pts, align=None), hauteur)
    cavity = Pos(0, 0, wall) * extrude(
        Polygon(*inner_pts, align=None), hauteur + 0.2
    )
    body = outer - cavity

    body = add_well(
        body,
        outer,
        puit_bas_x,
        puit_bas_y,
        puit_diametre,
        puit_peau,
        aimant_h,
        marge_puit,
    )

    # Chamfer wall (0, 20) → (20, 0): open the low-X half, leftmost
    # corner to the midpoint. Floor stays.
    chanfrein = measured("boitier_int_chanfrein")
    s2 = 2.0 ** 0.5
    tx, ty = 1.0 / s2, -1.0 / s2
    nx, ny = 1.0 / s2, 1.0 / s2
    ax, ay = 0.0, chanfrein
    mx, my = chanfrein / 2.0, chanfrein / 2.0
    past = 1.0
    inn = wall + 2.0
    margin = 0.5
    body = body - (
        Pos(0, 0, wall)
        * extrude(
            Polygon(
                (ax - past * tx - margin * nx, ay - past * ty - margin * ny),
                (mx + margin * tx - margin * nx, my + margin * ty - margin * ny),
                (mx + margin * tx + inn * nx, my + margin * ty + inn * ny),
                (ax - past * tx + inn * nx, ay - past * ty + inn * ny),
                align=None,
            ),
            hauteur + 0.2,
        )
    )

    body = _fuse_one(body)
    if draft:
        return body

    conc = {
        (round(e.center().X, 2), round(e.center().Y, 2), round(e.center().Z, 2))
        for e in concave_edges(body)
    }

    def keep(edge):
        c = edge.center()
        return (round(c.X, 2), round(c.Y, 2), round(c.Z, 2)) not in conc

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

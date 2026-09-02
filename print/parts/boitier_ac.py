from nurb import *

from system import MARGE_PUIT, _fuse_one, add_well, offset_in


def _contour(vide_haut, decrochage):
    x_max = measured("boitier_int_x")
    y_max = measured("boitier_int_y")
    aile_x = measured("boitier_int_aile_x")
    aile_y = measured("boitier_int_aile_y")
    marche_x = measured("boitier_int_marche_x")
    gout_x = measured("boitier_int_gouttiere_x")
    gout_haut = y_max - vide_haut
    sud_ouest = aile_y + decrochage
    return [
        (marche_x, sud_ouest),
        (aile_x, sud_ouest),
        (aile_x, aile_y),
        (x_max, aile_y),
        (x_max, gout_haut),
        (gout_x, gout_haut),
        (gout_x, y_max),
        (marche_x, y_max),
    ]


@part
def boitier_ac(
    hauteur=10.0,
    epaisseur_paroi=1.6,
    decrochage=11.0,
    gouttiere_vide_haut=10.0,
    puit_diametre=8.2,
    puit_peau=0.6,
    marge_puit=MARGE_PUIT,
    draft=False,
):
    """Boîtier AC : partie haute, décrochage sud-ouest sur la largeur du DC.

    hauteur: hauteur hors-tout depuis le lit (murs compris)
    epaisseur_paroi: épaisseur du fond et des murs, vers l'intérieur
    decrochage: face sud-ouest 11 mm plus au nord, largeur du DC (x = 12 à 50)
    gouttiere_vide_haut: vide sous y max à droite de x = 85 (contrainte machine)
    puit_diametre: diamètre intérieur du puits d'aimant Ø8×3
    puit_peau: plastique sous l'aimant
    marge_puit: plastique autour du puits (doctrine 1,6 mm)
    """
    wall = epaisseur_paroi
    aimant_d = measured("aimant_diametre")
    aimant_h = measured("aimant_hauteur")
    x_max = measured("boitier_int_x")
    y_max = measured("boitier_int_y")
    aile_y = measured("boitier_int_aile_y")
    gout_x = measured("boitier_int_gouttiere_x")

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
    if decrochage < 0.0:
        reject(
            f"decrochage {decrochage} is negative: raise it",
            param="decrochage",
        )
    if gouttiere_vide_haut < 0.5:
        reject(
            f"gouttiere_vide_haut {gouttiere_vide_haut} collapses the top "
            "edge of the east bay into y_max: raise it",
            param="gouttiere_vide_haut",
        )
    gout_haut = y_max - gouttiere_vide_haut
    if gout_haut <= aile_y + 2.0 * wall:
        reject(
            f"gouttiere_vide_haut {gouttiere_vide_haut} leaves no east bay "
            f"above y={aile_y}: lower it",
            param="gouttiere_vide_haut",
        )
    if aile_y + decrochage >= gout_haut - wall:
        reject(
            f"decrochage {decrochage} pushes the south-west face into the "
            f"east bay: lower it",
            param="decrochage",
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

    outer_pts = _contour(gouttiere_vide_haut, decrochage)
    inner_pts = offset_in(outer_pts, wall)

    outer = extrude(Polygon(*outer_pts, align=None), hauteur)
    cavity = Pos(0, 0, wall) * extrude(
        Polygon(*inner_pts, align=None), hauteur + 0.2
    )
    body = outer - cavity

    puit_cx = (gout_x + x_max) / 2.0
    puit_cy = (aile_y + gout_haut) / 2.0
    body = add_well(
        body, outer, puit_cx, puit_cy, puit_diametre, puit_peau, aimant_h, marge_puit
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

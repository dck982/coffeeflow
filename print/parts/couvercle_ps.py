from nurb import *

from system import AMIN, CMIN
from parts.boitier_ps import bb_overall, corbels

_AMIN = AMIN
_CMIN = CMIN


def _cadre(x0, y0, x1, y1, ep, z0, h):
    """Rectangular frame, thickness `ep` on every side, from z0 up by h."""
    outer = Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, h, align=_AMIN)
    inner = Pos(x0 + ep, y0 + ep, z0 - 0.5) * Box(
        x1 - x0 - 2.0 * ep, y1 - y0 - 2.0 * ep, h + 1.0, align=_AMIN
    )
    return outer - inner


@part
def couvercle_ps(
    epaisseur_paroi=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.2,
    gouttiere_profondeur=3.0,
    largeur_passage_cable=5.0,
    vis_diametre=None,
    draft=False,
):
    """Couvercle plat de `boitier_ps`, avec un rebord intérieur qui s'appuie
    contre la face intérieure des murs pour l'ajustage.

    Repère identique à `boitier_ps` : origine au coin intérieur sud-ouest, la
    plaque a exactement les dimensions extérieures de la boîte (elle ne
    déborde pas). Imprimée à plat, face visible sur le lit ; le rebord
    pousse vers +Z et plonge dans la cavité une fois la pièce retournée.

    epaisseur_paroi: épaisseur de la plaque et des murs de boitier_ps
    gouttiere_jeu: jeu entre le rebord et la face intérieure du mur
    gouttiere_epaisseur: épaisseur du rebord (3 périmètres)
    gouttiere_profondeur: profondeur du rebord dans la cavité
    largeur_passage_cable: largeur du canal nord, égal à boitier_ps
    vis_diametre: passage des deux vis M3, dans les inserts ; par défaut la
        cote mesurée `vis_passage`
    """
    wall = epaisseur_paroi
    bb = bb_overall()
    inner_x = bb.size.X
    inner_y = bb.size.Y
    jeu = gouttiere_jeu
    ep = gouttiere_epaisseur
    prof = gouttiere_profondeur
    if vis_diametre is None:
        vis_diametre = measured("vis_passage")

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
    if vis_diametre < 2.0:
        reject(f"vis_diametre {vis_diametre} is under 2 mm: raise it", param="vis_diametre")

    plate_x0, plate_y0 = -wall, -wall
    plate_x1, plate_y1 = inner_x + wall, inner_y + wall

    body = Pos(plate_x0, plate_y0, 0) * Box(
        plate_x1 - plate_x0, plate_y1 - plate_y0, wall, align=_AMIN
    )

    body = body + _cadre(
        jeu, jeu, inner_x - jeu, inner_y - jeu, ep, wall, prof
    )

    margin = 0.5
    for cx, cy, _d, pad in corbels(wall):
        # Extra cut only toward the wall the corbel sits on, so the rib
        # clears the pad without eating the rest of the frame.
        if pad.min.X <= 0.05:
            cutter = Pos(pad.min.X - margin, pad.min.Y, wall - 0.5) * Box(
                pad.size.X + margin, pad.size.Y, prof + 1.0, align=_AMIN
            )
        else:
            cutter = Pos(pad.min.X, pad.min.Y - margin, wall - 0.5) * Box(
                pad.size.X, pad.size.Y + margin, prof + 1.0, align=_AMIN
            )
        body = body - cutter
        body = body - Pos(cx, cy, -0.5) * Cylinder(
            vis_diametre / 2.0, wall + 1.0, align=_CMIN
        )

    body = body + Pos(plate_x0, plate_y1, 0) * Box(
        plate_x1 - plate_x0, largeur_passage_cable + wall, wall, align=_AMIN
    )

    mid_x = inner_x / 2.0
    mirror_x = Plane(origin=(mid_x, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
    body = mirror(body, about=mirror_x)

    if draft:
        return body

    bed = body.bounding_box().min.Z

    def keep(edge):
        ebb = edge.bounding_box()
        return ebb.min.Z <= bed + 0.05 or ebb.min.Z >= wall + prof - 0.05

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 0.6)

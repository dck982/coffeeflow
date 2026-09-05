from nurb import *

from system import INSERT_M3

_AMIN = (Align.MIN, Align.MIN, Align.MIN)
_CMIN = (Align.CENTER, Align.CENTER, Align.MIN)


def _cadre(x0, y0, x1, y1, ep, z0, h):
    """Rectangular frame, thickness `ep` on every side, from z0 up by h."""
    outer = Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, h, align=_AMIN)
    inner = Pos(x0 + ep, y0 + ep, z0 - 0.5) * Box(
        x1 - x0 - 2.0 * ep, y1 - y0 - 2.0 * ep, h + 1.0, align=_AMIN
    )
    return outer - inner


@part
def couvercle_ps(
    longueur_interne=50.0,
    largeur_interne=87.6,
    epaisseur_paroi=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.2,
    gouttiere_profondeur=3.0,
    insert_diametre=INSERT_M3.diametre_percage,
    insert_ouest_y=60.0,
    insert_sud_x=40.0,
    vis_diametre=None,
    draft=False,
):
    """Couvercle plat de `boitier_ps`, avec un rebord intérieur qui s'appuie
    contre la face intérieure des murs pour l'ajustage.

    Repère identique à `boitier_ps` : origine au coin intérieur sud-ouest, la
    plaque a exactement les dimensions extérieures de la boîte (elle ne
    déborde pas). Imprimée à plat, face visible sur le lit ; le rebord
    pousse vers +Z et plonge dans la cavité une fois la pièce retournée.

    longueur_interne: cavité en X, doit rester égal à boitier_ps
    largeur_interne: cavité en Y, doit rester égal à boitier_ps
    epaisseur_paroi: épaisseur de la plaque et des murs de boitier_ps
    gouttiere_jeu: jeu entre le rebord et la face intérieure du mur
    gouttiere_epaisseur: épaisseur du rebord (3 périmètres)
    gouttiere_profondeur: profondeur du rebord dans la cavité
    insert_diametre: alésage des heat inserts, pour caler la zone à éviter
    insert_ouest_y: Y de l'insert ouest de boitier_ps (doit rester égal à puit_y_nord)
    insert_sud_x: bord est de l'insert sud de boitier_ps (doit rester égal à
        longueur_interne − 10)
    vis_diametre: passage des deux vis M3, dans les inserts ; par défaut la
        cote mesurée `vis_passage`
    """
    wall = epaisseur_paroi
    inner_x = longueur_interne
    inner_y = largeur_interne
    jeu = gouttiere_jeu
    ep = gouttiere_epaisseur
    prof = gouttiere_profondeur
    if vis_diametre is None:
        vis_diametre = measured("vis_passage")

    if wall < 1.0:
        reject(f"epaisseur_paroi {wall} is under 1 mm: raise it", param="epaisseur_paroi")
    if inner_x < 10.0 or inner_y < 10.0:
        reject(
            f"longueur_interne {inner_x} / largeur_interne {inner_y} too small: raise them",
            param="longueur_interne",
        )
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
    if insert_diametre < 2.0:
        reject(
            f"insert_diametre {insert_diametre} is under 2 mm: raise it",
            param="insert_diametre",
        )
    if vis_diametre < 2.0:
        reject(f"vis_diametre {vis_diametre} is under 2 mm: raise it", param="vis_diametre")

    # Plate matches the box's outer footprint exactly: it must not overhang.
    plate_x0, plate_y0 = -wall, -wall
    plate_x1, plate_y1 = inner_x + wall, inner_y + wall

    body = Pos(plate_x0, plate_y0, 0) * Box(
        plate_x1 - plate_x0, plate_y1 - plate_y0, wall, align=_AMIN
    )

    # Inner rim only: inside face of the box wall (the cavity), `jeu` clear
    # of it. Nothing sits outside the wall.
    body = body + _cadre(
        jeu, jeu, inner_x - jeu, inner_y - jeu, ep, wall, prof
    )

    # Heat-insert corbels are flush with the rim on boitier_ps: the inner
    # rib would run straight into their pads. Clear the inner rib only,
    # over each pad's footprint (insert_along wide, insert_plat deep).
    insert_along = insert_diametre + 2.0 * wall
    insert_half = insert_along / 2.0
    insert_plat = insert_diametre + wall
    margin = 0.5
    body = body - (
        Pos(-margin, insert_ouest_y - insert_half, wall - 0.5)
        * Box(
            insert_plat + margin,
            insert_along,
            prof + 1.0,
            align=_AMIN,
        )
    )
    body = body - (
        Pos(
            insert_sud_x - insert_along,
            -margin,
            wall - 0.5,
        )
        * Box(insert_along, insert_plat + margin, prof + 1.0, align=_AMIN)
    )

    # Two M3 clearance holes, straight through the plate, over the same
    # bores as boitier_ps's heat inserts.
    insert_r = insert_diametre / 2.0
    vis_ouest = (insert_r, insert_ouest_y)
    vis_sud = (insert_sud_x - wall - insert_r, insert_r)
    for cx, cy in (vis_ouest, vis_sud):
        body = body - Pos(cx, cy, -0.5) * Cylinder(
            vis_diametre / 2.0, wall + 1.0, align=_CMIN
        )

    # Placing the lid means flipping it over (about a north-south axis, the
    # long way): that mirrors X. So the west heat-insert bay has to sit on
    # the EAST side of the as-printed/as-modelled part, to land back on the
    # west once flipped onto boitier_ps. Mirror the whole part in X here
    # rather than mirror every X coordinate above by hand.
    mid_x = inner_x / 2.0
    mirror_x = Plane(origin=(mid_x, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
    body = mirror(body, about=mirror_x)

    if draft:
        return body

    bed = body.bounding_box().min.Z

    def keep(edge):
        bb = edge.bounding_box()
        return bb.min.Z <= bed + 0.05 or bb.min.Z >= wall + prof - 0.05

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 0.6)

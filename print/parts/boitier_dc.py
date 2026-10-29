from nurb import *

from system import (
    VERROUS_HT396,
    AMIN,
    INSERT_M2,
    INSERT_M3,
    _fuse_one,
    add_corbel_site,
    add_heat_insert_against,
    add_well,
    carte_plug_rect,
    carte_trous,
    corbel_site,
    offset_in,
)
from parts.boitier_ac import languettes

CORBEL_LONGUEUR = INSERT_M3.diametre_percage + 2 * INSERT_M3.epaisseur_paroi_min

# Gap around each boitier_ac tongue in its notch, and how far the notch floor
# drops under the tongue so boitier_ac rests on the metal plate, not on the muret.
_JEU_LANGUETTE = 0.2
_LANGUETTE_DEGAGEMENT = 0.1
# Magnet wells, board frame: 24 V side, sensors side, and the neck under the XIAO.
_PUITS = ((12.0, 20.0), (38.0, 16.0), (37.0, 44.0))


def dc_contour(wall):
    """Lower part of the board plus one wall, with the 24 V wing to the west.
    The north side meets boitier_ac at the south of its outline."""
    east = measured("combined_dc_largeur") + 2 * wall
    north = measured("combined_ac_y")
    neck_x = measured("combined_ac_x")
    wing_y = measured("combined_ouest_y") + 2 * wall
    return [
        (0.0, 0.0),
        (east, 0.0),
        (east, north),
        (neck_x, north),
        (neck_x, wing_y),
        (0.0, wing_y),
    ]


def corbel_sites(wall):
    """Lid corbels: one in the middle of the 24 V wing's west wall, between H4
    and H5, and one on the east wall north of J9. H1 has none: J1 and J8 leave
    no room on the south wall."""
    east = measured("combined_dc_largeur") + wall
    return [
        corbel_site("west", wall, (measured("combined_ouest_y") - CORBEL_LONGUEUR) / 2 + wall),
        corbel_site("east", east, 25.5 + wall),
    ]


def corbels(wall):
    """Hole (cx, cy, d, pad bbox) of each corbel, for the lid."""
    return [site.hole for site in corbel_sites(wall)]


def _open_north(body, wall, floor, hauteur):
    """Open the north side onto boitier_ac, down to a muret level with its
    floor, notched for its tongues."""
    plate = measured("plaque_machine_epaisseur")
    east = measured("combined_dc_largeur") + wall
    north = measured("combined_ac_y")
    west = measured("combined_ac_x") + wall
    muret = plate + floor
    body = body - Pos(west, north - wall, muret) * Box(east - west, wall, hauteur - muret, align=AMIN)
    notch_z = plate - _LANGUETTE_DEGAGEMENT
    for x0, x1 in languettes(wall):
        body = body - Pos(x0 - _JEU_LANGUETTE, north - wall, notch_z) * Box(
            x1 - x0 + 2 * _JEU_LANGUETTE, wall, muret - notch_z, align=AMIN)
    return body


def _plug_notches(body, wall, floor, hauteur, jeu):
    """Notch the south wall over each HT396 header whose plug latch faces
    south (J7, J8): it sticks out past the board edge and through the wall,
    from the bottom of the latch arm up."""
    west_inner, east_inner = wall, measured("combined_dc_largeur") + wall
    pcb_top = (measured("plaque_machine_epaisseur") + floor
               + measured("combined_dessous_pcb") + measured("combined_epaisseur"))
    z0 = pcb_top + measured("ht396v_hauteur") - measured("ht396k_insertion") - jeu
    for name, side in VERROUS_HT396.items():
        if side != "south":
            continue
        x0, _, x1, _ = carte_plug_rect(name, wall)
        x0, x1 = max(x0 - jeu, west_inner), min(x1 + jeu, east_inner)
        body = body - Pos(x0, 0, z0) * Box(x1 - x0, wall, hauteur - z0, align=AMIN)
    return body


def _inserts(body, outer, wall, floor):
    """M2 inserts under H1, H4 and H5, filled out to their wall or corner,
    taller than boitier_ac's by the plate so the PCB sits level across both boxes."""
    height = measured("plaque_machine_epaisseur") + measured("combined_dessous_pcb")
    h1, _, _, h4, h5, _ = carte_trous(wall)
    wing_north = measured("combined_ouest_y") + wall
    body = add_heat_insert_against(body, outer, *h1, (h1[0], wall), floor, height, INSERT_M2)
    body = add_heat_insert_against(body, outer, *h4, (wall, wall), floor, height, INSERT_M2)
    return add_heat_insert_against(body, outer, *h5, (wall, wing_north), floor, height, INSERT_M2)


@part
def boitier_dc(
    hauteur=30.0,
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    jeu_connecteur=1.0,
    draft=False,
):
    """Boîtier DC : partie basse de la carte combined, posé sur la face de la
    machine, 4 mm sous boitier_ac. Nord ouvert sur boitier_ac.

    hauteur: hauteur des murs depuis le lit, boitier_ac + plaque métal
    epaisseur_paroi: épaisseur des murs, vers l'intérieur
    epaisseur_fond: épaisseur du fond, la même que boitier_ac
    jeu_connecteur: marge autour des plugs de J7 et J8 dans les entailles du mur sud
    """
    wall = epaisseur_paroi
    floor = epaisseur_fond

    if wall < 1.2:
        reject(f"epaisseur_paroi {wall} is under 1.2 mm: raise it", param="epaisseur_paroi")
    plate = measured("plaque_machine_epaisseur")
    xiao_top = plate + floor + measured("combined_dessous_pcb") + measured("combined_hauteur_xiao")
    if hauteur < xiao_top:
        reject(
            f"hauteur {hauteur} puts the lid into the XIAO, whose top is at "
            f"{xiao_top:.1f} mm: raise it",
            param="hauteur",
        )

    pts = dc_contour(wall)
    outer = extrude(Polygon(*pts, align=None), hauteur)
    cavity = Pos(0, 0, floor) * extrude(Polygon(*offset_in(pts, wall), align=None), hauteur)
    body = outer - cavity
    body = _open_north(body, wall, floor, hauteur)
    body = _plug_notches(body, wall, floor, hauteur, jeu_connecteur)
    body = _inserts(body, outer, wall, floor)

    for x, y in _PUITS:
        body = add_well(body, outer, x + wall, y + wall)

    for site in corbel_sites(wall):
        body = add_corbel_site(body, site, hauteur)

    body = _fuse_one(body)
    if draft:
        return body

    # Outer convex corners; the two at the north meet boitier_ac and stay sharp.
    east = pts[1][0]
    corners = [(0.0, 0.0), (east, 0.0), pts[5]]

    def keep(edge):
        c = edge.center()
        return any(abs(c.X - x) < 0.01 and abs(c.Y - y) < 0.01 for x, y in corners)

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

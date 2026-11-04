import inspect

from nurb import *

from system import (
    AMIN,
    INSERT_M25,
    _fuse_one,
    add_corbel_site,
    add_wall,
    add_well,
    corbel_site,
    surplomb,
    surplomb_xz,
)

# Wago catch: 1 mm return with a 45° underside, the bottom of its tip 1 mm
# above the terminals so the underside starts at their top edge on the wall.
_SURPLOMB = 1.0
_OVERLAP = 0.4
# The Wago X-rails stand this far from the sill and the bay's north wall.
_RAIL_WAGO_RETRAIT = 5.0
# The west ribs stop this far under the rim, clear of the lid's rim.
_MARGE_REBORD = 4.0
# Gap between an opening and the corbel next to it, to slide the wires under.
_ECART_CORBEAU = 3.0
# The magnet bore bites this far into the east wall: an exactly tangent bore
# leaves slivers.
_MORSURE_PUIT = 0.2
_PAS_BORNIER = 5.0


def disposition(
    wall,
    floor,
    air_ouest,
    air_fond,
    air_couvercle,
    ecart_alim_wago,
    passage_fils_sud,
    passage_fils_nord,
    decalage_wago_nord,
):
    """Every X, Y and Z of the layout, origin at the inner south-west corner,
    Z from the bed."""
    longueur = measured("alim24_longueur")
    largeur = measured("alim24_largeur")
    epaisseur = measured("alim24_hauteur")
    wago_x = 2 * measured("wago_epaisseur")
    wago_y = measured("wago_profondeur")

    d = {}
    d["alim_x0"] = air_ouest
    d["alim_x1"] = air_ouest + epaisseur
    d["wago_mur_x0"] = d["alim_x1"] + ecart_alim_wago
    d["wago_x0"] = d["wago_mur_x0"] + wall
    d["wago_x1"] = d["wago_x0"] + wago_x
    d["inner_x"] = d["wago_x1"]

    # The two wire passages count their muret.
    d["alim_y0"] = passage_fils_sud
    d["alim_y1"] = d["alim_y0"] + longueur
    d["inner_y"] = d["alim_y1"] + passage_fils_nord

    # Sill, terminals and north wall, centred on the cavity.
    baie_y = wall + wago_y + wall
    d["seuil_y0"] = (d["inner_y"] - baie_y) / 2.0 + decalage_wago_nord
    d["wago_y0"] = d["seuil_y0"] + wall
    d["wago_y1"] = d["wago_y0"] + wago_y

    d["alim_z0"] = floor + air_fond
    d["alim_z1"] = d["alim_z0"] + largeur
    d["hauteur"] = d["alim_z1"] + air_couvercle
    # 10 % of the supply length from each end.
    d["rails_alim_y"] = (
        d["alim_y0"] + 0.1 * longueur,
        d["alim_y1"] - 0.1 * longueur,
    )
    return d


def _add_supply_holders(body, outer, d, wall, floor, air_ouest, air_fond,
                        ecart_alim_wago, hauteur_muret_sud, hauteur_berceau,
                        hauteur_mur_nord):
    """West ribs, floor rails ending in a cradle lip, the south muret against
    the open end of the U, and the full-width north wall, under the 24 V wires."""
    for y in d["rails_alim_y"]:
        y0 = y - wall / 2.0
        body = add_wall(body, outer, 0.0, y0, air_ouest, wall,
                        floor, d["hauteur"] - _MARGE_REBORD - floor)
        body = add_wall(body, outer, 0.0, y0, d["alim_x1"], wall, floor, air_fond)
        body = add_wall(body, outer, d["alim_x1"], y0, ecart_alim_wago, wall,
                        floor, hauteur_berceau)
    body = add_wall(body, outer, 0.0, d["alim_y0"] - wall, d["alim_x1"], wall,
                    floor, hauteur_muret_sud)
    return add_wall(body, outer, 0.0, d["alim_y1"], d["inner_x"], wall,
                    floor, hauteur_mur_nord)


def _add_wago_bay(body, outer, d, wall, floor, hauteur_rails_wago, hauteur_seuil,
                  surplomb_ouest):
    """Two 221-415 on edge, openings south: west support wall, south sill,
    north wall with its catch along the bay, a second catch `surplomb_ouest`
    long on the support wall at the north-west corner, two X-rails underneath."""
    wago_z = floor + hauteur_rails_wago + measured("wago_415_largeur")
    catch_z = wago_z + _SURPLOMB
    wall_h = catch_z + _SURPLOMB - floor
    x0, x1 = d["wago_x0"], d["wago_x1"]
    body = add_wall(body, outer, d["wago_mur_x0"], d["seuil_y0"], wall,
                    d["wago_y1"] + wall - d["seuil_y0"], floor, wall_h)
    body = add_wall(body, outer, x0, d["seuil_y0"], x1 - x0, wall,
                    floor, hauteur_rails_wago + hauteur_seuil)
    body = add_wall(body, outer, x0, d["wago_y1"], x1 - x0, wall, floor, wall_h)
    for y0 in (d["wago_y0"] + _RAIL_WAGO_RETRAIT,
               d["wago_y1"] - _RAIL_WAGO_RETRAIT - wall):
        body = add_wall(body, outer, x0, y0, x1 - x0, wall, floor, hauteur_rails_wago)
    body = body + surplomb_xz(
        x0, d["wago_y1"] + wall, catch_z, _SURPLOMB, surplomb_ouest + wall,
        0.0, inverse=True, overlap=_OVERLAP,
    )
    return body + surplomb(
        d["wago_y1"], x1, catch_z, _SURPLOMB, x1 - x0,
        plane=Plane.YZ, overlap=_OVERLAP,
    )


def _wells(d, wall, recul_puits):
    """Two magnet wells against the east wall, south and north of the Wago bay,
    `recul_puits` off the sill and the bay's north wall."""
    bore = (measured("aimant_diametre") + measured("aimant_puit_press_fit")) / 2.0
    pad = bore + measured("aimant_puit_mur")
    x = d["inner_x"] - bore + _MORSURE_PUIT
    return (
        (x, d["seuil_y0"] - recul_puits - pad),
        (x, d["wago_y1"] + wall + recul_puits + pad),
    )


def _openings(d, wall, floor, largeur_ouverture, seuil_ouvertures_sud):
    """Cable slots open to the rim: (x0, y0, dx, dy, z0). The two north ones
    run to the floor (CAN and 5 V on it, 24 V out east); the machine and
    combined AC ones start at `seuil_ouvertures_sud`."""
    w, m = largeur_ouverture, 0.5
    through = wall + 2 * m
    z_sud = floor + seuil_ouvertures_sud
    ix, iy = d["inner_x"], d["inner_y"]
    return [
        (-wall - m, 0.0, through, w, z_sud),
        (ix - w, -wall - m, w, through, z_sud),
        (ix - m, iy - w, through, w, floor),
        (-wall - m, iy - w, through, w, floor),
    ]


def _corbel_sites(d, largeur_ouverture):
    """M2.5 lid corbels: on the north wall east of the north-west slot, and on
    the south wall west of the south-east slot."""
    along = INSERT_M25.diametre_percage + 2 * INSERT_M25.epaisseur_paroi_min
    return [
        corbel_site("north", d["inner_y"], largeur_ouverture + _ECART_CORBEAU,
                    INSERT_M25),
        corbel_site("south", 0.0,
                    d["inner_x"] - largeur_ouverture - _ECART_CORBEAU - along,
                    INSERT_M25),
    ]


@part
def boitier_ps_24v(
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    air_ouest=1.0,
    air_fond=1.0,
    air_couvercle=1.0,
    ecart_alim_wago=2.5,
    passage_fils_sud=8.0,
    passage_fils_nord=15.0,
    hauteur_berceau=4.0,
    hauteur_muret_sud=4.0,
    hauteur_mur_nord=22.0,
    hauteur_rails_wago=2.0,
    hauteur_seuil_wago=1.0,
    surplomb_ouest_wago=8.0,
    decalage_wago_nord=0.0,
    recul_puits=5.0,
    largeur_ouverture=5.0,
    seuil_ouvertures_sud=15.0,
    draft=False,
):
    """Boîtier de l'alimentation 24 V sur sa tranche, composants vers l'est,
    deux Wago 221-415 sur leur tranche à l'est, ouvertures vers le sud.

    epaisseur_paroi: murs de pourtour et murets
    epaisseur_fond: fond, contre la face de la machine
    air_ouest: air entre la plaque alu et le mur ouest, épaisseur des nervures
    air_fond: air sous l'alimentation, hauteur des rails du fond
    air_couvercle: air entre le haut de l'alimentation et le couvercle
    ecart_alim_wago: écart entre l'alimentation et le mur ouest des Wago
    passage_fils_sud: du bout sud de l'alimentation au mur sud, muret compris
    passage_fils_nord: du bout nord de l'alimentation au mur nord, mur de 22 mm compris
    hauteur_berceau: hauteur des rails du fond à l'est de l'alimentation
    hauteur_muret_sud: muret qui tient l'alimentation au sud
    hauteur_mur_nord: mur qui tient l'alimentation au nord, sous les fils 24 V
    hauteur_rails_wago: rails sous les Wago, loin de la face chaude
    hauteur_seuil_wago: seuil au sud des Wago, au-dessus des rails
    surplomb_ouest_wago: longueur du retour sur le mur ouest des Wago, depuis le mur nord
    decalage_wago_nord: déplace le logement des Wago vers le nord
    recul_puits: écart entre les puits d'aimant et le logement des Wago
    largeur_ouverture: largeur des quatre fentes de câble
    seuil_ouvertures_sud: bas des deux fentes sud, depuis le fond
    """
    wall = epaisseur_paroi
    floor = epaisseur_fond
    if wall < 1.2:
        reject(f"epaisseur_paroi {wall} is under 1.2 mm: raise it", param="epaisseur_paroi")
    for name, passage in (("passage_fils_sud", passage_fils_sud),
                          ("passage_fils_nord", passage_fils_nord)):
        if passage < wall + 3.0:
            reject(f"{name} {passage} leaves under 3 mm past the muret: raise it",
                   param=name)
    d = disposition(wall, floor, air_ouest, air_fond, air_couvercle, ecart_alim_wago,
                    passage_fils_sud, passage_fils_nord, decalage_wago_nord)

    fil_bas = (d["alim_z0"] + measured("alim24_bornier_centre")
               - _PAS_BORNIER / 2.0 - measured("fil_diametre") / 2.0)
    if floor + hauteur_mur_nord > fil_bas:
        reject(
            f"hauteur_mur_nord {hauteur_mur_nord} reaches the lower 24 V wire at "
            f"{fil_bas:.1f} mm from the bed: lower it under {fil_bas - floor:.1f}",
            param="hauteur_mur_nord",
        )

    ix, iy, hauteur = d["inner_x"], d["inner_y"], d["hauteur"]
    outer = Pos(-wall, -wall, 0) * Box(ix + 2 * wall, iy + 2 * wall, hauteur, align=AMIN)
    body = outer - Pos(0, 0, floor) * Box(ix, iy, hauteur, align=AMIN)

    body = _add_supply_holders(body, outer, d, wall, floor, air_ouest, air_fond,
                               ecart_alim_wago, hauteur_muret_sud, hauteur_berceau,
                               hauteur_mur_nord)
    body = _add_wago_bay(body, outer, d, wall, floor, hauteur_rails_wago,
                         hauteur_seuil_wago, surplomb_ouest_wago)
    bay = Pos(d["wago_x0"], d["wago_y0"], 0) * Box(
        d["wago_x1"] - d["wago_x0"], d["wago_y1"] - d["wago_y0"], hauteur, align=AMIN)
    for x, y in _wells(d, wall, recul_puits):
        body = add_well(body, outer - bay, x, y)
    for site in _corbel_sites(d, largeur_ouverture):
        body = add_corbel_site(body, site, hauteur)

    slots = _openings(d, wall, floor, largeur_ouverture, seuil_ouvertures_sud)
    for x0, y0, dx, dy, z0 in slots:
        body = body - Pos(x0, y0, z0) * Box(dx, dy, hauteur - z0 + 1.0, align=AMIN)

    body = _fuse_one(body)
    if draft:
        return body

    def in_slot(edge):
        b = edge.bounding_box()
        if b.diagonal < 2.0:
            return False
        return any(
            b.min.X > x0 - 0.1 and b.max.X < x0 + dx + 0.1
            and b.min.Y > y0 - 0.1 and b.max.Y < y0 + dy + 0.1
            and b.min.Z > z0 - 0.1
            for x0, y0, dx, dy, z0 in slots
        )

    corners = [(-wall, -wall), (ix + wall, -wall), (ix + wall, iy + wall), (-wall, iy + wall)]

    def outer_corner(edge):
        c = edge.center()
        return any(abs(c.X - x) < 0.01 and abs(c.Y - y) < 0.01 for x, y in corners)

    body = polish(body, body.edges().filter_by(in_slot), 0.6)
    return polish(body, body.edges().filter_by(Axis.Z).filter_by(outer_corner), 1.0)


def reperes():
    """Layout, outer outline and lid corbel holes at the part's defaults, for
    `couvercle_ps_24v`."""
    p = {k: v.default for k, v in inspect.signature(boitier_ps_24v).parameters.items()}
    wall = p["epaisseur_paroi"]
    d = disposition(wall, p["epaisseur_fond"], p["air_ouest"], p["air_fond"],
                    p["air_couvercle"], p["ecart_alim_wago"], p["passage_fils_sud"],
                    p["passage_fils_nord"], p["decalage_wago_nord"])
    ix, iy = d["inner_x"], d["inner_y"]
    contour = [(-wall, -wall), (ix + wall, -wall), (ix + wall, iy + wall), (-wall, iy + wall)]
    trous = [site.hole for site in _corbel_sites(d, p["largeur_ouverture"])]
    return d, contour, trous, wall

from nurb import *

from system import INSERT_M3, anti_tirage_ew, anti_tirage_ns, add_well


@part
def boitier_ps(
    longueur_interne=50.0,
    largeur_interne=87.6,
    hauteur=40.0,
    epaisseur_paroi=1.6,
    ouverture_bas=5.0,
    ouverture_gauche=5.0,
    ouverture_gauche_haut=5.0,
    ouverture_z=15.0,
    alim_x=21.8,
    alim_y=27.8,
    alim_z=35.0,
    surplus_nord=17.6,
    reglette_alim_x=28.2,
    reglette_alim_z=2.0,
    muret_batterie_z=32.5,
    wago_entre_murets=19.0,
    wago_largeur=29.8,
    wago_plateforme_z=15.0,
    wago_rebord_z=15.0,
    wago_rail_ecart=5.0,
    wago_muret_z=32.5,
    wago_surplomb=1.0,
    wago_surplomb_longueur=8.0,
    puit_x=10.0,
    puit_y_sud=30.0,
    puit_y_nord=60.0,
    nord_wago_z=19.8,
    nord_wago_x=18.6,
    insert_diametre=INSERT_M3.diametre_percage,
    insert_profondeur=INSERT_M3.profondeur_min,
    anti_tirage_jeu=1.2,
    anti_tirage_largeur=3.0,
    anti_tirage_bords=5.6,
    anti_tirage_z=5.0,
    anti_tirage_x=10.0,
    largeur_passage_cable=5.0,
    draft=False,
):
    """Boîtier alim : cavité 50 × 87,6 mm, berceau PSU, 221-415 sud et 221-423 nord.

    longueur_interne: cavité en X, murs non compris
    largeur_interne: cavité en Y, murs non compris (70 + 17,6 au nord)
    hauteur: murs de pourtour, depuis le lit
    epaisseur_paroi: fond et murs, 1,6 mm
    ouverture_bas: trou dans le mur Y min, 5 mm, calé à droite (X max)
    ouverture_gauche: trou dans le mur X min, depuis Y = 0
    ouverture_gauche_haut: largeur du trou nord dans le mur X min, 1,6 mm sous Y max
    ouverture_z: seuil des trois ouvertures, au-dessus du fond
    alim_x: volume alim en X (21,8 mm), du muret ouest au mur X max
    alim_y: volume alim en Y (27,8 mm), du muret batterie au muret nord
    alim_z: volume alim en Z (35 mm), au-dessus du fond ; hauteur du muret nord
    surplus_nord: rajout au nord, au-delà du berceau alim (16 mm mur à mur
        plus le muret nord 1,6 mm)
    reglette_alim_x: flanc droit de la réglette ; 50 − 21,8 = 28,2
    reglette_alim_z: hauteur de la réglette d'alim au-dessus du fond
    muret_batterie_z: hauteur du muret Y = 42,2 (flanc nord), même 32,5 mm que le muret Wago
    wago_entre_murets: vide face à face en Y entre les deux murets, hors épaisseur 1,6 mm
    wago_largeur: largeur du berceau 221-415 ; le flanc droit du muret est à 50 − 29,8
    wago_plateforme_z: hauteur des rails sous les Wago, 15 mm, les deux logements
    wago_rebord_z: rebord devant les Wago, les deux logements (15 mm = affleurant la plateforme)
    wago_rail_ecart: jeu entre chaque rail et les faces latérales, les deux logements
    wago_muret_z: muret ouest du berceau sud, 32,5 mm (17,5 au-dessus des rails)
    wago_surplomb: retour 1 mm au sommet, 45° en-dessous, les deux logements
    wago_surplomb_longueur: longueur du retour, 8 mm, les deux logements
    puit_x: X des deux puits ouest, origine = coin intérieur sud-ouest
    puit_y_sud: Y du puits ouest bas (10, 30)
    puit_y_nord: Y du puits ouest haut (10, 60)
    nord_wago_z: volume Z des 221-423 du logement nord, au-dessus de la plateforme
        (18,8 mm de borne + 1 mm sous le cran)
    nord_wago_x: profondeur du logement nord en X (221-423, sorties ouest)
    insert_diametre: alésage du heat insert, collé à la face intérieure
    insert_profondeur: profondeur de l'alésage depuis le sommet
    anti_tirage_jeu: jeu entre le U et la face intérieure, serre-fil 1 mm
    anti_tirage_largeur: largeur intérieure du U, serre-fil 2,5 mm
    anti_tirage_bords: largeur hors-tout avec les deux jambes du U
    anti_tirage_z: hauteur du U (le palier vertical)
    anti_tirage_x: 10 mm depuis le coin, chaque anneau (sud depuis l'ouest, est depuis le sud, nord depuis l'ouest)
    largeur_passage_cable: largeur en mm pour passer des câbles au nord du boitier
    """
    wall = epaisseur_paroi
    floor = epaisseur_paroi
    inner_x = longueur_interne
    inner_y = largeur_interne
    if wall < 1.2:
        reject(
            f"epaisseur_paroi {wall} is under 1.2 mm: raise it",
            param="epaisseur_paroi",
        )
    if inner_x < 20.0:
        reject(
            f"longueur_interne {inner_x} is under 20 mm: raise it",
            param="longueur_interne",
        )
    if inner_y < 40.0:
        reject(
            f"largeur_interne {inner_y} is under 40 mm: raise it",
            param="largeur_interne",
        )
    if hauteur < floor + 8.0:
        reject(
            f"hauteur {hauteur} leaves under 8 mm of wall above the floor: "
            f"raise it above {floor + 8.0:.1f}",
            param="hauteur",
        )
    if ouverture_bas < 2.0 or ouverture_bas > inner_x - 2.0:
        reject(
            f"ouverture_bas {ouverture_bas} must fit in the {inner_x} mm face "
            f"with 2 mm of wall on the left",
            param="ouverture_bas",
        )
    if ouverture_gauche < 2.0 or ouverture_gauche > inner_y - 4.0:
        reject(
            f"ouverture_gauche {ouverture_gauche} must fit on the {inner_y} mm "
            f"side with wall above it",
            param="ouverture_gauche",
        )
    if ouverture_gauche_haut < 2.0:
        reject(
            f"ouverture_gauche_haut {ouverture_gauche_haut} is under 2 mm: "
            "raise it",
            param="ouverture_gauche_haut",
        )
    # 1.6 mm of west wall stays at Y max so the north face runs to the corner.
    y_nw_max = inner_y
    y_nw_min = y_nw_max - ouverture_gauche_haut
    if y_nw_min < ouverture_gauche + 4.0:
        reject(
            f"ouverture_gauche_haut {ouverture_gauche_haut} plus the "
            f"{wall} mm north jamb leave under 4 mm of west wall above "
            f"ouverture_gauche {ouverture_gauche}: lower one",
            param="ouverture_gauche_haut",
        )
    if ouverture_z < 2.0:
        reject(
            f"ouverture_z {ouverture_z} is under 2 mm: raise it",
            param="ouverture_z",
        )
    if floor + ouverture_z + 2.0 > hauteur:
        reject(
            f"ouverture_z {ouverture_z} leaves under 2 mm of opening below "
            f"hauteur {hauteur}: lower it",
            param="ouverture_z",
        )
    if surplus_nord < wall:
        reject(
            f"surplus_nord {surplus_nord} is under the {wall} mm north PSU "
            f"wall: raise it",
            param="surplus_nord",
        )
    nord_clair = surplus_nord - wall
    if nord_clair < 16.0:
        reject(
            f"surplus_nord {surplus_nord} leaves {nord_clair:.1f} mm "
            "wall-to-wall in the north Wago bay, under 16 mm: raise it",
            param="surplus_nord",
        )
    if alim_z < 1.0:
        reject(
            f"alim_z {alim_z} is under 1 mm: raise it",
            param="alim_z",
        )
    if alim_z + floor > hauteur:
        reject(
            f"alim_z {alim_z} with a {floor} mm floor exceeds hauteur "
            f"{hauteur}: lower it",
            param="alim_z",
        )
    if reglette_alim_x < wall or reglette_alim_x > inner_x - alim_x:
        reject(
            f"reglette_alim_x {reglette_alim_x} must leave {alim_x} mm for the "
            f"PSU before X = {inner_x:.1f}",
            param="reglette_alim_x",
        )
    y_alim_nord = inner_y - surplus_nord
    y_psu_n = y_alim_nord + wall
    y_batt = y_alim_nord - alim_y
    wago_reglette_z = wago_rebord_z
    x_nord = inner_x - nord_wago_x
    if y_batt < ouverture_gauche + 4.0:
        reject(
            f"alim_y {alim_y} pushes the battery wall onto the left opening: "
            f"lower it",
            param="alim_y",
        )
    # North face of the battery wall (Y = 42.2), thickness toward −Y. Clear gap then
    # the north face of the lower wall: 19.0 mm face-to-face.
    y_wago = y_batt - wall - wago_entre_murets
    if y_wago < wall:
        reject(
            f"wago_entre_murets {wago_entre_murets} puts the Wago wall under "
            f"the floor: lower it",
            param="wago_entre_murets",
        )
    x_wago = inner_x - wago_largeur
    if x_wago < wall:
        reject(
            f"wago_largeur {wago_largeur} is wider than the {inner_x} mm "
            f"cavity: lower it",
            param="wago_largeur",
        )
    if wago_plateforme_z < 1.0:
        reject(
            f"wago_plateforme_z {wago_plateforme_z} is under 1 mm: raise it",
            param="wago_plateforme_z",
        )
    if floor + wago_plateforme_z + 4.0 > hauteur:
        reject(
            f"wago_plateforme_z {wago_plateforme_z} leaves under 4 mm of "
            f"opening below hauteur {hauteur}: lower it",
            param="wago_plateforme_z",
        )
    if wago_rail_ecart < 1.0:
        reject(
            f"wago_rail_ecart {wago_rail_ecart} is under 1 mm: the rails "
            "would stiffen the side walls. Raise it",
            param="wago_rail_ecart",
        )
    if 2.0 * (wago_rail_ecart + wall) > wago_largeur - 2.0:
        reject(
            f"wago_rail_ecart {wago_rail_ecart} leaves the two rails less "
            f"than 2 mm apart in the {wago_largeur} mm south bay: lower it",
            param="wago_rail_ecart",
        )
    if 2.0 * (wago_rail_ecart + wall) > nord_clair - 2.0:
        reject(
            f"wago_rail_ecart {wago_rail_ecart} leaves the two rails less "
            f"than 2 mm apart in the {nord_clair:.1f} mm north bay: lower it",
            param="wago_rail_ecart",
        )
    if wago_rebord_z < wago_plateforme_z:
        reject(
            f"wago_rebord_z {wago_rebord_z} is under the {wago_plateforme_z} mm "
            "platform: raise it",
            param="wago_rebord_z",
        )
    if wago_rebord_z > wago_muret_z:
        reject(
            f"wago_rebord_z {wago_rebord_z} exceeds wago_muret_z "
            f"{wago_muret_z}: lower it",
            param="wago_rebord_z",
        )
    if wago_muret_z < wago_plateforme_z + 8.0:
        reject(
            f"wago_muret_z {wago_muret_z} leaves under 8 mm of wall above the "
            f"{wago_plateforme_z} mm rails: raise it",
            param="wago_muret_z",
        )
    if wago_muret_z + floor > hauteur:
        reject(
            f"wago_muret_z {wago_muret_z} with a {floor} mm floor exceeds "
            f"hauteur {hauteur}: lower it",
            param="wago_muret_z",
        )
    if wago_surplomb < 0.8:
        reject(
            f"wago_surplomb {wago_surplomb} is under 0.8 mm: raise it",
            param="wago_surplomb",
        )
    if wago_surplomb_longueur < 2.0:
        reject(
            f"wago_surplomb_longueur {wago_surplomb_longueur} is under 2 mm: "
            f"raise it",
            param="wago_surplomb_longueur",
        )
    if wago_surplomb_longueur > wago_entre_murets - 2.0:
        reject(
            f"wago_surplomb_longueur {wago_surplomb_longueur} leaves under "
            f"2 mm to insert the Wagos at an angle: lower it",
            param="wago_surplomb_longueur",
        )
    if reglette_alim_z < 0.8:
        reject(
            f"reglette_alim_z {reglette_alim_z} is under 0.8 mm: raise it",
            param="reglette_alim_z",
        )
    if muret_batterie_z < 1.0:
        reject(
            f"muret_batterie_z {muret_batterie_z} is under 1 mm: raise it",
            param="muret_batterie_z",
        )
    if muret_batterie_z < wago_plateforme_z:
        reject(
            f"muret_batterie_z {muret_batterie_z} is under the "
            f"{wago_plateforme_z} mm rails: raise it",
            param="muret_batterie_z",
        )
    if muret_batterie_z + floor > hauteur:
        reject(
            f"muret_batterie_z {muret_batterie_z} with a {floor} mm floor exceeds "
            f"hauteur {hauteur}: lower it",
            param="muret_batterie_z",
        )

    if nord_wago_z < 8.0:
        reject(
            f"nord_wago_z {nord_wago_z} is under 8 mm: raise it",
            param="nord_wago_z",
        )
    if floor + wago_plateforme_z + nord_wago_z > hauteur:
        reject(
            f"nord_wago_z {nord_wago_z} plus the {wago_plateforme_z} mm "
            f"platform exceeds hauteur {hauteur}: lower it",
            param="nord_wago_z",
        )
    if nord_wago_x < 8.0:
        reject(
            f"nord_wago_x {nord_wago_x} is under 8 mm: raise it",
            param="nord_wago_x",
        )
    if nord_wago_x > alim_x:
        reject(
            f"nord_wago_x {nord_wago_x} is longer than the {alim_x} mm PSU "
            "north wall, so the muret would miss it: lower it",
            param="nord_wago_x",
        )
    if wago_surplomb_longueur > nord_wago_x - 2.0:
        reject(
            f"wago_surplomb_longueur {wago_surplomb_longueur} leaves under "
            f"2 mm to tilt the north Wagos in: lower it",
            param="wago_surplomb_longueur",
        )
    if insert_diametre < 2.0:
        reject(
            f"insert_diametre {insert_diametre} is under 2 mm: raise it",
            param="insert_diametre",
        )
    if insert_profondeur < 2.0:
        reject(
            f"insert_profondeur {insert_profondeur} is under 2 mm: raise it",
            param="insert_profondeur",
        )
    insert_plat = insert_diametre + wall
    insert_along = insert_diametre + 2.0 * wall
    z_insert = hauteur - insert_profondeur
    z_insert_45 = z_insert - insert_plat
    if z_insert_45 < floor + 8.0:
        reject(
            f"insert_profondeur {insert_profondeur} plus the 45° corbel "
            f"leaves under 8 mm of wall above the floor: lower it",
            param="insert_profondeur",
        )
    if puit_y_nord - 0.5 * insert_along < ouverture_gauche + 2.0:
        reject(
            f"puit_y_nord {puit_y_nord} puts the west insert onto the left "
            "opening: raise it",
            param="puit_y_nord",
        )
    if puit_y_nord + 0.5 * insert_along > y_nw_min - 2.0:
        reject(
            f"puit_y_nord {puit_y_nord} puts the west insert onto the "
            "north-west opening: lower it",
            param="puit_y_nord",
        )
    if anti_tirage_jeu < 1.0:
        reject(
            f"anti_tirage_jeu {anti_tirage_jeu} will not pass a 1 mm "
            "cable tie: raise it",
            param="anti_tirage_jeu",
        )
    if anti_tirage_largeur < 2.5:
        reject(
            f"anti_tirage_largeur {anti_tirage_largeur} is tighter than "
            "the 2.5 mm cable tie: raise it",
            param="anti_tirage_largeur",
        )
    if anti_tirage_z < 2.0:
        reject(
            f"anti_tirage_z {anti_tirage_z} is under 2 mm: raise it",
            param="anti_tirage_z",
        )
    at_leg = 0.5 * (anti_tirage_bords - anti_tirage_largeur)
    if at_leg < 1.2:
        reject(
            f"anti_tirage_bords {anti_tirage_bords} leaves only {at_leg:.1f} mm "
            f"each side of the {anti_tirage_largeur} mm slot: raise it",
            param="anti_tirage_bords",
        )
    if anti_tirage_x < 0.0:
        reject(
            f"anti_tirage_x {anti_tirage_x} is negative: raise it",
            param="anti_tirage_x",
        )
    if anti_tirage_x + anti_tirage_bords > inner_x - ouverture_bas:
        reject(
            f"anti_tirage_x {anti_tirage_x} plus anti_tirage_bords "
            f"{anti_tirage_bords} hits the south opening: lower them",
            param="anti_tirage_x",
        )
    if anti_tirage_x + anti_tirage_bords > inner_y - wall:
        reject(
            f"anti_tirage_x {anti_tirage_x} plus anti_tirage_bords "
            f"{anti_tirage_bords} hits the north-west opening: lower them",
            param="anti_tirage_x",
        )
    at_out = anti_tirage_jeu + wall
    at_bot = floor + ouverture_z - at_out
    at_top = floor + ouverture_z + anti_tirage_z + at_out
    if at_bot < floor + 0.4:
        reject(
            f"anti_tirage_z {anti_tirage_z} plus the 45° ramps hit the "
            "floor: lower ouverture_z or anti_tirage_z",
            param="anti_tirage_z",
        )
    if at_top > hauteur:
        reject(
            f"anti_tirage_z {anti_tirage_z} plus the 45° ramps stick out "
            f"above hauteur {hauteur}: lower it",
            param="anti_tirage_z",
        )

    amin = (Align.MIN, Align.MIN, Align.MIN)
    overlap = 0.4
    margin = 0.5
    outer_x = inner_x + 2.0 * wall
    outer_y = inner_y + 2.0 * wall

    body = Pos(-wall, -wall, 0) * Box(outer_x, outer_y, hauteur, align=amin)
    body = body - (
        Pos(0, 0, floor) * Box(inner_x, inner_y, hauteur, align=amin)
    )

    # Cable openings start 25 mm above the floor, not at the platform and
    # not at the bed.
    z_ouv = floor + ouverture_z
    # Bottom wall (Y min): 5 mm opening flush with inner X max (right).
    ouv_x0 = inner_x - ouverture_bas
    body = body - (
        Pos(ouv_x0, -wall - margin, z_ouv)
        * Box(ouverture_bas, wall + 2.0 * margin, hauteur, align=amin)
    )
    # Left wall (X min): 5 mm from inner Y = 0, and 5 mm ending 1.6 mm
    # south of Y max so the north wall meets the west wall.
    body = body - (
        Pos(-wall - margin, 0, z_ouv)
        * Box(wall + 2.0 * margin, ouverture_gauche, hauteur, align=amin)
    )
    body = body - (
        Pos(-wall - margin, y_nw_min, z_ouv)
        * Box(wall + 2.0 * margin, ouverture_gauche_haut, hauteur, align=amin)
    )

    # One cable-tie U per opening, 10 mm from the corner, same Z as the
    # sill. Shared `anti_tirage_*` from system.py.
    at_u0 = z_ouv
    at_u1 = at_u0 + anti_tirage_z
    at_span = anti_tirage_bords
    at_x0 = anti_tirage_x
    at_y0 = anti_tirage_x
    kw = dict(
        wall=wall,
        jeu=anti_tirage_jeu,
        largeur=anti_tirage_largeur,
        bords=anti_tirage_bords,
        hauteur_u=anti_tirage_z,
    )
    body = body + anti_tirage_ns(at_x0, 0.0, at_u0, True, **kw)
    body = body + anti_tirage_ew(at_y0, inner_x, at_u0, False, **kw)
    body = body + anti_tirage_ns(at_x0, inner_y, at_u0, False, **kw)

    # PSU réglette: right face at X = 28.2, along alim_y only, 2 mm above the floor.
    body = body + (
        Pos(reglette_alim_x - wall, y_batt, floor - overlap)
        * Box(
            wall,
            alim_y + overlap,
            reglette_alim_z + overlap,
            align=amin,
        )
    )
    # PSU north wall: south face at Y = 70, only alim_x wide, alim_z high.
    body = body + (
        Pos(reglette_alim_x - overlap, y_alim_nord, floor - overlap)
        * Box(
            alim_x + overlap + wall,
            wall,
            alim_z + overlap,
            align=amin,
        )
    )
    # Battery wall: north face at Y = 42.2, from the Wago wall's outer face
    # to X max, 32.5 mm. Full-thickness L with the west Wago wall, not a
    # 0.4 mm overlap T.
    x_batt_0 = min(reglette_alim_x, x_wago) - wall
    body = body + (
        Pos(x_batt_0, y_batt - wall, floor - overlap)
        * Box(
            inner_x + wall - x_batt_0,
            wall,
            muret_batterie_z + overlap,
            align=amin,
        )
    )

    # Wago 221-415: 19.0 mm clear south of the battery wall, right face at
    # X = 50 − 29.8 = 20.2. South réglette is flush with the 15 mm platform. The
    # réglette and the west wall meet as a full-thickness L (each runs to
    # the other's outer face) so the outer corner is one edge, not a T.
    body = body + (
        Pos(x_wago - wall, y_wago - wall, floor - overlap)
        * Box(
            inner_x + 2.0 * wall - x_wago,
            wall,
            wago_reglette_z + overlap,
            align=amin,
        )
    )
    body = body + (
        Pos(x_wago - wall, y_wago - wall, floor - overlap)
        * Box(
            wall,
            y_batt - (y_wago - wall),
            wago_muret_z + overlap,
            align=amin,
        )
    )
    # Two Y-rails under the Wago, 15 mm, 5 mm off the west and east walls
    # so those walls can still flex when the terminals go in.
    rail_y0 = y_wago - overlap
    rail_y = (y_batt - wall) - y_wago + 2.0 * overlap
    x_rail_ouest = x_wago + wago_rail_ecart
    x_rail_est = inner_x - wago_rail_ecart - wall
    body = body + (
        Pos(x_rail_ouest, rail_y0, floor - overlap)
        * Box(wall, rail_y, wago_plateforme_z + overlap, align=amin)
    )
    body = body + (
        Pos(x_rail_est, rail_y0, floor - overlap)
        * Box(wall, rail_y, wago_plateforme_z + overlap, align=amin)
    )

    # 1 mm return toward the PSU cradle, 1 mm vertical catch, 45° below.
    # Only 8 mm of the Y-wall, at the north (battery wall), so the Wagos
    # can be tilted in from the south.
    z_top = floor + wago_muret_z
    z_catch = z_top - wago_surplomb
    z_45 = z_catch - wago_surplomb
    hook_pts = [
        (x_wago - overlap, z_45),
        (x_wago + wago_surplomb, z_catch),
        (x_wago + wago_surplomb, z_top),
        (x_wago - overlap, z_top),
    ]
    hook_len = wall + wago_surplomb_longueur
    body = body + (
        Pos(0, y_batt, 0)
        * extrude(Plane.XZ * Polygon(*hook_pts, align=None), hook_len)
    )

    # North Wago bay: two 221-423 on edge, outputs west. 15 mm X-rails
    # fused into the east wall, 5 mm off the north wall and the PSU wall.
    # West muret 15 mm (flush with the platform) running in Y. One catch on
    # the outer north wall, 8 mm from the east, raised by the platform.
    body = body + (
        Pos(x_nord - wall, y_psu_n - overlap, floor - overlap)
        * Box(
            wall,
            nord_clair + 2.0 * overlap,
            wago_reglette_z + overlap,
            align=amin,
        )
    )
    rail_x0 = x_nord - overlap
    rail_x = nord_wago_x + overlap + wall
    y_rail_sud = y_psu_n + wago_rail_ecart
    y_rail_nord = inner_y - wago_rail_ecart - wall
    body = body + (
        Pos(rail_x0, y_rail_sud, floor - overlap)
        * Box(rail_x, wall, wago_plateforme_z + overlap, align=amin)
    )
    body = body + (
        Pos(rail_x0, y_rail_nord, floor - overlap)
        * Box(rail_x, wall, wago_plateforme_z + overlap, align=amin)
    )
    z_nord = floor + wago_plateforme_z + nord_wago_z
    z_nord_catch = z_nord - wago_surplomb
    z_nord_45 = z_nord_catch - wago_surplomb
    x_lip0 = inner_x - wago_surplomb_longueur
    lip_len = wago_surplomb_longueur + wall
    body = body + (
        Pos(x_lip0, 0, 0)
        * extrude(
            Plane.YZ
            * Polygon(
                (inner_y + overlap, z_nord),
                (inner_y - wago_surplomb, z_nord),
                (inner_y - wago_surplomb, z_nord_catch),
                (inner_y + overlap, z_nord_45),
                align=None,
            ),
            lip_len,
        )
    )

    # Heat-insert bays on the wall, not towers from the floor: 45° corbel
    # 5.6 mm in Z (and inboard), then a 5.6 × 7.2 mm pad to the rim with a
    # Ø4 mm bore flush on the inner face (the 1.6 mm wall is the outer wall
    # of the insert). West bay at the north magnet well's Y; south bay's
    # east edge 10 mm from inner X max (not tied to the 5 mm opening).
    insert_r = 0.5 * insert_diametre
    insert_half = 0.5 * insert_along
    cyl_amin = (Align.CENTER, Align.CENTER, Align.MIN)
    west_pts = [
        (-overlap, z_insert_45),
        (0.0, z_insert_45),
        (insert_plat, z_insert),
        (insert_plat, hauteur),
        (-overlap, hauteur),
    ]
    body = body + (
        Pos(0, puit_y_nord + insert_half, 0)
        * extrude(Plane.XZ * Polygon(*west_pts, align=None), insert_along)
    )
    body = body - (
        Pos(insert_r, puit_y_nord, z_insert)
        * Cylinder(insert_r, insert_profondeur + 0.1, align=cyl_amin)
    )
    south_pts = [
        (-overlap, z_insert_45),
        (0.0, z_insert_45),
        (insert_plat, z_insert),
        (insert_plat, hauteur),
        (-overlap, hauteur),
    ]
    x_insert_est = inner_x - 10.0
    body = body + (
        Pos(x_insert_est - insert_along, 0, 0)
        * extrude(Plane.YZ * Polygon(*south_pts, align=None), insert_along)
    )
    body = body - (
        Pos(x_insert_est - wall - insert_r, insert_r, z_insert)
        * Cylinder(insert_r, insert_profondeur + 0.1, align=cyl_amin)
    )

    # Three standing magnet wells, 0.6 mm skin on the bed. One at the Wago
    # bay centre, two at (puit_x, puit_y_sud) and (puit_x, puit_y_nord).
    x_wago_c = x_wago + 0.5 * wago_largeur
    y_wago_c = y_wago + 0.5 * wago_entre_murets
    outer = Pos(-wall, -wall, 0) * Box(outer_x, outer_y, hauteur, align=amin)
    for cx, cy in (
        (x_wago_c, y_wago_c),
        (puit_x, puit_y_sud),
        (puit_x, puit_y_nord),
    ):
        body = add_well(body, outer, cx, cy)

    # Add a channel for wires at the north    
    body = body + (
        Pos(-wall, inner_y+wall, 0) * 
        Box(inner_x+2*wall,largeur_passage_cable+wall,floor,align=amin)
    )
    body = body + (
        Pos(-wall, inner_y+wall+largeur_passage_cable, floor) * 
        Box(inner_x+2*wall,wall,hauteur,align=amin)
    )

    if draft:
        return body

    bed = body.bounding_box().min.Z
    x_min = -wall
    x_max = inner_x + wall
    y_min = -wall
    y_max = inner_y + wall
    # 0.6 mm on both lips of a 1.6 mm wall; 1 mm inner+outer would collide.
    chanfrein_ouv = 0.6

    def in_south_ouv(bb):
        return (
            bb.min.X > ouv_x0 - 0.5
            and bb.max.X < inner_x + 0.5
            and bb.min.Y > y_min - 0.5
            and bb.max.Y < 0.5
            and bb.min.Z > z_ouv - 0.5
        )

    def in_west_sud(bb):
        return (
            bb.min.X > x_min - 0.5
            and bb.max.X < 0.5
            and bb.min.Y > -0.5
            and bb.max.Y < ouverture_gauche + 0.5
            and bb.min.Z > z_ouv - 0.5
        )

    def in_west_nord(bb):
        return (
            bb.min.X > x_min - 0.5
            and bb.max.X < 0.5
            and bb.min.Y > y_nw_min - 0.5
            and bb.max.Y < y_nw_max + 0.5
            and bb.min.Z > z_ouv - 0.5
        )

    def opening_keep(edge):
        bb = edge.bounding_box()
        dx = bb.max.X - bb.min.X
        dy = bb.max.Y - bb.min.Y
        dz = bb.max.Z - bb.min.Z
        # Skip the 1.6 mm through-wall edges: chamfering them after the
        # inner and outer lips leaves a 0.3 mm2 sliver.
        if (dx * dx + dy * dy + dz * dz) ** 0.5 < 2.0:
            return False
        return in_south_ouv(bb) or in_west_sud(bb) or in_west_nord(bb)

    def pont_passage(edge):
        # The two 3 mm edges under each bar, where the tie runs (top and
        # bottom of the inner face, not the side walls).
        bb = edge.bounding_box()
        dx = bb.max.X - bb.min.X
        dy = bb.max.Y - bb.min.Y
        dz = bb.max.Z - bb.min.Z
        span = (dx * dx + dy * dy + dz * dz) ** 0.5
        if abs(span - anti_tirage_largeur) > 0.4:
            return False
        if dz > 0.4:
            return False
        zmid = 0.5 * (bb.min.Z + bb.max.Z)
        if abs(zmid - at_u0) > 0.3 and abs(zmid - at_u1) > 0.3:
            return False
        mx = 0.5 * (bb.min.X + bb.max.X)
        my = 0.5 * (bb.min.Y + bb.max.Y)
        mid = at_x0 + 0.5 * at_span
        y_mid = at_y0 + 0.5 * at_span
        south = abs(my - anti_tirage_jeu) < 0.3 and abs(mx - mid) < 2.0
        est = abs(mx - (inner_x - anti_tirage_jeu)) < 0.3 and abs(my - y_mid) < 2.0
        nord = abs(my - (inner_y - anti_tirage_jeu)) < 0.3 and abs(mx - mid) < 2.0
        return south or est or nord

    def box_keep(edge):
        bb = edge.bounding_box()
        if bb.min.Z < bed - 0.05:
            return False
        mx = 0.5 * (bb.min.X + bb.max.X)
        my = 0.5 * (bb.min.Y + bb.max.Y)
        on_x = abs(mx - x_min) < 0.4 or abs(mx - x_max) < 0.4
        on_y = abs(my - y_min) < 0.4 or abs(my - y_max) < 0.4
        return on_x and on_y

    body = polish(body, body.edges().filter_by(opening_keep), chanfrein_ouv)
    body = polish(body, body.edges().filter_by(pont_passage), 0.4)
    return polish(body, body.edges().filter_by(Axis.Z).filter_by(box_keep), 1.0)

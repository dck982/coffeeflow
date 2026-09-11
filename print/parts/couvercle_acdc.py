import math

from nurb import *

from system import INSERT_M25, INSERT_M3, offset_in, AMIN, CMIN

from parts.boitier_dc import dc_contour, corbels as dc_corbels
from parts.boitier_ac import ac_top_contour, corbels as ac_corbels

def _ring(pts, z0, rdz, rd, w, jeu):
    pts_out = offset_in(pts, w + jeu) 
    pts_in  = offset_in(pts, w + jeu + rd)
    a = Pos(0, 0, z0) * extrude(Polygon(*pts_out, align=None), rdz)
    b = Pos(0, 0, z0) * extrude(Polygon(*pts_in, align=None), rdz)
    return a - b

def _dc_east_opening(body, y, z0, rib_dz):
    """Create a 10x10 opening for cables in the cover, along the east side of the DC boitier"""
    opening_sz = 10
    x = measured("boitier_int_aile_x")-opening_sz
    margin = 1
    return body - (
        Pos(x, y, 0) * Box(opening_sz, opening_sz, z0, align=AMIN)
        +
        Pos(x, y-margin, z0) * Box(opening_sz, opening_sz+margin*2, rib_dz, align=AMIN)
    )

def _pcb_press(body, rib_west, rib_east, y0, y1, appui_pcb_z, wall):
    """PCB press on the the middle 1/3 of a horizontal rib"""
    rib_len_third = (rib_east - rib_west) / 3.0
    appui_x0 = rib_west + rib_len_third
    appui_x1 = appui_x0 + rib_len_third
    return body + Pos(appui_x0, y0, wall) * Box(
        appui_x1 - appui_x0, y1 - y0, appui_pcb_z, align=AMIN
    )


@part
def couvercle_acdc(
    epaisseur_paroi=1.68,
    epaisseur_fond=1.6,
    gouttiere_jeu=0.3,
    gouttiere_epaisseur=1.2,
    gouttiere_profondeur=3.0,
    chanfrein=15.0,
    ac_west_shift=5.0,
    ac_east_shift=1.5,
    ac_south_shift=4.0,
    ac_appui_pcb_hauteur=6.0,
    dc_appui_xiao_hauteur=6.0,
    vis_m25_diametre=2.9,
    vis_m3_diametre=None,
    draft=False,
):
    """Couvercle unique pour `ensemble_boitiers` (boitier_dc + boitier_ac).

    La plaque a exactement le contour extérieur des deux boîtes réunies
    (elle ne déborde pas). Le rebord intérieur, lui, est calculé séparément
    pour chaque boîte, à partir de son propre contour : boitier_dc et
    boitier_ac gardent chacun leur propre boucle fermée, visibles
    séparément, qui ne se rejoignent jamais. À la jonction (face est de
    boitier_dc contre face ouest de boitier_ac, collées sans jeu), les deux
    boucles passent l'une à côté de l'autre sans se toucher : les coller
    aurait mis le rebord en conflit avec les faces à cet endroit. Sur le
    chanfrein sud-ouest de boitier_dc, dont la moitié ouest est une
    ouverture (rien ne monte pour soutenir un rebord), la réglette reste
    telle quelle : elle ne sert à rien à cet endroit mais ne gêne pas non
    plus, et la couper laissait un défaut (esquille, lu comme un
    "chanfrein négatif") plus gênant que la réglette elle-même. Coupé aux
    trois logements de heat insert qui affleurent le sommet, avec un trou
    de vis dans chacun : deux M2.5 sur boitier_dc, un M3 sur boitier_ac.

    epaisseur_paroi: épaisseur de la plaque et des murs des deux boîtes
    epaisseur_fond: épaisseur du fond
    gouttiere_jeu: jeu entre le rebord et la face intérieure d'un mur
    gouttiere_epaisseur: épaisseur du rebord
    gouttiere_profondeur: profondeur du rebord dans la cavité
    chanfrein: doit être sync avec boitier_dc
    ac_west_shift: décalage de boitier_ac appliqué par ensemble_boitiers,
        doit rester égal à `boitier_int_aile_x − ac_west_shift` interne à
        boitier_ac
    ac_east_shift: retrait de la face est de boitier_ac, doit rester égal à
        sa valeur interne
    ac_south_shift: retrait de la face sud de boitier_ac, doit rester égal
        à sa valeur interne
    ac_appui_pcb_hauteur: profondeur du rebord nord de boitier_ac sur son
        tiers central en X (aile_x + (east_x−aile_x)/3 à aile_x +
        (east_x−aile_x)*2/3) : presse le PCB du module sud par le dessus.
        Valeur empirique, indépendante des cotes internes de boitier_ac
        (muret_depuis_ouest, tour_y, rebord...).
    dc_appui_pcb_hauteur: profondeur du rebord sud de boitier_dc pour tenir
        le module XIAO
    vis_m25_diametre: passage des deux vis M2.5 (boitier_dc)
    vis_m3_diametre: passage de la vis M3 (boitier_ac) ; par défaut la cote
        mesurée `vis_passage`
    """
    wall = epaisseur_paroi
    z0 = epaisseur_fond
    jeu = gouttiere_jeu
    ep = gouttiere_epaisseur
    prof = gouttiere_profondeur
    margin = 0.5
    if vis_m3_diametre is None:
        vis_m3_diametre = measured("vis_passage")

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
    if vis_m25_diametre < 2.0:
        reject(
            f"vis_m25_diametre {vis_m25_diametre} is under 2 mm: raise it",
            param="vis_m25_diametre",
        )
    if vis_m3_diametre < 2.0:
        reject(
            f"vis_m3_diametre {vis_m3_diametre} is under 2 mm: raise it",
            param="vis_m3_diametre",
        )

    marche_x = measured("boitier_int_marche_x")

    dc_pts, _ = dc_contour()
    ac_pts = ac_top_contour()

    # Plate matches the two boxes' outer footprint exactly: it must not
    # overhang.
    body = extrude(Polygon(*dc_pts, align=None), z0) + extrude(Polygon(*ac_pts, align=None), z0)

    # Box rim
    for pts in (dc_pts, ac_pts):
        body = body + _ring(pts, z0, prof, ep, wall, jeu)

    # boitier_dc's SW diagonal chamfer is open
    chanfrein = measured("boitier_int_chanfrein")+2
    body = body - (Pos(0,0,z0)*Box(chanfrein,chanfrein,prof,align=AMIN))

    # Openings for sensors
    for opening_y in (30, 70):
        body = _dc_east_opening(body, opening_y, z0, prof)

    # Openings for corbels 
    for (cx, cy, d) in (dc_corbels(wall)+ac_corbels(wall)):
        body = body - Pos(cx,cy,0)*Cylinder(d/2, z0, align=CMIN)   

    # PCB press AC: the north rim segment (boitier_ac) runs deeper than
    # `gouttiere_profondeur` over the box's middle third in X, to press down
    # on the south module's PCB from above. 
    if ac_appui_pcb_hauteur <= 0.0:
        reject(
            f"ac_appui_pcb_hauteur {ac_appui_pcb_hauteur} is not positive: raise it",
            param="ac_appui_pcb_hauteur",
        )
    y_max = ac_pts[2][1]
    appui_y1 = y_max - (wall + jeu)
    body = _pcb_press(body, ac_pts[0][0], ac_pts[2][0], appui_y1 - ep, appui_y1, ac_appui_pcb_hauteur, wall)

    # PCB press DC on south rib to hold the XIAO module
    if dc_appui_xiao_hauteur <= 0.0:
        reject(
            f"dc_appui_xiao_hauteur {dc_appui_xiao_hauteur} is not positive: raise it",
            param="dc_appui_xiao_hauteur",
        )
    appui_y0 = wall + jeu
    body = _pcb_press(body, dc_pts[1][0], dc_pts[2][0], appui_y0, appui_y0 + ep, dc_appui_xiao_hauteur, wall)

    # Placing the lid means flipping it over (about a north-south axis, the
    # long way): that mirrors X. Everything above is modelled in the direct
    # ensemble_boitiers frame; mirror the whole part in X here so it lands
    # correctly once flipped onto the boxes (same fix as couvercle_ps).
    east_x = ac_pts[1][0]
    mid_x = east_x / 2.0
    mirror_x = Plane(origin=(mid_x, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
    body = mirror(body, about=mirror_x)

    if draft:
        return body

    bed = body.bounding_box().min.Z

    def keep(edge):
        bb = edge.bounding_box()
        return bb.min.Z <= bed + 0.05 or bb.min.Z >= wall + prof - 0.05

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 0.6)

from nurb import *

_MIN = (Align.MIN, Align.MIN, Align.MIN)


def _bb(x0, x1, y0, y1, z0, z1):
    """Box from bounds, in the part's own printing frame."""
    return Pos(x0, y0, z0) * Box(x1 - x0, y1 - y0, z1 - z0, align=_MIN)


@part
def plateau_pesage(
    cadre_largeur=110.0,
    cadre_profondeur=80.0,
    cadre_centre_y=-22.0,
    barre_y=0.0,
    tablier_epaisseur=1.6,
    nervure_hauteur=10.0,
    nervure_epaisseur=2.4,
    nervures_x=3,
    nervures_y=3,
    ajour_marge=4.0,
    ajour_minimum=8.0,
    fenetre_demi_largeur=11.0,
    berceau_debord=2.75,
    fenetre_jeu=1.5,
    sangle_epaisseur=1.9,
    sangle_largeur=16.3,
    sangle_marge=5.0,
    tube_paroi=1.6,
    tube_amorce=3.0,
    tube_recouvrement=0.6,
    selle_degagement=-0.5,
    bride_epaisseur=3.0,
    bride_portee=3.0,
    butee_x=45.0,
    butee_y=-8.0,
    draft=False,
):
    """Plateau nervuré qui porte le drip tray et le pose sur le bout chargé de la barre.

    Modélisé DANS SA POSITION D'IMPRESSION, tablier contre le plateau de l'A1
    Mini et nervures vers le haut : la face qui touche le tray sort donc la plus
    plane possible et rien ne demande de support. C'est l'ensemble qui le
    retourne. Dans ce repère, X est l'inverse du X du bac (la sangle, à droite
    dans le bac, est à gauche ici) et Z descend depuis la face d'appui du tray.

    cadre_largeur: largeur du plateau en X
    cadre_profondeur: profondeur du plateau en Y
    cadre_centre_y: centre en Y du plateau dans le repère du bac
    barre_y: axe de la barre de charge, qui fixe la fenêtre et la sangle
    tablier_epaisseur: épaisseur du tablier ajouré qui touche le tray
    nervure_hauteur: profondeur des nervures sous le tablier, la seule vraie ressource en raideur
    nervure_epaisseur: épaisseur de chaque nervure
    nervures_x: nombre de nervures intérieures parallèles à Y
    nervures_y: nombre de nervures intérieures parallèles à X
    ajour_marge: bande pleine de tablier laissée le long de chaque nervure
    ajour_minimum: en dessous de cette taille une case n'est pas ajourée du tout
    fenetre_demi_largeur: demi-largeur de la fenêtre ouverte au-dessus de la barre et des joues
    berceau_debord: de combien les joues du berceau dépassent le bout de la barre
    fenetre_jeu: jeu autour de la barre, en bout de fenêtre comme le long du couloir
    sangle_epaisseur: épaisseur de la sangle qui se pose sur le bout chargé de la barre
    sangle_largeur: largeur extérieure du tube en Y ; le jeu autour de la barre en découle (sangle_largeur/2 − tube_paroi − demi-section)
    sangle_marge: plastique de chaque côté de la sangle au-delà des deux M4 du bout chargé
    tube_paroi: épaisseur des joues du tube et des deux plots de verrouillage
    tube_amorce: longueur du lead-in à 45° à la bouche d'entrée, pour glisser la barre dans l'alésage sans accrocher
    tube_recouvrement: de combien le retour 45° passe sous l'arête de la barre pour la retenir (et donner une paroi au grub)
    selle_degagement: recouvrement (négatif) ou jeu (positif) en Y entre les murets et les joues du tube ; −0,5 les fait mordre pour fusionner en appui, plus besoin de fléchir
    bride_epaisseur: hauteur de la bride arrière que la lèvre du socle vient coiffer
    bride_portee: de combien cette bride dépasse le tablier vers l'arrière
    butee_x: position en X de la butée de surcharge du socle, qui doit tomber sur une nervure
    butee_y: position en Y de cette butée
    """
    barre_l = measured("barre_longueur")
    trou_int = measured("barre_trou_interieur")
    trou_ext = measured("barre_trou_exterieur")
    barre_s = measured("barre_section")

    rib_top = tablier_epaisseur + nervure_hauteur
    demi = cadre_largeur / 2.0
    y0 = cadre_centre_y - cadre_profondeur / 2.0
    y1 = cadre_centre_y + cadre_profondeur / 2.0
    # La bride mange sa portée sur l'arrière : le tablier démarre après.
    ty0 = y0 + bride_portee

    if nervure_hauteur < 6.0:
        reject(
            f"nervure_hauteur {nervure_hauteur} est sous 6 mm : la profondeur est la "
            f"seule ressource en raideur ici, une rainure de 3 mm donne 4,2 mm de "
            f"flèche au coin contre 1,0 mm de budget. Monte-la",
            param="nervure_hauteur",
        )
    if bride_portee > bride_epaisseur:
        reject(
            f"bride_portee {bride_portee} dépasse bride_epaisseur {bride_epaisseur} : "
            f"le dessous de la bride ne tiendrait plus les 45°. Baisse-la sous "
            f"{bride_epaisseur}",
            param="bride_portee",
        )
    if sangle_epaisseur > tablier_epaisseur + nervure_hauteur:
        reject(
            f"sangle_epaisseur {sangle_epaisseur} traverse le plateau : baisse-la",
            param="sangle_epaisseur",
        )
    if sangle_epaisseur <= tablier_epaisseur:
        reject(
            f"sangle_epaisseur {sangle_epaisseur} ne dépasse pas le tablier "
            f"({tablier_epaisseur}) : c'est ce décrochement qui pose la sangle sur la "
            f"barre pendant que le tablier la survole. Monte-la au-delà de "
            f"{tablier_epaisseur + 0.2:.1f}",
            param="sangle_epaisseur",
        )
    if fenetre_demi_largeur < barre_s / 2.0 + 2.0:
        reject(
            f"fenetre_demi_largeur {fenetre_demi_largeur} frotte sur une barre de "
            f"{barre_s} : monte-la au-delà de {barre_s / 2.0 + 2.0:.1f}",
            param="fenetre_demi_largeur",
        )

    bore_half = sangle_largeur / 2.0 - tube_paroi
    tube_jeu = bore_half - barre_s / 2.0
    # Le retour 45° descend 2·(jeu + recouvrement) sous le dessus de la barre.
    tube_inward = tube_jeu + tube_recouvrement
    tube_sous = 2.0 * tube_inward
    if tube_recouvrement < 0.3:
        reject(
            f"tube_recouvrement {tube_recouvrement} est trop court pour retenir la barre : "
            f"un 45° de moins de 0,3 mm sous l'arête ne rattrape rien, et le grub n'aurait "
            f"pas de paroi en face. Monte-le",
            param="tube_recouvrement",
        )
    if tube_jeu < 0.2:
        reject(
            f"sangle_largeur {sangle_largeur} avec une paroi de {tube_paroi} ne laisse "
            f"que {tube_jeu:.2f} mm de jeu autour d'une barre de {barre_s} : le tube ne "
            f"l'accepterait pas. Monte sangle_largeur au-delà de "
            f"{2.0 * (barre_s / 2.0 + tube_paroi + 0.2):.1f}",
            param="sangle_largeur",
        )
    if tube_jeu > 0.9:
        reject(
            f"jeu de {tube_jeu:.2f} mm autour de la barre : le tube flotte et le "
            f"tangage revient. Baisse sangle_largeur sous "
            f"{2.0 * (barre_s / 2.0 + tube_paroi + 0.9):.1f} ou monte tube_paroi",
            param="sangle_largeur",
        )
    if tube_sous > 3.0:
        reject(
            f"le retour descend de {tube_sous:.2f} mm sous la barre "
            f"(2·(jeu {tube_jeu:.2f} + recouvrement {tube_recouvrement})), plus que les "
            f"3 mm libres jusqu'au fond du bac. Baisse tube_recouvrement ou le jeu",
            param="tube_recouvrement",
        )
    if selle_degagement < bore_half - sangle_largeur / 2.0 + 0.1:
        reject(
            f"selle_degagement {selle_degagement} fait mordre les murets dans l'alésage "
            f"du tube : ils gêneraient la barre. Garde-le au-dessus de "
            f"{bore_half - sangle_largeur / 2.0 + 0.1:.2f}",
            param="selle_degagement",
        )

    # X local = -X du bac. La sangle couvre le bout chargé (les deux M4 de la
    # barre, laissés vides : la selle localise, plus de goujons).
    sangle_x0 = -trou_ext - sangle_marge
    sangle_x1 = -trou_int + sangle_marge
    if sangle_x0 < -demi:
        reject(
            f"cadre_largeur {cadre_largeur} ne couvre pas la sangle au bout chargé : "
            f"monte-la au-delà de {2.0 * (trou_ext + sangle_marge):.0f}",
            param="cadre_largeur",
        )

    # Bout de la fenêtre : la barre s'arrête à barre_l/2, les joues du berceau
    # berceau_debord plus loin. Tout ce qui est au-delà est libre, et refermer le
    # cadre là plutôt que de laisser un C coûte deux nervures et rien d'autre.
    fenetre_bout = barre_l / 2.0 + berceau_debord + fenetre_jeu
    montant = demi - fenetre_bout
    if montant < nervure_epaisseur + 2.0:
        reject(
            f"cadre_largeur {cadre_largeur} ne laisse que {montant:.1f} mm entre le "
            f"bout de la fenêtre ({fenetre_bout:.1f}) et le bord : le cadre resterait "
            f"ouvert en C. Monte-la au-delà de "
            f"{2.0 * (fenetre_bout + nervure_epaisseur + 2.0):.0f}",
            param="cadre_largeur",
        )

    # --- nervures : pourtour, puis grille intérieure ---
    e = nervure_epaisseur
    xs = [-demi, demi, (fenetre_bout + demi) / 2.0] + [
        -demi + cadre_largeur * (i + 1) / (nervures_x + 1) for i in range(nervures_x)
    ]
    ys = [ty0, y1, butee_y] + [
        ty0 + (y1 - ty0) * (i + 1) / (nervures_y + 1) for i in range(nervures_y)
    ]
    xs = sorted(xs)
    ys = sorted(ys)

    body = _bb(-demi, demi, ty0, y1, 0.0, tablier_epaisseur)
    selle_y0 = barre_y - sangle_largeur / 2.0
    selle_y1 = barre_y + sangle_largeur / 2.0
    for x in xs:
        a = min(max(x - e / 2.0, -demi), demi - e)
        # Une nervure qui court en Y et vient buter contre une joue empêche le
        # 45° de camber : le coupon de 12 mm clipse, le plateau non. On coupe
        # toute nervure qui traverse la sangle, avec selle_degagement de chaque
        # côté, pour que les joues fléchissent comme sur le coupon.
        if a < sangle_x1 and a + e > sangle_x0:
            y_cut0 = selle_y0 - selle_degagement
            y_cut1 = selle_y1 + selle_degagement
            if y_cut0 - ty0 > e:
                body += _bb(a, a + e, ty0, y_cut0, 0.0, rib_top)
            if y1 - y_cut1 > e:
                body += _bb(a, a + e, y_cut1, y1, 0.0, rib_top)
        else:
            body += _bb(a, a + e, ty0, y1, 0.0, rib_top)
    for y in ys:
        b = min(max(y - e / 2.0, ty0), y1 - e)
        body += _bb(-demi, demi, b, b + e, 0.0, rib_top)

    # --- bride arrière : coin à 45°, donc imprimable telle quelle ---
    bride = _bb(-demi, demi, y0, ty0, rib_top - bride_epaisseur, rib_top)
    # Le dessous doit monter d'au moins autant qu'il avance : plan à 45°.
    bride -= Pos(0, ty0, rib_top - bride_epaisseur) * Rot(-45, 0, 0) * Box(
        400.0, 400.0, 400.0, align=(Align.CENTER, Align.CENTER, Align.MAX)
    )
    body += bride

    # --- couloir de la barre : au-dessus d'elle, rien de plus épais que le tablier ---
    # Le tablier (2,0) survole la barre avec 0,3 mm de jeu, les nervures (12,0)
    # non. La fenêtre ne dégageait que x > sangle_x1 ; côté sangle les nervures
    # traversaient la barre de 755 mm3, mesurés à l'assemblage. Le couloir est
    # serré sur la barre (demi-section + jeu) et pas sur la fenêtre : chaque
    # millimètre laissé aux nervures de part et d'autre est ce qui amène la
    # charge du tablier jusqu'à la sangle.
    couloir_demi = barre_s / 2.0 + fenetre_jeu
    body -= _bb(
        -barre_l / 2.0 - fenetre_jeu, sangle_x1,
        barre_y - couloir_demi, barre_y + couloir_demi,
        tablier_epaisseur, rib_top + 1.0,
    )

    # --- ajours du tablier, une case sur deux entre nervures ---
    # La sangle est une exclusion au même titre qu'une nervure : une case
    # tombait pile sur elle et la perçait. Elle garde sa bande pleine.
    # Du côté du cadre le plus proche, cette bande va jusqu'à la face
    # latérale : une marge de 4 mm laissait 0,45 mm de lucarne entre la
    # garde et le muret (deux slivers de 0,72 mm²), et la base de la sangle
    # ne rejoignait pas le cadre.
    garde_y0 = ty0 if (selle_y0 - ty0) <= (y1 - selle_y1) else (selle_y0 - ajour_marge)
    garde_y1 = y1 if (y1 - selle_y1) <= (selle_y0 - ty0) else (selle_y1 + ajour_marge)
    garde_sangle = _bb(
        sangle_x0 - ajour_marge, sangle_x1 + ajour_marge,
        garde_y0, garde_y1,
        -1.0, rib_top + 1.0,
    )
    for i in range(len(xs) - 1):
        for j in range(len(ys) - 1):
            ax0 = xs[i] + e / 2.0 + ajour_marge
            ax1 = xs[i + 1] - e / 2.0 - ajour_marge
            ay0 = ys[j] + e / 2.0 + ajour_marge
            ay1 = ys[j + 1] - e / 2.0 - ajour_marge
            if ax1 - ax0 < ajour_minimum or ay1 - ay0 < ajour_minimum:
                continue
            body -= _bb(ax0, ax1, ay0, ay1, -0.5, tablier_epaisseur + 0.5) - garde_sangle

    # --- sangle : elle seule passe au-dessus de la barre, au bout chargé ---
    body += _bb(
        sangle_x0, sangle_x1,
        barre_y - sangle_largeur / 2.0, barre_y + sangle_largeur / 2.0,
        0.0, sangle_epaisseur,
    )

    # --- fenêtre : rien de plein au-dessus du silicone ni du bout fixe ---
    # Elle s'arrête au bout des joues du berceau, pas au bord du plateau : au-delà
    # il n'y a plus d'obstacle, et ce montant est ce qui referme le cadre.
    body -= _bb(
        sangle_x1, fenetre_bout,
        barre_y - fenetre_demi_largeur, barre_y + fenetre_demi_largeur,
        -1.0, rib_top + 1.0,
    )

    # --- tube du bout chargé : losange fermé où la barre coulisse ---
    # Profil Y-Z extrudé en X sur la longueur de la sangle : sangle porteuse en
    # bas (paroi haute en usage), deux joues SERRÉES, et deux RETOURS 45° qui
    # passent sous les arêtes de la barre. Les joues rigides (bracées par les
    # murets, selle_degagement < 0) encaissent le tangage ; les retours retiennent
    # la barre — roulis + rétention — et donnent au grub une paroi EN FACE : sans
    # eux, serrer un grub ne ferait que soulever le plateau. Le 45° s'auto-supporte
    # (dessous à 45°), là où un plafond plat pontait sur 13,8 mm. La barre s'enfile
    # en X depuis la fenêtre, le jeu vertical la laissant passer sous les retours.
    # Le grub (optionnel) se serre par le JOUR CENTRAL entre les deux retours, là
    # où débouchent les M4 de la barre — aucun trou à percer.
    outer_half = sangle_largeur / 2.0
    tube_z_bar_top = sangle_epaisseur + barre_s
    z_tip = tube_z_bar_top + tube_inward
    z_top = z_tip + tube_inward
    nub = barre_s / 2.0 - tube_recouvrement
    oy = barre_y
    pts = [
        (oy - outer_half, 0.0),
        (oy + outer_half, 0.0),
        (oy + outer_half, z_top),
        (oy + bore_half, z_top),
        (oy + nub, z_tip),
        (oy + bore_half, tube_z_bar_top),
        (oy + bore_half, sangle_epaisseur),
        (oy - bore_half, sangle_epaisseur),
        (oy - bore_half, tube_z_bar_top),
        (oy - nub, z_tip),
        (oy - bore_half, z_top),
        (oy - outer_half, z_top),
    ]
    body += Pos(sangle_x0, 0, 0) * extrude(
        Plane.YZ * Polygon(*pts, align=None), sangle_x1 - sangle_x0
    )
    # Amorce à la bouche d'entrée (sangle_x1) : l'obstacle à l'insertion, ce sont
    # les RETOURS 45° (le sommet du U), pas les joues. On les rabote en rampe à
    # l'entrée — pleine hauteur à la bouche, retours pleins tube_amorce plus loin —
    # pour que le bout de la barre passe dessous en la glissant, sans buter sur le
    # premier débord du 45°. Un seul rabot en travers, pas deux vides dans les joues.
    if tube_amorce > 0.0:
        # Point bas à z_top − tube_amorce (sous le dessus de la barre) pour que la
        # diagonale monte à 45° pile jusqu'au sommet du U et coupe franchement la
        # lèvre, au lieu d'une pente molle qui l'effleure.
        ramp = [
            (sangle_x1 + 0.1, z_top - tube_amorce),
            (sangle_x1 + 0.1, z_top + 0.1),
            (sangle_x1 - tube_amorce, z_top + 0.1),
        ]
        # extrude sur Plane.XZ va vers −Y : on part de la face +Y du tube.
        body -= Pos(0.0, barre_y + outer_half + 0.1, 0.0) * extrude(
            Plane.XZ * Polygon(*ramp, align=None), 2.0 * outer_half + 0.2
        )

    solids = list(body.solids())
    if len(solids) > 1:
        base = solids[0]
        for s in solids[1:]:
            base = base.fuse(s)
        body = base

    if draft:
        return body

    # Les angles extérieurs verticaux du cadre, et rien d'autre : le tablier est
    # une face d'appui, les nervures sont trop minces pour deux chanfreins.
    coins = [(-demi, y0), (demi, y0), (-demi, y1), (demi, y1)]
    conc = {
        (round(ed.center().X, 2), round(ed.center().Y, 2), round(ed.center().Z, 2))
        for ed in concave_edges(body)
    }

    def garde(edge):
        c = edge.center()
        if (round(c.X, 2), round(c.Y, 2), round(c.Z, 2)) in conc:
            return False
        return any(abs(c.X - x) < 0.6 and abs(c.Y - y) < 0.6 for x, y in coins)

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(garde), 1.0)

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
    barre_y=-25.0,
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
    sangle_largeur=17.0,
    sangle_marge=5.0,
    goujon_passage=4.3,
    goujon_peau=0.6,
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
    sangle_largeur: largeur en Y de cette sangle
    sangle_marge: plastique autour de chaque trou de goujon
    goujon_passage: diamètre des deux poches borgnes qui reçoivent les goujons M4 sans tête
    goujon_peau: peau laissée côté tray au-dessus de chaque poche de goujon
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
    # La poche du goujon doit loger les 1,05 mm de saillie (1 tour et demi au pas
    # de 0,7) plus 0,2 de garde, sinon le goujon bute sur la peau et soulève le
    # plateau au lieu de le poser sur la barre — une erreur de tare que rien ne
    # signale. C'était 1,4 mm et 2 tours tant que la sangle faisait 2,3 ; les
    # 0,4 mm rendus à la pile verticale se sont payés ici, et nulle part ailleurs.
    goujon_poche = sangle_epaisseur - goujon_peau
    if goujon_peau < 0.6:
        reject(
            f"goujon_peau {goujon_peau} est sous 0,6 mm : c'est la peau qui ferme la "
            f"face d'appui du tray au-dessus du goujon, et sous trois couches elle se "
            f"perce au réglage. Monte-la",
            param="goujon_peau",
        )
    if goujon_poche < 1.25:
        reject(
            f"goujon_peau {goujon_peau} ne laisse que {goujon_poche:.2f} mm de poche "
            f"dans une sangle de {sangle_epaisseur} : il en faut 1,25 pour les 1,05 mm "
            f"de saillie du goujon plus la garde. Baisse-la sous "
            f"{sangle_epaisseur - 1.25:.2f}",
            param="goujon_peau",
        )
    if fenetre_demi_largeur < barre_s / 2.0 + 2.0:
        reject(
            f"fenetre_demi_largeur {fenetre_demi_largeur} frotte sur une barre de "
            f"{barre_s} : monte-la au-delà de {barre_s / 2.0 + 2.0:.1f}",
            param="fenetre_demi_largeur",
        )

    # X local = -X du bac. La sangle couvre les deux goujons plus leur marge.
    sangle_x0 = -trou_ext - goujon_passage / 2.0 - sangle_marge
    sangle_x1 = -trou_int + goujon_passage / 2.0 + sangle_marge
    if sangle_x0 < -demi:
        reject(
            f"cadre_largeur {cadre_largeur} ne couvre pas le goujon extérieur plus "
            f"sangle_marge : monte-la au-delà de "
            f"{2.0 * (trou_ext + goujon_passage / 2.0 + sangle_marge):.0f}",
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
    for x in xs:
        a = min(max(x - e / 2.0, -demi), demi - e)
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
    # La sangle est une exclusion au même titre qu'une nervure : la case du coin
    # arrière la traversait de part en part, à 3 mm du goujon extérieur. Elle
    # garde donc sa bande pleine de ajour_marge, comme tout le reste.
    garde_sangle = _bb(
        sangle_x0 - ajour_marge, sangle_x1 + ajour_marge,
        barre_y - sangle_largeur / 2.0 - ajour_marge,
        barre_y + sangle_largeur / 2.0 + ajour_marge,
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

    # --- poches borgnes des deux goujons M4 sans tête du bout chargé ---
    # Elles ne traversent PAS : la peau de goujon_peau est du côté du tray, donc
    # elle est imprimée à même le lit, sans pont ni support, et la face d'appui
    # du tray reste continue. Rien ne peut couler jusqu'à la barre, et il n'y a
    # plus de cuvette Ø4,3 au-dessus d'un goujon réglé un peu court.
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)
    for x in (-trou_ext, -trou_int):
        body -= Pos(x, barre_y, goujon_peau) * Cylinder(
            goujon_passage / 2.0, sangle_epaisseur - goujon_peau + 1.0, align=cmin
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

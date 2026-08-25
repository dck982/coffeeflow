from nurb import *


def _bb(x0, x1, y0, y1, z0, z1):
    """Box from bounds, in the bac frame."""
    return Pos(x0, y0, z0) * Box(
        x1 - x0, y1 - y0, z1 - z0, align=(Align.MIN, Align.MIN, Align.MIN)
    )


def _sous_plan_45(y0, z0):
    """Half-space z - z0 < y - y0: the 45 degree underside a corbel needs."""
    return Pos(0, y0, z0) * Rot(45, 0, 0) * Box(
        500.0, 400.0, 400.0, align=(Align.CENTER, Align.CENTER, Align.MAX)
    )


@part
def base_pesage(
    hauteur_pile=18.0,
    berceau_epaisseur=3.0,
    socle_epaisseur=4.0,
    barre_y=-25.0,
    jeu_barre=0.4,
    joue_epaisseur=2.5,
    joue_hauteur=9.0,
    fils_degagement=3.0,
    fils_arrondi=18.0,
    fils_cote_avant=True,
    cadre_largeur=110.0,
    cadre_profondeur=80.0,
    cadre_centre_y=-22.0,
    tablier_epaisseur=2.0,
    nervure_hauteur=10.0,
    bride_epaisseur=2.0,
    bride_portee=3.0,
    jeu_cage=1.2,
    jeu_cage_avant=4.0,
    paroi_cage=2.5,
    cage_hauteur=14.0,
    rail_debord=6.0,
    levre_portee=4.5,
    levre_jeu_vertical=1.5,
    pied_x=50.0,
    pied_y=40.0,
    aimant_coin_x=48.0,
    aimant_coin_y=-59.0,
    aimant_paroi_z=5.0,
    puit_diametre=8.2,
    poche_diametre=7.4,
    poche_profondeur=4.0,
    poche_plafond=1.5,
    butee_x=45.0,
    butee_y=-8.0,
    butee_jeu=0.25,
    insert_m25_diametre=4.0,
    insert_m25_longueur=4.0,
    butee_boss_hauteur=5.0,
    vis_fraisee_passage=4.5,
    plaque_x=-90.0,
    plaque_largeur=25.0,
    plaque_profondeur=40.0,
    plaque_epaisseur=2.0,
    marge_perforations=2.0,
    jeu_paroi_arriere=0.5,
    draft=False,
):
    """Socle de pesage posé au fond du bac de la Profitec Go : berceau de la barre, cage du plateau, butée de surcharge.

    hauteur_pile: hauteur du plan d'appui du tray au-dessus du fond du bac (18,0 mesuré ; à réajuster au dixième)
    berceau_epaisseur: épaisseur de plastique sous le bout FIXE de la barre
    socle_epaisseur: épaisseur du fond du socle partout ailleurs
    barre_y: position en Y de l'axe de la barre de charge (là où se pose la tasse)
    jeu_barre: jeu entre la barre et chaque joue du berceau
    joue_epaisseur: épaisseur des deux joues qui encaissent la barre
    joue_hauteur: hauteur des joues, ce qui raidit la potence du berceau
    fils_degagement: de combien le haut des joues descend pour laisser sortir les fils de la cellule
    fils_arrondi: rayon de ce dégagement, donc la douceur de la pente sous les fils
    fils_cote_avant: les fils sortent du côté avant de la barre ; décochez s'ils sortent côté paroi arrière
    cadre_largeur: largeur en X du plateau, que la cage doit entourer
    cadre_profondeur: profondeur en Y du plateau
    cadre_centre_y: centre en Y du plateau
    tablier_epaisseur: épaisseur du tablier du plateau, pour situer le dessous des nervures
    nervure_hauteur: profondeur des nervures du plateau, pour situer leur face basse
    bride_epaisseur: épaisseur de la bride basse du plateau, que la lèvre vient coiffer
    bride_portee: de combien cette bride dépasse le tablier vers l'arrière
    jeu_cage: jeu entre le plateau et la cage sur les côtés et l'arrière (1,0–1,5 : plus fin, le café sèche en pont)
    jeu_cage_avant: jeu à l'avant, plus large pour pouvoir dégager le plateau des lèvres à la main
    paroi_cage: épaisseur des parois de la cage
    cage_hauteur: hauteur des parois de la cage
    rail_debord: largeur de fond que chaque rail latéral glisse sous le plateau
    levre_portee: de combien chaque lèvre anti-basculement coiffe la bride arrière du plateau
    levre_jeu_vertical: jeu vertical sous la lèvre (la pesée ne fait que 0,05 mm de course)
    pied_x: écartement en X des deux pieds avant à aimant
    pied_y: position en Y des deux pieds avant à aimant
    aimant_coin_x: écartement en X des deux aimants ajoutés dans les angles arrière
    aimant_coin_y: position en Y de ces deux aimants d'angle
    aimant_paroi_z: hauteur de l'axe de l'aimant qui plaque le socle contre la paroi arrière
    puit_diametre: diamètre du puits d'aimant Ø8×3
    poche_diametre: largeur des deux poches qui coiffent les têtes de vis de la paroi arrière
    poche_profondeur: de combien chaque poche s'enfonce en Y depuis la face nord (tête + garde)
    poche_plafond: plastique laissé au-dessus du plafond de chaque poche
    butee_x: position en X de la butée de surcharge
    butee_y: position en Y de la butée, tenue hors de la fenêtre du plateau
    butee_jeu: chute autorisée avant que la butée n'arrête le plateau (un demi-tour de M3 = 0,25 mm)
    insert_m25_diametre: trou du boss pour l'insert laiton M2,5 (Ø4,6 hors tout) à emmancher à chaud
    insert_m25_longueur: longueur de cet insert
    butee_boss_hauteur: hauteur du bossage qui porte l'insert de la butée
    vis_fraisee_passage: passage des deux vis M4 fraisées qui tiennent le bout fixe
    plaque_x: bord extérieur de la plaque du module d'amplification
    plaque_largeur: largeur utile de cette plaque
    plaque_profondeur: profondeur de cette plaque
    plaque_epaisseur: épaisseur de cette plaque
    marge_perforations: de combien le socle s'écarte du cercle de perforations Ø80
    jeu_paroi_arriere: jeu entre le socle et la paroi arrière du bac
    """
    barre_l = measured("barre_longueur")
    barre_s = measured("barre_section")
    trou_int = measured("barre_trou_interieur")
    trou_ext = measured("barre_trou_exterieur")
    tete_d = measured("vis_fraisee_m4_tete")
    perfo_r = measured("bac_perforations_diametre") / 2.0
    perfo_y = measured("bac_perforations_y")
    vis_x = measured("bac_vis_basse_x")
    vis_z = measured("bac_vis_basse_z")
    tete_saillie = measured("bac_vis_tete_depassement")
    puit_fond = measured("puit_fond")
    aimant_h = measured("aimant_hauteur")
    bac_y = measured("bac_profondeur") / 2.0

    barre_dessus = berceau_epaisseur + barre_s
    sangle = hauteur_pile - barre_dessus
    plat_dessous = hauteur_pile - tablier_epaisseur - nervure_hauteur
    levre_dessous = plat_dessous + bride_epaisseur + levre_jeu_vertical
    butee_saillie = plat_dessous - butee_jeu - socle_epaisseur

    # Les têtes de vis sortent de la paroi ARRIÈRE, horizontalement : la poche est
    # un tunnel qui perce la face nord, pas un lamage dans le fond. Elle est
    # carrée et pas ronde, parce qu'un alésage rond couché a une voûte en arc que
    # rien ne soutient, là où un plafond plat de 7,4 mm est un pont banal.
    poche_r = poche_diametre / 2.0
    poche_z0 = vis_z - poche_r
    poche_z1 = vis_z + poche_r
    poche_boss_hauteur = poche_z1 + poche_plafond
    # Ce que la tête pénètre réellement dans le socle : sa saillie moins le jeu
    # que le socle garde devant la paroi.
    tete_penetration = tete_saillie - jeu_paroi_arriere

    if sangle < 1.6:
        reject(
            f"hauteur_pile {hauteur_pile} ne laisse que {sangle:.2f} mm de sangle "
            f"au-dessus de la barre (dessus à {barre_dessus:.2f}) : monte-la au-delà de "
            f"{barre_dessus + 1.6:.1f}",
            param="hauteur_pile",
        )
    if plat_dessous < socle_epaisseur + 1.5:
        reject(
            f"le dessous des nervures tombe à {plat_dessous:.2f} mm, sous les "
            f"{socle_epaisseur + 1.5:.2f} mm qu'il faut au-dessus d'un fond de "
            f"{socle_epaisseur} : "
            f"baisse nervure_hauteur ou socle_epaisseur",
            param="nervure_hauteur",
        )
    if cage_hauteur < levre_dessous + 2.5:
        reject(
            f"cage_hauteur {cage_hauteur} ne laisse pas 2,5 mm de lèvre au-dessus de "
            f"sa face basse ({levre_dessous:.1f}) : monte-la au-delà de "
            f"{levre_dessous + 2.5:.1f}",
            param="cage_hauteur",
        )
    if cage_hauteur > hauteur_pile - 2.0:
        reject(
            f"cage_hauteur {cage_hauteur} vient à moins de 2 mm du tray, qui pose à "
            f"{hauteur_pile} : la cage le toucherait et court-circuiterait la mesure. "
            f"Baisse-la sous {hauteur_pile - 2.0:.1f}",
            param="cage_hauteur",
        )
    if jeu_cage_avant < bride_portee + 0.5:
        reject(
            f"jeu_cage_avant {jeu_cage_avant} ne laisse pas basculer le plateau pour "
            f"dégager une bride de {bride_portee} : monte-le au-delà de "
            f"{bride_portee + 0.5}",
            param="jeu_cage_avant",
        )
    if butee_saillie < 0.5:
        reject(
            f"la vis de butée ne dépasserait que de {butee_saillie:.2f} mm du fond : "
            f"baisse socle_epaisseur",
            param="socle_epaisseur",
        )
    if joue_hauteur > barre_dessus - 1.0:
        reject(
            f"joue_hauteur {joue_hauteur} dépasse le dessus de la barre "
            f"({barre_dessus:.2f}) : baisse-la sous {barre_dessus - 1.0:.1f}",
            param="joue_hauteur",
        )
    if poche_profondeur < tete_penetration + 0.3:
        reject(
            f"poche_profondeur {poche_profondeur} ne coiffe pas une tête qui pénètre "
            f"de {tete_penetration:.1f} mm dans le socle (saillie {tete_saillie} moins "
            f"le jeu {jeu_paroi_arriere}) : le socle porterait sur les deux têtes au "
            f"lieu de poser sur la tôle. Monte-la au-delà de "
            f"{tete_penetration + 0.3:.1f}",
            param="poche_profondeur",
        )
    if poche_z0 < 0.6:
        reject(
            f"l'axe des têtes à z={vis_z} ne laisse que {poche_z0:.2f} mm de fond sous "
            f"la poche : sous 0,6 mm il n'y a plus de première couche pour la fermer. "
            f"Baisse poche_diametre sous {2.0 * (vis_z - 0.6):.1f}",
            param="poche_diametre",
        )
    if poche_boss_hauteur > cage_hauteur:
        reject(
            f"le bossage de poche monte à {poche_boss_hauteur:.1f} mm, au-dessus de la "
            f"cage ({cage_hauteur}) : il toucherait le tray. Baisse poche_plafond sous "
            f"{cage_hauteur - poche_z1:.1f}",
            param="poche_plafond",
        )
    if joue_hauteur - fils_degagement < berceau_epaisseur + 2.0:
        reject(
            f"fils_degagement {fils_degagement} descend le haut des joues à "
            f"{joue_hauteur - fils_degagement:.1f} mm, quand la barre commence à "
            f"{berceau_epaisseur} : il ne resterait plus 2 mm de joue pour la tenir "
            f"de flanc. Baisse-le sous {joue_hauteur - berceau_epaisseur - 2.0:.1f}",
            param="fils_degagement",
        )
    if fils_degagement > 0.0 and fils_arrondi < 2.0 * fils_degagement:
        reject(
            f"fils_arrondi {fils_arrondi} est trop court devant un creux de "
            f"{fils_degagement} : la cuvette deviendrait une encoche à flancs raides, "
            f"ce qu'on cherche justement à éviter sous une gaine. Monte-le au-delà de "
            f"{2.0 * fils_degagement:.1f}",
            param="fils_arrondi",
        )
    if socle_epaisseur < puit_fond + aimant_h + 0.4:
        reject(
            f"socle_epaisseur {socle_epaisseur} ne loge pas un aimant de {aimant_h} mm "
            f"sur un fond de {puit_fond} : monte-la au-delà de "
            f"{puit_fond + aimant_h + 0.4:.1f}",
            param="socle_epaisseur",
        )

    # Emprise du plateau, puis la cage autour.
    plat_x = cadre_largeur / 2.0
    plat_y0 = cadre_centre_y - cadre_profondeur / 2.0
    plat_y1 = cadre_centre_y + cadre_profondeur / 2.0
    cage_x_in = plat_x + jeu_cage
    cage_x_out = cage_x_in + paroi_cage
    cage_y_in = plat_y0 - jeu_cage
    cage_y_out = cage_y_in - paroi_cage
    stop_y_in = plat_y1 + jeu_cage_avant
    stop_y_out = stop_y_in + paroi_cage
    rail_x_in = cage_x_in - rail_debord
    arriere = -bac_y + jeu_paroi_arriere

    # Le cercle de perforations est une exclusion, pas un décor : la patte avant
    # doit démarrer en dehors, sinon il lui rogne un croissant et laisse une
    # section de quelques dixièmes. On le calcule là où le cercle est le plus
    # large sur la plage en Y de la patte, c'est-à-dire au plus près du centre.
    perfo_rc = perfo_r + marge_perforations

    def _perfo_x(y):
        d = perfo_rc**2 - (y - perfo_y) ** 2
        return d**0.5 if d > 0 else 0.0

    y_proche = min(max(perfo_y, stop_y_in), pied_y + rail_debord)
    puit_bord = pied_x - puit_diametre / 2.0
    # La patte se cale sur le puits (2,5 mm de plastique autour), pas sur
    # rail_debord, puis recule si le cercle de perforations la pousse.
    patte_x_in = max(puit_bord - 2.5, _perfo_x(y_proche) + 2.0)
    if puit_bord - patte_x_in < 2.0:
        reject(
            f"le puits d'aimant à x={pied_x} ne laisse que "
            f"{puit_bord - patte_x_in:.2f} mm de plastique côté intérieur : le cercle "
            f"de perforations force la patte à démarrer à x={patte_x_in:.1f}. "
            f"Monte pied_x au-delà de {patte_x_in + puit_diametre / 2.0 + 2.0:.1f}",
            param="pied_x",
        )
    if pied_x + puit_diametre / 2.0 + 2.0 > cage_x_out:
        reject(
            f"le puits d'aimant à x={pied_x} déborde la paroi de cage "
            f"(x={cage_x_out:.1f}) : baisse pied_x sous "
            f"{cage_x_out - puit_diametre / 2.0 - 2.0:.1f}",
            param="pied_x",
        )

    # --- fonds ---
    body = _bb(-cage_x_out, cage_x_out, arriere, cage_y_in, 0.0, socle_epaisseur)
    for s in (-1.0, 1.0):
        x0, x1 = sorted((s * rail_x_in, s * cage_x_out))
        body += _bb(x0, x1, arriere, stop_y_out, 0.0, socle_epaisseur)
        # Patte avant : elle porte la butée d'insertion et le pied à aimant.
        px0, px1 = sorted((s * patte_x_in, s * cage_x_out))
        body += _bb(px0, px1, stop_y_in, pied_y + rail_debord, 0.0, socle_epaisseur)

    # --- cage : parois latérales, paroi arrière, butées avant ---
    for s in (-1.0, 1.0):
        x0, x1 = sorted((s * cage_x_in, s * cage_x_out))
        body += _bb(x0, x1, cage_y_in, stop_y_in, 0.0, cage_hauteur)
        sx0, sx1 = sorted((s * patte_x_in, s * cage_x_out))
        body += _bb(sx0, sx1, stop_y_in, stop_y_out, 0.0, cage_hauteur)
    body += _bb(-cage_x_out, cage_x_out, cage_y_out, cage_y_in, 0.0, cage_hauteur)

    # --- lèvres anti-basculement, deux, avec un vide central qui draine ---
    coupe = _sous_plan_45(cage_y_in, levre_dessous)
    for s in (-1.0, 1.0):
        x0, x1 = sorted((s * (plat_x - 7.0), s * (plat_x * 0.3)))
        body += _bb(
            x0, x1, cage_y_in, cage_y_in + levre_portee, levre_dessous, cage_hauteur
        ) - coupe

    # --- bossages des deux poches, adossés à la paroi arrière de la cage ---
    # Ils montent maintenant à hauteur de tête (axe à 5,0, donc dessus à 8,7) au
    # lieu des 6,0 mm que demandait un lamage dans le fond, et ils fusionnent
    # avec la paroi de cage qui est juste devant.
    for s in (-1.0, 1.0):
        bx0, bx1 = sorted((s * (vis_x - poche_r - 2.5), s * (vis_x + poche_r + 2.5)))
        body += _bb(bx0, bx1, arriere, cage_y_in, 0.0, poche_boss_hauteur)

    # --- bossage de l'aimant qui plaque le socle contre la paroi arrière ---
    # Il est le seul morceau du socle qui DÉPASSE la ligne des autres : il va
    # jusqu'à -bac_y, donc sa peau touche la tôle pendant que le reste de la face
    # nord garde son jeu_paroi_arriere. C'est lui qui fait le zéro en Y.
    aimant_r = puit_diametre / 2.0
    paroi_boss_hauteur = aimant_paroi_z + aimant_r + poche_plafond
    body += _bb(
        -aimant_r - 2.5, aimant_r + 2.5, -bac_y, cage_y_in, 0.0, paroi_boss_hauteur
    )

    # --- deux aimants de plus, dans les angles arrière, face au fond du bac ---
    # La bande arrière ne fait que 6,3 mm de profondeur et les rails 8,5 mm de
    # large : ni l'une ni les autres ne logent un puits Ø8,2, qui demande
    # 12,2 × 12,2. D'où cette dalle locale, qui ne coûte que le morceau manquant
    # entre le rail et l'aimant.
    for s in (-1.0, 1.0):
        dx0, dx1 = sorted((s * (aimant_coin_x - aimant_r - 2.0), s * cage_x_out))
        body += _bb(
            dx0, dx1, arriere, aimant_coin_y + aimant_r + 2.0, 0.0, socle_epaisseur
        )

    # --- potence du berceau : fond, deux joues, siège de la barre ---
    joue_ext = barre_s / 2.0 + jeu_barre + joue_epaisseur
    joue_int = barre_s / 2.0 + jeu_barre
    berceau_x_int = -trou_int + 5.0
    # 2 mm de recouvrement dans le rail : un contact coplanaire ne fusionne pas.
    body += _bb(
        -rail_x_in - 2.0, berceau_x_int, barre_y - joue_ext, barre_y + joue_ext,
        0.0, socle_epaisseur,
    )
    for s in (-1.0, 1.0):
        jy0, jy1 = sorted((barre_y + s * joue_int, barre_y + s * joue_ext))
        body += _bb(
            -barre_l / 2.0 - 2.75, berceau_x_int, jy0, jy1, 0.0, joue_hauteur
        )
    body -= _bb(
        -barre_l / 2.0 - 3.25, berceau_x_int + 0.75,
        barre_y - joue_int, barre_y + joue_int,
        berceau_epaisseur, joue_hauteur + 1.0,
    )

    # --- dégagement des fils de la cellule, dans le haut d'UNE seule joue ---
    # Les fils sortent du FLANC de la barre, noyés dans la colle jusqu'au centre
    # du trou intérieur, donc vers z = 9,3 quand le haut de la joue est à 9,0.
    # Et entre le flanc et la joue il n'y a que jeu_barre (0,4) : ils ne peuvent
    # que passer par-dessus. D'où cette cuvette, centrée là où ils quittent la
    # colle. C'est un arc de cercle et pas une encoche : le rayon fait
    # redescendre le haut de la joue en pente douce au lieu de fabriquer deux
    # angles vifs contre lesquels la gaine travaillerait à chaque vibration.
    # Cuvette dans une face du DESSUS : rien à soutenir à l'impression.
    # Ils ne sortent que d'un côté, donc l'autre joue garde sa hauteur pleine.
    if fils_degagement > 0.0:
        s = 1.0 if fils_cote_avant else -1.0
        fy0, fy1 = sorted(
            (barre_y + s * (joue_int - 0.5), barre_y + s * (joue_ext + 1.0))
        )
        body -= Pos(
            -trou_int,
            (fy0 + fy1) / 2.0,
            joue_hauteur - fils_degagement + fils_arrondi,
        ) * Rot(90, 0, 0) * Cylinder(
            fils_arrondi, fy1 - fy0, align=(Align.CENTER,) * 3
        )

    # --- potence et bossage de la butée de surcharge ---
    boss_r = insert_m25_diametre / 2.0 + 2.0
    body += _bb(
        butee_x - boss_r - 2.0, rail_x_in + 2.0, butee_y - boss_r, butee_y + boss_r,
        0.0, socle_epaisseur,
    )
    body += Pos(butee_x, butee_y, 0) * Cylinder(
        boss_r, butee_boss_hauteur, align=(Align.CENTER, Align.CENTER, Align.MIN)
    )

    # --- plaque repère du module d'amplification, simple fond ---
    body += _bb(
        plaque_x, -cage_x_out + 2.0,
        barre_y - plaque_profondeur / 2.0, barre_y + plaque_profondeur / 2.0,
        0.0, plaque_epaisseur,
    )

    # --- le socle enjambe le cercle de perforations, il ne s'y pose jamais ---
    # Deux cuvettes, pas une. Partout le socle s'écarte de marge_perforations du
    # Ø80. Sur l'emprise du berceau il s'arrête au Ø80 nu : la tête fraisée
    # intérieure, à 46 mm du centre du cercle, dégage les trous de 1,5 mm toute
    # seule, alors que la marge de 2 mm la couperait et laisserait 0,4 mm de
    # section. Rien n'est posé sur une perforation dans les deux cas.
    cbot = (Align.CENTER, Align.CENTER, Align.MIN)
    emprise = _bb(
        -barre_l / 2.0 - 3.5, berceau_x_int + 1.0,
        barre_y - joue_ext - 0.5, barre_y + joue_ext + 0.5,
        -1.5, cage_hauteur + 2.5,
    )
    large = Pos(0, perfo_y, -1.0) * Cylinder(
        perfo_r + marge_perforations, cage_hauteur + 2.0, align=cbot
    )
    etroit = Pos(0, perfo_y, -1.0) * Cylinder(
        perfo_r, cage_hauteur + 2.0, align=cbot
    )
    body -= large - emprise
    body -= etroit.intersect(emprise)

    # --- percages ---
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)
    # Vis M4 fraisées ISO 10642 par le dessous. Le cône à 90° débouche EXACTEMENT
    # sur le plan de pose de la barre : s'il s'arrête plus bas il laisse un
    # anneau en biseau de quelques dixièmes sous le siège, et s'il descend
    # jusqu'au lit il y couche une arête vive. Sous le cône, une amorce droite
    # au Ø de tête noie la tête sous la face d'appui du socle.
    cone_h = (tete_d - vis_fraisee_passage) / 2.0
    if berceau_epaisseur < cone_h + 0.5:
        reject(
            f"berceau_epaisseur {berceau_epaisseur} ne loge pas le cône d'une tête "
            f"fraisée M4 ({cone_h:.2f} mm) plus 0,5 mm de fût : monte-la au-delà de "
            f"{cone_h + 0.5:.1f}",
            param="berceau_epaisseur",
        )
    # Fraisage naturel : le cône part du lit, le fût Ø4,5 le prolonge jusqu'au
    # plan de pose. C'est la seule des trois dispositions qui ne fabrique pas
    # d'arête en biseau. Elle coûte deux baselines assumées sur la carte, le
    # biseau de première couche et l'anneau de 0,77 mm qui reste au-dessus de la
    # tête — les 0,8 mm annoncés dans la pile verticale, en compression pure.
    for x in (-trou_ext, -trou_int):
        body -= Pos(x, barre_y, -0.5) * Cylinder(
            vis_fraisee_passage / 2.0, berceau_epaisseur + 1.0, align=cmin
        )
        body -= Pos(x, barre_y, 0.0) * Cone(
            tete_d / 2.0, vis_fraisee_passage / 2.0, cone_h, align=cmin
        )
    # Poches borgnes sur les têtes Ø7, percées dans la FACE NORD. Tunnel carré,
    # fermé dessus et dessous : le fond de 1,3 mm est de la première couche et le
    # plafond est un pont plat de 7,4 mm. Fermer les deux est ce qui empêche
    # l'eau du bac d'entrer par le dessous du bossage.
    for s in (-1.0, 1.0):
        x0, x1 = sorted((s * (vis_x - poche_r), s * (vis_x + poche_r)))
        body -= _bb(
            x0, x1, arriere - 1.0, arriere + poche_profondeur, poche_z0, poche_z1
        )
    # Puits d'aimant ouverts vers le HAUT : la pièce s'imprime dans sa position
    # d'usage, donc le fond de 0,6 mm est la première couche et c'est lui qui va
    # contre la tôle du bac. L'aimant s'enfile par le dessus après le retrait.
    for s in (-1.0, 1.0):
        body -= Pos(s * pied_x, pied_y, puit_fond) * Cylinder(
            puit_diametre / 2.0, socle_epaisseur - puit_fond + 0.5, align=cmin
        )
    # Les deux aimants d'angle, mêmes puits, même sens : ils passent la tenue
    # verticale de deux à quatre points, ce que demande une pompe vibratoire.
    for s in (-1.0, 1.0):
        body -= Pos(s * aimant_coin_x, aimant_coin_y, puit_fond) * Cylinder(
            aimant_r, socle_epaisseur - puit_fond + 0.5, align=cmin
        )
    # Puits horizontal de l'aimant de paroi. La peau puit_fond reste sur la face
    # NORD, contre la tôle, comme dans support_2x5w ; le puits est donc ouvert
    # côté baie et l'aimant s'y enfile après l'impression, poussé au fond. Le
    # jeu qui reste derrière lui est ce qui permet de le ressortir un jour.
    body -= Plane(
        origin=(0.0, -bac_y + puit_fond, aimant_paroi_z), z_dir=(0.0, 1.0, 0.0)
    ) * Cylinder(aimant_r, (cage_y_in + bac_y - puit_fond) + 0.1, align=cmin)
    # Puits de l'insert laiton M2,5, borgne et ouvert vers le HAUT : l'insert
    # s'emmanche au fer par le dessus, la vis sans tête se règle par le dessus
    # elle aussi, plateau retiré. Le trou vaut le Ø hors tout du moletage moins
    # 0,6 : c'est le plastique déplacé qui fait la tenue.
    body -= Pos(butee_x, butee_y, butee_boss_hauteur - insert_m25_longueur - 0.4) * (
        Cylinder(insert_m25_diametre / 2.0, insert_m25_longueur + 0.5, align=cmin)
    )

    body = _fuse_one(body)
    if draft:
        return body

    # Seuls les angles extérieurs du contour se chanfreinent : ce sont les seuls
    # qu'on manipule, et une joue de 2,5 mm n'a pas la place de deux chanfreins.
    coins = [
        (-cage_x_out, arriere), (cage_x_out, arriere),
        (-cage_x_out, pied_y + rail_debord), (cage_x_out, pied_y + rail_debord),
        (-patte_x_in, pied_y + rail_debord),
        (patte_x_in, pied_y + rail_debord),
        (plaque_x, barre_y - plaque_profondeur / 2.0),
        (plaque_x, barre_y + plaque_profondeur / 2.0),
    ]
    conc = {
        (round(e.center().X, 2), round(e.center().Y, 2), round(e.center().Z, 2))
        for e in concave_edges(body)
    }

    def garde(edge):
        c = edge.center()
        if (round(c.X, 2), round(c.Y, 2), round(c.Z, 2)) in conc:
            return False
        return any(abs(c.X - x) < 0.6 and abs(c.Y - y) < 0.6 for x, y in coins)

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(garde), 1.0)


def _fuse_one(shape):
    solids = list(shape.solids())
    if len(solids) <= 1:
        return shape
    body = solids[0]
    for s in solids[1:]:
        body = body.fuse(s)
    return body

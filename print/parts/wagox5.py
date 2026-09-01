from nurb import *

# Dual Wago 221 holder; hook sits 0.2 mm above the connector top.

N_BORNES = 2
EPAISSEUR_RENFORT = 1.6
BEC_RENFORT = 2.0
# Vertical inner catch at the top of the lip. The 45° is below this,
# so the hook is not a knife-edge (min_wall 0.36 mm / 0.26 mm² end faces).
EPAISSEUR_LEVRE = 1.0


@part
def wagox5(
    largeur_borne=30.0,
    epaisseur_borne=8.0,
    jeu=-0.2,
    epaisseur_base=2.0,
    extension_base=9.0,
    hauteur_jupe=4.0,
    epaisseur_jupe=1.2,
    epaisseur_mur=1.2,
    surplomb=1.0,
    jeu_sommet=0.2,
    hauteur_renfort=18.8,
    portee_renfort=1.6,
    draft=False,
):
    """Logement pour deux Wago 221, fils vers le haut, verrouillage au sommet.

    largeur_borne: largeur d'une borne en X (30 = 221-415, 18.8 = 221-423)
    epaisseur_borne: épaisseur d'une borne au fond du berceau, sans le levier (8 mm)
    jeu: press-fit (négatif) ; berceau = 2×epaisseur_borne + jeu, faces = largeur_borne + jeu
    epaisseur_base: semelle
    extension_base: prolongement de la semelle de chaque côté (à la place des demi-disques)
    hauteur_jupe: jupes sur la longueur, au-dessus de la semelle, sans bloquer les leviers
    epaisseur_jupe: épaisseur des jupes
    epaisseur_mur: murs de clip sur la largeur ; plus mince = plus souple
    surplomb: dépassement de la lèvre vers l'intérieur (1 mm de palier, 45° en-dessous)
    jeu_sommet: les murs de clip dépassent le sommet de la borne d'autant (0,2 mm pour clipper dessus)
    hauteur_renfort: jusqu'où les goussets montent sur le mur (18.8 = jusqu'en haut)
    portee_renfort: saillie des goussets derrière le mur (étroit, comme l'original)
    """
    profondeur = measured("wago_profondeur")
    zmin = (Align.CENTER, Align.CENTER, Align.MIN)

    if largeur_borne < 12.0:
        reject(
            f"largeur_borne {largeur_borne} est sous 12 mm : ce n'est plus une Wago 221. "
            f"Remonte au-dessus de 12",
            param="largeur_borne",
        )
    if jeu < -0.5:
        reject(
            f"jeu {jeu} est sous -0.5 mm : la borne n'entre plus. Remonte au-dessus de -0.5",
            param="jeu",
        )
    if jeu > 0.3:
        reject(
            f"jeu {jeu} est au-dessus de 0.3 mm : la borne flotte. Descends sous 0.3",
            param="jeu",
        )
    if epaisseur_borne < 6.0:
        reject(
            f"epaisseur_borne {epaisseur_borne} est sous 6 mm : ce n'est plus une Wago 221. "
            f"Remonte au-dessus de 6",
            param="epaisseur_borne",
        )
    if epaisseur_mur < 1.0:
        reject(
            f"epaisseur_mur {epaisseur_mur} est sous 1 mm : le mur ne s'imprime plus. "
            f"Remonte au-dessus de 1",
            param="epaisseur_mur",
        )
    if epaisseur_jupe < 1.0:
        reject(
            f"epaisseur_jupe {epaisseur_jupe} est sous 1 mm : remonte au-dessus de 1",
            param="epaisseur_jupe",
        )
    if surplomb < EPAISSEUR_LEVRE:
        reject(
            f"surplomb {surplomb} is under {EPAISSEUR_LEVRE:.1f} mm: the catch "
            f"is thinner than the printer lays. Raise it above {EPAISSEUR_LEVRE:.1f}",
            param="surplomb",
        )
    if hauteur_jupe > profondeur * 0.45:
        reject(
            f"hauteur_jupe {hauteur_jupe} monte trop : les leviers s'ouvrent sur les "
            f"longs côtés. Descends sous {profondeur * 0.45:.1f}",
            param="hauteur_jupe",
        )
    if portee_renfort > extension_base - 0.5:
        reject(
            f"portee_renfort {portee_renfort} dépasse l'extension de semelle "
            f"({extension_base} mm). Descends sous {extension_base - 0.5:.1f}",
            param="portee_renfort",
        )
    if jeu_sommet < 0.0:
        reject(
            f"jeu_sommet {jeu_sommet} est négatif : le crochet rentre dans la borne. "
            f"Remets au-dessus de 0",
            param="jeu_sommet",
        )
    if jeu_sommet > 1.0:
        reject(
            f"jeu_sommet {jeu_sommet} est au-dessus de 1 mm : le crochet ne clippe plus. "
            f"Descends sous 1",
            param="jeu_sommet",
        )
    if hauteur_renfort > profondeur + jeu_sommet:
        reject(
            f"hauteur_renfort {hauteur_renfort} dépasse le mur "
            f"({profondeur + jeu_sommet:.1f} mm). "
            f"Descends sous {profondeur + jeu_sommet:.1f}",
            param="hauteur_renfort",
        )
    if hauteur_renfort < BEC_RENFORT + 2.0:
        reject(
            f"hauteur_renfort {hauteur_renfort} est trop bas pour un gousset tronqué. "
            f"Monte au-dessus de {BEC_RENFORT + 2.0:.1f}",
            param="hauteur_renfort",
        )

    inner_x = largeur_borne + jeu
    inner_y = N_BORNES * epaisseur_borne + jeu
    wall_top = epaisseur_base + profondeur + jeu_sommet
    body_x = inner_x + 2.0 * epaisseur_mur
    body_y = inner_y + 2.0 * epaisseur_jupe
    base_x = body_x + 2.0 * extension_base

    base = Box(base_x, body_y, epaisseur_base, align=zmin)

    walls = None
    for sign in (-1.0, 1.0):
        wall = Pos(sign * (inner_x / 2.0 + epaisseur_mur / 2.0), 0, 0) * Box(
            epaisseur_mur, body_y, wall_top, align=zmin
        )
        walls = wall if walls is None else walls + wall

    skirts = None
    for sign in (-1.0, 1.0):
        skirt = Pos(0, sign * (inner_y / 2.0 + epaisseur_jupe / 2.0), 0) * Box(
            body_x, epaisseur_jupe, epaisseur_base + hauteur_jupe, align=zmin
        )
        skirts = skirt if skirts is None else skirts + skirt

    clips = None
    for sign in (-1.0, 1.0):
        ix = sign * (inner_x / 2.0)
        ox = sign * (inner_x / 2.0 + epaisseur_mur)
        hook = ix - sign * surplomb
        z_catch = wall_top - EPAISSEUR_LEVRE
        z_45 = z_catch - surplomb
        # 0.2 mm above the Wago top, 1 mm vertical catch, 45° lead-in below.
        # Overlaps the wall (ix→ox) so it fuses; the 45° starts on the inner face.
        pts = (
            [
                (ix, z_45),
                (hook, z_catch),
                (hook, wall_top),
                (ox, wall_top),
                (ox, z_45),
            ]
            if sign > 0
            else [
                (ix, z_45),
                (ox, z_45),
                (ox, wall_top),
                (hook, wall_top),
                (hook, z_catch),
            ]
        )
        clip = Pos(0, -inner_y / 2.0, 0) * extrude(
            Plane.XZ * Polygon(*pts, align=None), inner_y
        )
        clips = clip if clips is None else clips + clip

    ribs = None
    half_y = body_y / 2.0
    inset = EPAISSEUR_RENFORT / 2.0 + 0.8
    ys = (-(half_y - inset), half_y - inset)
    for sign in (-1.0, 1.0):
        ox = sign * (inner_x / 2.0 + epaisseur_mur)
        # Bite 0.2 mm into the wall and the base so the gousset is one solid.
        wx = ox - sign * 0.2
        z0 = epaisseur_base - 0.2
        pts = (
            [
                (wx, z0),
                (ox + sign * portee_renfort, z0),
                (ox + sign * portee_renfort, z0 + BEC_RENFORT + 0.2),
                (wx, z0 + hauteur_renfort + 0.2),
            ]
            if sign > 0
            else [
                (wx, z0),
                (wx, z0 + hauteur_renfort + 0.2),
                (ox + sign * portee_renfort, z0 + BEC_RENFORT + 0.2),
                (ox + sign * portee_renfort, z0),
            ]
        )
        profile = Plane.XZ * Polygon(*pts, align=None)
        for y in ys:
            rib = Pos(0, y, 0) * extrude(profile, EPAISSEUR_RENFORT / 2.0, both=True)
            ribs = rib if ribs is None else ribs + rib

    body = base + walls + skirts + clips + ribs
    if draft:
        return body

    bed = body.bounding_box().min.Z
    hx = base_x / 2.0
    hy = body_y / 2.0
    body_hx = body_x / 2.0

    def wing_outline(edge):
        bb = edge.bounding_box()
        if bb.max.Z < bed + 0.05:
            return False
        cx = abs((bb.min.X + bb.max.X) / 2.0)
        cy = abs((bb.min.Y + bb.max.Y) / 2.0)
        if cx < body_hx + 0.5:
            return False
        return abs(cx - hx) < 0.4 or abs(cy - hy) < 0.4

    keep = body.edges().filter_by(wing_outline)
    keep = keep - concave_edges(body)
    return polish(body, keep, 1.0)

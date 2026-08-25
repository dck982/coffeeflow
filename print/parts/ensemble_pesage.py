from nurb import *


@assembly
def ensemble_pesage(
    hauteur_pile=18.0,
    berceau_epaisseur=3.0,
    barre_y=-25.0,
    cadre_largeur=110.0,
    cadre_profondeur=80.0,
    cadre_centre_y=-22.0,
    nervure_hauteur=10.0,
    tray_epaisseur=1.0,
):
    """Les deux pièces PETG en place au fond du bac, avec la barre de charge et le tray en obstacles.

    Le repère est celui du bac : origine au centre du fond, X à droite (±96),
    Y vers l'avant (±70), Z vers le haut. Le plateau est modélisé dans sa
    position d'impression (tablier sur le lit) ; c'est ici qu'il se retourne.

    hauteur_pile: hauteur du plan d'appui du tray, le sommet de la pile verticale
    berceau_epaisseur: épaisseur du berceau, qui fixe le dessous de la barre
    barre_y: axe de la barre de charge
    cadre_largeur: largeur du plateau, passée aux deux pièces
    cadre_profondeur: profondeur du plateau, passée aux deux pièces
    cadre_centre_y: centre en Y du plateau, passé aux deux pièces
    nervure_hauteur: profondeur des nervures, passée aux deux pièces
    tray_epaisseur: tôle du drip tray, seulement pour l'obstacle
    """
    barre_l = measured("barre_longueur")
    barre_s = measured("barre_section")
    trou_int = measured("barre_trou_interieur")
    trou_ext = measured("barre_trou_exterieur")

    commun = dict(
        cadre_largeur=float(cadre_largeur),
        cadre_profondeur=float(cadre_profondeur),
        cadre_centre_y=float(cadre_centre_y),
        nervure_hauteur=float(nervure_hauteur),
        barre_y=float(barre_y),
    )
    socle = use("base_pesage", hauteur_pile=float(hauteur_pile),
                berceau_epaisseur=float(berceau_epaisseur), **commun)
    # Rotation de 180° autour de Y, pas de X : elle remet le tablier en haut
    # sans inverser Y, et c'est en X que le plateau est dissymétrique (sangle,
    # fenêtre). Le X local du plateau est donc l'inverse de celui du bac.
    plateau = (
        Pos(0, 0, float(hauteur_pile))
        * Rot(0, 180, 0)
        * use("plateau_pesage", **commun)
    )

    cmid = (Align.CENTER, Align.CENTER, Align.MIN)
    barre = Pos(0, float(barre_y), float(berceau_epaisseur)) * Box(
        barre_l, barre_s, barre_s, align=cmid
    )
    # Pâte silicone, au centre seulement : c'est elle qui interdit au tablier de
    # passer au-dessus de la barre, et donc elle qui impose la fenêtre.
    silicone = Pos(0, float(barre_y), float(berceau_epaisseur) + barre_s) * Box(
        30.0, barre_s, 1.0, align=cmid
    )
    dessus_barre = float(berceau_epaisseur) + barre_s
    goujons = None
    for x in (trou_int, trou_ext):
        g = Pos(x, float(barre_y), dessus_barre) * Cylinder(
            2.0, float(hauteur_pile) - dessus_barre, align=cmid
        )
        goujons = g if goujons is None else goujons + g

    tray = Pos(0, 0, float(hauteur_pile)) * Box(
        166.0, 140.0, float(tray_epaisseur), align=cmid
    )

    return (
        socle,
        plateau,
        obstacle(barre, "barre de charge 5 kg, 75,5 x 12,7 x 12,7"),
        obstacle(silicone, "pâte silicone centrale, 1 mm au-dessus de la barre"),
        obstacle(goujons, "deux goujons M4 sans tête, bout chargé"),
        obstacle(tray, "drip tray, tôle 166 x 140 posée sur le plateau"),
    )

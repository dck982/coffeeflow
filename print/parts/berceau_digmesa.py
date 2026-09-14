from nurb import *


@part
def berceau_digmesa(jeu_corps=0.3, jeu_pin=0.25, profondeur_appui=2.0,
                    epaisseur_paroi=3.0,
                    berceau_taquet_x=3,
                    jeu_taquet=0.2,
                    surelevation_berceau=17.0,
                    draft=False):
    """Coupon d'appui du Digmesa ; raccords orientés vers +X.

    jeu_corps: Jeu diamétral autour du corps inférieur Ø32.
    jeu_pin: Jeu diamétral ajouté aux deux pins du débitmètre.
    profondeur_appui: Hauteur du rebord autour du bas du capteur.
    epaisseur_paroi: Épaisseur radiale du rebord extérieur.
    berceau_taquet_x: Longueur des taquets anti-rotation
    jeu_taquet: Jeu total ajouté en X, Y et Z aux encoches des taquets.
    surelevation_berceau: Hauteur ajoutée sous l'assise du Digmesa.
    """
    if jeu_corps < 0.3:
        reject("Le jeu diamétral autour du corps doit être au moins 0,3 mm.", param="jeu_corps")
    if jeu_pin < 0.1:
        reject("Le jeu diamétral des pins doit être au moins 0,1 mm.", param="jeu_pin")
    if not 1.5 <= profondeur_appui <= 3.0:
        reject("L'appui doit rester entre 1,5 et 3 mm pour dégager le raccord inférieur.", param="profondeur_appui")
    if epaisseur_paroi < 2.5:
        reject("La paroi doit mesurer au moins 2,5 mm.", param="epaisseur_paroi")
    if not 0.1 <= jeu_taquet <= 0.8:
        reject("Le jeu des taquets doit rester entre 0,1 et 0,8 mm.", param="jeu_taquet")
    if not 12.0 <= surelevation_berceau <= 17.0:
        reject("Surélévation entre 12 et 17 mm pour placer la sortie basse à Z=35…40.",param="surelevation_berceau")
    rayon = (measured("digmesa_corps_diametre") + jeu_corps) / 2
    # 1 mm sous le pin : il ne porte jamais le poids du capteur.
    assise = surelevation_berceau + measured("digmesa_pin_longueur") + 1.0
    hauteur = assise + profondeur_appui
    body = Cylinder(rayon + epaisseur_paroi, hauteur,
                    align=(Align.CENTER, Align.CENTER, Align.MIN))
    # Fût creux, ouvert dessous. Le cutter cylindrique devient un cône à 45°
    # et se ferme juste sous la zone des pins : aucune grande voûte horizontale
    # n'est imprimée dans le vide. À 17 mm, la paroi droite reste à 3 mm.
    # Retrait supplémentaire autour des tunnels de taquets : leur extrémité
    # intérieure conserve au moins 1 mm de matière avant la cavité.
    hollow_radius=min(rayon-1.2,surelevation_berceau-1.0)
    hollow_straight=surelevation_berceau-hollow_radius
    body -= Pos(0,0,-1)*Cylinder(
        hollow_radius,hollow_straight+1,
        align=(Align.CENTER,Align.CENTER,Align.MIN))
    body -= Pos(0,0,hollow_straight)*Cone(
        hollow_radius,0,hollow_radius,
        align=(Align.CENTER,Align.CENTER,Align.MIN))
    if not draft:
        body = polish(body, body.edges().filter_by(
            lambda e: abs(e.center().Z - hauteur) < 1e-6), 1.0)
    body -= Pos(0, 0, assise) * Cylinder(rayon, profondeur_appui + 1,
                    align=(Align.CENTER, Align.CENTER, Align.MIN))
    # Pin central étagé : depuis la face d'appui à Z=assise, Ø3,85 sur
    # 2,5 mm, puis Ø4,25. Le jeu reste diamétral et commun aux deux alésages.
    pin_haut = measured("digmesa_pin_hauteur_haut")
    z_marche = assise - pin_haut
    pin_bottom = surelevation_berceau + 1.0
    rayon_large = (measured("digmesa_pin_diametre_max") + jeu_pin) / 2
    rayon_petit = (measured("digmesa_pin_diametre_haut") + jeu_pin) / 2
    # Un raccord à 45° de 0,2 mm évite un plafond circulaire au bas du Ø3,85.
    # Il retire légèrement plus de matière que le pin : le fit reste libre.
    raccord = rayon_large - rayon_petit
    body -= Pos(0, 0, pin_bottom) * Cylinder(
        rayon_large, z_marche - pin_bottom,
        align=(Align.CENTER, Align.CENTER, Align.MIN))
    body -= Pos(0, 0, z_marche) * Cone(
        rayon_large, rayon_petit, raccord,
        align=(Align.CENTER, Align.CENTER, Align.MIN))
    body -= Pos(0, 0, z_marche + raccord) * Cylinder(
        rayon_petit,
        pin_haut + 1 - raccord,
        align=(Align.CENTER, Align.CENTER, Align.MIN))
    body -= Pos(0, -measured("digmesa_pin_second_decalage"), pin_bottom) * Cylinder(
        (measured("digmesa_pin_second_diametre") + jeu_pin) / 2, hauteur - pin_bottom + 1,
        align=(Align.CENTER, Align.CENTER, Align.MIN))
    # Encoches d'indexage pour les taquets du support. Le rectangle bas donne
    # au taquet son jeu X, Y et Z. Son plafond est remplacé par un toit à 45° :
    # un triangle YZ extrudé dans X, soit l'axe du tunnel. Il n'y a plus de
    # plafond horizontal à imprimer au-dessus de l'encoche.
    ouverture_x = berceau_taquet_x + jeu_taquet
    ouverture_y = measured("digmesa_taquet_y") + jeu_taquet
    ouverture_z = measured("digmesa_taquet_z") + jeu_taquet
    x_min = body.bounding_box().min.X
    x_max = body.bounding_box().max.X
    roof = Plane.YZ * Polygon(
        (-ouverture_y / 2, ouverture_z),
        (ouverture_y / 2, ouverture_z),
        (0, ouverture_z + ouverture_y / 2),
        align=None,
    )
    for x_exterieur, longueur in (
        (x_min, ouverture_x),
        (x_max, -ouverture_x),
    ):
        body -= Pos(x_exterieur, 0, 0) * Box(
            ouverture_x,
            ouverture_y,
            ouverture_z,
            align=(Align.MIN if longueur > 0 else Align.MAX, Align.CENTER, Align.MIN),
        )
        body -= Pos(x_exterieur, 0, 0) * extrude(roof, longueur)

    return body

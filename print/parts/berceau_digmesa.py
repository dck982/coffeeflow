from nurb import *


@part
def berceau_digmesa(jeu_corps=0.5, jeu_pin=0.5, profondeur_appui=2.0,
                    epaisseur_paroi=3.0, draft=False):
    """Coupon d'appui du Digmesa ; raccords orientés vers +X.

    jeu_corps: Jeu diamétral autour du corps inférieur Ø32.
    jeu_pin: Jeu diamétral ajouté aux deux pins du débitmètre.
    profondeur_appui: Hauteur du rebord autour du bas du capteur.
    epaisseur_paroi: Épaisseur radiale du rebord extérieur.
    """
    if jeu_corps < 0.3 or jeu_pin < 0.3:
        reject("Les jeux diamétraux doivent être au moins 0,3 mm.")
    if not 1.5 <= profondeur_appui <= 3.0:
        reject("L'appui doit rester entre 1,5 et 3 mm pour dégager le raccord inférieur.", param="profondeur_appui")
    if epaisseur_paroi < 2.5:
        reject("La paroi doit mesurer au moins 2,5 mm.", param="epaisseur_paroi")
    rayon = (measured("digmesa_corps_diametre") + jeu_corps) / 2
    # 1 mm sous le pin : il ne porte jamais le poids du capteur.
    assise = measured("digmesa_pin_longueur") + 1.0
    hauteur = assise + profondeur_appui
    body = Cylinder(rayon + epaisseur_paroi, hauteur,
                    align=(Align.CENTER, Align.CENTER, Align.MIN))
    if not draft:
        body = polish(body, body.edges().filter_by(
            lambda e: abs(e.center().Z - hauteur) < 1e-6), 1.0)
    body -= Pos(0, 0, assise) * Cylinder(rayon, profondeur_appui + 1,
                    align=(Align.CENTER, Align.CENTER, Align.MIN))
    body -= Pos(0, 0, -1) * Cylinder(
        (measured("digmesa_pin_diametre_max") + jeu_pin) / 2, hauteur + 2,
        align=(Align.CENTER, Align.CENTER, Align.MIN))
    body -= Pos(0, -measured("digmesa_pin_second_decalage"), -1) * Cylinder(
        (measured("digmesa_pin_second_diametre") + jeu_pin) / 2, hauteur + 2,
        align=(Align.CENTER, Align.CENTER, Align.MIN))
    return body

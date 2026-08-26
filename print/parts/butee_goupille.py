from nurb import *


@part
def butee_goupille(
    goupille_hauteur=5.55,
    goupille_diametre=3.5,
    goupille_chanfrein=0.3,
    puits_diametre=4.0,
    puits_profondeur=4.4,
    draft=False,
):
    """Goupille imprimée de la butée de surcharge : elle tombe dans le puits de `base_pesage` et se ponce à la cote.

    goupille_hauteur: hauteur de la goupille, la seule cote qui compte — on l'imprime trop longue et on la ponce
    goupille_diametre: Ø de la goupille, glissant dans le puits du socle
    goupille_chanfrein: chanfrein du bout haut, celui qui vient sous le plateau
    puits_diametre: Ø du puits de `base_pesage` (butee_puits_diametre) — à garder en phase
    puits_profondeur: profondeur de ce puits, butee_boss_hauteur moins butee_puits_fond
    """
    saillie = goupille_hauteur - puits_profondeur

    if goupille_diametre > puits_diametre - 0.3:
        reject(
            f"goupille_diametre {goupille_diametre} ne laisse pas 0,3 mm de jeu dans un "
            f"puits de {puits_diametre} : la goupille ne tomberait pas au fond et ne se "
            f"reprendrait plus à la main. Descends-la sous {puits_diametre - 0.3:.1f}",
            param="goupille_diametre",
        )
    if saillie < 0.3:
        reject(
            f"goupille_hauteur {goupille_hauteur} ne sort que de {saillie:.2f} mm d'un "
            f"puits de {puits_profondeur} : elle n'arrête rien. Monte-la au-delà de "
            f"{puits_profondeur + 0.3:.2f}",
            param="goupille_hauteur",
        )
    if saillie > 1.5:
        reject(
            f"goupille_hauteur {goupille_hauteur} sort de {saillie:.2f} mm du puits, "
            f"soit plus que les 1,5 mm au-delà desquels elle bascule au lieu de porter — "
            f"et si la pile réelle demande vraiment ça, c'est hauteur_pile de "
            f"`base_pesage` qu'il faut reprendre, pas la goupille. Descends-la sous "
            f"{puits_profondeur + 1.5:.2f}",
            param="goupille_hauteur",
        )

    # Cylindre plein, imprimé debout : les deux faces plates sont des plans de
    # référence (le bas pose au fond du puits, le haut prend le plateau), et la
    # charge les traverse en compression pure, jamais en flexion. Elle est sous
    # le double de son Ø, donc ce n'est pas le `pin` de la doctrine.
    body = Cylinder(
        goupille_diametre / 2.0, goupille_hauteur, align=(Align.CENTER,) * 2 + (Align.MIN,)
    )
    if draft:
        return body

    # Seul le bout HAUT se chanfreine : le bout bas est la première couche, et la
    # doctrine interdit de modéliser autour du pied d'éléphant.
    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed)
    return polish(body, keep, goupille_chanfrein)

from nurb import *

from system import barreau_filete


@part
def ecrou_passe_cable(
    plats=22.0,
    epaisseur=5.0,
    diametre_fut=15.8,
    pas_filet=2.0,
    profondeur_filet=0.5,
    jeu_filet=0.3,
    amorce_entree=0.8,
    draft=False,
):
    """Écrou hex SW22 fileté, assorti au passe_cable (profil dents de scie, pas 2.0).

    Hors-tout repris d'un écrou de presse-étoupe trouvé en ligne (22×24.8×5),
    mais au pas 2.0 et non 1.41 : ce n'est pas un écrou du commerce et il ne se
    visse que sur le passe_cable imprimé.

    plats: distance entre deux plats
    epaisseur: hauteur de l'écrou
    diametre_fut: major du mâle
    pas_filet: pas du filet
    profondeur_filet: profondeur radiale d'une dent
    jeu_filet: jeu diamétral mâle/femelle
    amorce_entree: cône d'entrée à 45° (face du haut à l'impression, côté tôle
        à l'usage — on retourne l'écrou pour le visser)
    """
    if plats < diametre_fut + 4.0:
        reject(
            f"plats {plats} leaves under 2 mm of wall around the bore: raise it",
            param="plats",
        )
    if epaisseur < pas_filet * 2.0:
        reject(
            f"epaisseur {epaisseur} is under two turns of pitch {pas_filet}: "
            f"raise it",
            param="epaisseur",
        )
    if jeu_filet < 0.2:
        reject(
            f"jeu_filet {jeu_filet} is under 0.2 mm: raise it",
            param="jeu_filet",
        )
    if pas_filet < profondeur_filet + 0.3:
        reject(
            f"pas_filet {pas_filet} is too short for depth "
            f"{profondeur_filet}: raise it",
            param="pas_filet",
        )

    cmin = (Align.CENTER, Align.CENTER, Align.MIN)
    hex_face = RegularPolygon(plats / 2.0, 6, major_radius=False, rotation=30)
    body = extrude(hex_face, epaisseur)

    # No collar and no plain run on the cutter: a full-major cylinder at the
    # bottom counterbores the mouth to Ø16.1 and the first thread ridge then
    # starts 0.3 mm up on air (three `floating` fails). Threading from below
    # the bed grounds the first ridge on the first layer instead.
    cutter = barreau_filete(
        diametre_fut,
        pas_filet,
        profondeur_filet,
        epaisseur + 1.2,
        jeu_radial=jeu_filet / 2.0,
        start=0.0,
        collar_h=0.0,
    )
    body = body - Pos(0, 0, -0.3) * cutter

    # Lead-in cone at the TOP face as printed, not the bottom. The male enters
    # from the side that faces the sheet, so the nut is turned over to use it
    # (the assembly poses it that way). Cut into the bottom instead and the
    # cone lands a 45° knife edge on the first layer: bed_bevel, 41.5mm2.
    r_min = diametre_fut / 2.0 + jeu_filet / 2.0 - profondeur_filet
    if amorce_entree > 0.0:
        body = body - Pos(0, 0, epaisseur - amorce_entree) * Cone(
            r_min, r_min + amorce_entree, amorce_entree, align=cmin
        )
    solids = list(body.solids())
    if not solids:
        reject("nut boolean removed everything: check thread params")
    body = max(solids, key=lambda s: s.volume)

    apothem = plats / 2.0
    if draft:
        return body

    bed = body.bounding_box().min.Z

    def keep(edge):
        ebb = edge.bounding_box()
        if ebb.min.Z < bed - 0.05:
            return False
        mx = 0.5 * (ebb.min.X + ebb.max.X)
        my = 0.5 * (ebb.min.Y + ebb.max.Y)
        return (mx * mx + my * my) ** 0.5 > apothem - 0.2

    return polish(body, body.edges().filter_by(Axis.Z).filter_by(keep), 1.0)

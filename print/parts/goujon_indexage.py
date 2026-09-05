from nurb import *


@part
def goujon_indexage(
    tete_cote=10.0,
    tete_epaisseur=2.0,
    cle_cote=5.95,
    cle_longueur=2.0,
    fut_diametre=6.2,
    fut_longueur=4.0,
    draft=False,
):
    """Goujon anti-rotation pour la deuxième ouverture de `screen_base`, 20mm
    à droite du passe-câble : une clé carrée dans le trou carré 6x6 de
    screen_base (seul côté où une forme peut réellement bloquer la rotation),
    un fût rond dans le trou rond Ø6 de la tôle de la machine (un rond dans un
    rond ne bloque jamais rien géométriquement — seule la friction y résiste),
    et une tête carrée qui bute contre la paroi intérieure pour ne pas
    ressortir. Rien ne le retient à part cette butée : l'ensemble est déjà
    comprimé par l'écrou du `passe_cable` et les aimants, donc un goujon qui
    ne peut que buter (jamais traverser) suffit — pas besoin de cliquet.

    tete_cote: côté de la tête carrée, à l'intérieur de screen_base, qui bute
        contre la paroi et empêche le goujon de ressortir côté machine
    tete_epaisseur: épaisseur de cette tête
    cle_cote: côté de la clé carrée qui bloque la rotation dans le trou 6x6 de
        screen_base — 5.95, ajusté serré (quasi pas de jeu en rotation)
    cle_longueur: longueur de la clé, la paroi arrière de screen_base à
        traverser (2.0mm, son épaisseur `wall` par défaut)
    fut_diametre: diamètre du fût rond dans le trou Ø6 de la tôle machine —
        6.2, léger serrage : la seule résistance en rotation possible de ce
        côté, le trou étant rond
    fut_longueur: longueur du fût rond — la tôle machine (2.0mm) plus le
        dépassement à l'intérieur (2.0mm, la place libérée par l'ancienne LED)
    """
    if tete_cote < cle_cote + 2.0:
        reject(
            f"tete_cote {tete_cote} leaves under 1mm of shoulder around the "
            f"{cle_cote}mm key: raise it",
            param="tete_cote",
        )
    if cle_cote > 6.0:
        reject(
            f"cle_cote {cle_cote} is over the 6mm square opening it has to "
            "slide into: lower it",
            param="cle_cote",
        )
    if fut_diametre < cle_cote:
        reject(
            f"fut_diametre {fut_diametre} is under cle_cote {cle_cote}: the "
            "round shaft would be thinner than the key it stands on",
            param="fut_diametre",
        )

    cmin = (Align.CENTER, Align.CENTER, Align.MIN)
    body = Box(tete_cote, tete_cote, tete_epaisseur, align=cmin)
    body += Pos(0, 0, tete_epaisseur) * Box(
        cle_cote, cle_cote, cle_longueur, align=cmin
    )
    body += Pos(0, 0, tete_epaisseur + cle_longueur) * Cylinder(
        fut_diametre / 2.0, fut_longueur, align=cmin
    )

    if draft:
        return body

    bed = body.bounding_box().min.Z
    keep = body.edges().filter_by(lambda e: e.bounding_box().min.Z > bed + 0.05)
    keep = keep - concave_edges(body)
    return polish(body, keep, 1.0)

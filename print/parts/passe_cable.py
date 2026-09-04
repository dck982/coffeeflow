from nurb import *

from system import _fuse_one, barreau_filete


@part
def passe_cable(
    diametre_bride=20.0,
    epaisseur_bride=1.6,
    diametre_fut=15.8,
    diametre_passage=3.5,
    entraxe_passages=6.4,
    pas_filet=2.0,
    profondeur_filet=0.5,
    draft=False,
):
    """Passe-câble fileté : bride sous la tôle, fût mâle, deux câbles Ø3.1.

    Profil en dents de scie relevé sur un presse-étoupe trouvé en ligne, mais
    au pas 2.0 et non 1.41 : à 1.41 la crête ne fait plus que 0.12 mm, sous une
    extrusion. Le couple mâle/femelle est donc imprimé des deux côtés — aucun
    presse-étoupe du commerce ne s'y visse. Flanc porteur à 35° axial pour
    rester sous 45° après l'hélice.

    diametre_bride: disque qui porte contre la tôle
    epaisseur_bride: épaisseur du disque (z=0 au lit)
    diametre_fut: major du filet (passe le Ø16 tôle)
    diametre_passage: alésage de chaque câble
    entraxe_passages: distance entre les centres des deux alésages
    pas_filet: pas du filet (2.0 mm, choisi pour une crête imprimable)
    profondeur_filet: profondeur radiale d'une dent
    """
    ep_tole = measured("tole_epaisseur")
    ep_fond = measured("passe_cable_bride_epaisseur")
    ep_ecrou = measured("ecrou_m16_epaisseur")
    amorce = measured("passe_cable_amorce")
    cable = measured("cable_od_passe")
    trou = measured("passe_cable_trou")

    if diametre_bride < diametre_fut + 2.4:
        reject(
            f"diametre_bride {diametre_bride} leaves under 1.2 mm of flange "
            f"around the Ø{diametre_fut} barrel: raise it",
            param="diametre_bride",
        )
    if epaisseur_bride < 1.2:
        reject(
            f"epaisseur_bride {epaisseur_bride} is under 1.2 mm: raise it",
            param="epaisseur_bride",
        )
    if diametre_fut > trou - 0.2:
        reject(
            f"diametre_fut {diametre_fut} will not pass the Ø{trou} hole "
            f"with 0.2 mm snug: lower it",
            param="diametre_fut",
        )
    if diametre_passage < cable + 0.2:
        reject(
            f"diametre_passage {diametre_passage} is tighter than a {cable} mm "
            f"cable plus 0.2 mm snug: raise it",
            param="diametre_passage",
        )
    web = entraxe_passages - diametre_passage
    if web < 1.2:
        reject(
            f"entraxe_passages {entraxe_passages} leaves only {web:.2f} mm "
            f"between the bores: raise it",
            param="entraxe_passages",
        )
    bore_extent = entraxe_passages / 2.0 + diametre_passage / 2.0
    if diametre_fut / 2.0 - profondeur_filet - bore_extent < 1.0:
        reject(
            f"cable bores leave under 1 mm to the thread root: lower "
            f"entraxe_passages or profondeur_filet",
            param="entraxe_passages",
        )
    if pas_filet < profondeur_filet + 0.3:
        reject(
            f"pas_filet {pas_filet} is too short for depth "
            f"{profondeur_filet}: raise it",
            param="pas_filet",
        )

    h_col = ep_tole + ep_fond
    h_filet = ep_ecrou + amorce
    cmin = (Align.CENTER, Align.CENTER, Align.MIN)

    body = Cylinder(diametre_bride / 2.0, epaisseur_bride, align=cmin)
    body = body + Pos(0, 0, epaisseur_bride) * Cylinder(
        diametre_fut / 2.0, h_col, align=cmin
    )
    filet = barreau_filete(
        diametre_fut,
        pas_filet,
        profondeur_filet,
        h_filet,
        chanfrein_tete=profondeur_filet + 0.3,
    )
    body = _fuse_one(
        body + Pos(0, 0, epaisseur_bride + h_col - 0.2) * filet
    )

    # Flange rim chamfer BEFORE the cable bores: chamfering the rim on a body
    # that already carries the two through bores makes OCCT rebuild the solid
    # without them (measured: 2124mm3 polished against 1709mm3 draft, the two
    # Ø5.2 bores silently healed shut). Bores last is the same geometry and
    # survives.
    if not draft:

        def keep(edge):
            ebb = edge.bounding_box()
            if ebb.min.Z < epaisseur_bride - 0.2:
                return False
            if ebb.max.Z > epaisseur_bride + 0.2:
                return False
            span = max(ebb.max.X - ebb.min.X, ebb.max.Y - ebb.min.Y)
            return span > diametre_bride - 1.5

        body = polish(body, body.edges().filter_by(keep), 1.0)

    for sign in (-1.0, 1.0):
        body = body - Pos(sign * entraxe_passages / 2.0, 0, -0.5) * Cylinder(
            diametre_passage / 2.0,
            epaisseur_bride + h_col + h_filet + 1.0,
            align=cmin,
        )

    return body

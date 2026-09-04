from nurb import *

from system import passe_cable_body


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

    Le câble doit pouvoir être enfilé par le bout (fût ou écrou) : s'il est
    déjà terminé aux deux bouts, voir `passe_cable_demi_a` /
    `passe_cable_demi_b`, la même pièce fendue en deux pour se refermer
    autour d'un câble en place.

    diametre_bride: disque qui porte contre la tôle
    epaisseur_bride: épaisseur du disque (z=0 au lit)
    diametre_fut: major du filet (passe le Ø16 tôle)
    diametre_passage: alésage de chaque câble
    entraxe_passages: distance entre les centres des deux alésages
    pas_filet: pas du filet (2.0 mm, choisi pour une crête imprimable)
    profondeur_filet: profondeur radiale d'une dent
    """
    return passe_cable_body(
        diametre_bride,
        epaisseur_bride,
        diametre_fut,
        diametre_passage,
        entraxe_passages,
        pas_filet,
        profondeur_filet,
        draft,
    )

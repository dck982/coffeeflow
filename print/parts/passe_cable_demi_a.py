from nurb import *

from system import passe_cable_body, passe_cable_demi


@part
def passe_cable_demi_a(
    diametre_bride=25.0,
    epaisseur_bride=1.6,
    diametre_fut=15.8,
    diametre_passage=4.6,
    entraxe_passages=6.4,
    pas_filet=2.0,
    profondeur_filet=0.5,
    draft=False,
):
    """Moitié A du passe_cable fendu, pour un câble déjà terminé aux deux
    bouts (connecteurs sertis) qui ne peut plus être enfilé par le fût ou
    l'écrou.

    Coupé dans le plan qui porte les deux axes de trou : chaque moitié
    porte une demi-gorge par câble, à poser autour d'un câble déjà en place
    puis à refermer avec `passe_cable_demi_b`. `ecrou_passe_cable` (Ø16
    intérieur) s'enfile ensuite normalement par-dessus les connecteurs et
    maintient les deux moitiés serrées en se vissant sur le filet reconstitué.

    Le filet est une hélice à un seul départ : il n'est pas symétrique par
    rotation de 180°, donc A et B sont deux pièces différentes, pas la même
    imprimée deux fois. Toujours les utiliser en paire.

    diametre_bride: disque qui porte contre la tôle
    epaisseur_bride: épaisseur du disque (z=0 au lit)
    diametre_fut: major du filet (passe le Ø16 tôle)
    diametre_passage: alésage de chaque câble
    entraxe_passages: distance entre les centres des deux alésages
    pas_filet: pas du filet (2.0 mm, choisi pour une crête imprimable)
    profondeur_filet: profondeur radiale d'une dent
    """
    body = passe_cable_body(
        diametre_bride,
        epaisseur_bride,
        diametre_fut,
        diametre_passage,
        entraxe_passages,
        pas_filet,
        profondeur_filet,
        draft,
    )
    return passe_cable_demi(body, "a")

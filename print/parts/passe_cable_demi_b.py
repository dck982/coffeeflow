from nurb import *

from system import passe_cable_body, passe_cable_demi


@part
def passe_cable_demi_b(
    diametre_bride=20.0,
    epaisseur_bride=1.6,
    diametre_fut=15.8,
    diametre_passage=3.5,
    entraxe_passages=6.4,
    pas_filet=2.0,
    profondeur_filet=0.5,
    draft=False,
):
    """Moitié B du passe_cable fendu — se referme avec `passe_cable_demi_a`
    autour d'un câble déjà en place. Voir la docstring de `passe_cable_demi_a`
    pour le principe et les paramètres, identiques ici : les deux moitiés se
    règlent toujours ensemble.

    Le filet est une hélice à un seul départ : il n'est pas symétrique par
    rotation de 180°, donc B n'est pas A tournée de 180° — ce sont deux
    pièces différentes à imprimer et utiliser en paire.
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
    return passe_cable_demi(body, "b")

# Régulation en débit

Ce document garde l'analyse du 31 octobre 2026 sur un mode « flow control »,
alternative à la régulation en pression actuelle. **Rien n'est codé** :
David attend la pompe à engrenage ([pompe-engrenage.md](pompe-engrenage.md))
pour n'avoir qu'une implémentation à écrire et à régler. Les infusions
citées sont dans [journal-infusions.md](journal-infusions.md).

## Décisions

| Sujet | Décision |
| --- | --- |
| Quand | Implémentation avec la pompe à engrenage, pas avant |
| Pompe vibratoire, si un jour | Option **b** : débitmètre en boucle rapide, balance en correction lente de la consigne |
| Pompe à engrenage | Le débit suit la vitesse : consigne directe, corrigée du glissement (voir plus bas) |
| Pression | `brew.target_pressure_bar` devient un **plafond** en mode débit |
| Arrêt | Inchangé : poids cible moins `brew.stop_lead_g` |
| Ramp-down | Supprimé (0.4.25) ; il ne commandait rien, seule l'anticipation d'arrêt est restée |

## Pourquoi : la galette varie plus que la mouture

À réglages identiques, la résistance au plateau a varié de 6,0 bar·s/g
(30/10 14 h 44) à 5,2 bar·s/g (31/10 10 h 16), soit −14 %, dans les
± 18 % que le journal attribue au tassage. En pression, cette variation se
retrouve entière dans le débit, la durée et donc l'extraction : 1,47 g/s
puis 1,69 g/s. En débit, la durée et le ratio ne dépendent plus de la
galette ; c'est la pression qui varie.

## Le cas du 31 octobre à 10 h 16

Galette mesurée à 5,2 bar·s/g, régulée à 8,8 bar. Avec une consigne de
1,2 g/s (le débit qui a plu à David), en supposant une loi linéaire et un
arrêt vers 21 g :

| | Mesuré, en pression | Estimé, à 1,2 g/s |
| --- | ---: | ---: |
| Pression au plateau | 8,8 bar | **≈ 6,2 bar** (5,5 à 6,5) |
| Goutte → arrêt de la pompe | 13,4 s | **≈ 17 s** |
| Pompe en marche | 21,0 s | **≈ 25 s** |
| En tasse avant le plateau | 6,3 g | ≈ 2 à 3 g |

- Une galette se tasse moins à basse pression ; sa résistance baisse un
  peu, et la pression réelle serait plutôt dans le bas de la fourchette.
- La durée tombe dans les 25 à 27 s de pompe visées. La recette du
  torréfacteur, ramenée au panier LM1, donne ≈ 16 s de la goutte à
  l'arrêt pour 17,6 g : à 21 g, ≈ 17 s, comme ici.

## Ce que le débit corrige, et ce qu'il ne corrige pas

Estimation, non mesurée : une galette à 5,2 bar·s/g tenue à 1,2 g/s
s'approche du goût d'une galette à 7,4 bar·s/g qui coule à 1,2 g/s sous
9 bar, sans l'égaler.

- **Corrigé** : le temps de contact et l'eau par gramme de café sont les
  mêmes. C'est ce qui manquait à l'infusion « légère » du 31.
- **Pas corrigé** : la mouture plus grosse a moins de surface ; à temps
  égal, l'intérieur des particules s'extrait moins.
- **Peu d'effet de la pression elle-même** au-delà de ≈ 6 bar. Cameron
  et al. (2020, voir [turboshot.md](turboshot.md)) obtiennent même des
  extractions plus homogènes à 6 bar qu'à 9 bar, avec moins de colmatage.

Le débit rattrape un écart de galette ou une mouture un peu grosse ; il ne
remplace pas le réglage du moulin. Une consigne de débit qui fait tomber la
pression sous ≈ 5 bar dit que la mouture est trop grosse.

## Mesures disponibles

Relevé sur le plateau du 31/10 10 h 16 :

| Mesure | Qualité | Limite |
| --- | --- | --- |
| Débitmètre amont (3450 imp/L, ≈ 0,29 ml par impulsion, débit calculé d'un front à l'autre) | 1,63 ml/s, écart-type 0,03 ml/s ; ≈ 4 impulsions/s à 1,2 ml/s | Pendant la montée, il lit 3,5 ml/s quand la tasse reçoit ≈ 1,5 g/s : l'eau remplit la chambre au-dessus de la galette et la galette. Au plateau, tasse / amont = 1,00 à 1,04 |
| Balance | ± 6 % sur une fenêtre de 2 s | Muette avant la goutte ; retard des gouttes et des jets ; balance perdue possible |

## Pompe vibratoire : option b

Le débit d'une pompe vibratoire dépend fortement de la contre-pression ; il
faut une boucle fermée, réglée sur de vraies infusions.

- **Boucle rapide** sur le débitmètre amont (ml/s), à la période du PI de
  pression actuel (200 ms), commande de la pompe en %.
- **Correction lente** de la consigne de cette boucle par la balance : sur
  une fenêtre de 2 s, l'écart entre le débit en tasse (g/s) et la consigne
  ajuste la consigne amont, bornée à ± 20 %. Elle n'agit qu'après la
  première goutte et une fois la pression stabilisée.
- Avant la goutte, ou balance perdue : boucle amont seule, consigne
  amont = consigne en tasse (rapport 1,00 g/ml).

## Pompe à engrenage : consigne directe

La pompe donne 0,1 ml par tour ; la vitesse fixe le débit. Ce n'est pas
tout à fait gratuit : le **glissement** vaut 0,7 à 1 ml/s à 9 bar, de
l'ordre du débit visé. Il dépend surtout de la pression.

- Vitesse = (consigne + glissement(pression)) / 0,1 ml, avec la table de
  glissement relevée à réception (celle de la fiche est dans
  pompe-engrenage.md).
- Correction lente par la balance comme ci-dessus, pour absorber l'erreur
  de la table.
- Le débitmètre amont reste un contrôle ; il n'est plus nécessaire à la
  boucle.

## Règles communes

- **Plafond de pression.** La commande appliquée est le minimum de la
  commande en débit et de celle du PI de pression réglé sur
  `brew.target_pressure_bar`. Une galette trop fine plafonne à cette
  pression et coule moins vite que la consigne ; la capture le montre.
- **Pas de plancher de pression.** Une galette trop perméable coule à la
  consigne sous basse pression ; c'est un signal de mouture, pas une
  erreur.
- **Phases.** Remplissage et pré-infusion inchangés. La régulation en débit
  commence avec l'infusion ; la consigne monte de la pré-infusion à la
  valeur visée en ≈ 2 s pour éviter un coup de bélier.
- **Arrêt** au poids cible moins `brew.stop_lead_g`, comme en pression.

## Réglages prévus

| Clé | Rôle |
| --- | --- |
| `brew.control` | `pressure` (actuel) ou `flow` |
| `brew.target_flow_g_s` | consigne en tasse, g/s ; 0,8 à 2,5, pas de 0,05 |
| `brew.target_pressure_bar` | consigne en mode `pressure`, plafond en mode `flow` |

Ces clés entreront dans un nouveau schéma de configuration, avec la
pompe à engrenage.

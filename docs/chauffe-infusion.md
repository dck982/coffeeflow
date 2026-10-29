# Chauffe pendant l'infusion

Ce document décrit la loi de chauffe appliquée pendant une infusion, de la
précharge à l'arrêt de la pompe (version écran **0.3.39**), et les mesures qui
la justifient. Le code est dans `firmware/screen/main/core/brew_heating.h`.

## Périmètre

Deux lois de chauffe coexistent dans le firmware écran :

| Loi | Code | Phases |
| --- | --- | --- |
| **Loi d'infusion** (ce document) | `BrewHeating` dans `brew_heating.h` | précharge, remplissage, pré-infusion, infusion, rampe de fin |
| Régulateur de repos | `Controller` dans `thermal_control.h` | repos, purge, récupération après l'écoulement |

`Controller::step()` reste le point d'entrée unique. Il applique d'abord
les coupures générales (mesure invalide, chauffe désactivée, NTC au-delà de
100 °C), puis, pendant la précharge et l'écoulement d'une infusion,
**délègue la commande à `BrewHeating`**. Aucun élément du régulateur de
repos n'intervient alors : ni prédiction, ni intégrale (remise à zéro), ni
filtre de sortie. Il enregistre seulement la commande envoyée, qui lui sert
à estimer la chaleur en transit pendant la récupération.

Modifier le régulateur de repos (gains, prédiction, stabilisation à la
consigne) ne change donc pas l'infusion, et inversement. Seul point de
contact : la récupération, gérée par le régulateur de repos, démarre avec
l'historique de commande laissé par l'infusion.

## Objectif

L'utilisateur lit la consigne comme **la température moyenne de l'eau au
groupe pendant l'infusion**. Depuis l'écran 0.4.4, l'offset de −14 °C entre
la sonde et la température utilisateur (voir
[chauffe-chaudiere.md](chauffe-chaudiere.md#conversion-actuelle)) est mesuré
au panier : environ 104 °C à la sonde pour 90 °C au groupe.

Le critère est la **NTC en
température utilisateur, moyenne pondérée par la tasse** : chaque gramme
arrivé en tasse avant l'arrêt de la pompe compte avec la NTC du même
instant. Elle doit valoir la consigne. `analyze_hf_capture.py` la calcule
(poids en maximum courant, poids invalides ou négatifs ignorés).

| Choix | Raison |
| --- | --- |
| Pondération par la tasse plutôt que par le volume amont | seuls ≈ 21 g des ≈ 65 ml admis arrivent en tasse ; le reste remplit l'espace au-dessus de la galette et la mouille pendant le remplissage |
| NTC du même instant, sans décalage | le temps de passage chaudière → tasse n'est pas mesuré ; un décalage fixe se discutera avec la mesure au panier |
| Jusqu'à l'arrêt de la pompe | après, la tasse peut être retirée et le poids n'est plus exploitable |
| Creux et bosse sans critère propre | seule la moyenne est promise ; le minimum reste suivi, pour ne pas échanger un dépassement contre un creux profond |

L'offset de −10,5 °C, en vigueur jusqu'à 0.4.3, venait des essais de
*flashing* au repos. La sonde SCACE l'a recalé le 10 octobre 2026 : sur
trois infusions, la SCACE moyennée par la tasse était 3,4 à 9,9 K sous la
NTC utilisateur, et l'écart tendait vers 3 K avec un porte-filtre chaud
(voir [sonde-scace.md](sonde-scace.md#infusions-10-octobre-2026)). Les
moyennes en tasse ci-dessous sont dans l'ancien domaine jusqu'au
9 octobre : leur retirer 3,5 °C pour les comparer aux suivantes.

Écart au critère, consigne de 90 °C sauf mention :

| Infusion | Précharge | Tasse | Volume amont |
| --- | ---: | ---: | ---: |
| 29/09, 7 h 40 | 6 s | 87,20 °C | 88,81 °C |
| 28/09, 13 h 01 | 10 s (trou de chauffe) | 88,72 °C | 89,98 °C |
| 29/09, 14 h 24 | ≈ 8,7 s réelles | 91,30 °C | 91,46 °C |
| 30/09, 9 h 43 | 8 s | **92,55 °C** | 92,00 °C |
| 30/09, 14 h 25 | 5,5 s, mouture grossie | **89,60 °C** | 89,88 °C |
| 01/10, 8 h 29 | 5,5 s, mouture un peu moins fine, porte-filtre moins chauffé | **88,39 °C** | 89,25 °C |
| 04/10, 10 h 01 | 5,5 s, mouture trop fine (infusion de 34,4 s), pré-infusion à 60 % | **92,52 °C** | 91,21 °C |
| 04/10, 13 h 24 | 8 s, remboursement (0.3.37), pause de pré-infusion sans débit | **87,58 °C** | 90,17 °C |
| 05/10, 7 h 26 | 8 s, remboursement, sans coupure de fin, remplissage à 1 bar absolu (0.3.40) | **88,64 °C** | 89,62 °C |
| 06/10, 7 h 19 | 8 s, remplissage à 0,8 bar absolu, pré-infusion à 50 %, mouture trop grossière | **88,47 °C** | 89,68 °C |
| 08/10, 8 h 43 | 8 s, sonde analogique (0.3.46), mouture un peu trop fine (16,0 s depuis 8 bar) | **87,69 °C** | 89,28 °C |
| 08/10, 13 h 44 | 8 s, mouture grossie d'un pas, encore un peu fine (15,0 s depuis 8 bar) | **88,80 °C** | 89,93 °C |
| 09/10, 7 h 01 | 8 s, mouture grossie d'un pas de plus (13,7 s depuis 8 bar) | **89,26 °C** | 90,02 °C |
| 10/10, 11 h 01 | 8 s, offset −14 °C (0.4.4), débit un peu élevé (9,1 s depuis 8 bar) | **87,11 °C** | 88,76 °C |
| 10/10, 13 h 22 | 8 s, mouture affinée (13,7 s depuis 8 bar) | **88,53 °C** | 89,56 °C |
| 11/10, 9 h 47 | 8 s, même mouture, remplissage plus rapide (13,3 s depuis 8 bar) | **87,38 °C** | 88,58 °C |
| 11/10, 13 h 40 | 8 s, cible 23 g, tassage à reprendre (11,9 s depuis 8 bar) | **87,29 °C** | 88,98 °C |
| 12/10, 9 h 12 | 8 s, mouture affinée (18,7 s depuis 8 bar) | **88,75 °C** | 89,38 °C |
| 12/10, 13 h 34 | 8 s, même mouture, tassage variable (14,4 s depuis 8 bar) | **87,96 °C** | 89,04 °C |
| 13/10, 8 h 18 | 8 s, **consigne de 92 °C** (18,5 s depuis 8 bar) | **89,71 °C** | 90,76 °C |
| 13/10, 13 h 29 | 8 s, consigne de 92 °C, cible 22 g (15,6 s depuis 8 bar) | **91,96 °C** | 92,52 °C |
| 14/10, 9 h 42 | 8 s, consigne de 92 °C, mode guidé (16,1 s depuis 8 bar) | **90,89 °C** | 91,88 °C |
| 14/10, 14 h 48 | 8 s, consigne de 92 °C, mode guidé, porte-filtre 374 s sur le groupe (16,0 s depuis 8 bar) | **91,65 °C** | 92,10 °C |
| 15/10, 9 h 08 | 8 s, consigne de 92 °C, mode guidé, porte-filtre 274 s sur le groupe, 134 s de préparation (16,2 s depuis 8 bar) | **90,63 °C** | 91,48 °C |
| 15/10, 14 h 46 | 8 s, consigne de 92 °C, guide à un rinçage, porte-filtre 573 s sur le groupe, 102 s de préparation (14,8 s depuis 8 bar) | **90,76 °C** | 91,68 °C |
| 16/10, 9 h 34 | 8 s, consigne de 92 °C, écran 0.4.13, **dette effacée par une lecture NTC tardive** (14,7 s depuis 8 bar) | **93,16 °C** | 93,07 °C |
| 16/10, 14 h 10 | 8 s, consigne de 92 °C, écran 0.4.13, porte-filtre 550 s sur le groupe, 93 s de préparation (13,2 s depuis 8 bar) | **90,77 °C** | 91,73 °C |
| 17/10, 10 h 36 | 8 s, consigne de 92 °C, écran 0.4.14, porte-filtre 339 s sur le groupe, 79 s de préparation, un peu de channelling (13,4 s depuis 8 bar) | **92,42 °C** | 92,57 °C |
| 17/10, 13 h 57 | 8 s, consigne de 92 °C, écran 0.4.14, porte-filtre 140 s sur le groupe, 139 s de préparation, remplissage de 28,4 ml, acide (13,8 s depuis 8 bar) | **90,50 °C** | 91,11 °C |
| 18/10, 12 h 40 | 8 s, consigne de 92 °C, écran 0.4.14, porte-filtre à bec (même panier, tassage sur support troué), porte-filtre 243 s sur le groupe, 169 s de préparation, galette rapide, légèrement acide (8,4 s depuis 8 bar) | **90,44 °C** | 91,17 °C |
| 20/10, 14 h 29 | 8 s, consigne de 92 °C, écran 0.4.15, mouture affinée, porte-filtre sans fond 962 s sur le groupe, 92 s de préparation, remplissage de 21,4 ml, beaucoup plus acide, mouture rendue moins fine ensuite (18,8 s depuis 8 bar) | **92,57 °C** | 93,01 °C |
| 23/10, 14 h 38 | 8 s, consigne de 92 °C, carte sensors (0.4.19), nouveau café à 12,5 bar·s/g, porte-filtre 478 s sur le groupe, 72 s de préparation, remplissage à l'échéance, débitmètre soupçonné de sous-compter, assez acide (29,7 s depuis 8 bar) | **89,15 °C** | 91,28 °C |
| 23/10, 14 h 57 | 8 s, consigne de 92 °C, carte sensors (0.4.19), même café, mouture ouverte à 10,9 bar·s/g, hors guide, nettement moins acide, crema tigrée (26,4 s depuis 8 bar) | **92,42 °C** | 92,78 °C |
| 23/10, 15 h 08 | 8 s, consigne de 92 °C, carte sensors (0.4.19), même café, mouture ouverte à 6,8 bar·s/g, porte-filtre 205 s sur le groupe, 70 s de préparation, un peu « brûlé » (15,9 s depuis 8 bar) | **92,86 °C** | 93,03 °C |
| 24/10, 10 h 22 | 8 s, **consigne de 91 °C**, remplissage à 70 %, même mouture (6,7 bar·s/g), porte-filtre 220 s sur le groupe, 96 s de préparation, goût bon (13,9 s depuis 8 bar) | **92,15 °C** | 91,90 °C |
| 24/10, 13 h 23 | 8 s, consigne de 91 °C, remplissage à 70 %, même mouture (6,5 bar·s/g), porte-filtre 608 s sur le groupe, 86 s de préparation, goût bien (14,1 s depuis 8 bar) | **91,82 °C** | 92,16 °C |
| 25/10, 9 h 11 | 8 s, consigne de 91 °C, écran 0.4.23 (profil « single espresso »), remplissage à 70 %, tassage plus ferme (7,8 bar·s/g), porte-filtre 416 s sur le groupe, 109 s de préparation, amertume pas très agréable (16,8 s depuis 8 bar) | **91,17 °C** | 91,90 °C |
| 26/10, 9 h 34 | 8 s, consigne de 91 °C, écran 0.4.24, **turbo shot** (profil « turbo shot » : 11 g en panier 58 mm 15 g, cible 45 g), même mouture, tassé (0,43 bar·s/g, 2,17 bar max à 100 %), ≈ 5 g/s en tasse, 71,3 ml admis, porte-filtre 418 s sur le groupe, 131 s de préparation, zéro amertume, floral (10,9 s depuis la goutte) | **89,37 °C** | 90,50 °C |
| 26/10, 13 h 32 | 8 s, **consigne de 88 °C**, écran 0.4.24, turbo shot (cible 40 g), même mouture, même tassage (0,38 bar·s/g, 1,98 bar max à 100 %), ≈ 5 g/s en tasse, 61,7 ml admis, porte-filtre 1681 s sur le groupe, 198 s de préparation, très léger, légère acidité, sans crema (9,6 s depuis la goutte) | **88,74 °C** | 89,10 °C |
| 26/10, 15 h 35 | 8 s, consigne de 88 °C, écran 0.4.24, turbo shot à **15 g** (cible 45 g), même mouture (1,10 bar·s/g, 4,91 bar max à 100 %), 4,4 g/s en tasse, 62,6 ml admis, porte-filtre 1206 s sur le groupe, 85 s de préparation, espresso long, crema onctueuse, légère amertume (9,9 s depuis la goutte) | **89,37 °C** | 89,44 °C |
| 26/10, 16 h 32 | 8 s, consigne de 88 °C, écran 0.4.24, turbo shot à 15 g, **mouture grosse** (0,58 → 0,49 bar·s/g, channelling, 2,53 bar max à 100 %), 4,7 g/s en tasse, 58,6 ml admis, porte-filtre 1003 s sur le groupe, 91 s de préparation, plat, sans fruité (10,6 s depuis la goutte) | **88,77 °C** | 89,28 °C |
| 27/10, 7 h 44 | 8 s, consigne de 88 °C, écran 0.4.24, turbo shot à 15 g, mouture grosse, **pompe à 70 %**, **puck screen** (0,51 bar·s/g constants, 1,92 bar max), 3,8 g/s en tasse, 61,4 ml admis, porte-filtre 921 s sur le groupe, 106 s de préparation, acidité légère, jugé trop chaud (12,5 s depuis la goutte) | **88,20 °C** | 88,80 °C |
| 27/10, 13 h 16 | **2 s**, **consigne de 86 °C**, écran 0.4.24, turbo shot à 15 g, mouture grosse, puck screen, **pompe et remplissage à 100 %** (0,49 → 0,55 bar·s/g, 2,54 bar max), 4,6 g/s en tasse, 56,8 ml admis, guide ouvert machine froide, porte-filtre 331 s sur le groupe, 109 s de préparation, un peu amer, plat (9,6 s depuis la goutte) | **82,52 °C** | 84,10 °C |
| 27/10, 13 h 49 | 2 s, consigne de 86 °C, écran 0.4.24, turbo shot à 15 g, **mouture espresso**, puck screen, pompe et remplissage à 100 % (1,26 → 1,33 bar·s/g, 5,61 bar max, pompe bridée à 86–92 % par le PI), 4,0 g/s en tasse, 64,6 ml admis, porte-filtre 458 s sur le groupe, 87 s de préparation, moins amer, moins bon qu'à 89 °C (10,6 s depuis la goutte) | **83,16 °C** | 84,38 °C |
| 28/10, 10 h 09 | 8 s, consigne de 91 °C, écran 0.4.24, retour au profil « single espresso », **porte-filtre à bec**, mouture espresso (7,4 bar·s/g), remplissage court (17,1 ml, circuit à 0,51 bar au départ), première goutte à 8,98 bar, porte-filtre 1222 s sur le groupe, 259 s de préparation, goût vraiment bien (18,3 s depuis 8 bar) | **92,04 °C** | 92,38 °C |
| 28/10, 13 h 47 | 8 s, consigne de 91 °C, écran 0.4.24, single espresso, porte-filtre à bec, **fin du sachet**, galette rapide (5,5 bar·s/g), **remplissage long (26,7 ml, 2,69 ml/s, circuit à 0,25 bar)**, première goutte à 2,65 bar, porte-filtre 435 s sur le groupe, 133 s de préparation, belle crema, arôme agréable (11,3 s depuis 8 bar) | **90,12 °C** | 91,17 °C |
| 29/10, 9 h 19 | 8 s, consigne de 91 °C, écran 0.4.24, single espresso, porte-filtre à bec, **dernière dose du sachet**, galette à 6,0 bar·s/g, **pré-infusion sautée** (balance de −0,1 à 0,0 g pendant le remplissage), remplissage de 22,9 ml à 3,14 ml/s (circuit à 0,29 bar), première goutte à 6,25 bar, **porte-filtre 1081 s sur le groupe**, 96 s de préparation, excellent, très bons arômes (14,5 s depuis 8 bar) | **90,58 °C** | 91,56 °C |
| 29/10, 14 h 07 | 8 s, consigne de 91 °C, écran 0.4.24, single espresso, **ViCafé**, **porte-filtre sans fond**, mouture trop fine (8,7 bar·s/g), remplissage de 22,6 ml à 3,13 ml/s (circuit à 0,26 bar), NTC +0,38 °C au remplissage et −1,78 °C en pré-infusion, porte-filtre 561 s sur le groupe, 90 s de préparation (20,1 s depuis 8 bar) | **90,50 °C** | 90,98 °C |
| 29/10, 14 h 21 | 8 s, **consigne de 90 °C**, écran 0.4.24, ViCafé, sans fond, **mouture ouverte** (6,3 bar·s/g), remplissage de 21,2 ml à 3,14 ml/s (circuit à 0,29 bar), NTC +2,02 °C au remplissage, porte-filtre 528 s sur le groupe, 79 s de préparation, très bien (14,4 s depuis 8 bar) | **91,30 °C**, +1,30 °C sur la consigne | 91,38 °C |

Depuis le 5 octobre, chaque infusion est analysée dans
[journal-infusions.md](journal-infusions.md).

Les réglages et les recettes ont changé d'une infusion à l'autre ; la
dispersion de 5,35 °C (hors 13 h 24 le 4 octobre) ne mesure donc pas
seulement la loi. En simulation, à
précharge égale, l'hydraulique seule des quatre captures donne 2,1 °C
d'écart (voir [Simulation](#simulation)).

## Constats qui fondent la loi

Machine : Profitec GO, chaudière de 400 ml, résistance de 1 200 W. 1 % de
commande pendant 1 s vaut 12 J.

1. **Le besoin suit le débit.** Chauffer 1 ml/s d'eau du réservoir
   (≈ 24,5 °C, non mesurée) jusqu'à ≈ 95 °C réels demande
   4,18 × 70,5 ≈ 295 W, soit ≈ 24,6 % par ml/s. Le remplissage et la
   pré-infusion admettent ≈ 3,6 ml/s (≈ 92 %). L'infusion en régime admet
   1,4 à 2 ml/s selon la mouture et le tassage (38 à 53 %).
2. **L'eau froide atteint la sonde avant la chaleur.** Sur les infusions du
   28 septembre (13 h 01) et du 29 septembre (7 h 40), la NTC baisse 4 à 5 s
   après le début du débit, mais ne réagit à une chauffe qu'après environ
   8 s. Le modèle ajusté sur ces captures donne des délais moyens de
   **6,4 s** pour l'eau et de **17,6 s** pour la chauffe (voir
   [Simulation](#simulation)).
3. **Le creux ne vient pas d'un manque d'énergie totale.** À 7 h 40, le bilan
   cumulé est resté positif pendant toute l'infusion, et la NTC a quand même
   perdu 4,6 °C entre son pic et son minimum. La chaleur fournie *pendant*
   le fort débit arrive trop tard sur la sonde.
4. **La température finale suit « précharge − déficit pendant
   l'écoulement ».** +0,6 kJ laissent +0,95 °C (7 h 40) ; environ +4 kJ
   laissent +3 °C (13 h 01).
5. **Toute chauffe dans les ~11 dernières secondes n'agit que sur l'état
   final.** La chaleur envoyée alors atteint la sonde après l'arrêt de la
   pompe. En simulation, couper 8 ou 11 s avant l'arrêt ne change pas la
   moyenne en tasse et retire 1,7 à 3,7 °C au rebond. Ce constat a fondé la
   coupure de fin de 0.3.28 à 0.3.38 (voir
   [Coupure de fin](#coupure-de-fin-supprimée-en-0339)).

Conséquences :

- la commande pendant l'écoulement suit le débit mesuré, sans dépendre de la
  NTC ;
- la précharge sert à **devancer** l'eau froide, pas à équilibrer un bilan ;
  depuis 0.3.37, elle est rendue pendant l'infusion ;
- depuis 0.3.39, la chauffe suit le débit jusqu'à l'arrêt de la pompe.

Jusqu'à 0.3.27, un plancher de 45 % disparaissait dès que la NTC dépassait
consigne + 1 °C. À 13 h 01, le pic dû à une précharge de 10 s l'a retiré :
la commande est restée à 0–8 % pendant tout le remplissage, en plein fort
débit. La loi actuelle évite précisément cette dépendance.

## Loi

### Précharge

Pompe arrêtée, **90 %** pendant `heating.brew_preheat_time_s` (réglage
utilisateur, 0 à 15 s, 0 désactive). La précharge est supprimée si la NTC
dépasse déjà la consigne de 0,5 °C ou plus ; cette condition est réévaluée à
chaque pas.

Depuis 0.3.30, la première commande non nulle de la précharge, puis celle
de l'écoulement, demandent au module capteurs une fenêtre SSR neuve : la
chauffe part aussitôt, au lieu d'attendre jusqu'à 1 s la fin de la fenêtre
de repos en cours.

### Écoulement

Pendant le remplissage, la pré-infusion et l'infusion :

`P = min(90 %, 3,5 % + débit × 4,18 × 70,5 / 12)`

| Débit amont | Commande |
| ---: | ---: |
| 1,2 ml/s | 33,0 % |
| 1,4 ml/s | 37,9 % |
| 2,0 ml/s | 52,6 % |
| 3,0 ml/s | 77,2 % |
| ≥ 3,52 ml/s | 90 % |

La commande suit le débit **sans filtre, dans les deux sens**. Le débit est
celui du débitmètre (en amont de la pompe), accepté seulement si la mesure
est fraîche, si la dernière impulsion date de 500 ms au plus et si la pompe
est confirmée en marche. Il se lit sur les 10 dernières impulsions de
l'écoulement en cours, ou dès 3 impulsions à son début (depuis 0.4.13) ;
une impulsion après plus d'une seconde de silence ouvre un nouvel
écoulement.

Seuls modifient cette commande :

| Condition | Commande |
| --- | --- |
| NTC au-delà de consigne + **4 °C** (sécurité) | 0 % |
| Remplissage, moins de 3 impulsions depuis le début de l'écoulement (depuis 0.4.13) | **90 %** : l'eau entre, le débit n'est pas encore lu |
| Pré-infusion, pompe confirmée en marche, aucune impulsion depuis 500 ms (depuis 0.3.32) | **3,5 %** : débit nul, pas inconnu |
| Débit non mesurable (conditions ci-dessus), hors de ce cas | **45 %** (repli) |
| Mesure invalide, chauffe désactivée, NTC > 100 °C | 0 % (coupures générales) |
| Lecture NTC manquante pendant la précharge ou l'écoulement, 1 s au plus (depuis 0.4.14) | 0 % pour ces pas, loi d'infusion et dette conservées |

Depuis 0.3.32, la pré-infusion est une pause : pompe à 35 %, sous son seuil
de débit, vanne ouverte. Sans la ligne « pré-infusion » du tableau, la
commande serait restée au repli de 45 % pendant toute la pause. La pompe
n'est pas mise à 0 % : la vanne 3 voies se fermerait et viderait le
headspace, et la capture HF compterait sa queue depuis cet arrêt.

Le seuil de sécurité est volontairement large : le pic de NTC dû à la
précharge (≈ +1,5 à +2,5 °C) ne doit pas retirer l'appoint.

### Remboursement de la précharge

Depuis 0.3.37, la précharge est une **avance sur l'appoint**. Son énergie
forme une dette, rendue dès le début de l'infusion en retenant l'appoint :
la commande est de **0 %** jusqu'à ce que la somme des appoints retenus
égale la dette. Ensuite, la loi d'écoulement reprend sans changement.

| Règle | Choix |
| --- | --- |
| Dette | commande de précharge effectivement envoyée, chaque commande comptée jusqu'au pas suivant (1 s au plus) ; une précharge supprimée par la NTC ne compte pas |
| Début du remboursement | entrée en infusion (état `kBrew` de la machine, mode `kInfusion` du régulateur), quelle que soit la sortie de la pré-infusion (temps ou première goutte) |
| Remplissage et pré-infusion | appoint normal, dette intacte |
| Montant rendu à chaque pas | la commande que la loi aurait envoyée : appoint au débit, ou repli de 45 % sans débit mesurable |
| Sécurité à consigne + 4 °C | déjà à 0 % : ne rend rien, la dette reste |
| Dernier pas | peut rendre jusqu'à 250 ms de trop (≈ 0,27 kJ à 90 %) |
| Pompe arrêtée avant la fin du remboursement | le reste de la dette est abandonné |
| Précharge interrompue, cycle suivant sans précharge | dette remise à zéro au début de tout écoulement qui ne suit pas une précharge |
| Lecture NTC manquante 1 s au plus, pendant la précharge ou l'écoulement (depuis 0.4.14) | chauffe coupée pour ces pas, qui n'ajoutent rien à la dette ; la dette reste. Avant 0.4.14, une seule trame arrivée après le seuil de fraîcheur de 200 ms effaçait la dette (16 octobre, 9 h 34) |
| Coupure générale pendant l'écoulement (lecture manquante plus de 1 s, chauffe désactivée, NTC > 100 °C) | régulateur remis à zéro : la dette est perdue, l'appoint reprend normalement |

Le remboursement ne dépend ni de la balance ni de la NTC : seule la fin
de la pré-infusion le déclenche, et le montant suit le débitmètre comme
l'appoint lui-même. Il fonctionne donc à l'identique sans balance, à
l'arrêt au temps comme au poids.

Attendre que le débit amont rejoigne le débit en tasse, c'est-à-dire que
la galette soit saturée, a été écarté. Cette condition revient à rembourser
une fois le fort débit passé. En simulation, la variante « remboursée
sous 2,5 ml/s » laisse 1,88 °C d'écart entre les quatre captures du
28 au 30 septembre, contre 1,23 °C en remboursant dès l'infusion. Sur
l'infusion de 34 s du 4 octobre, elle laisse 91,8 °C en tasse contre 89,6 °C
(précharge de 8 s). La chaleur qui crée la double chauffe est celle de la
montée en pression ; il faut la retenir à ce moment-là. Un profil de débit
variable ne change rien à la règle : un débit plus fort rembourse plus
vite, un débit plus faible plus lentement.

### Coupure de fin (supprimée en 0.3.39)

De 0.3.28 à 0.3.38, la chauffe était coupée quand l'arrêt de la pompe était
estimé à 11 s ou moins (au temps cible, ou au poids sur le débit en tasse
des 2 dernières secondes). Elle retirait l'appoint de la fin de
l'infusion, ≈ 3 à 4,6 kJ, qui n'atteint la sonde qu'après l'arrêt de la
pompe.

Tant que la précharge restait acquise, elle finançait cette coupure : à
9 h 43 le 30 septembre, 8,5 kJ de précharge pour 4,6 kJ coupés. Depuis le
remboursement (0.3.37), la précharge est rendue et la coupure n'est plus
financée : elle laisse un déficit net. À 13 h 24 le 4 octobre (infusion de
24,1 s), la commande est restée à 0 % pendant 22 s :

| Intervalle | Commande |
| --- | --- |
| 16,6 → 28,4 s | 0 % : remboursement de 703 %·s, dont 6 s à 3,5–3,9 ml/s pendant la montée en pression |
| 28,4 → 30,6 s | ≈ 37 % : appoint au débit |
| 30,6 → 40,8 s | 0 % : coupure de fin, 10,2 s avant l'arrêt |

Bilan du cycle : ≈ 14,5 kJ fournis par le SSR pour ≈ 18,9 kJ demandés par
64 ml d'eau. La NTC finit à 84,1 °C à l'arrêt de la pompe et descend à
82,79 °C 10 s après.

Rejouée sur huit captures avec 8 s de précharge et remboursement, la
suppression de la coupure ne change pas la moyenne en tasse (±0,02 °C). Elle
relève le minimum de 0,2 à 1,3 °C et l'état 30 s après l'arrêt de 2 à 3,5 °C
(87,59 → 91,08 °C sur l'hydraulique de 13 h 24). Elle a été supprimée pour
simplifier la loi ; l'historique Git garde le code (`BrewEndEstimator`,
`kEndCutLeadS`).

Variantes écartées, même simulation :

| Variante | Effet |
| --- | --- |
| Pas de coupure tant que la dette n'est pas soldée | identique à 0.3.38 sur les huit captures : la dette est toujours soldée avant le début de la coupure |
| Dette limitée à l'appoint envoyé pendant le remplissage et la pré-infusion | ne change que les captures où cet appoint est inférieur à la précharge (9 h 43, 10 h 01 et 13 h 24) : +0,02 à +0,66 °C en tasse |

## Réglages

| Réglage | Emplacement | Valeur |
| --- | --- | --- |
| Durée de précharge | NVS `heating.brew_preheat_time_s`, 4e page des réglages | 0 à 15 s ; 5,5 s du 30 septembre au 4 octobre ; **8 s** avec le remboursement (0.3.37) |
| Puissance de précharge | `BrewHeating::kPreheatPowerPct` | 90 % |
| Bande de suppression de la précharge | `BrewHeating::kPreheatAboveTargetBandC` | +0,5 °C |
| Maintien | `BrewHeating::kHoldPowerPct` | 3,5 % |
| Pente débit → commande | `BrewHeating::kWaterHeatPctPerMlS` | 4,18 × 70,5 / 12 ≈ 24,56 % par ml/s |
| Plafond pendant l'écoulement | `BrewHeating::kPowerLimitPct` | 90 % |
| Repli sans débit | `BrewHeating::kFlowFallbackPct` | 45 % |
| Durée maximale imputée à une commande (dette) | `BrewHeating::kMaximumStepMs` | 1 s |
| Lecture NTC manquante tolérée pendant l'infusion | `Controller::BREW_READING_GAP_MAX_MS` | 1 s |
| Sécurité au-dessus de la consigne | `BrewHeating::kSafetyAboveTargetC` | +4 °C |

Les 70,5 K supposent une eau du réservoir à 24,5 °C et une chaudière à
≈ 95 °C réels. Une consigne très différente de 90 °C affichés, ou une eau plus
froide, changerait cette pente d'environ 1,4 % par kelvin.

Les tests hôte sont dans `firmware/screen/test/test_thermal.cpp`
(`sh test/run_tests.sh` depuis `firmware/screen`).

## Simulation

`firmware/tools/simulate_boiler.py` ajuste un modèle linéaire de la NTC et
rejoue des lois candidates sur l'hydraulique réelle de captures d'infusion :

```sh
uv run firmware/tools/simulate_boiler.py          # paramètres enregistrés
uv run firmware/tools/simulate_boiler.py --fit    # réajuster
uv run firmware/tools/simulate_boiler.py --capture captures/<capture>.json
```

Le modèle superpose deux chemins, chacun avec un retard pur suivi de trois
premiers ordres, plus une part locale qui se mélange en 4,7 s :

| Chemin | Retard pur | Constante × 3 | Délai moyen |
| --- | ---: | ---: | ---: |
| Chauffe → NTC | 3,95 s | 4,56 s | ≈ 17,6 s |
| Eau admise → NTC | 2,0 s | 1,47 s | ≈ 6,4 s |

Capacité effective : **1,47 kJ/K** (0,0082 °C par %·s). Ajustement sur les
infusions du 28 septembre à 13 h 01 et du 29 septembre à 7 h 40, et sur une
montée sans écoulement depuis 80 °C (28 septembre, 9 h 56). Écart RMS :
0,46 à 0,48 °C. En validation croisée, le minimum est prévu à **±1 °C**. Le
modèle sous-estime le pic avant infusion d'environ 1 °C. L'infusion du
28 septembre à 8 h 37 est exclue : elle précède le passage de la fenêtre du
SSR à 1 s, l'état du SSR n'y est pas enregistré et son bilan ne ferme pas.

Résultats (NTC prévue ; hydraulique de 13 h 01 / 7 h 40 ; précharge et
plafond à 90 %) :

| Variante | Minimum | 30 s après l'arrêt | Énergie |
| --- | ---: | ---: | ---: |
| Loi 0.3.27 (commande réelle) | 88,4 / 86,4 °C | 93,5 / 91,0 °C | 26,1 / 22,2 kJ |
| Appoint au débit, sans précharge | 82,4 / 82,5 °C | 89,5 / 89,6 °C | 20,0 kJ |
| Appoint au débit, précharge 3 s | 84,3 / 84,5 °C | 91,7 / 91,8 °C | 23,3 kJ |
| Appoint au débit, précharge 6 s | 86,4 / 86,7 °C | 93,9 / 94,1 °C | 26,5 kJ |
| Précharge 8 s, coupure 11 s avant l'arrêt | 87,9 / 88,1 °C | 92,0 / 90,0 °C | 23,7 / 20,9 kJ |
| **Précharge 10 s, coupure 11 s avant l'arrêt** | **89,3 / 89,6 °C** | **93,4 / 91,5 °C** | 25,9 / 23,1 kJ |
| Oracle : besoin commandé 11 s à l'avance | 88,6 / 88,7 °C | 89,4 / 89,5 °C | 20,0 kJ |

Chaque seconde de précharge relève le minimum d'environ 0,7 °C et l'état
final d'autant. Le plafond pèse peu sur le minimum (84,3 °C à 90 %, 84,4 °C
à 100 %, 83,7 °C à 70 %, avec 3 s de précharge). L'oracle, irréalisable tel
quel, borne ce qu'on peut attendre ; la précharge suivie d'une coupure de fin
en est l'approximation réalisable.

La récupération est simulée à 0 %, comme dans les captures de référence. Le
modèle doit être recalé à chaque nouvelle capture qui enregistre l'état du
SSR.

**Premier essai de la loi, 29 septembre à 14 h 24 (précharge de 10 s).**
Pic de 92,53 °C, minimum de 90,49 °C pendant l'écoulement, 90,86 °C 30 s
après l'arrêt, moyenne NTC pondérée par le volume de **91,46 °C** pour une
consigne de 90 °C : plus de creux, mais un dépassement. Détail dans le
[journal](chauffe-chaudiere.md#infusion-de-14-h-24-29-septembre--précharge-de-10-s).
Les « 10 s » n'ont fourni qu'environ 8,7 s réelles : le SSR a attendu 1 s
la fenêtre suivante. Chaque seconde de précharge vaut environ 0,55 °C sur
la moyenne pondérée. Depuis 0.3.30, la précharge part avec une fenêtre SSR
neuve ; l'essai suivant règle **8 s**.

**Deuxième essai, 30 septembre à 9 h 43 (précharge de 8 s, 0.3.30).** La
fenêtre SSR neuve fonctionne : le SSR fournit 8,52 kJ pour 8,53 kJ
commandés pendant la précharge. Le remplissage est couvert (minimum de
90,17 °C pendant l'écoulement), mais la moyenne en tasse monte à
**92,55 °C**, et la NTC à **94,8 °C** à l'arrêt de la pompe, chauffe coupée
depuis 10,5 s. Détail dans le
[journal](chauffe-chaudiere.md#infusion-de-9-h-43-30-septembre--précharge-de-8-s).

Cause : **l'eau du remplissage est chauffée deux fois.** La précharge couvre
l'eau froide du remplissage à l'avance. L'appoint au débit la chauffe une
seconde fois en temps réel : ≈ 12 kJ à 75–90 % entre 9,4 et 20,5 s. Avec le
délai de 17,6 s, cette chaleur atteint la sonde entre ~27 et 38 s, quand le
débit n'est plus que de 1,3–1,9 ml/s. La coupure de fin ne retire que
l'appoint des 10,5 dernières secondes (≈ 4,6 kJ), alors que la précharge
apporte 8,5 kJ. L'excédent de ≈ 3,9 kJ vaut ≈ +2,7 °C, à peu près l'état
final mesuré (+2,7 °C 30 s après l'arrêt). Plus l'infusion est longue et
lente, plus cette chaleur tombe en tasse plutôt qu'après l'arrêt de la
pompe.

**Troisième essai, 30 septembre à 14 h 25 (précharge de 5,5 s, mouture
grossie).** Moyenne en tasse de **89,60 °C**, minimum de 88,84 °C pendant
l'écoulement, 89,03 °C 30 s après l'arrêt ; café jugé bon, creux jugé
acceptable. L'infusion plus courte et plus rapide (pompe 21,7 s au lieu de
29,9 s) envoie la chaleur du fort débit en grande partie après l'arrêt de la
pompe : la bosse de fin tombe à +0,8 °C. Détail dans le
[journal](chauffe-chaudiere.md#infusion-de-14-h-25-30-septembre--précharge-de-55-s).
La précharge de 5,5 s est retenue.

**Quatrième essai, 1er octobre à 8 h 29 (précharge de 5,5 s, mouture un
peu moins fine, porte-filtre moins chauffé).** Moyenne en tasse de
**88,39 °C**, minimum de 87,03 °C pendant l'écoulement, 87,76 °C 30 s après
l'arrêt. La rondelle plus rapide amène la pression plus tôt ; la coupure de
fin tombe pendant la montée en pression, avec encore 3,5 ml/s. Rejoué avec
la commande réelle, le modèle prévoit la même moyenne en tasse qu'à
14 h 25 (87,71 contre 87,81 °C) : l'écart mesuré de 1,21 °C relève du
facteur manquant décrit plus bas. Mouture et porte-filtre ayant changé,
l'essai ne tranche pas la répétabilité ; rien n'est changé. Détail dans le
[journal](chauffe-chaudiere.md#infusion-de-8-h-29-1er-octobre--précharge-de-55-s).

**Cinquième essai, 4 octobre à 10 h 01 (précharge de 5,5 s, mouture trop
fine, écran 0.3.32 ou plus).** Nouveau paquet du même café. Moyenne en
tasse de **92,52 °C**, minimum de 88,71 °C pendant l'écoulement, 91,97 °C
20 s après l'arrêt. Le temps jusqu'à 8 bar ne change pas (11,6 s) ; le
débit en pression tombe à 0,8–1,3 ml/s et l'infusion dure 34,4 s. La
pré-infusion, restée à 60 % après la migration du schéma 8, n'a pas fait
de pause et a appelé ≈ 4 kJ en plein débit. La chaleur du fort débit
atteint la sonde vers 25–37 s, pompe en marche : c'est la double chauffe
de 9 h 43, à précharge plus courte. Détail dans le
[journal](chauffe-chaudiere.md#infusion-de-10-h-01-4-octobre--précharge-de-55-s-mouture-trop-fine).

**Remboursement de la précharge (implémenté en 0.3.37, voir
[Loi](#remboursement-de-la-précharge)).** L'appoint est
retenu jusqu'à ce que l'énergie retenue égale celle de la précharge ; la
coupure de fin, encore présente en 0.3.37, s'y ajoutait sans compter dans
ce remboursement. Moyenne en
tasse simulée sur l'hydraulique des quatre captures (13 h 01, 7 h 40,
14 h 24, 9 h 43), paramètres enregistrés :

| Loi | Précharge | Tasse | Écart entre captures | Minimum | 30 s après l'arrêt |
| --- | ---: | ---: | ---: | ---: | ---: |
| actuelle, coupure 11 s | 8 s | 88,5–90,6 °C | 2,10 °C | 87,9–89,1 °C | 90,0–92,7 °C |
| remboursée dès le début de l'infusion | 10 s | 89,6–90,8 °C | **1,23 °C** | 88,2–89,2 °C | 88,5–89,6 °C |
| remboursée quand le débit passe sous 2,5 ml/s | 8 s | 88,5–90,4 °C | 1,88 °C | 87,9–89,1 °C | 90,0–92,0 °C |

À moyenne comparable, le remboursement dès l'infusion garde les mêmes
minima. Il réduit l'écart entre captures de 2,1 à 1,2 °C et ramène l'état
final sous la consigne. Rejouées sur l'hydraulique de 14 h 25, 8 h 29 et
10 h 01 (infusions de 14,4, 13,7 et 34,4 s), les deux lois donnent :

| Loi | Précharge | Tasse 14 h 25 / 8 h 29 / 10 h 01 | Écart | Minimum | Énergie |
| --- | ---: | ---: | ---: | ---: | ---: |
| actuelle, coupure 11 s | 5,5 s | 87,82 / 87,62 / 91,14 °C | 3,52 °C | 86,4–87,3 °C | 13,7–23,0 kJ |
| remboursée dès le début de l'infusion | 8 s | 89,24 / 89,00 / 89,55 °C | 0,55 °C | 86,7–88,0 °C | 13,6–17,0 kJ |
| remboursée dès le début de l'infusion | 10 s | 90,67 / 90,37 / 90,88 °C | **0,51 °C** | 87,9–89,5 °C | 15,8–18,2 kJ |

Le modèle sous-estime les trois moyennes de 0,7 à 1,8 °C ; seul l'écart
entre captures se lit. Attendre la fin du fort débit (sous 2,5 ou
2,0 ml/s) laisse trop peu de temps pour rembourser avant l'arrêt. Chaque
seconde de précharge vaut ≈ 0,72 °C sur la moyenne en tasse, quelle que soit
la loi.

**Limite de ce résultat : l'erreur du modèle sur la moyenne en tasse est du
même ordre que ces écarts.** Rejoué avec la commande réelle, il prévoit
90,63 °C à 9 h 43 (mesuré 92,51 °C). Réajusté sur les quatre infusions avec
l'état du SSR, il laisse +1,3 °C à 13 h 01 et −1,3 °C à 9 h 43 ; en
validation croisée, l'erreur atteint ±1,9 °C. Un facteur manque au modèle
linéaire ; il n'est pas identifié. Le réajustement n'a pas été retenu. Le
réglage de précharge part donc de la mesure, et le remboursement reste à
confirmer sur des infusions réelles.

## Limites connues

- **La NTC n'est pas l'eau au groupe.** Tout ce qui précède optimise la
  courbe de la sonde. L'offset est recalé au panier (voir
  [Objectif](#objectif)) et confirmé à environ 1 K près le 12 octobre. Sur
  la forme, le creux de la NTC n'est pas celui de l'eau au panier : au
  débit de la SCACE (2,8 ml/s), 8 à 12 K de creux à la NTC ne donnent que
  0,5 à 1 K de baisse au-dessus du puck, avant la galette (voir
  [sonde-scace.md](sonde-scace.md#infusions-12-octobre-2026)). La cause
  n'est pas départagée : masse du groupe, mélange au-dessus du puck ou
  NTC qui exagère le creux. L'effet d'une vraie galette n'est pas mesuré.
- **Le critère dépend de la durée de l'infusion.** La NTC moyenne pondère
  le creux par le temps passé dedans : le 12 octobre, elle variait de
  2,6 K entre trois infusions de 30 à 35 s, quand l'eau au panier ne
  variait que de 0,5 K. Une infusion longue est donc jugée plus froide
  que l'eau réelle.
- **Remboursement non vérifié sur infusion réelle.** La double chauffe de
  l'eau du remplissage (précharge puis appoint au débit) est traitée depuis
  0.3.37 par le [remboursement](#remboursement-de-la-précharge). Deux
  essais réels : 13 h 24 le 4 octobre (coupure de fin encore active),
  87,58 °C en tasse pour 89,57 °C prévus avec la commande réelle ; 7 h 26
  le 5 octobre (sans coupure), 88,64 °C pour 89,63 °C prévus. L'écart de
  −1 à −2 °C n'est expliqué par aucune variante de la loi ; l'énergie SSR
  inférieure à la commande en couvre ≈ 0,6 °C le 5 octobre (voir
  [journal-infusions.md](journal-infusions.md)).
- **Le débitmètre est en amont de la pompe.** Une recirculation par l'OPV
  gonflerait le débit mesuré, donc la commande.
- **Premier instant du remplissage.** Le débit mesuré reste parfois proche
  de 0 pendant la première seconde environ (≈ 0,07 ml/s alors que 2 ml sont
  déjà passés) : la commande y tombe vers 5 %, après la précharge. D'autres
  fois, les premières impulsions sont lues à 10–71 ml/s (14 h 25 le
  30 septembre) et la commande passe au plafond. Correction du 15 octobre :
  pendant les 10 premières impulsions, le débit est calculé depuis une
  impulsion de l'écoulement précédent, dont l'horodatage sur 16 bits
  reboucle toutes les 65,5 s ; la valeur lue dépend donc du temps écoulé
  depuis cet écoulement, modulo 65,5 s. Le SSR, éteint, attend ensuite la
  fenêtre suivante : 1,7 à 2,0 s de coupure sur la plupart des infusions
  depuis le 10 octobre, ≈ 1,9 kJ et ≈ 1 °C en tasse en simulation.
  Corrigé en 0.4.13 : le débit ne s'ancre plus que sur l'écoulement en
  cours, et la chauffe reste à 90 % jusqu'à ce qu'il soit lu. Les
  simulations des captures antérieures incluent ces écarts, puisqu'elles
  utilisent le débit mesuré.
- **Température de l'eau admise** : constante, non mesurée.
- **Énergie fournie inférieure à la commande.** Le module capteurs ne
  rallume pas le SSR dans une fenêtre de 1 s où il s'est déjà éteint. À
  14 h 24, il a fourni 21,5 kJ pour 23,8 kJ demandés, dont 1 s perdue au
  départ de la précharge ; cette perte varie de 0 à 1 s selon la phase de la
  fenêtre. Depuis 0.3.30, l'écran demande une fenêtre neuve au premier
  paquet de la précharge et de l'écoulement (flag `restart_window`) ; les
  hausses en cours d'écoulement attendent encore la fenêtre suivante. Le
  simulateur utilise la commande et surestimait l'état final de 14 h 24
  (93,2 contre 90,86 °C).
- **Peu d'essais réels** : deux cafés par jour. Un panier de simulation, dont
  le headspace doit reproduire le temps de remplissage réel, permettra des
  essais plus fréquents.

## Captures de référence

Les fichiers `captures/` sont locaux et ignorés par Git.

| Capture | Précharge | Rôle |
| --- | ---: | --- |
| `260928-130104.json` | 10 s | ajustement ; trou de chauffe dû au plancher conditionnel |
| `260929-074002.json` | 6 s | ajustement ; bilan équilibré, creux inchangé |
| `260929-142427.json` | 10 s | premier essai de la loi 0.3.28 ; pas de creux, dépassement de +1,46 °C en moyenne ; hors ajustement |
| `260930-094308.json` | 8 s | premier essai de la 0.3.30 (fenêtre SSR neuve) ; mouture trop fine ; +2,55 °C en tasse, double chauffe du remplissage ; hors ajustement |
| `260930-142555.json` | 5,5 s | mouture grossie ; −0,40 °C en tasse, réglage retenu ; hors ajustement |
| `261001-082959.json` | 5,5 s | mouture un peu moins fine, porte-filtre moins chauffé ; −1,61 °C en tasse, coupure pendant la montée en pression ; hors ajustement |
| `261004-100128.json` | 5,5 s | mouture trop fine, infusion de 34,4 s, pré-infusion à 60 % sans pause ; +2,52 °C en tasse, double chauffe du remplissage ; hors ajustement |
| `261004-132439.json` | 8 s | premier essai du remboursement (0.3.37) ; pause de pré-infusion sans débit ; −2,42 °C en tasse, chauffe à 0 % pendant 22 des 24 s d'infusion ; hors ajustement |
| `261005-072633.json` | 8 s | premier essai sans coupure de fin (0.3.40) ; remplissage de 24,1 ml à 1 bar absolu ; −1,36 °C en tasse ; hors ajustement |
| `261006-071953.json` | 8 s | remplissage à 0,8 bar absolu, pré-infusion à 50 % ; mouture trop grossière (11,8 s depuis 8 bar) ; −1,53 °C en tasse ; hors ajustement |
| `261008-084342.json` | 8 s | première infusion sur la sonde analogique ; mouture un peu trop fine (16,0 s depuis 8 bar) ; 0 % pendant 15,4 s ; −2,31 °C en tasse, modèle à +2,14 °C ; hors ajustement |
| `261008-134433.json` | 8 s | mouture grossie d'un pas (7,5 bar·s/g au plateau) ; 15,0 s depuis 8 bar ; 0 % pendant 14,0 s ; −1,20 °C en tasse, modèle à +1,33 °C ; hors ajustement |
| `261009-070144.json` | 8 s | mouture grossie d'un pas de plus (7,1 bar·s/g au plateau) ; 13,7 s depuis 8 bar ; 0 % pendant 11,6 s ; −0,74 °C en tasse, modèle à +0,05 °C ; hors ajustement |
| `261010-110108.json` | 8 s | première infusion avec l'offset de −14 °C (0.4.4) ; 5,9 bar·s/g au plateau, 9,1 s depuis 8 bar ; 0 % pendant 11,4 s, appoint repris 2,9 s avant l'arrêt ; −2,89 °C en tasse, modèle à +2,26 °C ; hors ajustement |
| `261010-132259.json` | 8 s | mouture affinée (6,8 bar·s/g au plateau) ; 13,7 s depuis 8 bar ; 0 % pendant 11,9 s, appoint repris 5,7 s avant l'arrêt ; −1,47 °C en tasse, modèle à +0,72 °C ; hors ajustement |
| `261011-094749.json` | 8 s | même mouture (7,0 bar·s/g au plateau), remplissage à 3,6 ml/s ; 13,3 s depuis 8 bar ; 0 % pendant 11,1 s, appoint repris 5,7 s avant l'arrêt ; −2,62 °C en tasse, modèle à +1,82 °C (+0,98 °C avec l'énergie SSR) ; hors ajustement |
| `261011-134059.json` | 8 s | cible 23 g, même mouture, tassage à reprendre (6,2 bar·s/g au plateau) ; 11,9 s depuis 8 bar ; 0 % pendant 10,6 s, appoint repris 5,9 s avant l'arrêt ; −2,71 °C en tasse, modèle à +1,52 °C (+1,10 °C avec l'énergie SSR) ; hors ajustement |
| `261012-091234.json` | 8 s | mouture affinée (8,5 bar·s/g au plateau), pré-infusion finie à l'échéance ; 18,7 s depuis 8 bar ; 0 % pendant 12,5 s, rebond à 89,58 °C puis −0,24 °C/s pendant l'appoint, repris 9,5 s avant l'arrêt ; −1,25 °C en tasse, modèle à +0,44 °C (+0,06 °C avec l'énergie SSR) ; hors ajustement |
| `261012-133429.json` | 8 s | même mouture, tassage variable (7,0 bar·s/g au plateau), pré-infusion finie sur la goutte ; 14,4 s depuis 8 bar ; 0 % pendant 10,9 s, palier à 89,4 °C sans rebond, appoint repris 7,6 s avant l'arrêt ; −2,04 °C en tasse, modèle à +0,75 °C (+0,23 °C avec l'énergie SSR) ; hors ajustement |
| `261013-081846.json` | 8 s | consigne de 92 °C, même mouture (8,0 bar·s/g au plateau), pré-infusion finie à l'échéance ; 18,5 s depuis 8 bar ; 0 % pendant 12,0 s, palier à 90,4 °C sans rebond, appoint repris 9,6 s avant l'arrêt ; −2,29 °C en tasse, modèle à +1,25 °C (+0,43 °C avec l'énergie SSR, +0,16 °C avec un écart d'eau froide de 72,5 K) ; hors ajustement |
| `261013-132935.json` | 8 s | consigne de 92 °C, cible 22 g (7,2 bar·s/g au plateau), pré-infusion finie sur la goutte ; 15,6 s depuis 8 bar ; NTC +1,67 °C pendant le remplissage, pic à 94,01 °C ; 0 % pendant 12,1 s, rebond à 92,71 °C, appoint repris 6,6 s avant l'arrêt ; −0,04 °C en tasse, modèle à −0,53 °C (−1,15 °C avec l'énergie SSR), sous la mesure dès le remplissage ; hors ajustement |
| `261014-094256.json` | 8 s | première routine guidée (62,4 s à la consigne porte-filtre en place, 142,8 s de préparation) ; consigne de 92 °C, cible 22 g (8,1 bar·s/g au plateau), pré-infusion finie sur la goutte ; 16,1 s depuis 8 bar ; pic à 93,67 °C, creux à 90,36 °C ; 0 % pendant 11,3 s, appoint repris 8,5 s avant l'arrêt ; −1,11 °C en tasse, modèle à −0,01 °C (+0,06 °C avec l'énergie SSR) ; hors ajustement |
| `261014-144853.json` | 8 s | routine guidée sur machine sous la consigne (373,6 s à la consigne porte-filtre en place, 70,2 s de préparation) ; consigne de 92 °C, cible 22 g (8,1 bar·s/g au plateau), pré-infusion finie sur la goutte ; 16,0 s depuis 8 bar ; pic à 93,41 °C, sans creux au début de l'infusion ; 0 % pendant 11,4 s, rebond à 92,48 °C, appoint repris 8,3 s avant l'arrêt ; −0,35 °C en tasse, modèle à −0,44 °C (−0,50 °C avec l'énergie SSR) ; un échantillon NTC invalide (200 ms) rendu valide pour l'analyse ; hors ajustement |
| `261015-090846.json` | 8 s | routine guidée à deux rinçages (274 s sur le groupe après le rinçage du porte-filtre, 133,6 s de préparation) ; consigne de 92 °C, cible 22 g (7,6 bar·s/g au plateau), remplissage de 26,1 ml, pré-infusion finie à 4,18 s sur la goutte ou l'échéance ; 16,2 s depuis 8 bar ; SSR coupé ≈ 2 s au début du remplissage, pic à 93,13 °C, creux à 90,34 °C ; 0 % pendant 11,2 s, appoint repris 7,8 s avant l'arrêt ; −1,37 °C en tasse, modèle à +0,33 °C (−0,06 °C avec l'énergie SSR) ; un échantillon NTC invalide (211 ms) rendu valide pour l'analyse ; hors ajustement |
| `261015-144651.json` | 8 s | guide à un rinçage, ouvert sous la consigne (573 s sur le groupe après le rinçage, 102,2 s de préparation) ; consigne de 92 °C, cible 22 g (7,2 bar·s/g au plateau), pré-infusion finie sur la goutte ; 14,8 s depuis 8 bar ; SSR coupé ≈ 1,9 s au début du remplissage, pic à 93,74 °C, palier à 91,40 °C au début de l'infusion ; 0 % pendant 10,9 s, appoint repris 7,6 s avant l'arrêt ; −1,24 °C en tasse, modèle à +0,46 °C (−0,06 °C avec l'énergie SSR) ; trois échantillons NTC invalides (201 à 315 ms) rendus valides pour l'analyse ; hors ajustement |
| `261016-093425.json` | 8 s | écran 0.4.13 : chauffe à 90 % sur tout le remplissage, SSR coupé 0,19 s au plus ; routine guidée (808 s sur le groupe après le rinçage, 92,1 s de préparation) ; consigne de 92 °C, cible 22 g (7,2 bar·s/g au plateau), pré-infusion finie sur la goutte ; 14,7 s depuis 8 bar ; lecture NTC de 204 ms à 15,06 s : régulateur remis à zéro, dette de précharge effacée, aucun pas à 0 % pendant l'infusion, 12,40 kJ au SSR ; +1,16 °C en tasse, NTC à 99,61 °C 17 s après l'arrêt ; modèle à −0,79 °C (−0,76 °C avec l'énergie SSR) ; hors ajustement |
| `261016-141025.json` | 8 s | écran 0.4.13, remboursement complet ; routine guidée (550 s sur le groupe après le rinçage, 93,5 s de préparation) ; consigne de 92 °C, cible 22 g (6,9 bar·s/g au plateau), pré-infusion finie sur la goutte ; 13,2 s depuis 8 bar ; 1,87 kJ de plus au SSR sur le remplissage que le 15 à 14 h 46, tasse identique ; 0 % pendant 10,3 s, appoint repris 6,9 s avant l'arrêt ; −1,23 °C en tasse, modèle à +0,58 °C (+0,50 °C avec l'énergie SSR) ; hors ajustement |
| `261017-103625.json` | 8 s | écran 0.4.14 ; routine guidée (339 s sur le groupe après le rinçage, 78,5 s de préparation, NTC à 89,9 °C à l'ouverture du guide) ; consigne de 92 °C, cible 22 g (6,6 bar·s/g au plateau, un peu de channelling vu), pré-infusion finie sur la goutte en 3,10 s ; 13,4 s depuis 8 bar ; deux lectures NTC invalides (222 et 234 ms) hors des pas thermiques ; 0 % pendant 10,6 s, NTC en hausse à 93,25 °C pendant le remboursement, appoint repris 6,3 s avant l'arrêt ; +0,42 °C en tasse, modèle à −0,62 °C (−0,69 °C avec l'énergie SSR) ; hors ajustement |
| `261017-135720.json` | 8 s | écran 0.4.14 ; routine guidée (140 s sur le groupe après le rinçage, porte-filtre retiré 3,6 s après la consigne, 139,4 s de préparation) ; consigne de 92 °C, cible 22 g (6,9 bar·s/g au plateau), remplissage de 28,4 ml en 8,24 s, pré-infusion finie sur la goutte ; 13,8 s depuis 8 bar ; NTC à 90,25 °C au début de l'infusion ; deux lectures NTC invalides (206 et 201 ms) hors des pas thermiques ; 0 % pendant 10,7 s, appoint repris 6,5 s avant l'arrêt ; −1,50 °C en tasse, café acide ; modèle à +0,88 °C (+0,50 °C avec l'énergie SSR) ; hors ajustement |
| `261018-124031.json` | 8 s | écran 0.4.14 ; premier porte-filtre à bec, même panier, tassage sur support troué ; routine guidée (243 s sur le groupe après le rinçage, 168,6 s de préparation) ; consigne de 92 °C, cible 22 g (**4,9 bar·s/g** au plateau), pré-infusion finie sur la goutte ; 8,4 s depuis 8 bar, 2,0 g de gouttes ; NTC −3,37 °C en pré-infusion, 88,36 °C au début de l'infusion ; lecture NTC invalide (205 ms) sur un pas de pré-infusion : 0 % sur ce pas, dette conservée ; 0 % pendant 9,8 s, appoint repris 2,4 s avant l'arrêt ; −1,56 °C en tasse, légèrement acide ; modèle à +1,36 °C (+1,39 °C avec l'énergie SSR) ; hors ajustement |
| `261020-142940.json` | 8 s | écran 0.4.15 ; mouture affinée, retour au porte-filtre sans fond ; routine guidée ouverte machine en chauffe (NTC à 67,2 °C, 962 s sur le groupe après le rinçage, 92,4 s de préparation) ; consigne de 92 °C, cible 22 g (**9,0 bar·s/g** au plateau, pic à 9,61 bar), remplissage de 21,4 ml, pré-infusion finie sur la goutte ; 18,8 s depuis 8 bar, 0,8 g de gouttes ; NTC +2,25 °C pendant le remplissage, pic à 94,38 °C, 93,70 °C au début de l'infusion ; 0 % pendant 12,1 s, appoint repris 10,2 s avant l'arrêt, NTC à 94,1 °C après la capture ; deux lectures NTC invalides (211 et 209 ms) après l'arrêt ; +0,57 °C en tasse, beaucoup plus acide ; modèle à −0,51 °C (−0,85 °C avec l'énergie SSR), sous la mesure dès le remplissage ; hors ajustement |
| `261023-143811.json` | 8 s | écran 0.4.15, module capteurs sur la carte sensors (0.4.19) ; nouveau café, galette à **12,5 bar·s/g** ; routine guidée (478 s sur le groupe après le rinçage, 71,6 s de préparation, rinçage de 7,2 ml et 14,3 g seulement) ; consigne de 92 °C, cible 22 g ; remplissage fini à l'échéance de 10 s à 0,62 bar, 2,38 ml/s comptés à 75 %, pré-infusion finie à l'échéance, première goutte à 7,33 bar ; 29,7 s depuis 8 bar ; rapport tasse / amont de 1,02 g/ml ; commande de 55 à 65 % au remplissage, 0 % pendant 21,1 s, appoint repris 13,3 s avant l'arrêt ; lecture NTC invalide (206 ms) pendant l'appoint, hors d'un pas ; −2,85 °C en tasse, assez acide, peu de crema ; modèle à −4,78 °C (−4,86 °C avec l'énergie SSR), ramené à 1 °C près en multipliant les volumes par 1,4 à 1,6, mais l'infusion de 14 h 57 colle sans facteur : débitmètre écarté en permanence, écart non expliqué (pompe plus faible et couche froide brassée par la purge à 60 % possibles) ; hors ajustement |
| `261023-145708.json` | 8 s | écran 0.4.15, carte sensors (0.4.19) ; même café, mouture ouverte, galette à **10,9 bar·s/g** ; hors guide, balance non tarée ; consigne de 92 °C, cible 22 g ; remplissage de 22,0 ml en 6,90 s à 3,19 ml/s, pré-infusion finie à l'échéance, première goutte à 9,14 bar ; 26,4 s depuis 8 bar, 0,8 g de gouttes ; NTC +1,03 °C pendant le remplissage, 0 % pendant 21,7 s avec NTC jusqu'à 94,36 °C, appoint repris 8,3 s avant l'arrêt ; trois lectures NTC invalides (200 à 207 ms), commande non nulle ; +0,42 °C en tasse, nettement moins acide, crema tigrée ; modèle à −0,25 °C (−0,19 °C avec l'énergie SSR), mesure au-dessus au remplissage puis −0,18 °C/s jusqu'à −3,09 °C à l'arrêt ; hors ajustement |
| `261023-150828.json` | 8 s | écran 0.4.15, carte sensors (0.4.19) ; même café, mouture ouverte, galette à 6,8 bar·s/g ; routine guidée (205 s sur le groupe après le rinçage, 69,9 s de préparation) ; consigne de 92 °C, cible 22 g ; remplissage de 24,3 ml en 7,03 s à 3,46 ml/s, pré-infusion finie à l'échéance, première goutte à 5,64 bar ; 15,9 s depuis 8 bar, 1,2 g de gouttes ; NTC +0,83 °C pendant le remplissage, 0 % pendant 16,0 s, appoint repris 3,3 s avant l'arrêt ; lecture NTC invalide (203 ms) 0,1 s avant l'arrêt ; +0,86 °C en tasse, un peu « brûlé » ; modèle à −0,53 °C (−0,20 °C avec l'énergie SSR) ; hors ajustement |
| `261024-102248.json` | 8 s | écran 0.4.15, carte sensors (0.4.19) ; même café, même mouture, galette à 6,7 bar·s/g ; routine guidée (220 s sur le groupe après le rinçage, 96,0 s de préparation) ; **consigne de 91 °C**, cible 22 g ; **remplissage à 70 %** : 28,1 ml en 8,98 s à 3,13 ml/s, pré-infusion finie à l'échéance, première goutte à 1,55 bar ; 13,9 s depuis 8 bar, 1,7 g de gouttes ; creux de −0,62 °C au remplissage, 0 % pendant 15,4 s, appoint repris 1,8 s avant l'arrêt ; lecture NTC invalide (236 ms) au début de la précharge ; +1,15 °C en tasse, goût bon ; modèle à −0,02 °C ; hors ajustement |
| `261024-132354.json` | 8 s | écran 0.4.15, carte sensors (0.4.19) ; mêmes réglages qu'à 10 h 22, galette à 6,5 bar·s/g ; routine guidée ouverte sous la consigne (608 s sur le groupe après le rinçage, 86,2 s de préparation, balance absente 14,3 s avant l'infusion) ; consigne de 91 °C, cible 22 g ; remplissage à 70 % : 20,0 ml en 6,46 s, pré-infusion finie à l'échéance, première goutte à 1,85 bar ; 14,1 s depuis 8 bar, pic à 9,11 bar, 1,6 g de gouttes ; NTC +1,27 °C pendant le remplissage, 0 % pendant 15,2 s, appoint repris 2,1 s avant l'arrêt ; +0,82 °C en tasse, goût bien ; modèle à −0,60 °C ; hors ajustement |
| `261025-091128.json` | 8 s | écran 0.4.23 (profil « single espresso », capture au schéma v3), carte sensors (0.4.19) ; mêmes réglages, tassage plus ferme, galette à **7,8 bar·s/g** ; routine guidée ouverte sous la consigne (416 s sur le groupe après le rinçage, 108,6 s de préparation, balance absente 11,8 s avant l'infusion) ; consigne de 91 °C, cible 22 g ; remplissage à 70 % : 21,4 ml en 6,84 s, circuit à 0,40 bar au départ, pré-infusion finie sur la goutte (1,28 bar) ; 16,8 s depuis 8 bar, sans pic, 0,9 g de gouttes ; NTC +1,24 °C pendant le remplissage, 0 % pendant 16,5 s sans remontée, appoint repris 3,8 s avant l'arrêt ; +0,17 °C en tasse, galette dégradée au visuel, amertume pas très agréable ; modèle à −1,03 °C ; hors ajustement |
| `261026-093443.json` | 8 s | écran et sensors 0.4.24 ; premier **turbo shot** (profil « turbo shot », 11 g en panier 58 mm 15 g, cible 45 g, 6 bar), même mouture que l'espresso, tassé (« mis à niveau », déjà un tassage maximal selon David), galette à **0,43 bar·s/g** ; routine guidée (418 s sur le groupe après le rinçage, 130,6 s de préparation, balance absente 15,2 s avant l'infusion) ; consigne de 91 °C ; remplissage fini à l'échéance de 10 s à 0,77 bar, 28,7 ml, pré-infusion finie sur la goutte (0,80 bar, 32,2 ml) ; pompe à 100 % dès 2,9 s, plateau à 2,1 bar, 3,7 ml/s comptés, ≈ 5 g/s en tasse, 10,9 s d'infusion, 48,5 g final pour 44 g de seuil ; 71,3 ml admis ; 0 % pendant 9,4 s, appoint repris 1,45 s avant l'arrêt, minimum de 84,59 °C 8,3 s après l'arrêt ; −1,63 °C en tasse, zéro amertume, floral ; modèle à −2,07 °C, hors de son domaine de débit ; hors ajustement |
| `261026-133226.json` | 8 s | écran et sensors 0.4.24 ; deuxième **turbo shot**, cible 40 g, **consigne de 88 °C**, même mouture et même tassage, galette à **0,38 bar·s/g** ; routine guidée (1681 s sur le groupe après le rinçage, 197,6 s de préparation) ; remplissage fini sur la pression à 0,89 bar, 25,2 ml, pré-infusion finie sur la goutte (0,90 bar, 26,4 ml) ; pompe à 100 % dès 2,7 s, plateau à 1,9 bar, 3,7 ml/s comptés, ≈ 5 g/s en tasse, 9,6 s d'infusion, 44,2 g final pour 39 g de seuil ; 61,7 ml admis ; 0 % pendant 8,9 s, appoint repris 0,67 s avant l'arrêt, NTC à +1,33 °C au début de l'infusion, minimum de 83,71 °C 6,4 s après l'arrêt ; +0,74 °C en tasse, très léger, sans crema ; modèle à +0,85 °C, hors de son domaine de débit ; hors ajustement |
| `261026-153548.json` | 8 s | écran et sensors 0.4.24 ; troisième **turbo shot**, **15 g** dans le panier 58 mm 15 g, cible 45 g, `lead_weight_g` 5,0 g, consigne de 88 °C, même mouture, galette à **1,10 bar·s/g** ; routine guidée ouverte sous la consigne (1206 s sur le groupe après le rinçage, 85,2 s de préparation) ; remplissage fini sur la pression à 0,97 bar, 19,7 ml, pré-infusion finie à l'échéance (1,31 bar) ; plateau à 4,85 bar, pompe à 97–100 %, 3,3 ml/s comptés, 4,4 g/s en tasse, 9,9 s de la goutte à l'arrêt, 43,9 g final pour 40 g de seuil ; 62,6 ml admis ; 0 % pendant 9,3 s, appoint repris 1,93 s avant l'arrêt, minimum de 82,73 °C 6,0 s après l'arrêt ; +1,37 °C en tasse, crema onctueuse, légère amertume ; modèle à +0,49 °C ; hors ajustement |
| `261026-163228.json` | 8 s | écran et sensors 0.4.24 ; quatrième **turbo shot**, 15 g, **mouture nettement plus grosse**, cible 45 g, consigne de 88 °C ; galette à 0,52 bar·s/g, qui s'érode (0,58 → 0,49, channelling vu par David) ; routine guidée (1003 s sur le groupe après le rinçage, 91,3 s de préparation) ; remplissage fini sur la pression (0,87 bar, 18,3 ml), pré-infusion finie sur la goutte (2,43 s) ; plateau de 2,52 à 2,33 bar, 4,7 g/s en tasse, 10,6 s de la goutte à l'arrêt, 44,0 g final ; 0 % pendant 9,1 s, appoint repris 1,50 s avant l'arrêt ; NTC de 90,97 à 84,05 °C pendant l'écoulement, minimum de 80,28 °C ; +0,77 °C en tasse, plat ; modèle à +0,09 °C en tasse mais minimum à 86,16 °C ; hors ajustement |
| `261027-074432.json` | 8 s | écran et sensors 0.4.24 ; cinquième **turbo shot**, 15 g, mouture grosse, cible 45 g, consigne de 88 °C, **pompe à 70 %** et **puck screen** ; galette à 0,51 bar·s/g, constante à ± 2 % (plus d'érosion) ; routine guidée (921 s sur le groupe après le rinçage, 105,7 s de préparation) ; remplissage fini sur la pression (0,88 bar, 24,3 ml), pré-infusion finie sur la goutte (0,50 s) ; plateau de 1,85 à 1,90 bar, 3,8 g/s en tasse, 12,5 s de la goutte à l'arrêt, 43,0 g final ; 0 % pendant 10,4 s, appoint repris 2,42 s avant l'arrêt ; NTC de 89,83 à 86,10 °C pendant l'écoulement, minimum de 82,80 °C 7,9 s après l'arrêt ; +0,20 °C en tasse, jugé trop chaud ; modèle à 0,00 °C en tasse mais minimum à 86,30 °C ; sert aux variantes de précharge et « sans chauffe » du 27/10 ; hors ajustement |
| `261027-131653.json` | 2 s | écran et sensors 0.4.24 ; sixième **turbo shot**, 15 g, mouture grosse, puck screen, **pompe et remplissage à 100 %**, **consigne de 86 °C** ; galette à 0,49 → 0,55 bar·s/g (+9 % corrigé de la viscosité) ; routine guidée ouverte machine froide (331 s sur le groupe après le retour à la consigne, 108,7 s de préparation) ; remplissage fini sur la pression (0,94 bar, 19,7 ml), pré-infusion sautée ; 2,34 → 2,53 bar, 4,6 g/s en tasse, 9,6 s de la goutte à l'arrêt, 43,7 g final ; 0 % pendant 2,58 s ; NTC de 85,99 à 79,73 °C pendant l'écoulement, minimum de 78,91 °C 3,3 s après l'arrêt ; −3,48 °C en tasse, un peu amer et plat ; modèle à −0,23 °C en tasse, minimum à 79,36 °C ; hors ajustement |
| `261027-134905.json` | 2 s | écran et sensors 0.4.24 ; septième **turbo shot**, 15 g, **mouture espresso**, puck screen, pompe et remplissage à 100 %, consigne de 86 °C ; galette à 1,26–1,39 bar·s/g (1,25 corrigé de la viscosité, +12 % sur 15 h 35 la veille) ; routine guidée (458 s sur le groupe, 87,0 s de préparation) ; remplissage fini sur la pression (0,95 bar, 23,2 ml), pré-infusion finie à l'échéance (4,12 s) ; PI de pression actif dès 4 bar, pompe à 86–92 %, 4,9 → 5,58 bar, 4,0 g/s en tasse, 10,6 s de la goutte à l'arrêt, 42,9 g final ; 0 % pendant 3,30 s ; NTC de 85,77 à 83,62 °C pendant la pré-infusion, 81,63 °C à l'arrêt, minimum de 80,54 °C 3,5 s après ; −2,84 °C en tasse, moins amer, moins bon qu'à 89 °C ; modèle à −0,97 °C en tasse ; hors ajustement |
| `261028-100949.json` | 8 s | écran et sensors 0.4.24 ; retour au profil « single espresso » avec le **porte-filtre à bec**, même café, mouture espresso, galette à **7,4 bar·s/g** ; routine guidée (1222 s sur le groupe après le rinçage, 259 s de préparation, balance absente 25,5 s avant l'infusion) ; consigne de 91 °C, cible 22 g ; remplissage à 70 % : 17,1 ml en 5,42 s, circuit à 0,51 bar au départ, pré-infusion finie à l'échéance ; **première goutte à 8,98 bar**, 0,2 g en tasse à 8 bar : le bec retient le filet de basse pression ; 18,3 s depuis 8 bar, 1,4 g de gouttes ; NTC +2,30 °C pendant le remplissage, 0 % pendant 16,5 s, appoint repris 5,3 s avant l'arrêt ; +1,04 °C en tasse, goût vraiment bien ; modèle à −0,53 °C ; hors ajustement |
| `261028-134735.json` | 8 s | écran et sensors 0.4.24 ; single espresso, porte-filtre à bec, même café en **fin de sachet**, galette à **5,5 bar·s/g** ; routine guidée (435 s sur le groupe après le rinçage, 133 s de préparation, balance absente 8,9 s avant l'infusion) ; **rinçage à 12,8 ml et 14,1 g** ; consigne de 91 °C, cible 22 g ; remplissage à 70 % : **26,7 ml en 10,01 s (2,5 à 3,0 ml/s)**, circuit à 0,25 bar au départ, +9,6 à +10,1 ml à chaque palier de pression sur 10 h 09 ; pré-infusion finie à l'échéance ; première goutte à 2,65 bar, 3,4 g à 8 bar ; 11,3 s depuis 8 bar, 1,9 g de gouttes ; NTC +0,13 °C pendant le remplissage, 0 % pendant toute l'infusion (15,0 s), appoint jamais repris ; −0,88 °C en tasse, belle crema, arôme agréable ; **modèle à −2,77 °C**, −0,08 °C avec les volumes × 1,3 ; hors ajustement |
| `261029-091927.json` | 8 s | écran et sensors 0.4.24 ; single espresso, porte-filtre à bec, **dernière dose du Cozy Chocolate**, galette à **6,0 bar·s/g** ; routine guidée (**1081 s sur le groupe** après le rinçage, 95,9 s de préparation, balance absente 12,2 s avant l'infusion) ; rinçage à 17,4 ml et 21,0 g ; consigne de 91 °C, cible 22 g ; remplissage à 70 % : 22,9 ml en 7,29 s (3,0 à 3,3 ml/s), circuit à 0,29 bar au départ, fin sur la pression ; **pré-infusion sautée**, la balance passant de −0,1 à 0,0 g pendant le remplissage ; première goutte à 6,25 bar, 0,9 g à 8 bar ; 14,5 s depuis 8 bar, 1,5 g de gouttes ; NTC +1,06 °C pendant le remplissage, 0 % pendant 14,4 s, appoint repris 4,4 s avant l'arrêt ; −0,42 °C en tasse, excellent, très bons arômes ; modèle à −0,47 °C ; hors ajustement |
| `261029-140951.json` | 8 s | écran et sensors 0.4.24 ; single espresso, **retour au ViCafé**, **porte-filtre sans fond**, galette à **8,7 bar·s/g** (mouture trop fine) ; routine guidée (561 s sur le groupe après le rinçage, 90,4 s de préparation, balance absente 16,4 s avant l'infusion) ; consigne de 91 °C, cible 22 g ; remplissage à 70 % : 22,6 ml en 7,23 s (3,0 à 3,3 ml/s), circuit à 0,26 bar au départ ; pré-infusion finie à l'échéance ; première goutte à 6,95 bar, pic à 9,46 bar ; 20,1 s depuis 8 bar, arrêt au poids, 1,0 g de gouttes ; NTC +0,38 °C pendant le remplissage, −1,78 °C en pré-infusion, 0 % pendant 18,1 s, appoint repris 5,2 s avant l'arrêt ; −0,50 °C en tasse ; **modèle à −1,96 °C**, écart formé pendant la pré-infusion ; hors ajustement |
| `261029-142259.json` | 8 s | écran et sensors 0.4.24 ; single espresso, ViCafé, sans fond, **mouture ouverte**, galette à **6,3 bar·s/g** ; routine guidée (528 s sur le groupe après le rinçage, 78,5 s de préparation) ; **consigne de 90 °C**, cible 22 g ; remplissage à 70 % : 21,2 ml en 6,74 s (2,9 à 3,2 ml/s), circuit à 0,29 bar au départ ; pré-infusion finie à l'échéance ; première goutte à 3,18 bar, 2,1 g à 8 bar ; 14,4 s depuis 8 bar, 1,4 g de gouttes ; NTC +2,02 °C pendant le remplissage, 0 % pendant 17,7 s, appoint repris 0,3 s avant l'arrêt ; **+1,30 °C en tasse**, +0,80 °C sur 14 h 07 malgré la consigne abaissée ; très bien ; modèle à −0,49 °C ; hors ajustement |
| `monitor-heating-20260928-095631-275212.json` | — | ajustement ; montée sans écoulement depuis 80 °C |
| `260928-083730.json` | 5 s | exclue (fenêtre SSR de 5 s) |

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
105 °C), puis, pendant la précharge et l'écoulement d'une infusion,
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
groupe pendant l'infusion**. L'offset de −10 °C entre la sonde et la
température utilisateur (voir
[chauffe-chaudiere.md](chauffe-chaudiere.md#conversion-actuelle)) va dans ce
sens : environ 100 °C à la sonde pour 90 °C au groupe.

Tant qu'aucune mesure au panier n'existe, le critère est la **NTC en
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

Hypothèse non vérifiée : l'offset de −10 °C vient des essais de *flashing*
au repos. Rien ne dit qu'il tient pendant l'écoulement, avec l'inertie du
groupe. Un panier de mesure de type Scace, en préparation, mesurera l'eau
dans le panier. Il remplacera la NTC comme critère et permettra de recaler
l'offset.

Écart au critère, consigne de 90 °C :

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
est confirmée en marche.

Seuls modifient cette commande :

| Condition | Commande |
| --- | --- |
| NTC au-delà de consigne + **4 °C** (sécurité) | 0 % |
| Pré-infusion, pompe confirmée en marche, aucune impulsion depuis 500 ms (depuis 0.3.32) | **3,5 %** : débit nul, pas inconnu |
| Débit non mesurable (conditions ci-dessus), hors de ce cas | **45 %** (repli) |
| Mesure invalide, chauffe désactivée, NTC > 105 °C | 0 % (coupures générales) |

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
| Coupure générale pendant l'écoulement (mesure invalide, chauffe désactivée, NTC > 105 °C) | régulateur remis à zéro : la dette est perdue, l'appoint reprend normalement |

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
  courbe de la sonde. Le panier de mesure en préparation dira si le creux
  de la NTC est aussi celui de la tasse, et si l'offset de −10 °C tient
  pendant l'écoulement (voir [Objectif](#objectif)).
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
  30 septembre) et la commande passe au plafond. Cela dépend de la phase
  des premières impulsions. Les simulations incluent ces écarts, puisqu'elles
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
| `monitor-heating-20260928-095631-275212.json` | — | ajustement ; montée sans écoulement depuis 80 °C |
| `260928-083730.json` | 5 s | exclue (fenêtre SSR de 5 s) |

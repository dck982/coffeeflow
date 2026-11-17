# Journal des infusions

Ce journal consigne l'analyse de chaque infusion capturée tant que le
firmware n'est pas stabilisé. Il couvre l'hydraulique (remplissage,
pré-infusion, montée, débit) autant que la chauffe. Il ne décrit pas les
lois ; il dit ce que chaque infusion a montré et ce qu'on règle ensuite.

| Sujet | Document |
| --- | --- |
| Loi de chauffe pendant l'infusion, simulation, remboursement de la précharge | [chauffe-infusion.md](chauffe-infusion.md) |
| Régulation de la chaudière, sonde, journal thermique jusqu'au 4 octobre | [chauffe-chaudiere.md](chauffe-chaudiere.md) |
| Régulation au repos | [chauffe-repos.md](chauffe-repos.md) |
| Débit libre selon la puissance de pompe, choix de remplissage et de pré-infusion | [purge_sans_resistance.html](purge_sans_resistance.html) |
| Réglages (`filling.*`, `preinfusion.*`) et règles de fin de phase | [firmware.md](firmware.md) |

Les fichiers `captures/` sont locaux et ignorés par Git.

## Analyser une capture

Les mesures d'une entrée sortent de
`uv run firmware/analysis/describe_hf_capture.py captures/<capture>.json`,
avec `--dose-g` quand la dose a été pesée. Ne pas les recalculer avec du
Python écrit à la main pour l'occasion : une définition qui change d'une
entrée à l'autre rend les entrées incomparables. Le bloc « mesures du
journal » imprimé dans le terminal donne toutes les valeurs d'une entrée :
phases, première goutte, seuil de 8 bar, poids à l'arrêt et final,
plateau, arrivées en tasse, thermique, ratio et session guidée. Une mesure qui manque au
rapport s'ajoute au script, avec un test, avant d'entrer au journal.

La **résistance au plateau** est celle du script, imprimée dans le
terminal et affichée dans le rapport : pression moyenne de consigne − 1 bar
+ 2 s jusqu'au dernier échantillon d'infusion avant l'arrêt de la pompe,
divisée par le poids gagné en tasse sur cette fenêtre par seconde, en
bar·s/g. Elle ne dépend pas de la dose ; avec `--dose-g`, le script la
ramène aussi à la dose du profil, proportionnellement à la dose.

## Réglages en cours

Profil actif « single espresso » : Cozy Chocolate depuis le 15 novembre
(ViCafé du 29 octobre au 11 novembre), panier LM1 41 mm, **porte-filtre
sans fond**. Dernière configuration sauvegardée : `configs/261031-154935.json`
(schéma 13) ; les changements suivants sont lus dans les captures. Le profil
« turbo shot » et ses essais sont dans [turboshot.md](turboshot.md).
L'historique des réglages avant le 4 novembre est dans
`git show 7c17133:docs/journal-infusions.md`.

| Réglage | Valeur | Depuis |
| --- | --- | --- |
| Firmware | écran et carte sensors **0.5.8** | 17 novembre, 14 h 30 (0.5.7 à 8 h 08 ; écran 0.5.4 et sensors 0.5.3 le 14 novembre, écran 0.4.28 au 11 novembre) |
| Pompe | **à engrenage MPB003**, validée au goût le 17 novembre, fuite interne ≈ 3,2 ml/s à 9 bar, plateau de 9 bar vers 66 à 71 % pour ≈ 1 ml/s ; Digmesa à 2 490 imp/L. Voir [pompe-engrenage.md](pompe-engrenage.md) | 14 novembre (vibratoire OLAB avant) |
| Dose | **7,5 g**, pesée à chaque infusion | 30 octobre ; pesée depuis le 2 novembre |
| Mouture | Cozy Chocolate : ≈ deux crans plus serré que le cran ViCafé du 10 novembre (estimation de David) le 15, **desserré de deux crans** ensuite | 15 novembre |
| Offset NTC | **−14 °C** (écran 0.4.4), mesuré au panier avec la SCACE ; −10,5 °C avant. Consigne bornée à 95 °C, coupure à 100 °C utilisateur (114 °C sonde) | 10 octobre |
| Sonde de pression | XDB401 lu sur J10 de la carte sensors avec la calibration de A2, loi de chauffe inchangée | 23 octobre (A2 de l'écran depuis le 7 octobre) |
| Précharge | 8 s à 90 % | 4 octobre |
| Consigne | **92 °C** | 17 novembre (90 °C du 29 octobre ; 91 °C du 24 octobre ; 92 °C du 13 octobre ; 90 °C avant) |
| Remplissage | pompe à **100 %**, fin à 0,8 bar (10 s au plus) | 16 novembre, 15 h 02 (70 % du 24 octobre ; 75 % avant) |
| Pré-infusion | pompe à 50 %, fin à 4 s ou au premier poids en tasse | 5 octobre, 7 h 59 |
| Infusion | pompe jusqu'à 100 %, cible 9 bar, arrêt à **20 g** ou 27 s, pompe arrêtée **1,0 g** avant la cible | cible depuis le 1er novembre, 13 h 31 ; anticipation depuis le 4 novembre après 9 h 44 (1,5 g du 31 octobre) |
| Purge | pompe à **80 %**, 10 s au plus | 24 octobre, 10 h 22 |
| Machine | fermée (panneaux posés), filtre ECM Aqua Aroma Crema dans le réservoir ; état final. Remplissage à 70 % : 2,9 à 3,3 ml/s avec la vibratoire, 3,6 ml/s avec la MPB003 | 10 octobre |

Depuis l'écran 0.4.22, la capture (schéma v3) enregistre les versions du
firmware et le profil utilisé.

## Méthode

### Commandes

```sh
uv run firmware/analysis/describe_hf_capture.py captures/<capture>.json
uv run firmware/analysis/analyze_hf_capture.py captures/<capture>.json
uv run firmware/analysis/simulate_boiler.py --capture captures/<capture>.json
```

Le rapport HTML contient, sous le graphe, une table texte par phase (durée,
volume, débit, pression, poids, NTC, commande et énergie SSR) et les
repères hydrauliques comptés depuis le début du remplissage. Ce sont les
sources des chiffres de chaque entrée. `shot_summary()` du même script
donne la durée et les débits depuis 8 bar.

### Grandeurs relevées

Toujours avec ces définitions, pour que les entrées restent comparables :

| Grandeur | Définition |
| --- | --- |
| Remplissage | durée, volume et pression de sortie de la table par phase |
| Pression de fin réelle | premier paquet au-dessus du seuil (repère « pression ≥ … ») et passage en pré-infusion ; noter l'écart |
| Pré-infusion | durée, volume, et ce qui l'a terminée : première goutte (firmware, +0,1 g) ou échéance |
| Première goutte | repère du rapport (poids ≥ 0,3 g), en temps et en volume depuis le remplissage ; le firmware déclenche plus tôt (+0,1 g) |
| Marge de remplissage | volume à la première goutte − volume à la fin du remplissage |
| Montée | temps de la fin de pré-infusion à 8 bar |
| Infusion | durée **depuis 8 bar** et débits moyens (tasse en g/s, amont en ml/s) depuis 8 bar ; la durée de la phase « infusion » (fin de pré-infusion → arrêt) est notée à part, c'est celle des tableaux de `chauffe-infusion.md` |
| Rendement | poids en tasse à l'arrêt de la pompe et poids final (gouttes comprises) |
| Thermique | NTC pondérée par la tasse (`analyze_hf_capture.py`, critère de la consigne) ; minimum pendant l'écoulement ; minimum total et son instant ; NTC à l'arrêt et en fin de capture (20 s après jusqu'à 0.4.19, 10 s depuis 0.4.20) ; maximum du rebond relevé à l'écran |
| Routine guidée | durée du porte-filtre sur le groupe (retour à la consigne + porte-filtre en place après la consigne, depuis la fin du rinçage du porte-filtre) et préparation, rapprochées de l'écart du modèle avec l'énergie SSR ; voir [wizard.md](wizard.md#piste--temps-minimal-sur-le-groupe) |
| Énergie | commande et SSR par phase ; l'écart entre les deux dit ce que le simulateur surestime |

### Simulation

`simulate_boiler.py` ne modélise que la NTC, sur l'hydraulique réelle de la
capture. Il ne simule ni le remplissage ni la pré-infusion : un effet de
réglage hydraulique s'estime sur la courbe pression–volume de la capture et
se présente comme une estimation.

Le modèle se trompe de 1 à 2 °C en valeur absolue (moyenne en tasse,
minimum, récupération). Ne lire que les **écarts entre variantes** rejouées
sur la même capture, et rejouer aussi avec l'énergie SSR (`heater_on`), pas
seulement la commande. Tout résultat « confirmé » doit l'être par une
mesure ; un résultat du modèle s'écrit « en simulation ».

### Rédaction d'une entrée

1. **Contexte** : capture, réglages changés depuis l'entrée précédente,
   observations de David (visuel, goût), telles quelles.
2. **Hydraulique** puis **thermique** : chiffres mesurés, comparés aux
   entrées précédentes avec les mêmes définitions.
3. **Attendus vérifiés** : reprendre un par un les attendus de l'entrée
   précédente, avec la mesure en regard. Un attendu non atteint se dit.
4. **Lecture** : causes probables, chacune marquée mesurée, estimée ou
   simulée. Une hypothèse non testée s'écrit comme telle.
5. **Décision et attendus suivants** : réglages retenus et valeurs
   attendues à la prochaine infusion, chiffrées.

Règles :

- deux réglages changés à la fois : dire quelle mesure sépare leurs effets,
  ou que rien ne les sépare ;
- ne pas reprendre un chiffre d'une autre doc sans vérifier sa définition ;
- une erreur découverte dans une entrée passée se corrige en place, avec
  une ligne « Correction du <date> » ;
- ne modifier ni le firmware ni les réglages : les décisions viennent de
  David ;
- une entrée de plus de 7 jours sort des entrées : on la résume dans
  le [chemin de recherche](#chemin-de-recherche), et on donne le commit
  qui garde son détail ;
- reporter la moyenne en tasse et la capture dans les tableaux « Écart au
  critère » et « Captures de référence » de
  [chauffe-infusion.md](chauffe-infusion.md) ; changer
  [firmware.md](firmware.md) seulement si un défaut ou une règle de fin de
  phase change.

## Synthèse

Valeurs du rapport (définitions ci-dessus). « NTC tasse » est dans le
domaine de l'offset de −10,5 °C jusqu'au 9 octobre, de −14 °C ensuite :
retirer 3,5 °C aux premières pour les comparer. Les infusions avant le
5 octobre finissaient le remplissage sur +0,1 bar au-dessus du plancher
(schéma 8) et pré-infusaient à 60 % (35 % le 4 octobre à 13 h 24).

| Infusion | Remplissage | Pré-infusion | 1re goutte | Montée à 8 bar | Depuis 8 bar | Tasse | NTC tasse |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: |
| 30/09 14 h 25 | 4,05 s, 13,6 ml, 0,42 bar | 3,30 s, 11,0 ml, goutte | 24,6 ml | 5,3 s | 9,3 s, 1,47 g/s, 1,90 ml/s | 22,3 g | 89,60 °C |
| 01/10 8 h 29 | 4,40 s, 14,8 ml, 0,42 bar | 1,30 s, 4,1 ml, goutte | 18,8 ml | 6,4 s | 7,3 s, 1,59 g/s, 2,06 ml/s | 22,5 g | 88,42 °C |
| 04/10 10 h 01 | 3,00 s, 10,1 ml, 0,33 bar | 4,00 s, 12,5 ml, échéance | 38,0 ml | 4,6 s | 29,8 s, 0,71 g/s, 0,91 ml/s | 22,2 g | 92,50 °C |
| 04/10 13 h 24 | 4,50 s, 15,4 ml, 0,39 bar | 4,00 s, 2,0 ml, échéance | 35,1 ml | 6,4 s | 17,8 s, 1,10 g/s, 1,40 ml/s | 22,2 g | 87,58 °C |
| 05/10 7 h 26 | 7,00 s, 24,1 ml, 1,19 bar | 1,20 s, 1,2 ml, goutte | 25,8 ml | 4,5 s | 13,3 s, 1,28 g/s, 1,85 ml/s | 22,4 g | 88,64 °C |
| 06/10 7 h 19 | 6,10 s, 21,2 ml, 0,90 bar | 2,10 s, 4,1 ml, goutte | 25,5 ml | 4,8 s | 11,8 s, 1,35 g/s, 1,89 ml/s | 22,6 g | 88,47 °C |
| 08/10 8 h 43 | 6,30 s, 20,0 ml, 0,88 bar | 3,40 s, 6,1 ml, goutte | 26,7 ml | 4,5 s | 16,0 s, 1,07 g/s, 1,16 ml/s | 22,2 g | 87,69 °C |
| 08/10 13 h 44 | 5,80 s, 20,3 ml, 0,83 bar | 3,40 s, 6,7 ml, goutte | 27,0 ml | 4,0 s | 15,0 s, 1,15 g/s, 1,39 ml/s | 22,2 g | 88,80 °C |
| 09/10 7 h 01 | 6,00 s, 20,6 ml, 0,89 bar | 3,10 s, 6,7 ml, goutte | 29,3 ml | 4,4 s | 13,7 s, 1,27 g/s, 1,93 ml/s | 22,2 g | 89,26 °C |
| **10/10 11 h 01**, offset −14 °C | 8,30 s, 24,9 ml, 0,83 bar | 0,84 s, 1,4 ml, goutte | 27,2 ml | 5,4 s | 9,1 s, 1,52 g/s, 1,98 ml/s | 22,8 g | 87,11 °C |
| 10/10 13 h 22 | 8,28 s, 24,9 ml, 0,87 bar | 3,65 s, 7,5 ml, goutte | 32,8 ml | 4,0 s | 13,7 s, 1,27 g/s, 1,69 ml/s | 22,4 g | 88,53 °C |
| 11/10 9 h 47 | 6,67 s, 23,5 ml, 0,90 bar | 3,56 s, 6,7 ml, goutte | 30,1 ml | 3,7 s | 13,3 s, 1,32 g/s, 2,23 ml/s | 22,5 g | 87,38 °C |
| **11/10 13 h 40**, cible 23 g | 6,07 s, 21,2 ml, 0,85 bar | 2,34 s, 4,6 ml, goutte | 25,8 ml | 4,6 s | 11,9 s, 1,39 g/s, 1,98 ml/s | 23,7 g | 87,29 °C |
| **12/10 9 h 12**, mouture affinée | 5,35 s, 18,6 ml, 0,89 bar | 4,03 s, 8,7 ml, échéance | 30,1 ml | 3,3 s | 18,7 s, 1,04 g/s, 1,85 ml/s | 23,3 g | 88,75 °C |
| 12/10 13 h 34 | 5,43 s, 18,6 ml, 0,86 bar | 2,47 s, 6,4 ml, goutte | 25,5 ml | 4,3 s | 14,4 s, 1,26 g/s, 1,99 ml/s | 23,3 g | 87,96 °C |
| **13/10 8 h 18**, consigne 92 °C | 5,12 s, 17,7 ml, 0,96 bar | 4,00 s, 8,1 ml, échéance | 28,7 ml | 3,2 s | 18,5 s, 1,09 g/s, 2,03 ml/s | 23,6 g | 89,71 °C |
| **13/10 13 h 29**, cible 22 g | 6,35 s, 21,7 ml, 0,98 bar | 3,28 s, 7,8 ml, goutte | 30,1 ml | 3,2 s | 15,6 s, 1,18 g/s, 2,06 ml/s | 22,4 g | 91,96 °C |
| **14/10 9 h 42**, mode guidé | 7,08 s, 24,3 ml, 0,92 bar | 1,76 s, 4,9 ml, goutte | 31,0 ml | 3,9 s | 16,1 s, 1,10 g/s, 2,04 ml/s | 22,5 g | 90,89 °C |
| 14/10 14 h 48 | 6,88 s, 24,1 ml, 0,87 bar | 2,84 s, 7,2 ml, goutte | 32,2 ml | 3,9 s | 16,0 s, 1,10 g/s, 2,13 ml/s | 22,7 g | 91,65 °C |
| 15/10 9 h 08 | 7,34 s, **26,1 ml**, 0,89 bar | 4,18 s, 9,3 ml, goutte ou échéance | 36,5 ml | 2,9 s | 16,2 s, 1,16 g/s, 2,26 ml/s | 22,8 g | 90,63 °C |
| **15/10 14 h 46**, guide à un rinçage | 6,92 s, 24,1 ml, 0,81 bar | 3,01 s, 6,4 ml, goutte | 32,2 ml | 3,8 s | 14,8 s, 1,21 g/s, 2,25 ml/s | 22,9 g | 90,76 °C |
| **16/10 9 h 34**, écran 0.4.13, dette effacée | 6,61 s, 22,9 ml, 0,96 bar | 1,99 s, 5,2 ml, goutte | 28,1 ml | 3,7 s | 14,7 s, 1,21 g/s, 2,17 ml/s | 22,5 g | **93,16 °C** |
| 16/10 14 h 10 | 6,63 s, 23,2 ml, 0,92 bar | 1,71 s, 4,1 ml, goutte | 29,0 ml | 4,3 s | 13,2 s, 1,31 g/s, 2,14 ml/s | 22,7 g | 90,77 °C |
| **17/10 10 h 36**, écran 0.4.14 | 7,24 s, 24,9 ml, 0,85 bar | 3,10 s, 6,7 ml, goutte | 33,0 ml | 3,7 s | 13,4 s, 1,32 g/s, 2,40 ml/s | 22,9 g | 92,42 °C |
| 17/10 13 h 57 | 8,24 s, **28,4 ml**, 0,94 bar | 3,07 s, 7,0 ml, goutte | 36,2 ml | 3,6 s | 13,8 s, 1,29 g/s, 2,41 ml/s | 22,7 g | 90,50 °C |
| **18/10 12 h 40**, porte-filtre à bec | 7,30 s, 25,2 ml, 0,87 bar | 3,79 s, 8,1 ml, goutte | 33,3 ml | 3,9 s | **8,4 s, 1,71 g/s**, 2,53 ml/s | 23,6 g | 90,44 °C |
| **20/10 14 h 29**, mouture affinée, sans fond, écran 0.4.15 | 6,24 s, 21,4 ml, 0,86 bar | 3,75 s, 8,1 ml, goutte | 30,1 ml | 3,6 s | **18,8 s, 0,98 g/s**, 2,01 ml/s | 22,3 g | 92,57 °C |
| **23/10 14 h 38**, nouveau café, carte sensors, débit faible inexpliqué | **10,01 s**, 23,8 ml, **0,62 bar** | 4,18 s, 3,8 ml, échéance | 40,6 ml | 4,9 s | **29,7 s, 0,70 g/s**, 0,72 ml/s | 22,3 g | **89,15 °C** |
| 23/10 14 h 57, mouture ouverte, hors guide | 6,90 s, 22,0 ml, 0,85 bar | 4,02 s, 4,3 ml, échéance | 36,2 ml | 3,8 s | **26,4 s, 0,82 g/s**, 0,91 ml/s | 22,4 g | 92,42 °C |
| 23/10 15 h 08, mouture ouverte, guide | 7,03 s, 24,3 ml, 0,91 bar | 3,99 s, 5,5 ml, échéance | 37,1 ml | 3,5 s | 15,9 s, 1,26 g/s, 1,58 ml/s | 22,7 g | 92,86 °C |
| **24/10 10 h 22**, consigne 91 °C, remplissage à 70 % | **8,98 s, 28,1 ml**, 0,86 bar | 3,90 s, 5,8 ml, échéance | 35,1 ml | 3,5 s | 13,9 s, 1,32 g/s, 1,63 ml/s | 23,1 g | 92,15 °C |
| 24/10 13 h 23 | 6,46 s, 20,0 ml, 0,89 bar | 4,08 s, 5,8 ml, échéance | 27,5 ml | 3,4 s | 14,1 s, 1,36 g/s, 1,55 ml/s | 23,0 g | 91,82 °C |
| **25/10 9 h 11**, écran 0.4.23, tassage plus ferme | 6,84 s, 21,4 ml, 0,85 bar | 3,50 s, 5,2 ml, goutte | 26,7 ml | 3,6 s | **16,8 s, 1,09 g/s**, 1,29 ml/s | 22,5 g | 91,17 °C |
| **26/10 9 h 34**, **turbo shot** (11 g, panier 58 mm 15 g, cible 45 g, 6 bar), écran 0.4.24 | **10,05 s**, 28,7 ml, 0,77 bar | 1,92 s, 2,6 ml, goutte | 32,2 ml | jamais (2,17 bar max) | **10,9 s depuis la goutte, 5,05 g/s**, 3,44 ml/s | 48,5 g | **89,37 °C** |
| **26/10 13 h 32**, turbo shot, cible 40 g, **consigne 88 °C** | 7,90 s, 25,2 ml, 0,89 bar | 0,79 s, 1,2 ml, goutte | 26,4 ml | jamais (1,98 bar max) | **9,6 s depuis la goutte, 4,93 g/s**, 3,51 ml/s | 44,2 g | **88,74 °C** |
| **26/10 15 h 35**, turbo shot, **15 g**, cible 45 g | 6,28 s, 19,7 ml, 0,97 bar | 4,08 s, 5,5 ml, échéance | 28,4 ml | jamais (4,91 bar max) | **9,9 s depuis la goutte, 4,40 g/s**, 3,29 ml/s | 43,9 g | **89,37 °C** |
| **26/10 16 h 32**, turbo shot, 15 g, **mouture grosse**, channelling | 6,28 s, 18,3 ml, 0,87 bar | 2,43 s, 3,2 ml, goutte | 21,4 ml | jamais (2,53 bar max) | **10,6 s depuis la goutte, 4,72 g/s**, 3,53 ml/s | 44,0 g | **88,77 °C** |
| **27/10 7 h 44**, turbo shot, **pompe à 70 %**, **puck screen** | 7,98 s, 24,3 ml, 0,88 bar | 0,50 s, 1,2 ml, goutte | 26,1 ml | jamais (1,92 bar max) | **12,5 s depuis la goutte, 3,78 g/s**, 2,83 ml/s | 43,0 g | **88,20 °C** |
| **27/10 13 h 16**, turbo shot, **pompe et remplissage à 100 %**, **86 °C, précharge 2 s** | 5,10 s, 19,7 ml, 0,94 bar | **sautée** (seuil de poids franchi pendant le remplissage) | 21,4 ml | jamais (2,54 bar max) | **9,6 s depuis la goutte, 4,60 g/s**, 3,61 ml/s | 43,7 g | **82,52 °C** |
| **27/10 13 h 49**, turbo shot, **mouture espresso**, 86 °C, précharge 2 s | 6,10 s, 23,2 ml, 0,95 bar | 4,12 s, 5,8 ml, échéance | 30,4 ml | jamais (5,61 bar max, PI dès 4 bar) | **10,6 s depuis la goutte, 4,03 g/s**, 3,08 ml/s | 42,9 g | **83,16 °C** |
| **28/10 10 h 09**, single espresso, **porte-filtre à bec**, 1re goutte à 8,98 bar | **5,42 s, 17,1 ml**, 0,81 bar | 4,11 s, 5,8 ml, échéance | **33,0 ml** | 3,4 s | 18,3 s, 1,16 g/s, 1,38 ml/s | 23,1 g | 92,04 °C |
| **28/10 13 h 47**, single espresso, bec, **fin du sachet**, 1re goutte à 2,65 bar | **10,01 s, 26,7 ml**, 0,83 bar, **2,69 ml/s** | 4,03 s, 5,2 ml, échéance | 36,5 ml | 3,9 s | **11,3 s, 1,57 g/s**, 1,52 ml/s | 23,8 g | **90,12 °C** |
| **29/10 9 h 19**, single espresso, bec, **dernière dose du Cozy Chocolate**, longue chauffe du porte-filtre | 7,29 s, 22,9 ml, 0,85 bar, 3,14 ml/s | **sautée** (balance de −0,1 à 0,0 g pendant le remplissage) | 34,8 ml | 4,4 s | 14,5 s, 1,40 g/s, 1,28 ml/s | 23,2 g | 90,58 °C |
| **29/10 14 h 07**, single espresso, **ViCafé**, **sans fond**, mouture trop fine, pic à 9,46 bar | 7,23 s, 22,6 ml, 0,85 bar, 3,13 ml/s | 4,10 s, 5,8 ml, échéance | 35,9 ml | 3,3 s | **20,1 s, 1,03 g/s**, 1,24 ml/s | 22,6 g | 90,50 °C |
| **29/10 14 h 21**, ViCafé, sans fond, **mouture ouverte**, **consigne 90 °C** | 6,74 s, 21,2 ml, 0,93 bar, 3,14 ml/s | 4,09 s, 5,2 ml, échéance | 31,6 ml | 3,6 s | 14,4 s, 1,34 g/s, 1,35 ml/s | 23,1 g | **91,30 °C** |
| **30/10 7 h 55**, ViCafé, sans fond, **7,5 g** (profil à 8,0 g, cible 22 g), 1re goutte à 1,16 bar | 7,36 s, 22,9 ml, 0,84 bar, 3,11 ml/s | 2,42 s, 3,8 ml, goutte | 27,8 ml | 4,9 s | **8,2 s, 1,77 g/s**, 1,76 ml/s | 22,9 g | 90,34 °C |
| **30/10 14 h 44**, ViCafé, sans fond, 7,5 g, **un cran plus fin** (cible restée à 22 g), 1re goutte à 2,22 bar | 6,70 s, 20,3 ml, 0,83 bar, 3,03 ml/s | 4,06 s, 5,2 ml, échéance | 29,3 ml | 4,0 s | 12,1 s, 1,47 g/s, 1,42 ml/s | 22,8 g | 89,93 °C |
| **31/10 10 h 16**, mêmes réglages, **circuit à 0,83 bar au départ**, rinçage à 9,04 bar, 1re goutte à 1,05 bar | 5,17 s, 15,7 ml, 0,81 bar, 3,03 ml/s | 2,12 s, 3,2 ml, goutte | 19,1 ml | 5,2 s | **8,5 s, 1,70 g/s**, 1,61 ml/s | 22,8 g | **92,47 °C** |
| **31/10 14 h 06**, mêmes réglages, **anticipation d'arrêt de 1,5 g** (écran 0.4.25), rinçage à 0,26 bar, 1re goutte à 1,26 bar | 6,69 s, 19,7 ml, 0,88 bar, 2,94 ml/s | 3,63 s, 5,2 ml, goutte | 25,2 ml | 4,6 s | **8,4 s, 1,76 g/s**, 1,79 ml/s | 22,5 g | 91,48 °C |
| **31/10 15 h 10**, **un cran plus fin**, 79 s sur le groupe, brûlé/amer, 1re goutte à 1,48 bar | 6,73 s, 20,6 ml, 0,88 bar, 3,06 ml/s | 3,61 s, 4,6 ml, goutte | 26,7 ml | 4,2 s | 10,3 s, 1,58 g/s, 1,66 ml/s | 21,9 g | 90,54 °C |
| **01/11 9 h 07**, mêmes réglages, **matin**, écran 0.4.26, 1re goutte à 1,21 bar, pic à 9,37 bar | 7,93 s, 24,6 ml, 0,90 bar, 3,11 ml/s | 2,34 s, 3,5 ml, goutte | 29,3 ml | 4,3 s | **13,0 s, 1,26 g/s**, 1,53 ml/s | 22,2 g | 90,51 °C |
| **01/11 13 h 31**, mêmes réglages, **cible 20 g**, après-midi, rinçage à 1,12 bar, 1re goutte à 1,67 bar | 9,00 s, 27,8 ml, 0,91 bar, 3,09 ml/s | 3,56 s, 4,9 ml, goutte | 34,2 ml | 3,7 s | **10,6 s, 1,41 g/s**, 1,48 ml/s | 20,0 g | 90,38 °C |
| **02/11 9 h 08**, **dose pesée (7,4 g)**, circuit à 0,64 bar au départ, 1re goutte à 1,14 bar | 5,83 s, 18,0 ml, 0,85 bar, 3,08 ml/s | 2,06 s, 2,9 ml, goutte | 21,7 ml | 4,4 s | 9,8 s, 1,35 g/s, 1,48 ml/s | 19,6 g | 90,48 °C |
| **04/11 9 h 44**, 7,4 g pesés, matin purgé, pic à 9,41 bar, 6,9 bar·s/g | 7,17 s, 22,3 ml, 0,85 bar, 3,11 ml/s | 4,04 s, 5,5 ml, échéance | 31,3 ml | 3,7 s | 12,9 s, 1,30 g/s, 1,49 ml/s | 19,6 g | 91,07 °C |
| **04/11 13 h 48**, **anticipation d'arrêt de 1,0 g**, 7,5 bar·s/g | 8,70 s, 26,4 ml, 0,94 bar, 3,03 ml/s | 4,01 s, 5,2 ml, échéance | 32,5 ml | 3,7 s | 13,0 s, 1,24 g/s, 1,47 ml/s | 20,1 g | 90,82 °C |
| **05/11 8 h 54**, matin sans purge, 7,7 bar·s/g, amer | 7,31 s, 22,3 ml, 0,91 bar, 3,05 ml/s | 1,75 s, 2,6 ml, goutte | 26,1 ml | 4,1 s | 13,5 s, 1,20 g/s, 1,26 ml/s | 20,4 g | 90,81 °C |
| **05/11 9 h 11**, **un cran plus gros**, 5,0 bar·s/g, plus fade | 7,88 s, 24,6 ml, 0,80 bar, 3,13 ml/s | 4,02 s, 5,5 ml, échéance | 32,2 ml | 4,1 s | 8,9 s, 1,76 g/s, 1,80 ml/s | 20,7 g | 90,26 °C |
| 05/11 14 h 31, même cran, 6,8 bar·s/g, 87,9 s après la consigne | 5,71 s, 17,4 ml, 0,90 bar, 3,05 ml/s | 2,86 s, 4,3 ml, goutte | 21,7 ml | 4,3 s | 11,2 s, 1,35 g/s, 1,45 ml/s | 20,4 g | 90,41 °C |
| 06/11 7 h 39, même cran, matin sans purge, 7,6 bar·s/g, amer | **10,02 s**, 29,6 ml, **0,68 bar**, 2,95 ml/s | 4,07 s, 5,5 ml, échéance | **38,3 ml** | 3,4 s | 14,3 s, 1,18 g/s, 1,44 ml/s | 20,0 g | 90,55 °C |
| 06/11 13 h 19, même cran, **4,7 bar·s/g**, rinçage à 8,31 bar | 7,65 s, 22,0 ml, 0,84 bar, 2,88 ml/s | 2,94 s, 3,8 ml, goutte | 26,1 ml | 4,9 s | **6,9 s, 1,87 g/s**, 1,73 ml/s | 20,9 g | 90,19 °C |

## Pistes pour le goût

Recette : grain ViCafé (100 % arabica), 8,0 g en panier, jusqu'au
20 octobre ; depuis le 23 octobre, « Cozy Chocolate » de Coffee Coaching
Club (100 % arabica du Brésil, nature, torréfié le 8 octobre), 8,0 g,
jusqu'au 29 octobre au matin ; depuis le 29 octobre à 14 h 07, retour au
ViCafé (sachet du 28 octobre), 8,0 g, porte-filtre sans fond. Routine de
David avant chaque infusion : purge de 5 à 6 s sans porte-filtre dans la
tasse (rinçage du circuit, tasse chauffée), puis aussitôt une seconde
purge de 5 à 6 s avec le porte-filtre, dans la même tasse ; attente que
la NTC revienne à la consigne, porte-filtre en place avec l'eau restée
dedans ; porte-filtre vidé et séché, galette préparée, tasse vidée et
rincée, infusion. La durée de l'attente à la consigne varie ; depuis le
14 octobre, le mode guidé de l'écran suit cette routine et la capture
enregistre ses durées. Depuis le 15 octobre, la purge sans porte-filtre
se fait au bouton purge, avant d'ouvrir le guide (elle giclait autour de
la tasse et mouillait la balance) ; le guide ne mène plus que le rinçage
du porte-filtre, de 5,0 s. Les pistes
ci-dessous attendent que la chauffe soit stabilisée ; on n'en change
qu'une à la fois. Les repères « avant » viennent du journal de dégustation
tenu avant la modification de la machine (juillet–août 2026, local, hors
Git).

| Piste | Priorité | Lecture |
| --- | --- | --- |
| Température | **première** | Les meilleurs cafés d'avant étaient à « 93 °C » affichés par la machine. Avec l'offset de −14 °C (10 octobre), la consigne désigne l'eau au panier : les infusions jusqu'au 9 octobre, 87,7 à 89,3 °C en tasse à l'ancien offset, avaient environ 84 à 86 °C au panier, 4 à 6 °C sous la consigne. Le goût un peu sous-extrait de ces infusions va dans ce sens. Reprendre à 90 °C avec 0.4.4. Le 13 octobre, 92 °C ne donne que +1,0 °C en NTC tasse (besoin de l'eau calculé pour 90 °C), avec un peu d'amertume. L'après-midi, à 22 g, la NTC tasse arrive au critère (91,96 °C, cause non identifiée) et le goût est impeccable : la température n'était pas la cause de l'amertume |
| Pression | basse | Peu de marge attendue, mais une pression plus basse (≈ 8,5 bar) vaut un essai. Avant, 9 → 8 bar à débit égal avait donné de la rondeur, mais sur un grain avec 30 % de robusta ; sur le ViCafé, tous les bons cafés étaient à 9 bar. Les « 9 bar » d'avant venaient de la machine, ceux d'aujourd'hui du XDB401 (plateau à 8,87 bar). Essai à débit égal : baisser la pression ralentit le débit, ouvrir la mouture en même temps, sinon l'amertume du ralentissement se lit comme un effet de la pression |
| Poids | basse | 22 g (ratio 2,75) est le point retenu pour 8 g. Avant : ratio 2,70 à 2,85 retenu, 24 g (ratio 3,00) amer à galette impeccable. Le filet blondit déjà à 22 g. Seul ajustement possible : jusqu'à ≈ 23 g si la mouture plus grosse laisse le filet encore crémeux à 22 g, une galette rapide s'épuisant plus tard. 23 g essayé du 11 au 13 octobre ; à 92 °C, filet fin et blond à la fin : retour à 22 g, impeccable le 13 octobre à 13 h 29 |

### Repère : recette du torréfacteur pour le mélange ViCafé

Recette donnée par le torréfacteur le 28 octobre, avec un nouveau sachet
du « mélange de la maison ViCafé » : **18 g en panier de 58 mm, 40 g en
tasse, 24 à 26 s depuis le démarrage de la pompe**. Machine La Marzocco
professionnelle, pompe rotative, très probablement à 9 bar ; porte-filtre
à bec. Première goutte à **5 s** sur le chronomètre de l'écran de la
machine, relevée par David ; sous le panier, elle tombe un peu plus tôt.
La tasse n'est pas pesée pendant l'infusion : l'arrêt est probablement
volumétrique, réglé pour viser 40 g, et la mouture se règle sur la durée
affichée à l'arrêt. Les 40 g sont donc un poids visé, pas mesuré ; une
dose volumétrique de 40 ml d'eau admise ne donnerait que 10 à 15 g en
tasse (28 à 36 ml admis avant la goutte avec 8 g dans le LM1). Prochaine
visite : relever la durée affichée à l'arrêt.

| Grandeur | ViCafé, 58 mm | Ramené au panier LM1 41 mm, 8 g | 28/10 10 h 09 (Cozy Chocolate) |
| --- | ---: | ---: | ---: |
| Ratio | 1:2,2 | 17,6 g en tasse | 1:2,9 (23,1 g) |
| Durée depuis la goutte | 19 à 21 s | ≈ 16 s pour 17,6 g | 18,3 s depuis 8 bar (goutte à 8,98 bar) |
| Débit moyen en tasse | **1,9 à 2,1 g/s** | **1,07 à 1,18 g/s** | 1,16 g/s |
| Résistance à 9 bar | ≈ 4,5 bar·s/g | **≈ 8,0 bar·s/g** (7,6 à 8,4) | 7,4 bar·s/g |

- Mise à l'échelle : à mouture égale, le débit suit la surface divisée par
  l'épaisseur de la galette (dose divisée par la surface). Le panier de
  41 mm a la moitié de la surface (1320 mm² contre 2642 mm²) et une
  galette 11 % plus mince (6,1 contre 6,8 mg/mm²) : 0,56 fois le débit
  du panier de 58 mm. Les diamètres sont nominaux ; le fond des paniers
  n'est pas mesuré.
- Le débit est la moyenne de la goutte à l'arrêt, montée comprise ; une
  seconde d'écart sur la goutte le change de 5 %.
- La galette du 28 octobre, avec le Cozy Chocolate, est déjà au débit
  visé par le torréfacteur ; les galettes sans fond du ViCafé, de 6,6 à
  7,2 bar·s/g les 16 et 17 octobre, étaient un peu plus rapides.
- L'écart est dans le ratio : 1:2,2 chez le torréfacteur, 1:2,75 retenu
  ici (voir la piste « Poids »). C'est un arrêt plus tôt au même débit
  équivalent, dans le sens contraire de l'hypothèse du 20 octobre (à débit
  faible, l'arrêt à 22 g ne garde que la partie acide de l'extraction).

## Chemin de recherche

Le journal garde en entrées détaillées les infusions des **7 derniers
jours**. Les plus anciennes sont résumées ici, par étape. Leur détail
(tables, attendus, lectures) reste dans l'historique Git :
`git show 7fdee50:docs/journal-infusions.md` pour les entrées jusqu'au
18 octobre, `git show 02dc5fc:docs/journal-infusions.md` pour celle du
20 octobre, `git show 7c17133:docs/journal-infusions.md` pour celles du
23 octobre au 3 novembre, `git show 8dc00d4:docs/journal-infusions.md`
pour celles du 4 au 6 novembre. Les chiffres de chaque infusion restent dans la
table de synthèse ; ceux des turbo shots, dans [turboshot.md](turboshot.md).

### Du 5 au 6 octobre — fin de remplissage et pré-infusion

Firmware 0.3.40, précharge de 8 s, espresso de 8 g en LM1, cible de 22 g,
consigne de 90 °C (offset de −10,5 °C).

- La fin de remplissage passe à **0,8 bar** et la pré-infusion à
  **50 %** : à 35 %, avec un seuil à 0,7 bar, la pause ne comblait pas
  les ≈ 4,9 ml de marge, et la pré-infusion finissait à l'échéance. À
  50 %, la galette est mouillée jusqu'à la goutte, sans pause sans débit.
- Remplissage gardé à 75 % : plus lent, il dégradait la galette.
- Chauffe : la coupure de fin de chauffe est supprimée (0.3.39) ; le
  remboursement de la précharge tient la commande à 0 % ≈ 10 à 16 s.
  Les tasses froides (−1 à −2 °C sous la consigne) ne viennent pas d'un
  manque d'énergie mais de son moment.

### Du 6 au 9 octobre — mouture pas à pas, sonde XDB401

- La **résistance au plateau** (bar·s/g) devient la mesure de la
  mouture : 8,3 → 7,5 → 7,1 bar·s/g sur trois pas de mouture vers le plus
  gros. Galette trop perméable le 6 octobre (débit élevé, amertume),
  trop fine le 8 au matin, juste le 8 à 13 h 44.
- Pression lue sur le XDB401 analogique depuis le 8 octobre (écran
  0.3.46, [xdb401_swap.md](xdb401_swap.md)), plateau régulé à ≈ 8,9 bar.
- Le modèle thermique se trompe de 1 à 2 °C, surtout sur les infusions
  courtes à long 0 % ; il sous-estime les minima. D'où la règle de ne lire
  que des écarts entre variantes.

### 10 octobre — offset de −14 °C

La sonde SCACE au panier ([sonde-scace.md](sonde-scace.md#infusions-10-octobre-2026))
montre que le panier était ≈ 3,5 °C plus froid que ne le disait la NTC.
Offset porté à
**−14 °C** (écran 0.4.4), consigne bornée à 95 °C, coupure à 100 °C
utilisateur. Les NTC tasse d'avant se comparent en leur retirant 3,5 °C.

### Du 10 au 12 octobre — recette à 90 °C

- Goût bon dès l'offset ; une faible amertume reste. Cible à **23 g** le
  11 octobre : l'amertume baisse, mais la résistance a changé en même
  temps.
- Le **tassage** fait varier la résistance de ± 18 % à mouture égale
  (5,9 à 8,5 bar·s/g). Accepté : goût excellent de 7,0 à 8,5 bar·s/g et
  de 88,0 à 88,8 °C en NTC tasse.
- Repère de David : la meilleure pré-infusion finit quand la goutte
  apparaît sous le panier (12 octobre, galette parfaite).
- La SCACE du 12 octobre montre que le creux de la NTC atteint à peine
  l'eau au-dessus de la galette : un écart de NTC tasse de moins de 1 °C
  ne se transpose pas tel quel au panier.

### 13 octobre — consigne de 92 °C, retour à 22 g

- +2 °C de consigne ne donnent que **+0,96 °C** en NTC tasse : la
  précharge et le remboursement sont fixés en énergie, et l'appoint
  suppose un écart d'eau froide fixe de 70,5 K.
- Un peu d'amertume à 23 g et 92 °C, filet blond en fin. Retour à
  **22 g** : impeccable l'après-midi à 91,96 °C, donc la température
  n'était pas la cause de l'amertume. ≈ 1,6 °C de cette tasse restent
  hors modèle (porte-filtre resté longtemps sur le groupe, hypothèse).

### Du 14 au 15 octobre — mode guidé

- Mode guidé (écran 0.4.10), puis guide à un seul rinçage (0.4.12) ; la
  capture enregistre la routine.
- Piste du **temps minimal sur le groupe** : au-delà de ≈ 2,5 min, l'écart
  au modèle ne bouge plus ; elle perd son appui
  ([wizard.md](wizard.md#piste--temps-minimal-sur-le-groupe)).
- **Trou de chauffe** : le SSR coupé ≈ 2 s au début du remplissage sur
  presque toutes les infusions, ≈ 1 °C de variation en tasse en
  simulation. Corrigé en 0.4.13.
- Goût : le plus équilibré le 15 à 9 h 08 (90,63 °C). Hypothèse non
  testée : le goût suit le volume de pré-infusion (4,9 ml légèrement
  acide, 7 à 8 ml très bon).
- Conclusion de David le 15 : la différence entre deux tasses se fait
  maintenant au moulin et à la préparation ; le montage a atteint son
  objectif.

### Du 16 au 18 octobre — écrans 0.4.13 et 0.4.14

- 0.4.13 : une lecture NTC manquante remettait la loi à zéro et effaçait
  la dette ; ≈ 7 kJ de trop, **97,2 °C** après l'infusion du 16 au matin.
  0.4.14 coupe la chauffe sur une lecture manquante d'au plus 1 s et
  garde la dette.
- Le gain de +1 °C attendu de 0.4.13 ne se voit pas en tasse (une seule
  capture) ; goût impeccable à 90,77 °C. La rétention du moulin (café du
  matin plus amer) devient une variable hors machine.
- Écarts au modèle de −0,5 à +0,7 °C selon la routine, non expliqués ;
  acidité sur les deux routines les plus courtes sur le groupe (hypothèse).
- Porte-filtre à bec le 18 : galette plus rapide (8,4 s depuis 8 bar,
  tassage moins fort sur le support troué), acidité ; le bec rend 2 g
  après l'arrêt.

### 20 octobre — mouture affinée, retour au sans fond

- Mouture plus fine et retour au porte-filtre sans fond le même jour :
  9,0 bar·s/g, 18,8 s depuis 8 bar, dépassement à 9,61 bar au passage de
  8 bar. Rien ne sépare les deux effets ; la mouture suffit à les
  expliquer. Le sans fond retire les 2,0 g de gouttes du bec (0,8 g).
- Remplissage court (21,4 ml) : moins d'eau froide sous la précharge,
  NTC +2,25 °C pendant le remplissage, tasse à 92,57 °C, rebond jusqu'à
  94,1 °C après la capture. Le modèle ne suit pas cette hausse
  (+0,85 °C au-dessus).
- Beaucoup plus acide, pas désagréable. Hypothèse de David, non testée :
  à débit faible, l'arrêt à 22 g ne garde que la première partie de
  l'extraction, acide. Mouture rendue moins fine ensuite.

### Ce qui reste établi au 18 octobre

| Sujet | Acquis |
| --- | --- |
| Hydraulique | remplissage fini à 0,8 bar ; pré-infusion à 50 % finie sur la goutte ou à 4 s ; volume de remplissage variable (17 à 28 ml) sans cause identifiée |
| Mouture | se lit dans la résistance au plateau ; le tassage la fait varier de ± 18 % |
| Chauffe | la consigne ne passe qu'à moitié en NTC tasse ; remboursement de 9 à 12 s à 0 % ; le modèle se trompe de 1 à 2 °C en absolu et sous-estime les minima |
| Goût | la chauffe n'est plus la source principale des écarts ; moulin, préparation et temps d'extraction le sont |

### Du 23 au 25 octobre — Cozy Chocolate, carte sensors

- Première infusion sur la carte `pcb/sensors` le 23 et nouveau café
  (« Cozy Chocolate », arabica nature du Brésil) : 12,5 bar·s/g,
  0,70 g/s, acide. Débitmètre et sonde lus plus bas ce jour-là
  (estimé), sans cause établie ; le défaut ne revient pas.
- Mouture ouverte deux fois : 10,9 puis 6,8 bar·s/g. Sur ce café, la
  goutte attend la pression (5,6 à 9,1 bar), même à résistance égale au
  ViCafé : une galette qui résiste au mouillage (dégazage ou nature, non
  séparés).
- Le 24, consigne de 91 °C, remplissage à 70 % et purge à 80 % ensemble :
  goût bon deux fois, 6,5 à 6,8 bar·s/g. Le remplissage lent ne fait pas
  perdre de résistance ; le volume admis avant 0,5 bar varie de 15,7 à
  23,8 ml sans changement de réglage.
- Le 25, écran 0.4.23 (profils) et tassage plus ferme : 7,8 bar·s/g
  (+20 %), amertume ; la galette dégradée à l'œil ne se lit pas dans la
  capture.

### Du 26 au 27 octobre, puis du 2 au 3 novembre — turbo shots

Résumé et détail dans [turboshot.md](turboshot.md). À retenir pour
l'espresso : dans le panier 58 mm, la mouture espresso ne laisse monter la
pression qu'à 2 bar avec 11 g, 4,9 bar avec 15 g ; la mouture grosse érode
la galette (channelling) ; à hydraulique proche, la mouture espresso
plaisait plus à 89,4 °C en tasse (26 octobre) qu'à 83,2 °C avec 86 °C de
consigne (27 octobre). Le 3 novembre,
retour au single espresso, turbo mis en pause.

### Du 28 au 29 octobre — porte-filtre à bec, fin du Cozy Chocolate

- Avec le bec, rien n'arrive en tasse avant 8 bar et la pré-infusion
  finit à l'échéance : la balance ne voit plus la goutte.
- 7,4, 5,5 puis 6,0 bar·s/g en trois infusions au même réglage ; goût
  « vraiment bien » à « excellent ».
- **La tasse suit le volume de remplissage** (mesuré) : 17,1 ml, NTC
  +2,30 °C au remplissage ; 22,9 ml, +1,06 °C ; 26,7 ml, +0,13 °C ; moins
  d'eau froide entre sous la précharge quand le remplissage est court.
- Le 28 à 13 h 47, rinçage et remplissage plus faibles (pompe ou
  débitmètre, non tranché), revenus à la normale le 29. Le 29, la
  pré-infusion est sautée quand la balance repasse de −0,1 à 0,0 g.

### Du 29 au 31 octobre — ViCafé sans fond, 7,5 g, mouture pas à pas

- Retour au ViCafé sans fond : 8,7 bar·s/g, trop fin ; deux crans plus
  ouverts et 90 °C : 6,3 bar·s/g, « très bien » (≈ 15 % par cran).
- **7,5 g** au lieu de 8,0 g : 4,9 bar·s/g (−22 % pour 6 % de dose),
  léger et un peu acide ; un cran plus fin : 6,0, « très bonne », cible
  restée à 22 g (ratio 3,04).
- Le 31, galettes plus rapides sans rien changer (5,2 puis 5,0 bar·s/g),
  rinçage à 9,04 bar le matin ; douchette nettoyée au porte-filtre
  aveugle, sans effet sur la résistance. Écran 0.4.25 et anticipation
  d'arrêt de 1,5 g. Un cran plus fin : 5,6 bar·s/g, « brûlé/amer » à
  1,58 g/s.

### Du 1er au 3 novembre — cible de 20 g, dose pesée

- Le 1er au matin, 6,9 bar·s/g au même cran que la veille (5,6) : le
  moulin semble demander deux infusions pour se stabiliser (estimé).
  Bon, un peu amer, ratio 2,96.
- **Cible de 20 g** l'après-midi : 6,1 bar·s/g, ratio 2,67, dans la plage
  des cafés notés 2 avant la modification (2,63 à 2,86) ; David est
  satisfait de l'arrêt à 20 g.
- Le 2, dose pesée (7,4 g pour 7,5 g réglés) et moulin calibré : 6,4
  bar·s/g ramené à 7,5 g, bon, petite amertume au centre de la langue.
  Un circuit déjà à 0,64 bar au départ raccourcit le remplissage
  (18,0 ml).

### Ce qui reste établi au 3 novembre

| Sujet | Acquis |
| --- | --- |
| Mouture | ≈ 12 à 23 % de résistance par cran ; le moulin peut demander deux infusions après un changement de cran |
| Dose | 7,5 g en LM1, cible 20 g (ratio ≈ 2,7) ; dose pesée à chaque infusion |
| Porte-filtre | sans fond retenu ; le bec retient le filet de basse pression et rend plus de gouttes |
| Chauffe | la tasse suit le volume de remplissage, de 17 à 27 ml ; à 90 °C de consigne, ≈ 90 à 92,5 °C en tasse |
| Rinçage | au-dessus de 0,7 bar par moments (1,12 et 9,04 bar), sans cause établie |

### Du 4 au 6 novembre — anticipation de 1,0 g, variabilité du moulin

- Le 4 au matin, l'anticipation d'arrêt de 1,5 g laissait ≈ 0,4 g sous
  la cible : **1,0 g** retenu (pas de 0,5 g dans le firmware). Elle laisse
  1,0 g de gouttes vers 1,2 g/s, jusqu'à 1,7 g vers 1,8 g/s. Premier matin
  avec purge du canal du moulin, sans amertume.
- Le 5 au matin, sans purge, 7,7 bar·s/g et amer : **un cran plus gros**
  donne 4,9 (−36 %), « plus fade », puis 6,8 au même cran l'après-midi
  (+39 %), « bon, peut-être un peu trop rapide ». Ce jour-là, le moulin a
  donné 7,2 g pour 7,5 g : dose pesée et complétée. Porte-filtre 87,9 s
  sur le groupe après la consigne : −2,25 °C pompe en marche.
- Le 6, au même cran, 7,6 le matin (remplissage fini sur l'échéance à
  0,68 bar, goutte à 38,3 ml, amer, sans purge) puis 4,7 l'après-midi
  sans rien changer. **À cran fixe, ce moulin donne 4,7 à 7,6 bar·s/g
  (+62 %)**, au-delà de la dispersion de préparation (± 18 %) ; ni l'âge
  de la mouture retenue ni l'humidité ne suivent l'écart. Variabilité
  acceptée (David), moulin single dose envisagé. Rinçage à 8,31 bar
  l'après-midi, sans cause établie.

### Ce qui reste établi au 6 novembre

| Sujet | Acquis |
| --- | --- |
| Mouture | à cran fixe, 4,7 à 7,6 bar·s/g : une infusion seule ne juge plus un cran ; un cran vaut ≈ 12 à 23 %, dans la dispersion |
| Arrêt | anticipation de 1,0 g : 1,0 g de gouttes vers 1,2 g/s, 1,7 g vers 1,8 g/s |
| Chauffe | porte-filtre posé peu après la consigne : baisse pompe en marche sous la plage |
| Amertume | sur les matins sans purge (1/11, 2/11, 5/11, 6/11), absente le matin purgé ; plus suivie depuis le 11/11 |


## Entrées

### 10 novembre, 13 h 42 — `261110-134225`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, profil
inchangé : **7,5 g**, cible 20 g, anticipation d'arrêt de 1,0 g, consigne
de 90 °C. **Cran : un pas plus serré que le 6/11** : les infusions
suivantes, non capturées, coulaient vers 1,6 g/s, trop vite au goût de
David. Une infusion ce matin, bonne et non capturée : la mouture retenue
dans le canal date de ce matin. Non notés : niveau de la trémie, Netatmo. Écran 0.4.27, sensors 0.4.24 ; boîtier de
l'écran à 33,2 °C et 38,9 % (point de rosée 17,4 °C). Observation de
David : **résultat très satisfaisant**.

**Routine** : guide ouvert à 75,2 °C ; rinçage de 5,0 s, 17,7 ml, 32,5 g,
**pression maximale 0,70 bar** (8,31 bar le 6/11 à 13 h 19). Retour à la
consigne en 123,9 s, minimum 82,6 °C. **710 s sur le groupe** ;
préparation de 106,6 s. Tasse de 125,4 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,04 s | — | 0,27 → 0,36 bar |
| Remplissage | 8,04 s | 7,65 s, pression | 22,6 ml (2,95 ml/s) | 0,36 → 0,87 bar |
| Pré-infusion | 15,69 s | **1,92 s, poids** | 2,9 ml | 0,88 → 1,09 bar |
| Infusion | 17,61 s | 15,47 s | 27,5 ml | 9,06 bar max |

- **Première goutte (0,3 g) à 18,29 s, à 1,20 bar**, **27,0 ml**, 0,7 s
  après le début de l'infusion.
- 8 bar à 21,73 s (4,1 s après le début de l'infusion), 4,0 g en tasse.
  Pas de pic transitoire ; la pression monte lentement de 8,5 à
  **9,06 bar** juste avant l'arrêt, pompe stable à 67–70 %.
- Depuis 8 bar : **11,3 s**, **1,34 g/s** en tasse, 1,43 ml/s en amont.
  **19,2 g à l'arrêt de la pompe**, **20,7 g final**, **1,5 g de
  gouttes**. Goutte → arrêt de la pompe : **14,8 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,82 ± 0,09 bar**, pompe à
  67–70 %, 1,36 g/s, soit **6,5 bar·s/g** à 7,5 g.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 10/11 13 h 42 | 2,1 s | 4,8 s | 8,7 s | 14,7 s |
  | 06/11 13 h 19 | 1,8 s | 4,6 s | 6,9 s | 11,2 s |
  | 06/11 7 h 39 | 2,6 s | 6,0 s | 10,6 s | 17,0 s |
  | 05/11 14 h 31 | 1,8 s | 4,8 s | 8,5 s | 14,5 s |

**Thermique.** NTC de 90,07 °C au départ, **+1,46 °C** pendant le
remplissage (22,6 ml), −0,22 °C pendant la pré-infusion ; 91,48 °C au
début de l'infusion, 89,00 °C à l'arrêt. Baisse pompe en marche
**−1,07 °C** (89,0 à 91,7 °C). **Tasse : 90,60 °C, +0,60 °C.**

**Attendus vérifiés** (entrée du 6/11 à 13 h 19, prévue pour un matin
avec purge ; celle-ci est une infusion de l'après-midi, un pas plus
serré).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée avant complément ; masse purgée ; niveau de la trémie | 7,5 g ; ratio **2,76** ; canal renouvelé par l'infusion du matin ; trémie non notée |
| Hydraulique | 4,7 à 7,6 bar·s/g (≈ 7 à 14 s depuis 8 bar, ≈ 1,2 à 1,9 g/s) ; plateau sous 9,2 bar | **6,5 bar·s/g**, 11,3 s, 1,34 g/s : atteint ; plateau 8,82 bar, maximum 9,06 bar : atteint |
| Rinçage | 0,1 à 0,7 bar | 0,70 bar : atteint, en limite haute |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 30 ml ; noter la fin | 2,95 ml/s ; 22,6 ml ; fin sur la pression : atteint |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,0 g final | 19,2 g ; 20,7 g : atteint |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | +1,46 °C ; −1,07 °C : atteint |
| Tasse | 89,0 à 92,5 °C ; amertume notée | 90,60 °C : atteint ; « très satisfaisant », amertume non notée |

**Lecture.**

- *Résistance, mesuré.* 6,5 bar·s/g au cran resserré d'un pas, dans la
  plage de 4,7 à 7,6 bar·s/g relevée au cran précédent : une seule
  infusion à ce cran ne dit pas encore de combien il déplace la plage,
  la dispersion du moulin à cran fixe (+62 %) dépassant l'effet d'un
  pas.
- *Rinçage, mesuré.* 0,70 bar contre 8,31 bar le 6/11 : l'anomalie du
  6/11 ne se reproduit pas, sa cause reste non établie.
- *Pré-infusion, mesuré.* Close sur le poids en 1,92 s (2,94 s le 6/11),
  la goutte arrivant à 27,0 ml, dans la fourchette des entrées
  précédentes (21,7 à 38,3 ml).
- *Plateau, observé.* À pompe constante, la pression monte de 8,5 à
  9,06 bar pendant que le débit en tasse reste vers 1,3 à 1,4 g/s : la
  galette se resserre légèrement en fin d'infusion.
- *Goût, observé.* Très satisfaisant à 6,5 bar·s/g et 1,34 g/s depuis
  8 bar, plus lent que les infusions vers 1,6 g/s jugées trop rapides.

**Décision.** Aucune ; cran resserré conservé, profil inchangé.

### 11 novembre, 8 h 25 — `261111-082554`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, profil
inchangé : **7,5 g**, cible 20 g, anticipation d'arrêt de 1,0 g, consigne
de 90 °C, cran resserré du 10/11. **Première infusion du matin, machine
froide** : la chauffe, coupée la veille pour une mise à jour du firmware,
n'a été remise que ce matin (NTC à 11,3 °C à l'ouverture du guide).
**Canal du moulin non purgé**, **trémie presque vide** ; porte-filtre
propre, rinçage sans problème selon David. Écran 0.4.28,
qui affiche la résistance au plateau : **6,7 bar·s/g à l'écran**, comme
le script. Salon (Netatmo) : 52 %, point de rosée **12,5 °C** (11,0 °C le
6/11 après-midi) ; extérieur : 100 %, point de rosée 11,4 °C ; boîtier
de l'écran à 30,2 °C et 46,7 % (point de rosée 17,6 °C). Observation de
David : **goût tout à fait bon** ; avec la tasse plus froide, il en tire
que le meilleur café est un café assez froid.

**Routine** : guide ouvert à 11,3 °C ; rinçage de 5,0 s, 17,7 ml, 35,3 g,
**pression maximale 9,12 bar**. Retour à la consigne en 270,2 s, minimum
82,2 °C ; **porte-filtre en place 147,3 s après la consigne** ;
préparation de 117,3 s. Tasse de 115,1 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | 0,37 → 0,46 bar |
| Remplissage | 8,01 s | 7,47 s, pression | 22,0 ml (2,95 ml/s) | 0,46 → 0,89 bar |
| Pré-infusion | 15,47 s | **0,46 s, poids** | 0,9 ml | 0,89 → 0,93 bar |
| Infusion | 15,93 s | 14,23 s | 28,4 ml | 8,94 bar max |

- **Première goutte (0,3 g) à 16,30 s, à 0,95 bar**, **23,5 ml**, 0,4 s
  après le début de l'infusion.
- 8 bar à 20,96 s (5,0 s après le début de l'infusion), 6,2 g en tasse.
  Pas de pic transitoire : maximum de 8,94 bar à 29,42 s.
- Depuis 8 bar : **9,2 s**, **1,39 g/s** en tasse, 1,48 ml/s en amont.
  **19,0 g à l'arrêt de la pompe**, **20,2 g final**, **1,2 g de
  gouttes**. Pompe en marche 22,0 s. Goutte → arrêt de la pompe :
  **13,9 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,83 ± 0,07 bar**, pompe à
  67–70 %, 1,32 g/s, soit **6,7 bar·s/g** à 7,5 g.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 11/11 8 h 25 | 1,5 s | 4,4 s | 7,8 s | 13,6 s |
  | 10/11 13 h 42 | 2,1 s | 4,8 s | 8,7 s | 14,7 s |
  | 06/11 13 h 19 | 1,8 s | 4,6 s | 6,9 s | 11,2 s |
  | 06/11 7 h 39 | 2,6 s | 6,0 s | 10,6 s | 17,0 s |

**Thermique.** NTC de 90,18 °C au départ, **+1,30 °C** pendant le
remplissage (22,0 ml), +0,01 °C pendant la pré-infusion ; 91,60 °C au
début de l'infusion, **85,29 °C à l'arrêt**. Baisse pompe en marche
**−4,89 °C** (85,3 à 91,6 °C). **Tasse : 88,08 °C, −1,92 °C.**

**Attendus vérifiés** (entrée du 6/11 à 13 h 19, prévue pour un matin
avec purge ; celle-ci est un matin sans purge, un pas plus serré, machine
partie froide).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée avant complément ; masse purgée ; niveau de la trémie | 7,5 g ; ratio **2,69** ; pas de purge ; trémie presque vide |
| Hydraulique | 4,7 à 7,6 bar·s/g (≈ 7 à 14 s depuis 8 bar, ≈ 1,2 à 1,9 g/s) ; plateau sous 9,2 bar | **6,7 bar·s/g**, 9,2 s, 1,39 g/s : atteint ; plateau 8,83 bar, maximum 8,94 bar : atteint |
| Rinçage | 0,1 à 0,7 bar ; au-dessus, noter l'état du porte-filtre | **9,12 bar, très au-dessus** ; 35,3 g pour 17,7 ml ; porte-filtre propre |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 30 ml ; noter la fin | 2,95 ml/s ; 22,0 ml ; fin sur la pression : atteint |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,0 g final | 19,0 g ; 20,2 g : atteint |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | +1,30 °C : atteint ; **−4,89 °C, très en dessous** |
| Tasse | 89,0 à 92,5 °C ; amertume notée | **88,08 °C, 0,9 °C sous la plage** ; « tout à fait bon », amertume non signalée |

**Lecture.**

- *Résistance, mesuré.* 6,7 bar·s/g, contre 6,5 hier après-midi au même
  cran (+3 %) : les deux infusions au cran resserré sont proches, mais
  deux valeurs ne font pas une plage devant la dispersion du moulin
  (+62 % au cran précédent).
- *Résistance à l'écran, mesuré.* La tuile de l'écran 0.4.28 affiche
  6,7 bar·s/g, la valeur du script (6,70) : la mesure est lisible à la
  machine sans capture.
- *Chauffe, mesuré.* Départ à froid, consigne atteinte 270 s après le
  rinçage, porte-filtre en place 147 s après la consigne : baisse pompe
  en marche de **−4,89 °C**, la plus forte de la série, et tasse à
  88,1 °C. Le 5/11 à 14 h 31, 87,9 s après la consigne mais après un
  minimum de 80,0 °C seulement, donnaient −2,25 °C : le départ à froid
  pèse plus que le temps après la consigne.
- *Chauffe, cause donnée par David.* La chauffe était coupée depuis la
  veille : chaudière, groupe et porte-filtre partaient de la température
  ambiante. Hypothèse : le groupe et le porte-filtre, pas encore en
  équilibre, prennent à l'eau plus de chaleur que la loi de chauffe,
  réglée sur une machine chaude, n'en apporte. Le remplissage (+1,30 °C)
  est normal : l'écart naît pompe en marche.
- *Rinçage, mesuré.* 9,12 bar, après 8,31 bar le 6/11 à 13 h 19 et
  0,70 bar hier, avec un porte-filtre propre et un rinçage sans problème
  à l'œil de David : le porte-filtre n'explique pas la pression lue, sa
  cause reste non établie. Les 35,3 g pour 17,7 ml ne la signent pas :
  hier, 32,5 g pour 17,7 ml à 0,70 bar.
- *Pré-infusion, mesuré.* Close sur le poids en 0,46 s, la goutte arrivant
  à 23,5 ml, dans la fourchette des entrées précédentes (21,7 à 38,3 ml).
- *Goût, observé.* « Tout à fait bon » sur une première infusion du
  matin sans purge, à 6,7 bar·s/g et une tasse plus froide de 2 °C :
  l'amertume des matins sans purge (1/11, 2/11, 5/11, 6/11) ne revient
  pas. Avec la trémie presque vide, David juge que la purge et la trémie
  ne comptent pas autant ; le journal ne les suit plus.
- *Tasse froide, hypothèse de David.* Le meilleur café serait un café
  assez froid. Les tasses « excellentes » du 10 au 12 octobre étaient à
  88,0 à 88,8 °C en NTC tasse, comme ce matin (88,1 °C) ; depuis le
  29 octobre, à 90 °C de consigne, elles sont à 89,9 à 92,5 °C. Une seule
  infusion froide, et la galette de ce matin était aussi dans la plage
  habituelle : rien ne sépare encore la température du reste.
- *Humidité, observé.* Point de rosée du salon à 12,5 °C, le plus haut
  noté (10,7 et 11,0 °C le 6/11), pour une résistance au milieu de la
  plage : l'humidité ne suit toujours pas la résistance.

**Décision** (David). Réglages inchangés pour l'instant : cran resserré,
7,5 g, cible 20 g, anticipation de 1,0 g, 90 °C. Purge du moulin et niveau
de la trémie ne sont plus suivis. **Essai de 89 °C de consigne envisagé**,
plus tard.

*Estimé.* Le 13 octobre, +2 °C de consigne ne donnaient que +0,96 °C en
NTC tasse, et le 24 octobre −1 °C donnait −0,71 °C : 89 °C devrait mettre
la tasse vers 89,3 à 89,5 °C sur une machine chaude, pas aux 88,1 °C de
ce matin. Pour retrouver cette tasse, il faudrait plutôt 88 °C, ou une
précharge plus courte (≈ 0,7 °C de moins en tasse par seconde retirée,
simulé sur un turbo shot le 27 octobre).

| Point | Attendu (prochaine infusion, machine chaude, mêmes réglages) |
| --- | --- |
| Dose | pesée ; ratio = final / dose pesée |
| Hydraulique | 4,7 à 7,6 bar·s/g (≈ 7 à 14 s depuis 8 bar, ≈ 1,2 à 1,9 g/s) ; plateau sous 9,2 bar |
| Rinçage | 0,1 à 0,7 bar |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 30 ml ; noter la fin |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,0 g final |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche |
| Tasse | 89,0 à 92,5 °C ; goût noté au regard de la température |

### 15 novembre, 21 h 41 — `261115-214106`

**Contexte.** **Première infusion avec la pompe à engrenage MPB003**,
remontée dans la machine le 14 novembre (voir
[pompe-engrenage.md](pompe-engrenage.md#dans-la-machine-contre-lopv-14-novembre-2026)).
Écran 0.5.4, sensors 0.5.3. **Retour au Cozy Chocolate**, probablement
**deux crans plus serré** que le cran ViCafé du 10/11 selon David ; il y
voit la cause de la galette résistante. Single espresso, porte-filtre
sans fond, profil inchangé : 7,5 g (pesée non précisée), cible 20 g,
anticipation d'arrêt de 1,0 g, consigne de 90 °C. Boîtier de l'écran à
28,9 °C et 45 %. **Café non bu** : David attend avant d'en boire et fera
d'abord une purge de 2 L ; pas de goût noté.

Deux changements à la fois, la pompe et le café avec sa mouture. La
résistance au plateau (pression divisée par le débit en tasse) ne dépend
pas de la pompe : elle appartient à la galette. Le remplissage, la
première goutte et la montée dépendent des deux : rien ne les sépare
dans cette infusion.

**Routine** : guide ouvert à 87,2 °C ; rinçage de 5,1 s, 21,7 ml, 26,9 g,
**pression maximale 0,96 bar**. **Retour à la consigne en 750,7 s**,
minimum 81,6 °C, chauffe allumée 82,3 s pendant l'attente ; porte-filtre
en place 113,9 s après la consigne ; préparation de 99,2 s. Tasse de
114,6 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,08 s | — | 0,17 → 0,26 bar |
| Remplissage | 8,08 s | **10,08 s, échéance** | **36,1 ml (3,59 ml/s)** | 0,26 → **0,48 bar** |
| Pré-infusion | 18,16 s | **3,96 s, échéance** | 8,4 ml | 0,48 → 0,70 bar |
| Infusion | 22,12 s | 24,84 s | 47,4 ml | **9,92 bar max** |

- **Première goutte (0,3 g) à 27,73 s, à 3,95 bar**, **67,1 ml**, 5,6 s
  après le début de l'infusion ; marge de remplissage de 31,0 ml.
- 8 bar à 28,73 s (6,6 s après le début de l'infusion), 1,1 g en tasse.
  **Pic à 9,92 bar** à 29,04 s, 0,3 s après 8 bar : la pompe était à
  100 % (≈ 4 500 tr/min) de 25,0 s jusqu'au seuil, et la pression a
  gagné 5 bar dans la dernière seconde.
- Depuis 8 bar : **18,2 s**, **0,99 g/s** en tasse, 1,17 ml/s en amont.
  **19,2 g à l'arrêt de la pompe**, **20,7 g final**, **1,5 g de
  gouttes**. Pompe en marche 38,9 s. Goutte → arrêt de la pompe :
  19,2 s.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,97 ± 0,05 bar**, pompe à
  66–71 %, 0,99 g/s, soit **9,05 bar·s/g** à 7,5 g. Pompe à
  2 550 tr/min, 4,25 ml/s refoulés, 1,08 ml/s au Digmesa : **fuite
  interne de 3,17 ml/s**.
- Après l'arrêt, la pression reste à **6,6 bar** et remonte à 6,8 bar en
  4 s, chauffe rallumée.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 15/11 21 h 41 | 6,4 s | 11,0 s | 16,4 s | 24,0 s |
  | 11/11 8 h 25 | 1,5 s | 4,4 s | 7,8 s | 13,6 s |
  | 10/11 13 h 42 | 2,1 s | 4,8 s | 8,7 s | 14,7 s |

**Thermique.** NTC de 90,16 °C au départ, **+2,56 °C** pendant le
remplissage (36,1 ml), **+1,42 °C** pendant la pré-infusion ; 94,70 °C au
début de l'infusion, **maximum de 96,73 °C** à 27,5 s, chauffe à 0 %
depuis 21,0 s ; 89,48 °C à l'arrêt. Baisse pompe en marche **−0,62 °C**.
**Tasse : 91,72 °C, +1,72 °C** ; 93,35 °C pondérée par le volume.

**Attendus vérifiés** (entrée du 11/11, prévue pour la vibratoire et le
ViCafé, machine chaude).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée | pesée non précisée ; ratio **2,76** à la dose du profil |
| Hydraulique | 4,7 à 7,6 bar·s/g (≈ 7 à 14 s depuis 8 bar, ≈ 1,2 à 1,9 g/s) ; plateau sous 9,2 bar | **9,05 bar·s/g, 18,2 s, 0,99 g/s : au-dessus** ; plateau 8,97 bar atteint, **pic à 9,92 bar** |
| Rinçage | 0,1 à 0,7 bar | **0,96 bar, au-dessus** |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 30 ml ; noter la fin | **3,59 ml/s, au-dessus ; 36,1 ml, au-dessus** ; fin sur l'échéance |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,0 g final | 19,2 g ; 20,7 g : atteint |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | **+2,56 °C, au-dessus** ; −0,62 °C : atteint |
| Tasse | 89,0 à 92,5 °C ; goût noté au regard de la température | 91,72 °C : atteint ; goût non noté |

**Lecture.**

- *Résistance, mesuré.* 9,05 bar·s/g, au-dessus de la plage du moulin à
  cran fixe (4,7 à 7,6) et la plus haute depuis le ViCafé trop fin du
  29/10 (8,7). Elle ne doit rien à la pompe : c'est la galette, comme le
  dit David (Cozy Chocolate, deux crans plus serré). Le Cozy Chocolate
  donnait 5,5 à 7,4 bar·s/g fin octobre, au porte-filtre à bec et à
  un autre cran.
- *Pompe au plateau, mesuré.* 66 à 71 % tiennent 9 bar pour 1,08 ml/s,
  avec une fuite de 3,17 ml/s : les mesures du 14/11 contre l'OPV
  (70 % → 1,19 ml/s à 9,68 bar, fuite de 3,18 ml/s) se retrouvent sur
  une galette. Le plateau est aussi lisse qu'avec la vibratoire
  (± 0,05 bar, ± 0,07 le 11/11).
- *Pic, mesuré.* 9,92 bar, contre 9,26 à 9,46 bar au plus avec la
  vibratoire, et souvent pas de pic. La pompe reste à 100 % (7,5 ml/s
  refoulés) jusqu'à 8 bar, et la commande ne retombe à 54 % qu'au-dessus
  de la cible. Hypothèse : la régulation de la montée, réglée sur la
  vibratoire, coupe trop tard pour cette pompe sur une galette serrée,
  qui fait monter la pression vite une fois mouillée. À revoir sur une
  galette moins résistante.
- *Remplissage, mesuré.* Fini sur l'échéance de 10 s à 0,48 bar, avec
  36,1 ml à 3,59 ml/s. Ce volume est réel : à 70 %, la pompe tourne à
  3 150 tr/min (5,25 ml/s refoulés) et le Digmesa compte ≈ 4,0 ml/s,
  76 % du refoulé, comme les 75 % des purges sans porte-filtre du 14/11.
  44,5 ml entrent avant 0,7 bar, contre 17 à 30 ml avant 0,8 bar avec la
  vibratoire.
- *Remplissage, ce qui ne se sépare pas.* Le 6/11 à 7 h 39, la galette la
  plus résistante de la vibratoire (7,6 bar·s/g) finissait déjà sur
  l'échéance, à 0,68 bar, avec la goutte la plus tardive (38,3 ml) : une
  galette serrée mouille lentement. La pompe peut aussi y être pour
  quelque chose. Les rinçages en donnent un indice : 1,84 à 2,16 g en
  tasse par ml au Digmesa avec la vibratoire (10/11, 11/11, 6/11 à
  13 h 19), 1,24 g/ml ce soir. Hypothèse : le Digmesa sous-comptait la
  vibratoire à basse pression, sous sa pulsation, et les volumes de
  remplissage et de goutte des entrées précédentes ne se comparent pas à
  celui de ce soir. Non établi.
- *Première goutte, mesuré.* À 3,95 bar, contre 0,95 à 2,30 bar avec la
  vibratoire : elle arrive pendant la montée, pas pendant la
  pré-infusion, et la pré-infusion finit sur l'échéance.
- *Chauffe, mesuré.* +3,98 °C entre le remplissage et la pré-infusion, et
  la NTC monte encore jusqu'à 96,73 °C, 6,7 °C au-dessus de la consigne,
  chauffe coupée. Le remplissage chauffe à 90 % sur toute sa durée, et ce
  soir il a duré 10 s. Le 6/11 à 7 h 39, un remplissage de 10 s ne donnait
  pourtant que +0,04 °C : la durée seule ne l'explique pas. Cause non
  établie. La tasse reste dans sa plage (91,72 °C), avec une baisse pompe
  en marche faible (−0,62 °C).
- *Machine ouverte, remarque de David.* La machine est ouverte depuis le
  changement de pompe. Elle devrait perdre plus d'énergie, mais elle peut
  aussi en transmettre davantage à la sonde ou au groupe : son effet sur la
  NTC n'est pas connu.
- *Retour à la consigne, mesuré.* 750,7 s après le rinçage, chauffe
  allumée 82,3 s seulement, depuis un minimum de 81,6 °C. Le 11/11, la
  machine partie froide (11,3 °C) revenait en 270,2 s. Cause non
  établie : la capture ne garde que la durée de chauffe de l'attente.
- *Pression retenue, mesuré.* Après l'arrêt, le clapet garde 6,6 bar dans
  la chaudière, comme au banc le 14/11, et la chauffe la fait remonter
  (6,8 bar en 4 s). Le départ de ce soir était à 0,17 bar : elle était
  retombée depuis la purge précédente.
- *Rinçage, mesuré.* 0,96 bar, au-dessus de la plage, sans lien établi avec
  la pompe : la vibratoire donnait déjà 0,14 à 9,12 bar.
- *Concentration, mesuré.* Premier relevé au réfractomètre DiFluid R2 :
  **TDS de 7,75 %** (indice 1,34676, à 25,1 °C). Rendement d'extraction =
  TDS × poids final / dose : 7,75 % × 20,7 g / 7,5 g = **21,4 %**, avec les
  masses de la capture. Second relevé plus tard, par `coffeetool tds
  --capture` : 7,63 % (indice 1,34653, à 24,4 °C), soit 21,1 %.

**Décision** (David). **Mouture desserrée de deux crans** pour l'infusion
de demain ; **purge de 2 L** avant de boire le café de la nouvelle pompe.
Profil inchangé : 7,5 g, cible 20 g, anticipation de 1,0 g, 90 °C.
**Temps de réaction de la montée en pression à reprendre** pour la MPB003,
qui monte plus vite que la vibratoire : 6,7 bar/s de 6 à 8,5 bar ce soir,
contre 3,1 bar/s le 11/11 et 5,5 bar/s le 6/11 à 7 h 39, la galette la
plus serrée de la vibratoire (pic à 9,47 bar). **Loi de chauffe pas
reprise pour l'instant** : on y revient si la montée de la NTC au
remplissage se répète sur plusieurs infusions.

*Estimé.* Avec 12 à 23 % par cran (ViCafé), deux crans plus gros
ramènent 9,05 vers **6,0 à 7,2 bar·s/g**, soit ≈ 1,25 à 1,5 g/s à 9 bar ;
la dispersion du moulin à cran fixe (+62 %) peut porter la mesure de
4,7 à 7,6. Au plateau, la pompe tiendrait alors 9 bar vers 70 à 75 %
(14/11 : 70 % → 1,19 ml/s, 80 % → 2,18 ml/s). Le remplissage sépare la
pompe et la galette : s'il finit sur la pression avant 10 s, la galette
expliquait l'échéance de ce soir ; s'il finit encore sur l'échéance
vers 36 ml, c'est la pompe.

| Point | Attendu (prochaine infusion, deux crans plus gros, machine chaude) |
| --- | --- |
| Dose | pesée ; ratio = final / dose pesée |
| Hydraulique | 4,7 à 7,6 bar·s/g (≈ 1,2 à 1,9 g/s) ; plateau vers 9,0 bar, pompe à 70 à 75 % ; pic au-dessus de 9,2 bar possible |
| Remplissage | ≈ 3,6 ml/s ; noter la fin (pression ou échéance) et le volume |
| Première goutte | sous 3,95 bar ; noter si elle finit la pré-infusion |
| Rinçage | 0,1 à 0,7 bar |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,0 g final |
| Chauffe | NTC +0,1 à +2,6 °C au remplissage ; −1,7 à +1,9 °C pompe en marche ; retour à la consigne à noter s'il dépasse 300 s |
| Tasse | 89,0 à 92,5 °C ; goût noté, après la purge de 2 L |
| Extraction | TDS au DiFluid R2, avec la dose pesée et le poids final de la capture |

### 16 novembre, 11 h 29 — `261116-112941`

**Contexte.** Deuxième infusion avec la MPB003, **première en 0.5.5** (écran
et sensors) : le PI de pression tourne à chaque mesure (≈ 100 ms au lieu de
200 ms) et reçoit un terme dérivé, Kd = 7,5 %/(bar/s). Cozy Chocolate,
**mouture desserrée de deux crans** et purge de 2 L faite, comme décidé le
15/11. Single espresso, porte-filtre sans fond, profil inchangé : 7,5 g
(pesée non précisée), cible 20 g, anticipation de 1,0 g, consigne de
90 °C. Boîtier de l'écran à 30,4 °C et 38,9 %.

**Observé par David.** Le café apparaît sur la couronne du panier à la fin
de la pré-infusion, mais ne couvre le fond qu'une fois l'infusion lancée :
« comme si le remplissage ne fournissait pas assez de volume ». Au plateau,
l'aiguille du manomètre fait des allers-retours et la pompe tourne par
à-coups, « comme une visseuse ».

**Routine** : guide ouvert à 88,2 °C ; rinçage de 5,0 s, 27,7 ml, 28,6 g,
pression maximale 0,51 bar. **Retour à la consigne en 897,0 s**, minimum
82,0 °C, chauffe allumée 94,5 s pendant l'attente ; porte-filtre en place
524,0 s après la consigne ; préparation de 155,9 s. Tasse de 125,2 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,02 s | — | 0,15 → 0,25 bar |
| Remplissage | 8,02 s | **10,00 s, échéance** | **36,1 ml (3,61 ml/s)** | 0,25 → **0,46 bar** |
| Pré-infusion | 18,02 s | 4,04 s, échéance | 8,4 ml (2,09 ml/s) | 0,46 → 0,66 bar |
| Infusion | 22,06 s | 19,41 s | 52,6 ml | 9,68 bar max |

- **Première goutte (0,3 g) à 26,80 s, à 2,78 bar**, 63,5 ml, 4,7 s après
  le début de l'infusion.
- 8 bar à 29,69 s (7,6 s après le début de l'infusion), 4,2 g en tasse.
  Depuis 8 bar : 11,8 s, **1,27 g/s** en tasse, 2,01 ml/s en amont.
  **19,1 g à l'arrêt de la pompe**, **21,2 g final**, 2,1 g de gouttes.
  Pompe en marche 33,4 s.
- Plateau (31,7 à 41,3 s) : **8,65 ± 0,64 bar**, pompe à **10–100 %**,
  1,29 g/s, soit **6,72 bar·s/g** à 7,5 g. Pompe à 2 807 tr/min en
  moyenne, 4,68 ml/s refoulés, 2,06 ml/s au Digmesa : fuite interne de
  2,62 ml/s.
- **Oscillation** : la pression va de 7,34 à 9,68 bar avec une période de
  **0,78 s** (13 passages vers le haut de 31,9 à 41,3 s) ; le moteur passe
  de ≈ 830 à ≈ 4 500 tr/min à chaque cycle. La veille, en 0.5.4, le même
  plateau tenait 8,86 à 9,11 bar avec une commande de 66 à 71 %.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 16/11 11 h 29 | 5,1 s | 8,3 s | 12,7 s | 18,4 s |
  | 15/11 21 h 41 | 6,4 s | 11,0 s | 16,4 s | 24,0 s |
  | 11/11 8 h 25 | 1,5 s | 4,4 s | 7,8 s | 13,6 s |

**Thermique.** NTC de 90,00 °C au départ, **+3,23 °C** pendant le
remplissage, +1,12 °C pendant la pré-infusion ; 95,04 °C au début de
l'infusion, maximum de 96,04 °C pompe en marche ; 89,75 °C à l'arrêt.
Baisse pompe en marche −0,17 °C. **Tasse : 92,87 °C, +2,87 °C** ;
93,51 °C pondérée par le volume.

**Attendus vérifiés** (entrée du 15/11, deux crans plus gros, machine
chaude).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée | pesée non précisée ; ratio **2,83** à la dose du profil |
| Hydraulique | 4,7 à 7,6 bar·s/g ; plateau vers 9,0 bar, pompe à 70 à 75 % ; pic au-dessus de 9,2 bar possible | 6,72 bar·s/g : atteint ; **plateau 8,65 ± 0,64 bar, pompe à 10–100 % : oscillation** ; pic 9,68 bar |
| Remplissage | ≈ 3,6 ml/s ; noter la fin et le volume | 3,61 ml/s : atteint ; **fin sur l'échéance, 36,1 ml** |
| Première goutte | sous 3,95 bar ; noter si elle finit la pré-infusion | 2,78 bar : atteint ; pendant l'infusion, pas la pré-infusion |
| Rinçage | 0,1 à 0,7 bar | 0,51 bar : atteint |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,0 g final | 19,1 g : atteint ; **21,2 g, au-dessus** |
| Chauffe | NTC +0,1 à +2,6 °C au remplissage ; −1,7 à +1,9 °C pompe en marche ; retour à la consigne à noter s'il dépasse 300 s | **+3,23 °C, au-dessus** ; −0,17 °C : atteint ; **retour en 897,0 s** |
| Tasse | 89,0 à 92,5 °C ; goût noté, après la purge de 2 L | **92,87 °C, au-dessus de 0,4 °C** ; goût noté |
| Extraction | TDS au DiFluid R2 | **7,63 %, 21,5 %** |

**Lecture.**

- *Galette, mesuré.* Deux crans plus gros ramènent la résistance de 9,05 à
  6,72 bar·s/g, dans la fourchette estimée le 15/11 (6,0 à 7,2).
- *Remplissage : c'est la pompe, mesuré.* Avec une galette dans la plage,
  le remplissage finit encore sur l'échéance à 36,1 ml et 0,46 bar, comme
  la veille. Selon la règle posée le 15/11, la galette n'expliquait pas
  l'échéance. La pression au repos est de 0,25 bar ; le débit de 3,6 ml/s
  n'y ajoute que 0,21 bar.
- *Le Digmesa sous-comptait la vibratoire, indice fort.* Au rinçage, la
  vibratoire donnait 15 à 19 ml au Digmesa pour 18,6 à 36,5 g en tasse
  (1,07 à 2,15 g/ml, ≈ 1,5 en médiane, 17 infusions du 1er au 11/11) ; la
  MPB003 donne 1,24 puis 1,03 g/ml. Les 17 à 30 ml de remplissage de la
  vibratoire valaient donc plutôt ≈ 25 à 45 ml réels, à ≈ 4,5 ml/s contre
  3,6 ml/s ce matin. Les volumes de remplissage et de goutte de la
  vibratoire ne servent plus de référence. Moins de débit, c'est moins de
  pression dans le même circuit : le seuil de 0,8 bar n'est jamais atteint,
  et la pré-infusion à 50 % (2,1 ml/s) n'ajoute que 0,2 bar. C'est
  l'impression de David.
- *Oscillation : le terme dérivé, mesuré et simulé.* Entre une commande et
  son effet sur la pression, il passe ≈ 0,25 à 0,4 s. À la période de
  l'oscillation (1,3 Hz), le terme dérivé pèse ≈ 60 %/bar, quatre fois le
  proportionnel (15 %/bar). Le calcul de 0.5.5 avait mesuré le bruit de la
  pente sur un plateau régulé sans dérivée (±1,3 % de commande) ; boucle
  fermée, la dérivée entretient sa propre oscillation. Le premier pic
  baisse (9,56 bar contre 9,92), mais chaque cycle remonte à 9,4–9,7 bar.
- *Simulation.* Modèle pompe + galette (retard, moteur du premier ordre,
  fuite interne de 0,33 ml/s par bar, compliance, galette linéaire), calé
  en boucle fermée pour reproduire les deux infusions : en 0.5.5,
  7,42–9,42 bar, commande 28–100 %, période 0,90 s (mesuré 7,34–9,68,
  10–100 %, 0,78 s) ; en 0.5.4, pic à 10,07 bar et plateau stable
  9,00–9,08 (mesuré 9,92 et 8,86–9,11). Pic simulé sur la galette de la
  veille, montée depuis 3 bar à 100 %, retard de 0,25 / 0,35 / 0,4 s :

  | Régulation | Pic | Plateau |
  | --- | --- | --- |
  | 0.5.4 (PI toutes les 200 ms) | 10,06 bar (retard 0,25 s) | stable |
  | 0.5.5 (Kd 7,5 permanent) | 9,59 bar | **oscille, 7,4–9,6 bar** |
  | 1 : Kd = 0, PI toutes les 100 ms | 9,39 / 9,78 / 9,96 bar | stable, ±1 % |
  | 2 : Kd 7,5 jusqu'au premier pic | 9,44 bar ; 8,7 bar atteint 0,8 s plus tard | stable |
  | 2 : Kd 2 jusqu'au premier pic | 8,93 / 9,20 / 9,41 bar | stable, ±1 % |
  | 2 : Kd 3 jusqu'au premier pic | 8,93 / 9,04 / 9,26 bar | stable, ±1 % |
  | 3 : Kd 2 permanent | 8,93 / 9,20 / 9,41 bar | stable, ±2 % |

  Une bonne part du gain sur 0.5.4 vient du PI à chaque mesure (10,06 →
  9,39 bar). Kd = 7,5 est trop fort même pendant la seule montée : la
  pression cale avant la cible, puis dépasse. Avec Kd = 2 à 3, le pic baisse
  encore de 0,4 à 0,7 bar ; limité à la montée, le terme dérivé ne touche
  plus le plateau, et reste stable à 0,5 s de retard (±0,2 bar, contre
  ±0,34 pour Kd = 3 permanent). Simulé seulement : un modèle calé sur deux
  infusions, galette à résistance constante.
- *Chauffe, mesuré.* +3,23 °C au remplissage, après +2,56 °C la veille :
  deuxième dépassement de suite, le critère posé le 15/11 pour reprendre la
  loi de chauffe. Tasse à 92,87 °C, 0,4 °C au-dessus de la plage. Retour à
  la consigne encore long : 897,0 s, chauffe allumée 94,5 s.
- *Concentration, mesuré.* Deux moyennes de trois tests au DiFluid R2 :
  **7,62 et 7,64 %** (indice 1,34652 et 1,34655, échantillon à 24,1–24,2 °C),
  soit **21,5 et 21,6 %** d'extraction avec 21,2 g / 7,5 g. Proche de la
  veille (7,63 à 7,75 %, 21,1 à 21,4 %) avec une mouture plus grosse de deux
  crans et un ratio plus haut.
- *Goût, David.* **Très bon, acidulé et fruité, « probablement dans les
  meilleurs cafés que j'ai goûtés »**. Peu de crème, que David attribue au
  peu de temps passé à 9 bar.

**Décision** (David). **Remplissage à 100 %** pour voir s'il finit sur la
pression. **Écran 0.5.6** : terme dérivé de **Kd = 3 %/(bar/s) pendant la
seule montée**, éteint à la première pente nulle ou négative une fois le PI
actif ; PI à chaque mesure comme en 0.5.5. Profil inchangé par ailleurs. La
simulation est reprise dans `firmware/analysis/simulate_brew_pressure.py`
(`calibrate` sur les deux captures, `compare` pour les variantes).

*Estimé.* À 100 %, la pompe tournait à ≈ 4 500 tr/min et le Digmesa
comptait ≈ 5,1 ml/s à 1,5 bar ce matin : ≈ 5 à 5,5 ml/s au remplissage,
50 ml et plus en 10 s. La pression au repos est de 0,25 bar et le débit
ajoutait 0,21 bar à 3,6 ml/s ; à 5,3 ml/s, elle irait vers 0,55 bar si la
perte de charge suit le débit, 0,7 bar si elle suit son carré. **Le seuil
de 0,8 bar reste donc incertain** ; l'échéance à ≈ 50 ml est le cas
probable.

| Point | Attendu (prochaine infusion, remplissage à 100 %) |
| --- | --- |
| Dose | pesée ; ratio = final / dose pesée |
| Remplissage | ≈ 5 à 5,5 ml/s ; fin sur la pression possible, sinon échéance vers 50 ml ; noter la fin et la pression |
| Première goutte | plus tôt qu'à 4,7 s dans l'infusion ; noter si elle arrive pendant la pré-infusion |
| Hydraulique | 4,7 à 7,6 bar·s/g ; en 0.5.6, pic sous 9,4 bar, plateau stable vers 9,0 bar, commande à ± 5 % au plus, plus d'à-coups audibles |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,2 g final |
| Chauffe | NTC au remplissage à noter : un remplissage plus court ou plus long change la durée de chauffe à 90 % |
| Tasse | 89,0 à 92,5 °C ; goût noté |
| Extraction | TDS au DiFluid R2 |

### 16 novembre, 15 h 02 — `261116-150255`

**Contexte.** **Première infusion en écran 0.5.6** (terme dérivé de
Kd = 3 %/(bar/s) pendant la seule montée) ; sensors encore en 0.5.5, qui ne
diffère de 0.5.6 que par son numéro. **Remplissage à 100 %** au lieu de
70 %. Cozy Chocolate, même mouture que le matin. Single espresso,
porte-filtre sans fond, 7,5 g (pesée non précisée), cible 20 g, anticipation
de 1,0 g, consigne de 90 °C. Boîtier de l'écran à 29,0 °C et 40,8 %.

**Observé par David.** Au retrait du porte-filtre, **encore pas mal d'eau
au-dessus de la galette**, « comme si la pression avait baissé avant la
fermeture de la vanne » : pas de grande décharge.

**Routine** : guide ouvert à 84,4 °C ; rinçage de 5,0 s, 28,5 ml, 28,9 g,
pression maximale 0,16 bar. **Retour à la consigne en 512,6 s**, minimum
78,3 °C, chauffe allumée 78,6 s pendant l'attente ; porte-filtre en place
679,9 s après la consigne ; préparation de 87,7 s. Tasse de 110,8 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | 0,28 → 0,37 bar |
| Remplissage | 8,01 s | **5,58 s, sur la pression** | **30,5 ml (5,47 ml/s)** | 0,37 → **0,91 bar** |
| Pré-infusion | 13,59 s | 4,13 s, échéance | 8,4 ml (2,04 ml/s) | 0,91 → 1,14 bar |
| Infusion | 17,72 s | 20,07 s | 29,7 ml | **9,05 bar max** |

- **Première goutte (0,3 g) à 19,46 s, à 1,60 bar**, 42,6 ml, **1,7 s**
  après le début de l'infusion (4,7 s le matin).
- 8 bar à 21,89 s (4,2 s après le début de l'infusion), 2,2 g en tasse.
  **Pic à 9,05 bar** à 22,96 s. La commande descend à 66 % vers 7,4 bar ;
  la pression marque un palier à 7,8 bar (21,4 à 21,7 s), où la pente nulle
  éteint le terme dérivé, puis le PI finit la montée.
- Depuis 8 bar : 15,9 s, **1,08 g/s** en tasse, 1,14 ml/s en amont.
  **19,3 g à l'arrêt de la pompe**, **20,6 g final**, 1,3 g de gouttes.
  Pompe en marche 29,7 s.
- Plateau (24,0 à 37,7 s) : **8,87 ± 0,08 bar**, pompe à **67–70 %**,
  1,07 g/s, soit **8,28 bar·s/g** à 7,5 g. Pompe à 2 530 tr/min,
  4,22 ml/s refoulés, 1,09 ml/s au Digmesa : fuite interne de 3,13 ml/s.
- **Après l'arrêt** : la pression reste à 8,98 bar jusqu'à 0,2 s, descend à
  8,49 bar à 0,4 s, 7,65 bar à 0,6 s, **6,48 bar à 0,9 s**, puis remonte
  avec la chauffe (8,05 bar à 7,8 s). Le tachymètre lit 0 à 0,71 s ; la
  vanne se ferme 200 ms après, vers 0,9 s.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 16/11 15 h 02 | 2,9 s | 6,9 s | 11,7 s | 19,1 s |
  | 16/11 11 h 29 | 5,1 s | 8,3 s | 12,7 s | 18,4 s |
  | 15/11 21 h 41 | 6,4 s | 11,0 s | 16,4 s | 24,0 s |

**Thermique.** NTC de 90,12 °C au départ, **+1,33 °C** pendant le
remplissage, −2,03 °C pendant la pré-infusion ; 89,71 °C au début de
l'infusion, maximum de 91,79 °C pompe en marche ; 87,97 °C à l'arrêt.
**Baisse pompe en marche −2,14 °C.** **Tasse : 89,79 °C, −0,21 °C** ;
90,70 °C pondérée par le volume.

**Attendus vérifiés** (entrée de 11 h 29, remplissage à 100 %).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée | pesée non précisée ; ratio **2,75** à la dose du profil |
| Remplissage | ≈ 5 à 5,5 ml/s ; fin sur la pression possible, sinon échéance vers 50 ml | 5,47 ml/s : atteint ; **fin sur la pression, 0,91 bar à 5,58 s, 30,5 ml** |
| Première goutte | plus tôt qu'à 4,7 s dans l'infusion ; noter si elle arrive pendant la pré-infusion | 1,7 s : atteint ; pendant l'infusion |
| Hydraulique | 4,7 à 7,6 bar·s/g ; en 0.5.6, pic sous 9,4 bar, plateau stable vers 9,0 bar, commande à ± 5 % au plus, plus d'à-coups audibles | **8,28 bar·s/g, au-dessus** ; pic 9,05 bar, plateau 8,87 ± 0,08 bar, pompe 67–70 % : atteint ; à-coups non signalés |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,2 g final | 19,3 g ; 20,6 g : atteint |
| Chauffe | NTC au remplissage à noter | +1,33 °C ; **baisse pompe en marche −2,14 °C, au-delà de −1,7** |
| Tasse | 89,0 à 92,5 °C ; goût noté | 89,79 °C : atteint ; goût noté |
| Extraction | TDS au DiFluid R2 | **7,60 %, 20,9 %** |

**Lecture.**

- *Régulation 0.5.6, mesuré.* Pic à 9,05 bar et plateau à ± 0,08 bar,
  commande de 67 à 70 % : le plateau est aussi calme qu'en 0.5.4
  (± 0,05 bar le 15/11), sans le pic à 9,92 bar. La simulation donnait 8,96
  à 9,34 bar de pic pour une galette serrée. Le palier à 7,8 bar montre que
  Kd = 3 freine encore un peu tôt : 8 bar arrive 0,4 s plus tard qu'en
  ligne droite. Sans conséquence ici.
- *Remplissage à 100 %, mesuré.* Il finit sur la pression en 5,58 s, avec
  30,5 ml à 5,47 ml/s, soit 0,91 bar contre 0,46 bar à 3,6 ml/s le matin.
  L'estimation (0,55 à 0,7 bar) était trop basse : la perte de charge
  croît plus vite que le carré du débit, ou la galette mouillée résiste
  déjà davantage. La goutte arrive 1,7 s après le début de l'infusion.
- *Chauffe, mesuré.* Avec 5,6 s de remplissage au lieu de 10, la NTC ne
  monte que de 1,33 °C (+3,23 et +2,56 °C sur 10 s) : la durée de chauffe
  à 90 % pendant le remplissage explique au moins une part de la montée.
  La pré-infusion la reprend (−2,03 °C) et la baisse pompe en marche
  (−2,14 °C) dépasse la plage ; la tasse tombe à 89,79 °C, dans la plage.
- *Galette, mesuré.* 8,28 bar·s/g contre 6,72 le matin, à mouture égale :
  dans la dispersion du moulin à cran fixe (+62 %).
- *Eau sur la galette, hypothèse.* Le capteur de pression est en amont de
  la vanne : après sa fermeture, il lit la chaudière, pas le groupe. Avant
  la fermeture, il lit les deux. Ce soir, la pression descend de 8,98 à
  6,48 bar entre 0,2 et 0,9 s après la consigne à 0 : la pompe ralentit,
  la galette laisse passer l'eau, et la vanne ne se ferme qu'une fois le
  tachymètre à l'arrêt depuis 200 ms (règle de 0.5.1, pour les purges à
  100 % sans charge). Avec la vibratoire, la même chute finissait en
  0,4 s (10/11 : 9,06 → 6,81 bar ; 11/11 : 8,83 → 6,50 bar) ; avec la
  MPB003, en 0,5 à 0,9 s (15/11 et 16/11). La décharge du groupe part donc
  peut-être d'une pression plus basse. Non établi : la chute à ≈ 6,5 bar
  existe aussi avec la vibratoire, où elle est la fermeture elle-même, et
  le capteur ne voit pas le groupe après la fermeture. Premier signalement :
  rien sur les deux infusions précédentes de la MPB003.
- *OPV, signalé par David et mesuré.* Un peu d'eau dans la tasse de
  retour de l'OPV. Pendant l'infusion, la pression ne dépasse pas
  9,05 bar (ouverture vers 9,45 bar), et le Digmesa (1,09 ml/s) suit la
  tasse (1,07 g/s) au plateau : rien n'est passé par l'OPV pendant la
  mesure. Après l'arrêt, la chaudière fermée remonte avec la chauffe, à
  0,26 bar/s en fin de capture (8,64 bar à 47,9 s) : l'OPV s'ouvre
  probablement ≈ 3 s plus tard. **Le matin, en revanche**, 15 mesures
  dépassent 9,3 bar pendant l'oscillation, et le Digmesa compte 2,06 ml/s
  pour 1,29 g/s en tasse : la « fuite interne » de 2,62 ml/s de 11 h 29
  inclut le retour de l'OPV.
- *Concentration, mesuré.* **TDS de 7,60 %** (un seul test, indice
  1,34649, échantillon à 27,0 °C), **20,9 %** d'extraction avec
  20,6 g / 7,5 g.
- *Goût, David.* **Très bon, un peu moins acide que le matin.** Tasse plus
  froide de 3,1 °C (89,79 contre 92,87 °C), extraction un peu plus basse
  (20,9 contre 21,5 %).

**Décision** (David). **Sensors 0.5.7 : en fin d'infusion, sous au moins
5 bar et à 3 500 tr/min au plus, la vanne se ferme avec la consigne à 0** ;
la règle du tachymètre reste pour les purges, qui doivent toujours faire
baisser la pression. **Consigne à 92 °C envisagée** par David pour
retrouver l'acidité du matin (tasse attendue vers 91,8 °C) ; pré-infusion à
60 % envisagée aussi, à essayer après la consigne pour ne changer qu'une
chose à la fois.

| Point | Attendu (prochaine infusion, 0.5.7, si la consigne passe à 92 °C) |
| --- | --- |
| Dose | pesée ; ratio = final / dose pesée |
| Remplissage | fin sur la pression vers 0,9 bar, ≈ 5,5 ml/s |
| Hydraulique | 4,7 à 7,6 bar·s/g ; pic sous 9,4 bar, plateau 9,0 ± 0,1 bar |
| Arrêt | pression lue à la fermeture plus haute que 6,5 bar ; noter l'eau sur la galette et la décharge ; retour OPV après l'arrêt probable |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,2 g final |
| Chauffe | NTC +1 à +2 °C au remplissage |
| Tasse | 90,5 à 92,5 °C ; goût noté au regard de l'acidité |
| Extraction | TDS au DiFluid R2 |

### 17 novembre, 8 h 08 — `261117-080814`

**Contexte.** **Première infusion en 0.5.7**, écran et sensors : en fin
d'infusion, sous au moins 5 bar et à 3 500 tr/min au plus, la vanne se
ferme avec la consigne à 0. **Consigne à 92 °C** (90 °C la veille).
Remplissage à 100 %. Cozy Chocolate, mouture non précisée. Single espresso,
porte-filtre sans fond, **7,4 g selon David** (dose du profil 7,5 g), cible 20 g,
anticipation de 1,0 g. Boîtier de l'écran à 34,3 °C et 40,2 %.

**Observé par David.** **Goût : très satisfait.** Pas de mesure TDS. À
l'écran, la pression se tient plutôt vers 8,6–8,7 bar que vers 9. À
l'arrêt, **l'aspiration par la purge est très forte** : galette très
dégradée, café remonté contre la douchette. La fermeture lui semble « un
poil trop tôt » ; garder un peu d'eau au-dessus de la galette serait
probablement plus propre.

**Routine** : guide ouvert à 88,52 °C ; rinçage de 5,1 s, 27,7 ml, 23,6 g,
pression maximale 0,17 bar. **Retour à la consigne en 191,9 s**, minimum
82,75 °C, chauffe allumée 70,3 s pendant l'attente ; porte-filtre en place
931,3 s après la consigne ; préparation de 76,6 s. Tasse de 125 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,00 s | — | 0,21 → 0,32 bar |
| Remplissage | 8,00 s | **8,94 s, sur la pression** | **50,6 ml (5,66 ml/s)** | 0,32 → 0,87 bar |
| Pré-infusion | 16,94 s | 3,27 s, goutte | 6,4 ml (1,96 ml/s) | 0,87 → 1,27 bar |
| Infusion | 20,21 s | 14,84 s | 33,3 ml | **8,90 bar max** |

- **Première goutte (0,3 g) à 20,97 s, à 1,38 bar**, 58,2 ml, **0,75 s**
  après le début de l'infusion ; la pré-infusion s'arrête sur le premier
  poids.
- 8 bar à 25,83 s (5,6 s après le début de l'infusion), 5,5 g en tasse.
  La commande descend de 100 à 76 % entre 24,4 et 25,2 s (terme dérivé),
  la pression marque un palier à 8,1 bar (25,9 à 26,3 s), puis le PI la
  fait monter lentement : 8,5 bar à 27,2 s, 8,7 bar à 29,6 s, **pic à
  8,90 bar à 34,58 s**, juste avant l'arrêt.
- Depuis 8 bar : 9,2 s, **1,49 g/s** en tasse, 1,66 ml/s en amont.
  **19,2 g à l'arrêt de la pompe**, **20,7 g final**, 1,5 g de gouttes.
  Pompe en marche 26,9 s.
- **Plateau (27,8 à 34,9 s) : 8,73 ± 0,09 bar**, pompe à **71–73 %**,
  1,50 g/s, soit **5,84 bar·s/g**, 5,92 ramenés à 7,5 g. Pompe à 2 805 tr/min,
  4,68 ml/s refoulés, 1,60 ml/s au Digmesa : fuite interne de 3,07 ml/s.
- **Après l'arrêt** : la vanne se ferme avec la consigne à 0, à 35,05 s,
  sur une dernière mesure à 8,89 bar et 2 835 tr/min. Le capteur lit
  ensuite la chaudière fermée : **7,48 bar à 0,33 s**, puis remontée avec
  la chauffe (8,03 bar à 2,3 s, 9,09 bar à 9,9 s, fin de capture). Le
  tachymètre lit 816 tr/min à 0,33 s, 0 à 0,65 s.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 17/11 8 h 08 | 1,8 s | 5,4 s | 8,6 s | 14,1 s |
  | 16/11 15 h 02 | 2,9 s | 6,9 s | 11,7 s | 19,1 s |
  | 16/11 11 h 29 | 5,1 s | 8,3 s | 12,7 s | 18,4 s |

**Thermique.** NTC de 91,71 °C au départ, +0,28 °C pendant la précharge,
**+2,62 °C pendant le remplissage**, **+0,34 °C pendant la
pré-infusion** ; 94,98 °C au début de l'infusion, **maximum de 97,11 °C** à
28,7 s, chauffe éteinte pendant toute l'infusion ; 95,80 °C à l'arrêt.
**Aucune baisse pompe en marche** (minimum de 92,04 °C, au début du
remplissage). **Tasse : 96,57 °C, +4,57 °C au-dessus de la consigne** ;
94,77 °C pondérée par le volume.

**Attendus vérifiés** (entrée de la veille, 15 h 02).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée | 7,4 g selon David ; ratio **2,80** |
| Remplissage | fin sur la pression vers 0,9 bar, ≈ 5,5 ml/s | 0,87 bar, 5,66 ml/s : atteint ; **mais 8,94 s et 50,6 ml** (30,5 ml la veille) |
| Hydraulique | 4,7 à 7,6 bar·s/g ; pic sous 9,4 bar, plateau 9,0 ± 0,1 bar | 5,84 bar·s/g, pic 8,90 bar : atteint ; **plateau 8,73 ± 0,09 bar, sous la plage** |
| Arrêt | pression lue à la fermeture plus haute que 6,5 bar ; noter l'eau sur la galette et la décharge | **8,89 bar** : atteint ; **décharge très forte, galette dégradée, café sur la douchette** |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,2 g final | 19,2 g ; 20,7 g : atteint |
| Chauffe | NTC +1 à +2 °C au remplissage | **+2,62 °C, au-dessus** |
| Tasse | 90,5 à 92,5 °C ; goût noté au regard de l'acidité | **96,57 °C, au-dessus de 4,1 °C** ; très satisfait |
| Extraction | TDS au DiFluid R2 | non mesurée |

**Lecture.**

- *Pression au plateau, signalé par David et mesuré.* 8,73 ± 0,09 bar en
  moyenne sur le plateau, 0,27 bar sous la consigne (8,87 bar la veille,
  sur 13,6 s de plateau). Après le palier du terme dérivé à 8,1 bar,
  l'écart se referme au rythme de l'intégrale : Ki = 3 %/(bar·s), soit
  ≈ 1 %/s pour 0,3 bar d'écart, pendant que le terme proportionnel
  décroît. La commande reste entre 71 et 73 % et la pression gagne
  0,4 bar en 7 s. La galette rapide (1,5 g/s) arrête l'infusion avant que
  la pression atteigne 9 bar.
- *Arrêt 0.5.7, mesuré.* La condition est remplie (8,89 bar,
  2 835 tr/min) et la vanne se ferme avec la pompe : le groupe se vide
  par la purge depuis ≈ 8,9 bar au lieu de ≈ 6,5 bar la veille. De l'eau
  sur la galette sans grande décharge, la veille ; décharge très forte et
  galette dégradée ce matin.
- *Délai de 50 ms, mesuré sur l'infusion de la veille.* Après la consigne
  à 0, la pression du groupe reste à 8,98 bar à 57 ms, puis passe à
  8,49 bar à 284 ms, 7,65 bar à 415 ms, 7,33 bar à 584 ms et 6,48 bar à
  770 ms. Fermer la vanne 50 ms plus tard la fermerait donc encore vers
  8,9 bar. Les captures, à 100 ms, ne distinguent pas 50 ms.
- *Chaudière fermée, mesuré.* Elle perd 1,4 bar dans les 0,33 s qui
  suivent la fermeture, au lieu de monter de moins de 1 bar comme
  l'estimait 0.5.7. Hypothèses : retour par le clapet avant qu'il se
  ferme, ou passage par la vanne pendant sa commutation. Non établi.
- *Chauffe, mesuré.* La tasse dépasse la consigne de 4,57 °C (−0,21 °C la
  veille). La NTC monte de 2,62 °C pendant un remplissage de 8,94 s,
  chauffe à 90 % (5,58 s la veille), puis continue de monter pendant la
  pré-infusion et pendant l'infusion, chauffe éteinte, jusqu'à 97,11 °C.
  La veille, la pré-infusion faisait −2,03 °C. Le remplissage plus long et
  l'inertie de la résistance expliquent peut-être une part de l'écart.
  Non établi.
- *Galette, mesuré.* 5,84 bar·s/g contre 8,28 la veille. Le remplissage
  admet 50,6 ml avant 0,8 bar (30,5 ml la veille) et la goutte arrive
  0,75 s après le début de l'infusion : galette plus perméable. David
  l'attribue à la dose, 7,4 g au lieu de 7,5 g. Ramenée à 7,5 g
  proportionnellement, la résistance ne passe qu'à 5,92 bar·s/g ; une
  galette moins haute laisse aussi plus d'espace au-dessus d'elle, ce que
  la règle proportionnelle ne compte pas.
- *Goût, David.* **Très satisfait.** Tasse à 96,57 °C selon la NTC,
  extraction non mesurée.

**Décision** (David). **Sensors 0.5.8 : en fin d'infusion, la vanne se
ferme quand le groupe passe sous 7,5 bar**, au plus tard 1 s après la
consigne à 0 ; conditions d'entrée inchangées (au moins 5 bar, pompe à
3 500 tr/min au plus). Relue toutes les 100 ms, la fermeture tombe vers 7,0
à 7,5 bar, ≈ 0,4 s après la consigne à 0.

| Point | Attendu (prochaine infusion, 0.5.8) |
| --- | --- |
| Dose | pesée ; ratio = final / dose pesée |
| Remplissage | fin sur la pression vers 0,9 bar, ≈ 5,5 ml/s |
| Hydraulique | 4,7 à 7,6 bar·s/g ; pic sous 9,4 bar ; plateau à noter au regard des 8,73 bar de ce matin |
| Arrêt | dernière pression lue avant la fermeture entre 7,0 et 7,5 bar, ≈ 0,4 s après la consigne à 0 ; décharge moins forte, galette moins dégradée, peu ou pas d'eau dessus |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,2 g final |
| Chauffe | NTC au remplissage et en tasse à noter au regard des +4,57 °C de ce matin |
| Tasse | goût noté |

### 17 novembre, 14 h 30 — `261117-143024`

**Contexte.** **Première infusion en 0.5.8**, écran et sensors : en fin
d'infusion, la vanne se ferme quand le groupe passe sous 7,5 bar, au plus
tard 1 s après la consigne à 0. Réglages inchangés depuis 8 h 08 : consigne
à 92 °C, remplissage à 100 %, Cozy Chocolate, single espresso, porte-filtre
sans fond, **7,6 g pesés** (dose du profil 7,5 g), cible 20 g, anticipation
de 1,0 g. **Infusion directe, hors guide** : David a oublié de lancer
l'infusion guidée, donc pas de rinçage guidé ni de tare automatique.
Boîtier de l'écran à 38,2 °C et 30,9 %.

**Observé par David.** **La MPB003 est validée au goût.** Rien d'autre de
signalé, ni sur la décharge ni sur la galette.

**Balance.** Sans la tare automatique du guide, elle lit **0,3 g dès le
début de la capture** et jusqu'à 19,76 s ; David aurait dû tarer à la
main. Le script prend ces 0,3 g pour la première goutte (« 0,15 s,
0,26 bar ») ; c'est l'oubli de la tare, pas le script, qui est en cause.
Les poids ci-dessous sont lus tels quels, avec la valeur nette (−0,3 g)
entre parenthèses quand elle compte. Le firmware compare le poids à celui
du début de la pré-infusion, il n'est pas trompé ; l'arrêt de la pompe,
lui, se fait sur le poids lu, donc 0,3 g plus tôt en tasse.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,15 s | 7,91 s | — | 0,26 → 0,35 bar |
| Remplissage | 8,06 s | **8,41 s, sur la pression** | **47,4 ml (5,63 ml/s)** | 0,35 → 0,90 bar |
| Pré-infusion | 16,47 s | 3,78 s, goutte | 8,0 ml (2,12 ml/s) | 0,90 → 1,29 bar |
| Infusion | 20,25 s | 14,01 s | 31,7 ml | **8,89 bar max** |

- **Première goutte (0,3 → 0,4 g) à 19,82 s, à 1,25 bar**, 54,6 ml, lue
  à la main ; le firmware termine la pré-infusion à 20,25 s, à 0,5 g lu.
- 8 bar à 25,69 s (5,44 s après le début de l'infusion), 5,5 g lus en
  tasse (5,2 g nets). **Pic à 8,89 bar à 33,82 s**, juste avant l'arrêt.
- Depuis 8 bar : 8,57 s, **1,61 g/s** en tasse, 1,64 ml/s en amont.
  **19,3 g lus à l'arrêt de la pompe (19,0 g nets)**, **21,6 g lus au final
  (21,3 g nets)**, **2,3 g de gouttes**. Pompe en marche 26,09 s.
- **Plateau (27,77 à 34,09 s) : 8,78 ± 0,06 bar**, pompe à 71–73 %,
  1,60 g/s, soit **5,50 bar·s/g**, 5,42 ramenés à 7,5 g. Pompe à
  2 785 tr/min, 4,64 ml/s refoulés, 1,59 ml/s au Digmesa : fuite interne
  de 3,05 ml/s.
- **Après l'arrêt** (consigne à 0 à 34,26 s, 8,87 bar) : 8,46 bar à
  0,35 s, **8,00 bar à 0,48 s**, puis **6,82 bar à 0,61 s**, chaudière
  fermée. La lecture sous 7,5 bar qui a fermé la vanne tombe entre deux
  échantillons de la capture. La chaudière fermée lit **6,74 bar à
  0,72 s**, puis remonte : 7,45 bar à 3,2 s, 8,82 bar en fin de capture
  (10,1 s). Le tachymètre lit 537 tr/min à 0,48 s, 0 à 0,72 s.
- Arrivée en tasse, depuis le début de l'infusion (poids nets pour
  14 h 30) :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 17/11 14 h 30 | 1,8 s | 5,0 s | 8,3 s | 13,6 s |
  | 17/11 8 h 08 | 1,8 s | 5,4 s | 8,6 s | 14,1 s |
  | 16/11 15 h 02 | 2,9 s | 6,9 s | 11,7 s | 19,1 s |

**Thermique.** NTC de 91,98 °C au départ, +0,52 °C pendant la précharge.
Pendant le remplissage, elle monte jusqu'à 93,58 °C puis redescend :
**+0,27 °C sur la phase** (+2,62 °C à 8 h 08). **−0,62 °C pendant la
pré-infusion** ; 92,15 °C au début de l'infusion, **maximum de 94,54 °C**
à 28,7 s, chauffe éteinte jusqu'à 32,47 s ; 93,45 °C à l'arrêt.
**Aucune baisse pompe en marche.** **Tasse : 94,03 °C, +2,03 °C
au-dessus de la consigne** (+4,57 °C à 8 h 08).

**Attendus vérifiés** (entrée de 8 h 08).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée | 7,6 g ; ratio **2,84** lu, 2,80 net |
| Remplissage | fin sur la pression vers 0,9 bar, ≈ 5,5 ml/s | 0,90 bar, 5,63 ml/s : atteint ; 8,41 s, 47,4 ml |
| Hydraulique | 4,7 à 7,6 bar·s/g ; pic sous 9,4 bar ; plateau à noter au regard des 8,73 bar | 5,50 bar·s/g, pic 8,89 bar : atteint ; plateau **8,78 ± 0,06 bar** |
| Arrêt | dernière pression lue avant la fermeture entre 7,0 et 7,5 bar, ≈ 0,4 s après la consigne à 0 ; décharge moins forte, galette moins dégradée | **non visible à 100 ms** : fermeture entre 0,48 et 0,61 s, après une lecture à 8,00 bar ; rien de signalé sur la décharge ni la galette |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,2 g final | 19,3 g lus : atteint ; **21,6 g lus, au-dessus**, 21,3 g nets |
| Chauffe | NTC au remplissage et en tasse à noter au regard des +4,57 °C | +0,27 °C au remplissage ; **+2,03 °C en tasse** |
| Tasse | goût noté | **MPB003 validée au goût** |

**Lecture.**

- *Arrêt 0.5.8, mesuré en partie.* La vanne se ferme entre 0,48 et 0,61 s
  après la consigne à 0, un peu plus tard que les ≈ 0,4 s estimés : la
  pression descend moins vite qu'à la fermeture de la veille (8,00 bar à
  0,48 s ici). La chaudière fermée lit 6,74 bar (7,48 bar à 8 h 08,
  fermée vers 8,9 bar) ; la perte après la fermeture se réduit donc à
  quelques dixièmes de bar si la vanne s'est fermée vers 7,4 bar
  (estimé, la valeur de fermeture n'est pas capturée).
- *Gouttes, mesuré.* 2,3 g après l'arrêt (1,5 g à 8 h 08). Pendant le
  ≈ 0,5 s où la vanne reste ouverte, le groupe sous 8 bar pousse encore à
  travers la galette ; ça explique peut-être le surplus. Hypothèse non
  testée. Le final dépasse la plage de 0,4 g lu, il y reste net.
- *Galette, mesuré.* 5,50 bar·s/g (5,84 à 8 h 08) pour 7,6 g au lieu de
  7,4 g : la dose plus forte ne ralentit pas la galette. Le remplissage
  admet 47,4 ml (50,6 ml à 8 h 08) et la goutte arrive 0,43 s avant le
  début de l'infusion.
- *Plateau, mesuré.* 8,78 bar, 0,05 bar au-dessus de 8 h 08 et toujours
  sous la consigne de 9 bar ; même commande (71–73 %), même lecture que
  le matin : l'intégrale n'a pas le temps de refermer l'écart.
- *Chauffe, mesuré.* À consigne, précharge et remplissage égaux, la NTC
  gagne +0,27 °C au remplissage au lieu de +2,62 °C, et la tasse
  2,54 °C de moins qu'à 8 h 08. Différence connue entre les deux : routine
  guidée le matin, infusion directe ici (sans rinçage guidé). Rien ne
  sépare les causes.

**Décision** (David). **MPB003 validée au goût ; réglages inchangés.**

| Point | Attendu (prochaine infusion, 0.5.8) |
| --- | --- |
| Balance | infusion guidée (tare automatique), ou tare à la main : 0,0 g au début de la capture |
| Dose | pesée ; ratio = final / dose pesée |
| Remplissage | fin sur la pression vers 0,9 bar, ≈ 5,5 ml/s |
| Hydraulique | 4,7 à 7,6 bar·s/g ; pic sous 9,4 bar ; plateau vers 8,7 à 8,8 bar |
| Arrêt | fermeture entre 0,4 et 0,7 s après la consigne à 0 ; chaudière fermée vers 6,7 bar |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,5 à 21,6 g final (gouttes vers 2 g) |
| Chauffe | tasse entre +2 et +4,6 °C sur la consigne |
| Tasse | goût noté |

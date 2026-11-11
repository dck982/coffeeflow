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

Profil actif « single espresso » : ViCafé, panier LM1 41 mm, **porte-filtre
sans fond**. Dernière configuration sauvegardée : `configs/261031-154935.json`
(schéma 13) ; les changements suivants sont lus dans les captures. Le profil
« turbo shot » et ses essais sont dans [turboshot.md](turboshot.md).
L'historique des réglages avant le 4 novembre est dans
`git show 7c17133:docs/journal-infusions.md`.

| Réglage | Valeur | Depuis |
| --- | --- | --- |
| Firmware | écran **0.4.28** (tuile de résistance au plateau), carte sensors 0.4.24 au dernier relevé | au plus tard le 11 novembre (0.4.27 du 1er au 10 novembre, 0.4.26 du 31 octobre) |
| Dose | **7,5 g**, pesée à chaque infusion | 30 octobre ; pesée depuis le 2 novembre |
| Mouture | un pas plus serré que du 5 au 6 novembre | 10 novembre |
| Offset NTC | **−14 °C** (écran 0.4.4), mesuré au panier avec la SCACE ; −10,5 °C avant. Consigne bornée à 95 °C, coupure à 100 °C utilisateur (114 °C sonde) | 10 octobre |
| Sonde de pression | XDB401 lu sur J10 de la carte sensors avec la calibration de A2, loi de chauffe inchangée | 23 octobre (A2 de l'écran depuis le 7 octobre) |
| Précharge | 8 s à 90 % | 4 octobre |
| Consigne | **90 °C** | 29 octobre, 14 h 21 (91 °C du 24 octobre ; 92 °C du 13 octobre ; 90 °C avant) |
| Remplissage | pompe à **70 %**, fin à 0,8 bar (10 s au plus) | 24 octobre, 10 h 22 (75 % avant) |
| Pré-infusion | pompe à 50 %, fin à 4 s ou au premier poids en tasse | 5 octobre, 7 h 59 |
| Infusion | pompe jusqu'à 100 %, cible 9 bar, arrêt à **20 g** ou 27 s, pompe arrêtée **1,0 g** avant la cible | cible depuis le 1er novembre, 13 h 31 ; anticipation depuis le 4 novembre après 9 h 44 (1,5 g du 31 octobre) |
| Purge | pompe à **80 %**, 10 s au plus | 24 octobre, 10 h 22 |
| Machine | fermée (panneaux posés), filtre ECM Aqua Aroma Crema dans le réservoir ; état final. Remplissage à 2,9 à 3,3 ml/s à 70 % | 10 octobre |

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
23 octobre au 3 novembre. Les chiffres de chaque infusion restent dans la
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


## Entrées

### 4 novembre, 9 h 44 — `261104-094440`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, **dose
pesée : 7,4 g** (réglage 7,5 g), cible 20 g, anticipation d'arrêt de
1,5 g, consigne de 90 °C. Écran 0.4.27, sensors 0.4.24. Premier single
espresso depuis les turbo shots du 2 et du 3/11 ; moulin ramené au cran
du single espresso, **canal du moulin purgé** avant la mouture. Point de rosée du salon non relevé. Observation de
David : **très bon**, **aucune amertume**, aucun goût de « carton / terre
/ cosse de café » ; arrêt **un peu tôt**.

**Routine** : guide ouvert à 84,9 °C ; rinçage de 5,0 s, 17,7 ml, 25,9 g,
**pression maximale 0,25 bar**. Consigne atteinte 201,3 s après le
rinçage, minimum 82,6 °C. **953 s sur le groupe** ; préparation de
216,4 s. Tasse de 117,6 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,00 s | — | 0,29 → 0,37 bar |
| Remplissage | 8,00 s | 7,17 s, pression | 22,3 ml (3,11 ml/s) | 0,37 → 0,85 bar |
| Pré-infusion | 15,18 s | **4,04 s, échéance** | 5,5 ml | 0,85 → 1,33 bar |
| Infusion | 19,21 s | 16,58 s | 28,7 ml | 9,41 bar max |

- Circuit à 0,29 bar au départ, comme avant le 2/11 (0,55 bar ce
  jour-là).
- **Première goutte (0,3 g) à 20,58 s, à 1,93 bar**, 30,1 ml, 1,4 s
  après le début de l'infusion : la pré-infusion finit sur l'échéance.
- 8 bar à 22,9 s, 1,8 g en tasse. **Pic à 9,41 bar** à 23,27 s, juste
  après la montée, puis plateau.
- Depuis 8 bar : **12,9 s**, **1,27 g/s** en tasse, 1,46 ml/s en amont.
  **18,5 g à l'arrêt de la pompe**, **19,7 g final**, **1,1 g de
  gouttes**. Pompe en marche 27,8 s. Goutte → arrêt de la pompe :
  **15,2 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,91 ± 0,05 bar**, pompe vers
  70 %, 1,30 g/s, soit **6,9 bar·s/g**, ou **7,0 ramené à 7,5 g** (6,4 le
  2/11 au matin).
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 04/11 9 h 44 | 2,9 s | 6,1 s | 10,3 s | 16,2 s |
  | 02/11 9 h 08 | 1,7 s | 4,7 s | 8,0 s | 14,0 s |

**Thermique.** NTC de 90,04 °C au départ, **+1,26 °C** pendant le
remplissage (22,3 ml), −1,06 °C pendant la pré-infusion ; 90,52 °C au
début de l'infusion, 89,62 °C à l'arrêt. Baisse pompe en marche
**−0,42 °C** (89,6 à 92,0 °C). **Tasse : 91,07 °C, +1,07 °C.**

**Attendus vérifiés** (entrée du 2/11 à 9 h 08).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | écart à 7,5 g nettement sous ±0,3 g après calibration | **7,4 g (−0,1 g)**, comme avant la calibration ; ratio 19,7 / 7,4 = **2,66** |
| Hydraulique | 5,8 à 7,0 bar·s/g ramené à 7,5 g ; plateau sous 9,2 bar | **7,0 : haut de plage** ; plateau 8,91 bar, pic transitoire de 9,41 bar |
| Rinçage | 0,1 à 0,7 bar | 0,25 bar : atteint |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml ; pression de départ | 3,11 ml/s, 22,3 ml : atteint ; départ à 0,37 bar |
| Rendement | arrêt vers 18,6 g, 19,8 à 20,5 g final | 18,5 g : atteint ; **19,7 g, 0,1 g sous la plage** |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | +1,26 °C ; −0,42 °C : atteint |
| Tasse | 89,0 à 92,5 °C | 91,07 °C : atteint |

**Lecture.**

- *Arrêt, mesuré.* 1,1 g arrive après la coupure, comme le 2/11 (1,1 g)
  à un débit voisin (1,27 et 1,35 g/s), soit ≈ 0,85 s de débit.
  L'anticipation de 1,5 g laisse donc ≈ 0,4 g sous la cible : c'est
  l'« arrêt un peu tôt ». Le firmware règle l'anticipation par pas de
  0,5 g : 1,0 g donne ≈ 20,1 g final, 1,5 g en donne 19,7.
- *Galette, mesuré.* 7,0 bar·s/g ramené à 7,5 g, en haut de la plage :
  la galette du matin la plus résistante depuis le 1/11 (6,9 et 6,4).
  La goutte, plus tardive, et le débit plus bas allongent l'écoulement
  (15,2 s de la goutte à l'arrêt, contre 13,4 s).
- *Purge, observé.* Premier matin avec purge du canal du moulin : pas de
  mouture retenue de la veille. La « petite amertume » du 1/11 et du 2/11
  au matin, sans purge, est absente ; un seul essai ne suffit pas à
  l'attribuer à la purge.
- *Goût et turbo shots, observé.* Même café (ViCafé) et même moulin que
  les turbo shots du panier 49 mm des 2 et 3/11, sans le goût de carton,
  de terre ou de cosse qu'ils avaient : le café n'en est pas la source.
  La lecture est dans [turboshot.md](turboshot.md).

**Décision** (David). **Anticipation d'arrêt de 1,0 g** (1,1 g refusé
par le firmware, pas de 0,5 g), cible 20 g
inchangée ; dose, mouture et 90 °C inchangés. Turbo shots en pause
jusqu'à la pompe à engrenages.

| Point | Attendu (prochaine infusion, anticipation de 1,0 g) |
| --- | --- |
| Dose | pesée ; écart à 7,5 g sous ±0,3 g ; ratio = final / dose pesée |
| Hydraulique | 5,8 à 7,0 bar·s/g ramené à 7,5 g ; plateau sous 9,2 bar |
| Rinçage | 0,1 à 0,7 bar |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml ; noter la pression de départ |
| Rendement | arrêt de la pompe vers 19,0 g, **19,9 à 20,4 g final**, ratio ≈ 2,7 |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche |
| Tasse | 89,0 à 92,5 °C |

### 4 novembre, 13 h 48 — `261104-134842`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, **dose
supposée de 7,5 g** (réglage, non pesée), cible 20 g, **anticipation d'arrêt
de 1,0 g** (1,1 g refusé par le firmware), consigne de 90 °C, cran du
single espresso. Écran 0.4.27, sensors 0.4.24. Salon (Netatmo) : 44 %,
point de rosée **9,8 °C** (10,2 °C le 2/11 au matin) ; extérieur : 74 %,
point de rosée 5,7 °C. Observation de David : **très bon shot**, **le bon
poids** cette fois ; galette **beaucoup plus « soupe »** après
l'infusion que celle de 9 h 44.

**Routine** : guide ouvert à 88,8 °C ; rinçage de 5,0 s, 16,8 ml, 20,4 g,
**pression maximale 0,20 bar**. Consigne atteinte 84,7 s après le
rinçage, minimum 83,5 °C. **951 s sur le groupe** ; préparation de
154,7 s. Tasse de 115,4 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | 0,25 → 0,33 bar |
| Remplissage | 8,01 s | 8,70 s, pression | 26,4 ml (3,03 ml/s) | 0,33 → 0,94 bar |
| Pré-infusion | 16,71 s | **4,01 s, échéance** | 5,2 ml | 0,94 → 1,36 bar |
| Infusion | 20,72 s | 16,77 s | 29,3 ml | 9,26 bar max |

- **Première goutte (0,3 g) à 21,37 s, à 1,55 bar**, 32,5 ml, 0,65 s
  après le début de l'infusion.
- 8 bar à 24,46 s, 2,8 g en tasse. **Pic à 9,26 bar** à 24,74 s, juste
  après la montée (9,41 bar le matin), puis plateau.
- Depuis 8 bar : **13,0 s**, **1,21 g/s** en tasse, 1,45 ml/s en amont.
  **≈ 19,0 g à l'arrêt de la pompe**, **20,0 g final**, 1,0 à 1,1 g de
  gouttes. Pompe en marche 29,4 s. Goutte → arrêt de la pompe :
  **16,1 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,99 ± 0,05 bar**, pompe à
  68–72 %, 1,20 g/s, soit **7,5 bar·s/g** à la dose réglée (6,9 le matin,
  6,1 le 1/11 après-midi).
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 04/11 13 h 48 | 1,9 s | 5,4 s | 9,6 s | 16,0 s |
  | 04/11 9 h 44 | 2,9 s | 6,1 s | 10,3 s | 16,2 s |
  | 02/11 9 h 08 | 1,7 s | 4,7 s | 8,0 s | 14,0 s |

**Thermique.** NTC de 90,11 °C au départ, **+0,95 °C** pendant le
remplissage (26,4 ml), +0,15 °C pendant la pré-infusion ; 91,38 °C au
début de l'infusion, 88,98 °C à l'arrêt. Baisse pompe en marche
**−1,06 °C** (89,0 à 92,0 °C). **Tasse : 90,82 °C, +0,82 °C.**

**Attendus vérifiés** (entrée de 9 h 44, anticipation de 1,0 g).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée | **non pesée**, supposée de 7,5 g ; ratio **2,67** |
| Hydraulique | 5,8 à 7,0 bar·s/g ramené à 7,5 g ; plateau sous 9,2 bar | **7,5 à la dose réglée : au-dessus de la plage** ; plateau 8,99 bar, pic transitoire de 9,26 bar |
| Rinçage | 0,1 à 0,7 bar | 0,20 bar : atteint |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml ; pression de départ | 3,03 ml/s, 26,4 ml : atteint ; départ à 0,33 bar |
| Rendement | arrêt vers 19,0 g, 19,9 à 20,4 g final | ≈ 19,0 g ; **20,0 g : atteint** |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | +0,95 °C ; −1,06 °C : atteint |
| Tasse | 89,0 à 92,5 °C | 90,82 °C : atteint |

**Lecture.**

- *Arrêt, mesuré.* L'anticipation de 1,0 g donne 20,0 g pour 20 g visés :
  ≈ 1 g arrive après la coupure à 1,2 g/s, comme le matin.
- *Galette, mesuré.* 7,5 bar·s/g, au-dessus de la plage de 5,8 à 7,0 et
  en hausse sur trois infusions (6,4 le 2/11 au matin, 6,9 ce matin) ;
  l'après-midi était jusqu'ici plus rapide que le matin (6,1 le 1/11).
  Sans la dose pesée, la part de la dose ne se sépare pas de celle de la
  mouture : 0,3 g de plus n'expliquerait que +4 % (Darcy, estimé), pas
  les +9 % mesurés. Le goût, très bon, ne demande pas de correction.
- *Hypothèses de David, non testées.* (1) Le moulin, revenu de trois
  crans plus gros (turbo du 3/11) au cran du single espresso, met une ou
  deux infusions à se stabiliser : la résistance monterait alors vers sa
  valeur et s'arrêterait. (2) L'humidité : le point de rosée du salon a
  baissé de 0,4 °C depuis le 2/11, une variation faible. La prochaine
  infusion les départage en partie : une résistance stable vers 7,5 va
  dans le sens de (1), une poursuite de la hausse demande une autre cause.
- *Galette « soupe », observé.* Plus humide et moins tenue qu'à 9 h 44
  alors que la résistance est plus haute ; aucune mesure de la capture ne
  s'y rattache (récupération : 8,95 → 8,98 bar sur 10 s, 8,89 → 8,42
  bar à 9 h 44).

**Décision** (David). Réglages inchangés (7,5 g, cible 20 g, anticipation
de 1,0 g, 90 °C) ; **dose pesée systématiquement**.

| Point | Attendu (prochaine infusion, mêmes réglages) |
| --- | --- |
| Dose | pesée ; ratio = final / dose pesée |
| Hydraulique | 5,8 à 7,5 bar·s/g ramené à 7,5 g ; plateau sous 9,2 bar ; au-delà de 7,5, noter la tendance |
| Rinçage | 0,1 à 0,7 bar |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml |
| Rendement | arrêt de la pompe vers 19,0 g, 19,9 à 20,4 g final |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche |
| Tasse | 89,0 à 92,5 °C |

### 5 novembre, 8 h 54 — `261105-085428`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, **dose
pesée : 7,5 g**, cible 20 g, anticipation d'arrêt de
1,0 g, consigne de 90 °C, cran du single espresso. **Canal du moulin
non purgé.** Écran 0.4.27, sensors 0.4.24. Point de rosée du
salon non relevé ; boîtier de l'écran à 34,8 °C et 33,6 %. Observation de
David : **un peu d'amertume revenue**.

**Routine** : guide ouvert à 89,9 °C ; rinçage de 5,0 s, 18,0 ml, 27,6 g,
**pression maximale 0,22 bar**. Consigne déjà atteinte à la fin du
rinçage, minimum 84,5 °C ensuite. **886 s sur le groupe** ; préparation de
183,4 s. Tasse de 124,9 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,05 s | — | 0,30 → 0,39 bar |
| Remplissage | 8,05 s | 7,31 s, pression | 22,3 ml (3,05 ml/s) | 0,40 → 0,91 bar |
| Pré-infusion | 15,36 s | **1,75 s, poids** | 2,6 ml | 0,91 → 1,06 bar |
| Infusion | 17,11 s | 17,61 s | 29,0 ml | 9,02 bar max |

- **La pré-infusion finit sur le poids**, pas sur l'échéance de 4 s comme
  le 4/11 : 0,1 g en tasse à 24,9 ml.
- **Première goutte (0,3 g) à 17,87 s, à 1,23 bar**, **26,1 ml** (30,1
  et 32,5 ml le 4/11), 0,76 s après le début de l'infusion.
- 8 bar à 21,19 s (4,1 s après le début de l'infusion, 3,7 s le 4/11),
  3,2 g en tasse. **Pas de pic transitoire** : le maximum, 9,02 bar,
  tombe à 33,77 s, en fin de plateau (9,41 et 9,26 bar le 4/11).
- Depuis 8 bar : **13,5 s**, **1,20 g/s** en tasse, 1,26 ml/s en amont.
  **19,4 g à l'arrêt de la pompe**, **20,4 g final**, **1,0 g de
  gouttes**. Pompe en marche 26,5 s. Goutte → arrêt de la pompe :
  **16,9 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,87 ± 0,07 bar**, pompe à
  64–68 %, 1,16 g/s, soit **7,7 bar·s/g**.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 05/11 8 h 54 | 1,6 s | 5,5 s | 9,8 s | 16,7 s |
  | 04/11 13 h 48 | 1,9 s | 5,4 s | 9,6 s | 16,0 s |
  | 04/11 9 h 44 | 2,9 s | 6,1 s | 10,3 s | 16,2 s |

**Thermique.** NTC de 90,11 °C au départ, **+1,24 °C** pendant le
remplissage (22,3 ml), +0,03 °C pendant la pré-infusion ; 91,56 °C au
début de l'infusion, 89,54 °C à l'arrêt. Baisse pompe en marche
**−0,53 °C** (89,6 à 91,7 °C). **Tasse : 90,81 °C, +0,81 °C.**

**Attendus vérifiés** (entrée du 4/11 à 13 h 48, mêmes réglages).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée | 7,5 g ; ratio **2,72** |
| Hydraulique | 5,8 à 7,5 bar·s/g ramené à 7,5 g ; plateau sous 9,2 bar ; au-delà de 7,5, noter la tendance | **7,7 : au-delà**, la hausse continue ; plateau 8,87 bar : atteint |
| Rinçage | 0,1 à 0,7 bar | 0,22 bar : atteint |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml | 3,05 ml/s, 22,3 ml : atteint |
| Rendement | arrêt de la pompe vers 19,0 g, 19,9 à 20,4 g final | 19,4 g ; **20,4 g : atteint**, en haut de la plage |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | +1,24 °C ; −0,53 °C : atteint |
| Tasse | 89,0 à 92,5 °C | 90,81 °C : atteint |

**Lecture.**

- *Galette, mesuré.* 7,7 bar·s/g, quatrième hausse de suite (6,4 le 2/11
  au matin, 6,9 et 7,5 le 4/11), mais la plus faible : +3 %, contre +8 %
  et +9 %. Cela va plutôt dans le sens de l'hypothèse (1) du 4/11, un
  moulin qui se stabilise après les turbo shots, sans l'établir : le
  canal n'était pas purgé, et la mouture de la veille restée dans le
  moulin fait partie de la dose.
- *Amertume, observé.* Elle revient sur la galette la plus résistante de
  la série, ce qui est cohérent avec une extraction plus poussée. La
  résistance seule ne l'explique pas : le 2/11 au matin, à 6,4, il y
  avait déjà une « petite amertume », et le 4/11 à 9 h 44, à 7,0, aucune.
  La purge du canal du moulin suit l'amertume du matin : absente le 1/11,
  le 2/11 et ce matin (amertume), présente le 4/11 à 9 h 44 (aucune).
- *Pré-infusion, mesuré.* La première goutte arrive après 26,1 ml, 4 à
  6 ml plus tôt que le 4/11, et met fin à la pré-infusion sur le poids
  après 1,75 s. Une galette plus résistante mouillée moins longtemps : ce
  n'est pas l'ordre attendu, et une seule infusion ne dit pas si c'est un
  passage préférentiel ou la préparation de la galette.
- *Arrêt, mesuré.* 1,0 g après la coupure à 1,2 g/s, comme les deux
  infusions précédentes : l'anticipation de 1,0 g tient.

**Décision** (David). **Mouture d'un cran plus grosse.** Dose, cible 20 g,
anticipation de 1,0 g et 90 °C inchangés.

*Un cran plus gros, estimé.* Le dernier cran plus fin a donné +12 % à la
première infusion (31/10, 15 h 10), le journal estimait ≈ +24 % pour un
cran entier une fois la rétention du moulin écoulée : un cran plus gros
ramènerait 7,7 vers
**6,2 à 6,9 bar·s/g**, et la première infusion peut ne montrer qu'une
partie de l'écart si le canal n'est pas purgé. À débit plus élevé
(≈ 1,3 g/s), la queue d'écoulement reste vers 1,0 à 1,1 g.

| Point | Attendu (prochaine infusion, un cran plus gros) |
| --- | --- |
| Dose | pesée ; ratio = final / dose pesée ; canal du moulin purgé |
| Hydraulique | **6,2 à 6,9 bar·s/g** ramené à 7,5 g (≈ 12 à 13 s depuis 8 bar, ≈ 1,3 g/s) ; plateau sous 9,2 bar ; vers 7,5, le cran n'a pas encore pris |
| Rinçage | 0,1 à 0,7 bar |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml |
| Rendement | arrêt de la pompe vers 19,0 g, 19,9 à 20,4 g final |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche |
| Tasse | 89,0 à 92,5 °C |

### 5 novembre, 9 h 11 — `261105-091115`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, **dose
pesée : 7,4 g** (réglage 7,5 g ; le moulin donne ± 0,1 g selon David),
cible 20 g, anticipation d'arrêt de 1,0 g, consigne de 90 °C, **mouture
d'un cran plus grosse** (décision de 8 h 54), **7,5 g purgés** après le
changement de cran (d'habitude, David ne purge qu'avant la première
infusion du matin). Écran 0.4.27, sensors
0.4.24. Point de rosée du salon non relevé ; boîtier de l'écran à 36,7 °C
et 32,6 %. Observation de David : **beaucoup plus rapide**, **plus fade**.

**Routine** : guide ouvert à 90,1 °C ; rinçage de 5,0 s, 18,0 ml, 27,0 g,
**pression maximale 0,35 bar**. Consigne déjà atteinte à la fin du
rinçage, minimum 85,1 °C ensuite. **522 s sur le groupe** ; préparation de
126,7 s. Tasse de 125,2 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,10 s | — | 0,28 → 0,40 bar |
| Remplissage | 8,10 s | 7,88 s, pression | 24,6 ml (3,13 ml/s) | 0,40 → 0,80 bar |
| Pré-infusion | 15,97 s | **4,02 s, échéance** | 5,5 ml | 0,80 → 1,25 bar |
| Infusion | 19,99 s | 12,93 s | 27,2 ml | 8,83 bar max |

- **Première goutte (0,3 g) à 21,19 s, à 1,69 bar**, 32,2 ml, 1,2 s
  après le début de l'infusion.
- 8 bar à 24,07 s (4,1 s après le début de l'infusion), 3,7 g en tasse.
  Pas de pic transitoire : maximum de 8,83 bar à 32,56 s.
- Depuis 8 bar : **9,0 s**, **1,74 g/s** en tasse, 1,81 ml/s en amont.
  **19,3 g à l'arrêt de la pompe**, **20,6 g final**, **1,3 g de
  gouttes**. Pompe en marche 25,0 s. Goutte → arrêt de la pompe :
  **11,9 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,69 ± 0,09 bar**, pompe à
  70–73 %, 1,78 g/s, soit **4,9 bar·s/g**, ramené à 7,5 g comme à 7,4 g.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 05/11 9 h 11 | 2,3 s | 4,8 s | 7,8 s | 12,3 s |
  | 05/11 8 h 54 | 1,6 s | 5,5 s | 9,8 s | 16,7 s |
  | 31/10 14 h 06 (7,5 g) | 1,7 s | 4,4 s | 7,2 s | 12,7 s |

**Thermique.** NTC de 90,09 °C au départ, **+0,76 °C** pendant le
remplissage (24,6 ml), −1,43 °C pendant la pré-infusion ; 89,69 °C au
début de l'infusion, 89,52 °C à l'arrêt. Baisse pompe en marche
**−0,61 °C** (89,5 à 91,7 °C). **Tasse : 90,26 °C, +0,26 °C.**

**Attendus vérifiés** (entrée de 8 h 54, un cran plus gros).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée ; canal du moulin purgé | 7,4 g ; ratio **2,78** ; 7,5 g purgés après le changement de cran |
| Hydraulique | 6,2 à 6,9 bar·s/g ramené à 7,5 g (≈ 12 à 13 s depuis 8 bar, ≈ 1,3 g/s) ; plateau sous 9,2 bar | **4,9 bar·s/g, 9,0 s, 1,74 g/s : bien en dessous** ; plateau 8,69 bar : atteint |
| Rinçage | 0,1 à 0,7 bar | 0,35 bar : atteint |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml | 3,13 ml/s, 24,6 ml : atteint |
| Rendement | arrêt de la pompe vers 19,0 g, 19,9 à 20,4 g final | 19,3 g ; **20,6 g, 0,2 g au-dessus** |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | +0,76 °C ; −0,61 °C : atteint |
| Tasse | 89,0 à 92,5 °C | 90,26 °C : atteint |

**Lecture.**

- *Galette, mesuré.* De 7,7 à 4,9 bar·s/g : **−36 %** pour un cran, bien
  plus que les 12 à 24 % estimés. La dose n'y est pour rien : 0,1 g de
  moins vaut ≈ −1 %, dans la précision du moulin.
- *Retour au cran du 31/10 à 14 h 06, estimé.* Le cran du single espresso
  est celui de 15 h 10, un cran plus fin que 14 h 06 ; un cran plus gros y
  ramène. 14 h 06 donnait **5,0 bar·s/g, 8,4 s, 1,76 g/s**, « toujours
  trop rapide » ; ce matin, 4,9, 9,0 s, 1,74 g/s, « plus fade ». Les deux
  infusions se superposent, arrivée en tasse comprise.
- *8 h 54 surestimé, hypothèse.* Si ce cran vaut ≈ 5 et le précédent
  ≈ 6 à 7 (6,1 à 7,5 du 31/10 au 4/11), les 7,7 de 8 h 54 tiennent en
  partie à la mouture de la veille restée dans le moulin, sans purge.
  L'écart de ce matin mêle alors le cran et la purge manquante.
- *Convergence du moulin, observé (David).* Une purge de 7,5 g après un
  changement de cran ne suffit peut-être pas : il semble falloir bien
  plus de 7 g pour que la mouture corresponde au cran. La dose de 9 h 11
  peut donc encore contenir de la mouture du cran précédent.
- *Arrêt, mesuré.* 1,3 g de gouttes à 1,74 g/s (≈ 0,75 s de débit) contre
  1,0 g à 1,2 g/s : le débit plus élevé allonge la queue, d'où 20,6 g.
- *Préparation.* La balance a été perdue 2,2 s après le départ, pendant la
  chauffe, sans trou dans la mesure du poids.

**Décision** (David). **Même cran, dose de 7,5 g** ; cible 20 g,
anticipation de 1,0 g et 90 °C inchangés.

*Estimé.* 0,1 g de plus vaut ≈ +1 à +2 % (Darcy), sous la dispersion de
préparation (± 18 %) : l'infusion suivante mesurera surtout la
répétabilité du cran. Si une part de la dose de 9 h 11 venait encore du
cran précédent malgré la purge de 7,5 g, la suivante peut être un peu
plus rapide.

| Point | Attendu (prochaine infusion, même cran, 7,5 g) |
| --- | --- |
| Dose | pesée ; ratio = final / dose pesée ; noter la purge éventuelle |
| Hydraulique | **4,4 à 5,4 bar·s/g** (≈ 8 à 10 s depuis 8 bar, ≈ 1,6 à 1,9 g/s) ; plateau sous 9,2 bar |
| Rinçage | 0,1 à 0,7 bar |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml |
| Rendement | arrêt de la pompe vers 19,3 g, 20,3 à 20,8 g final |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche |
| Tasse | 89,0 à 92,5 °C |

### 5 novembre, 14 h 31 — `261105-143133`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, même cran
qu'à 9 h 11 (un cran plus gros que le single espresso du 31/10 à
15 h 10), cible 20 g, anticipation d'arrêt de 1,0 g, consigne de 90 °C.
**Dose : le moulin a donné 7,2 g pour 7,5 g réglés**, complétée à
7,5 g ; écart de −0,3 g (−4 %), trois fois la précision de ± 0,1 g
admise le matin. **Canal du moulin non purgé** : l'après-midi, David
ne purge que s'il change de cran. Écran 0.4.27, sensors
0.4.24. Point de rosée du salon non relevé ; boîtier de l'écran à
29,0 °C et 39,4 %. Observation de David : **bon**, **peut-être un peu
trop rapide**.

**Routine** : guide ouvert à 85,6 °C ; rinçage de 5,0 s, 18,0 ml, 22,8 g,
**pression maximale 0,14 bar**. Consigne atteinte 495,4 s après le
rinçage, minimum 80,0 °C ; porte-filtre en place 87,9 s après la
consigne, soit **583 s sur le groupe** ; préparation de 133,8 s. Tasse de
115,4 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,03 s | — | 0,44 → 0,50 bar |
| Remplissage | 8,03 s | 5,71 s, pression | 17,4 ml (3,05 ml/s) | 0,50 → 0,90 bar |
| Pré-infusion | 13,74 s | **2,86 s, poids** | 4,3 ml | 0,90 → 1,19 bar |
| Infusion | 16,59 s | 15,49 s | 28,7 ml | 9,03 bar max |

- **Remplissage court** : 17,4 ml, en bas de la plage, sur une précharge
  partie de 0,44 bar (0,28 à 0,40 bar le matin).
- **Première goutte (0,3 g) à 16,68 s, à 1,17 bar**, **21,7 ml** (32,2 ml
  à 9 h 11, 26,1 ml à 8 h 54), dès le début de l'infusion ; la
  pré-infusion finit sur le poids, comme à 8 h 54.
- 8 bar à 20,91 s (4,3 s après le début de l'infusion), 4,1 g en tasse.
  Pas de pic transitoire : maximum de 9,03 bar à 28,87 s.
- Depuis 8 bar : **11,2 s**, **1,32 g/s** en tasse, 1,43 ml/s en amont.
  **19,2 g à l'arrêt de la pompe**, **20,4 g final**, **1,2 g de
  gouttes**. Pompe en marche 23,8 s. Goutte → arrêt de la pompe :
  **15,4 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,85 ± 0,08 bar**, pompe à
  65–69 %, 1,30 g/s, soit **6,8 bar·s/g**.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 05/11 14 h 31 | 1,8 s | 4,8 s | 8,5 s | 14,5 s |
  | 05/11 9 h 11 | 2,3 s | 4,8 s | 7,8 s | 12,3 s |
  | 05/11 8 h 54 | 1,6 s | 5,5 s | 9,8 s | 16,7 s |

**Thermique.** NTC de 90,03 °C au départ, **+2,21 °C** pendant le
remplissage (17,4 ml), −0,61 °C pendant la pré-infusion ; 91,75 °C au
début de l'infusion, 87,71 °C à l'arrêt. Baisse pompe en marche
**−2,25 °C** (87,8 à 92,4 °C). Chauffe presque nulle pendant
l'infusion : 0,38 kJ commandés, 0,49 kJ au SSR. **Tasse : 90,41 °C,
+0,41 °C.**

**Attendus vérifiés** (entrée de 9 h 11, même cran, 7,5 g).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée ; ratio = final / dose pesée ; noter la purge éventuelle | 7,2 g moulus, complétés à 7,5 g ; ratio **2,72** ; pas de purge, même cran que le matin |
| Hydraulique | 4,4 à 5,4 bar·s/g (≈ 8 à 10 s depuis 8 bar, ≈ 1,6 à 1,9 g/s) ; plateau sous 9,2 bar | **6,8 bar·s/g, 11,2 s, 1,32 g/s : bien au-dessus** ; plateau 8,85 bar : atteint |
| Rinçage | 0,1 à 0,7 bar | 0,14 bar : atteint |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml | 3,05 ml/s, 17,4 ml : atteint, en bas de la plage |
| Rendement | arrêt de la pompe vers 19,3 g, 20,3 à 20,8 g final | 19,2 g ; 20,4 g : atteint |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | +2,21 °C : atteint, en haut ; **−2,25 °C : en dessous** |
| Tasse | 89,0 à 92,5 °C | 90,41 °C : atteint |

**Lecture.**

- *Galette, mesuré.* 6,8 bar·s/g au même cran que les 4,9 de 9 h 11 :
  **+39 %**, deux fois la dispersion de préparation (± 18 %). La dose
  n'y est pour rien : 7,5 g contre 7,4 g, ≈ +1 à +2 % (Darcy).
- *9 h 11 hors série, hypothèse.* Rapporté aux 7,7 de 8 h 54, 6,8 donne
  **−12 % pour un cran**, le bas de la plage estimée avant 9 h 11 (12 à
  24 %, soit 6,2 à 6,9 bar·s/g) ; les −36 % de 9 h 11 en sortent. Le
  sens contredit aussi la rétention : un reste du cran plus fin aurait
  ralenti 9 h 11, pas accéléré. Deux infusions au même cran ne disent pas
  laquelle donne la valeur du cran.
- *Dose du moulin, mesuré.* −0,3 g sur 7,5 g réglés, contre ± 0,1 g le
  matin. Sans pesée, c'est −4 % de café et ≈ −6 % de résistance
  (estimé) ; la pesée suivie d'un complément le corrige. Le complément de
  0,3 g, versé sur le dessus, peut changer la répartition dans la galette
  (hypothèse non testée).
- *Goût, observé.* « Bon, peut-être un peu trop rapide » à 6,8, entre les
  4,9 « plus fades » de 9 h 11 et les 7,5 à 7,7 du cran plus fin, avec le
  « très bon » du 4/11 à 13 h 48 (7,5) et l'amertume revenue ce matin
  (7,7).
- *Chauffe, mesuré.* Le remplissage a porté la NTC à 92,36 °C, 2,4 °C
  au-dessus de la consigne ; la loi a coupé la chauffe pendant
  l'infusion, d'où −2,25 °C pompe en marche, sous la plage. La tasse
  reste à 90,41 °C, dans la sienne.
- *Porte-filtre peu chauffé, hypothèse de David.* Le porte-filtre est
  resté **87,9 s** sur le groupe après la consigne atteinte, contre 443 à
  953 s pour les sept infusions du 1/11 au matin de ce jour ; la chaudière
  venait de remonter de 80,0 °C, le minimum le plus bas de la série. Un
  groupe et une chaudière pas encore à l'équilibre iraient dans le sens
  d'une NTC qui grimpe au remplissage puis retombe pompe en marche ; une
  seule infusion ne l'établit pas.
- *Première goutte précoce, observé.* 21,7 ml, le plus tôt de la série,
  sur une galette plus résistante que celle de 9 h 11 ; avec 8 h 54
  (26,1 ml, 7,7), cela fait deux fois que la goutte arrive tôt quand la
  résistance est haute. Aucune cause établie.

**Décision** (David). **Réglages inchangés** : même cran, 7,5 g pesés et
complétés au besoin, cible 20 g, anticipation de 1,0 g, 90 °C. Pas de
purge l'après-midi sans changement de cran.

*Estimé.* La prochaine infusion départage 9 h 11 et 14 h 31 : vers 6,8,
le cran vaut ≈ −12 % sur le précédent et 9 h 11 sort de la série ; vers
5, c'est 14 h 31 qui sort. Avec un porte-filtre resté plus longtemps
après la consigne, la baisse pompe en marche devrait revenir dans la
plage.

| Point | Attendu (prochaine infusion, mêmes réglages) |
| --- | --- |
| Dose | pesée avant complément ; ratio = final / dose pesée ; noter la purge |
| Hydraulique | **6,1 à 7,5 bar·s/g** (≈ 10,5 à 12,5 s depuis 8 bar, ≈ 1,2 à 1,4 g/s) ; plateau sous 9,2 bar ; vers 5, c'est 9 h 11 qui donnait la valeur du cran |
| Rinçage | 0,1 à 0,7 bar |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml |
| Rendement | arrêt de la pompe vers 19,2 g, 20,2 à 20,6 g final |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche ; noter le temps du porte-filtre après la consigne |
| Tasse | 89,0 à 92,5 °C |

### 6 novembre, 7 h 39 — `261106-073933`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, même cran
que le 5/11 à 9 h 11 et 14 h 31, cible 20 g, anticipation d'arrêt de
1,0 g, consigne de 90 °C. **Dose : le moulin a donné 7,4 g pour 7,5 g
réglés**, complétée à 7,5 g. **Canal du moulin non purgé**, première
infusion du matin : la mouture retenue date de la veille, au même cran.
Écran 0.4.27, sensors 0.4.24. Salon (Netatmo) : 47 %, point de rosée
**10,7 °C** (9,8 °C le 4/11 après-midi) ; extérieur : 100 %, point de
rosée 7,3 °C ; boîtier de l'écran à 35,2 °C et 31,7 %. Observation de
David : **un côté amer** ; le café **a mis plus longtemps que d'habitude
à sortir sous le panier**.

**Routine** : guide ouvert à 88,1 °C ; rinçage de 5,0 s, 14,8 ml, 18,5 g,
**pression maximale 0,18 bar**. Consigne atteinte 78,4 s après le
rinçage, minimum 84,4 °C. **941 s sur le groupe** ; préparation de
148,6 s. Tasse de 125,1 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | 0,21 → 0,31 bar |
| Remplissage | 8,01 s | **10,02 s, échéance** | 29,6 ml (2,95 ml/s) | 0,31 → **0,68 bar** |
| Pré-infusion | 18,03 s | **4,07 s, échéance** | 5,5 ml | 0,68 → 0,84 bar |
| Infusion | 22,10 s | 17,67 s | 29,9 ml | 9,47 bar max |

- **Le remplissage finit sur l'échéance de 10 s**, à 0,68 bar, sans
  atteindre le seuil de 0,8 bar : il finissait sur la pression, en 5,7 à
  8,7 s, dans les entrées du 4/11 et du 5/11. 29,6 ml, au-dessus de la
  plage de 17 à 28 ml, sur une précharge partie de 0,21 bar.
- **Première goutte (0,3 g) à 23,67 s, à 1,50 bar**, **38,3 ml**, la plus
  tardive de la série (21,7 ml le 5/11 à 14 h 31, 32,5 ml le 4/11 à
  13 h 48), 1,57 s après le début de l'infusion : c'est le café « plus
  long à sortir » vu par David.
- 8 bar à 25,48 s (3,4 s après le début de l'infusion), 2,2 g en tasse.
  **Pic à 9,47 bar** à 25,84 s, juste après la montée, comme le 4/11
  (9,41 et 9,26 bar), puis plateau.
- Depuis 8 bar : **14,3 s**, **1,18 g/s** en tasse, 1,44 ml/s en amont.
  **19,0 g à l'arrêt de la pompe**, **20,0 g final**, **1,0 g de
  gouttes**. Pompe en marche 31,8 s. Goutte → arrêt de la pompe :
  **16,1 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **9,02 ± 0,06 bar**, pompe à
  67–71 %, 1,19 g/s, soit **7,6 bar·s/g**.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 06/11 7 h 39 | 2,6 s | 6,0 s | 10,6 s | 17,0 s |
  | 05/11 14 h 31 | 1,8 s | 4,8 s | 8,5 s | 14,5 s |
  | 05/11 9 h 11 | 2,3 s | 4,8 s | 7,8 s | 12,3 s |
  | 05/11 8 h 54 | 1,6 s | 5,5 s | 9,8 s | 16,7 s |

**Thermique.** NTC de 90,02 °C au départ, **+0,04 °C** pendant le
remplissage (29,6 ml), −0,57 °C pendant la pré-infusion ; 89,91 °C au
début de l'infusion, 89,48 °C à l'arrêt. Baisse pompe en marche
**−0,54 °C** (89,5 à 91,4 °C). **Tasse : 90,53 °C, +0,53 °C.**

**Attendus vérifiés** (entrée du 5/11 à 14 h 31, mêmes réglages).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée avant complément ; ratio = final / dose pesée ; noter la purge | 7,4 g moulus, complétés à 7,5 g ; ratio **2,67** ; pas de purge |
| Hydraulique | 6,1 à 7,5 bar·s/g (≈ 10,5 à 12,5 s depuis 8 bar, ≈ 1,2 à 1,4 g/s) ; plateau sous 9,2 bar ; vers 5, c'est 9 h 11 qui donnait la valeur du cran | **7,6 bar·s/g, 14,3 s, 1,18 g/s : juste au-dessus** ; plateau 9,02 bar : atteint, pic transitoire de 9,47 bar |
| Rinçage | 0,1 à 0,7 bar | 0,18 bar : atteint |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml | 2,95 ml/s : atteint ; **29,6 ml, au-dessus**, fin sur l'échéance |
| Rendement | arrêt de la pompe vers 19,2 g, 20,2 à 20,6 g final | 19,0 g ; **20,0 g, 0,2 g sous la plage** |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche ; noter le temps du porte-filtre après la consigne | **+0,04 °C : en dessous** ; −0,54 °C : atteint ; 941 s sur le groupe |
| Tasse | 89,0 à 92,5 °C | 90,53 °C : atteint |

**Lecture.**

- *Variabilité au même cran, mesuré.* Trois infusions au même cran, à
  7,4–7,5 g : **4,9**, **6,8** et **7,6 bar·s/g**, un écart de **+55 %**
  entre les extrêmes. La dose n'y est pour rien (≈ 1 à 2 %, Darcy) et
  l'écart dépasse largement la dispersion de préparation (± 18 %) : à
  cran fixe, la galette varie plus que ce que la dose et la préparation
  expliquent. C'est la preuve que David attendait, avec une réserve :
  l'écart tient surtout à 9 h 11 ; 14 h 31 et ce matin ne diffèrent que de
  +12 %, dans la dispersion de préparation.
- *Ce que l'écart ne départage pas, hypothèse.* Deux sources restent
  possibles : la mouture elle-même qui varie à cran fixe, ou la mouture
  retenue dans le moulin. La rétention d'un autre cran est exclue ce
  matin (même cran depuis 9 h 11) ; une mouture retenue toute la nuit
  peut encore se comporter autrement (humidité reprise, fines tassées),
  sans mesure qui l'établisse. 9 h 11, le plus rapide, avait suivi une
  purge de 7,5 g ; les deux plus résistantes, 8 h 54 (7,7) et ce matin
  (7,6), sont des premières infusions du matin sans purge.
- *Galette lente à mouiller, mesuré.* Le remplissage n'atteint pas
  0,8 bar en 10 s et la goutte arrive après 38,3 ml, 16,6 ml plus tard
  qu'hier après-midi : à basse pression, la galette absorbe davantage
  avant de laisser passer. À 9 bar, elle résiste pourtant plus. Le lien
  « goutte précoce quand la résistance est haute » noté le 5/11 (8 h 54,
  14 h 31) ne tient donc pas ce matin.
- *Amertume, observé.* Elle revient sur une première infusion du matin
  sans purge, comme le 1/11, le 2/11 et le 5/11 à 8 h 54 ; le seul matin
  purgé (4/11, 9 h 44) n'en avait aucune. La résistance seule ne
  l'explique pas : le 4/11 à 13 h 48, à 7,5 bar·s/g et sans purge
  l'après-midi, était « très bon ». Le matin sans purge reste le meilleur
  prédicteur de l'amertume, avec un seul contre-essai.
- *Arrêt, mesuré.* 1,0 g après la coupure à 1,19 g/s, comme les infusions
  à débit voisin : l'anticipation de 1,0 g tient.
- *Chauffe, mesuré.* Le porte-filtre est resté 941 s sur le groupe ; la
  baisse pompe en marche revient dans la plage (−2,25 °C hier
  après-midi, après 87,9 s), comme le prévoyait l'hypothèse de David. Au
  remplissage, +0,04 °C seulement, sur le volume le plus grand de la
  série (29,6 ml) : davantage d'eau froide pour la même chauffe, estimé.
- *Humidité, observé.* Point de rosée du salon à 10,7 °C, 0,9 °C au-dessus
  du 4/11 après-midi ; la variation est faible devant l'écart de
  résistance, et une seule valeur ne dit rien de la mouture retenue.

**Décision** (David). **Purge du canal du moulin demain matin**, avant la
première infusion ; même cran, 7,5 g pesés et complétés au besoin, cible
20 g, anticipation de 1,0 g, 90 °C.

*Estimé.* Si la mouture retenue la nuit fait la résistance du matin, la
galette purgée revient vers celle de 9 h 11 et 14 h 31 (4,9 à 6,8, soit
≈ 1,3 à 1,8 g/s) et l'amertume disparaît. Vers 7,6 avec amertume, la
purge n'y est pour rien et c'est le moulin qui varie à cran fixe. Un
débit plus élevé allonge la queue d'écoulement (1,3 g à 1,74 g/s le 5/11).

| Point | Attendu (demain matin, purge, mêmes réglages) |
| --- | --- |
| Dose | pesée avant complément ; ratio = final / dose pesée ; masse purgée notée |
| Hydraulique | **4,9 à 6,9 bar·s/g** si la mouture retenue est en cause (≈ 9 à 12 s depuis 8 bar, ≈ 1,3 à 1,8 g/s) ; vers 7,6, elle ne l'est pas ; plateau sous 9,2 bar |
| Rinçage | 0,1 à 0,7 bar |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml ; noter la fin (pression ou échéance) et le volume à la première goutte |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 20,6 g final |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche |
| Tasse | 89,0 à 92,5 °C ; amertume notée |

### 6 novembre, 13 h 19 — `261106-131948`

**Contexte.** Single espresso, ViCafé, porte-filtre sans fond, même cran
que le 5/11 à 9 h 11 et 14 h 31 et ce matin à 7 h 39, cible 20 g,
anticipation d'arrêt de 1,0 g, consigne de 90 °C. **Dose : 7,4 g affichés
pour 7,5 g réglés, non complétée** : la balance a montré 7,5 g puis 7,4 g
après quelques secondes, et une impulsion du moulin vaut 0,3 g. **Canal du
moulin non purgé** ; la mouture retenue date de 7 h 39, au même cran.
Écran 0.4.27, sensors 0.4.24. Salon (Netatmo) : 48 %, point de rosée
**11,0 °C** (10,7 °C ce matin) ; extérieur : 100 %, point de rosée
10,0 °C ; boîtier de l'écran à 34,1 °C et 36,2 %. Observation de David :
**débit bien plus grand que ce matin**, **amertume en baisse**.

**Routine** : guide ouvert à 90,0 °C ; rinçage de 5,0 s, 16,5 ml, 35,6 g,
**pression maximale 8,31 bar**. Consigne déjà atteinte à la fin du
rinçage, minimum 85,4 °C ensuite. **537 s sur le groupe** ; préparation de
125,9 s. Tasse de 124,9 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,04 s | — | 0,34 → 0,45 bar |
| Remplissage | 8,04 s | 7,65 s, pression | 22,0 ml (2,88 ml/s) | 0,45 → 0,84 bar |
| Pré-infusion | 15,69 s | **2,94 s, poids** | 3,8 ml | 0,84 → 1,12 bar |
| Infusion | 18,62 s | 11,78 s | 25,5 ml | 8,78 bar max |

- **Première goutte (0,3 g) à 18,71 s, à 1,12 bar**, **26,1 ml** (38,3 ml
  ce matin), dès le début de l'infusion ; la pré-infusion finit sur le
  poids.
- 8 bar à 23,54 s (4,9 s après le début de l'infusion), 6,4 g en tasse.
  Pas de pic transitoire : maximum de 8,78 bar à 30,20 s.
- Depuis 8 bar : **6,9 s**, **1,87 g/s** en tasse, 1,73 ml/s en amont.
  **19,2 g à l'arrêt de la pompe**, **20,9 g final**, **1,7 g de
  gouttes**. Pompe en marche 22,2 s. Goutte → arrêt de la pompe :
  **11,7 s**.
- Plateau (de 8 bar + 2 s à l'arrêt) : **8,61 ± 0,11 bar**, pompe à
  71–74 %, 1,81 g/s, soit **4,7 bar·s/g** (≈ 4,8 ramené à 7,5 g).
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 18 g |
  | --- | ---: | ---: | ---: | ---: |
  | 06/11 13 h 19 | 1,8 s | 4,6 s | 6,9 s | 11,2 s |
  | 06/11 7 h 39 | 2,6 s | 6,0 s | 10,6 s | 17,0 s |
  | 05/11 14 h 31 | 1,8 s | 4,8 s | 8,5 s | 14,5 s |
  | 05/11 9 h 11 | 2,3 s | 4,8 s | 7,8 s | 12,3 s |

**Thermique.** NTC de 90,03 °C au départ, **+1,32 °C** pendant le
remplissage (22,0 ml), −0,98 °C pendant la pré-infusion ; 90,68 °C au
début de l'infusion, 89,38 °C à l'arrêt. Baisse pompe en marche
**−0,65 °C** (89,4 à 92,2 °C). **Tasse : 90,17 °C, +0,17 °C.**

**Attendus vérifiés** (entrée de 7 h 39, prévue pour demain matin après
purge ; celle-ci est une infusion de l'après-midi sans purge).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Dose | pesée avant complément ; ratio = final / dose pesée ; masse purgée notée | 7,4 g, non complétés ; ratio **2,82** ; pas de purge |
| Hydraulique | 4,9 à 6,9 bar·s/g si la mouture retenue est en cause ; plateau sous 9,2 bar | **4,7 bar·s/g, 6,9 s, 1,87 g/s : juste en dessous** ; plateau 8,61 bar : atteint |
| Rinçage | 0,1 à 0,7 bar | **8,31 bar, très au-dessus** ; 35,6 g pour 16,5 ml |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 28 ml ; fin et volume à la goutte | **2,88 ml/s, juste en dessous** ; 22,0 ml : atteint ; fin sur la pression ; goutte à 26,1 ml |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 20,6 g final | 19,2 g ; **20,9 g, 0,3 g au-dessus** |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche | +1,32 °C ; −0,65 °C : atteint |
| Tasse | 89,0 à 92,5 °C ; amertume notée | 90,17 °C : atteint ; amertume en baisse |

**Lecture.**

- *Variabilité au même cran, mesuré.* Quatre infusions au même cran, à
  7,4–7,5 g : **4,9**, **6,8**, **7,6** et **4,7 bar·s/g**, un écart de
  **+62 %** entre les extrêmes, dont **−38 %** entre ce matin et cet
  après-midi sans rien changer. La dose n'y est pour rien (0,1 g de moins,
  ≈ −1 à −2 %, Darcy) ; la dispersion de préparation (± 18 %) n'en couvre
  pas le tiers.
- *Âge de la mouture retenue, mesuré.* Les deux après-midi sans purge
  partent d'une mouture retenue depuis 5 h 20 à 5 h 40 (9 h 11 → 14 h 31,
  7 h 39 → 13 h 19) et donnent 6,8 et 4,7 : **+45 %** à rétention
  comparable. L'âge de la mouture retenue n'explique donc pas l'écart de
  l'après-midi ; le moulin varie à cran fixe, ce que conclut David.
- *Matins sans purge, hypothèse de David.* Les deux matins sans purge
  sont les plus résistants (7,7 au cran précédent, 7,6) et les plus amers ;
  la mouture restée toute la nuit dans la chute ajouterait l'amertume. Cet
  après-midi, plus rapide, l'amertume a baissé. La purge de demain matin
  le teste.
- *Remplissage de la trémie, hypothèse de David.* David a remarqué une
  différence quand la trémie est presque vide : la charge de grains sur
  les meules, donc la vitesse d'entrée des grains, change la mouture. Le
  niveau de la trémie n'a été noté pour aucune infusion de la série ; il
  peut contribuer à l'écart à cran fixe.
- *Arrêt, mesuré.* 1,7 g de gouttes à 1,81 g/s, la plus longue queue de
  la série (1,3 g à 1,74 g/s le 5/11 à 9 h 11, 1,0 g à 1,19 g/s ce matin) :
  au débit élevé, l'anticipation de 1,0 g laisse finir à 20,9 g.
- *Première goutte, observé.* 26,1 ml sur la galette la plus rapide,
  38,3 ml ce matin sur la plus résistante, mais 21,7 ml le 5/11 à
  14 h 31 à 6,8 : le volume à la goutte ne suit pas la résistance.
- *Rinçage, mesuré.* 8,31 bar au rinçage, contre 0,14 à 0,35 bar dans les
  entrées du 4/11 au matin de ce jour, et 35,6 g en tasse pour 16,5 ml au
  débitmètre. Cause non établie : à confirmer par David (porte-filtre,
  panier, tasse).
- *Humidité, observé.* Point de rosée du salon à 11,0 °C, +0,3 °C sur ce
  matin, pendant que la résistance baissait de 38 % : l'humidité ne suit
  pas l'écart.

**Décision** (David). **Accepter la variabilité de ce moulin** ; réglages
inchangés : même cran, 7,5 g, cible 20 g, anticipation de 1,0 g, 90 °C.
La purge de demain matin, décidée à 7 h 39, reste prévue. L'amélioration
suivante est un **moulin single dose**, le **Mazzer Philos** en ligne de
mire.

*Estimé.* À cran fixe, ce moulin donne 4,7 à 7,6 bar·s/g : une infusion
dans cette plage ne départage plus rien sur la galette. Le test de la
purge repose donc surtout sur le goût : une amertume qui disparaît avec la
purge confirme l'hypothèse de David ; 7,6 avec amertume après purge
l'écarte.

| Point | Attendu (demain matin, purge, mêmes réglages) |
| --- | --- |
| Dose | pesée avant complément ; ratio = final / dose pesée ; masse purgée notée ; **niveau de la trémie noté** |
| Hydraulique | **4,7 à 7,6 bar·s/g** (≈ 7 à 14 s depuis 8 bar, ≈ 1,2 à 1,9 g/s) ; plateau sous 9,2 bar |
| Rinçage | 0,1 à 0,7 bar ; au-dessus, noter l'état du porte-filtre |
| Remplissage | 2,9 à 3,3 ml/s à 70 % ; 17 à 30 ml ; noter la fin (pression ou échéance) |
| Rendement | arrêt de la pompe vers 19,0 à 19,3 g, 20,0 à 21,0 g final |
| Chauffe | NTC +0,1 à +2,3 °C au remplissage ; −1,7 à +1,9 °C pompe en marche |
| Tasse | 89,0 à 92,5 °C ; **amertume notée** : c'est le critère de la purge |

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

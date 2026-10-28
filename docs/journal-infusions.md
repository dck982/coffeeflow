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

## Réglages en cours

Configuration sauvegardée `configs/261024-155512.json` (schéma 11), profil
actif « single espresso » (8,0 g, panier LM1 41 mm). Ses valeurs sont
celles de `configs/261024-102547.json`, en vigueur le 24 octobre à
10 h 22 ; le profil les reprend depuis l'écran 0.4.22.
Depuis le 26 octobre, `configs/261026-093452.json` ajoute un profil
« turbo shot » (11,0 g, panier 58 mm 15 g, cible 45 g, 6 bar) ; ses
essais sont dans les entrées du journal et résumés dans
[turboshot.md](turboshot.md). Depuis le 26 octobre à 13 h 32,
`configs/261026-142235.json` : profil « turbo shot » à 40 g et 88 °C ;
depuis 15 h 35, `configs/261026-153711.json` : 45 g et `lead_weight_g` de
5,0 g, avec 15 g de café (le champ `dose_g` est resté à 11,0 g).
Depuis le 27 octobre à 7 h 44, `configs/261027-074444.json` : profil
« turbo shot » avec `brew.pump_pct` à 70 % et `dose_g` à 15,0 g, puck
screen sur la galette. Le 27 octobre à 13 h 16, sans configuration
sauvegardée (valeurs lues dans la capture) : `brew.pump_pct` et
`filling.pump_pct` à 100 %, consigne de 86 °C, précharge de 2 s.
Depuis 13 h 49, mouture espresso ; ensuite 88 °C et 8 s de précharge.
Le 28 octobre à 10 h 09, retour au profil « single espresso » (valeurs
lues dans la capture, celles du tableau ci-dessous) avec le
**porte-filtre à bec**.

| Réglage | Valeur | Depuis |
| --- | --- | --- |
| Firmware | écran et carte sensors **0.4.24** | 26 octobre au plus tard (écran 0.4.23 du 24 octobre après 13 h 23, 0.4.15 avant) |
| Offset NTC | **−14 °C** (écran 0.4.4), mesuré au panier avec la SCACE ; −10,5 °C avant. Consigne bornée à 95 °C, coupure à 100 °C utilisateur (114 °C sonde) | 10 octobre |
| Sonde de pression | XDB401 lu sur J10 de la carte sensors avec la calibration de A2, loi de chauffe inchangée | 23 octobre (A2 de l'écran depuis le 7 octobre) |
| Précharge | 8 s à 90 % | 4 octobre |
| Consigne | **91 °C** | 24 octobre, 10 h 22 (92 °C du 13 octobre, 8 h 18 ; 90 °C avant) |
| Remplissage | pompe à **70 %**, fin à 0,8 bar (10 s au plus) | 24 octobre, 10 h 22 (75 % du 5 octobre, 7 h 59) |
| Pré-infusion | pompe à 50 %, fin à 4 s ou à la première goutte | 5 octobre, 7 h 59 |
| Infusion | pompe jusqu'à 100 %, cible 9 bar, arrêt à **22 g** ou 27 s, sans ramp-down | 13 octobre, 13 h 29 (23 g du 11 octobre, 13 h 40 au 13 octobre, 8 h 18) |
| Purge | pompe à **80 %**, 10 s au plus | 24 octobre, 10 h 22 (60 % le 23 octobre) |
| Machine | fermée (panneaux posés), filtre ECM Aqua Aroma Crema dans le réservoir ; état final. Remplissage à 3,1 à 3,3 ml/s à 70 % (3,5 à 3,6 ml/s à 75 %) | 10 octobre |

Depuis l'écran 0.4.22, la capture (schéma v3) enregistre les versions du
firmware et le profil utilisé. Avant, le numéro de firmware est celui du
dernier flash connu.

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

## Pistes pour le goût

Recette : grain ViCafé (100 % arabica), 8,0 g en panier, jusqu'au
20 octobre ; depuis le 23 octobre, « Cozy Chocolate » de Coffee Coaching
Club (100 % arabica du Brésil, nature, torréfié le 8 octobre), 8,0 g. Routine de
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
20 octobre. Les chiffres de chaque infusion restent dans la table de
synthèse.

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

## Entrées

### 23 octobre, 14 h 38 — `261023-143811`

**Contexte.** Première infusion depuis le passage du module capteurs sur
la carte `pcb/sensors` (0.4.16 à 0.4.19, 21 octobre) : débitmètre sur la
pull-up de 1 kΩ et le filtre de la carte, SSR chaudière commandé par
Q3/Q2, XDB401 lu sur J10 avec la calibration de A2. Réglages de l'écran
inchangés. **Nouveau café**, à la mouture apparemment beaucoup plus fine ;
dose de 8,0 g. Observations de David : goût
**assez acide**, **peu de crema**, le café n'est apparu sous le
porte-filtre **qu'après la montée en pression**.

**Routine** : rinçage 21,6 s après l'ouverture du guide (NTC à 87,9 °C),
de 5,02 s, **7,2 ml, 14,3 g** (18,8 à 19,4 ml et 24,4 à 33,3 g du 17 au
20 octobre), pression maximale 0,19 bar ; retour à la consigne en 429,4 s
(minimum 84,5 °C, résistance active 41,1 s) ; porte-filtre en place
49,0 s à la consigne, 478,4 s sur le groupe après le rinçage ;
préparation de 71,6 s. La balance n'a pas manqué.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | 0,28 → 0,34 bar |
| Remplissage | 8,01 s | **10,01 s**, échéance | 23,8 ml (**2,38 ml/s**) | 0,34 → **0,62 bar** |
| Pré-infusion | 18,01 s | 4,18 s, échéance | 3,8 ml (0,90 ml/s) | 0,63 → 0,85 bar |
| Infusion | 22,19 s | **34,43 s** | 34,8 ml | 9,41 bar max |

- Pression au repos avant la pompe : 0,28 bar (0,50 bar le 20, 0,29 à
  0,30 bar les 17 et 18).
- Remplissage fini sur l'échéance de 10 s, pour la première fois dans la
  synthèse : 0,8 bar n'est atteint qu'en infusion. Le débitmètre compte
  2,38 à 2,51 ml/s à 75 % (3,44 à 3,57 ml/s le 20, 3,1 à 3,7 ml/s
  depuis le 10 octobre). La pression reste à 0,28–0,33 bar pendant les
  7,5 premières secondes, contre 0,44–0,47 bar le 20 au même instant,
  puis monte à 0,62 bar.
- Pré-infusion finie sur l'échéance, sans goutte. Première goutte (0,3 g)
  à 26,93 s, **à 7,33 bar** et 40,6 ml, 4,7 s après le début de
  l'infusion : marge de **16,8 ml** (8,7 ml le 20). C'est ce que David a
  vu : rien ne passe sous 1 bar.
- Montée de 4,9 s jusqu'à 8 bar, avec 0,4 g en tasse (2,8 g le 20). Pic à
  9,41 bar à 27,51 s, 0,4 s après 8 bar.
- Depuis 8 bar : **29,7 s**, **0,70 g/s** en tasse, 0,72 ml/s en amont,
  l'infusion la plus lente de la synthèse avec celle du 4 octobre à
  10 h 01 (29,8 s, 0,71 g/s). 21,8 g à l'arrêt, au poids, 22,3 g final,
  0,5 g de gouttes. Pompe en marche 48,6 s.
- Plateau : 9,01 ± 0,07 bar, pompe à 66 %, 0,72 g/s, soit
  **12,5 bar·s/g** (9,0 le 20, 6,6 à 7,2 du 16 au 17 octobre). Rapport du
  débit en tasse au débit amont : **1,02 g/ml**, contre 0,50 à 0,69 g/ml
  jusqu'ici ; il suivait la perméabilité de la galette et devrait être le
  plus bas de la série, il est le plus haut.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 23/10 14 h 38 (nouveau café) | 5,6 s | 12,9 s | 20,0 s | **32,7 s** |
  | 20/10 14 h 29 (mouture fine) | 1,9 s | 6,0 s | 11,3 s | 20,8 s |
  | 17/10 13 h 57 | 1,3 s | 4,5 s | 8,8 s | 16,3 s |

**Thermique.**

| Repère | 23/10 14 h 38 | 20/10 14 h 29 | 18/10 12 h 40 |
| --- | ---: | ---: | ---: |
| NTC au départ | 91,85 °C | 92,00 °C | 91,83 °C |
| Pic avant l'infusion | 93,14 °C à 11,9 s | 94,38 °C à 13,9 s | 93,15 °C à 12,3 s |
| NTC au début de l'infusion | 91,73 °C | 93,70 °C | 88,36 °C |
| Commande à 0 % (remboursement) | **21,1 s** | 12,1 s | 9,8 s |
| Appoint repris avant l'arrêt | 13,3 s | 10,2 s | 2,4 s |
| Pente de la NTC pendant l'appoint | **−0,26 °C/s** | −0,17 °C/s | −0,14 °C/s |
| **Tasse** | **89,15 °C**, −2,85 °C | 92,57 °C | 90,44 °C |
| Minimum | **85,48 °C**, 5,7 s après l'arrêt | 90,44 °C | 88,36 °C |
| 20 s après l'arrêt | 87,75 °C, en hausse | 93,73 °C | 91,37 °C |

- Énergie, commande / SSR : précharge 8,32 / 8,41 kJ, remplissage 7,79 /
  7,81 kJ, pré-infusion 2,13 / 1,79 kJ, infusion 3,65 / **3,49** kJ
  (6,08 kJ le 20). Total SSR jusqu'à l'arrêt : 21,50 kJ, dont 18,01 kJ
  avant l'infusion.
- Remplissage : commande de **55 à 65 %**, qui suit le débit mesuré
  (83 à 90 % le 20 à 3,44 ml/s). La NTC y perd 0,89 °C (92,36 →
  91,44 °C) ; elle gagnait 2,25 °C le 20.
- Pendant les 21,1 s à 0 %, la NTC descend de 92,58 °C (26 s) à
  ≈ 89,4 °C ; l'appoint repris à 21 % puis 26 % ne l'arrête pas.
- Deux lectures NTC invalides : 206 ms à 45,93 s, pendant l'appoint, la
  commande reste à 21,2 % (aucun pas ne tombe dessus, estimé) ; 202 ms à
  61,19 s, après l'arrêt.
- Modèle : 93,99 °C avec la commande réelle, 94,07 °C avec l'énergie SSR,
  soit **−4,86 °C** sous le modèle (−1,39 à +0,85 °C jusqu'ici) ; minimum
  91,85 °C au modèle contre 85,48 °C mesurés. Le modèle tire le
  refroidissement du débitmètre.

**Attendus vérifiés** (entrée du 20 octobre, 14 h 29). Ils visaient le
ViCafé à mouture moins fine : le café a changé.

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,5 à 8,5 bar·s/g ; 14 à 18 s depuis 8 bar ; pic de 9,0 à 9,6 bar | **12,5 bar·s/g, 29,7 s : dépassé** ; 9,41 bar : atteint |
| Rendement | ≈ 21,5 g à l'arrêt, 0,7 à 1,3 g de gouttes, 22,2 à 22,8 g final | 21,8 g, 22,3 g final : atteint ; **0,5 g de gouttes : non atteint** |
| Remplissage | 3,4 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 20 à 26 ml ; marge positive | **2,38 ml/s, 10,01 s à l'échéance : non atteint** ; 23,8 ml, marge de 16,8 ml : atteint |
| Pré-infusion | goutte jusqu'à ≈ 9 ml de marge, échéance au-delà | échéance à 16,8 ml : atteint |
| Chauffe | 90 % sur le remplissage ; NTC +0,5 à +2,3 °C au remplissage ; 0 % pendant 10 à 12,5 s ; appoint de 6 à 10 s ; lecture invalide sur un pas : 0 % ; modèle avec l'énergie SSR entre −1,2 et +1,0 °C | **55 à 65 %, −0,89 °C, 21,1 s, 13,3 s : non atteints** ; aucune lecture sur un pas : **non vérifié** ; **−4,86 °C : non atteint** |
| Tasse | 91,0 à 92,8 °C ; maximum après l'arrêt de 93,0 à 94,3 °C | **89,15 °C : non atteint** ; maximum hors capture : **non vérifié** |
| Goût | acidité moindre ; acidité marquée sous 8 bar·s/g et 16 s infirmerait le lien avec le débit faible | galette à 12,5 bar·s/g, café changé : **non concluant** |

**Lecture.**

- *Débitmètre, estimé et simulé.* Plusieurs écarts pointent vers un
  débitmètre qui compte moins depuis la carte sensors :
  1. le rapport tasse / amont de 1,02 g/ml, sur la galette la plus
     résistante de la série, est mesuré ;
  2. en simulation, les volumes multipliés par 1,4 à 1,6 ramènent le
     modèle sur la mesure à 1 °C près (tasse à 90,07 puis 88,07 °C avec
     l'énergie SSR, contre 89,21 °C mesurés ; minimum à 87,30 puis
     84,71 °C, fin de capture à 90,45 puis 87,91 °C, contre 85,48 et
     87,75 °C mesurés). Sur la capture du 20, un facteur de 1,2 suffit
     déjà à mettre le modèle 2,9 °C sous la mesure ; le facteur n'existait
     donc pas avant ;
  3. avec ce facteur, le remplissage revient à 3,3–3,8 ml/s, le débit
     habituel à 75 %.

  Si c'est le cas, la loi a chauffé pour ≈ 2/3 de l'eau réelle à chaque
  phase. La commande de 55 à 65 % au remplissage, le remboursement étiré à
  21 s et la tasse à 89,15 °C en découlent. Ni le compteur d'impulsions ni
  l'entrée FLOW n'ont été examinés.

  Correction du 23 octobre : l'infusion de 14 h 57, même carte et même
  firmware, compte 3,2 à 3,5 ml/s au remplissage et colle au modèle
  (−0,19 °C en tasse) sans facteur sur les volumes. Un débitmètre qui
  sous-compterait en permanence est écarté ; seul un défaut propre à cette
  infusion reste possible.
- *Sonde de pression, estimé.* Le remplissage à 0,29 bar, contre 0,44 à
  0,47 bar à débit réel supposé égal, et l'échéance du remplissage vont
  avec une lecture plus basse d'environ 0,2 bar depuis J10. La pression
  au repos ne départage pas : elle est piégée et varie d'un jour à l'autre.
  Le plateau régulé à 9,0 bar lus serait alors à ≈ 9,2 bar réels.

  Correction du 23 octobre : à 14 h 57, le remplissage lit 0,37 à
  0,39 bar à 3,4 ml/s, contre 0,44 à 0,47 bar le 20 à 3,56 ml/s. L'écart
  de lecture possible tombe à ≈ 0,07 bar ; les 0,29 bar de cette infusion
  vont plutôt avec un débit réellement plus faible.
- *Rinçage, expliqué par David.* 14,3 g en tasse en 5 s, contre 24,4 à
  33,3 g : la purge était à 60 % au lieu de 100 %, pour brasser la
  chaudière sans y amener trop d'eau froide (voir l'entrée suivante). Ce
  chiffre ne dit rien de la pompe.
- *Galette, mesuré.* 12,5 bar·s/g, +39 % sur le 20, déjà jugé trop fin.
  Le poids en tasse ne dépend ni du débitmètre ni de la sonde : la galette
  est bien beaucoup plus résistante. La goutte n'arrive qu'à 7,33 bar.
- *Acidité, mesuré.* Deux causes se cumulent, et rien ne les sépare :
  l'extraction la plus lente de la série (0,70 g/s, comme le 20 où David
  liait l'acidité au débit faible) et la tasse la plus froide depuis le
  12 octobre à 13 h 34 (89,15 °C, 2,85 °C sous la consigne). Le café lui-même a
  changé.
- *Crema, non mesurée.* La capture ne la mesure pas. Fraîcheur du nouveau
  café et eau plus froide sont deux causes possibles, non testées.

**Décision** (David). Mouture rendue moins fine, réglages inchangés.

**Attendus pour la vérification du débit** (purge au bouton, sans
porte-filtre, pompe à 75 %, dans une tasse sur la balance). Ce sont des
critères de diagnostic, pas des réglages.

| Point | Attendu |
| --- | --- |
| Pompe intacte | 3,3 à 3,8 g/s en tasse |
| Débitmètre qui sous-compte | volume compté de 0,6 à 0,7 fois le poids en tasse |
| Pompe qui débite moins | poids et volume tous deux sous 2,6 g/s et ml/s, rapport proche de 1 |
| Sonde de pression | lecture à vide groupe ouvert : ≈ 0 bar si la sonde est juste, ≈ −0,2 bar si elle lit bas |

### 23 octobre, 14 h 57 — `261023-145708`

**Contexte.** Même café qu'à 14 h 38, mouture rendue moins fine, mêmes
réglages et mêmes firmwares. Infusion lancée hors du mode guidé : la
capture n'a pas de routine, et la balance n'a pas été tarée (115,4 g au
départ, retirés de tous les poids ci-dessous). Observations de David :
encore trop fin selon lui ; **nettement moins acide** ; crema
**tachetée (tigrée)**. David précise aussi que la purge du rinçage est
maintenant à **60 %** au lieu de 100 % : à faible débit, elle brasse la
chaudière, dont l'eau chaude est en haut, autour de la sonde, et la couche
froide au fond. Elle fait aussi passer de l'eau dans le bras de la vanne
solénoïde et dans le groupe, sans amener trop d'eau froide.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,15 s | 7,91 s | — | 0,23 → 0,39 bar |
| Remplissage | 8,06 s | 6,90 s | 22,0 ml (3,19 ml/s) | 0,39 → 0,85 bar |
| Pré-infusion | 14,97 s | 4,02 s, échéance | 4,3 ml (1,08 ml/s) | 0,85 → 1,21 bar |
| Infusion | 18,99 s | 30,17 s | 33,6 ml | 9,39 bar max |

- Pression au repos avant la pompe : 0,23 bar.
- Remplissage fini sur la pression : 3,21 à 3,47 ml/s comptés à 75 %,
  0,37 à 0,39 bar sur le palier (0,29 à 14 h 38, 0,44 à 0,47 le 20).
- Pré-infusion finie sur l'échéance, sans goutte. Première goutte à
  22,89 s, **à 9,14 bar**, 36,2 ml : marge de 14,2 ml (16,8 ml à 14 h 38).
  Rien ne passe encore avant la pression.
- Montée de 3,8 s jusqu'à 8 bar, rien en tasse ; pic à 9,39 bar à
  23,07 s.
- Depuis 8 bar : **26,4 s**, 0,82 g/s en tasse, 0,91 ml/s en amont.
  21,6 g à l'arrêt, au poids, 22,4 g final, 0,8 g de gouttes. Pompe en
  marche 40,9 s.
- Plateau : 9,03 ± 0,08 bar, pompe à 66 %, 0,83 g/s, soit
  **10,9 bar·s/g** (12,5 à 14 h 38, 9,0 le 20, 6,6 à 7,2 les 16 et
  17 octobre). Rapport tasse / amont : 0,91 g/ml (1,02 à 14 h 38, 0,50 à
  0,69 jusqu'au 20).
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 23/10 14 h 57 (mouture ouverte) | 5,0 s | 11,9 s | 17,9 s | **28,6 s** |
  | 23/10 14 h 38 | 5,6 s | 12,9 s | 20,0 s | 32,7 s |
  | 20/10 14 h 29 | 1,9 s | 6,0 s | 11,3 s | 20,8 s |

**Thermique.**

| Repère | 23/10 14 h 57 | 23/10 14 h 38 | 20/10 14 h 29 |
| --- | ---: | ---: | ---: |
| NTC au départ | 92,02 °C | 91,85 °C | 92,00 °C |
| NTC pendant le remplissage | **+1,03 °C** | −0,89 °C | +2,25 °C |
| Pic avant l'infusion | 93,30 °C à 14,1 s | 93,14 °C à 11,9 s | 94,38 °C à 13,9 s |
| NTC au début de l'infusion | 92,66 °C | 91,73 °C | 93,70 °C |
| Commande à 0 % (remboursement) | **21,7 s** | 21,1 s | 12,1 s |
| NTC pendant le 0 % | monte à 94,36 °C (25 s), 92,3 °C à la fin | 92,58 → ≈ 89,4 °C | — |
| Appoint repris avant l'arrêt | 8,3 s | 13,3 s | 10,2 s |
| **Tasse** | **92,42 °C**, +0,42 °C | 89,15 °C | 92,57 °C |
| Minimum | 88,12 °C, 5,3 s après l'arrêt | 85,48 °C | 90,44 °C |
| 20 s après l'arrêt | 90,24 °C, en hausse | 87,75 °C | 93,73 °C |

- Énergie, commande / SSR : précharge 8,11 / 7,89 kJ, remplissage 7,09 /
  6,89 kJ, pré-infusion 2,27 / 2,18 kJ, infusion 2,99 / 2,59 kJ.
- Remplissage : commande de 77 à 90 %, au débit compté.
- Trois lectures NTC invalides : 200 ms à 15,28 s, en pré-infusion,
  commande à 73,1 % sur l'échantillon ; 207 et 206 ms à 44,55 et 44,86 s,
  pendant l'appoint, à 28,5 %. Aucune ne fait tomber la commande à 0 %.
  Si un pas tombe sur l'une d'elles, la règle n'est pas appliquée ; rien
  ne permet de le dire.
- Modèle : 92,33 °C avec la commande réelle, 92,27 °C avec l'énergie SSR,
  soit −0,25 et −0,19 °C. La mesure passe au-dessus du modèle pendant le
  remplissage (+1,12 °C au début de l'infusion, +1,49 °C 5 s après), puis
  dessous à −0,18 °C/s jusqu'à −3,09 °C à l'arrêt. Ce rythme est dans la
  plage des infusions passées (−0,31 à +0,17 °C/s, rejouées depuis le
  1er octobre) ; l'écart de 14 h 38 venait surtout de son départ, à
  −1,28 °C au début de l'infusion.

**Attendus vérifiés** (entrée de 14 h 38). La purge de diagnostic n'a pas
été faite. L'infusion en donne une partie :

| Point | Attendu | Mesure |
| --- | --- | --- |
| Pompe intacte | 3,3 à 3,8 ml/s | 3,21 à 3,47 ml/s comptés au remplissage, à galette en place : compatible |
| Débitmètre qui sous-compte | volume compté de 0,6 à 0,7 fois le poids | non mesuré ; le modèle colle sans facteur : **écarté en permanence** |
| Sonde de pression | ≈ −0,2 bar à vide | non mesurée ; palier de remplissage 0,07 bar sous le 20 au même débit |

**Lecture.**

- *L'infusion de 14 h 38 reste à part, non expliqué.* Comparées à 14 h 57,
  même carte, même café, presque même galette, deux différences
  mesurées : le remplissage à 2,38 ml/s comptés et 0,29 bar, contre 3,19 et
  0,37–0,39 bar, et une NTC 1,9 °C plus bas à la fin du remplissage, pour
  autant d'eau comptée (23,8 contre 22,0 ml) et plus d'énergie (7,81 contre
  6,89 kJ). Pression et débit plus bas ensemble vont avec une pompe qui
  débitait vraiment moins, pas avec un débitmètre qui compte mal. Ils
  n'expliquent pas pourquoi la NTC a refroidi davantage. Deux pistes, non
  testées :
  1. **la couche froide** : à 14 h 38, la purge de rinçage à 60 % venait
     de brasser la chaudière (retour à la consigne en 429 s, minimum
     84,5 °C). La sonde, en haut, a pu revenir à la consigne avant le
     reste de l'eau, qui la refroidit pendant l'infusion ;
  2. **un débit réel différent du débit compté**, ponctuel, à 14 h 38
     seulement.

  Ce qui a précédé l'infusion de 14 h 57 n'est pas enregistré.
- *Galette, mesuré.* −1,6 bar·s/g pour l'ouverture de mouture, et 10,9
  bar·s/g restent 50 % au-dessus des galettes sans fond du ViCafé (6,6 à
  7,2), au-dessus aussi du 20 jugé trop fin (9,0). L'avis de David,
  « encore trop fin », va dans le même sens. La goutte n'arrive qu'à
  9 bar : la pré-infusion, finie sur l'échéance, ne mouille pas la galette
  jusqu'au bout.
- *Acidité, mesuré.* Nettement moins acide avec, ensemble, un débit
  plus rapide (0,82 contre 0,70 g/s) et une tasse plus chaude de 3,27 °C
  (92,42 contre 89,15 °C). Rien ne sépare les deux.
- *Crema tigrée, non mesurée.* Aucune grandeur de la capture ne la suit.

**Décision** (David). Mouture encore ouverte, réglages inchangés.

**Attendus pour l'infusion suivante** (même café, mouture encore ouverte,
mêmes réglages). Le pas de mouture n'est pas chiffré ; celui de ce jour
vaut −1,6 bar·s/g, celui du 8 octobre ≈ −0,8.

| Point | Attendu |
| --- | --- |
| Hydraulique | 8,5 à 10,0 bar·s/g pour un pas comme celui-ci ; 19 à 25 s depuis 8 bar ; pic de 9,0 à 9,6 bar |
| Pré-infusion | échéance tant que la galette dépasse ≈ 9 bar·s/g, goutte en dessous ; première goutte sous 9 bar dès que la goutte arrive en pré-infusion |
| Rendement | ≈ 21,6 g à l'arrêt, 0,7 à 1,2 g de gouttes |
| Remplissage | 3,1 à 3,6 ml/s comptés à 75 % ; fin sur la pression en 6 à 8 s. Un remplissage à l'échéance sous 2,6 ml/s refait 14 h 38 : purge de diagnostic dans la foulée |
| Chauffe | NTC de +0,5 à +2,3 °C pendant le remplissage ; 0 % pendant 15 à 21 s ; modèle avec l'énergie SSR entre −1,2 et +1,0 °C en tasse |
| Tasse | 91,0 à 92,8 °C |
| Goût | acidité encore moindre ; si elle revient à galette plus ouverte et tasse dans la plage, le lien acidité–débit lent ne suffit plus |

### 23 octobre, 15 h 08 — `261023-150828`

**Contexte.** Même café, « Cozy Chocolate » de Coffee Coaching Club
(100 % arabica du Brésil, nature, torréfié le 8 octobre, 15 jours avant).
Mouture encore ouverte, mêmes réglages, routine guidée. Observations de
David : débit meilleur ; la pré-infusion finit toujours au temps et rien
n'apparaît sous le porte-filtre pendant le remplissage, qui est peut-être
trop court ; café bon mais un peu « brûlé », crema assez timide pour un
café frais ; David pense que ce café demande une température plus basse.

**Routine** : rinçage 13,5 s après l'ouverture du guide (NTC à 90,7 °C),
de 5,05 s, 12,2 ml, 19,1 g ; retour à la consigne en 54,5 s (minimum
87,6 °C, résistance active 13,2 s) ; porte-filtre en place 150,8 s à la
consigne, 205,3 s sur le groupe après le rinçage ; préparation de 69,9 s.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | 0,34 → 0,46 bar |
| Remplissage | 8,01 s | 7,03 s | 24,3 ml (3,46 ml/s) | 0,46 → 0,91 bar |
| Pré-infusion | 15,04 s | 3,99 s, échéance | 5,5 ml (1,38 ml/s) | 0,91 → 1,38 bar |
| Infusion | 19,03 s | 19,32 s | 34,5 ml | 9,14 bar max |

- Remplissage fini sur la pression : 3,48 à 3,59 ml/s à 75 %, palier à
  0,36 bar. Les débits de 14 h 57 et de 15 h 08 encadrent les 3,4 à
  3,6 ml/s habituels : seul 14 h 38 en sort.
- Pré-infusion finie sur l'échéance. Première goutte à 21,91 s, **à
  5,64 bar**, 37,1 ml : marge de 12,8 ml.
- Montée de 3,5 s jusqu'à 8 bar ; pic à 9,14 bar à 22,83 s.
- Depuis 8 bar : 15,9 s, 1,26 g/s en tasse, 1,58 ml/s en amont. 21,5 g à
  l'arrêt, 22,7 g final, 1,2 g de gouttes. Pompe en marche 30,4 s.
- Plateau : 8,84 ± 0,06 bar, pompe à 69 %, 1,31 g/s, soit
  **6,8 bar·s/g**, dans la plage des galettes sans fond du ViCafé (6,6 à
  7,2 du 16 au 17 octobre). Rapport tasse / amont : 0,80 g/ml.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 23/10 15 h 08 | 3,4 s | 7,1 s | 11,2 s | 18,5 s |
  | 23/10 14 h 57 | 5,0 s | 11,9 s | 17,9 s | 28,6 s |
  | 17/10 13 h 57 (ViCafé, 6,9 bar·s/g) | 1,3 s | 4,5 s | 8,8 s | 16,3 s |

**Thermique.**

| Repère | 23/10 15 h 08 | 23/10 14 h 57 |
| --- | ---: | ---: |
| NTC au départ | 92,02 °C | 92,02 °C |
| NTC pendant le remplissage | +0,83 °C | +1,03 °C |
| Pic avant l'infusion | 93,52 °C à 12,9 s | 93,30 °C à 14,1 s |
| NTC au début de l'infusion | 92,55 °C | 92,66 °C |
| Commande à 0 % (remboursement) | 16,0 s, NTC jusqu'à 93,89 °C | 21,7 s |
| Appoint repris avant l'arrêt | 3,3 s | 8,3 s |
| **Tasse** | **92,86 °C**, +0,86 °C | 92,42 °C |
| Minimum | 88,80 °C, 6,4 s après l'arrêt | 88,12 °C |
| 20 s après l'arrêt | 90,22 °C | 90,24 °C |

- Énergie, commande / SSR : précharge 8,49 / 8,31 kJ, remplissage 7,58 /
  7,16 kJ, pré-infusion 2,52 / 2,30 kJ, infusion 1,87 / 1,79 kJ.
- Remplissage : commande de 88 à 90 %.
- Lecture NTC invalide : 203 ms à 38,24 s, 0,1 s avant l'arrêt, commande
  à 55,1 % sur l'échantillon.
- Modèle : 93,42 °C avec la commande réelle, −0,53 °C ; −0,20 °C avec
  l'énergie SSR.

**Attendus vérifiés** (entrée de 14 h 57).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 8,5 à 10,0 bar·s/g ; 19 à 25 s depuis 8 bar ; pic de 9,0 à 9,6 bar | **6,8 bar·s/g, 15,9 s : dépassé**, pas de mouture plus grand ; 9,14 bar : atteint |
| Pré-infusion | goutte sous ≈ 9 bar·s/g | **échéance à 6,8 bar·s/g : non atteint** |
| Rendement | ≈ 21,6 g à l'arrêt, 0,7 à 1,2 g de gouttes | 21,5 g, 1,2 g : atteint |
| Remplissage | 3,1 à 3,6 ml/s, fin sur la pression en 6 à 8 s | 3,46 ml/s, 7,03 s : atteint |
| Chauffe | NTC +0,5 à +2,3 °C au remplissage ; 0 % pendant 15 à 21 s ; modèle SSR entre −1,2 et +1,0 °C | +0,83 °C, 16,0 s, −0,20 °C : atteint |
| Tasse | 91,0 à 92,8 °C | **92,86 °C : non atteint** de 0,06 °C |
| Goût | acidité encore moindre | pas d'acidité citée ; « un peu brûlé » |

**Lecture.**

- *La goutte attend la pression, mesuré.* Sur les trois infusions de ce
  café, la première goutte arrive à 7,33, 9,14 puis 5,64 bar, toujours en
  infusion. Sur le ViCafé, à la même résistance (6,6 à 7,2 bar·s/g, 16 et
  17 octobre), la goutte finissait la pré-infusion, et le repère de 0,3 g
  tombait juste après, entre 1,21 et 1,38 bar. La résistance au plateau ne dit
  donc pas tout : cette galette ne laisse rien passer à basse pression.
- *Remplissage plus long, estimé.* Un seuil plus haut ne ferait pas sortir
  le café pendant le remplissage. À 3,5 ml/s, la pression atteint 1 bar à
  26,7 ml et 2 bar à 32,5 ml (repères de la capture), mais la goutte n'est
  venue qu'à 5,64 bar. Il faudrait finir le remplissage vers 5 bar, soit
  faire monter la pression à 75 % de pompe. Une pré-infusion plus longue
  mouillerait davantage la galette à basse pression : à 1,35 ml/s, il
  manquait 7,3 ml après 4 s, soit ≈ 9,5 s de pré-infusion en tout, si le
  débit ne baisse pas avec la pression qui monte. Rien ne dit qu'une
  galette saturée lâche sa goutte sous 1,4 bar.
- *Cause de la galette, non testée.* Une galette qui résiste au mouillage
  plutôt qu'au débit va avec le dégazage d'un café de 15 jours ou avec la
  mouture d'un nature. Rien ne sépare les deux.
- *Température, mesuré.* 92,86 °C en tasse, la plus chaude des trois, au
  critère à +0,86 °C près. Le 13 octobre, 90 → 92 °C de consigne n'avait
  donné que +1,0 °C en tasse ; depuis, la tasse suit la consigne à ±1 °C.
  Une consigne de 91 °C donnerait ≈ 91,9 °C, 90 °C ≈ 90,9 °C (estimé sur
  cet écart).
- *« Brûlé » et crema, non mesurés.* La capture ne suit ni l'un ni
  l'autre.
- *Galette trempée, observation de David.* Après l'infusion, la galette
  est nettement plus trempée que d'habitude, sans channelling (un seul
  filet). Il reste de l'espace entre la galette et la douchette, sans
  marque, à 8,0 g. Une prise d'eau trop longue ne l'explique pas, mesuré :
  remplissage et pré-infusion durent 11,0 s pour 29,9 ml, comme les 8,3 à
  11,3 s et 27 à 35 ml du ViCafé ; seule la goutte vient ≈ 3 s plus tard
  (13,9 s après le début du remplissage, 9,0 à 11,6 s du 16 au
  20 octobre). L'eau gardée à l'arrêt (amont − tasse) est même plus
  faible : 42,8 ml, et 38,4 à 40,7 ml dans la journée, contre 45 à 59 ml
  du 16 au 20 octobre. Le plateau stable (± 0,06 bar) ne montre pas de
  channelling. Cause non identifiée.

**Décision** (David). Pistes retenues pour la suite, une à la fois :
baisser la consigne, puis la dose sous 8,0 g. Infusion suivante le
24 octobre.

### 24 octobre, 10 h 22 — `261024-102248`

**Contexte.** Même café, même mouture qu'à 15 h 08 la veille, routine
guidée. Trois réglages changés ensemble (configuration sauvegardée à
10 h 25) : **consigne de 91 °C** (92 °C), **remplissage à 70 %** (75 %),
**purge à 80 %** (60 % le 23). Temps
cible de 27 s, sans effet sur l'arrêt au poids. Observations de David :
mouture adaptée, **goût bon** ; pas de channelling visible, débit
constant pendant l'infusion.

Hypothèse de David sur le remplissage plus lent : il amène l'eau froide
plus lentement dans la chaudière et peut réduire le creux du remplissage,
mais il laisse plus de temps à la galette pour se dégrader (perdre de la
résistance) avant la mise en pression.

**Routine** : rinçage 10,1 s après l'ouverture du guide (NTC à 90,9 °C),
de 5,03 s, **18,8 ml, 20,6 g** (12,2 ml et 19,1 g à 15 h 08, purge à
60 %), 3,75 ml/s à 80 %, pression maximale 0,24 bar ; retour à la consigne en
87,0 s (minimum 86,3 °C, résistance active 14,9 s) ; porte-filtre en place
133,0 s à la consigne, 220,0 s sur le groupe après le rinçage ;
préparation de 96,0 s.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,09 s | — | 0,22 → 0,31 bar |
| Remplissage | 8,09 s | **8,98 s** | **28,1 ml** (3,13 ml/s) | 0,31 → 0,86 bar |
| Pré-infusion | 17,07 s | 3,90 s, échéance | 5,8 ml (1,49 ml/s) | 0,86 → 1,29 bar |
| Infusion | 20,97 s | 17,41 s | 31,9 ml | 8,98 bar max |

- Remplissage fini sur la pression : 3,13 à 3,24 ml/s à 70 % (3,48 à
  3,59 ml/s à 75 % la veille), palier à 0,24–0,32 bar (0,36 bar).
- La pression monte plus tard en volume :

  | Repère | 24/10 10 h 22 | 23/10 15 h 08 |
  | --- | ---: | ---: |
  | 0,5 bar | 23,8 ml | 18,6 ml |
  | 0,8 bar | 27,5 ml | 23,2 ml |
  | 1 bar | 31,3 ml | 26,7 ml |
  | 2 bar | 36,5 ml | 32,5 ml |

  Environ 4 à 5 ml de plus entrent avant chaque palier de pression.
- Pré-infusion finie sur l'échéance (3,90 s au pas de l'écran). Les
  premiers 0,2 g arrivent 0,1 s après, à 1,32 bar ; **première goutte
  (0,3 g) à 21,87 s, à 1,55 bar**, 35,1 ml. Sur ce café, c'est la première
  fois que la goutte n'attend pas la pression (7,33, 9,14 puis 5,64 bar le
  23).
- Montée de 3,5 s jusqu'à 8 bar, **3,1 g** déjà en tasse (1,0 g à 15 h 08).
  **Pas de pic** : la pression monte jusqu'à 8,98 bar à 30,5 s, 6 s après
  8 bar (9,14 bar 0,3 s après 8 bar la veille).
- Depuis 8 bar : 13,9 s, 1,32 g/s en tasse, 1,63 ml/s en amont. 21,4 g à
  l'arrêt, 23,1 g final, **1,7 g de gouttes**. Pompe en marche 30,3 s.
- Plateau : 8,88 ± 0,05 bar, pompe à 70 %, 1,32 g/s, soit
  **6,7 bar·s/g** (6,8 la veille). Rapport tasse / amont : 0,81 g/ml
  (0,80).
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 24/10 10 h 22 | 2,0 s | 5,0 s | 8,9 s | 16,1 s |
  | 23/10 15 h 08 | 3,4 s | 7,1 s | 11,2 s | 18,5 s |

**Thermique.**

| Repère | 24/10 10 h 22 | 23/10 15 h 08 |
| --- | ---: | ---: |
| Consigne | 91 °C | 92 °C |
| NTC au départ | 90,72 °C, −0,28 °C | 92,02 °C, +0,02 °C |
| Pic avant l'infusion | 92,18 °C à 11,8 s, +1,18 °C | 93,52 °C à 12,9 s, +1,52 °C |
| Creux du remplissage | 92,06 → 91,44 °C (13 → 15 s), −0,62 °C | 93,51 → 92,70 °C (13 → 20 s), −0,81 °C |
| NTC au début de l'infusion | 91,92 °C, +0,92 °C | 92,55 °C, +0,55 °C |
| Commande à 0 % (remboursement) | 15,4 s, NTC jusqu'à 92,89 °C | 16,0 s, NTC jusqu'à 93,89 °C |
| Appoint repris avant l'arrêt | 1,8 s | 3,3 s |
| **Tasse** | **92,15 °C**, +1,15 °C | 92,86 °C, +0,86 °C |
| Minimum | 88,69 °C, 7,5 s après l'arrêt | 88,80 °C, 6,4 s après |

- Énergie, commande / SSR : précharge 8,58 / 8,42 kJ, remplissage 8,91 /
  9,09 kJ, pré-infusion 2,39 / 2,23 kJ, infusion 1,01 / 1,21 kJ.
- Remplissage : commande de 80 à 83 % (88 à 90 % à 75 %), qui suit le
  débit compté. Énergie par millilitre admis au remplissage : 0,317 kJ/ml
  (0,312 la veille).
- Lecture NTC invalide : 236 ms au premier échantillon de la précharge,
  commande à 90 %.
- Modèle : 92,19 °C avec la commande réelle, **−0,02 °C** (−0,53 °C la
  veille).

**Lecture.**

- *Résistance de la galette, mesuré.* 6,7 bar·s/g contre 6,8, même
  mouture : 2 s de remplissage en plus n'ont pas fait perdre de résistance
  à la galette. Le plateau stable (± 0,05 bar) et le débit constant ne
  montrent pas de channelling. Une seule infusion : un écart de ±0,4
  bar·s/g, celui des deux galettes ViCafé du 16 et du 17 octobre, resterait
  dans le bruit.
- *Mouillage, mesuré.* À débit plus lent, 4 à 5 ml de plus entrent avant
  chaque palier de pression, et la goutte vient à 1,55 bar au lieu de
  5,64 bar. La galette s'est mouillée avant la mise en pression, ce que
  l'entrée du 23 octobre à 15 h 08 jugeait possible seulement avec ≈ 9,5 s
  de pré-infusion. Remplissage et pré-infusion durent ensemble 12,9 s pour
  33,9 ml (11,0 s et 29,9 ml). La montée en pression sans pic va avec une
  galette qui laisse déjà passer le café. Rien ne dit si c'est la durée ou
  le débit qui compte.
- *Creux du remplissage, estimé.* Le creux est plus faible (−0,62 contre
  −0,81 °C) et la NTC commence l'infusion plus haut au-dessus de la
  consigne (+0,92 contre +0,55 °C). Mais la commande suit le débit : à
  0,317 kJ/ml, l'énergie par millilitre est la même. Le modèle, qui ne
  connaît que les volumes et la commande, retrouve la tasse à 0,02 °C près ;
  il n'y a pas d'écart à attribuer au débit de remplissage. Le départ à
  −0,28 °C de la consigne et la purge changée empêchent aussi de comparer
  les creux directement.
- *Température, mesuré.* 92,15 °C en tasse, +1,15 °C. L'entrée de la veille
  estimait ≈ 91,9 °C pour une consigne de 91 °C : +0,25 °C au-dessus.
  −1 °C de consigne n'a donné que −0,71 °C en tasse.
- *Goût, non mesuré.* « Bon », sans « brûlé » cité. Consigne, remplissage
  et purge ont changé ensemble : rien ne sépare leurs effets.
- *Gouttes, mesuré.* 1,7 g après l'arrêt, au-dessus des 0,7 à 1,2 g
  habituels, avec une galette qui coule déjà à basse pression.

**Attendus pour l'infusion suivante** (mêmes réglages, même mouture).

| Point | Attendu |
| --- | --- |
| Hydraulique | 6,3 à 7,3 bar·s/g ; 12 à 16 s depuis 8 bar ; pression au plateau sous 9,2 bar |
| Remplissage | 3,1 à 3,3 ml/s comptés à 70 %, fin sur la pression en 8,5 à 9,5 s |
| Pré-infusion | première goutte sous 2 bar. Une goutte au-dessus de 5 bar à ce débit infirmerait le lien entre remplissage lent et mouillage |
| Chauffe | 0 % pendant 14 à 17 s ; modèle à ±0,6 °C en tasse |
| Tasse | 91,6 à 92,6 °C |

### 24 octobre, 13 h 23 — `261024-132354`

**Contexte.** Même café, même mouture, mêmes réglages qu'à 10 h 22
(consigne de 91 °C, remplissage à 70 %, purge à 80 %), routine guidée.
Observation de David : goût bien.

**Routine** : guide ouvert machine sous la consigne (NTC à 86,3 °C) ;
rinçage 23,5 s après l'ouverture (NTC à 87,3 °C), de 5,01 s, 16,8 ml,
18,0 g, pression maximale 0,11 bar ; retour à la consigne en 104,6 s
(minimum 81,8 °C, résistance active 33,4 s) ; porte-filtre en place
503,2 s à la consigne, **607,8 s sur le groupe** après le rinçage ;
préparation de 86,2 s. Balance absente 14,3 s en tout pendant la routine,
avant l'infusion ; les poids de l'infusion sont continus.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,00 s | — | 0,30 → 0,39 bar |
| Remplissage | 8,00 s | **6,46 s** | **20,0 ml** (3,10 ml/s) | 0,39 → 0,89 bar |
| Pré-infusion | 14,46 s | 4,08 s, échéance | 5,8 ml (1,42 ml/s) | 0,89 → 1,42 bar |
| Infusion | 18,54 s | 17,59 s | 31,0 ml | 9,11 bar max |

- Remplissage fini sur la pression : jusqu'à 3,26 ml/s à 70 %, comme à
  10 h 22, palier à 0,33–0,39 bar.
- Volume admis à chaque palier de pression :

  | Repère | 24/10 13 h 23 (70 %) | 24/10 10 h 22 (70 %) | 23/10 15 h 08 (75 %) |
  | --- | ---: | ---: | ---: |
  | 0,5 bar | **15,7 ml** | 23,8 ml | 18,6 ml |
  | 0,8 bar | 19,4 ml | 27,5 ml | 23,2 ml |
  | 1 bar | 22,0 ml | 31,3 ml | 26,7 ml |
  | 2 bar | 28,1 ml | 36,5 ml | 32,5 ml |

  Au même débit, 8 ml de moins qu'à 10 h 22.
- Pré-infusion finie sur l'échéance. **Première goutte (0,3 g) à 19,53 s,
  à 1,85 bar**, 27,5 ml, 1,0 s après le début de l'infusion.
- Montée de 3,4 s jusqu'à 8 bar, 2,3 g en tasse ; **pic à 9,11 bar** à
  22,39 s, 0,5 s après 8 bar, puis 8,8 bar.
- Depuis 8 bar : 14,1 s, 1,36 g/s en tasse, 1,55 ml/s en amont. 21,4 g à
  l'arrêt, 23,0 g final, **1,6 g de gouttes**. Pompe en marche 28,1 s.
- Plateau : 8,88 ± 0,05 bar, pompe à 68 %, 1,37 g/s, soit
  **6,5 bar·s/g** (6,7 à 10 h 22, 6,8 la veille).
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 24/10 13 h 23 | 2,3 s | 5,2 s | 9,2 s | 16,5 s |
  | 24/10 10 h 22 | 2,0 s | 5,0 s | 8,9 s | 16,1 s |
  | 23/10 15 h 08 | 3,4 s | 7,1 s | 11,2 s | 18,5 s |

**Thermique.**

| Repère | 24/10 13 h 23 | 24/10 10 h 22 |
| --- | ---: | ---: |
| NTC au départ | 91,05 °C, +0,05 °C | 90,72 °C, −0,28 °C |
| NTC pendant le remplissage | **+1,27 °C**, sans creux | −0,62 °C de creux |
| Pic avant l'infusion | 92,78 °C à 17,4 s | 92,18 °C à 11,8 s |
| NTC au début de l'infusion | 92,67 °C, +1,67 °C | 91,92 °C, +0,92 °C |
| Commande à 0 % (remboursement) | 15,2 s, NTC jusqu'à 93,25 °C | 15,4 s |
| Appoint repris avant l'arrêt | 2,1 s | 1,8 s |
| **Tasse** | **91,82 °C**, +0,82 °C | 92,15 °C, +1,15 °C |
| Minimum | 88,10 °C, 8,0 s après l'arrêt | 88,69 °C, 7,5 s après |

- Énergie, commande / SSR : précharge 8,49 / 8,59 kJ, remplissage 6,43 /
  5,82 kJ, pré-infusion 2,21 / 1,71 kJ, infusion 1,16 / 1,26 kJ.
- Remplissage : commande de 75 à 90 %, 0,32 kJ/ml admis.
- Aucune lecture NTC invalide.
- Modèle : 92,44 °C avec la commande réelle, **−0,60 °C**.

**Attendus vérifiés** (entrée de 10 h 22).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 6,3 à 7,3 bar·s/g ; 12 à 16 s depuis 8 bar ; plateau sous 9,2 bar | 6,5 bar·s/g, 14,1 s, pic à 9,11 bar : atteint |
| Remplissage | 3,1 à 3,3 ml/s à 70 %, fin en 8,5 à 9,5 s | 3,10 à 3,26 ml/s : atteint ; **6,46 s : non atteint** |
| Pré-infusion | première goutte sous 2 bar | 1,85 bar : atteint |
| Chauffe | 0 % pendant 14 à 17 s ; modèle à ±0,6 °C | 15,2 s : atteint ; −0,60 °C : à la limite |
| Tasse | 91,6 à 92,6 °C | 91,82 °C : atteint |

**Lecture.**

- *Mouillage, mesuré.* La lecture de 10 h 22 ne tient pas : au même débit
  de remplissage, la pression monte 8 ml plus tôt, et le volume admis avant
  0,5 bar varie de 15,7 à 23,8 ml sans changement de réglage. Ce volume
  dépend de la galette et de l'espace au-dessus (dose, tassage), pas du
  débit. La goutte arrive pourtant de nouveau sous 2 bar (1,85 bar), avec
  8 ml de moins qu'à 10 h 22 : la goutte précoce ne vient pas d'un plus
  grand volume admis. Elle va avec le remplissage à 70 % ou avec le café,
  un jour plus vieux (16 jours depuis la torréfaction) ; rien ne sépare
  les deux.
- *Résistance de la galette, mesuré.* 6,5 bar·s/g : les trois dernières
  galettes tiennent entre 6,5 et 6,8. Toujours pas de perte de résistance
  au remplissage lent.
- *Pic de pression, mesuré.* Il revient (9,11 bar, 0,5 s après 8 bar) :
  l'absence de pic de 10 h 22 n'est pas reproduite.
- *Chauffe du remplissage, mesuré.* Pas de creux : la NTC monte de 1,27 °C,
  sur un remplissage plus court (6,46 contre 8,98 s) et avec 8 ml d'eau
  froide en moins. Comme le 20 octobre (+2,25 °C sur 21,4 ml), le creux
  suit le volume admis au remplissage plus que son débit.
- *Température, mesuré.* 91,82 °C en tasse, +0,82 °C, malgré un départ
  1,67 °C au-dessus de la consigne à l'infusion. Le modèle surestime la
  tasse de 0,60 °C, à la limite de la plage. 607,8 s de porte-filtre sur
  le groupe, le plus long depuis le 20 octobre (962 s), pouvaient laisser
  attendre une tasse plutôt chaude ; elle ne l'est pas.
- *Gouttes, mesuré.* 1,6 g, comme à 10 h 22 (1,7 g) : les deux galettes
  qui coulent tôt gouttent plus après l'arrêt.
- *Goût, non mesuré.* Bien, deux fois de suite aux mêmes réglages.

**Décision** (David). Le remplissage plus lent ne nuit pas à la galette ;
réglages conservés (configuration sauvegardée à 13 h 28, identique à celle
de 10 h 25 : consigne de 91 °C, remplissage à 70 %, purge à 80 %).

### 25 octobre, 9 h 11 — `261025-091128`

**Contexte.** Même café (Cozy Chocolate, 17 jours depuis la torréfaction),
même mouture, routine guidée. Écran **0.4.23** : les réglages passent par
le profil « single espresso » (configuration sauvegardée le 24 octobre à
15 h 55), avec les mêmes valeurs que la veille. Consigne de 91 °C,
remplissage à 70 %, purge à 80 %. La capture passe au schéma v3, qui ajoute
le firmware et le profil ; ses échantillons sont ceux du v2. Observations de David : **mouture un peu mieux tassée** ;
**galette passablement dégradée** au visuel, sans signe de channelling
massif ; au goût, **une amertume pas très agréable**.

**Routine** : guide ouvert machine sous la consigne (NTC à 87,3 °C) ;
rinçage 15,7 s après l'ouverture (NTC à 88,2 °C), de 5,02 s, 18,8 ml,
28,7 g, pression maximale 0,30 bar ; retour à la consigne en 62,2 s
(minimum 84,5 °C, résistance active 22,9 s) ; porte-filtre en place
353,4 s à la consigne, **415,6 s sur le groupe** après le rinçage ;
préparation de 108,6 s. Balance absente 11,8 s en tout pendant la
routine, avant l'infusion ; les poids de l'infusion sont continus.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | **0,40 → 0,46 bar** |
| Remplissage | 8,01 s | 6,84 s | 21,4 ml (3,13 ml/s) | 0,46 → 0,85 bar |
| Pré-infusion | 14,85 s | **3,50 s, goutte** | 5,2 ml (1,49 ml/s) | 0,85 → 1,28 bar |
| Infusion | 18,35 s | **20,31 s** | 32,2 ml | 9,03 bar max |

- Remplissage fini sur la pression (premier paquet à 0,8 bar à 14,69 s,
  20,9 ml), jusqu'à 3,27 ml/s à 70 %. Le circuit part de 0,40 bar
  (0,22 à 0,30 bar les deux dernières fois) : le repère de 0,5 bar est
  atteint dès le premier échantillon et ne se compare pas.
- Volume admis à chaque palier de pression :

  | Repère | 25/10 9 h 11 | 24/10 13 h 23 | 24/10 10 h 22 |
  | --- | ---: | ---: | ---: |
  | 0,8 bar | 20,9 ml | 19,4 ml | 27,5 ml |
  | 1 bar | 23,5 ml | 22,0 ml | 31,3 ml |
  | 2 bar | 29,9 ml | 28,1 ml | 36,5 ml |
  | 8 bar | 36,8 ml | 34,8 ml | 43,2 ml |

- Pré-infusion **finie sur la goutte**, à 3,50 s, une première depuis le
  changement de café : **première goutte (0,3 g) à 18,35 s, à 1,28 bar**,
  26,7 ml.
- Montée de 3,6 s jusqu'à 8 bar, 2,8 g en tasse. **Pas de pic** : la
  pompe passe de 92 à 65 % et la pression monte lentement de 8,2 à
  8,9 bar en 8 s ; le maximum, 9,03 bar, vient à 36,7 s, 2 s avant
  l'arrêt.
- Depuis 8 bar : **16,8 s**, **1,09 g/s** en tasse, 1,29 ml/s en amont
  (14,1 s, 1,36 g/s et 1,55 ml/s à 13 h 23). 21,5 g à l'arrêt, 22,5 g
  final, **0,9 g de gouttes**. Pompe en marche 30,5 s.
- Plateau (de 8 bar + 2 s à l'arrêt) : 8,86 ± 0,09 bar, pompe à 66 %,
  1,13 g/s, soit **7,8 bar·s/g** (6,5 à 13 h 23 et à 10 h 22 avec la même
  mesure). Le débit en tasse reste constant jusqu'à l'arrêt, sans
  accélération ni chute de pression. Rapport tasse / amont : 0,84 g/ml.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 25/10 9 h 11 | 1,6 s | 5,7 s | 10,2 s | **19,1 s** |
  | 24/10 13 h 23 | 2,3 s | 5,2 s | 9,2 s | 16,5 s |
  | 24/10 10 h 22 | 2,0 s | 5,0 s | 8,9 s | 16,1 s |

**Thermique.**

| Repère | 25/10 9 h 11 | 24/10 13 h 23 |
| --- | ---: | ---: |
| NTC au départ | 91,00 °C, +0,00 °C | 91,05 °C, +0,05 °C |
| NTC pendant le remplissage | +1,24 °C, sans creux | +1,27 °C, sans creux |
| Pic avant l'infusion | 92,94 °C à 12,5 s | 92,78 °C à 17,4 s |
| NTC au début de l'infusion | 92,35 °C, +1,35 °C | 92,67 °C, +1,67 °C |
| Commande à 0 % (remboursement) | **16,5 s**, NTC sans remontée | 15,2 s, NTC jusqu'à 93,25 °C |
| Appoint repris avant l'arrêt | **3,8 s** | 2,3 s (même mesure ; 2,1 s dans l'entrée) |
| NTC à l'arrêt | 89,30 °C | 89,52 °C |
| **Tasse** | **91,17 °C**, +0,17 °C | 91,82 °C, +0,82 °C |
| Minimum | 87,89 °C, 6,1 s après l'arrêt | 88,10 °C, 8,0 s après |

- Énergie, commande / SSR : précharge 8,50 / 8,56 kJ, remplissage 6,77 /
  6,45 kJ, pré-infusion 2,27 / 2,27 kJ, infusion 1,60 / 1,81 kJ.
- Remplissage : commande de 71 à 90 %, 0,32 kJ/ml admis.
- Aucune lecture NTC invalide.
- Moyenne en volume amont : 91,90 °C.
- Modèle : 92,23 °C avec la commande réelle, **−1,03 °C** (mesure du
  simulateur : 91,20 °C).

**Attendus vérifiés.** L'entrée de 13 h 23 n'en fixait pas ; ceux de
10 h 22 portaient sur les mêmes réglages.

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 6,3 à 7,3 bar·s/g ; 12 à 16 s depuis 8 bar ; plateau sous 9,2 bar | **7,8 bar·s/g, 16,8 s : non atteint** ; 9,03 bar : atteint |
| Remplissage | 3,1 à 3,3 ml/s à 70 %, fin en 8,5 à 9,5 s | 3,13 à 3,27 ml/s : atteint ; **6,84 s : non atteint**, comme à 13 h 23 |
| Pré-infusion | première goutte sous 2 bar | 1,28 bar : atteint |
| Chauffe | 0 % pendant 14 à 17 s ; modèle à ±0,6 °C | 16,5 s : atteint ; **−1,03 °C : non atteint** |
| Tasse | 91,6 à 92,6 °C | **91,17 °C : non atteint** |

**Lecture.**

- *Résistance de la galette, mesuré.* 7,8 bar·s/g contre 6,5 et 6,7 les
  trois dernières fois : +20 %, au-delà de l'écart de ±0,4 bar·s/g pris
  pour du bruit. Le tassage plus ferme est le seul changement signalé ;
  le lien est probable mais pas mesuré : le café a vieilli d'un jour, et
  rien ne sépare les deux.
- *Channelling, mesuré.* Le plateau ne le montre pas : pression stable à
  ± 0,09 bar, sans chute, et débit en tasse constant jusqu'à l'arrêt.
  L'aspect dégradé de la galette ne se lit pas dans la capture. La
  galette reste sous ≈ 9 bar pendant la récupération (8,91 → 9,28 bar),
  comme dans toutes les captures depuis le 23 octobre : ce n'est pas un
  changement.
- *Mouillage, mesuré.* La goutte arrive tôt (1,28 bar) et termine la
  pré-infusion pour la première fois sur ce café, avec 1,5 à 2 ml de plus
  admis qu'à 13 h 23 à chaque palier. Le remplissage à 70 % donne une
  troisième goutte sous 2 bar.
- *Amertume, estimé.* La galette plus résistante allonge l'infusion :
  20,31 s de phase contre 17,59, 20 g en 19,1 s au lieu de 16,5, à 22 g
  comme avant. Plus de temps de contact au même rendement va avec plus
  d'extraction, et l'amertume avec une extraction poussée ; c'est une
  hypothèse, pas une mesure. La température ne l'explique pas : la tasse
  est la plus froide depuis le 23 octobre à 14 h 38. Le 23 octobre à
  15 h 08 (6,8 bar·s/g, 15,9 s), le « brûlé » venait sans galette lente :
  l'amertume n'a donc pas une seule cause sur ce café.
- *Température, mesuré.* 91,17 °C en tasse, +0,17 °C, 0,65 °C sous
  13 h 23. L'infusion part 0,32 °C plus bas (92,35 contre 92,67 °C) et la
  NTC ne remonte pas pendant le remboursement ; l'appoint revient 3,8 s
  avant l'arrêt. Une infusion plus longue fait couler plus de café
  pendant la descente de la NTC (92,35 → 89,30 °C). Le modèle surestime
  la tasse de 1,03 °C, hors de la plage de ±0,6 °C, et de 0,4 °C de plus
  qu'à 13 h 23 ; cause non identifiée.
- *Gouttes, mesuré.* 0,9 g, retour dans la plage habituelle (1,6 et
  1,7 g la veille).

### 26 octobre, 9 h 34 — `261026-093443`

**Contexte.** Premier **turbo shot**, profil « turbo shot » de la
configuration `configs/261026-093452.json` : **11,0 g** dans un panier
**58 mm 15 g**, cible **45 g**, pression cible **6 bar**, pompe jusqu'à
100 %, consigne de 91 °C ; remplissage, pré-infusion et purge identiques au
profil « single espresso » ; `rampdown.lead_weight_g` de 1,0 g (0,5 g pour
l'espresso). Même café (Cozy Chocolate, 18 jours depuis la torréfaction) et
**même mouture que le single espresso** : David veut faire les deux cafés
sans toucher au moulin. Écran et carte sensors **0.4.24**. Recettes et
définition du turbo shot : [turboshot.md](turboshot.md). Observations de
David : café **pratiquement pas tassé, juste mis à niveau** ; pas de café
visible avant l'infusion ; 3 g au-dessus de la cible ; au goût, **très
intéressant, zéro amertume, notes florales**.
Correction du 26 octobre, 13 h 32 : David a tassé normalement l'essai
suivant et constate que ce « faible tassage » était déjà un tassage
maximal ; la galette de 9 h 34 était tassée.

**Routine** : guide ouvert à la consigne (NTC à 90,9 °C) ; rinçage de
5,04 s, 18,3 ml, 24,0 g, pression maximale 0,26 bar ; consigne retrouvée à
la fin du rinçage, minimum 85,6 °C ensuite, résistance active 24,2 s ;
porte-filtre **417,6 s sur le groupe** après le rinçage ; préparation de
130,6 s. Balance absente 15,2 s en tout pendant la routine, avant
l'infusion ; les poids de l'infusion sont continus.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,02 s | — | 0,24 → 0,34 bar |
| Remplissage | 8,02 s | **10,05 s, échéance** | 28,7 ml (2,91 ml/s) | 0,34 → 0,77 bar |
| Pré-infusion | 18,07 s | 1,92 s, goutte | 2,6 ml (1,51 ml/s) | 0,77 → 0,80 bar |
| Infusion | 19,99 s | **10,88 s** | 37,4 ml (3,44 ml/s) | **2,17 bar max** |

- Remplissage fini à l'échéance de 10 s, à 0,77 bar, sous le seuil de
  0,8 bar ; 2,8 à 3,1 ml/s comptés à 70 %. La pression reste à 0,24 à
  0,31 bar jusqu'à 21 ml (15,5 s), puis monte de 0,44 à 0,77 bar sur les
  7 derniers ml. L'espace libre du panier de 58 mm avec 11 g est plus grand
  que celui du LM1 avec 8 g : 28,7 ml admis sans goutte, 21,4 ml le 25/10.
  La balance reste à 0,0 g pendant tout le remplissage.
- Pré-infusion **finie sur la goutte** après 1,92 s : la balance passe de
  0,0 à 0,3 g au passage en infusion, à 0,80 bar, **32,2 ml** (26,7 ml le
  25/10). Le firmware passe en infusion à la première goutte : aucun café
  ne peut couler pendant la pré-infusion, ce que David a vu.
- Montée : la pompe passe de 50 à 100 % en 2,9 s et y reste jusqu'à
  l'arrêt. La pression atteint 2,0 bar à 23,5 s, **plafonne à 2,1–2,2 bar**
  de 24 à 28 s, puis redescend à 2,0 bar dans les 2 dernières secondes à
  commande constante. La cible de 6 bar n'est jamais approchée : le PI de
  pression ne s'active pas, la pompe est saturée.
- Débit : 3,7 ml/s comptés en amont du début à la fin du plateau, soit
  ≈ 5,2 g/s avec le sous-comptage de 1,42 mesuré en débit libre (le débit
  libre à 100 % est de 5,68 g/s). **La pompe débite presque son débit
  libre** ; le shot est borné par la pompe, pas par la galette.
- Arrivée en tasse, depuis le début de l'infusion :

  | 1 g | 5 g | 10 g | 20 g | 30 g | 40 g | 44 g |
  | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | 0,8 s | 2,9 s | 4,2 s | 6,0 s | 8,1 s | 10,1 s | 10,9 s |

  De 10 à 40 g : **5,05 g/s**, régulier (5,4, 4,7 et 5,0 g/s par tranche
  de 10 g).
- Résistance au plateau (24 à 30 s) : 2,12 bar pour 4,9 g/s, soit
  **0,43 bar·s/g**, 18 fois moins que l'espresso du 25/10 (7,8 bar·s/g).
- Arrêt : seuil de la pompe à `45 − 1,0 = 44,0 g`, franchi à 30,87 s
  (43,3 → 44,4 g) ; 45,3 g à l'arrêt rapporté de la pompe, **48,5 g
  final**, stable dès 33,6 s. **4,5 g arrivent après le seuil**, soit
  ≈ 0,9 s au débit de la fin ; le 25/10, 1,0 g après le seuil de 21,5 g à
  1,13 g/s, soit le même ≈ 0,9 s. Écart à la cible : **+3,5 g**.
- Après l'arrêt, la pression lue monte de 1,92 à 2,87 bar pompe arrêtée
  alors que la balance ne bouge plus : elle ne représente plus la galette.
- Ratio final **1:4,4** (1:4,1 visé). Pompe en marche 22,9 s, dont 12 s
  avant la première goutte.

**Thermique.**

| Repère | Mesure |
| --- | ---: |
| NTC au départ | 91,05 °C, +0,05 °C |
| Pic avant l'infusion | 92,38 °C à 11,6 s, pendant le remplissage |
| NTC au début de l'infusion | 90,45 °C, −0,55 °C |
| Commande à 0 % (remboursement) | **9,4 s**, de 19,99 à 29,42 s |
| Appoint repris avant l'arrêt | 1,45 s |
| NTC à l'arrêt | 88,34 °C, minimum pendant l'écoulement |
| **Tasse** | **89,37 °C**, −1,63 °C |
| Minimum | **84,59 °C**, 8,3 s après l'arrêt ; 84,66 °C en fin de capture (10 s) |

- Énergie, commande / SSR : précharge 8,51 / 8,52 kJ, remplissage 9,34 /
  8,74 kJ (0,32 kJ/ml admis), pré-infusion 1,47 / 1,44 kJ, infusion 1,57 /
  1,60 kJ.
- 71,3 ml admis en tout, contre ≈ 40 ml pour l'espresso : l'infusion admet
  37 ml en 10,9 s et la résistance est à 0 % presque tout ce temps. La NTC
  baisse de 2,1 °C pendant l'écoulement et de 3,8 °C de plus après.
- Moyenne en volume amont : 90,50 °C.
- Aucune lecture NTC invalide.
- Modèle : 91,48 °C avec la commande réelle, **−2,07 °C** (mesure du
  simulateur : 89,41 °C), le plus grand écart depuis le 23/10 à 14 h 38.
  Le modèle n'a été ajusté que sur des débits d'espresso ; la mesure n'est
  pas comparable.

**Attendus vérifiés.** Premier essai du profil : aucun attendu.

**Lecture.**

- *Pression, mesuré.* La galette n'offre pas assez de résistance pour
  que la pompe, à pleine puissance, dépasse 2,2 bar. Les 6 bar n'ont servi
  que de plafond, jamais atteint. Pour une même mouture, la géométrie
  explique une partie de l'écart : à tassement égal, la résistance d'un lit
  varie comme la dose divisée par le carré de la surface, soit
  `(11 / 8) × (41 / 58)⁴ = 0,34` de celle de l'espresso, ≈ 2,2 à
  2,7 bar·s/g (estimé). Les 0,43 bar·s/g mesurés sont 5 à 6 fois plus bas :
  le reste vient probablement de l'absence de tassage et d'un lit moins
  compacté à 2 bar qu'à 9 bar ; rien ne sépare les deux.
  Correction du 26 octobre, 13 h 32 : la galette était tassée (voir le
  contexte) ; l'absence de tassage n'explique pas l'écart, et l'essai de
  13 h 32, tassé de même, mesure 0,38 bar·s/g.
- *Débit, mesuré.* ≈ 5 g/s en tasse, dans la plage de 4,5 à 5 g/s de
  Cameron ; avec un débit libre sous 7 g/s, la règle de Hedrick vise 11 à
  14 s d'écoulement : 10,9 s ici.
- *Fin au poids, mesuré.* Le café qui arrive après le seuil correspond à
  ≈ 0,9 s de débit, comme pour l'espresso : l'avance fixe de 1,0 g valait
  pour 1,1 g/s, pas pour 5 g/s.
- *Mouillage, mesuré.* Le remplissage à 70 % ne finit pas sur la pression
  parce que l'espace libre est plus grand ; avec 10 s d'échéance, il fait
  l'essentiel du mouillage, à moins de 0,8 bar. Les recettes publiées
  partent sans pré-infusion ; ces 12 s avant la goutte n'y existent pas.
- *Goût, estimé.* Zéro amertume et notes florales vont avec ce que
  décrivent les sources pour un écoulement rapide à basse pression ; à
  1:4,4, la boisson est aussi plus diluée qu'un turbo à 1:3. Sans TDS,
  rien ne dit l'extraction.
- *Température, mesuré.* 89,37 °C en tasse, la plus froide depuis le
  23/10 à 14 h 38, parce que deux fois plus d'eau froide entre pendant que
  le remboursement de la précharge tient la résistance à 0 %. Le minimum de
  84,59 °C après l'arrêt allonge le retour à la consigne avant le café
  suivant.

**Pistes retenues par David** pour les turbo shots suivants : température
plus basse ; tassage normal, comme pour le single espresso, au lieu d'une
mise à niveau ; puck screen pour ménager la galette ; **35 g** au lieu de
45 g. La mouture reste celle du single espresso.

**Attendus pour le prochain turbo shot** (estimés, à affiner avec les
valeurs que David choisira).

| Point | Attendu |
| --- | --- |
| Résistance | entre 0,43 et ≈ 2,5 bar·s/g selon l'effet du tassage. Le papier de Cameron n'a pas vu d'effet notable du tassage sur le temps à 6 bar ; le tassage « un peu plus ferme » du 25/10 a donné +20 % sur l'espresso. La pression n'atteint 6 bar qu'à partir de ≈ 1,2 bar·s/g (6 bar / 5 g/s), plus si la pompe débite moins à 6 bar ; le PI de pression prend alors la main |
| Débit et temps | tant que la pression reste sous ≈ 4 bar, ≈ 5 g/s : 35 g en ≈ 7 s depuis la goutte ; à 6 bar avec 2 bar·s/g, ≈ 3 g/s et ≈ 12 s |
| Poids final | avec `lead_weight_g` à 1,0 g : 34 g + 0,9 s × débit, soit ≈ 38,5 g à 5 g/s et ≈ 36,7 g à 3 g/s. Une avance de 0,9 s × débit attendu (≈ 4,5 g à 5 g/s, ≈ 2,7 g à 3 g/s) ramène à 35 g |
| Remplissage | le puck screen réduit l'espace libre de quelques ml : première goutte avant 32 ml, fin du remplissage peut-être sur la pression |
| Tasse | ≈ consigne − 1,6 °C si le débit reste à 5 g/s ; moins d'écart si le tassage ralentit le débit |

Séparation des effets : la température se lit à part dans la NTC, et le
ratio dans le poids ; le tassage et le puck screen changent tous deux la
résistance, et rien dans la capture ne les sépare s'ils changent ensemble.
Au goût, rien ne sépare les quatre.

### 26 octobre, 13 h 32 — `261026-133226`

**Contexte.** Deuxième **turbo shot**, profil « turbo shot » de
`configs/261026-142235.json` : par rapport à 9 h 34, **cible 40 g** (45 g)
et **consigne de 88 °C** (91 °C) ; dose de 11,0 g, panier 58 mm 15 g,
6 bar, pompe jusqu'à 100 %, remplissage, pré-infusion et
`lead_weight_g` de 1,0 g inchangés. Même café, même mouture. Écran et
carte sensors 0.4.24. Observations de David : **tassage normal, comme ce
matin** — en essayant de tasser plus, il constate que le « faible
tassage » de 9 h 34 était déjà un tassage maximal ; pas de puck screen. Au
goût : **bon, mais clairement très léger** ; pas d'amertume, une légère
acidité ; **pas de crema**. Pas encore de référence de turbo shot pour
comparer.

Deux réglages changent : la température se lit dans la NTC, le ratio dans
le poids ; au goût, rien ne les sépare.

**Routine** : guide ouvert à la consigne (NTC à 88,0 °C) ; rinçage de
5,04 s, 18,6 ml, 29,0 g, pression maximale 0,31 bar ; consigne retrouvée à
la fin du rinçage, minimum 82,6 °C ensuite, résistance active 73,5 s ;
porte-filtre **1681,3 s sur le groupe** (28 min) après le rinçage ;
préparation de 197,6 s. Balance présente toute la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,11 s | — | 0,28 → 0,35 bar |
| Remplissage | 8,11 s | **7,90 s, pression** | 25,2 ml (3,19 ml/s) | 0,35 → 0,89 bar |
| Pré-infusion | 16,00 s | 0,79 s, goutte | 1,2 ml (1,47 ml/s) | 0,89 → 0,90 bar |
| Infusion | 16,79 s | **9,57 s** | 33,6 ml (3,51 ml/s) | **1,98 bar max** |

- Remplissage **fini sur la pression** : premier paquet au-dessus de
  0,8 bar à 15,83 s (0,85 bar), passage en pré-infusion à 16,00 s
  (0,89 bar), 0,17 s d'écart. 25,2 ml admis, contre 28,7 ml sans atteindre
  0,8 bar à 9 h 34 ; même dose, même tassage selon David, pas de puck
  screen : rien dans la capture n'explique ces 3,5 ml.
- Première goutte à **26,4 ml** depuis le remplissage (32,2 ml à 9 h 34),
  8,69 s après son début ; marge de remplissage 1,2 ml (3,5 ml).
- Montée : pompe de 50 à 100 % en 2,7 s ; 1 bar à 17,49 s. La pression
  **plafonne à 1,9–2,0 bar** de 21,3 s à l'arrêt, 1,98 bar au plus ; pompe
  saturée, le PI de pression ne s'active pas.
- Débit au plateau (21,3 à 26,4 s) : 3,74 ml/s comptés en amont,
  5,16 g/s en tasse, rapport 1,38 (1,42 en débit libre).
- Arrivée en tasse, depuis le début de l'infusion :

  | 1 g | 5 g | 10 g | 20 g | 30 g | 35 g | 39 g (seuil) | 44 g |
  | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | 0,8 s | 2,5 s | 3,8 s | 5,8 s | 8,0 s | 8,6 s | 9,6 s | 11,0 s |

  De 10 à 39,6 g : **4,93 g/s** (4,9, 4,8 et 5,1 g/s par tranche).
- Résistance au plateau : 1,94 bar pour 5,16 g/s, soit
  **0,38 bar·s/g** (0,43 à 9 h 34).
- Arrêt : seuil de `40 − 1,0 = 39,0 g` franchi à 26,37 s (38,4 →
  39,6 g) ; **44,2 g final**, stable dès ≈ 28,8 s. **5,2 g arrivent après
  le seuil**, soit ≈ 1,0 s au débit de la fin (4,5 g et ≈ 0,9 s à 9 h 34).
  Écart à la cible : **+4,2 g**.
- Après l'arrêt, la pression lue monte de 1,88 à 2,51 bar pompe arrêtée,
  balance immobile, comme à 9 h 34.
- Ratio final **1:4,0** (1:3,6 visé). Pompe en marche 18,4 s, dont 8,7 s
  avant la première goutte (12 s à 9 h 34). 61,7 ml admis en tout
  (71,3 ml).

**Thermique.**

| Repère | Mesure |
| --- | ---: |
| NTC au départ | 88,01 °C, +0,01 °C |
| Pic avant l'infusion | 90,17 °C à 12,9 s, pendant le remplissage, +2,17 °C (+1,38 °C à 9 h 34) |
| NTC au début de l'infusion | **89,33 °C, +1,33 °C** (−0,55 °C à 9 h 34) |
| Commande à 0 % (remboursement) | 8,9 s, de 16,79 à 25,70 s |
| Appoint repris avant l'arrêt | 0,67 s |
| NTC à l'arrêt | 87,92 °C, minimum pendant l'écoulement |
| **Tasse** | **88,74 °C, +0,74 °C** (−1,63 °C à 9 h 34) |
| Minimum | **83,71 °C**, 6,4 s après l'arrêt ; 83,97 °C en fin de capture (10 s) |

- Énergie, commande / SSR : précharge 8,60 / 8,80 kJ, remplissage 7,85 /
  7,54 kJ (0,31 kJ/ml admis), pré-infusion 0,75 / 0,81 kJ, infusion 0,73 /
  0,82 kJ.
- La NTC baisse de 1,4 °C pendant l'écoulement et de 4,2 °C de plus
  après l'arrêt.
- Moyenne en volume amont : 89,10 °C.
- Aucune lecture NTC invalide.
- Modèle : 87,91 °C avec la commande réelle, **+0,85 °C** (mesure du
  simulateur : 88,76 °C) ; −2,07 °C à 9 h 34. Hors de son domaine de
  débit, le modèle se trompe dans les deux sens d'un essai à l'autre.

**Attendus vérifiés.**

| Attendu (9 h 34) | Mesure |
| --- | --- |
| Résistance entre 0,43 et ≈ 2,5 bar·s/g selon l'effet du tassage | **0,38 bar·s/g, non atteint** : le tassage n'a pas changé (voir la correction de 9 h 34) et il n'y a pas de puck screen ; la pression reste sous 2 bar |
| ≈ 5 g/s tant que la pression reste sous ≈ 4 bar ; 35 g en ≈ 7 s depuis la goutte | 4,93 g/s, atteint ; 35 g à **8,6 s**, la montée de la pompe (2,7 s jusqu'à 100 %) n'était pas comptée |
| Poids final ≈ seuil + 0,9 s × débit | 39,0 + 0,9 × 4,9 = 43,4 g attendus, **44,2 g** mesurés (≈ 1,0 s) |
| Première goutte avant 32 ml, fin du remplissage peut-être sur la pression, avec le puck screen | **26,4 ml et fin sur la pression**, mais sans puck screen ; la cause n'est pas identifiée |
| Tasse ≈ consigne − 1,6 °C à 5 g/s | **consigne + 0,74 °C, non atteint** |

**Lecture.**

- *Tassage, mesuré.* À tassage égal selon David, la résistance passe de
  0,43 à 0,38 bar·s/g : le tassage n'est pas un levier sur cette galette,
  et l'écart de 5 à 6 fois avec l'estimation géométrique (≈ 2,2 à
  2,7 bar·s/g) reste inexpliqué. Le rapport mesuré entre l'espresso du
  25/10 et ce turbo est de 7,8 / 0,38 ≈ 20, contre ≈ 3 par la géométrie
  (estimé).
- *Pression, mesuré.* Avec la mouture du single espresso et 11 g dans ce
  panier, la pompe à 100 % ne dépasse pas 2 bar. Atteindre 6 bar à 5 g/s
  demande ≈ 1,2 bar·s/g, 3,2 fois la résistance mesurée. À tassement égal,
  15 g (la capacité du panier) la multiplieraient par ≈ 1,4 (estimé) :
  ≈ 0,5 bar·s/g, ≈ 2,6 bar. Sans changer la mouture, aucun levier connu
  ne donne 6 bar dans ce panier.
- *Débit et temps, mesuré.* 4,9 g/s et 9,6 s de la goutte au seuil, dans
  les repères de Cameron (4,5 à 5 g/s) mais sous les 11 à 14 s de Hedrick,
  parce que la cible est plus basse.
- *Fin au poids, mesuré.* Comme à 9 h 34, ≈ 1 s de débit arrive après le
  seuil : à ≈ 5 g/s, une avance de ≈ 5 g mettrait le poids final sur la
  cible.
- *Température, mesuré.* La consigne baisse de 3 °C, la tasse de
  **0,63 °C** seulement (89,37 → 88,74 °C) : la NTC est 1,33 °C au-dessus
  de la consigne au début de l'infusion, contre 0,55 °C en dessous à
  9 h 34. Cause principale probable, estimée : **28 min de porte-filtre
  sur le groupe** contre 7 min. Le modèle, qui ne connaît que la
  chaudière, passe de 2,07 °C au-dessus de la mesure à 0,85 °C en dessous :
  le sens de la piste du temps minimal sur le groupe
  ([wizard.md](wizard.md#piste--temps-minimal-sur-le-groupe)), où 642 s au
  lieu de 148 s plaçaient la mesure 0,5 °C au-dessus du modèle ; l'écart
  est ici bien plus grand. La NTC ne voit le porte-filtre
  qu'indirectement, par la chaleur que le groupe tire moins à la
  chaudière ; la boisson, elle, en profite directement. Varient aussi :
  6 ml de moins admis avant la goutte pour la même précharge (8,60 kJ,
  8,51 kJ à 9 h 34) et une préparation plus longue (197,6 s, 130,6 s à
  9 h 34), porte-filtre hors du groupe. Rien ne les sépare dans la
  capture ; le modèle n'a pas été rejoué avec l'énergie SSR.
- *Goût, estimé.* « Très léger » va avec une boisson diluée (1:4,0) et
  extraite à 2 bar ; l'absence de crema avec une pression basse. La
  légère acidité, nouvelle par rapport aux notes florales de 9 h 34, peut
  venir du ratio plus court comme de la tasse un peu plus froide ; rien ne
  sépare les deux, et sans TDS rien ne dit l'extraction.

**Suite.** Réglages à choisir par David. Ce que la capture chiffre pour
chaque levier :

| Levier | Effet attendu |
| --- | --- |
| `lead_weight_g` à 5,0 g, cible inchangée | poids final ≈ cible ± 0,5 g à 5 g/s |
| Ratio plus court (cible 33 à 35 g) | 1:3 à 1:3,2 ; ≈ 8 s depuis la goutte |
| Dose de 13 à 15 g, même panier | résistance ≈ 0,45 à 0,5 bar·s/g, 2,2 à 2,6 bar ; boisson plus concentrée à ratio égal (estimé) |
| Consigne | la tasse était à −1,63 °C puis +0,74 °C de la consigne : l'écart change de signe d'un essai à l'autre, il faut une mesure de plus avant de choisir la consigne pour une température en tasse |

### 26 octobre, 15 h 35 — `261026-153548`

**Contexte.** Troisième **turbo shot**, profil « turbo shot » de
`configs/261026-153711.json` : par rapport à 13 h 32, **dose de 15,0 g**
dans le même panier 58 mm 15 g, **cible 45 g** (1:3), **`lead_weight_g`
de 5,0 g** ; consigne de 88 °C, 6 bar, pompe jusqu'à 100 %, remplissage
et pré-infusion inchangés. Le champ `dose_g` du profil est resté à 11,0 g.
Même café, **même mouture**, même tassage, sans puck screen. Écran et
carte sensors 0.4.24. Observations de David : la galette résiste moins
que celle du LM1 à la même mouture ; au goût, **très intéressant : un
espresso long, crema onctueuse, goût caractéristique de l'espresso,
légère amertume**, loin de la « soupe » plus fruitée d'avant ; un café
proche de ce qu'on sert dans les restaurants en Suisse, mais éloigné du
turbo shot selon lui, notamment par le temps.

La dose et la cible changent ensemble : le ratio se lit dans le poids ;
la résistance et le goût mêlent les deux.

**Routine** : guide ouvert sous la consigne (NTC à 86,4 °C) ; rinçage de
5,02 s, 18,8 ml, 24,3 g, pression maximale 0,51 bar ; consigne retrouvée
125,5 s après la fin du rinçage, minimum 80,1 °C, résistance active
63,8 s ; porte-filtre **1205,6 s sur le groupe** (20 min) ; préparation de
85,2 s. Balance présente toute la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,00 s | — | 0,41 → 0,47 bar |
| Remplissage | 8,00 s | **6,28 s, pression** | 19,7 ml (3,14 ml/s) | 0,47 → 0,97 bar |
| Pré-infusion | 14,28 s | **4,08 s, échéance** | 5,5 ml (1,35 ml/s) | 0,97 → 1,31 bar |
| Infusion | 18,37 s | **11,46 s** | 35,9 ml (3,14 ml/s) | **4,91 bar max** |

- Remplissage fini sur la pression : premier paquet au-dessus de 0,8 bar
  à 14,04 s (0,82 bar), passage à 14,28 s (0,97 bar), 0,24 s d'écart.
  19,7 ml admis (25,2 ml à 11 g) : 4 g de café de plus prennent ≈ 5 ml
  d'espace libre.
- Pré-infusion **finie à l'échéance de 4 s**, sans goutte, la pression
  montant à 1,31 bar à 50 %.
- Première goutte à **28,4 ml** depuis le remplissage, à 19,97 s, 1,6 s
  après le début de l'infusion ; 2 bar à 19,85 s. Marge de remplissage
  8,7 ml.
- Montée : pompe de 50 à 100 % en 2,6 s ; pression à 4,5 bar à 22,0 s,
  puis **plateau à 4,85 bar** de 23,0 s à l'arrêt, 4,91 bar au plus.
  Pompe à 97–100 % : saturée ou presque, les 6 bar ne sont pas atteints.
- Débit au plateau (23,0 à 29,8 s) : 3,29 ml/s comptés en amont,
  **4,40 g/s** en tasse, rapport 1,34.
- Arrivée en tasse, depuis la première goutte (19,97 s) :

  | 1 g | 5 g | 10 g | 20 g | 30 g | 40 g (seuil) | 43 g |
  | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | 0,6 s | 1,8 s | 3,0 s | 5,3 s | 7,5 s | 9,9 s | 10,5 s |

  De 10 à 40,8 g : **4,40 g/s** (4,4, 4,4 et 4,3 g/s par tranche).
- Résistance au plateau : 4,85 bar pour 4,40 g/s, soit **1,10 bar·s/g**
  (0,38 à 11 g, 7,8 pour l'espresso du 25/10 en LM1).
- Arrêt : seuil de `45 − 5,0 = 40,0 g` franchi à 29,82 s (39,3 →
  40,8 g) ; **43,9 g final** (rapport), stable dès ≈ 31 s. 3,9 g arrivent
  après le seuil, ≈ 0,9 s au débit de la fin. Écart à la cible : **−1,1 g**.
- Après l'arrêt, la pression lue monte de 4,80 à 7,22 bar pompe arrêtée,
  balance immobile.
- Ratio final **1:2,9**. Pompe en marche 21,9 s, dont 12,0 s avant la
  première goutte ; **9,9 s de la goutte à l'arrêt**. 62,6 ml admis.

**Thermique.**

| Repère | Mesure |
| --- | ---: |
| NTC au départ | 88,00 °C |
| Pic avant l'infusion | 90,73 °C à 15,6 s, pendant la pré-infusion, +2,73 °C |
| NTC au début de l'infusion | 89,59 °C, +1,59 °C |
| Commande à 0 % (remboursement) | 9,3 s, de 18,57 à 27,89 s |
| Appoint repris avant l'arrêt | 1,93 s |
| NTC à l'arrêt | 86,93 °C, minimum pendant l'écoulement |
| **Tasse** | **89,37 °C, +1,37 °C** |
| Minimum | **82,73 °C**, 6,0 s après l'arrêt ; 83,23 °C en fin de capture (10 s) |

- Énergie, commande / SSR : précharge 8,49 / 8,49 kJ, remplissage 6,13 /
  5,92 kJ, pré-infusion 2,42 / 2,04 kJ, infusion 2,02 / 1,90 kJ.
- Moyenne en volume amont : 89,44 °C.
- Aucune lecture NTC invalide.
- Modèle : 88,96 °C avec la commande réelle, **+0,49 °C** (mesure du
  simulateur : 89,45 °C) ; +0,85 °C à 13 h 32 avec 28 min sur le groupe,
  −2,07 °C à 9 h 34 avec 7 min.

**Attendus vérifiés.**

| Attendu (13 h 32 et échanges suivants) | Mesure |
| --- | --- |
| À mouture espresso, 15 g résistent ≈ 0,5 bar·s/g (proportion depuis 11 g) ou ≈ 3,7 bar·s/g (géométrie depuis l'espresso) | **1,10 bar·s/g** : entre les deux, aucune des deux estimations n'est juste |
| Si la géométrie est juste, 6 bar et ≈ 1,6 g/s ; sinon 2 à 3 bar à ≈ 5 g/s | **4,85 bar, 4,40 g/s** : la pompe reste presque saturée, sous 6 bar |
| Poids final ≈ cible avec `lead_weight_g` ≈ 5 g à ≈ 5 g/s | 43,9 g pour 45 g : à 4,4 g/s, l'avance de 5 g est trop grande d'≈ 1 g |
| Remplissage plus court avec moins d'espace libre | 19,7 ml contre 25,2 ml, atteint |

**Lecture.**

- *Résistance, mesuré.* Passer de 11 à 15 g multiplie la résistance par
  **2,9** (0,38 → 1,10 bar·s/g) pour une dose multipliée par 1,36. Un lit
  plus épais ne l'explique pas seul. Causes possibles, non séparées : la
  galette mince de 11 g se dégradait (estimé) ; une galette se compacte
  davantage à 4,9 bar qu'à 2 bar, ce qui augmente encore sa résistance
  (estimé). À 1,10 bar·s/g, la galette est dans la plage du turbo du
  papier (≈ 1 à 2 bar·s/g, estimé depuis 15 g → 40 g à 6 bar en 7 à 15 s).
- *Écart avec le LM1, estimé.* 1,10 contre 7,8 bar·s/g, un facteur 7 là
  où la géométrie seule donnait ≈ 2 (`(15 / 8) × (41 / 58)⁴ = 0,47`).
  L'espresso du LM1 est extrait à 9 bar, plus compacté ; la forme et le
  perçage des deux paniers diffèrent. Rien ne sépare ces causes.
- *Temps, mesuré.* 9,9 s de la goutte à l'arrêt, dans les 7 à 15 s du
  papier et sous les 11 à 14 s de Hedrick. Ce qui allonge le shot est le
  **mouillage** : 12,0 s de pompe avant la goutte (remplissage 6,3 s,
  pré-infusion jusqu'à l'échéance de 4 s, 1,6 s d'infusion), que le papier
  n'a pas. Depuis le démarrage de la pompe, 21,9 s.
- *Débit, mesuré.* 4,40 g/s, juste sous les 4,5 à 5 g/s de Cameron ; la
  pompe à 100 % ne donne que 4,4 g/s contre 4,9 sous 2 bar. Premier point
  mesuré du débit de la pompe sous charge : **≈ 4,4 g/s à 4,85 bar**.
- *Goût, estimé.* Crema et texture vont avec la pression (4,9 bar contre
  2 bar sans crema à 13 h 32) ; le goût d'espresso et la légère amertume,
  avec un ratio de 1:2,9 et une mouture espresso, qui extrait plus vite
  qu'une mouture de turbo. Sans TDS, rien ne dit l'extraction.
- *Température, mesuré.* +1,37 °C en tasse, l'écart du modèle à +0,49 °C,
  avec 20 min sur le groupe : même sens qu'à 13 h 32.

### 26 octobre, 16 h 32 — `261026-163228`

**Contexte.** Quatrième **turbo shot** : par rapport à 15 h 35, **mouture
nettement plus grosse**, seul changement de recette ; 15,0 g dans le
panier 58 mm 15 g (le profil enregistre désormais `dose_g` à 15,0 g),
cible 45 g, `lead_weight_g` de 5,0 g, 88 °C, 6 bar, pompe jusqu'à 100 %,
remplissage et pré-infusion inchangés. Même café, même tassage, sans puck
screen. Écran et carte sensors 0.4.24. Observations de David : **du
channelling** ; au goût, le fruité ne ressort pas vraiment, **surtout plus
plat** ; le channelling apporte probablement de l'amertume. Après
l'essai de 15 h 35, David avait vu l'empreinte de la douchette sur la
galette.

La mouture plus grosse raccourcit aussi le mouillage (pré-infusion finie
sur la goutte) : rien ne sépare les deux effets.

**Routine** : guide ouvert sous la consigne (NTC à 86,5 °C) ; rinçage de
5,03 s, 18,6 ml, 25,6 g, pression maximale 0,25 bar ; consigne retrouvée
à la fin du rinçage, minimum 83,4 °C ensuite, résistance active 39,1 s ;
porte-filtre **1002,8 s sur le groupe** (17 min) ; préparation de 91,3 s.
Balance présente toute la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | 0,47 → 0,54 bar |
| Remplissage | 8,01 s | **6,28 s, pression** | 18,3 ml (2,91 ml/s) | 0,54 → 0,87 bar |
| Pré-infusion | 14,29 s | **2,43 s, goutte** | 3,2 ml (1,31 ml/s) | 0,87 → 0,95 bar |
| Infusion | 16,71 s | **10,88 s** | 35,7 ml (3,28 ml/s) | **2,53 bar max** |

- Remplissage fini sur la pression : premier paquet au-dessus de 0,8 bar
  à 14,07 s, passage à 14,29 s, 0,22 s d'écart. 18,3 ml admis (19,7 ml à
  15 h 35).
- Première goutte à **21,4 ml** depuis le remplissage, à 16,97 s ; marge
  de remplissage 3,1 ml. **Pompe avant la goutte : 9,0 s** (12,0 s à
  15 h 35).
- Montée : pompe de 50 à 100 % en 2,6 s ; 2 bar à 19,03 s ; plateau de
  2,52 bar à 21,0 s, qui **redescend à 2,33 bar** à l'arrêt, pompe à
  100 %.
- Débit au plateau (20,5 à 27,6 s) : 3,53 ml/s comptés en amont,
  4,72 g/s en tasse ; résistance moyenne **0,52 bar·s/g** (1,10 à
  15 h 35).
- **Résistance par tranche de 10 g** :

  | Tranche | 15 h 35 (mouture espresso) | 16 h 32 (mouture grosse) |
  | --- | ---: | ---: |
  | 10 → 20 g | 4,40 g/s, 4,86 bar, 1,10 bar·s/g | 4,30 g/s, 2,49 bar, 0,58 bar·s/g |
  | 20 → 30 g | 4,43 g/s, 4,88 bar, 1,10 bar·s/g | 4,65 g/s, 2,43 bar, 0,52 bar·s/g |
  | 30 → 40 g | 4,37 g/s, 4,82 bar, 1,10 bar·s/g | 4,86 g/s, 2,37 bar, 0,49 bar·s/g |

  À commande constante, le débit monte et la pression baisse : la
  résistance **perd 16 %** pendant l'écoulement, alors qu'elle était
  constante à 15 h 35. C'est la trace dans la capture du channelling vu
  par David.
- Arrivée en tasse, depuis la première goutte :

  | 1 g | 5 g | 10 g | 20 g | 30 g | 40 g (seuil) | 43 g |
  | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | 1,5 s | 2,9 s | 4,1 s | 6,4 s | 8,6 s | 10,6 s | 11,2 s |

- Arrêt : seuil de 40,0 g franchi à 27,6 s ; **44,0 g final** (rapport),
  4,0 g après le seuil, ≈ 0,85 s au débit de la fin. Écart à la cible :
  **−1,0 g**. Ratio **1:2,9**.
- Pompe en marche 19,6 s, dont 9,0 s avant la goutte ; **10,6 s de la
  goutte à l'arrêt**. 58,6 ml admis.

**Thermique.**

| Repère | Mesure |
| --- | ---: |
| NTC au départ | 87,99 °C |
| Pic avant l'infusion | 91,07 °C à 16,4 s, pendant la pré-infusion, +3,07 °C |
| NTC au début de l'infusion | 90,97 °C, +2,97 °C |
| Commande à 0 % (remboursement) | 9,1 s, de 16,97 à 26,10 s |
| Appoint repris avant l'arrêt | 1,50 s |
| NTC à l'arrêt | **84,05 °C**, minimum pendant l'écoulement : **−6,9 °C** pendant l'écoulement (−2,7 °C à 15 h 35) |
| **Tasse** | **88,77 °C, +0,77 °C** |
| Minimum | **80,28 °C**, 4,8 s après l'arrêt ; 81,84 °C en fin de capture (10 s) |

- Énergie, commande / SSR : précharge 8,29 / 8,50 kJ, remplissage 6,00 /
  5,82 kJ, pré-infusion 1,68 / 1,86 kJ, infusion 1,73 / 1,42 kJ.
- Moyenne en volume amont : 89,28 °C.
- Aucune lecture NTC invalide.
- Modèle : 88,76 °C en tasse avec la commande réelle, **+0,09 °C** ; mais
  il donne un minimum de 86,16 °C pour 80,28 °C mesurés. La chute de la
  NTC à partir de 23 s (90,6 → 84,0 °C en 4,6 s), pour un volume admis
  semblable à celui de 15 h 35, n'est pas expliquée.

**Attendus vérifiés.**

| Attendu (15 h 35) | Mesure |
| --- | --- |
| Résistance ≈ 0,3 à 0,6 bar·s/g | 0,52 bar·s/g en moyenne, 0,58 → 0,49, atteint |
| Pression au plateau ≈ 1,5 à 3 bar | 2,33 à 2,52 bar, atteint |
| Débit ≈ 4,8 à 5 g/s | 4,72 g/s en moyenne, 4,30 → 4,86 g/s |
| De la goutte à l'arrêt ≈ 9 s | **10,6 s** : la première tranche est plus lente (1,5 s pour le premier gramme) |
| Pré-infusion probablement finie sur la goutte | 2,43 s, sur la goutte, atteint |
| Poids final ≈ 44,4 g avec `lead_weight_g` de 5,0 g | 44,0 g, atteint |

**Lecture.**

- *Galette, mesuré.* Avec la mouture grosse, la galette s'érode : sa
  résistance baisse de 16 % en 7 s à commande constante. À mouture
  espresso, à 4,9 bar, elle restait constante. L'érosion n'est donc pas
  liée à la pression, plus basse ici, mais à la mouture.
- *Mouture, mesuré.* Grossir divise la résistance par 2,1 (1,10 →
  0,52 bar·s/g) et ne raccourcit pas l'écoulement (10,6 s contre 9,9 s) :
  la pompe borne le débit, comme prévu. Le mouillage, lui, passe de 12,0 à
  9,0 s.
- *Goût, estimé.* « Plat » et sans fruité vont avec une extraction
  inégale : l'eau passe par les chemins ouverts, une partie du café est
  peu extraite, une autre trop, ce qui peut aussi donner l'amertume. Sans
  TDS, rien ne le mesure.
- *Température, mesuré.* La NTC chute de 6,9 °C pendant l'écoulement, la
  plus forte chute des quatre turbo shots, et atteint 80,3 °C après
  l'arrêt ; la tasse reste à 88,77 °C parce que la plus grande partie du
  café arrive avant la chute. La cause n'est pas identifiée.

**Suite.** Pistes proposées par David :

| Piste | Effet attendu |
| --- | --- |
| Puissance de pompe plus basse (`brew.pump_pct`) | à cette résistance, la puissance fixe presque le débit (sans porte-filtre : 4,6 g/s à 70 %, 3,5 g/s à 60 %). À 60 %, ≈ 3,3 g/s : 45 g en ≈ 13 s depuis la goutte, dans les 11 à 14 s de Hedrick, et un débit plus lent à travers la galette (estimé) |
| Puck screen | répartit l'eau de la douchette sur la galette. Il ajoute ≈ 1,5 à 2 mm : avec l'empreinte de la douchette vue à 15 h 35, il demande probablement de descendre à 14 g, ce qui change aussi la résistance |

Mesure qui juge l'une ou l'autre : la résistance par tranche de 10 g.
Constante, la galette tient ; en baisse, elle s'érode encore.

**Préférence de David** : sur les quatre turbo shots du 26 octobre, le
meilleur au goût est celui de **15 h 35** (15 g, mouture espresso,
plateau à 4,85 bar).

**Décision de David** : prochain essai avec **`brew.pump_pct` à 70 %**,
le reste inchangé (15 g, même mouture grosse, 45 g, `lead_weight_g` de
5,0 g, 88 °C) ; pas de puck screen, pour ne changer qu'un réglage.

**Attendus pour l'essai à 70 %** (estimés).

| Point | Attendu |
| --- | --- |
| Débit en tasse | ≈ 4,2 g/s (4,64 g/s en débit libre à 70 %, moins la perte sous ≈ 2 bar) |
| Pression au plateau | ≈ 2 bar à 0,5 bar·s/g |
| De la goutte à l'arrêt | ≈ 11 s |
| Poids final | 40 + 0,9 × 4,2 ≈ 43,8 g |
| Érosion | si un débit plus lent ménage la galette, résistance par tranche de 10 g constante à ± 5 % ; sinon, baisse comparable aux 16 % de 16 h 32 |

**Piste : régulation par débit.** Non retenue pour l'instant ; l'essai à
70 % dit d'abord si un débit plus lent ménage la galette. À 100 % fixe, le
débit est passé de 4,30 à 4,86 g/s (+13 %) pendant que la galette
s'érodait : une boucle de débit supprimerait cet écart en baissant la
pompe quand la résistance chute ; elle ne guérit pas le channelling.
Ce qu'elle demanderait, tout dans l'écran, sans toucher à la carte
sensors ni au protocole :

- passer le débit du débitmètre, déjà calculé sur 10 impulsions (4,2 ml),
  à la machine d'état de l'infusion, qui ne reçoit que poids et pression ;
- un PI de débit avec la pression en plafond (la plus petite des deux
  commandes), précommandé par la courbe de débit libre (3,5 g/s ≈ 60 %) ;
  pompe sans débit sous 40 %, saturée au-delà de 80 %, ≈ 0,15 g/s par
  point vers 60 % ;
- un schéma de configuration 12 : mode `pressure` ou `flow`, consigne de
  débit ;
- trois choix à trancher : consigne en g/s en tasse convertie (le
  débitmètre compte 1,34 à 1,42 fois moins que la balance) ou en ml/s
  comptés ; régulation dès la fin de la pré-infusion ou après la première
  goutte ; avance d'arrêt exprimée en secondes × débit mesuré (≈ 0,9 s)
  au lieu de grammes fixes.

### 27 octobre, 7 h 44 — `261027-074432`

**Contexte.** Cinquième **turbo shot**, `configs/261027-074444.json` :
par rapport à 16 h 32 la veille, **`brew.pump_pct` à 70 %** (100 %
avant) et **puck screen** posé sur la galette. Le reste est inchangé :
15,0 g dans le panier 58 mm 15 g, cible 45 g, `lead_weight_g` de 5,0 g,
88 °C, 6 bar, remplissage et pré-infusion. Même café. La mouture est la
mouture grosse de 16 h 32, d'après la décision de la veille ; la
résistance de la galette va dans ce sens (0,51 bar·s/g, contre 0,52 à
16 h 32 et 1,10 à mouture espresso). Écran et carte sensors 0.4.24.
Observations de David : **la galette semble avoir mieux tenu
l'infusion** ; au goût, **moins fort qu'un espresso**, mais **une
acidité agréable, car légère**, commence à ressortir ; le café lui
semble **trop chaud**.

La puissance de pompe et le puck screen changent ensemble : l'érosion
(résistance par tranche de 10 g) et le débit mêlent les deux. Rien dans
la capture ne les sépare. Seul indice : la résistance moyenne est la
même qu'à 16 h 32, donc le puck screen n'ajoute pas de résistance
mesurable.

**Routine** : guide ouvert sous la consigne (NTC à 86,2 °C) ; rinçage de
5,05 s, 18,6 ml, 23,2 g, pression maximale 0,14 bar ; consigne retrouvée
66,7 s après la fin du rinçage, minimum 81,6 °C, résistance active
46,8 s ; porte-filtre **921,3 s sur le groupe** (15 min) ; préparation de
105,7 s. Balance présente toute la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | 0,30 → 0,35 bar |
| Remplissage | 8,01 s | **7,98 s, pression** | **24,3 ml** (3,05 ml/s) | 0,35 → 0,87 bar |
| Pré-infusion | 15,99 s | **0,50 s, goutte** | 1,2 ml | 0,87 → 0,88 bar |
| Infusion | 16,49 s | **12,91 s** | 34,5 ml (2,67 ml/s) | **1,92 bar max** |

- Remplissage fini sur la pression : premier paquet au-dessus de 0,8 bar
  à 15,81 s (0,85 bar), passage à 15,99 s, 0,18 s d'écart. **24,3 ml**
  admis en 7,98 s, contre 18,3 ml en 6,28 s à 16 h 32. Le puck screen
  devrait réduire l'espace libre de quelques ml ; il en faut 6 de plus
  pour atteindre 0,8 bar. Cause non identifiée.
- Pré-infusion **finie sur la goutte** après 0,50 s (firmware à +0,1 g).
- Première goutte à **26,1 ml** depuis le remplissage, à 16,86 s ;
  marge de remplissage 1,8 ml. **Pompe avant la goutte : 8,9 s** (9,0 s à
  16 h 32, 12,0 s à 15 h 35).
- Montée : pompe de 50 à 70 % en 2,6 s ; 1,5 bar à 19,76 s ; **plateau
  de 1,85 à 1,90 bar** de 22,0 s à l'arrêt. 2 bar ne sont jamais
  atteints pendant l'infusion.
- Débit au plateau (21,0 s à l'arrêt) : 2,83 ml/s comptés en amont,
  **3,78 g/s** en tasse, rapport 1,34.
- **Résistance par tranche de 10 g** :

  | Tranche | 16 h 32 (100 %, sans screen) | 7 h 44 (70 %, puck screen) |
  | --- | ---: | ---: |
  | 10 → 20 g | 4,30 g/s, 2,49 bar, 0,58 bar·s/g | 3,61 g/s, 1,85 bar, 0,51 bar·s/g |
  | 20 → 30 g | 4,65 g/s, 2,43 bar, 0,52 bar·s/g | 3,80 g/s, 1,89 bar, 0,50 bar·s/g |
  | 30 → 40 g | 4,86 g/s, 2,37 bar, 0,49 bar·s/g | 3,61 g/s, 1,89 bar, 0,52 bar·s/g |

  La résistance reste **constante à ± 2 %** (−16 % à 16 h 32) : la
  galette ne s'érode plus. C'est ce que David a vu.
- Arrivée en tasse, depuis la première goutte :

  | 1 g | 5 g | 10 g | 20 g | 30 g | 40 g (seuil) |
  | ---: | ---: | ---: | ---: | ---: | ---: |
  | 0,9 s | 2,9 s | 4,4 s | 7,1 s | 9,9 s | 12,5 s |

  De 10 g au seuil : **3,68 g/s**.
- Arrêt : seuil de `45 − 5,0 = 40,0 g` franchi à 29,40 s (39,6 →
  40,4 g) ; **43,0 g final** (rapport, 2,6 g de gouttes après l'arrêt).
  3,0 g arrivent après le seuil, ≈ 0,8 s au débit de la fin. Écart à la
  cible : **−2,0 g**.
- Après l'arrêt, la pression lue monte de 1,87 à 2,44 bar pompe arrêtée.
- Ratio final **1:2,9**. Pompe en marche 21,4 s, dont 8,9 s avant la
  goutte ; **12,5 s de la goutte à l'arrêt**. 61,4 ml admis.

**Thermique.**

| Repère | Mesure |
| --- | ---: |
| NTC au départ | 88,05 °C |
| Pic avant l'infusion | 90,00 °C à 12,8 s, pendant le remplissage, +1,95 °C |
| NTC au début de l'infusion | 89,83 °C, +1,83 °C |
| Commande à 0 % (remboursement) | **10,4 s**, de 16,59 à 26,98 s |
| Appoint repris avant l'arrêt | 2,42 s |
| NTC à l'arrêt | 86,10 °C, minimum pendant l'écoulement : −3,7 °C pendant l'écoulement (−6,9 °C à 16 h 32, −2,7 °C à 15 h 35) |
| **Tasse** | **88,20 °C, +0,20 °C** |
| Minimum | **82,80 °C**, 7,9 s après l'arrêt ; 83,06 °C en fin de capture (10 s) |

- Énergie, commande / SSR : précharge 8,50 / 8,73 kJ, remplissage 7,73 /
  7,63 kJ, pré-infusion 0,47 / 0,13 kJ, infusion 2,23 / 2,28 kJ.
- Moyenne en volume amont : 88,80 °C.
- Aucune lecture NTC invalide.
- Modèle : 88,23 °C en tasse avec la commande réelle, **0,00 °C** d'écart
  (mesure du simulateur : 88,23 °C) ; minimum de 86,30 °C pour 82,80 °C
  mesurés. Comme à 16 h 32, le modèle tient la tasse mais pas le minimum
  après l'arrêt (−3,5 °C ici, −5,9 °C à 16 h 32).

**Attendus vérifiés.**

| Attendu (16 h 32, essai à 70 %) | Mesure |
| --- | --- |
| Débit en tasse ≈ 4,2 g/s | **3,78 g/s** au plateau : la pompe à 70 % perd plus que prévu sous 1,9 bar (4,64 g/s en débit libre) |
| Pression au plateau ≈ 2 bar | 1,85 à 1,90 bar, atteint |
| De la goutte à l'arrêt ≈ 11 s | **12,5 s**, plus long, avec le débit plus faible |
| Poids final ≈ 43,8 g | 43,0 g, 0,8 g de moins : le débit après le seuil est plus faible |
| Érosion : résistance constante à ± 5 % si un débit plus lent ménage la galette | **± 2 %**, atteint ; le puck screen, posé en même temps, peut aussi en être la cause |

**Lecture.**

- *Galette, mesuré.* La résistance ne baisse plus pendant l'écoulement
  (0,51 → 0,52 bar·s/g, −16 % à 16 h 32). La pompe à 70 % et le puck
  screen ont été changés ensemble : rien ne sépare leurs effets. Pour les
  séparer, il faudrait retirer le puck screen à 70 %, ou repasser à
  100 % avec le screen ; le critère reste la résistance par tranche.
- *Temps, mesuré.* 12,5 s de la goutte à l'arrêt, dans les 11 à 14 s de
  Hedrick pour cette classe de pompe ; 21,4 s depuis le démarrage de la
  pompe, dont 8,9 s de mouillage.
- *Débit, mesuré.* Deuxième point de la pompe sous charge :
  **≈ 3,8 g/s à 70 % sous 1,9 bar** (4,4 g/s à 100 % sous 4,85 bar,
  4,7 g/s à 100 % sous 2,4 bar).
- *Goût, estimé.* « Moins fort qu'un espresso » va avec une mouture
  grosse à 0,5 bar·s/g, et l'acidité légère avec une extraction plus
  homogène, si la galette ne s'érode plus. Sans TDS, rien ne le mesure.
- *Température, mesuré.* 88,20 °C en tasse, la plus basse des cinq turbo
  shots (88,74 à 89,37 °C), et pourtant jugée trop chaude. La NTC monte
  de +1,83 °C au-dessus de la consigne avant l'infusion : c'est la
  précharge de 8 s, faite pour l'espresso. Puis elle redescend de 3,7 °C
  pendant l'écoulement.

**Leviers de température**, proposés par David après la vidéo de Hedrick
sur une machine à pompe rotative, où il ajoute de l'eau froide dans le
réservoir pendant le shot (87 °C au départ, ≈ 70 °C à la fin). Les
valeurs ci-dessous sont **en simulation**, sur l'hydraulique de cette
capture ; la loi actuelle rembourse la précharge dès le début de
l'infusion.

| Variante | Tasse | NTC à l'arrêt | Minimum |
| --- | ---: | ---: | ---: |
| Commande réelle (88 °C, précharge 8 s) | 88,23 °C | 87,88 °C | 86,30 °C |
| Loi actuelle, précharge 8 s | 88,00 °C | — | 85,87 °C |
| Loi actuelle, précharge 6 s | 86,59 °C | — | 84,67 °C |
| Loi actuelle, précharge 4 s | 85,25 °C | — | 83,66 °C |
| Loi actuelle, précharge 0 s | 83,07 °C | — | 82,30 °C |
| Précharge 8 s, **sans chauffe pendant l'infusion** | 88,21 °C | 87,84 °C | 86,09 °C |
| Précharge 8 s, **sans chauffe dès la pompe** | 85,92 °C | 83,53 °C | 80,55 °C |
| Précharge 4 s, sans chauffe dès la pompe | 83,22 °C | 80,56 °C | 77,59 °C |
| Précharge 0 s, sans chauffe dès la pompe | 80,88 °C | 77,63 °C | 74,53 °C |

- *Consigne plus basse, estimé.* Elle déplace toute la courbe : à 86 °C
  avec 8 s de précharge, ≈ 86,2 °C en tasse si l'écart au critère reste
  celui d'aujourd'hui (+0,20 °C, 15 min sur le groupe). Cet écart a varié
  de −1,63 à +1,37 °C sur les turbo shots, avec la durée sur le groupe :
  compter ± 1 °C. La consigne règle aussi le rinçage et le porte-filtre
  du guide.
- *Précharge plus courte, simulé.* ≈ **0,7 °C de moins en tasse par
  seconde retirée**, de 8 à 4 s. Elle change la pente plus que le
  départ : la NTC part de la consigne et monte moins avant l'infusion.
  Réglable aujourd'hui (`heating.brew_preheat_time_s`, de 0 à 15 s par
  pas de 0,5 s).
- *Sans chauffe pendant l'infusion seule, simulé.* **Sans effet**
  (−0,02 °C) : le remboursement tient déjà l'appoint à 0 % pendant 10,4
  des 12,9 s d'infusion ; seules les 2,4 dernières secondes changent, et
  leur chaleur arrive après l'arrêt.
- *Sans chauffe dès la pompe, simulé.* C'est le levier qui fait une
  température **descendante**, comme chez Hedrick : −2,3 °C en tasse avec
  8 s de précharge, et la NTC tombe de ≈ 5 °C pendant le shot au lieu de
  ≈ 1 °C. Le modèle sous-estime la chute réelle (≈ 1 °C rejoué, 3,7 °C
  mesurés) : la descente mesurée sera probablement plus forte. Sans précharge, ≈ 80,9 °C en tasse et ≈ 77,6 °C à l'arrêt. La
  chaudière amortit l'eau froide : la descente à ≈ 70 °C de Hedrick n'est
  pas atteignable avec ≈ 60 ml admis (estimé, le modèle sort de son
  domaine sous 82 °C). Il n'existe pas aujourd'hui : `heating.enabled`
  coupe aussi la régulation au repos.
- *Deux leviers à la fois.* Consigne et précharge agissent sur la même
  moyenne en tasse : le goût ne les sépare pas. La capture les sépare :
  la consigne se lit dans la NTC au départ, la précharge dans le pic
  avant l'infusion et dans la chute pendant l'écoulement, à comparer au
  modèle rejoué.
- *Domaine de l'offset, estimé.* L'offset de −14 °C a été mesuré au
  panier sur des infusions d'espresso. Si la température descend pendant
  le shot, l'eau au panier suit la NTC avec retard ; une mesure SCACE
  dirait de combien.

**Mouture plus grosse pour revenir à 4 à 5 g/s ? Estimé : non.** La
résistance moyenne n'a pas augmenté (0,51 contre 0,52 bar·s/g) ; elle est
devenue **stable**. Le débit a baissé parce que la pompe à 70 % débite
moins : 4,64 g/s au plus en débit libre, 3,78 g/s sous 1,9 bar, soit
≈ 0,45 g/s de perte par bar. Une mouture deux fois plus grosse
(≈ 0,25 bar·s/g) ne donnerait que ≈ 4,2 g/s sous ≈ 1 bar, avec le risque
d'érosion vu à 16 h 32. Le levier du débit est la puissance : à 100 % et
0,5 bar·s/g, ≈ 4,7 g/s (16 h 32). Repasser à 100 % avec le puck screen
dirait aussi lequel des deux a stabilisé la galette : si la résistance
reste constante, c'est le puck screen.

**Décision de David** : prochain essai avec **`brew.pump_pct` à 100 %**
et **`filling.pump_pct` à 100 %** (70 % avant), pour écourter le
remplissage ; le reste inchangé (même mouture grosse, puck screen, 15 g,
45 g, `lead_weight_g` de 5,0 g, 88 °C, précharge de 8 s). L'infusion est
celle de 16 h 32 la veille avec le puck screen en plus : elle sépare
l'effet du puck screen de celui de la pompe à 70 %. Le remplissage plus
rapide agit avant la goutte et se lit dans le mouillage ; s'il change la
tenue de la galette pendant l'écoulement, rien ne le sépare du puck
screen. La mouture espresso n'est pas reprise
en même temps, pour garder cette séparation ; elle pourra être essayée
ensuite avec le puck screen. L'essai de température (86 °C, 2 s de
précharge) est reporté : l'eau plus froide, plus visqueuse
(≈ 1,7 % par kelvin), ferait monter la résistance pendant la descente et
masquerait en partie une érosion.

**Attendus pour l'essai à 100 % (infusion et remplissage) avec puck screen** (estimés depuis
16 h 32 la veille).

| Point | Attendu |
| --- | --- |
| Résistance par tranche de 10 g | constante à ± 5 % si le puck screen tient la galette ; baisse proche des 16 % de 16 h 32 si c'est le débit qui l'érodait |
| Pression au plateau | ≈ 2,4 à 2,5 bar |
| Débit en tasse | ≈ 4,7 g/s (4,30 → 4,86 g/s à 16 h 32) |
| De la goutte à l'arrêt | ≈ 10,5 s |
| Remplissage | ≈ 4 ml/s comptés (5,68 g/s en débit libre, débitmètre à ≈ 1,4) contre 3,05 ml/s ; ≈ 6 s pour ≈ 24 ml, fini sur la pression ; dépassement du seuil de 0,8 bar un peu plus grand à débit plus fort |
| Mouillage avant la goutte | ≈ 7 s (8,9 s aujourd'hui) ; la pré-infusion à 50 % reste, finie sur la goutte |
| Poids final | ≈ 44 g (40 + 0,85 s × 4,7 g/s) |
| NTC tasse | ≈ 88,8 °C (88,77 °C à 16 h 32, 88,20 °C aujourd'hui) |

**Attendus de l'essai de température, reporté** (86 °C, précharge de
2 s ; estimés sur l'hydraulique d'aujourd'hui).

| Point | Attendu |
| --- | --- |
| NTC au départ | 86,0 °C |
| Pic avant l'infusion | ≈ 86,2 °C (+1,95 °C aujourd'hui avec 8 s ; 2 s n'en laissent que ≈ 0,1 en simulation) |
| Remboursement | ≈ 2,6 s à 0 % au début de l'infusion (180 %·s à ≈ 70 %), puis appoint repris pendant ≈ 10 s |
| Tasse | ≈ **82,3 °C** : 88,20 − 2,0 (consigne) − 3,95 (précharge, simulé) ; ± 1 °C avec le temps sur le groupe, et le modèle sort de son domaine sous ≈ 82 °C |
| Minimum | ≈ 80 °C, probablement plus bas : le modèle sous-estime la chute des turbo shots |
| Débit, pression, temps | 3,8 g/s, 1,9 bar, ≈ 12,5 s de la goutte à l'arrêt |
| Poids final | ≈ 43 g |
| Résistance par tranche | constante à ± 5 % |

### 27 octobre, 13 h 16 — `261027-131653`

**Contexte.** Sixième **turbo shot**. Par rapport à 7 h 44 :
**`brew.pump_pct` à 100 %** et **`filling.pump_pct` à 100 %** (70 %
avant). Le profil de la capture enregistre aussi **88 → 86 °C** et une
**précharge de 8 → 2 s** : l'essai de température reporté a été fait en
même temps. Aucune configuration sauvegardée depuis
`configs/261027-074444.json` ; ces valeurs sont lues dans la capture. Le
reste est inchangé : 15,0 g de mouture grosse, puck screen, cible 45 g,
`lead_weight_g` de 5,0 g, 6 bar, pré-infusion à 50 %. Même café. Écran
et carte sensors 0.4.24. Observations de David : **un peu amer** ; **on
sent un peu plus les saveurs**, mais il pense que c'est parce que le café
est **plus plat** ; pas convaincu. Il a **l'impression d'un channelling
massif**, difficile à juger, parce que le porte-filtre low-touch ne
donne pas un filet unique.

Trois réglages changent ensemble. La capture les sépare en partie : la
température se lit dans la NTC, le remplissage dans le mouillage, la
pompe dans le débit. Au goût, rien ne les sépare.

**Routine** : guide ouvert machine froide (NTC à 63,0 °C) ; rinçage de
5,04 s, 18,6 ml, 30,3 g, pression maximale 0,70 bar, sous la consigne
(81,3 → 80,4 °C) ; consigne retrouvée **565,9 s** après la fin du
rinçage, minimum 73,4 °C, résistance active 70,5 s ; porte-filtre
**331,2 s sur le groupe** (5,5 min, 921 s à 7 h 44) ; préparation de
108,7 s. Balance perdue 38,7 s après le départ, après la capture.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | **2,09 s** | — | 0,51 bar |
| Remplissage | 2,09 s | **5,10 s, pression** | **19,7 ml** (3,87 ml/s) | 0,51 → **0,94 bar** |
| Pré-infusion | — | **0 s** | — | — |
| Infusion | 7,19 s | **10,54 s** | 35,9 ml (3,41 ml/s) | **2,54 bar max** |

- Remplissage fini sur la pression : premier paquet au-dessus de 0,8 bar
  à 6,92 s (0,82 bar), passage à 7,19 s, 0,28 s d'écart, **0,94 bar** au
  passage (0,87 à 0,88 bar à 70 %). **19,7 ml** admis, contre 24,3 ml à
  7 h 44 et 18,3 ml à 16 h 32 sans puck screen : les 6 ml de plus de
  7 h 44 ne se reproduisent pas.
- **Pré-infusion sautée** : la capture passe du remplissage à
  l'infusion, pompe d'abord à 50 %. La balance passe de −0,2 à 0,0 g à
  7,04 s, pendant le remplissage : le seuil de +0,1 g du firmware est
  franchi avant la vraie goutte. Que ce soit une première goutte ou
  l'à-coup du remplissage à 100 % sur la balance, la capture ne le dit
  pas.
- Première goutte (≥ 0,3 g) à **21,4 ml** depuis le remplissage, à
  7,93 s ; marge de remplissage 1,7 ml. **Pompe avant la goutte : 5,8 s**
  (8,9 s à 7 h 44, 9,0 s à 16 h 32).
- Montée : pompe de 50 à 100 % en 2,5 s ; 2 bar à 10,38 s ; pression
  **montante** pendant tout l'écoulement, de 2,34 à 2,53 bar (de 2,52 à
  2,33 bar, descendante, à 16 h 32).
- Débit au plateau (11,0 s à l'arrêt) : 3,61 ml/s comptés en amont,
  **4,60 g/s** en tasse, rapport 1,27.
- **Résistance par tranche de 10 g**, recalculée avec le même script pour
  les trois captures ; « corrigée » ramène la viscosité de l'eau à 88 °C
  avec la NTC moyenne de la tranche (estimé : l'eau au panier suit la NTC
  avec retard) :

  | Tranche | 16 h 32 (100 %, sans screen) | 7 h 44 (70 %, screen) | 13 h 16 (100 %, screen, 86 °C) | NTC | Corrigée |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | 10 → 20 g | 0,58 bar·s/g | 0,51 bar·s/g | 4,82 g/s, 2,34 bar, **0,49 bar·s/g** | 83,3 °C | 0,46 |
  | 20 → 30 g | 0,52 | 0,50 | 4,72 g/s, 2,45 bar, **0,52 bar·s/g** | 81,7 °C | 0,48 |
  | 30 → 40 g | 0,49 | 0,51 | 4,57 g/s, 2,52 bar, **0,55 bar·s/g** | 80,2 °C | 0,50 |

  La résistance **monte de 12 %** (+9 % corrigée), à l'inverse de
  l'érosion de 16 h 32 (−16 %). La moyenne corrigée, 0,48 bar·s/g, est
  6 % sous celle de 7 h 44.
- Arrivée en tasse, depuis la première goutte :

  | 1 g | 5 g | 10 g | 20 g | 30 g | 40 g (seuil) |
  | ---: | ---: | ---: | ---: | ---: | ---: |
  | 0,5 s | 2,1 s | 3,4 s | 5,6 s | 7,5 s | 9,8 s |

  De 10 g au seuil : **4,69 g/s**.
- Arrêt : seuil de 40,0 g franchi à 17,56 s ; **43,7 g final** (rapport).
  4,1 g arrivent après le seuil, ≈ 0,9 s au débit de la fin. Écart à la
  cible : **−1,3 g**.
- Après l'arrêt, la pression lue monte de 2,53 à 5,33 bar pompe arrêtée,
  toujours en hausse en fin de capture (+2,4 bar à 15 h 35, +0,6 bar à
  16 h 32).
- Ratio final **1:2,9**. Pompe en marche 15,5 s ; **9,6 s de la goutte à
  l'arrêt**. 56,8 ml admis.

**Thermique.**

| Repère | Mesure |
| --- | ---: |
| NTC au départ | 86,17 °C |
| Pic avant l'infusion | 86,19 °C, +0,02 °C (+1,95 °C à 7 h 44 avec 8 s) |
| NTC au début de l'infusion | 85,99 °C |
| Commande à 0 % (remboursement) | **2,58 s**, de 7,30 à 9,88 s |
| Appoint repris avant l'arrêt | 7,68 s |
| NTC à l'arrêt | 79,73 °C ; **−6,3 °C pendant l'écoulement** (−3,7 °C à 7 h 44) |
| **Tasse** | **82,52 °C, −3,48 °C** |
| Minimum | **78,91 °C**, 3,3 s après l'arrêt ; 81,70 °C en fin de capture (10 s) |

- Énergie, commande / SSR : précharge 2,11 / 2,17 kJ, remplissage 5,50 /
  5,91 kJ, infusion 8,46 / 8,69 kJ.
- Moyenne en volume amont : 84,10 °C.
- Aucune lecture NTC invalide.
- Modèle : 82,29 °C en tasse avec la commande réelle, **−0,23 °C**
  d'écart ; minimum de 79,36 °C pour 78,91 °C mesurés (−3,5 °C d'écart à
  7 h 44). Avec une précharge courte, le modèle tient aussi le minimum.
- La tasse est **5,7 °C sous celle de 7 h 44**, pour 2 °C de consigne en
  moins : l'écart au critère passe de +0,20 à −3,48 °C. En simulation sur
  cette capture, 8 s de précharge donnent 85,43 °C en tasse contre
  82,71 °C avec 3 s, ≈ 0,55 °C par seconde : la précharge fait le reste.

**Attendus vérifiés.**

| Attendu (essai à 100 % avec puck screen) | Mesure |
| --- | --- |
| Résistance constante à ± 5 % si le puck screen tient la galette ; baisse proche de 16 % si le débit l'érodait | **+12 %** (+9 % corrigée de la viscosité) : pas de baisse, donc pas d'érosion mesurable ; la montée dépasse ce que la viscosité explique |
| Pression au plateau ≈ 2,4 à 2,5 bar | 2,34 → 2,53 bar, atteint |
| Débit en tasse ≈ 4,7 g/s | 4,82 → 4,57 g/s, 4,60 g/s au plateau, atteint |
| De la goutte à l'arrêt ≈ 10,5 s | **9,6 s**, plus court |
| Remplissage ≈ 4 ml/s, ≈ 6 s, ≈ 24 ml, dépassement un peu plus grand | 3,87 ml/s, **5,10 s, 19,7 ml** ; 0,94 bar au passage, dépassement plus grand, atteint |
| Mouillage ≈ 7 s, pré-infusion finie sur la goutte | **5,8 s**, pré-infusion **sautée** (seuil de poids franchi pendant le remplissage) |
| Poids final ≈ 44 g | 43,7 g, atteint |
| NTC tasse ≈ 88,8 °C | **82,52 °C** : consigne et précharge changées en même temps |

| Attendu (essai de température, 86 °C et 2 s) | Mesure |
| --- | --- |
| NTC au départ 86,0 °C | 86,17 °C |
| Pic ≈ 86,2 °C | 86,19 °C, atteint |
| Remboursement ≈ 2,6 s à 0 % | 2,58 s, atteint |
| Tasse ≈ 82,3 °C | **82,52 °C**, atteint |
| Minimum ≈ 80 °C, probablement plus bas | 78,91 °C, atteint |

**Lecture.**

- *Channelling, mesuré et estimé.* Un canal qui s'ouvre pendant le shot
  fait baisser la résistance, comme à 16 h 32 (−16 %, channelling vu par
  David). Ici elle **monte** de 9 % une fois la viscosité corrigée :
  **aucun canal ne se creuse pendant l'écoulement**. La capture ne
  dit rien d'un canal ouvert dès le début : il baisserait la résistance
  moyenne, qui n'est que 6 % sous celle de 7 h 44. Le plancher du
  circuit (douchette, puck screen, panier sans café) n'est pas mesuré :
  s'il fait une bonne part des ≈ 0,5 bar·s/g, la galette pèse peu dans la
  mesure, et même un gros canal y changerait peu de chose. L'état de la
  galette après le shot (trous, zones sèches, empreinte du puck screen)
  reste le meilleur indice.
- *Mouillage, mesuré.* Le remplissage à 100 % a raccourci le mouillage
  de 3,1 s, et la pré-infusion n'a pas eu lieu : la pompe est passée de
  la fin du remplissage à la montée sans attendre la goutte. Le seuil de
  +0,1 g est fait pour la vraie goutte : à 100 %, il est franchi 0,9 s
  avant. Un mouillage écourté est une cause classique de channelling dès
  le départ (hypothèse non testée).
- *Résistance montante, estimé.* La viscosité explique ≈ 3 des
  12 points. Le reste peut venir de fines qui migrent ou d'une galette
  qui se tasse sous un débit plus fort ; rien dans la capture ne
  tranche.
- *Goût, estimé.* La tasse perd 5,7 °C, le contact de la goutte à
  l'arrêt 2,9 s, le mouillage 3,1 s. Les trois vont vers une extraction
  plus faible, ce qui va avec « plus plat ». L'amertume ne vient pas
  d'ordinaire d'une eau plus froide. Elle peut venir d'un canal
  (sur-extraction locale), ce qui irait avec l'impression de David ;
  hypothèse non testée. Sans TDS, rien ne le mesure.
- *Température, mesuré.* 82,5 °C en tasse avec une NTC descendante de
  86,0 à 79,7 °C : c'est la courbe « à la Hedrick » sans couper la
  chauffe. Elle est passée avec la pompe et le remplissage : l'effet de
  la température sur le goût n'est pas séparé.

**Décision de David** : revenir à la **mouture espresso** de 15 h 35 la
veille, pour remonter la pression et retrouver le shot préféré. Seule la
mouture change : pompe à 100 % au remplissage et à l'infusion, puck
screen, 15 g, cible 45 g, `lead_weight_g` de 5,0 g, 86 °C, précharge de
2 s, pré-infusion à 50 % (fin au temps ou à la goutte). Par rapport à
15 h 35, l'essai garde donc trois différences : le puck screen, le
remplissage à 100 % et la température (≈ 82,5 °C en tasse contre
89,37 °C). Ce ne sera pas une reprise exacte de 15 h 35.

**Attendus pour l'essai à mouture espresso** (estimés depuis 15 h 35 la
veille et 13 h 16).

| Point | Attendu |
| --- | --- |
| Résistance | ≈ 1,1 bar·s/g, si le puck screen n'en ajoute pas (rien de mesurable à 7 h 44) ; par tranche, **montante de ≈ 5 à 10 %** avec l'eau qui refroidit, constante une fois corrigée de la viscosité ; une baisse dirait un canal |
| Pression au plateau | ≈ **4,8 bar** (4,85 bar à 15 h 35), sous les 6 bar : pas de régulation |
| Débit en tasse | ≈ 4,4 g/s |
| De la goutte à l'arrêt | ≈ 10 s (9,9 s à 15 h 35) |
| Remplissage | ≈ 20 ml (19,7 ml à 15 h 35) en ≈ 5 s, fini sur la pression vers 0,95 bar |
| Pré-infusion | finie à l'échéance de 4 s, comme à 15 h 35 (la goutte arrivait 1,6 s après) ; si le seuil de +0,1 g est franchi pendant le remplissage, comme à 13 h 16, elle saute |
| Pompe avant la goutte | ≈ 10,5 s si la pré-infusion va à l'échéance, ≈ 7 s si elle saute (12,0 s à 15 h 35, remplissage à 70 %) |
| Démarrage de la pompe → arrêt | ≈ 20 s, au-dessus des 12 à 18 s du turbo courant et à la limite des 20 s de Cameron |
| Poids final | ≈ 44 g |
| NTC tasse | ≈ **82 à 83 °C** : la précharge de 2 s fixe la tasse plus que la mouture ; minimum sous 79 °C avec un écoulement plus long |

### 27 octobre, 13 h 49 — `261027-134905`

**Contexte.** Septième **turbo shot**. Par rapport à 13 h 16, seule la
mouture change : retour à la **mouture espresso** de 15 h 35 la veille.
Le reste est identique : 15,0 g, puck screen, cible 45 g, `lead_weight_g`
de 5,0 g, 6 bar, pompe à 100 % au remplissage et à l'infusion,
pré-infusion à 50 % (4 s ou goutte), 86 °C, précharge de 2 s. Écran et
carte sensors 0.4.24. Observations de David : **moins d'amertume**
qu'à 13 h 16, mais **pas aussi bon que le shot à plus haute
température** (15 h 35 la veille, 89,37 °C en tasse).

**Routine** : guide ouvert à 84,2 °C ; rinçage de 5,05 s, 18,6 ml,
27,7 g, pression maximale 0,22 bar ; consigne retrouvée à la fin du
rinçage, minimum 80,9 °C, résistance active 22,4 s ; porte-filtre
**458,0 s sur le groupe** ; préparation de 87,0 s. Balance présente
toute la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 2,00 s | — | 0,32 bar |
| Remplissage | 2,00 s | **6,10 s, pression** | **23,2 ml** (3,80 ml/s) | 0,32 → 0,95 bar |
| Pré-infusion | 8,10 s | **4,12 s, échéance** | 5,8 ml (1,41 ml/s) | 0,95 → 1,48 bar |
| Infusion | 12,22 s | **11,68 s** | 34,5 ml (2,95 ml/s) | **5,61 bar max** |

- Remplissage fini sur la pression : premier paquet au-dessus de 0,8 bar
  à 7,88 s (0,85 bar), passage à 8,10 s, 0,22 s d'écart. 23,2 ml admis
  (19,7 ml en 6,28 s à 70 % le 26 à 15 h 35).
- Pré-infusion **finie à l'échéance**, sans goutte, comme à 15 h 35 ;
  la balance ne bouge pas pendant le remplissage cette fois.
- Première goutte à **30,4 ml** depuis le remplissage, à 13,10 s, 0,9 s
  après le début de l'infusion ; marge de remplissage 7,2 ml. **Pompe
  avant la goutte : 11,0 s** (12,0 s à 15 h 35).
- Montée : pompe de 50 à 95 % en 2,8 s ; le PI de pression prend la main
  à 4 bar (2/3 de la consigne de 6 bar) et **ramène la pompe à 86 %**,
  puis la laisse remonter lentement jusqu'à 92 % à l'arrêt. La pression
  monte de 4,9 à **5,58 bar** pendant l'écoulement, sans atteindre 6 bar.
  À 15 h 35, la pompe restait à 97–100 % sous 4,85 bar.
- Débit de 10 g à l'arrêt : 3,08 ml/s comptés en amont, **4,03 g/s** en
  tasse (4,29 g/s à 15 h 35).
- **Résistance par tranche de 10 g**, même script qu'à 13 h 16 :

  | Tranche | 15 h 35 la veille (88 °C, sans screen) | 13 h 49 | NTC | Corrigée |
  | --- | ---: | ---: | ---: | ---: |
  | 10 → 20 g | 4,40 g/s, 4,86 bar, 1,10 bar·s/g | 4,13 g/s, 5,22 bar, **1,26 bar·s/g** | 83,8 °C | 1,20 |
  | 20 → 30 g | 4,43 g/s, 4,88 bar, 1,10 | 3,89 g/s, 5,39 bar, **1,39** | 83,5 °C | 1,32 |
  | 30 → 40 g | 4,37 g/s, 4,82 bar, 1,10 | 4,14 g/s, 5,51 bar, **1,33** | 82,1 °C | 1,24 |

  Corrigée de la viscosité (à 88 °C), la résistance est **≈ 12 % au-dessus
  de 15 h 35** (1,25 contre 1,12 bar·s/g corrigés ; +20 % sans correction). Pas de baisse d'une
  tranche à l'autre : pas d'érosion mesurable.
- Arrivée en tasse, depuis la première goutte :

  | 1 g | 5 g | 10 g | 20 g | 30 g | 40 g (seuil) |
  | ---: | ---: | ---: | ---: | ---: | ---: |
  | 0,9 s | 2,2 s | 3,5 s | 5,9 s | 8,3 s | 10,8 s |

- Arrêt : seuil de 40,0 g franchi à 23,73 s ; **42,9 g final**. 3,4 g
  arrivent après le seuil. Écart à la cible : **−2,1 g**.
- Après l'arrêt, la pression lue monte de 5,61 à 9,25 bar pompe arrêtée,
  9,57 bar au plus (+2,4 bar à 15 h 35). Cause, selon David : la chauffe
  reprend (NTC de 80,5 à 82,5 °C, 94,5 à 96,5 °C réels avec l'offset)
  dans un circuit fermé par la galette ; l'eau qui se dilate fait monter
  la pression.
- Ratio final **1:2,9**. **10,6 s de la goutte à l'arrêt** ;
  **21,6 s du démarrage de la pompe à l'arrêt** (21,7 s à 15 h 35).
  64,6 ml admis.

**Thermique.**

| Repère | Mesure |
| --- | ---: |
| NTC au départ | 86,05 °C |
| Pic avant l'infusion | 86,10 °C, +0,05 °C |
| NTC au début de la pré-infusion | 85,77 °C |
| NTC au début de l'infusion | 83,62 °C, −2,2 °C pendant la pré-infusion |
| Commande à 0 % (remboursement) | **3,30 s**, de 12,22 à 15,52 s |
| Appoint repris avant l'arrêt | 8,2 s |
| NTC à l'arrêt | 81,63 °C ; −2,0 °C pendant l'infusion |
| **Tasse** | **83,16 °C, −2,84 °C** |
| Minimum | **80,54 °C**, 3,5 s après l'arrêt ; 82,45 °C en fin de capture (10 s) |

- Énergie, commande / SSR : précharge 1,95 / 2,03 kJ, remplissage 6,58 /
  6,46 kJ, pré-infusion 2,63 / 2,85 kJ, infusion 7,91 / 6,95 kJ.
- Moyenne en volume amont : 84,38 °C.
- Aucune lecture NTC invalide.
- Modèle : 82,19 °C en tasse avec la commande réelle, **−0,97 °C**
  d'écart ; minimum de 80,99 °C pour 80,54 °C mesurés. En simulation sur
  cette capture, 8 s de précharge donnent 86,29 °C en tasse contre
  82,76 °C avec 3 s.

**Attendus vérifiés.**

| Attendu (mouture espresso) | Mesure |
| --- | --- |
| Résistance ≈ 1,1 bar·s/g, montante de ≈ 5 à 10 %, constante une fois corrigée | **1,26 → 1,39 → 1,33**, 1,25 corrigée : ≈ 12 % au-dessus ; ni baisse ni montée régulière |
| Pression au plateau ≈ 4,8 bar | **4,9 → 5,58 bar**, plus haute ; pompe bridée à 86–92 % par le PI |
| Débit en tasse ≈ 4,4 g/s | **4,03 g/s**, plus bas : résistance plus haute et pompe bridée |
| De la goutte à l'arrêt ≈ 10 s | 10,6 s, atteint |
| Remplissage ≈ 20 ml en ≈ 5 s, ≈ 0,95 bar | **23,2 ml en 6,10 s**, 0,95 bar |
| Pré-infusion finie à l'échéance | 4,12 s, échéance, atteint |
| Pompe avant la goutte ≈ 10,5 s | 11,0 s, atteint |
| Démarrage de la pompe → arrêt ≈ 20 s | 21,6 s |
| Poids final ≈ 44 g | 42,9 g |
| NTC tasse ≈ 82 à 83 °C | **83,16 °C**, atteint |
| Minimum sous 79 °C | **80,54 °C** : la pré-infusion à faible débit étale la chute |

**Lecture.**

- *Hydraulique proche de 15 h 35, mesuré.* Mêmes temps (11,0 contre
  12,0 s avant la goutte, 10,6 contre 9,7 s ensuite, 21,6 contre 21,7 s
  au total), même ratio ; pression ≈ 11 % et résistance corrigée ≈ 12 % plus
  hautes, débit ≈ 6 % plus bas. Le puck screen, le remplissage à 100 % et la mouture
  (réglée à la main, peut-être pas au même cran) peuvent expliquer la
  résistance plus haute ; rien ne les sépare.
- *Température, mesuré.* **83,16 °C en tasse contre 89,37 °C** à
  15 h 35, −6,2 °C : c'est l'écart le plus grand entre les deux shots.
  Il vient de la consigne (−2 °C) et de la précharge (≈ −3,5 °C en
  simulation pour 8 → 2 s).
- *Goût, estimé.* Moins amer qu'à 13 h 16, à température égale : la
  mouture espresso fait mieux que la mouture grosse. Moins bon qu'à
  15 h 35, à hydraulique proche : la température est la cause la plus
  probable. Hypothèse non testée tant que la température n'est pas
  remontée seule.
- *Régulation de pression, mesuré.* Avec une galette à ≈ 1,3 bar·s/g, la
  pompe à 100 % pousserait vers 6 bar. Le PI, réglé pour l'espresso à
  9 bar, prend la main dès 4 bar et retire ≈ 10 % de pompe : il borne le
  shot vers 5,5 bar et 4,0 g/s. C'est le plafond de 6 bar qui joue, comme
  le veut la recette du papier ; `brew.target_pressure_bar` le règle.

**Décision de David** : **88 °C et 8 s de précharge**, seul changement ;
mouture espresso et tout le reste comme à 13 h 49. L'essai dit si la
température fait l'écart de goût avec 15 h 35 la veille. S'il est trop
chaud ensuite : garder 88 °C et raccourcir la précharge, ≈ 0,6 à 0,7 °C
de moins en tasse par seconde retirée (simulé).

| Point | Attendu si seule la température change |
| --- | --- |
| NTC tasse | ≈ **89 °C** : +2 °C de consigne, ≈ +3,5 °C de précharge en simulation, ± 1 °C ; 89,37 °C à 15 h 35 |
| Pression, débit, temps | comme à 13 h 49 : 4,9 → 5,6 bar, ≈ 4,0 g/s, ≈ 10,6 s de la goutte à l'arrêt, ≈ 21,6 s au total ; l'eau plus chaude, moins visqueuse (≈ −8 %), donne un peu plus de débit et un peu moins de pression |
| Résistance mesurée | ≈ 1,15 à 1,25 bar·s/g, la viscosité en moins ; corrigée, ≈ 1,25 comme aujourd'hui |
| Poids final | ≈ 43 g |

### 28 octobre, 10 h 09 — `261028-100949`

**Contexte.** Parenthèse dans les turbo shots : David a bientôt plus de
café et revient au profil « single espresso » (8,0 g, panier LM1 41 mm),
**avec le porte-filtre à bec**. Valeurs du profil lues dans la capture,
celles du tableau des réglages : 91 °C, précharge de 8 s, remplissage à
70 % jusqu'à 0,8 bar, pré-infusion à 50 % (4 s ou goutte), 9 bar, arrêt à
22 g. Même café (Cozy Chocolate, 20 jours depuis la torréfaction) ; la
mouture n'est pas signalée comme changée depuis le turbo shot de la
veille à 13 h 49 (mouture espresso). Écran et carte sensors 0.4.24.
Observations de David : **le goût est vraiment bien** ; la première
goutte lui semble **retardée par la géométrie du bec**.

**Routine** : guide ouvert à 81,8 °C ; rinçage 35,4 s après l'ouverture
(NTC à 86,4 °C), de 5,01 s, 18,0 ml, 23,9 g, pression maximale 0,31 bar ;
retour à la consigne en 593,4 s (minimum 79,3 °C, résistance active
93,6 s) ; porte-filtre en place 628,2 s à la consigne, **1222 s sur le
groupe** après le rinçage ; préparation de 259,0 s. Balance absente
25,5 s en tout pendant la routine, avant l'infusion ; les poids de
l'infusion sont continus.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,00 s | — | **0,51 → 0,58 bar** |
| Remplissage | 8,00 s | **5,42 s**, pression | **17,1 ml** (3,16 ml/s) | 0,58 → 0,81 bar |
| Pré-infusion | 13,42 s | **4,11 s, échéance** | 5,8 ml (1,41 ml/s) | 0,81 → 1,32 bar |
| Infusion | 17,53 s | **21,76 s** | 35,4 ml | 9,01 bar max |

- Remplissage fini sur la pression : premier paquet au-dessus de 0,8 bar
  à 13,2 s (16,5 ml), passage à 13,42 s. Le circuit part de **0,51 bar**
  (0,40 bar le 25/10, 0,22 à 0,30 bar le 24/10) ; 17,1 ml est le
  remplissage le plus court de la synthèse.
- Volume admis à chaque palier de pression :

  | Repère | 28/10 10 h 09 (bec) | 25/10 9 h 11 | Écart |
  | --- | ---: | ---: | ---: |
  | 0,8 bar | 16,5 ml | 20,9 ml | −4,4 ml |
  | 1 bar | 19,7 ml | 23,5 ml | −3,8 ml |
  | 2 bar | 25,8 ml | 29,9 ml | −4,1 ml |
  | 4 bar | 29,0 ml | 33,6 ml | −4,6 ml |
  | 8 bar | 32,5 ml | 36,8 ml | −4,3 ml |

- Pré-infusion **finie à l'échéance** : la balance reste à 0,0 g pendant
  toute la phase.
- **Première goutte (0,3 g) à 21,27 s, à 8,98 bar**, 33,0 ml, 3,7 s
  après la fin de la pré-infusion et 0,3 s après le passage de 8 bar.
  Marge de remplissage : **15,9 ml**, la plus grande de la synthèse. Sans
  fond, la goutte arrivait à 1,28 bar le 25/10 et à 1,85 bar le 24/10 à
  13 h 23 ; avec le bec le 18/10, à 1,42 bar.
- Montée de 3,4 s jusqu'à 8 bar, pompe de 50 à 100 %, **0,2 g en tasse à
  8 bar** (2,8 g le 25/10, 6,7 g le 18/10). Pas de pic : 8,99 bar au
  passage, la pompe redescend à 66 %. Le café arrive d'un coup : 1,0 g
  0,6 s après 8 bar, puis ≈ 1 g/s.
- Depuis 8 bar : **18,3 s**, 1,16 g/s en tasse, 1,38 ml/s en amont
  (16,8 s, 1,09 g/s et 1,29 ml/s le 25/10). 21,7 g à l'arrêt, 23,1 g
  final, **1,4 g de gouttes** (0,9 g le 25/10, 2,0 g avec le bec le
  18/10). Pompe en marche 31,2 s.
- Plateau (de 8 bar + 2 s à l'arrêt) : 8,91 ± 0,04 bar, pompe à 67 %,
  1,20 g/s, soit **7,4 bar·s/g** (7,9 le 25/10 avec le même script ; 7,8
  dans son entrée). Rapport tasse / amont : 0,84 g/ml, comme le 25/10.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 28/10 10 h 09 (bec) | **4,0 s** | 7,9 s | 12,6 s | **20,5 s** |
  | 25/10 9 h 11 | 1,6 s | 5,7 s | 10,2 s | 19,1 s |
  | 24/10 13 h 23 | 2,3 s | 5,2 s | 9,2 s | 16,5 s |
  | 18/10 12 h 40 (bec) | 0,7 s | 3,3 s | 5,9 s | 11,4 s |

**Thermique.**

| Repère | 28/10 10 h 09 | 25/10 9 h 11 |
| --- | ---: | ---: |
| NTC au départ | 91,12 °C, +0,12 °C | 91,00 °C |
| NTC pendant le remplissage | **+2,30 °C**, sans creux | +1,22 °C |
| Pic avant l'infusion | **93,63 °C** à 14,8 s | 92,94 °C à 12,5 s |
| NTC au début de l'infusion | **93,13 °C**, +2,13 °C | 92,35 °C |
| Commande à 0 % (remboursement) | 16,5 s | 16,7 s |
| Appoint repris avant l'arrêt | 5,3 s | 3,7 s |
| NTC à l'arrêt | 88,97 °C | 89,30 °C |
| **Tasse** | **92,04 °C**, +1,04 °C | 91,17 °C |
| Minimum | 87,47 °C, 5,9 s après l'arrêt ; 88,04 °C en fin de capture (10 s) | 87,89 °C, 6,1 s après |

- Énergie, commande / SSR : précharge 8,43 / 8,28 kJ, remplissage 5,48 /
  5,18 kJ, pré-infusion 2,49 / 2,33 kJ, infusion 2,69 / 2,66 kJ.
- Remplissage : commande de 77 à 90 %.
- Moyenne en volume amont : 92,38 °C.
- Cinq lectures NTC invalides (201 à 337 ms), toutes après l'arrêt de la
  pompe : aucune pendant l'écoulement.
- Modèle : 92,59 °C avec la commande réelle, **−0,53 °C** (mesure du
  simulateur : 92,06 °C).

**Attendus vérifiés.** Ceux du 27 octobre à 13 h 49 visaient le turbo
shot suivant : non vérifiés. La dernière entrée du single espresso
(25/10) n'en fixait pas ; ceux du 24/10 à 10 h 22, repris le 25/10,
portaient sur les mêmes réglages sans bec.

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 6,3 à 7,3 bar·s/g ; 12 à 16 s depuis 8 bar ; plateau sous 9,2 bar | **7,4 bar·s/g, 18,3 s : non atteint** ; 9,01 bar : atteint |
| Remplissage | 3,1 à 3,3 ml/s à 70 %, fin en 8,5 à 9,5 s | 3,16 ml/s : atteint ; **5,42 s : non atteint** |
| Pré-infusion | première goutte sous 2 bar | **8,98 bar : non atteint**, bec |
| Chauffe | 0 % pendant 14 à 17 s ; modèle à ±0,6 °C | 16,5 s : atteint ; −0,53 °C : atteint |
| Tasse | 91,6 à 92,6 °C | 92,04 °C : atteint |

**Lecture.**

- *Le bec retient le filet de basse pression, mesuré et estimé.* Sans
  fond, le 25/10, le café passait en petit filet avant 8 bar : 0,6 g à
  1,5 bar, 1,7 g à 4 bar, 2,8 g à 8 bar. Ici, rien n'arrive en tasse
  avant 8,8 bar (mesuré). Le manque à 8 bar, ≈ 2,6 g, est ce filet : il
  mouille le dessous du panier et l'intérieur du bec avant de couler
  (estimé). Ensuite le débit au plateau est le même, et le retard ne se
  rattrape pas : ≈ 1,5 à 2,4 s à 5, 10 et 20 g. Le 18/10, avec le bec,
  la goutte arrivait à 1,42 bar parce que la galette, bien plus rapide,
  avait déjà donné 6,7 g à 8 bar : le bec ne retarde la goutte que si le
  filet à basse pression est faible.
- *La galette n'a pas changé, mesuré.* 7,4 contre 7,9 bar·s/g, même
  rapport tasse / amont. L'écart de ≈ 4,3 ml aux paliers de pression est
  constant de 0,8 à 8 bar : c'est moins de volume vide au-dessus de la
  galette, pas une galette plus dure. Le bec, sous la galette, ne peut
  pas en être la cause ; le 18/10, avec le bec, le remplissage admettait
  24,3 ml jusqu'à 0,8 bar. Le circuit à 0,51 bar au départ en explique
  une partie (estimé) ; le reste peut venir de la galette (dose,
  tassage), rien ne le sépare.
- *Plus de café extrait que pesé, estimé.* À poids presque égal à
  l'arrêt (21,7 contre 21,5 g), ≈ 3,5 ml de plus ont traversé la
  galette qu'au 25/10, une fois retiré l'écart de volume vide. Une
  partie revient en gouttes (+0,5 g), ≈ 2 g restent sur le métal du bec.
  La galette a donc donné l'équivalent de ≈ 24 à 25 g pour 23,1 g en
  tasse, et 20 g arrivent 1,4 s plus tard. C'est une extraction un peu
  plus poussée, avec un goût jugé vraiment bien : ça va dans le sens de
  l'explication de David du 20/10 (un arrêt plus tardif apporte la partie
  sucrée), sans la tester, car le café, la routine et la température ont
  aussi changé.
- *Fin de pré-infusion, mesuré.* Avec le bec, la balance ne voit plus la
  goutte pendant la pré-infusion : elle finit toujours à l'échéance de
  4 s. Sans effet ici, la goutte du 25/10 n'y arrivait que 0,5 s plus tôt.
- *Tasse chaude, mesuré.* Le remplissage court (17,1 ml) fait passer
  moins d'eau froide sous la précharge : NTC +2,30 °C pendant le
  remplissage, infusion à 93,13 °C, tasse à 92,04 °C, +0,87 °C sur le
  25/10. Même lien que le 20/10 (21,4 ml, +2,25 °C). La NTC ne voit pas
  l'eau qui refroidit dans le bec.
- *Porte-filtre longtemps sur le groupe.* 1222 s, la durée la plus
  longue des routines du single espresso ; la tasse est bonne, comme le
  20/10 après 962 s.

**Décision.** Pas de décision de David à ce jour. Les attendus supposent
les mêmes réglages, le porte-filtre à bec, le même café et la même
mouture. Seule une infusion sans fond le même jour, à mouture égale,
séparerait l'effet du bec de celui du café qui vieillit.

| Point | Attendu |
| --- | --- |
| Première goutte | **entre 8 et 9 bar**, moins de 0,5 g en tasse à 8 bar ; pré-infusion finie à l'échéance (4 s). Une goutte sous 2 bar infirmerait la rétention du bec, ou dirait que la galette est plus rapide (plus de 5 g à 8 bar) |
| Hydraulique | 7,0 à 8,0 bar·s/g ; 16,5 à 19,5 s depuis 8 bar ; plateau sous 9,2 bar |
| Rendement | ≈ 21,6 g à l'arrêt ; 1,2 à 1,8 g de gouttes, soit 22,8 à 23,4 g final |
| Remplissage | 3,1 à 3,3 ml/s à 70 % ; 17 à 22 ml selon la pression au départ ; plus il est court, plus la NTC monte |
| Chauffe | NTC +1,2 à +2,4 °C pendant le remplissage ; 0 % pendant 15 à 17 s ; modèle entre −1,1 et −0,3 °C |
| Tasse | 91,4 à 92,4 °C |

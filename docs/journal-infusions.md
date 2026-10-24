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

Configuration sauvegardée `configs/261005-075907.json` (schéma 9). Seuls le
seuil de remplissage et la puissance de pré-infusion diffèrent de
`configs/261004-142011.json`, en vigueur le 5 octobre à 7 h 26.

| Réglage | Valeur | Depuis |
| --- | --- | --- |
| Firmware écran | 0.3.40 : remboursement de la précharge, sans coupure de fin, fin de remplissage en pression absolue | avant le 5 octobre |
| Offset NTC | **−14 °C** (écran 0.4.4), mesuré au panier avec la SCACE ; −10,5 °C avant. Consigne bornée à 95 °C, coupure à 100 °C utilisateur (114 °C sonde) | 10 octobre |
| Sonde de pression | XDB401 analogique sur A2 (0.3.46), loi de chauffe inchangée | 7 octobre |
| Précharge | 8 s à 90 % | 4 octobre |
| Consigne | **92 °C** | 13 octobre, 8 h 18 (90 °C avant) |
| Remplissage | pompe à 75 %, fin à **0,8 bar** (10 s au plus) | 5 octobre, 7 h 59 |
| Pré-infusion | pompe à **50 %**, fin à 4 s ou à la première goutte | 5 octobre, 7 h 59 |
| Infusion | pompe jusqu'à 100 %, cible 9 bar, arrêt à **22 g** ou 27 s, sans ramp-down | 13 octobre, 13 h 29 (23 g du 11 octobre, 13 h 40 au 13 octobre, 8 h 18) |
| Machine | fermée (panneaux posés), filtre ECM Aqua Aroma Crema dans le réservoir ; état final. Remplissage à 3,6 ml/s à 75 % (3,1 ml/s le 10 octobre seulement) | 10 octobre |

Le numéro de firmware est celui du dernier flash connu ; la capture ne
l'enregistre pas.

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

## Entrées

### 5 octobre, 7 h 26 — `261005-072633`

**Contexte.** Première infusion du firmware 0.3.40 : fin de remplissage à
1,0 bar absolu, sans coupure de fin de chauffe. Précharge de 8 s,
remplissage à 75 %, pré-infusion à 35 %. Mouture plus grosse que les
précédentes. Observations de David : le café était visible sous le panier
à la fin du remplissage, la première goutte est venue très tôt ; café jugé
trop rapide, mouture trop grossière.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,04 s | 8,05 s | — | — |
| Remplissage | 8,09 s | 7,00 s | 24,1 ml (3,44 ml/s) | 0,39 → 1,19 bar |
| Pré-infusion | 15,09 s | 1,20 s | 1,2 ml | 1,19 → 1,00 bar |
| Infusion | 16,29 s | 17,80 s | 38,3 ml | 9,46 bar max |

- Premier paquet au-dessus de 1,0 bar à 14,99 s, passage en pré-infusion à
  15,09 s : 100 ms. [firmware.md](firmware.md) décrit une confirmation d'au
  moins 150 ms ; l'écart n'est pas expliqué (décalage de la capture ou
  description imprécise).
- La pression monte vite en fin de remplissage : 0,71 bar à 19,7 ml,
  0,85 bar à 22,0 ml, 1,10 bar à 23,8 ml, soit ≈ 0,5 bar/s.
- Le firmware a vu la première goutte à 16,29 s et 25,2 ml, et a terminé la
  pré-infusion dessus ; le rapport la place à 25,8 ml (≥ 0,3 g). Marge de
  remplissage : 1,1 à 1,7 ml selon la définition.
- La pré-infusion à 35 % ne pousse presque rien : 1,2 ml en 1,2 s ici,
  2,0 ml en 4,0 s le 4 octobre à 13 h 24. À 60 % (30 septembre au
  4 octobre à 10 h 01), elle passait 3,1 à 3,3 ml/s.
- Montée de 4,5 s jusqu'à 8 bar. La pompe passe 2 s à 100 % (≈ 18,5 à
  20,5 s), puis tient 9 bar vers 70 %.
- Depuis 8 bar : 13,3 s, 1,28 g/s en tasse, 1,85 ml/s en amont, 21,4 g à
  l'arrêt, 22,4 g final. C'est **plus lent** que les deux autres infusions
  à mouture grossière (1,47 et 1,59 g/s) et dans la fourchette de 1,4 à
  2 ml/s de [chauffe-infusion.md](chauffe-infusion.md#constats-qui-fondent-la-loi).
  Le jugement « trop rapide » de David ne se lit donc pas dans le débit
  comparé aux infusions précédentes.

**Thermique.**

| Grandeur | Mesure |
| --- | ---: |
| NTC pondérée par la tasse | **88,64 °C** (−1,36 °C) |
| NTC pondérée par le volume | 89,62 °C |
| Pic avant l'infusion | 91,58 °C |
| Minimum pendant l'écoulement | 87,40 °C, à l'arrêt de la pompe |
| Minimum total | 86,20 °C, ≈ 5 s après l'arrêt (−3,88 °C) |
| 20 s après l'arrêt (fin de la capture) | 88,30 °C, en hausse |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 8,54 kJ | 8,28 kJ |
| Remplissage | 6,53 kJ | 5,64 kJ |
| Pré-infusion | 1,12 kJ | 1,08 kJ |
| Infusion | 3,45 kJ | 3,24 kJ |
| Récupération | 1,80 kJ | 2,28 kJ |

La commande est restée à 0 % de 16,49 à 28,89 s (remboursement de la
précharge), puis a suivi le débit à 45–62 % jusqu'à l'arrêt. Besoin estimé
de l'eau admise (64,9 ml, eau à 24,5 °C supposée) : 19,1 kJ. Le SSR a fourni
18,2 kJ de la précharge à l'arrêt, la commande 19,6 kJ. L'écart vient surtout
du remplissage : les hausses de commande attendent la fenêtre SSR suivante.

**Attendus vérifiés** (section « Prochain essai » de
[chauffe-chaudiere.md](chauffe-chaudiere.md), écrite pour 0.3.40) :

| Attendu | Mesure |
| --- | --- |
| Commande à 0 % dès l'infusion, ≈ 700 %·s retenus, puis appoint jusqu'à l'arrêt | oui : 0 % pendant 12,4 s, puis 45–62 % jusqu'à l'arrêt |
| ≈ 10 ml de plus dans le remplissage | oui : 24,1 ml contre 13,6 à 15,4 ml |
| Moyenne en tasse inchangée par la suppression de la coupure | invérifiable sans essai A/B ; en simulation, −0,03 °C avec la coupure |
| État 20 à 30 s après l'arrêt proche de la consigne ou au-dessus | **non** : 88,30 °C à +20 s, encore en hausse ; la capture s'arrête là |

**Lecture.**

- *Coupure de fin, en simulation.* La loi 0.3.38 aurait laissé la commande
  à 0 % de 16,5 s à l'arrêt : le remboursement n'était pas soldé quand la
  coupure (11 s avant l'arrêt) aurait commencé. Rejouée, elle donne la même
  moyenne en tasse (89,60 contre 89,63 °C), un minimum plus bas de 0,6 °C et
  2,3 °C de moins 20 s après l'arrêt. Le modèle se trompe de 2,4 °C sur ce
  dernier point (90,70 simulé, 88,30 mesuré) : seul le sens de l'écart est
  fiable. Pas de dépassement après l'arrêt cette fois : la coupure n'avait
  rien à corriger.
- *Baisse de 3,88 °C, décomposition estimée.* Rejouée avec la commande et
  sans chauffe après l'arrêt, la NTC simulée descend à 88,19 °C puis revient
  à 90,42 °C : le montant remboursé est bon, et ≈ 1,9 °C de baisse sont du
  transit (chauffe ≈ 17,6 s jusqu'à la sonde, eau froide ≈ 6,4 s). Avec
  l'énergie SSR, minimum à 87,25 °C et retour à 89,47 °C : ≈ 0,9 °C vient du
  SSR en retrait. Le reste, ≈ 1 °C, est le facteur que le modèle ne connaît
  pas, déjà vu à 13 h 24 (−2 °C). Piste non vérifiée : eau du réservoir sous
  24,5 °C à 7 h 26 (≈ 1,4 % de besoin en plus par kelvin).
- *Tasse froide.* Rien n'attribue les −1,36 °C au remboursement : aucune
  variante sans remboursement n'a été rejouée sur cette capture. Plus chaude
  qu'à 13 h 24 (87,58 °C), mais deux changements séparent les deux
  infusions et rien ne départage leurs effets.
- *Énergie et seuil de remplissage.* Si la dette est soldée avant l'arrêt
  (ici à 28,9 s), l'énergie totale vaut l'appoint sur toute l'eau admise,
  quelle que soit la phase où elle passe. Le seuil ne déplace alors que le
  moment de la chauffe. Cela contredit le « ≈ +2,9 kJ » attendu du seuil à
  1 bar dans [chauffe-chaudiere.md](chauffe-chaudiere.md) ; point à
  vérifier.
- *Flow profiling, en estimation.* Un plafond de 1,2 g/s en tasse aurait peu
  changé cette infusion : 1,25 à 1,35 g/s au plateau, soit un plateau vers
  8,3 bar si la galette est linéaire, et ≈ 1,5 s de plus. Il aurait surtout
  retenu la montée (1,45 g/s à 5,2 bar, pompe à 100 % pendant 2 s).
  Avec une mouture nettement plus grossière, la pression resterait basse :
  le flow profiling égalise la durée, pas l'extraction. Option à garder :
  plafond de débit sous un plafond de pression de 9 bar.

**Décision.** Remplissage gardé à 75 % : remplir lentement dégrade la
galette, qui devient perméable à faible pression ; à 70 %, le remplissage
serait 10 % plus long pour ≈ 0,1 ml de dépassement en moins (débit libre de
4,64 contre 5,12 g/s, [purge_sans_resistance.html](purge_sans_resistance.html)).
Fin de remplissage à **0,8 bar** et pré-infusion à **50 %**. Un seuil plus
bas (0,7 bar) laissait ≈ 4,9 ml de marge, que la pause à 35 % n'aurait pas
comblés : pré-infusion terminée à l'échéance. La pompe à 50 % doit pousser
la galette jusqu'à la goutte.

Les deux changements se séparent : le seuil ne touche que la fin du
remplissage, la puissance ne touche que la pré-infusion.

**Attendus pour l'infusion suivante** (estimés sur la courbe de cette
capture, même mouture supposée) :

| Point | Attendu |
| --- | --- |
| Paquet au-dessus de 0,8 bar | ≈ 21,2 ml, ≈ 6,2 s après le début du remplissage |
| Fin du remplissage | ≈ 21,5 ml, ≈ 6,3 s |
| Marge avant la première goutte | ≈ 3,7 ml (goutte à 25,2 ml selon le firmware) ; dépend de la mouture |
| Pré-infusion à 50 % | débit libre de 2,06 g/s à froid, moins sous la galette ; si elle passe 1,5 à 2,5 ml/s, la goutte tombe en 1,5 à 2,5 s et termine la pré-infusion avant l'échéance |
| Si la pré-infusion va à l'échéance | la galette retient plus d'eau qu'ici, ou 50 % ne pousse pas : relever le volume passé |
| Thermique | pas d'attendu chiffré : noter commande et SSR par phase, et la NTC 20 s après l'arrêt |

Si possible, prolonger la capture à 40–60 s après l'arrêt pour voir où la
NTC se stabilise.

### 6 octobre, 7 h 19 — `261006-071953`

**Contexte.** Premiers essais du seuil de remplissage à 0,8 bar et de la
pré-infusion à 50 % (`configs/261005-075907.json`), firmware et mouture
inchangés depuis le 5 octobre. Observations de David : débit assez élevé,
amertume en bouche ; mouture jugée trop grossière.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,00 s | — | — |
| Remplissage | 8,00 s | 6,10 s | 21,2 ml (3,47 ml/s) | 0,38 → 0,90 bar |
| Pré-infusion | 14,10 s | 2,10 s | 4,1 ml (1,93 ml/s) | 0,90 → 0,98 bar |
| Infusion | 16,20 s | 16,50 s | 36,8 ml | 9,07 bar max |

- Premier paquet au-dessus de 0,8 bar à 13,90 s (20,6 ml, 5,9 s après le
  début du remplissage), passage en pré-infusion à 14,10 s : 200 ms,
  compatible avec la confirmation d'au moins 150 ms de
  [firmware.md](firmware.md). Le 5 octobre, l'écart était de 100 ms.
- Le firmware a vu la première goutte à 16,20 s et 25,2 ml, comme le
  5 octobre, et a terminé la pré-infusion dessus, 1,9 s avant l'échéance ;
  le rapport la place à 25,5 ml (≥ 0,3 g). Marge de remplissage : 4,0 à
  4,3 ml selon la définition.
- Montée de 4,8 s jusqu'à 8 bar, avec 5,5 g déjà en tasse (4,4 g le
  5 octobre). Le débit en tasse culmine à 1,8 g/s vers 19,5 s, sous
  ≈ 4 bar.
- Depuis 8 bar : 11,8 s, 1,35 g/s en tasse, 1,89 ml/s en amont, 21,6 g à
  l'arrêt, 22,6 g final. Plus court et plus rapide que le 5 octobre
  (13,3 s, 1,28 g/s).
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,59 bar en moyenne, pompe
  à 69 %, 1,37 g/s, soit 6,3 bar·s/g contre 6,8 le 5 octobre (pompe à
  70 %). Galette ≈ 8 % moins résistante, à mouture inchangée.

**Thermique.**

| Grandeur | Mesure |
| --- | ---: |
| NTC pondérée par la tasse | **88,47 °C** (−1,53 °C) |
| NTC pondérée par le volume | 89,68 °C |
| Pic avant l'infusion | 92,03 °C |
| Minimum pendant l'écoulement | 86,74 °C, à l'arrêt de la pompe |
| Minimum total | 85,89 °C à 37,2 s, ≈ 4,4 s après l'arrêt (−3,92 °C) |
| 20 s après l'arrêt (fin de la capture) | 87,74 °C, en hausse |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 8,64 kJ | 8,64 kJ |
| Remplissage | 5,61 kJ | 4,68 kJ |
| Pré-infusion | 1,83 kJ | 1,68 kJ |
| Infusion | 2,81 kJ | 2,76 kJ |
| Récupération | 2,43 kJ | 2,76 kJ |

La commande, à 90 % pendant le remplissage, descend de 78 à 52 % pendant
la pré-infusion, reste à 0 % de 16,20 à 27,70 s (11,5 s), puis suit le
débit à 36–57 % jusqu'à l'arrêt. De la précharge à l'arrêt : 18,9 kJ
commandés, 17,8 kJ fournis par le SSR, pour 62,3 ml admis. Comme le
5 octobre, l'écart vient surtout du remplissage.

**Attendus vérifiés** (entrée du 5 octobre) :

| Attendu | Mesure |
| --- | --- |
| Paquet au-dessus de 0,8 bar à ≈ 21,2 ml, ≈ 6,2 s | 20,6 ml, 5,9 s |
| Fin du remplissage à ≈ 21,5 ml, ≈ 6,3 s | 21,2 ml, 6,1 s |
| Marge de ≈ 3,7 ml avant la goutte (25,2 ml selon le firmware) | 4,0 ml, goutte à 25,2 ml |
| Pré-infusion à 1,5 à 2,5 ml/s, goutte en 1,5 à 2,5 s avant l'échéance | 1,93 ml/s, goutte en 2,1 s |
| Thermique : noter commande, SSR et NTC à +20 s | ci-dessus ; 87,74 °C à +20 s |
| Capture prolongée à 40–60 s après l'arrêt | **non** : 19,9 s |

**Lecture.**

- *Mouture, mesuré.* Tout désigne une galette trop perméable : débit en
  tasse de 1,8 g/s sous 4 bar pendant la montée, 5,5 g en tasse à 8 bar,
  11,8 s depuis 8 bar, résistance au plateau en baisse. La mouture est la
  même que le 5 octobre ; la différence de résistance vient de la
  préparation ou de la pré-infusion, sans mesure pour départager.
- *Pré-infusion, mesuré.* À 50 %, elle passe 4,1 ml et ne laisse pas de
  pause sans débit : la galette est mouillée jusqu'à la goutte, comme
  attendu. Qu'elle soit pour quelque chose dans la perméabilité au plateau
  reste une hypothèse non testée.
- *Amertume, hypothèse non testée.* Une mouture grossière sous-extrait et
  donne plutôt un café acide ou creux. Une amertume avec un débit élevé
  évoque une extraction inégale (canal) ou de l'astringence. La pression
  ne s'effondre pas au plateau, aucun canal franc n'est visible.
- *Pic avant l'infusion, estimé.* 92,03 °C contre 91,58 °C le 5 octobre :
  le remplissage s'arrête 2,9 ml plus tôt, donc moins d'eau froide sous la
  précharge, pour une commande identique jusque-là.
- *Tasse froide, en simulation.* Rejouée avec la commande réelle, la
  moyenne en tasse vaut 89,15 °C (0,68 °C au-dessus de la mesure), le
  minimum 87,88 °C (2,0 °C au-dessus) et le pic 90,66 °C (1,4 °C
  en dessous). Le modèle surestime le creux comme le 5 octobre ; le
  facteur inconnu reste inexpliqué.

**Décision.** David affine la mouture pour la prochaine infusion. Réglages
inchangés (`configs/261005-075907.json`), pour attribuer à la mouture seule
les écarts de la prochaine capture.

**Attendus pour l'infusion suivante** (même configuration, mouture plus
fine) :

| Point | Attendu |
| --- | --- |
| Remplissage | fin à 0,8 bar, 20 à 22 ml, ≈ 6 s ; la mouture ne change que la galette sèche, peu le remplissage |
| Pré-infusion à 50 % | moins de 1,9 ml/s ; goutte au-delà de 2,1 s, peut-être à l'échéance de 4 s (≈ 7 ml passés au plus) |
| Montée | moins de 5,5 g en tasse à 8 bar, pic de débit en tasse sous 1,8 g/s |
| Plateau | résistance au-dessus de 6,8 bar·s/g, pompe sous 69 % pour tenir 9 bar |
| Depuis 8 bar | plus de 13,3 s, moins de 1,28 g/s ; au-delà de 27 s le temps arrête la pompe avant 22 g |
| Thermique | pas d'attendu chiffré : la commande à 0 % (≈ 11,5 s ici) couvrira une part plus faible d'une infusion plus longue ; noter commande, SSR et NTC à +20 s |

Prolonger la capture à 40–60 s après l'arrêt pour voir où la NTC se
stabilise.

### 8 octobre, 8 h 43 — `261008-084342`

**Contexte.** Première infusion avec la pression lue sur le XDB401
analogique (écran 0.3.46, voir [xdb401_swap.md](xdb401_swap.md)) ; loi de
chauffe inchangée depuis 0.3.40, réglages d'infusion inchangés
(`configs/261005-075907.json`). Mouture affinée depuis le 6 octobre.
Observations de David : mouture probablement un peu trop fine, infusion
plus longue.

Le changement de sonde touche peu les comparaisons : l'analogique a son
zéro à l'atmosphère (l'I2C lisait +0,027 bar) et lit le plateau de l'OPV
0,06 bar plus haut que l'I2C.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,04 s | 8,05 s | — | — |
| Remplissage | 8,09 s | 6,30 s | 20,0 ml (3,17 ml/s) | 0,48 → 0,88 bar |
| Pré-infusion | 14,38 s | 3,40 s | 6,1 ml (1,79 ml/s) | 0,88 → 1,18 bar |
| Infusion | 17,79 s | 20,40 s | 32,2 ml | 9,07 bar max |

- Pression au repos avant l'infusion : 0,44 bar piégés.
- Premier paquet au-dessus de 0,8 bar à 14,19 s (19,4 ml, 6,1 s après le
  début du remplissage), passage en pré-infusion à 14,38 s : 200 ms, comme
  le 6 octobre. La fin de remplissage sur la sonde analogique se comporte
  comme sur l'I2C.
- Le firmware a vu la première goutte à 17,79 s et 26,1 ml, 0,6 s avant
  l'échéance ; le rapport la place à 26,7 ml (≥ 0,3 g). Marge de
  remplissage : 6,1 à 6,7 ml (4,0 à 4,3 ml le 6 octobre).
- Montée de 4,5 s jusqu'à 8 bar, avec 4,1 g en tasse (5,5 g le 6 octobre).
  Débit en tasse au plus de 1,35 g/s vers 21,3 s, sous ≈ 4,7 bar.
- Depuis 8 bar : 16,0 s, 1,07 g/s en tasse, 1,16 ml/s en amont, 21,5 g à
  l'arrêt (au poids), 22,2 g final.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,91 ± 0,06 bar, pompe à
  67 %, 1,08 g/s, soit **8,3 bar·s/g** contre 6,3 le 6 octobre et 6,8 le
  5 octobre : galette ≈ 30 % plus résistante qu'au 6 octobre.
- Après l'arrêt, la pression piégée tombe à 7,06 bar puis remonte à
  9,35 bar en 9 s, chaudière en chauffe (dilatation), et s'y plafonne
  (9,27 bar en fin de capture) : l'OPV limite le tronçon.

**Thermique.**

| Grandeur | Mesure |
| --- | ---: |
| NTC pondérée par la tasse | **87,69 °C** (−2,31 °C) |
| NTC pondérée par le volume | 89,28 °C |
| Pic avant l'infusion | 91,71 °C, à 13,99 s (fin du remplissage) |
| Minimum pendant l'écoulement | 85,37 °C, à l'arrêt de la pompe |
| Minimum total | 83,41 °C à 45,3 s, ≈ 7,0 s après l'arrêt (6,38 °C sous la NTC de départ, 89,79 °C) |
| 20 s après l'arrêt (fin de la capture) | 86,19 °C, en hausse |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 8,54 kJ | 8,40 kJ |
| Remplissage | 5,45 kJ | 4,68 kJ |
| Pré-infusion | 2,45 kJ | 2,28 kJ |
| Infusion | 2,06 kJ | 2,04 kJ |
| Récupération | 6,78 kJ | 6,72 kJ |

La commande reste à 0 % de 17,89 à 33,29 s (**15,4 s**, remboursement de
712 %·s), puis suit le débit à 33–37 % pendant les 4,9 dernières secondes.
15,8 des 21,5 g en tasse arrivent pendant ce 0 %. De la précharge à
l'arrêt : 18,5 kJ commandés, 17,4 kJ fournis par le SSR, pour 58,3 ml
admis, soit 0,298 kJ/ml fournis (0,286 le 6 octobre, 0,280 le 5 octobre).

**Attendus vérifiés** (entrée du 6 octobre) :

| Attendu | Mesure |
| --- | --- |
| Remplissage : fin à 0,8 bar, 20 à 22 ml, ≈ 6 s | 20,0 ml, 6,3 s |
| Pré-infusion sous 1,9 ml/s, goutte au-delà de 2,1 s, ≈ 7 ml au plus | 1,79 ml/s, goutte en 3,4 s, 6,1 ml |
| Montée : moins de 5,5 g à 8 bar, pic sous 1,8 g/s | 4,1 g, 1,35 g/s |
| Plateau : au-dessus de 6,8 bar·s/g, pompe sous 69 % | 8,3 bar·s/g, 67 % |
| Depuis 8 bar : plus de 13,3 s, moins de 1,28 g/s | 16,0 s, 1,07 g/s ; arrêt au poids, pas au temps |
| Thermique : noter commande, SSR et NTC à +20 s | ci-dessus ; 86,19 °C à +20 s |
| Capture prolongée à 40–60 s après l'arrêt | **non** : 19,9 s |

**Lecture.**

- *Énergie, mesuré.* La tasse froide ne vient pas d'un manque d'énergie :
  le SSR a fourni plus par millilitre que les deux infusions précédentes.
  Ce qui change, c'est le moment : à débit lent, la dette se rembourse
  lentement, la commande reste à 0 % 15,4 s au lieu de 11,5 à 12,4 s, et
  l'appoint ne reprend que 4,9 s avant l'arrêt. Avec ≈ 17,6 s de délai
  chauffe → sonde, il arrive après l'arrêt ; la NTC baisse encore 7 s après.
- *Perte thermique proportionnelle à la durée, estimé.* Une fuite de
  chaleur expliquerait mal l'écart : 4 s d'infusion de plus pour ≈ 0,8 °C
  de moins en tasse qu'au 6 octobre demanderait ≈ 300 W de pertes en plus
  (1,47 kJ/K), un quart de la résistance.
- *Écart au modèle, en simulation.* Rejouée avec la commande réelle, la
  moyenne en tasse vaut 89,85 °C (mesure −2,14 °C), le minimum 88,93 °C
  (mesure −5,5 °C), le pic 90,55 °C (mesure +1,2 °C). La NTC réelle baisse
  bien plus que le modèle. Sur les infusions avec remboursement, l'écart
  suit la durée du 0 % :

  | Infusion | 0 % pendant | Tasse sous 0 % | Mesure − modèle (tasse) |
  | --- | ---: | ---: | ---: |
  | 06/10 7 h 19 | 11,5 s | 14,6 g | −0,68 °C |
  | 05/10 7 h 26 | 12,4 s | 14,7 g | −0,99 °C |
  | 08/10 8 h 43 | 15,4 s | 15,8 g | −2,14 °C |
  | 04/10 13 h 24 (coupure de fin en plus) | 21,8 s | 19,3 g | −1,99 °C |

  Avant le remboursement, résistance en marche pendant l'écoulement,
  l'écart était positif (+1,9 °C à 9 h 43 le 30 septembre). Hypothèse non
  testée : sans chauffe, plus de brassage par convection ; l'eau froide
  admise stratifie autour de la NTC, qui voit une capacité locale plus
  petite que les 1,47 kJ/K du modèle. Ce serait le « facteur manquant » de
  [chauffe-infusion.md](chauffe-infusion.md#simulation).
- *Eau du réservoir.* Non mesurée ; les trois dernières infusions sont du
  matin, rien ne la sépare de la durée.

**Décision.** David grossit un peu la mouture pour la prochaine infusion.
Réglages inchangés (`configs/261005-075907.json`). La loi de chauffe reste
telle quelle jusqu'au panier SCACE, attendu dans deux infusions : il dira
si le creux de la NTC est aussi celui de l'eau au groupe avant de toucher
au remboursement (plafond ou plancher de commande).

**Attendus pour l'infusion suivante** (même configuration, mouture entre
celles du 6 et du 8 octobre) :

| Point | Attendu |
| --- | --- |
| Remplissage | fin à 0,8 bar, 19 à 22 ml, ≈ 6 s |
| Pré-infusion à 50 % | goutte entre 2,1 et 3,4 s, 4 à 6 ml |
| Plateau | résistance entre 6,3 et 8,3 bar·s/g, pompe entre 67 et 69 % |
| Depuis 8 bar | entre 11,8 et 16,0 s, entre 1,07 et 1,35 g/s |
| Commande à 0 % | entre 11,5 et 15,4 s ; plus court avec un débit plus fort |
| Moyenne en tasse | entre 87,7 et 88,5 °C si l'écart suit la durée du 0 % ; sous la consigne dans tous les cas |
| Modèle, commande réelle | écart entre −0,7 et −2,1 °C, pour situer le point dans le tableau ci-dessus |

### 8 octobre, 13 h 44 — `261008-134433`

**Contexte.** Firmware, loi de chauffe et réglages d'infusion inchangés
depuis le matin (même remplissage, pré-infusion, cible et arrêt que
`configs/261005-075907.json`). Mouture un peu grossie depuis 8 h 43. Pas de
SCACE dans la capture. Observations de David : la mouture reste trop fine,
mais moins que ce matin ; il change la finesse pas à pas, c'est plus un
chemin qu'un saut. L'arrêt s'est fait quand le filet commençait à blondir :
proche de ce qu'il attend. Le goût est bon.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,01 s | 8,00 s | — | — |
| Remplissage | 8,01 s | 5,80 s | 20,3 ml (3,50 ml/s) | 0,37 → 0,83 bar |
| Pré-infusion | 13,81 s | 3,40 s | 6,7 ml (1,96 ml/s) | 0,83 → 1,18 bar |
| Infusion | 17,21 s | 18,90 s | 33,3 ml | 9,02 bar max |

- Pression au repos avant l'infusion : 0,30 bar piégés (0,44 bar le
  matin).
- Premier paquet au-dessus de 0,8 bar à 13,71 s (19,7 ml, 5,7 s après le
  début du remplissage), passage en pré-infusion à 13,81 s : 100 ms, contre
  200 ms le 6 et le 8 octobre au matin. Même écart inexpliqué que le
  5 octobre au regard des 150 ms de [firmware.md](firmware.md).
- Le firmware a vu la première goutte à 17,21 s et 27,0 ml, 0,6 s avant
  l'échéance, comme le matin ; le rapport la place aussi à 27,0 ml
  (≥ 0,3 g). Marge de remplissage : 6,7 ml (6,1 à 6,7 ml le matin).
- La pré-infusion a duré exactement comme le matin (3,40 s), mais a passé
  6,7 ml au lieu de 6,1 : 1,96 contre 1,79 ml/s.
- Montée de 4,0 s jusqu'à 8 bar (4,5 s le matin), avec 3,8 g en tasse
  (4,1 g le matin). Pompe à 100 % de 19,8 à 20,9 s. Débit en tasse au plus
  de 1,35 g/s vers 20,7 s, sous ≈ 6,2 bar.
- Depuis 8 bar : 15,0 s, 1,15 g/s en tasse, 1,39 ml/s en amont, 21,5 g à
  l'arrêt (au poids), 22,2 g final.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,87 ± 0,07 bar, pompe à
  66 %, 1,18 g/s, soit **7,5 bar·s/g** : entre le 6 octobre (6,3, trop
  grossière) et le matin (8,3, trop fine), ≈ 10 % sous le matin. C'est la
  mesure de ce que vaut le pas de mouture fait à midi.
- Après l'arrêt, la pression piégée tombe à 6,98 bar puis remonte à
  9,38 bar en 8 s, comme le matin.

**Thermique.**

| Grandeur | Mesure |
| --- | ---: |
| NTC pondérée par la tasse | **88,80 °C** (−1,20 °C) |
| NTC pondérée par le volume | 89,93 °C |
| Pic avant l'infusion | 92,07 °C, à 13,41 s (fin du remplissage) |
| Minimum pendant l'écoulement | 87,65 °C, à l'arrêt de la pompe |
| Minimum total | 86,18 °C à 42,0 s, ≈ 5,8 s après l'arrêt (3,81 °C sous la NTC de départ, 89,99 °C) |
| 20 s après l'arrêt (fin de la capture) | 88,23 °C, en hausse |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 8,54 kJ | 8,40 kJ |
| Remplissage | 5,19 kJ | 4,44 kJ |
| Pré-infusion | 2,52 kJ | 2,28 kJ |
| Infusion | 2,27 kJ | 2,28 kJ |
| Récupération | 3,69 kJ | 3,96 kJ |

La commande reste à 0 % de 17,31 à 31,31 s (**14,0 s**), puis suit le
débit à 34–46 % pendant les 4,8 dernières secondes. 15,6 des 21,5 g en
tasse arrivent pendant ce 0 %. De la précharge à l'arrêt : 18,5 kJ
commandés, 17,4 kJ fournis par le SSR, pour 60,3 ml admis, soit
0,289 kJ/ml fournis (0,298 le matin, 0,286 le 6 octobre).

**Attendus vérifiés** (entrée du 8 octobre, 8 h 43) :

| Attendu | Mesure |
| --- | --- |
| Remplissage : fin à 0,8 bar, 19 à 22 ml, ≈ 6 s | 20,3 ml, 5,8 s |
| Pré-infusion : goutte entre 2,1 et 3,4 s, 4 à 6 ml | goutte en 3,4 s ; **6,7 ml**, un peu au-dessus |
| Plateau : entre 6,3 et 8,3 bar·s/g, pompe entre 67 et 69 % | 7,5 bar·s/g ; pompe à **66 %**, un point sous la fourchette |
| Depuis 8 bar : entre 11,8 et 16,0 s, entre 1,07 et 1,35 g/s | 15,0 s, 1,15 g/s |
| Commande à 0 % : entre 11,5 et 15,4 s | 14,0 s |
| Moyenne en tasse : entre 87,7 et 88,5 °C, sous la consigne | **non** : 88,80 °C, 0,3 °C au-dessus de la fourchette ; sous la consigne |
| Modèle, commande réelle : écart entre −0,7 et −2,1 °C | −1,33 °C |

**Lecture.**

- *Mouture, mesuré.* Le pas de midi a fait baisser la résistance au plateau
  de 8,3 à 7,5 bar·s/g et la durée depuis 8 bar de 16,0 à 15,0 s. La
  pré-infusion, elle, ne le voit pas : même durée jusqu'à la goutte, un peu
  plus de volume. Si le jugement de David tient (encore trop fine, goût
  bon), la cible est entre 6,3 et 7,5 bar·s/g, soit au plus un pas de plus
  si un pas vaut toujours ≈ 0,8 bar·s/g (estimé sur ce seul pas).
- *Tasse plus chaude que prévu, mesuré et en simulation.* Rejouée avec la
  commande réelle, la moyenne en tasse vaut 90,13 °C (mesure −1,33 °C), le
  minimum 89,11 °C (mesure −2,9 °C), le pic 90,97 °C (mesure +1,1 °C). Le
  point se range dans le tableau du matin, entre le 5 octobre et le matin :

  | Infusion | 0 % pendant | Tasse sous 0 % | Mesure − modèle (tasse) |
  | --- | ---: | ---: | ---: |
  | 06/10 7 h 19 | 11,5 s | 14,6 g | −0,68 °C |
  | 05/10 7 h 26 | 12,4 s | 14,7 g | −0,99 °C |
  | 08/10 13 h 44 | 14,0 s | 15,6 g | −1,33 °C |
  | 08/10 8 h 43 | 15,4 s | 15,8 g | −2,14 °C |

  L'écart suit toujours la durée du 0 %. Interpolé entre 12,4 et 15,4 s,
  il vaudrait ≈ −1,6 °C à 14,0 s : la mesure est ≈ 0,3 °C plus chaude, ce
  qui est aussi l'écart à la fourchette attendue. Deux causes possibles,
  non séparées : l'eau du réservoir, plus chaude à 13 h 44 qu'au matin
  (première infusion de l'après-midi depuis le 4 octobre), ou la dispersion
  ordinaire d'une relation tirée de trois points. La NTC de départ, 0,2 °C
  au-dessus du matin, joue dans le même sens mais le modèle la connaît déjà.
- *Pic avant l'infusion, mesuré.* 92,07 °C contre 91,71 °C le matin, pour
  une commande identique en précharge ; départ 0,2 °C plus haut et
  remplissage 0,5 s plus court.
- *Minimum total, mesuré.* −3,81 °C sous le départ, contre −6,38 °C le matin
  et −3,92 °C le 6 octobre. L'appoint reprend aussi ≈ 5 s avant l'arrêt
  dans les deux infusions du jour ; ce qui change, c'est 1,4 s de 0 % en
  moins et une infusion plus courte de 1,5 s. Que cela suffise à 2,6 °C
  d'écart sur le minimum n'est pas vérifié : le modèle ne reproduit aucun
  des deux creux.

**Décision.** David grossit la mouture d'un pas de plus. Réglages
inchangés (`configs/261005-075907.json`). La mouture est la seule
variable entre cette infusion et la suivante.

**Attendus pour l'infusion suivante** (un pas plus gros, si un pas vaut
≈ 0,8 bar·s/g comme à midi ; estimé sur ce seul pas) :

| Point | Attendu |
| --- | --- |
| Remplissage | fin à 0,8 bar, 19 à 22 ml, ≈ 6 s |
| Pré-infusion à 50 % | goutte en 2,1 à 3,4 s, 4 à 7 ml |
| Plateau | 6,4 à 7,0 bar·s/g, pompe à 68–70 % |
| Depuis 8 bar | 12,5 à 14,5 s, 1,2 à 1,35 g/s |
| Commande à 0 % | 12 à 13,5 s |
| Moyenne en tasse | 88,5 à 89,3 °C si l'écart suit le 0 % ; plus froide si l'infusion est du matin et que l'eau du réservoir compte |
| Modèle, commande réelle | écart de −0,9 à −1,4 °C |
| Filet à 22 g | moins blond qu'ici : à débit plus fort, la galette s'épuise plus tard |

Une infusion du matin et une de l'après-midi à mouture égale sépareraient
l'eau du réservoir de la durée du 0 % ; mesurer la température du réservoir
au moment de l'infusion suffirait aussi.

### 9 octobre, 7 h 01 — `261009-070144`

**Contexte.** Firmware, loi de chauffe et réglages d'infusion inchangés
(`configs/261005-075907.json`). Mouture grossie d'un pas depuis le 8 octobre
à 13 h 44, seule variable. Pas de SCACE dans la capture. Observation de
David : on voit que la mouture est plus grossière. Pas de remarque sur le
goût ni sur le filet à 22 g.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,04 s | 8,05 s | — | — |
| Remplissage | 8,09 s | 6,00 s | 20,6 ml (3,43 ml/s) | 0,38 → 0,89 bar |
| Pré-infusion | 14,09 s | 3,10 s | 6,7 ml (2,15 ml/s) | 0,89 → 1,07 bar |
| Infusion | 17,19 s | 18,00 s | 40,6 ml | 9,09 bar max |

- Pression au repos avant l'infusion : 0,31 bar piégés.
- Premier paquet au-dessus de 0,8 bar à 13,89 s (20,0 ml, 5,8 s après le
  début du remplissage), passage en pré-infusion à 14,09 s : 200 ms.
- Le firmware a vu la première goutte à 17,19 s et 27,2 ml, 0,9 s avant
  l'échéance ; le rapport la place à 29,3 ml (≥ 0,3 g), 0,8 s plus tard.
  Marge de remplissage : 6,6 à 8,7 ml selon la définition (6,7 ml la
  veille à 13 h 44).
- Montée de 4,4 s jusqu'à 8 bar, avec 4,0 g en tasse (3,8 g la veille).
  Pompe à 100 % de 19,8 à 21,3 s. Débit en tasse sur 1 s au plus de
  1,7 g/s, à 8 bar ; même fenêtre : 1,4 g/s la veille à 13 h 44, 1,5 g/s
  le 8 octobre au matin, 1,9 g/s le 6 octobre.
- Depuis 8 bar : 13,7 s, 1,27 g/s en tasse, 1,93 ml/s en amont, 21,6 g à
  l'arrêt (au poids), 22,2 g final.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,89 ± 0,08 bar, pompe à
  67 %, 1,26 g/s, soit **7,1 bar·s/g** (7,5 la veille, 6,3 le 6 octobre).
- Après l'arrêt, la pression piégée tombe à 7,21 bar puis remonte à
  9,36 bar en 4 s, comme les deux infusions précédentes.

**Thermique.**

| Grandeur | Mesure |
| --- | ---: |
| NTC pondérée par la tasse | **89,26 °C** (−0,74 °C) |
| NTC pondérée par le volume | 90,02 °C |
| Pic avant l'infusion | 92,12 °C, à 13,29 s (fin du remplissage) |
| Minimum pendant l'écoulement | 87,50 °C, à l'arrêt de la pompe |
| Minimum total | 86,55 °C à 40,2 s, ≈ 4,9 s après l'arrêt (3,38 °C sous la NTC de départ, 89,93 °C) |
| 20 s après l'arrêt (fin de la capture) | 88,93 °C, en hausse |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 8,64 kJ | 9,00 kJ |
| Remplissage | 5,49 kJ | 5,16 kJ |
| Pré-infusion | 2,51 kJ | 2,52 kJ |
| Infusion | 4,18 kJ | 3,60 kJ |
| Récupération | 0,81 kJ | 1,20 kJ |

La commande reste à 0 % de 17,29 à 28,89 s (**11,6 s**), puis suit le
débit à 44–63 % pendant les 6,4 dernières secondes. 13,5 des 21,6 g en
tasse arrivent pendant ce 0 %. De la précharge à l'arrêt : 20,8 kJ
commandés, 20,3 kJ fournis par le SSR, pour 67,9 ml admis, soit
0,299 kJ/ml fournis (0,289 la veille à 13 h 44). Le SSR suit la commande
de plus près que d'habitude (−0,5 kJ contre −1,1 kJ) : en précharge, il
fournit 0,36 kJ de plus que commandé.

**Attendus vérifiés** (entrée du 8 octobre, 13 h 44) :

| Attendu | Mesure |
| --- | --- |
| Remplissage : fin à 0,8 bar, 19 à 22 ml, ≈ 6 s | 20,6 ml, 6,0 s |
| Pré-infusion : goutte en 2,1 à 3,4 s, 4 à 7 ml | 3,1 s, 6,7 ml |
| Plateau : 6,4 à 7,0 bar·s/g, pompe à 68–70 % | **non** : 7,1 bar·s/g, pompe à 67 % ; juste au-dessus |
| Depuis 8 bar : 12,5 à 14,5 s, 1,2 à 1,35 g/s | 13,7 s, 1,27 g/s |
| Commande à 0 % : 12 à 13,5 s | **non** : 11,6 s |
| Moyenne en tasse : 88,5 à 89,3 °C ; plus froide si l'eau du réservoir compte le matin | 89,26 °C, dans la fourchette, sans froid du matin |
| Modèle, commande réelle : écart de −0,9 à −1,4 °C | **non** : −0,05 °C |
| Filet à 22 g moins blond | non observé |

**Lecture.**

- *Mouture, mesuré.* Le pas a fait baisser la résistance au plateau de 7,5
  à 7,1 bar·s/g, deux fois moins que le pas de midi la veille (8,3 → 7,5).
  Rien ne dit si le pas était plus petit ou si la préparation a joué : à
  mouture égale, la résistance a déjà varié de 8 % (5 et 6 octobre). La
  galette reste entre le 6 octobre (trop grossière) et la veille à midi.
- *Écart au modèle, en simulation.* Rejouée, la moyenne en tasse vaut
  89,32 °C avec la commande et 89,35 °C avec l'énergie SSR : mesure −0,05
  et −0,08 °C. Le minimum mesuré est 1,0 °C sous le modèle (2,9 °C la
  veille à 13 h 44), le pic 1,3 °C au-dessus. Avec l'énergie SSR, l'écart en
  tasse des infusions précédentes vaut −0,20 °C (6 octobre), −1,69 °C
  (8 octobre, 8 h 43) et −0,79 °C (13 h 44).

  | Infusion | 0 % pendant | Tasse sous 0 % | Mesure − modèle (commande) | Mesure − modèle (SSR) |
  | --- | ---: | ---: | ---: | ---: |
  | 06/10 7 h 19 | 11,5 s | 14,6 g | −0,68 °C | −0,20 °C |
  | 09/10 7 h 01 | 11,6 s | 13,5 g | −0,05 °C | −0,08 °C |
  | 08/10 13 h 44 | 14,0 s | 15,6 g | −1,33 °C | −0,79 °C |
  | 08/10 8 h 43 | 15,4 s | 15,8 g | −2,14 °C | −1,69 °C |

  La durée du 0 % n'explique pas tout : à 11,6 s contre 11,5 s, l'écart à
  la commande passe de −0,68 à −0,05 °C. Avec l'énergie SSR, l'ordre se
  tient, et la part du SSR en retrait sort de l'écart (≈ 0,5 °C le
  6 octobre, rien ici). Ce qui reste suit encore la durée du 0 %, sur
  quatre points.
- *Eau du réservoir, mesuré.* Infusion du matin et tasse la plus proche du
  modèle de la série : l'eau froide du matin ne suffit pas à expliquer
  l'écart du 6 et du 8 octobre au matin. L'hypothèse de la veille (eau plus
  chaude à 13 h 44) perd son appui ; la température du réservoir n'est
  toujours pas mesurée.
- *Tasse, mesuré.* La plus proche de la consigne depuis le remboursement
  (−0,74 °C) : 0 % plus court, appoint repris 6,4 s avant l'arrêt (≈ 5 s
  les deux infusions précédentes), SSR qui a suivi la commande. Ces trois
  effets ne se séparent pas sur cette seule capture.
- *Critère.* Tout ceci reste la NTC. Qu'une moyenne en tasse de 89,3 °C à
  la NTC soit ≈ 89 °C dans le panier n'est pas vérifié : l'offset de
  −10 °C (voir [chauffe-infusion.md](chauffe-infusion.md#objectif)) vient
  d'essais au repos.

**Décision.** David passe à la sonde SCACE pour vérifier ce que « consigne
90 °C » donne réellement dans le panier. Réglages inchangés
(`configs/261005-075907.json`), loi de chauffe inchangée : la mesure au
panier décide avant tout changement de consigne ou de remboursement.

**Attendus pour l'infusion suivante** (SCACE, même configuration) :

| Point | Attendu |
| --- | --- |
| Préparation | `uv run firmware/tools/set_scace.py true` avant ; `scace_temperature_c` non nul pendant toute l'infusion |
| Hydraulique | panier de simulation, trou de 0,3 mm : sans galette, ni le remplissage ni la goutte ne se comparent aux infusions ci-dessus ; débit à 9 bar estimé à 1,5 à 2 ml/s (orifice, coefficient 0,6), à relever |
| Comparaison | SCACE et NTC sur la même capture : moyenne pondérée par la tasse des deux, écart au plateau, retard de la NTC sur la SCACE au creux |
| Consigne | si la moyenne SCACE en tasse tombe à 90 ± 0,5 °C, l'offset de −10 °C tient pendant l'écoulement et la NTC reste un critère utilisable ; sinon, l'écart chiffre la correction de consigne |
| Modèle, commande réelle | noter l'écart à la NTC, pour un cinquième point du tableau ci-dessus |

Prolonger la capture à 40–60 s après l'arrêt : la SCACE montrera si l'eau au
panier suit le creux de la NTC après l'arrêt.

### 10 octobre — offset de −14 °C (écran 0.4.4)

**Contexte.** Pas d'infusion de café : trois infusions avec le panier de
simulation et la sonde SCACE, détaillées dans
[sonde-scace.md](sonde-scace.md#infusions-10-octobre-2026).

**Mesure.** Moyennée par la tasse, l'eau au panier était 3,4 à 9,9 K sous
la NTC utilisateur de l'ancien offset (−10,5 °C). L'écart tend vers 3 K
quand le porte-filtre part chaud, ce qui est le cas en usage.

**Décision.** Offset porté à **−14 °C** (écran 0.4.4) : une consigne de
90 °C donne 104 °C à la sonde au lieu de 100,5 °C. La consigne est bornée à
95 °C et la coupure de sécurité passe à 100 °C utilisateur, soit 114 °C à la
sonde (115,5 °C avant). Consigne en NVS inchangée : 90 °C.

**Lecture des entrées précédentes.** Les colonnes « NTC tasse » jusqu'au
9 octobre sont dans l'ancien domaine : retirer 3,5 °C pour les comparer
aux infusions à partir de 0.4.4. Les infusions notées « un peu trop fine »,
au goût un peu sous-extrait, avaient donc environ 84 à 86 °C au panier pour
une consigne de 90 °C.

**Attendus pour l'infusion suivante** (café, écran 0.4.4, réglages
inchangés) :

| Point | Attendu |
| --- | --- |
| Tasse | NTC en tasse vers 90 °C dans le nouveau domaine, soit une chaudière 3,5 °C plus chaude qu'au 9 octobre |
| Goût | moins sous-extrait à mouture égale ; ne corriger la mouture qu'après |
| Pression | la chaudière plus chaude monte plus haut après l'arrêt de la pompe ; noter le maximum |

### 10 octobre, 11 h 01 — `261010-110108`

**Contexte.** Première infusion de café avec l'écran 0.4.4 (offset de
−14 °C) : consigne de 90 °C, soit 104 °C à la sonde au lieu de 100,5 °C.
Réglages d'infusion inchangés (`configs/261005-075907.json`), mouture du
9 octobre. Observation de David : débit un peu trop élevé, peut-être lié à
la nouvelle température. Goût bon ; la mouture plus fine devrait encore
aider.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,14 s | 7,89 s | — | — |
| Remplissage | 8,03 s | 8,30 s | 24,9 ml (3,04 ml/s) | 0,39 → 0,83 bar |
| Pré-infusion | 16,32 s | 0,84 s | 1,4 ml | 0,91 → 0,94 bar |
| Infusion | 17,16 s | 14,35 s | 34,5 ml | 8,98 bar max |

- Remplissage plus long que le 9 octobre (8,30 s et 24,9 ml contre 6,00 s
  et 20,6 ml) : la pression monte plus lentement.
- Première goutte à 27,2 ml (rapport, ≥ 0,3 g), 2,3 ml après la fin du
  remplissage (6,6 à 8,7 ml le 9 octobre) ; la pré-infusion s'arrête à la
  goutte après 0,84 s.
- Montée de 5,4 s jusqu'à 8 bar (4,4 s le 9 octobre).
- Depuis 8 bar : **9,1 s**, 1,52 g/s en tasse, 1,98 ml/s en amont (13,7 s,
  1,27 g/s et 1,93 ml/s le 9 octobre). Arrêt au poids à 21,7 g, 22,8 g
  final.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,87 ± 0,06 bar, pompe à
  70 %, 1,50 g/s, soit **5,9 bar·s/g** (7,1 le 9 octobre).
- Après l'arrêt, la pression remonte à 9,38 bar (9,36 le 9 octobre).

**Thermique.** Ce jour et le 9 octobre, dans leurs domaines respectifs :

| Repère | 10/10 11 h 01 (−14 °C) | 09/10 7 h 01 (−10,5 °C) |
| --- | ---: | ---: |
| NTC au départ | 89,87 °C | 89,93 °C |
| Pic avant l'infusion | 91,42 °C à 13,3 s | 92,12 °C à 13,3 s |
| **Tasse** | **87,11 °C**, −2,89 °C | 89,26 °C, −0,74 °C |
| Minimum pendant l'écoulement | 85,98 °C, à l'arrêt | 87,56 °C, à l'arrêt |
| Minimum total | 84,77 °C à 36,5 s | 86,55 °C à 40,2 s |
| 20 s après l'arrêt | 86,58 °C | 88,93 °C |
| Commande à 0 % (remboursement) | 11,4 s | 11,6 s |
| Appoint repris avant l'arrêt | **2,9 s** | 6,3 s |

- Énergie, commande / SSR : précharge 8,52 / 8,48 kJ, remplissage 6,88 /
  6,13 kJ, pré-infusion 0,79 / 0,74 kJ, infusion 1,89 / 2,15 kJ.
- Modèle, commande réelle : 89,39 °C en tasse, soit +2,26 °C sur la mesure
  (+2,14 le 8 octobre à 8 h 43, +0,05 le 9 octobre).

**Attendus vérifiés** (entrée « offset de −14 °C ») :

| Point | Attendu | Mesure |
| --- | --- | --- |
| Tasse | vers 90 °C dans le nouveau domaine | **87,11 °C : non atteint.** Ramené au domaine de l'offset, +1,35 °C au panier par rapport au 9 octobre (85,76 °C), au lieu des +3,5 °C du changement d'offset |
| Goût | moins sous-extrait à mouture égale | **bon** ; débit plus rapide, mouture à affiner |
| Pression | maximum plus haut après l'arrêt | 9,38 bar contre 9,36 : pas de différence |

**Lecture.**

- *Tasse, mesuré.* L'infusion la plus rapide depuis le remboursement : le
  remboursement de la précharge tient la commande à 0 % pendant 11,4 s,
  presque toute l'infusion, et l'appoint ne reprend que 2,9 s avant
  l'arrêt. La chaleur de l'écoulement arrive après l'arrêt : minimum à
  84,77 °C 5 s après, puis remontée. C'est le mécanisme du 8 octobre à
  8 h 43 (0 % pendant 15,4 s, −2,31 °C). La chaudière plus chaude n'a donc
  apporté que +1,35 °C au panier sur les +3,5 °C attendus.
- *Débit, estimé.* La résistance au plateau baisse de 17 % (5,9 contre
  7,1 bar·s/g). L'eau 3,5 °C plus chaude est moins visqueuse d'environ 6 %
  (≈ 1,7 % par kelvin vers 90 °C) : un tiers de l'écart au plus. Le reste
  relève de la mouture, de la dose ou du tassage, que la capture ne sépare
  pas. Remplissage plus long et goutte précoce vont dans le même sens : une
  galette moins résistante.
- *Loi, estimé.* La pente de l'appoint suppose 70,5 K d'élévation ; avec
  la chaudière 3,5 °C plus chaude, le besoin réel est d'environ 5 % plus
  fort. L'effet est faible devant les 11,4 s à 0 %.
- *Modèle.* Il retrouve l'écart d'environ +2 °C des infusions courtes avec
  long 0 % (8 octobre au matin) ; le facteur manquant n'est toujours pas
  identifié.

**Décision.** David affine la mouture. Firmware 0.4.4, loi et réglages
d'infusion inchangés.

**Attendus pour l'infusion suivante** (mouture plus fine, même
configuration) :

| Point | Attendu |
| --- | --- |
| Hydraulique | plateau vers 7 bar·s/g, 12 à 14 s depuis 8 bar, remplissage vers 6 s et 20 ml, première goutte 6 à 8 ml après le remplissage |
| Chauffe | appoint repris 5 s ou plus avant l'arrêt |
| Tasse | 88 à 89 °C (−1 à −2 °C), comme le 9 octobre ramené à la consigne |
| Goût | relever si le sous-extrait du 8 et du 9 octobre a disparu |


### 10 octobre, 13 h 22 — `261010-132259`

**Contexte.** Mouture affinée après l'infusion de 11 h 01. Écran 0.4.4
(offset de −14 °C), réglages d'infusion inchangés
(`configs/261005-075907.json`). Observation de David : le goût est au
rendez-vous, le débit semble bien.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,14 s | 7,89 s | — | — |
| Remplissage | 8,02 s | 8,28 s | 24,9 ml (3,01 ml/s) | 0,33 → 0,87 bar |
| Pré-infusion | 16,31 s | 3,65 s | 7,5 ml | 0,87 → 1,23 bar |
| Infusion | 19,95 s | 17,61 s | 35,4 ml | 9,33 bar max |

- Remplissage identique à 11 h 01 (8,28 s et 24,9 ml) et toujours plus long
  qu'avant : 3,01 ml/s à 75 % contre 3,2 à 3,5 ml/s du 5 au 9 octobre, sous
  0,9 bar donc sans effet de la galette.
- Première goutte à 32,8 ml (rapport, ≥ 0,3 g), 7,9 ml après la fin du
  remplissage (2,3 ml à 11 h 01, 8,7 ml le 9 octobre) ; la pré-infusion
  s'arrête à la goutte après 3,65 s.
- Montée de 4,0 s jusqu'à 8 bar (5,4 s à 11 h 01, 4,4 s le 9 octobre).
- Depuis 8 bar : **13,7 s**, 1,27 g/s en tasse, 1,69 ml/s en amont (9,1 s,
  1,52 g/s et 1,98 ml/s à 11 h 01 ; 13,7 s, 1,27 g/s et 1,93 ml/s le
  9 octobre). Arrêt au poids à 21,6 g, 22,4 g final.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,91 ± 0,08 bar, pompe à
  67 %, 1,30 g/s, soit **6,8 bar·s/g** (5,9 à 11 h 01, 7,1 le 9 octobre).
- Après l'arrêt, la pression remonte à 9,59 bar (9,38 à 11 h 01).

**Thermique.** Même domaine (−14 °C) que 11 h 01 :

| Repère | 10/10 13 h 22 | 10/10 11 h 01 |
| --- | ---: | ---: |
| NTC au départ | 89,44 °C | 89,87 °C |
| Pic avant l'infusion | 90,97 °C à 11,5 s | 91,42 °C à 13,3 s |
| **Tasse** | **88,53 °C**, −1,47 °C | 87,11 °C, −2,89 °C |
| Minimum pendant l'écoulement | 86,56 °C, à l'arrêt | 85,98 °C, à l'arrêt |
| Minimum total | 85,49 °C à 43,5 s | 84,77 °C à 36,5 s |
| 20 s après l'arrêt | 88,31 °C | 86,58 °C |
| Commande à 0 % (remboursement) | 11,9 s | 11,4 s |
| Appoint repris avant l'arrêt | **5,7 s** | 2,9 s |

- Énergie, commande / SSR : précharge 8,41 / 8,67 kJ, remplissage 6,96 /
  6,17 kJ, pré-infusion 2,74 / 2,51 kJ, infusion 3,02 / 3,16 kJ.
- Modèle, commande réelle : 89,29 °C en tasse, soit +0,72 °C sur la mesure
  (+2,26 à 11 h 01, +0,05 le 9 octobre).

**Attendus vérifiés** (entrée de 11 h 01) :

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | plateau vers 7 bar·s/g, 12 à 14 s depuis 8 bar | **6,8 bar·s/g, 13,7 s : atteint** |
| | remplissage vers 6 s et 20 ml | **8,28 s et 24,9 ml : non atteint**, comme à 11 h 01 |
| | première goutte 6 à 8 ml après le remplissage | 7,9 ml : atteint |
| Chauffe | appoint repris 5 s ou plus avant l'arrêt | 5,7 s : atteint |
| Tasse | 88 à 89 °C | 88,53 °C : atteint |
| Goût | relever si le sous-extrait a disparu | « au rendez-vous » |

**Lecture.**

- *Tasse, mesuré.* À durée depuis 8 bar égale (13,7 s), l'infusion
  reproduit celle du 9 octobre : appoint repris 5,7 s avant l'arrêt
  (6,3 s), écart au critère de −1,47 °C (−0,74 °C). Ramenée au domaine de
  l'offset, l'eau au panier gagne **+2,77 °C** sur le 9 octobre (85,76 °C),
  pour les +3,5 °C du changement d'offset. Le gain manquant de 11 h 01
  venait donc bien surtout de la durée d'infusion trop courte.
- *Débit, mesuré.* La mouture plus fine remonte la résistance de 5,9 à
  6,8 bar·s/g, juste sous le 9 octobre (7,1) ; l'écart de 0,3 est de
  l'ordre de l'effet de viscosité estimé à 11 h 01 (≈ 6 %).
- *Remplissage, mesuré.* Débit médian sous 1 bar, pompe à 75 % :
  3,46 à 3,65 ml/s du 1er octobre aux infusions SCACE de 10 h 34, puis
  **3,09 ml/s** à 11 h 01 et à 13 h 22, à pression plus basse (0,3 bar
  contre 0,4 à 0,55) : la pompe débite moins, la cause est en amont du
  groupe. Les purges à 100 % de la veille au soir et du matin (3,95 à
  4,11 ml/s) étaient déjà faites avec le filtre ECM Aqua Aroma Crema dans
  le réservoir. Le même jour, la machine a été fermée (panneaux posés) et le
  réservoir vidé puis rempli. La cause n'est pas isolée et ne sera pas
  recherchée : la machine est dans son état final, 3,1 ml/s à 75 % devient
  la référence du remplissage. Les 2,3 s de remplissage en plus n'ont pas
  d'effet visible sur la tasse.
- *Modèle.* L'écart revient à +0,7 °C, entre le 9 octobre (+0,05) et les
  infusions courtes à long 0 % (+2,1 à +2,3).

**Décision.** Mouture, firmware 0.4.4 et réglages d'infusion conservés.

**Attendus pour l'infusion suivante** (même configuration) :

| Point | Attendu |
| --- | --- |
| Hydraulique | répétabilité : 6,5 à 7,2 bar·s/g, 12 à 15 s depuis 8 bar |
| Remplissage | nouvelle référence : vers 3,1 ml/s à 75 %, 8,3 s et 25 ml ; suivre la dérive sur plusieurs infusions |
| Tasse | 88 à 89 °C, appoint repris 5 s ou plus avant l'arrêt |
| Goût | confirmer « au rendez-vous » |


### 11 octobre, 9 h 47 — `261011-094749`

**Contexte.** Même configuration qu'à 13 h 22 la veille : mouture, écran
0.4.4 (offset de −14 °C) et réglages d'infusion inchangés. Observation de
David : satisfait du résultat, l'offset de −14 °C est une amélioration ;
une faible amertume reste.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,14 s | 7,91 s | — | — |
| Remplissage | 8,05 s | 6,67 s | 23,5 ml (3,52 ml/s) | 0,42 → 0,90 bar |
| Pré-infusion | 14,72 s | 3,56 s | 6,7 ml | 0,90 → 1,33 bar |
| Infusion | 18,29 s | 16,96 s | 40,9 ml | 9,28 bar max |

- Remplissage plus court que la veille (6,67 s et 23,5 ml contre 8,28 s et
  24,9 ml) : débit médian sous 1 bar de **3,59 ml/s** à 75 %, à 0,35 bar,
  revenu dans la plage du 1er au 9 octobre (3,46 à 3,65 ml/s).
- Première goutte à 30,1 ml (rapport, ≥ 0,3 g), 6,6 ml après la fin du
  remplissage (7,9 ml la veille) ; la pré-infusion s'arrête à la goutte
  après 3,56 s.
- Montée de 3,7 s jusqu'à 8 bar (4,0 s la veille).
- Depuis 8 bar : **13,3 s**, 1,32 g/s en tasse, 2,23 ml/s en amont
  (13,7 s, 1,27 g/s et 1,69 ml/s la veille). Arrêt au poids à 21,7 g,
  22,5 g final.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,99 ± 0,05 bar, pompe à
  68 %, 1,28 g/s, soit **7,0 bar·s/g** (6,8 la veille).
- Sur la phase d'infusion, la tasse reçoit 0,52 g par ml admis (0,59 la
  veille, 0,62 à 11 h 01 le 10 octobre). Après l'arrêt, la pression
  remonte à 9,30 bar (9,59 la veille).

**Thermique.**

| Repère | 11/10 9 h 47 | 10/10 13 h 22 |
| --- | ---: | ---: |
| NTC au départ | 89,60 °C | 89,44 °C |
| Pic avant l'infusion | 90,84 °C à 11,3 s | 90,97 °C à 11,5 s |
| **Tasse** | **87,38 °C**, −2,62 °C | 88,53 °C, −1,47 °C |
| Minimum pendant l'écoulement | 85,75 °C, à l'arrêt | 86,56 °C, à l'arrêt |
| Minimum total | 84,61 °C à 40,1 s | 85,49 °C à 43,5 s |
| 20 s après l'arrêt | 88,36 °C | 88,31 °C |
| Commande à 0 % (remboursement) | 11,1 s | 11,9 s |
| Appoint repris avant l'arrêt | 5,7 s | 5,7 s |

- Énergie, commande / SSR : précharge 8,55 / 8,38 kJ, remplissage 6,20 /
  5,05 kJ, pré-infusion 2,57 / 2,33 kJ, infusion 4,20 / 4,39 kJ. Total
  fourni par le SSR : 20,15 kJ pour 70,7 ml admis à l'arrêt (20,51 kJ pour
  67,8 ml la veille).
- Trou de commande au début du remplissage : 6 % de 8,3 à 9,1 s, et le SSR
  reste coupé ≈ 1,8 s en attendant la fenêtre suivante.
- Modèle : 89,20 °C en tasse avec la commande réelle, soit +1,82 °C sur la
  mesure (+0,72 la veille) ; 88,38 °C avec l'énergie SSR, +0,98 °C (+0,39
  la veille).

**Attendus vérifiés** (entrée du 10 octobre, 13 h 22) :

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 6,5 à 7,2 bar·s/g, 12 à 15 s depuis 8 bar | **7,0 bar·s/g, 13,3 s : atteint** |
| Remplissage | vers 3,1 ml/s à 75 %, 8,3 s et 25 ml | **3,59 ml/s, 6,67 s et 23,5 ml : non atteint**, retour aux valeurs d'avant le 10 octobre |
| Tasse | 88 à 89 °C | **87,38 °C : non atteint** |
| | appoint repris 5 s ou plus avant l'arrêt | 5,7 s : atteint |
| Goût | confirmer « au rendez-vous » | satisfait, faible amertume |

**Lecture.**

- *Débit, mesuré.* Même mouture, même résistance : 7,0 bar·s/g contre 6,8
  la veille, 13,3 s contre 13,7 s depuis 8 bar. La mouture est répétable.
- *Remplissage, mesuré.* La baisse à 3,1 ml/s du 10 octobre n'a duré
  qu'une journée ; la pompe débite de nouveau 3,6 ml/s sous 1 bar. La
  cause de l'écart n'est toujours pas isolée. Le débit amont en infusion
  monte aussi (2,23 ml/s pour la même tasse), d'où 0,52 g par ml : le
  débitmètre, en amont de la pompe, voit plus d'eau sans que la tasse en
  reçoive plus. Rien dans la capture ne dit où passe la différence.
- *Tasse, mesuré et simulé.* À durée et appoint égaux, la tasse perd
  1,15 °C sur la veille. Le modèle avec l'énergie SSR en retrouve
  0,58 °C en simulation (88,38 contre 88,96 °C) : remplissage plus court
  (5,05 kJ fournis contre 6,17), trou de commande au début du remplissage,
  2,9 ml de plus admis. Les 0,6 °C restants ne sont pas expliqués ; l'écart
  mesure − modèle varie d'autant d'une infusion à l'autre.
- *Goût.* La tasse plus froide d'environ 1 °C n'a pas dégradé le résultat
  selon David.

**Décision.** Firmware 0.4.4 et chauffe conservés. David passe à la
recette : poids cible à **23 g**, voire 24 g, et mouture affinée d'un
cran, pour réduire la faible amertume. Les deux réglages changent ensemble.
La résistance au plateau (bar·s/g) ne dépend que de la mouture et sépare
son effet hydraulique ; rien ne sépare leurs effets sur le goût.

**Attendus pour l'infusion suivante** (mouture d'un cran plus fine,
cible 23 g) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,4 à 8,0 bar·s/g ; 15 à 17 s depuis 8 bar (16 à 18 s à 24 g) |
| Rendement | ≈ 23,5 g final (≈ 24,5 g à 24 g) |
| Remplissage | 3,5 à 3,6 ml/s à 75 %, 6,5 à 7 s et 23 à 24 ml ; noter si la baisse du 10 octobre revient |
| Tasse | 87,5 à 88,5 °C, appoint repris 7 s ou plus avant l'arrêt |
| Goût | amertume relevée |


### 11 octobre, 13 h 40 — `261011-134059`

**Contexte.** Première infusion avec l'arrêt au poids à **23 g** (22 g
avant). Mouture inchangée depuis le 10 octobre à 13 h 22 : l'affinage
prévu n'a pas été fait. Écran 0.4.4, réglages de chauffe et de remplissage
inchangés. Observations de David : débit un peu élevé, le tassage est à
travailler ; moins d'amertume.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,14 s | 7,93 s | — | — |
| Remplissage | 8,07 s | 6,07 s | 21,2 ml (3,49 ml/s) | 0,40 → 0,85 bar |
| Pré-infusion | 14,14 s | 2,34 s | 4,6 ml | 0,85 → 1,08 bar |
| Infusion | 16,47 s | 16,44 s | 38,6 ml | 9,00 bar max |

- Remplissage : débit médian sous 1 bar de **3,59 ml/s** à 75 %, comme le
  matin, mais fin à 0,8 bar après 21,2 ml au lieu de 23,5 ml (6,07 s
  contre 6,67 s).
- Première goutte à 25,8 ml (rapport, ≥ 0,3 g), 4,6 ml après la fin du
  remplissage (6,6 ml le matin) ; la pré-infusion s'arrête à la goutte
  après 2,34 s.
- Montée de 4,6 s jusqu'à 8 bar (3,7 s le matin).
- Depuis 8 bar : **11,9 s**, 1,39 g/s en tasse, 1,98 ml/s en amont
  (13,3 s, 1,32 g/s et 2,23 ml/s le matin). Arrêt au poids à 23,0 g,
  23,7 g final : 0,7 g de gouttes (0,8 g le matin).
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,85 ± 0,08 bar, pompe à
  68 %, 1,44 g/s, soit **6,2 bar·s/g** (7,0 le matin).
- Sur la phase d'infusion, la tasse reçoit 0,58 g par ml admis (0,52 le
  matin). Après l'arrêt, la pression remonte à 9,37 bar.

**Thermique.**

| Repère | 11/10 13 h 40 | 11/10 9 h 47 |
| --- | ---: | ---: |
| NTC au départ | 89,64 °C | 89,60 °C |
| Pic avant l'infusion | 91,58 °C à 13,9 s | 90,84 °C à 11,3 s |
| **Tasse** | **87,29 °C**, −2,71 °C | 87,38 °C, −2,62 °C |
| Minimum pendant l'écoulement | 85,24 °C, à l'arrêt | 85,75 °C, à l'arrêt |
| Minimum total | 84,28 °C à 37,9 s | 84,61 °C à 40,1 s |
| 20 s après l'arrêt | 87,67 °C | 88,36 °C |
| Commande à 0 % (remboursement) | 10,6 s | 11,1 s |
| Appoint repris avant l'arrêt | 5,9 s | 5,7 s |

- Énergie, commande / SSR : précharge 8,45 / 8,28 kJ, remplissage 5,42 /
  4,72 kJ, pré-infusion 1,95 / 1,71 kJ, infusion 3,59 / 3,53 kJ. Total
  fourni par le SSR : 18,24 kJ pour 64,6 ml admis à l'arrêt (20,15 kJ pour
  70,7 ml le matin).
- Trou de commande au début du remplissage : 6 % de 8,4 à 9,4 s, et le SSR
  reste coupé ≈ 1,8 s, comme le matin.
- Modèle : 88,81 °C en tasse avec la commande réelle, soit +1,52 °C sur la
  mesure (+1,82 le matin) ; 88,39 °C avec l'énergie SSR, +1,10 °C (+0,98
  le matin).

**Attendus vérifiés** (entrée du 11 octobre, 9 h 47, prévus pour une
mouture affinée qui n'a pas été faite) :

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,4 à 8,0 bar·s/g ; 15 à 17 s depuis 8 bar | **6,2 bar·s/g, 11,9 s : non atteint**, mouture inchangée et résistance en baisse |
| Rendement | ≈ 23,5 g final | 23,7 g : atteint |
| Remplissage | 3,5 à 3,6 ml/s à 75 %, 6,5 à 7 s et 23 à 24 ml | 3,59 ml/s : atteint ; **6,07 s et 21,2 ml : non atteint** |
| Tasse | 87,5 à 88,5 °C | **87,29 °C : non atteint** |
| | appoint repris 7 s ou plus avant l'arrêt | **5,9 s : non atteint** |
| Goût | amertume relevée | moins d'amertume |

**Lecture.**

- *Débit, mesuré.* À mouture égale, la résistance au plateau passe de 7,0
  à 6,2 bar·s/g (−11 %) : c'est la plus faible depuis le 10 octobre à
  11 h 01 (5,9). Le temps depuis 8 bar baisse de 1,4 s alors que la tasse
  reçoit 1,3 g de plus.
- *Tassage, estimé.* À débit de pompe égal (3,59 ml/s), le remplissage
  atteint 0,8 bar 2,3 ml plus tôt et la première goutte vient 2,0 ml plus
  tôt après lui. Moins d'espace au-dessus de la galette et une galette
  plus perméable vont avec un tassage plus léger, qui laisse la galette
  plus haute. Hypothèse non testée : une dose plus forte réduirait aussi
  l'espace, mais ralentirait le débit.
- *Tasse, mesuré et simulé.* Moyenne en tasse égale à celle du matin
  (−0,09 °C). Le modèle avec l'énergie SSR donne la même tasse aux deux
  infusions (88,39 et 88,38 °C) : le remplissage plus court (1,9 kJ de
  moins fournis) compense l'eau en moins. L'appoint reprend 5,9 s avant
  l'arrêt : l'attendu de 7 s supposait 15 à 17 s depuis 8 bar.
- *Goût.* L'amertume baisse, mais le poids (+1 g) et la résistance (−11 %)
  ont changé ensemble ; rien ne sépare leurs effets sur le goût.

**Décision.** Firmware 0.4.4, chauffe, mouture et cible de 23 g
conservées. David travaille le tassage pour retrouver la résistance du
matin.

**Attendus pour l'infusion suivante** (même mouture, cible 23 g, tassage
corrigé) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 6,7 à 7,2 bar·s/g ; 13,5 à 15,5 s depuis 8 bar |
| Rendement | ≈ 23,7 g final |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 6,5 à 7 s et 23 à 24 ml si le tassage est la cause de l'écart du jour |
| Tasse | 87 à 88 °C, appoint repris 6,5 s ou plus avant l'arrêt |
| Goût | amertume au moins aussi basse qu'aujourd'hui ; si elle remonte avec la résistance, c'est le débit qui l'avait réduite, pas le poids |


### 12 octobre, 9 h 12 — `261012-091234`

**Contexte.** Mouture affinée. Cible de 23 g, écran 0.4.4, réglages de
chauffe et de remplissage inchangés. Tassage manuel jugé bon, peut-être
un peu plus appuyé que la veille. Hygrométrie de 50 %, la même que la
veille ; elle baisse ces jours-ci (nuits froides, journées douces).
Observations de David : très bon résultat gustatif ; le café est apparu
sous le panier plus tard, et la première goutte est tombée pile à la fin
de la pré-infusion. Pour une fois, la pré-infusion s'est faite au bon
moment, quand les trous du panier se remplissent de café. Galette
parfaite à la sortie, sans aucune dégradation.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,14 s | 7,91 s | — | — |
| Remplissage | 8,04 s | 5,35 s | 18,6 ml (3,47 ml/s) | 0,71 → 0,89 bar |
| Pré-infusion | 13,39 s | 4,03 s | 8,7 ml | 0,89 → 1,46 bar |
| Infusion | 17,43 s | 21,89 s | 44,3 ml | 9,39 bar max |

- Pression avant la pompe : 0,58 à 0,66 bar (0,24 à 0,43 bar du 9 au
  11 octobre). Elle tombe dès le remplissage à 0,34 bar, comme la veille.
  Le remplissage part donc du même point bas.
- Remplissage : débit médian sous 1 bar de **3,59 ml/s** à 75 %. Fin à
  0,8 bar après 18,6 ml (21,2 ml la veille, 23,5 ml le 11 octobre au
  matin).
- Pré-infusion : la première goutte n'arrive pas dans les 4 s, et la phase
  s'arrête sur l'**échéance**, pour la première fois depuis le 4 octobre.
  La balance voit la goutte à 30,1 ml (rapport, ≥ 0,3 g), 1,0 s après la
  fin de la pré-infusion ; à l'œil, elle tombe à la fin de la
  pré-infusion. La marge de remplissage est de **11,5 ml** (4,6 ml la
  veille).
- Montée de 3,3 s jusqu'à 8 bar (4,6 s la veille).
- Depuis 8 bar : **18,7 s**, 1,04 g/s en tasse, 1,85 ml/s en amont (11,9 s,
  1,39 g/s et 1,98 ml/s la veille). Arrêt au poids à 22,8 g, 23,3 g final,
  soit 0,5 g de gouttes (23,0 g, 23,7 g et 0,7 g la veille).
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,98 ± 0,06 bar, pompe à
  66 %, 1,06 g/s, soit **8,5 bar·s/g** (6,2 la veille). C'est la plus forte
  résistance depuis l'offset de −14 °C.
- Sur la phase d'infusion, la tasse reçoit 0,51 g par ml admis (0,58 la
  veille). Après l'arrêt, la pression remonte à 9,34 bar.

**Thermique.**

| Repère | 12/10 9 h 12 | 11/10 13 h 40 |
| --- | ---: | ---: |
| NTC au départ | 89,77 °C | 89,64 °C |
| Pic avant l'infusion | 91,46 °C à 13,9 s | 91,58 °C à 13,9 s |
| Creux au début de l'infusion | 88,24 °C à 18,9 s | — |
| Rebond, commande à 0 % | 89,58 °C à 23,5 s | — |
| **Tasse** | **88,75 °C**, −1,25 °C | 87,29 °C, −2,71 °C |
| Minimum pendant l'écoulement | 87,22 °C, à l'arrêt | 85,24 °C, à l'arrêt |
| Minimum total | 86,78 °C à 43,7 s | 84,28 °C à 37,9 s |
| 20 s après l'arrêt | 89,49 °C | 87,67 °C |
| Commande à 0 % (remboursement) | 12,5 s | 10,6 s |
| Appoint repris avant l'arrêt | 9,5 s | 5,9 s |
| Pente de la NTC pendant l'appoint | −0,24 °C/s | −0,08 °C/s |

- Énergie, commande / SSR : précharge 8,40 / 8,49 kJ, remplissage 4,72 /
  4,07 kJ, pré-infusion 3,19 / 2,87 kJ, infusion 5,56 / 4,83 kJ. Total
  fourni par le SSR : 20,26 kJ pour 71,6 ml admis à l'arrêt (18,24 kJ pour
  64,6 ml la veille).
- Appoint, de sa reprise à l'arrêt : commande moyenne de 50 %, SSR actif
  44 % du temps. Le besoin calculé pour 1,88 ml/s est aussi de 50 %. La
  veille, l'appoint était de 51 % de commande et 50 % de SSR, pour un
  besoin de 51 %.
- La tasse reçoit 12,2 g avant la reprise de l'appoint, NTC moyenne
  89,2 °C, et 10,6 g après, NTC moyenne 88,3 °C.
- Trou de commande au début du remplissage : 5 % de 8,5 à 9,3 s ; le SSR
  reste coupé 2,0 s (1,8 s la veille).
- Modèle : 89,19 °C en tasse avec la commande réelle, soit +0,44 °C sur la
  mesure (+1,52 la veille) ; 88,81 °C avec l'énergie SSR, soit +0,06 °C
  (+1,10 la veille).

**Attendus vérifiés** (entrée du 11 octobre, 13 h 40, prévus pour la même
mouture avec un tassage corrigé ; la mouture a changé) :

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 6,7 à 7,2 bar·s/g ; 13,5 à 15,5 s depuis 8 bar | **8,5 bar·s/g, 18,7 s : dépassé**, mouture affinée |
| Rendement | ≈ 23,7 g final | **23,3 g : non atteint** (−0,4 g) |
| Remplissage | 3,5 à 3,6 ml/s à 75 % | 3,59 ml/s : atteint |
| | 6,5 à 7 s et 23 à 24 ml si le tassage expliquait l'écart | **5,35 s et 18,6 ml : non atteint**, plus court encore |
| Tasse | 87 à 88 °C | **88,75 °C : dépassé**, plus près de la consigne |
| | appoint repris 6,5 s ou plus avant l'arrêt | 9,5 s : atteint |
| Goût | amertume au moins aussi basse | très bon résultat |

**Lecture.**

- *Débit, mesuré.* La résistance au plateau passe de 6,2 à 8,5 bar·s/g
  (+37 %). Elle dépasse les 7,4 à 8,0 bar·s/g attendus le 11 octobre au
  matin pour un cran plus fin. Le tassage, peut-être plus appuyé, a pu
  changer avec la mouture, et rien ne sépare leurs effets. L'hygrométrie
  est la même que la veille (50 %) et n'explique pas l'écart entre ces
  deux infusions. Sa baisse pendant les jours précédents a pu jouer sur la
  série. La galette sort intacte, sans canal visible : le plateau régulier
  (± 0,06 bar) va dans ce sens.
- *Remplissage, estimé.* À débit de pompe égal (3,59 ml/s) et depuis le
  même point bas (0,34 bar), 0,8 bar arrive après 18,6 ml. Avec une galette
  plus fine, la pression monte plus tôt pour le même volume. Le remplissage
  laisse donc plus d'eau à la pré-infusion, qui finit à l'échéance avant la
  goutte. La pression résiduelle plus haute avant la pompe n'y est pour
  rien : elle tombe dès le début du remplissage.
- *Pré-infusion, observé et mesuré.* La goutte tombe à l'œil à la fin
  de la pré-infusion ; la balance la voit 1,0 s plus tard, sur un débit
  qui démarre. Les 4 s à 50 % ont donc rempli la galette jusqu'aux trous
  du panier sans écoulement. Aux infusions précédentes, la pré-infusion
  finissait sur une goutte déjà tombée, après 0,8 à 3,7 s. Hypothèse non
  testée : avec la mouture du jour, l'échéance de 4 s coïncide avec la
  saturation de la galette ; une mouture plus grosse la ferait de nouveau
  finir à la goutte.
- *Rendement, estimé.* Le seuil d'arrêt est le même. La balance dépasse
  moins ce seuil à 1,04 g/s qu'à 1,39 g/s, et le volume de gouttes baisse
  (0,5 g au lieu de 0,7 g). D'où 23,3 g au lieu de 23,7 g.
- *Chauffe, mesuré.* L'infusion longue montre en entier la réponse de la
  chaudière à la loi :
  1. Creux à 88,24 °C, dû à l'eau froide de la pré-infusion.
  2. Rebond à 89,58 °C à 23,5 s, alors que la commande est à 0 % depuis
     6 s : la chaleur de la précharge et du remplissage arrive avec le
     retard de la chaudière.
  3. Baisse régulière de 89,33 à 87,22 °C pendant les 9,5 s d'appoint.
     L'appoint couvre pourtant le besoin calculé pour l'eau admise.

  La veille, la même phase d'appoint ne baissait que de 0,08 °C/s, mais
  elle partait de 85,8 °C et ne durait que 5,9 s.
- *Baisse finale, simulé.* Le modèle rejoue la mesure à +0,06 °C avec
  l'énergie SSR, creux et rebond compris. La baisse de la fin s'explique
  ainsi en simulation : les 12,5 s à 0 % arrivent à la NTC avec ≈ 8,5 s de
  retard (constantes de chauffe du modèle), et tombent dans les dernières
  secondes de l'écoulement. L'appoint compense l'eau admise, pas ce déficit.
  Sur une infusion courte, ce déficit tombait après l'arrêt : le minimum de
  la veille vient 4,9 s après l'arrêt.
- *Tasse, mesuré.* 88,75 °C, la moyenne la plus proche de la consigne
  depuis l'offset de −14 °C (88,53 °C le 10 octobre à 13 h 22). Plus de la
  moitié du poids (12,2 g) arrive pendant le rebond, à 89,2 °C en moyenne.
- *Goût.* La mouture change le débit et la température en tasse
  (+1,46 °C) en même temps. Rien ne sépare leurs effets sur le goût.

**Décision.** Aucun réglage n'est changé : mouture du jour, cible de 23 g,
firmware 0.4.4 et chauffe conservés.

**Attendus pour l'infusion suivante** (mêmes réglages) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 8,0 à 9,0 bar·s/g ; 17 à 20 s depuis 8 bar |
| Rendement | ≈ 23,3 g final |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 6 s et 18 à 20 ml |
| Pré-infusion | fin à l'échéance de 4 s ; goutte visible à la fin de la pré-infusion, vue par la balance dans la seconde suivante |
| Tasse | 88,2 à 89,2 °C, appoint repris 8 s ou plus avant l'arrêt |
| Chauffe | rebond au-dessus de 89 °C avec la commande à 0 % ; NTC à l'arrêt ≈ 87,2 °C |
| Goût | même résultat ; s'il change, comparer d'abord la résistance au plateau |

### 12 octobre, 13 h 34 — `261012-133429`

**Contexte.** Mêmes réglages et même mouture qu'à 9 h 12, entre les deux
la série SCACE du matin. Rien n'a changé dans la préparation, mais le
tassage manuel peut varier un peu d'une infusion à l'autre. Observations
de David : goût équilibré, rien à redire, excellent. À nouveau, la
pré-infusion s'est faite quand le café commençait à être visible sous le
panier.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,14 s | 7,88 s | — | — |
| Remplissage | 8,01 s | 5,43 s | 18,6 ml (3,42 ml/s) | 0,52 → 0,86 bar |
| Pré-infusion | 13,44 s | 2,47 s | 6,4 ml | 0,86 → 1,10 bar |
| Infusion | 15,91 s | 18,55 s | 42,3 ml | 9,06 bar max |

- Pression avant la pompe : 0,45 à 0,51 bar (0,58 à 0,66 bar à 9 h 12).
  Elle tombe dès le remplissage à 0,35 bar ; le remplissage part du même
  point bas.
- Remplissage : débit médian sous 1 bar de **3,59 ml/s** à 75 %, comme à
  9 h 12. Fin à 0,8 bar après 18,6 ml, le même volume.
- Pré-infusion : finie sur la **première goutte** (firmware, +0,1 g) après
  2,47 s, et non plus sur l'échéance de 4 s. La balance voit la goutte
  0,6 s plus tard, à 25,5 ml. La marge de remplissage est de **6,9 ml**
  (11,5 ml à 9 h 12, 4,6 ml la veille).
- Montée de 4,3 s jusqu'à 8 bar (3,3 s à 9 h 12).
- Depuis 8 bar : **14,4 s**, 1,26 g/s en tasse, 1,99 ml/s en amont (18,7 s,
  1,04 g/s et 1,85 ml/s à 9 h 12). Arrêt au poids à 22,6 g, 23,3 g final,
  soit 0,7 g de gouttes (22,8 g, 23,3 g et 0,5 g à 9 h 12).
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,86 ± 0,09 bar, pompe à
  67 %, 1,27 g/s, soit **7,0 bar·s/g** (8,5 à 9 h 12, 7,0 le 11 octobre à
  9 h 47 avec la mouture plus grosse).
- Sur la phase d'infusion, la tasse reçoit 0,52 g par ml admis (0,51 à
  9 h 12). Après l'arrêt, la pression remonte à 9,43 bar.

**Thermique.**

| Repère | 12/10 13 h 34 | 12/10 9 h 12 |
| --- | ---: | ---: |
| NTC au départ | 89,55 °C | 89,77 °C |
| Pic avant l'infusion | 91,15 °C à 12,9 s | 91,46 °C à 13,9 s |
| Creux au début de l'infusion | 89,39 °C à 18,6 s | 88,24 °C à 18,9 s |
| Rebond, commande à 0 % | 89,43 °C à 20,0 s, palier | 89,58 °C à 23,5 s |
| **Tasse** | **87,96 °C**, −2,04 °C | 88,75 °C, −1,25 °C |
| Minimum pendant l'écoulement | 85,81 °C, à l'arrêt | 87,22 °C, à l'arrêt |
| Minimum total | 85,18 °C à 39,0 s | 86,78 °C à 43,7 s |
| 20 s après l'arrêt | 88,60 °C | 89,49 °C |
| Commande à 0 % (remboursement) | 10,9 s | 12,5 s |
| Appoint repris avant l'arrêt | 7,6 s | 9,5 s |
| Pente de la NTC pendant l'appoint | −0,25 °C/s | −0,24 °C/s |

- Énergie, commande / SSR : précharge 8,38 / 8,14 kJ, remplissage 4,90 /
  3,99 kJ, pré-infusion 2,19 / 2,23 kJ, infusion 4,99 / 4,36 kJ. Total
  fourni par le SSR : 18,72 kJ pour 67,0 ml admis à l'arrêt (20,26 kJ pour
  71,6 ml à 9 h 12). Jusqu'au début de l'infusion, le SSR fournit
  14,36 kJ (15,43 kJ à 9 h 12).
- Pendant les 10,9 s à 0 %, 25,8 ml entrent dans la chaudière (26,7 ml en
  12,5 s à 9 h 12).
- Appoint, de sa reprise à l'arrêt : commande moyenne de 55 %, SSR actif
  48 % du temps. Le besoin calculé pour 2,09 ml/s est aussi de 55 %.
- La tasse reçoit 13,0 g avant la reprise de l'appoint, NTC moyenne
  88,8 °C, et 9,2 g après, NTC moyenne 86,8 °C.
- Trou de commande au début du remplissage : 5 % de 8,3 à 9,2 s ; le SSR
  reste coupé 1,7 s (2,0 s à 9 h 12).
- Modèle : 88,71 °C en tasse avec la commande réelle, soit +0,75 °C sur la
  mesure (+0,44 à 9 h 12) ; 88,19 °C avec l'énergie SSR, soit +0,23 °C
  (+0,06 à 9 h 12).

**Attendus vérifiés** (entrée du 12 octobre, 9 h 12, mêmes réglages) :

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 8,0 à 9,0 bar·s/g ; 17 à 20 s depuis 8 bar | **7,0 bar·s/g, 14,4 s : non atteint**, galette plus perméable |
| Rendement | ≈ 23,3 g final | 23,3 g : atteint |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 6 s et 18 à 20 ml | 3,59 ml/s, 5,43 s et 18,6 ml : atteint |
| Pré-infusion | fin à l'échéance de 4 s | **fin sur la goutte après 2,47 s : non atteint** |
| | goutte visible à la fin de la pré-infusion, vue par la balance dans la seconde suivante | café visible sous le panier à la fin, balance 0,6 s plus tard : atteint |
| Tasse | 88,2 à 89,2 °C | **87,96 °C : non atteint** |
| | appoint repris 8 s ou plus avant l'arrêt | **7,6 s : non atteint** |
| Chauffe | rebond au-dessus de 89 °C avec la commande à 0 % | palier à 89,4 °C, sans rebond : atteint de justesse |
| | NTC à l'arrêt ≈ 87,2 °C | **85,81 °C : non atteint** |
| Goût | même résultat | équilibré, excellent : atteint |

**Lecture.**

- *Débit, mesuré.* La résistance au plateau passe de 8,5 à 7,0 bar·s/g
  (−18 %) sans changement de mouture déclaré. C'est la valeur du
  11 octobre à 9 h 47, avec la mouture plus grosse. Seul le tassage est
  désigné, et rien ne le mesure. Hypothèse non testée : le tassage peut-être
  plus appuyé de 9 h 12 explique une partie des 8,5 bar·s/g, et la mouture
  affinée seule pèse moins que ne le laissait croire cette infusion. Deux
  infusions ne suffisent pas à départager.
- *Pré-infusion, mesuré et observé.* Le remplissage admet le même volume
  (18,6 ml) jusqu'au même seuil. La galette, plus perméable, laisse passer
  la goutte après 6,4 ml de pré-infusion au lieu de plus de 8,7 ml. La
  fin sur la goutte coïncide aussi, à l'œil, avec le café visible sous le
  panier. Sur les deux infusions du jour, la pré-infusion finit donc au
  bon moment, que ce soit sur la goutte ou sur l'échéance.
- *Chauffe, mesuré.* Le creux du début de l'infusion est faible
  (89,39 °C), car la pré-infusion admet moins d'eau froide (6,4 ml contre
  8,7 ml). Il n'y a ensuite ni rebond ni remontée : la NTC tient 89,4 °C
  2 s, puis baisse sans arrêt jusqu'à l'arrêt. Jusqu'au début de
  l'infusion, le SSR a fourni 1,07 kJ de moins qu'à 9 h 12. La
  pré-infusion plus courte (1,56 s de moins) en fournit 0,64 kJ, la
  précharge et le remplissage le reste. Pour une capacité de 1,47 kJ/K
  (modèle), c'est ≈ 0,7 K de moins dans la chaudière, estimé.
- *Tasse, simulé.* Le modèle rejoue la mesure à +0,23 °C avec l'énergie
  SSR, comme à 9 h 12 à 0,2 °C près. L'écart entre les deux infusions
  s'explique donc, en simulation, par l'énergie réellement fournie et par
  l'hydraulique. Le premier tiers de la tasse arrive vers 89 °C, mais
  l'infusion plus rapide fait arriver les 9,2 g de la fin pendant la
  baisse, à 86,8 °C en moyenne.
- *Tasse, réserve.* La SCACE du matin montre que le creux de la NTC
  n'atteint presque pas l'eau au-dessus du puck. Les 0,8 °C de moins en
  NTC tasse que l'infusion de 9 h 12 ne valent donc probablement pas
  0,8 °C de moins au panier. Ce n'est pas un effet de la durée : cette
  infusion est la plus courte des deux, et elle est pourtant jugée plus
  froide.
- *Rendement, mesuré.* Au débit de 1,26 g/s, la balance dépasse moins le
  seuil à l'arrêt (22,6 g) et les gouttes remontent à 0,7 g. Le poids
  final reste à 23,3 g.
- *Goût.* Excellent à 7,0 comme à 8,5 bar·s/g, à 87,96 comme à 88,75 °C
  en NTC tasse. Sur cette plage, ni le débit ni la température ne
  dégradent le goût d'une manière perceptible.

**Décision.** Aucun réglage n'est changé : mouture du jour, cible de 23 g,
firmware 0.4.4 et chauffe conservés. La variation du tassage est acceptée.

**Attendus pour l'infusion suivante** (mêmes réglages, tassage variable) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 14 à 19 s depuis 8 bar |
| Rendement | 23,3 ± 0,2 g final |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 6 s et 18 à 20 ml |
| Pré-infusion | fin sur la goutte sous ≈ 7,5 bar·s/g, à l'échéance au-dessus ; café visible sous le panier à la fin dans les deux cas |
| Tasse | 87,8 à 89,0 °C, d'autant plus haut que l'infusion depuis 8 bar est longue |
| Chauffe | rebond au-dessus de 89 °C si la pré-infusion va à l'échéance, palier sinon ; modèle avec l'énergie SSR à ± 0,3 °C de la mesure |
| Goût | même résultat ; s'il change, comparer d'abord la résistance au plateau |

### 13 octobre, 8 h 18 — `261013-081846`

**Contexte.** Consigne relevée de 90 à **92 °C** ; mouture, cible de 23 g,
écran 0.4.4 et autres réglages supposés inchangés depuis le 12 octobre.
Observations de David : un peu d'amertume ; en fin d'infusion, le filet
de café devenait vraiment fin et blond.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,13 s | 7,89 s | — | — |
| Remplissage | 8,03 s | 5,12 s | 17,7 ml (3,45 ml/s) | 1,09 → 0,96 bar |
| Pré-infusion | 13,15 s | 4,00 s | 8,1 ml | 0,96 → 1,39 bar |
| Infusion | 17,15 s | 21,56 s | 48,1 ml | 9,20 bar max |

- Pression avant la pompe : 0,88 à 1,07 bar (0,45 à 0,51 bar le
  12 octobre à 13 h 34), au-dessus du seuil de fin de remplissage. La
  garde de 1 s suffit : la pression tombe à 0,36 bar en 0,5 s, et le
  remplissage part du même point bas que la veille.
- Remplissage : débit médian sous 1 bar de **3,59 ml/s** à 75 %. Premier
  paquet au-dessus de 0,8 bar à 12,95 s (0,81 bar), passage en
  pré-infusion à 13,15 s, après 17,7 ml (18,6 ml les deux fois la veille).
- Pré-infusion : finie sur l'**échéance** de 4 s, comme à 9 h 12 la
  veille. La balance voit la goutte 1,0 s après, à 28,7 ml. La marge de
  remplissage est de **11,0 ml** (11,5 ml à 9 h 12, 6,9 ml à 13 h 34).
- Montée de 3,2 s jusqu'à 8 bar (3,3 s à 9 h 12).
- Depuis 8 bar : **18,5 s**, 1,09 g/s en tasse, 2,03 ml/s en amont (18,7 s,
  1,04 g/s et 1,85 ml/s à 9 h 12). Arrêt au poids à 23,1 g, 23,6 g final,
  soit 0,5 g de gouttes (22,8 g, 23,3 g et 0,5 g à 9 h 12).
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,92 ± 0,10 bar, pompe à
  66 %, 1,12 g/s, soit **8,0 bar·s/g** (8,5 à 9 h 12, 7,0 à 13 h 34).
- Sur la phase d'infusion, la tasse reçoit 0,47 g par ml admis (0,51 à
  9 h 12, 0,52 à 13 h 34). Après l'arrêt, la pression remonte à 9,39 bar.

**Thermique.** L'infusion de 9 h 12 la veille a la même hydraulique
(échéance, 18,7 s depuis 8 bar) : c'est la référence de comparaison.

| Repère | 13/10 8 h 18, 92 °C | 12/10 9 h 12, 90 °C | 12/10 13 h 34, 90 °C |
| --- | ---: | ---: | ---: |
| NTC au départ | 91,61 °C, −0,39 °C | 89,77 °C, −0,23 °C | 89,55 °C, −0,45 °C |
| Pic avant l'infusion | 92,93 °C à 12,5 s, +0,93 °C | 91,46 °C à 13,9 s, +1,46 °C | 91,15 °C à 12,9 s, +1,15 °C |
| Creux au début de l'infusion | —, baisse continue jusqu'à 90,40 °C à 22,2 s | 88,24 °C à 18,9 s | 89,39 °C à 18,6 s |
| Rebond, commande à 0 % | 90,45 °C à 23,5 s, palier | 89,58 °C à 23,5 s | 89,43 °C à 20,0 s, palier |
| **Tasse** | **89,71 °C**, −2,29 °C | 88,75 °C, −1,25 °C | 87,96 °C, −2,04 °C |
| Minimum pendant l'écoulement | 87,72 °C, à l'arrêt | 87,22 °C, à l'arrêt | 85,81 °C, à l'arrêt |
| Minimum total | 87,32 °C à 42,9 s | 86,78 °C à 43,7 s | 85,18 °C à 39,0 s |
| 20 s après l'arrêt | 90,97 °C | 89,49 °C | 88,60 °C |
| Commande à 0 % (remboursement) | 12,0 s | 12,5 s | 10,9 s |
| Appoint repris avant l'arrêt | 9,6 s | 9,5 s | 7,6 s |
| Pente de la NTC pendant l'appoint | −0,26 °C/s | −0,24 °C/s | −0,25 °C/s |

- Énergie, commande / SSR : précharge 8,52 / 8,40 kJ, remplissage 4,57 /
  3,67 kJ, pré-infusion 2,99 / 2,56 kJ, infusion 6,50 / 5,79 kJ. Total
  fourni par le SSR : 20,42 kJ pour 73,9 ml admis à l'arrêt (20,26 kJ pour
  71,6 ml à 9 h 12). Jusqu'au début de l'infusion, le SSR fournit
  14,63 kJ (15,43 kJ à 9 h 12).
- Pendant les 12,0 s à 0 %, 27,8 ml entrent dans la chaudière (27,0 ml en
  12,5 s à 9 h 12).
- Appoint, de sa reprise à l'arrêt : commande moyenne de 57 %, SSR actif
  49 % du temps. Le besoin calculé pour 2,13 ml/s est de 56 %.
- La tasse reçoit 11,8 g avant la reprise de l'appoint, NTC moyenne
  90,5 °C, et 11,0 g après, NTC moyenne 88,9 °C (12,2 g à 89,2 °C et
  10,6 g à 88,3 °C à 9 h 12).
- Trou de commande au début du remplissage : 5 % de 8,4 à 9,3 s ; le SSR
  reste coupé 1,8 s.
- Modèle : 90,96 °C en tasse avec la commande réelle, soit +1,25 °C sur la
  mesure (+0,44 à 9 h 12) ; 90,14 °C avec l'énergie SSR, soit +0,43 °C
  (+0,06 à 9 h 12).

**Attendus vérifiés** (entrée du 12 octobre, 13 h 34). Ils supposaient la
consigne de 90 °C ; les attendus de température sont relevés ici de
+2 °C.

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 14 à 19 s depuis 8 bar | 8,0 bar·s/g, 18,5 s : atteint |
| Rendement | 23,3 ± 0,2 g final | **23,6 g : non atteint** (+0,1 g) |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 6 s et 18 à 20 ml | 3,59 ml/s, 5,12 s ; **17,7 ml : non atteint** de 0,3 ml |
| Pré-infusion | fin à l'échéance au-dessus de ≈ 7,5 bar·s/g | échéance à 8,0 bar·s/g : atteint |
| | café visible sous le panier à la fin | non observé ; balance 1,0 s après la fin |
| Tasse | 89,8 à 91,0 °C, haut de la plage pour une infusion longue | **89,71 °C pour 18,5 s : non atteint** |
| Chauffe | rebond au-dessus de 91 °C si la pré-infusion va à l'échéance | **palier à 90,4 °C, sans rebond : non atteint** |
| | modèle avec l'énergie SSR à ± 0,3 °C de la mesure | **+0,43 °C : non atteint** |
| Goût | même résultat ; s'il change, comparer d'abord la résistance au plateau | **un peu d'amertume, filet fin et blond en fin d'infusion : non atteint** ; résistance de 8,0 bar·s/g, entre 7,0 et 8,5 |

**Lecture.**

- *Tasse, mesuré.* Les 2 °C de consigne en plus ne donnent que +0,96 °C
  en NTC tasse par rapport à 9 h 12, à hydraulique égale. L'écart au
  critère passe de −1,25 à −2,29 °C. Le départ (−0,39 °C) et l'appoint
  (repris 9,6 s avant l'arrêt) sont comparables ; la différence vient
  du début de l'infusion : le pic avant l'infusion n'est qu'à +0,93 °C
  au-dessus de la consigne (+1,46 °C à 9 h 12), puis la NTC baisse sans
  creux ni rebond.
- *Chauffe, estimé.* La précharge (8 s à 90 %) et le remboursement sont
  fixés en énergie, et le firmware calcule le besoin de l'eau avec un
  écart fixe de 70,5 K entre l'eau froide et la chaudière, juste pour une
  consigne de 90 °C. À 92 °C, chaque ml demande 2,8 % de plus. Bilan à
  l'arrêt contre 9 h 12 : l'eau demande ≈ 1,3 kJ de plus (2,3 ml de plus,
  et 2 K de plus sur 74 ml), le SSR ne fournit que 0,16 kJ de plus. Pour
  une capacité de 1,47 kJ/K (modèle), le déficit vaut ≈ 0,8 K, du même
  ordre que le 1,04 K perdu sur l'écart au critère.
- *Tasse, simulé.* Le modèle suppose le même écart fixe de 70,5 K. Rejoué
  avec 72,5 K, il donne 89,87 °C avec l'énergie SSR, soit +0,16 °C sur la
  mesure : l'écart d'eau froide explique, en simulation, 0,27 des 0,37 °C
  d'écart supplémentaire au modèle par rapport à 9 h 12.
- *Tasse, réserve.* La SCACE du 12 octobre montre que les variations de
  la NTC n'atteignent pas toutes l'eau au-dessus du puck. Les +0,96 °C en
  NTC tasse ne se transposent pas tels quels au panier. Hypothèse non
  testée : le relèvement de la consigne chauffe toute la chaudière, et
  l'eau au panier gagne entre 1 et 2 °C.
- *Remplissage, mesuré.* La pression au repos, plus haute, n'a pas
  écourté le remplissage grâce à la garde. Le volume admis perd 0,9 ml
  sans changement de débit ; la cause n'est pas identifiée.
- *Rendement, mesuré.* La balance dépasse le seuil de 0,1 g à l'arrêt
  (23,1 g, contre 22,8 g à 9 h 12 et 22,6 g à 13 h 34) ; avec 0,5 g de
  gouttes, le poids final monte à 23,6 g.
- *Goût, hypothèse non testée.* Par rapport à 9 h 12, très bon à
  8,5 bar·s/g et 18,7 s, deux choses changent : la température (+0,96 °C
  en NTC tasse, plus en fin d'infusion : 88,9 °C contre 88,3 °C sur les
  11 g d'après l'appoint) et 0,3 g de plus en tasse. Le filet blond de la
  fin désigne les derniers grammes. Rien ne sépare l'effet de la
  température de celui du poids sur cette seule infusion.

**Décision** (David). Retour à la cible de **22 g** pour retirer les
derniers grammes, les plus blonds. Consigne de 92 °C, mouture, firmware
0.4.4 et chauffe conservés. Les deux changements depuis 9 h 12
(consigne, puis poids) se séparent ainsi : si l'amertume disparaît à
22 g et 92 °C, elle venait de la fin de l'infusion ; si elle reste, de la
température.

**Attendus pour l'infusion suivante** (consigne de 92 °C, cible de 22 g) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar (≈ 0,9 s de moins pour 1 g de moins) |
| Rendement | 22,3 à 22,6 g final |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 6 s et 17 à 20 ml |
| Pré-infusion | fin sur la goutte sous ≈ 7,5 bar·s/g, à l'échéance au-dessus |
| Tasse | 89,4 à 90,4 °C ; la coupe à 22 g retire ≈ 0,1 °C de grammes froids (estimé sur cette capture) |
| Chauffe | palier vers 90,4 °C après le début de l'infusion, sans rebond au-dessus de 91 °C ; modèle avec l'énergie SSR à +0,2 à +0,5 °C de la mesure, tant qu'il suppose 70,5 K |
| Goût | moins d'amertume, filet moins blond à la fin ; sinon la température est la variable restante |

### 13 octobre, 13 h 29 — `261013-132935`

**Contexte.** Cible ramenée de 23 à **22 g** ; consigne de 92 °C,
mouture, écran 0.4.4 et chauffe inchangés depuis 8 h 18. Observations
de David : résultat gustatif impeccable ; porte-filtre resté sur le groupe
plus longtemps qu'à l'habitude avant l'infusion, la machine étant à la
consigne (durée non relevée).

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,14 s | 7,89 s | — | — |
| Remplissage | 8,02 s | 6,35 s | 21,7 ml (3,43 ml/s) | 0,45 → 0,98 bar |
| Pré-infusion | 14,37 s | 3,28 s | 7,8 ml | 0,98 → 1,40 bar |
| Infusion | 17,65 s | 18,86 s | 41,7 ml | 9,33 bar max |

- Pression avant la pompe : 0,33 à 0,45 bar (0,88 à 1,07 bar à 8 h 18,
  0,45 à 0,51 bar le 12 octobre à 13 h 34).
- Remplissage : débit médian sous 1 bar de **3,56 ml/s** à 75 %. La
  pression reste vers 0,4 bar jusqu'à 12,7 s et 15,9 ml. Premier paquet
  au-dessus de 0,8 bar à 14,12 s (0,87 bar), passage en pré-infusion à
  14,37 s, après **21,7 ml** (17,7 ml à 8 h 18, 18,6 ml le 12 octobre).
- Pré-infusion : finie sur la **goutte** (firmware, 0,1 g à 17,65 s). Le
  repère du rapport suit 0,47 s après, à 30,1 ml. La marge de remplissage
  est de **8,4 ml** (11,0 ml à 8 h 18, 7,0 ml le 12 octobre à 13 h 34).
- Montée de 3,2 s jusqu'à 8 bar (3,2 s à 8 h 18, 4,3 s le 12 octobre à
  13 h 34).
- Depuis 8 bar : **15,6 s**, 1,18 g/s en tasse, 2,06 ml/s en amont (18,5 s,
  1,09 g/s et 2,03 ml/s à 8 h 18). Arrêt au poids à 21,8 g, 22,4 g final,
  soit 0,6 g de gouttes.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,88 ± 0,08 bar, pompe à
  67 %, 1,23 g/s, soit **7,2 bar·s/g** (8,0 à 8 h 18, 7,1 le 12 octobre à
  13 h 34).
- Sur la phase d'infusion, la tasse reçoit 0,52 g par ml admis (0,47 à
  8 h 18, 0,53 le 12 octobre à 13 h 34). Après l'arrêt, la pression remonte
  à 9,71 bar.

**Thermique.** L'infusion du 12 octobre à 13 h 34 a la même hydraulique
(goutte, 7,1 bar·s/g) à 90 °C ; celle de 8 h 18, la même consigne.

| Repère | 13/10 13 h 29, 92 °C | 13/10 8 h 18, 92 °C | 12/10 13 h 34, 90 °C |
| --- | ---: | ---: | ---: |
| NTC au départ | 92,10 °C, +0,10 °C | 91,61 °C, −0,39 °C | 89,55 °C, −0,45 °C |
| Pic avant l'infusion | **94,01 °C** à 13,8 s, +2,01 °C | 92,93 °C à 12,5 s, +0,93 °C | 91,15 °C à 12,9 s, +1,15 °C |
| Creux au début de l'infusion | 91,99 °C à 20,6 s | —, baisse continue jusqu'à 90,40 °C à 22,2 s | 89,39 °C à 18,6 s |
| Rebond, commande à 0 % | **92,71 °C** à 23,4 s | 90,45 °C à 23,5 s, palier | 89,43 °C à 20,0 s, palier |
| **Tasse** | **91,96 °C**, −0,04 °C | 89,71 °C, −2,29 °C | 87,96 °C, −2,04 °C |
| Minimum pendant l'écoulement | 90,77 °C, à l'arrêt | 87,72 °C, à l'arrêt | 85,81 °C, à l'arrêt |
| Minimum total | 89,67 °C à 41,8 s | 87,32 °C à 42,9 s | 85,18 °C à 39,0 s |
| 20 s après l'arrêt | 92,14 °C | 90,97 °C | 88,60 °C |
| Commande à 0 % (remboursement) | 12,1 s | 12,0 s | 10,9 s |
| Appoint repris avant l'arrêt | 6,6 s | 9,6 s | 7,6 s |
| Pente de la NTC pendant l'appoint | −0,18 °C/s | −0,26 °C/s | −0,25 °C/s |

- Énergie, commande / SSR : précharge 8,40 / 8,33 kJ, remplissage 5,82 /
  4,90 kJ, pré-infusion 2,74 / 2,52 kJ, infusion 4,70 / 4,66 kJ. Total
  fourni par le SSR : 20,41 kJ pour 71,3 ml admis à l'arrêt (20,42 kJ pour
  73,9 ml à 8 h 18). Jusqu'au début de l'infusion, le SSR fournit
  15,75 kJ (14,63 kJ à 8 h 18), pour 29,6 ml admis (25,8 ml).
- Pendant les 12,1 s à 0 %, 27,0 ml entrent dans la chaudière (27,8 ml à
  8 h 18).
- Appoint, de sa reprise à l'arrêt : commande moyenne de 58 %, SSR actif
  59 % du temps. Le besoin calculé pour 2,36 ml/s est de 62 %.
- La tasse reçoit 13,5 g avant la reprise de l'appoint, NTC moyenne
  92,2 °C, et 8,3 g après, NTC moyenne 91,5 °C (11,8 g à 90,5 °C et
  11,0 g à 88,9 °C à 8 h 18).
- Trou de commande au début du remplissage : 5 % de 8,4 à 9,0 s ; le SSR
  reste coupé 1,8 s, comme à 8 h 18.
- Modèle : 91,43 °C en tasse avec la commande réelle, soit **−0,53 °C**
  sur la mesure (+1,25 à 8 h 18) ; 90,81 °C avec l'énergie SSR, soit
  **−1,15 °C** (+0,43 à 8 h 18) ; 90,55 °C avec un écart d'eau froide de
  72,5 K. Avec l'énergie SSR, le modèle passe sous la mesure dès le
  remplissage (−0,93 °C 3 s après son début, −0,34 °C à 8 h 18) et y
  reste jusqu'à la fin (−1,7 à −0,7 °C). Il faudrait un écart d'eau
  froide de ≈ 62 K, au lieu de 70,5, pour retrouver la tasse mesurée.

**Attendus vérifiés** (entrée de 8 h 18).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | 7,2 bar·s/g, 15,6 s : atteint |
| Rendement | 22,3 à 22,6 g final | 22,4 g : atteint |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 6 s et 17 à 20 ml | 3,56 ml/s ; **6,35 s et 21,7 ml : non atteint** |
| Pré-infusion | fin sur la goutte sous ≈ 7,5 bar·s/g, à l'échéance au-dessus | goutte à 7,2 bar·s/g : atteint |
| Tasse | 89,4 à 90,4 °C | **91,96 °C : non atteint** (+1,56 °C au-dessus de la plage) |
| Chauffe | palier vers 90,4 °C après le début de l'infusion, sans rebond au-dessus de 91 °C | **creux à 91,99 °C, rebond à 92,71 °C : non atteint** |
| | modèle avec l'énergie SSR à +0,2 à +0,5 °C de la mesure | **−1,15 °C : non atteint** |
| Goût | moins d'amertume, filet moins blond à la fin | impeccable : atteint ; filet non rapporté |

**Lecture.**

- *Tasse, mesuré.* À consigne égale, la NTC tasse gagne **+2,25 °C** sur
  8 h 18 et arrive au critère (−0,04 °C). Tout l'écart est déjà là avant
  l'infusion : départ plus haut de 0,49 °C, puis la NTC monte de 1,67 °C
  pendant le remplissage (92,24 → 93,91 °C) et le pic dépasse la consigne
  de 2,01 °C. Le remboursement (12,1 s à 0 %) est le même ; l'appoint
  reprend plus tard (6,6 s avant l'arrêt) parce que l'infusion est plus
  courte, et seuls 8,3 g arrivent après sa reprise.
- *Tasse, simulé.* Le modèle, rejoué sur les deux captures du jour,
  n'explique que +0,67 °C de l'écart avec l'énergie SSR (+0,47 °C avec la
  commande) : départ plus haut et infusion plus courte. Les ≈ 1,6 °C
  restants ne viennent ni de la commande ni du SSR, qui fournit 1,1 kJ de
  plus avant l'infusion pour 3,8 ml de plus admis, soit l'équivalent.
- *Chauffe, hypothèse non testée.* Le modèle tombe sous la mesure dès le
  remplissage, comme si l'écoulement retirait moins de chaleur que prévu
  (≈ 62 K d'écart d'eau froide au lieu de 70,5 K). Cause la plus
  probable, rapportée par David : une attente plus longue à la consigne,
  porte-filtre en place. Le groupe et le porte-filtre, plus chauds,
  prennent moins de chaleur à l'eau et à la chaudière ; le départ à
  +0,10 °C, au lieu de −0,39 °C, va dans le même sens. Le réservoir est
  exclu : David y avait ajouté de l'eau du robinet avant l'infusion, donc
  plus froide, pas plus chaude. Le 12 octobre à 13 h 34, le modèle était à +0,23 °C : l'heure seule ne
  suffit pas. La capture ne contient pas la durée de l'attente, et l'eau
  au panier n'est pas mesurée.
- *Remplissage, mesuré.* La pression au repos, plus basse que le matin
  (0,33 à 0,45 bar), fait partir le remplissage du même point bas, mais
  la pression reste vers 0,4 bar jusqu'à 15,9 ml, et le remplissage
  admet 4,0 ml de plus qu'à 8 h 18 à débit égal. L'espace au-dessus de la
  galette a pris plus d'eau avant de monter en pression ; la cause n'est
  pas identifiée. La marge reste de 8,4 ml.
- *Hydraulique, mesuré.* Galette plus rapide qu'à 8 h 18 (7,2 contre
  8,0 bar·s/g), dans la plage du tassage variable du 12 octobre.
- *Goût, mesuré et hypothèse.* L'amertume de 8 h 18 disparaît alors que
  la NTC tasse monte de 2,25 °C : la température n'en est pas la cause,
  ce que la décision de 8 h 18 voulait séparer. La coupe à 22 g et la
  galette plus rapide retirent toutes deux de l'extraction en fin
  d'infusion (15,6 s contre 18,5 s depuis 8 bar) ; rien ne sépare l'une
  de l'autre sur cette infusion.

**Décision** (David). Réglages conservés : consigne de 92 °C, cible de
22 g, mouture, firmware 0.4.4 et chauffe.

**Attendus pour l'infusion suivante** (mêmes réglages) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 22,6 g final |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 6,5 s et 17 à 22 ml ; marge positive |
| Pré-infusion | fin sur la goutte sous ≈ 7,5 bar·s/g, à l'échéance au-dessus |
| Tasse | 89,7 à 92,0 °C, entre les deux infusions du jour ; au critère si la NTC dépasse à nouveau la consigne de ≈ 2 °C à la fin du remplissage |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C de la mesure ; s'il est de nouveau sous la mesure dès le remplissage, rapprocher l'écart de la durée d'attente à la consigne (à noter, même approximative) |
| Goût | même résultat ; s'il change, comparer d'abord la résistance au plateau puis la NTC tasse |

### 14 octobre, 9 h 42 — `261014-094256`

**Contexte.** Première infusion en **mode guidé** (écran 0.4.10) : deux
rinçages de 5,0 s chronométrés par le firmware, attente de la consigne,
préparation, puis infusion lancée par « démarrer » après une tare. Consigne
de 92 °C, cible de 22 g, mouture et chauffe inchangées depuis le
13 octobre à 13 h 29. Observation de David : goût **légèrement plus acide
que d'habitude**.

**Routine** (clé `session` de la capture, première mesure de l'attente) :

| Étape | Mesure |
| --- | --- |
| Rinçage 1, tasse tarée, sans porte-filtre | 5,05 s, 19,1 ml au débitmètre, 31,7 g en tasse ; NTC 91,1 → 90,6 °C |
| Rinçage 2, porte-filtre en place, 25,0 s après le début du premier | 5,02 s, 19,1 ml, 24,3 g ; NTC 87,6 → 88,1 °C |
| Retour à la consigne | 85,3 s après le second rinçage ; minimum 85,3 °C ; résistance active 17,2 s |
| Porte-filtre en place à la consigne | 62,4 s, de la consigne atteinte au retrait de la tasse |
| Préparation (porte-filtre hors du groupe) | 142,8 s, du retrait de la tasse à « démarrer » |
| Routine entière | 5 min 41 s, de l'ouverture du guide à « démarrer » |

- Les rinçages donnent 1,66 g par ml compté. Le débitmètre sous-compte en
  débit libre (5,68 g/s contre 4,0 ml/s à 100 % dans
  [purge_sans_resistance.html](purge_sans_resistance.html), soit 1,42) ;
  le reste vient probablement de l'égouttage compris dans la pesée, lue
  2 s après l'arrêt.
- La NTC continue de baisser 20 s après le premier rinçage (90,6 →
  87,6 °C) : le minimum relevé pendant un rinçage ne dit pas le creux, la
  sonde est en retard.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,03 s | — | — |
| Remplissage | 8,03 s | 7,08 s | 24,3 ml (3,44 ml/s) | 0,44 → 0,92 bar |
| Pré-infusion | 15,11 s | 1,76 s | 4,9 ml | 0,92 → 1,07 bar |
| Infusion | 16,87 s | 19,93 s | 46,1 ml | 9,42 bar max |

- Pression avant la pompe : 0,36 à 0,46 bar (0,33 à 0,45 bar à 13 h 29
  la veille).
- Remplissage : débit médian sous 1 bar de **3,57 ml/s** à 75 %. Premier
  paquet au-dessus de 0,8 bar à 14,85 s (0,81 bar), passage en
  pré-infusion à 15,11 s, après **24,3 ml** (21,7 ml à 13 h 29, 17,7 ml à
  8 h 18 la veille).
- Pré-infusion : finie sur la **goutte** (firmware, 0,2 g à 16,87 s). Le
  repère du rapport suit 0,75 s après, à 31,0 ml. La marge de remplissage
  est de **6,7 ml** (8,4 ml à 13 h 29, 11,0 ml à 8 h 18).
- Montée de 3,9 s jusqu'à 8 bar (3,2 s les deux fois la veille).
- Depuis 8 bar : **16,1 s**, 1,10 g/s en tasse, 2,04 ml/s en amont (15,6 s,
  1,18 g/s et 2,06 ml/s à 13 h 29). Arrêt au poids à 21,8 g, 22,5 g final,
  soit 0,7 g de gouttes.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 9,05 ± 0,06 bar, pompe à
  68 %, 1,12 g/s, soit **8,1 bar·s/g** (7,2 à 13 h 29, 8,0 à 8 h 18).
- Sur la phase d'infusion, la tasse reçoit 0,47 g par ml admis (0,52 à
  13 h 29, 0,47 à 8 h 18). Après l'arrêt, la pression remonte à 9,19 bar.

**Thermique.** L'infusion de 13 h 29 la veille a les mêmes réglages ;
celle de 8 h 18, une galette aussi lente.

| Repère | 14/10 9 h 42 | 13/10 13 h 29 | 13/10 8 h 18 |
| --- | ---: | ---: | ---: |
| NTC au départ | 91,99 °C, −0,01 °C | 92,10 °C, +0,10 °C | 91,61 °C, −0,39 °C |
| Pic avant l'infusion | 93,67 °C à 13,1 s, +1,67 °C | 94,01 °C à 13,8 s, +2,01 °C | 92,93 °C à 12,5 s, +0,93 °C |
| Creux au début de l'infusion | **90,36 °C** à 23,5 s | 91,99 °C à 20,6 s | —, baisse continue jusqu'à 90,40 °C à 22,2 s |
| Rebond, commande à 0 % | 91,24 °C à 28,3 s | 92,71 °C à 23,4 s | 90,45 °C à 23,5 s, palier |
| **Tasse** | **90,89 °C**, −1,11 °C | 91,96 °C, −0,04 °C | 89,71 °C, −2,29 °C |
| Minimum pendant l'écoulement | 89,89 °C, à l'arrêt | 90,77 °C, à l'arrêt | 87,72 °C, à l'arrêt |
| Minimum total | 89,50 °C à 40,4 s | 89,67 °C à 41,8 s | 87,32 °C à 42,9 s |
| 20 s après l'arrêt | 92,34 °C | 92,14 °C | 90,97 °C |
| Commande à 0 % (remboursement) | 11,3 s | 12,1 s | 12,0 s |
| Appoint repris avant l'arrêt | 8,5 s | 6,6 s | 9,6 s |
| Pente de la NTC pendant l'appoint | −0,16 °C/s | −0,18 °C/s | −0,26 °C/s |

- Énergie, commande / SSR : précharge 8,54 / 8,65 kJ, remplissage 6,72 /
  6,69 kJ, pré-infusion 1,67 / 1,56 kJ, infusion 5,65 / 5,24 kJ. Total
  fourni par le SSR : 22,14 kJ pour 75,4 ml admis à l'arrêt (20,41 kJ pour
  71,3 ml à 13 h 29). Jusqu'au début de l'infusion, le SSR fournit
  16,90 kJ (15,75 kJ à 13 h 29), pour 29,3 ml admis (29,6 ml).
- Pendant les 11,3 s à 0 %, 28,7 ml entrent dans la chaudière (27,0 ml en
  12,1 s à 13 h 29).
- Appoint, de sa reprise à l'arrêt : commande moyenne de 55 %, SSR actif
  51 % du temps. Le besoin calculé pour 2,08 ml/s est de 55 %.
- La tasse reçoit 11,5 g avant la reprise de l'appoint, NTC moyenne
  91,2 °C, et 10,1 g après, NTC moyenne 90,5 °C (13,5 g à 92,2 °C et
  8,3 g à 91,5 °C à 13 h 29).
- Trou de commande au début du remplissage : 5 % de 8,3 à 9,2 s ; le SSR
  reste coupé 1,0 s (1,8 s la veille).
- Modèle : 90,89 °C en tasse avec la commande réelle, soit **−0,01 °C**
  sur la mesure (−0,53 à 13 h 29) ; 90,96 °C avec l'énergie SSR, soit
  **+0,06 °C** (−1,15 à 13 h 29) ; 90,68 °C avec un écart d'eau froide de
  72,5 K. Avec l'énergie SSR, le modèle est sous la mesure 3 s après le
  début du remplissage (−0,78 °C ; −0,93 à 13 h 29, −0,34 à 8 h 18), puis
  la rejoint.

**Attendus vérifiés** (entrée du 13 octobre, 13 h 29).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | 8,1 bar·s/g, 16,1 s : atteint |
| Rendement | 22,3 à 22,6 g final | 22,5 g : atteint |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 6,5 s et 17 à 22 ml ; marge positive | 3,57 ml/s, marge de 6,7 ml ; **7,08 s et 24,3 ml : non atteint** |
| Pré-infusion | fin sur la goutte sous ≈ 7,5 bar·s/g, à l'échéance au-dessus | **goutte à 8,1 bar·s/g : non atteint** |
| Tasse | 89,7 à 92,0 °C ; au critère si la NTC dépasse la consigne de ≈ 2 °C à la fin du remplissage | 90,89 °C : atteint ; dépassement de +1,67 °C, tasse à −1,11 °C |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C ; s'il est sous la mesure dès le remplissage, rapprocher l'écart de la durée d'attente | +0,06 °C : atteint ; attente mesurée (62,4 s à la consigne, 142,8 s de préparation) |
| Goût | même résultat ; s'il change, comparer d'abord la résistance au plateau puis la NTC tasse | **légèrement plus acide : non atteint** ; résistance de 8,1 bar·s/g (7,2 à 13 h 29), NTC tasse −1,07 °C |

**Lecture.**

- *Tasse, mesuré.* La NTC tasse perd **1,07 °C** sur 13 h 29. Le départ
  est le même (−0,01 contre +0,10 °C) ; l'écart se fait au début de
  l'infusion : pic plus bas de 0,34 °C, puis creux à 90,36 °C au lieu de
  91,99 °C. La galette plus lente fait reprendre l'appoint 8,5 s avant
  l'arrêt (6,6 s à 13 h 29) : 10,1 g arrivent après sa reprise, à 90,5 °C,
  au lieu de 8,3 g.
- *Chauffe, mesuré.* Le modèle retombe sur la mesure (+0,06 °C avec
  l'énergie SSR) : l'excès de 13 h 29 (−1,15 °C) ne se reproduit pas. Le
  SSR fournit 1,15 kJ de plus qu'à 13 h 29 avant l'infusion, pour un
  volume admis égal, et la tasse est pourtant plus froide : l'écart tient
  au système, pas à la commande.
- *Chauffe, hypothèse non testée.* L'hypothèse de 13 h 29 (groupe et
  porte-filtre plus chauds après une longue attente) reste compatible :
  ici le porte-filtre n'est resté que 62 s à la consigne, puis 143 s hors
  du groupe pendant la préparation. Une seule routine mesurée ne la
  confirme pas ; la durée de préparation, pendant laquelle le
  porte-filtre refroidit, est la candidate à rapprocher de l'écart du
  modèle sur les prochaines routines guidées.
- *Remplissage, mesuré.* Troisième hausse de suite du volume admis
  (17,7 → 21,7 → 24,3 ml) à débit égal ; la pression reste basse plus
  longtemps. Le volume jusqu'à la goutte (31,0 ml) reste dans la plage de
  la série (28,7 à 32,8 ml) : l'eau en plus passe de la pré-infusion
  (4,9 ml) au remplissage, et la goutte termine la pré-infusion malgré
  une résistance de 8,1 bar·s/g. La cause n'est pas identifiée.
- *Goût, hypothèse non testée.* Une acidité plus marquée va dans le sens
  d'une extraction plus faible. La galette, plus lente qu'à 13 h 29
  (8,1 contre 7,2 bar·s/g, 16,1 contre 15,6 s), extrait plutôt
  davantage : elle n'explique pas l'acidité. Restent la température, plus
  basse de 1,07 °C en NTC tasse, et le porte-filtre, plus longtemps hors
  du groupe, qui n'est pas mesuré ; rien ne les sépare sur cette
  infusion.

**Décision** (David). Rien ne change : consigne de 92 °C, cible de 22 g,
mouture, chauffe et routine guidée conservées, pour suivre l'évolution
sans changement. Les routines guidées suivantes donnent la durée de
préparation à rapprocher de l'écart du modèle et de la NTC tasse.

**Attendus pour l'infusion suivante** (mêmes réglages, routine guidée) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 22,6 g final |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 7,5 s et 17 à 25 ml ; marge positive ; une quatrième hausse du volume (au-delà de 24,3 ml) en ferait une tendance |
| Pré-infusion | sur la série, la marge de remplissage sépare mieux que la résistance : goutte jusqu'à ≈ 8,5 ml de marge (6,7 à 8,4 ml), échéance à partir de ≈ 11 ml (11,0 et 11,5 ml) |
| Tasse | 89,7 à 92,0 °C ; vers 90,9 °C si la galette et la préparation restent proches |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C ; d'autant plus sous la mesure que le porte-filtre est resté sur le groupe (13 h 29 la veille) et que la préparation est courte |
| Goût | si l'acidité revient avec une NTC tasse sous ≈ 91 °C, la température reste la candidate ; si elle revient au-dessus de ≈ 91,5 °C, chercher ailleurs (porte-filtre, préparation) |

### 14 octobre, 14 h 48 — `261014-144853`

**Contexte.** Seconde routine guidée, réglages inchangés depuis 9 h 42 :
consigne de 92 °C, cible de 22 g, mouture et chauffe identiques. Le guide
s'ouvre sur une machine **sous la consigne** (NTC à 84,4 °C) : David
l'avait laissée chauffer en faisant autre chose, puis a fait autre chose
pendant l'attente de la consigne, porte-filtre en place. Observation de
David : **très bonne infusion, très satisfait**.

La capture contient un échantillon NTC marqué invalide à 18,10 s (âge de
200 ms, valeur tenue à 92,14 °C, voisins à 92,14 et 92,12 °C) :
`analyze_hf_capture.py` refuse la capture. Les chiffres viennent d'une
copie où cet échantillon est rendu valide ; l'effet sur la moyenne est
inférieur à 0,01 °C.

**Routine** (clé `session` de la capture) :

| Étape | 14/10 14 h 48 | 14/10 9 h 42 |
| --- | --- | --- |
| NTC à l'ouverture du guide | **84,4 °C** | 91,7 °C |
| Rinçage 1, sans porte-filtre | 5,02 s, 19,7 ml, 28,7 g ; NTC 87,9 → 88,2 °C | 5,05 s, 19,1 ml, 31,7 g ; 91,1 → 90,6 °C |
| Rinçage 2, porte-filtre en place | 47,8 s après le début du premier ; 5,03 s, 19,7 ml, 27,2 g ; NTC 87,1 → 87,4 °C | 25,0 s après ; 5,02 s, 19,1 ml, 24,3 g ; 87,6 → 88,1 °C |
| Retour à la consigne | **268,7 s** ; minimum 82,2 °C ; résistance active 50,6 s | 85,3 s ; 85,3 °C ; 17,2 s |
| Porte-filtre en place à la consigne | **373,6 s** | 62,4 s |
| Préparation (porte-filtre hors du groupe) | **70,2 s** | 142,8 s |
| Routine entière | 13 min 5 s | 5 min 41 s |

- Les deux rinçages ont lieu 4 à 5 °C sous la consigne : la machine
  chauffait encore à l'ouverture du guide. Le porte-filtre reste
  642 s sur le groupe après le second rinçage (148 s à 9 h 42), dont
  374 s à la consigne, et la préparation est deux fois plus courte
  qu'à 9 h 42.
- Les rinçages donnent 1,46 et 1,38 g par ml compté (1,66 et 1,27 à
  9 h 42) : le rapport varie d'un rinçage à l'autre, la pesée à 2 s
  après l'arrêt ne le fixe pas.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,06 s | — | — |
| Remplissage | 8,06 s | 6,88 s | 24,1 ml (3,50 ml/s) | 0,52 → 0,87 bar |
| Pré-infusion | 14,95 s | 2,84 s | 7,2 ml | 0,87 → 1,27 bar |
| Infusion | 17,78 s | 19,87 s | 46,7 ml | 9,26 bar max |

- Pression avant la pompe : 0,44 à 0,52 bar (0,36 à 0,46 bar à 9 h 42).
- Remplissage : débit médian sous 1 bar de **3,59 ml/s** à 75 %. Premier
  paquet au-dessus de 0,8 bar à 14,69 s (0,82 bar), passage en
  pré-infusion à 14,95 s, après **24,1 ml** (24,3 ml à 9 h 42, 21,7 ml
  à 13 h 29 la veille) : pas de quatrième hausse.
- Pré-infusion : finie sur la **goutte** (firmware, 0,1 g à 17,78 s). Le
  repère du rapport suit 0,43 s après, à 32,2 ml. La marge de remplissage
  est de **8,1 ml** (6,7 ml à 9 h 42, 8,4 ml à 13 h 29).
- Montée de 3,9 s jusqu'à 8 bar (3,9 s à 9 h 42).
- Depuis 8 bar : **16,0 s**, 1,10 g/s en tasse, 2,13 ml/s en amont (16,1 s,
  1,10 g/s et 2,04 ml/s à 9 h 42). Arrêt au poids à 21,7 g, 22,7 g final,
  soit **1,0 g de gouttes** (0,7 g à 9 h 42).
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 9,03 ± 0,05 bar, pompe à
  67 %, 1,11 g/s, soit **8,1 bar·s/g** (8,1 à 9 h 42, 7,2 à 13 h 29).
- Sur la phase d'infusion, la tasse reçoit 0,46 g par ml admis (0,47 à
  9 h 42). Après l'arrêt, la pression remonte à 9,21 bar.

**Thermique.** Même galette et mêmes réglages qu'à 9 h 42 ; seule la
routine diffère. 13 h 29 la veille garde les mêmes réglages.

| Repère | 14/10 14 h 48 | 14/10 9 h 42 | 13/10 13 h 29 |
| --- | ---: | ---: | ---: |
| NTC au départ | 91,95 °C, −0,05 °C | 91,99 °C, −0,01 °C | 92,10 °C, +0,10 °C |
| Pic avant l'infusion | 93,41 °C à 13,2 s, +1,41 °C | 93,67 °C à 13,1 s, +1,67 °C | 94,01 °C à 13,8 s, +2,01 °C |
| Creux au début de l'infusion | **aucun** : 92,14 °C à 18,1 s, plat | 90,36 °C à 23,5 s | 91,99 °C à 20,6 s |
| Rebond, commande à 0 % | 92,48 °C à 22,9 s | 91,24 °C à 28,3 s | 92,71 °C à 23,4 s |
| **Tasse** | **91,65 °C**, −0,35 °C | 90,89 °C, −1,11 °C | 91,96 °C, −0,04 °C |
| Minimum pendant l'écoulement | 90,27 °C, à l'arrêt | 89,89 °C, à l'arrêt | 90,77 °C, à l'arrêt |
| Minimum total | 89,98 °C à 39,5 s | 89,50 °C à 40,4 s | 89,67 °C à 41,8 s |
| 20 s après l'arrêt | 91,98 °C | 92,34 °C | 92,14 °C |
| Commande à 0 % (remboursement) | 11,4 s | 11,3 s | 12,1 s |
| Appoint repris avant l'arrêt | 8,3 s | 8,5 s | 6,6 s |
| Pente de la NTC pendant l'appoint | −0,19 °C/s | −0,16 °C/s | −0,18 °C/s |

- Énergie, commande / SSR : précharge 8,59 / 8,38 kJ, remplissage 7,12 /
  7,35 kJ, pré-infusion 2,52 / 2,29 kJ, infusion 5,80 / 5,47 kJ. Total
  fourni par le SSR : 23,49 kJ pour 78,0 ml admis à l'arrêt (22,14 kJ
  pour 75,4 ml à 9 h 42). Jusqu'au début de l'infusion, le SSR fournit
  18,02 kJ (16,90 kJ à 9 h 42), pour 31,3 ml admis (29,3 ml).
- Pendant les 11,4 s à 0 %, 29,0 ml entrent dans la chaudière (28,7 ml
  à 9 h 42).
- Appoint, de sa reprise à l'arrêt : commande moyenne de 56 %, SSR actif
  51 % du temps. Le besoin calculé pour 2,10 ml/s est de 55 %.
- La tasse reçoit 12,2 g avant la reprise de l'appoint, NTC moyenne
  92,1 °C, et 9,5 g après, NTC moyenne 91,1 °C (11,5 g à 91,2 °C et
  10,1 g à 90,5 °C à 9 h 42).
- Trou de commande au début du remplissage : 5 % de 9,0 à 9,1 s ; le SSR
  reste coupé ≈ 0,4 s (1,0 s à 9 h 42).
- Modèle : 91,23 °C en tasse avec la commande réelle, soit **−0,44 °C**
  sur la mesure (−0,01 à 9 h 42) ; 91,17 °C avec l'énergie SSR, soit
  **−0,50 °C** (+0,06 à 9 h 42, −1,15 à 13 h 29) ; 90,94 °C avec un
  écart d'eau froide de 72,5 K. Avec l'énergie SSR, le modèle est sous
  la mesure 3 s après le début du remplissage (−0,44 °C ; −0,78 à
  9 h 42), puis le reste.

**Attendus vérifiés** (entrée du 14 octobre, 9 h 42).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | 8,1 bar·s/g, 16,0 s : atteint |
| Rendement | 22,3 à 22,6 g final | **22,7 g : non atteint** de 0,1 g ; arrêt à 21,7 g, 1,0 g de gouttes |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 7,5 s et 17 à 25 ml ; marge positive ; une quatrième hausse du volume en ferait une tendance | 3,59 ml/s, 6,88 s, 24,1 ml, marge de 8,1 ml : atteint ; pas de quatrième hausse |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml | goutte à 8,1 ml : atteint |
| Tasse | 89,7 à 92,0 °C ; vers 90,9 °C si la galette et la préparation restent proches | 91,65 °C : atteint ; préparation pas proche (70 s contre 143 s) |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C ; d'autant plus sous la mesure que le porte-filtre est resté sur le groupe et que la préparation est courte | −0,50 °C : atteint, dans le sens attendu (374 s sur le groupe, 70 s de préparation) |
| Goût | si l'acidité revient sous ≈ 91 °C de NTC tasse, la température reste la candidate ; au-dessus de ≈ 91,5 °C, chercher ailleurs | pas d'acidité, très bon, à 91,65 °C : atteint |

**Lecture.**

- *Tasse, mesuré.* La NTC tasse gagne **0,76 °C** sur 9 h 42 à galette
  égale (8,1 bar·s/g). Le départ et le pic sont les mêmes à 0,3 °C
  près ; l'écart se fait au début de l'infusion : la NTC reste à
  92,1 °C au lieu de creuser à 90,36 °C, comme à 13 h 29 la veille
  (91,99 °C). Les 12,2 g d'avant l'appoint arrivent à 92,1 °C (91,2 °C
  à 9 h 42).
- *Chauffe, simulé.* Le modèle avec la commande réelle donne +0,34 °C
  sur 9 h 42 (91,23 contre 90,89 °C) : un peu plus d'énergie avant
  l'infusion (+1,1 kJ au SSR). Il reste 0,42 °C hors modèle : le modèle
  est sous la mesure (−0,50 °C avec l'énergie SSR), comme à 13 h 29.
- *Chauffe, hypothèse.* Sur les trois infusions à 92 °C et 22 g, l'écart
  du modèle suit le temps passé par le porte-filtre sur le groupe
  avant la préparation : +0,06 °C après 62 s et 143 s de préparation,
  −0,50 °C après 374 s et 70 s, −1,15 °C à 13 h 29 (attente longue, non
  mesurée). Un porte-filtre et un groupe plus chauds prennent moins de
  chaleur à l'eau du début de l'infusion, ce que le modèle (chaudière
  seule) ne voit pas. Deux routines mesurées ne séparent pas la durée
  sur le groupe de la durée de préparation : elles ont varié ensemble,
  en sens inverse.
- *Goût, hypothèse.* Les trois infusions de même galette se rangent dans
  l'ordre de la NTC tasse : 90,89 °C légèrement acide, 91,65 °C très bon,
  91,96 °C impeccable. La température reste la candidate de l'acidité
  de 9 h 42 ; la mouture et la galette, égales à 9 h 42, n'y sont pour
  rien.
- *Rendement, mesuré.* 0,3 g de gouttes en plus qu'à 9 h 42 pour un arrêt
  à 0,1 g près : 22,7 g, au-dessus de la plage d'un dixième, sans effet
  au goût.

**Décision** (David). Rien ne change : consigne de 92 °C, cible de
22 g, mouture, chauffe et routine guidée conservées. Idée retenue : un
**temps minimal du porte-filtre sur le groupe**, que le guide imposerait
même consigne atteinte. Chaque capture suivante est lue sous cet angle
(durée sur le groupe, préparation, écart du modèle) ; si elles ne
suffisent pas au 28 octobre, la sonde SCACE le mesure sans pompe. Règles
de lecture et de bascule dans
[wizard.md](wizard.md#piste--temps-minimal-sur-le-groupe).

**Attendus pour l'infusion suivante** (mêmes réglages, routine guidée) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 22,8 g final (gouttes de 0,7 à 1,0 g) |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 7,5 s et 17 à 25 ml ; marge positive |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml |
| Tasse | 90,9 à 92,0 °C ; vers 91,6 °C si le porte-filtre reste plusieurs minutes sur le groupe et que la préparation dure ≈ 1 min, vers 90,9 °C si elle dure ≈ 2,5 min |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C ; une routine qui casse l'ordre (porte-filtre longtemps sur le groupe et modèle au-dessus de la mesure, ou l'inverse) infirme l'hypothèse du porte-filtre |
| Goût | très bon si la NTC tasse dépasse ≈ 91,5 °C ; une acidité au-dessus de ce seuil sortirait la température des candidates |

### 15 octobre, 9 h 08 — `261015-090846`

**Contexte.** Troisième routine guidée, réglages inchangés depuis le
14 octobre : consigne de 92 °C, cible de 22 g, mouture et chauffe
identiques. Le firmware de l'écran mène encore les deux rinçages de
5,0 s ; la clé `session` porte donc un tableau `purges`. Niveau d'eau
du réservoir assez bas. Observation de David : **le café le plus
équilibré bu depuis le début, très satisfait**.

La capture contient un échantillon NTC marqué invalide à 20,05 s (âge de
211 ms, valeur tenue à 90,85 °C, voisins à 90,85 et 90,80 °C) :
`analyze_hf_capture.py` refuse la capture. Les chiffres viennent d'une
copie où cet échantillon est rendu valide, comme le 14 octobre à
14 h 48.

**Routine** (clé `session` de la capture) :

| Étape | 15/10 9 h 08 | 14/10 9 h 42 | 14/10 14 h 48 |
| --- | --- | --- | --- |
| NTC à l'ouverture du guide | 88,8 °C | 91,7 °C | 84,4 °C |
| Rinçage 1, sans porte-filtre | 5,05 s, 19,4 ml, 27,6 g ; NTC 89,9 → 89,5 °C | 5,05 s, 19,1 ml, 31,7 g ; 91,1 → 90,6 °C | 5,02 s, 19,7 ml, 28,7 g ; 87,9 → 88,2 °C |
| Rinçage 2, porte-filtre en place | 42,0 s après le début du premier ; 5,00 s, 19,4 ml, 25,2 g ; 88,5 → 88,9 °C | 25,0 s ; 5,02 s, 19,1 ml, 24,3 g ; 87,6 → 88,1 °C | 47,8 s ; 5,03 s, 19,7 ml, 27,2 g ; 87,1 → 87,4 °C |
| Retour à la consigne | 103,8 s ; minimum 85,5 °C ; résistance active 23,5 s | 85,3 s ; 85,3 °C ; 17,2 s | 268,7 s ; 82,2 °C ; 50,6 s |
| Porte-filtre en place à la consigne | 170,3 s | 62,4 s | 373,6 s |
| **Sur le groupe après le rinçage 2** | **274 s** | 148 s | 642 s |
| **Préparation** | **133,6 s** | 142,8 s | 70,2 s |
| Routine entière | 7 min 51 s | 5 min 41 s | 13 min 5 s |

- La préparation est celle de 9 h 42 à 9 s près, et le porte-filtre
  reste presque deux fois plus longtemps sur le groupe : les deux
  routines forment la première paire à préparation égale.
- La balance a manqué 3,8 s au total pendant la routine
  (`scale_lost_ms`), sans effet sur les étapes.
- Les rinçages donnent 1,42 et 1,30 g par ml compté.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | — |
| Remplissage | 8,01 s | 7,34 s | **26,1 ml** (3,55 ml/s) | 0,43 → 0,89 bar |
| Pré-infusion | 15,35 s | 4,18 s | 9,3 ml | 0,89 → 1,46 bar |
| Infusion | 19,53 s | 18,99 s | 45,5 ml | 9,30 bar max |

- Pression avant la pompe : **0,34 à 0,43 bar** (0,44 à 0,52 bar à
  14 h 48, 0,36 à 0,46 bar à 9 h 42). Elle descend à 0,29 bar pendant le
  remplissage.
- Remplissage : débit médian sous 1 bar de **3,65 ml/s** à 75 %. Premier
  paquet au-dessus de 0,8 bar à 15,16 s (0,86 bar), passage en
  pré-infusion à 15,35 s, après **26,1 ml** : le plus grand volume de la
  série (24,3 ml à 9 h 42, 24,1 ml à 14 h 48, 21,7 ml le 13 à 13 h 29).
- Pré-infusion : finie à **4,18 s**, sur le premier échantillon après
  l'échéance de 4 s, qui porte aussi le premier 0,1 g. Goutte et échéance
  ne se séparent pas. Le repère du rapport (0,3 g) suit à 19,99 s, à
  36,5 ml. La marge de remplissage est de **10,4 ml** (8,1 ml à 14 h 48,
  6,7 ml à 9 h 42), entre la goutte (≈ 8,5 ml) et l'échéance (≈ 11 ml).
- Montée de **2,9 s** jusqu'à 8 bar (3,9 s les deux fois du 14) : la
  pré-infusion finit à 1,46 bar au lieu de 1,27.
- Depuis 8 bar : **16,2 s**, 1,16 g/s en tasse, 2,26 ml/s en amont (16,0 s,
  1,10 g/s et 2,13 ml/s à 14 h 48). Arrêt au poids à 21,5 g, 22,8 g final,
  soit **1,3 g de gouttes** (1,0 g à 14 h 48, 0,7 g à 9 h 42).
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 9,02 ± 0,07 bar, pompe à
  68 %, 1,19 g/s, soit **7,6 bar·s/g** (8,1 les deux fois du 14).
- Sur la phase d'infusion, la tasse reçoit 0,47 g par ml admis (0,46 à
  14 h 48). Après l'arrêt, la pression remonte à 9,25 bar.

**Thermique.** Mêmes réglages qu'au 14 octobre ; 9 h 42 a la même
préparation.

| Repère | 15/10 9 h 08 | 14/10 9 h 42 | 14/10 14 h 48 |
| --- | ---: | ---: | ---: |
| NTC au départ | 91,89 °C, −0,11 °C | 91,99 °C, −0,01 °C | 91,95 °C, −0,05 °C |
| Pic avant l'infusion | **93,13 °C à 11,5 s**, +1,13 °C | 93,67 °C à 13,1 s, +1,67 °C | 93,41 °C à 13,2 s, +1,41 °C |
| Creux au début de l'infusion | 90,34 °C à 22,0 s | 90,36 °C à 23,5 s | aucun : 92,14 °C à 18,1 s |
| Remontée, commande à 0 % | 91,07 °C à 30,7 s, à la reprise de l'appoint | 91,24 °C à 28,3 s | 92,48 °C à 22,9 s |
| **Tasse** | **90,63 °C**, −1,37 °C | 90,89 °C, −1,11 °C | 91,65 °C, −0,35 °C |
| Minimum pendant l'écoulement | 89,68 °C, à l'arrêt | 89,89 °C, à l'arrêt | 90,27 °C, à l'arrêt |
| Minimum total | 88,83 °C à 42,9 s | 89,50 °C à 40,4 s | 89,98 °C à 39,5 s |
| 20 s après l'arrêt | 91,74 °C | 92,34 °C | 91,98 °C |
| Commande à 0 % (remboursement) | 11,2 s | 11,3 s | 11,4 s |
| Appoint repris avant l'arrêt | 7,8 s | 8,5 s | 8,3 s |
| Pente de la NTC pendant l'appoint | −0,19 °C/s | −0,16 °C/s | −0,19 °C/s |

- Énergie, commande / SSR : précharge 8,31 / 8,46 kJ, remplissage 6,96 /
  **6,29** kJ, pré-infusion 3,35 / 2,91 kJ, infusion 5,70 / 5,53 kJ.
  Total fourni par le SSR : 23,19 kJ pour 80,9 ml admis à l'arrêt
  (23,49 kJ pour 78,0 ml à 14 h 48). Jusqu'au début de l'infusion, le SSR
  fournit 17,66 kJ pour **35,4 ml** admis (18,02 kJ pour 31,3 ml à
  14 h 48, 16,90 kJ pour 29,3 ml à 9 h 42).
- Trou de commande au début du remplissage : 45 % à 8,41 s puis 6,6 à
  6,9 % jusqu'à 9,31 s ; le SSR reste coupé de 8,5 à 10,46 s, **≈ 2,0 s**
  (0,4 s à 14 h 48, 1,0 s à 9 h 42). Le pic arrive plus tôt et plus bas.
- Pendant les 11,2 s à 0 %, 27,2 ml entrent dans la chaudière (29,0 ml
  à 14 h 48).
- Appoint, de sa reprise à l'arrêt : commande moyenne de 60 %, SSR actif
  61 % du temps.
- La tasse reçoit 11,7 g avant la reprise de l'appoint, NTC moyenne
  90,8 °C, et 9,8 g après, NTC moyenne 90,4 °C (11,5 g à 91,2 °C et
  10,1 g à 90,5 °C à 9 h 42).
- Modèle : 90,97 °C en tasse avec la commande réelle, soit **+0,33 °C**
  sur la mesure ; 90,58 °C avec l'énergie SSR, soit **−0,06 °C** (+0,06 à
  9 h 42, −0,50 à 14 h 48) ; 90,27 °C avec un écart d'eau froide de
  72,5 K.

**Attendus vérifiés** (entrée du 14 octobre, 14 h 48).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | 7,6 bar·s/g, 16,2 s : atteint |
| Rendement | 22,3 à 22,8 g final (gouttes de 0,7 à 1,0 g) | 22,8 g : atteint, en limite ; **1,3 g de gouttes : non atteint** |
| Remplissage | 3,5 à 3,6 ml/s à 75 % ; 5 à 7,5 s et 17 à 25 ml ; marge positive | 3,65 ml/s, 7,34 s, marge de 10,4 ml ; **26,1 ml : non atteint** de 1,1 ml |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml | 10,4 ml, fin à 4,18 s sur goutte et échéance confondues : dans l'intervalle |
| Tasse | 90,9 à 92,0 °C ; vers 90,9 °C si la préparation dure ≈ 2,5 min | **90,63 °C : non atteint** de 0,27 °C ; préparation de 2,2 min |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C ; porte-filtre longtemps sur le groupe et modèle au-dessus de la mesure infirmerait l'hypothèse | −0,06 °C : atteint ; 274 s sur le groupe pour un écart de −0,06 °C, contre +0,06 °C après 148 s |
| Goût | très bon si la NTC tasse dépasse ≈ 91,5 °C ; une acidité au-dessus de ce seuil sortirait la température des candidates | **le plus équilibré depuis le début, à 90,63 °C** : la tasse est excellente sous le seuil |

**Lecture.**

- *Tasse, mesuré.* La NTC tasse perd **0,26 °C** sur 9 h 42, à
  préparation égale et galette un peu plus rapide (7,6 contre
  8,1 bar·s/g). Le creux du début de l'infusion est le même (90,34 contre
  90,36 °C) ; l'écart vient du pic plus bas (−0,54 °C) et de la fin
  d'infusion (minimum total plus bas de 0,67 °C).
- *Chauffe, mesuré.* Deux causes côté chaudière : le SSR coupé ≈ 2 s au
  début du remplissage (0,67 kJ de moins que la commande sur le
  remplissage, 0,44 kJ sur la pré-infusion), et 35,4 ml d'eau froide
  admis avant l'infusion au lieu de 29,3 ml, par un remplissage plus long
  (26,1 ml) et une pré-infusion de 4,18 s.
- *Chauffe, simulé.* Avec l'énergie SSR, le modèle donne 90,58 °C, soit
  −0,38 °C sur 9 h 42 pour −0,26 °C mesurés : la baisse s'explique par la
  chaudière, sans rien demander au porte-filtre.
- *Porte-filtre, hypothèse.* À préparation égale (134 et 143 s), passer de
  148 à 274 s sur le groupe ne déplace l'écart du modèle que de 0,12 °C,
  sous le seuil de 0,2 °C de la règle de lecture de
  [wizard.md](wizard.md#piste--temps-minimal-sur-le-groupe). Seule
  14 h 48 garde un écart marqué (−0,50 °C), avec 642 s sur le groupe
  **et** 70 s de préparation. Deux lectures restent ouvertes : l'effet
  du groupe n'apparaît qu'au-delà de ≈ 4,5 min, ou le levier est la
  préparation. La paire du jour penche vers la préparation sans la
  prouver. Avec 274 s, la capture n'entre dans aucun des deux groupes de
  la règle (4 min ou moins, 6 min ou plus).
- *Remplissage, mesuré.* 26,1 ml, au-dessus de la plage de 17 à 25 ml.
  La pression avant le départ de la pompe est la plus basse de la série
  (0,34 à 0,43 bar), et le plancher pendant le remplissage aussi
  (0,29 à 0,32 bar, contre 0,38 à 0,40 bar à 14 h 48). Le capteur est
  entre la pompe et la chaudière : pompe arrêtée, il lit la pression du
  circuit côté chaudière, pas l'entrée de la pompe. Le niveau bas du
  réservoir n'y pèse que d'environ 0,01 bar par 10 cm d'eau, dix fois
  moins que l'écart. Le débit de 3,65 ml/s montre une pompe qui n'est pas
  affamée. La cause du volume n'est pas identifiée.
- *Rendement, mesuré.* Plateau plus rapide (1,19 g/s) et arrêt à 21,5 g :
  1,3 g de gouttes, 22,8 g final.
- *Goût, mesuré.* La tasse la plus équilibrée de la série arrive à
  90,63 °C, sous 9 h 42 (90,89 °C, légèrement acide). La température en
  tasse n'explique donc pas seule l'acidité de 9 h 42, et l'ordre des
  trois infusions du 13 et du 14 par la NTC tasse ne tient plus.
- *Goût, hypothèse non testée.* Sur les quatre infusions à 92 °C et
  22 g, le goût suit le volume de pré-infusion : 4,9 ml à 9 h 42,
  légèrement acide ; 7,2 ml à 14 h 48, très bon ; 7,8 ml le 13 à
  13 h 29, impeccable ; 9,3 ml aujourd'hui, le plus équilibré. Le volume
  dépend de la goutte, que les réglages ne fixent pas, et varie avec la
  galette et la NTC : rien ne le sépare encore des autres causes.

**Décision** (David). Pas de changement matériel pour l'instant, donc
pas de sonde sur le groupe. Consigne de 92 °C, cible de 22 g, mouture,
chauffe et routine guidée conservées. Conclusion de David : à ce stade
d'optimisation, la différence entre deux tasses se fait au moulin et à
la préparation ; le montage de la machine a atteint son objectif.

**Attendus pour l'infusion suivante** (mêmes réglages, routine guidée) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 22,9 g final (gouttes de 0,7 à 1,3 g) |
| Remplissage | 3,5 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 17 à 26 ml ; marge positive ; un second volume au-dessus de 26 ml en ferait une tendance, à rapprocher du plancher de pression pendant le remplissage |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml ; entre les deux, fin vers 4,0 à 4,2 s sans séparation possible |
| Tasse | 90,6 à 92,0 °C ; modèle avec l'énergie SSR, plus l'écart attendu ci-dessous |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C. À préparation de 130 à 150 s, un écart entre −0,2 et +0,2 °C quelle que soit la durée sur le groupe confirme la préparation comme levier ; un écart de −0,4 °C ou moins avec 6 min ou plus sur le groupe et cette préparation relance le groupe |
| Goût | équilibré. Une acidité avec moins de ≈ 5 ml de pré-infusion soutiendrait la piste du volume de pré-infusion ; une acidité au-delà de ≈ 7 ml l'infirmerait |

### 15 octobre, 14 h 46 — `261015-144651`

**Contexte.** Première routine du guide à un seul rinçage (écran 0.4.12) :
la purge sans porte-filtre se fait avant, au bouton purge, et la session
ne l'enregistre pas. Réglages inchangés : consigne de 92 °C, cible de
22 g, mouture et chauffe identiques. Le guide s'ouvre **sous la consigne**
(NTC à 87,7 °C), comme le 14 à 14 h 48. Tasse de 125,5 g, pesée par le
guide. Observation de David : café **assez bon**, galette un peu
dégradée, **filet par sursauts sur la fin**.

La capture contient trois échantillons NTC marqués invalides, à 36,61,
36,67 et 41,05 s (âges de 258, 315 et 201 ms, valeurs tenues égales à
leurs voisins) : `analyze_hf_capture.py` refuse la capture. Les chiffres
viennent d'une copie où ils sont rendus valides.

**Routine** (clé `session` de la capture) :

| Étape | 15/10 14 h 46 | 15/10 9 h 08 | 14/10 14 h 48 |
| --- | --- | --- | --- |
| NTC à l'ouverture du guide | **87,7 °C** | 88,8 °C | 84,4 °C |
| Rinçage du porte-filtre | 5,00 s, 19,7 ml, 32,5 g ; NTC 88,7 → 88,9 °C | 5,00 s, 19,4 ml, 25,2 g ; 88,5 → 88,9 °C | 5,03 s, 19,7 ml, 27,2 g ; 87,1 → 87,4 °C |
| Retour à la consigne | 153,7 s ; minimum 82,3 °C ; résistance active 43,7 s | 103,8 s ; 85,5 °C ; 23,5 s | 268,7 s ; 82,2 °C ; 50,6 s |
| Porte-filtre en place à la consigne | 419,1 s | 170,3 s | 373,6 s |
| **Sur le groupe après le rinçage** | **573 s** | 274 s | 642 s |
| **Préparation** | **102,2 s** | 133,6 s | 70,2 s |
| Routine entière | 11 min 44 s | 7 min 51 s | 13 min 5 s |

- Le rinçage a lieu 3,3 °C sous la consigne, et le porte-filtre reste
  9,5 min sur le groupe : la routine ressemble à celle du 14 à 14 h 48,
  avec une préparation de 102 s au lieu de 70 s.
- La tasse revient 7,5 s avant **démarrer**. La balance n'a pas manqué.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,02 s | — | — |
| Remplissage | 8,02 s | 6,92 s | 24,1 ml (3,48 ml/s) | 0,50 → 0,81 bar |
| Pré-infusion | 14,94 s | 3,01 s | 6,4 ml | 0,81 → 1,23 bar |
| Infusion | 17,95 s | 18,53 s | 46,4 ml | 9,22 bar max |

- Pression avant le départ de la pompe : 0,41 à 0,50 bar (0,34 à
  0,43 bar le matin) ; plancher de 0,34 bar pendant le remplissage
  (0,29 bar le matin).
- Remplissage : débit médian sous 1 bar de **3,59 ml/s** à 75 %. Premier
  paquet au-dessus de 0,8 bar à 14,67 s (0,80 bar), passage en
  pré-infusion à 14,94 s, après **24,1 ml** : de retour dans la plage
  après les 26,1 ml du matin.
- Pré-infusion : finie sur la **goutte** à 3,01 s (firmware, 0,1 g à
  17,95 s). Le repère du rapport suit à 18,59 s, à 32,2 ml. Marge de
  remplissage de **8,1 ml** (10,4 ml le matin).
- Montée de 3,8 s jusqu'à 8 bar (2,9 s le matin, 3,9 s le 14).
- Depuis 8 bar : **14,8 s**, 1,21 g/s en tasse, 2,25 ml/s en amont (16,2 s,
  1,16 g/s et 2,26 ml/s le matin). Arrêt au poids à 21,5 g, 22,9 g final,
  soit **1,5 g de gouttes**, le plus de la série (1,3 g le matin).
- Fin d'écoulement : la tasse passe de 1,2–1,3 g/s à **2,1 g/s** de 34,5
  à 35,5 s, puis 1,8 g/s jusqu'à l'arrêt, à pression égale (8,85 à
  9,0 bar). Le matin, elle ne dépasse pas 1,5 g/s.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,96 ± 0,06 bar, pompe à
  68 %, 1,24 g/s, soit **7,2 bar·s/g** (7,6 le matin, 8,1 le 14).
- Sur la phase d'infusion, la tasse reçoit 0,46 g par ml admis. Après
  l'arrêt, la pression remonte à 9,28 bar.

**Thermique.**

| Repère | 15/10 14 h 46 | 15/10 9 h 08 | 14/10 14 h 48 |
| --- | ---: | ---: | ---: |
| NTC au départ | 92,11 °C, +0,11 °C | 91,89 °C, −0,11 °C | 91,95 °C, −0,05 °C |
| Pic avant l'infusion | 93,74 °C à 12,3 s, +1,74 °C | 93,13 °C à 11,5 s, +1,13 °C | 93,41 °C à 13,2 s, +1,41 °C |
| Creux au début de l'infusion | palier à 91,40 °C, de 19,6 à 21,1 s | 90,34 °C à 22,0 s | aucun : 92,14 °C à 18,1 s |
| Rebond, commande à 0 % | 91,65 °C à 24,3 s, puis descente à 90,84 °C à la reprise | 91,07 °C à 30,7 s | 92,48 °C à 22,9 s |
| **Tasse** | **90,76 °C**, −1,24 °C | 90,63 °C, −1,37 °C | 91,65 °C, −0,35 °C |
| Minimum pendant l'écoulement | 88,94 °C, à l'arrêt | 89,68 °C, à l'arrêt | 90,27 °C, à l'arrêt |
| Minimum total | 88,76 °C à 38,5 s | 88,83 °C à 42,9 s | 89,98 °C à 39,5 s |
| 20 s après l'arrêt | 91,73 °C | 91,74 °C | 91,98 °C |
| Commande à 0 % (remboursement) | 10,9 s | 11,2 s | 11,4 s |
| Appoint repris avant l'arrêt | 7,6 s | 7,8 s | 8,3 s |
| Pente de la NTC pendant l'appoint | **−0,31 °C/s** | −0,19 °C/s | −0,19 °C/s |

- Énergie, commande / SSR : précharge 8,46 / 8,30 kJ, remplissage 6,33 /
  **5,41** kJ, pré-infusion 2,45 / 2,31 kJ, infusion 5,73 / 5,45 kJ.
  Total fourni par le SSR : 21,48 kJ pour 76,8 ml admis à l'arrêt
  (23,19 kJ pour 80,9 ml le matin). Jusqu'au début de l'infusion, le SSR
  fournit 16,02 kJ pour 30,4 ml admis (17,66 kJ pour 35,4 ml le matin).
- Trou de commande au début du remplissage : 6,0 à 6,4 % de 8,39 à
  9,34 s ; le SSR reste coupé de 8,53 à 10,47 s, **≈ 1,9 s**, comme le
  matin (≈ 2,0 s). Le remplissage perd 0,92 kJ entre commande et SSR.
- Pendant les 10,9 s à 0 %, 27,8 ml entrent dans la chaudière.
- Appoint, de sa reprise à l'arrêt : commande moyenne de 63 %, SSR actif
  56 % du temps.
- La tasse reçoit 11,5 g avant la reprise de l'appoint, NTC moyenne
  91,4 °C, et 10,0 g après, NTC moyenne 90,0 °C (11,7 g à 90,8 °C et
  9,8 g à 90,4 °C le matin).
- Modèle : 91,22 °C en tasse avec la commande réelle, soit **+0,46 °C**
  sur la mesure ; 90,71 °C avec l'énergie SSR, soit **−0,06 °C** (−0,06
  le matin, −0,50 le 14 à 14 h 48) ; 90,43 °C avec un écart d'eau froide
  de 72,5 K.

**Attendus vérifiés** (entrée du 15 octobre, 9 h 08).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | 7,2 bar·s/g, 14,8 s : atteint, en bas de plage |
| Rendement | 22,3 à 22,9 g final (gouttes de 0,7 à 1,3 g) | 22,9 g : atteint, en limite ; **1,5 g de gouttes : non atteint** |
| Remplissage | 3,5 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 17 à 26 ml ; marge positive | 3,59 ml/s, 6,92 s, 24,1 ml, marge de 8,1 ml : atteint ; pas de second volume au-dessus de 26 ml |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml | goutte à 8,1 ml : atteint |
| Tasse | 90,6 à 92,0 °C ; modèle avec l'énergie SSR plus l'écart attendu | 90,76 °C : atteint ; modèle à 90,71 °C |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C ; règles de lecture à préparation de 130 à 150 s | −0,06 °C : atteint ; préparation de 102 s, hors des règles |
| Goût | une acidité sous ≈ 5 ml de pré-infusion soutiendrait la piste du volume ; au-delà de ≈ 7 ml, l'infirmerait | assez bon, sans acidité relevée, à 6,4 ml : ni soutenue ni infirmée |

**Lecture.**

- *Chauffe, mesuré.* Le SSR coupé ≈ 1,9 s au début du remplissage est
  le cas courant, pas l'exception : 1,7 à 2,0 s sur presque toutes les
  infusions depuis le 10 octobre ; le 14 (1,0 et 0,4 s) faisait exception.
  Cause relevée dans le code de l'écran : pendant les 10 premières
  impulsions d'un écoulement, le débit est calculé depuis une impulsion de
  l'écoulement précédent, avec un horodatage sur 16 bits qui reboucle
  toutes les 65,5 s. Le débit lu tombe au hasard entre 0,04 et 60 ml/s :
  bas, la commande descend à 5–6 % pendant ≈ 0,9 s ; haut, elle reste au
  plafond. Une fois éteint, le SSR ne se rallume pas avant la fenêtre de
  1 s suivante du module capteurs, d'où ≈ 1 s de plus.
- *Chauffe, simulé.* Le SSR tenu à 90 % pendant ces ≈ 2 s fournit
  1,94 kJ de plus et donne **+1,02 °C** en tasse (+1,11 °C sur la capture
  du matin). Ce trou est une source de variation d'environ 1 °C d'une
  infusion à l'autre, tirée au hasard.
- *Chauffe, mesuré.* La NTC descend à −0,31 °C/s pendant l'appoint (−0,19
  le matin) et l'infusion est plus rapide (7,2 bar·s/g) : les 10,0 g de
  fin arrivent à 90,0 °C.
- *Porte-filtre, mesuré.* 573 s sur le groupe, presque autant que les
  642 s du 14 à 14 h 48, ne donnent qu'un écart de −0,06 °C, contre
  −0,50 °C ce jour-là. Sur quatre routines, l'écart ne suit pas la durée
  sur le groupe (148 s : +0,06 ; 274 s : −0,06 ; 573 s : −0,06 ; 642 s :
  −0,50). Il reste ordonné par la préparation (143 s : +0,06 ; 134 s :
  −0,06 ; 102 s : −0,06 ; 70 s : −0,50), mais seul 14 h 48 s'écarte.
- *Porte-filtre, hypothèse.* Un temps minimal sur le groupe perd son
  appui : au-delà de ≈ 2,5 min, rien ne bouge. Restent deux lectures,
  non séparées : une préparation très courte (≈ 1 min) laisse le
  porte-filtre plus chaud, ou 14 h 48 est un écart isolé. La durée de
  préparation n'est pas seule en jeu : la façon de sécher le porte-filtre
  compte aussi, et ni l'une ni l'autre n'est contrôlée. Pour la règle
  de [wizard.md](wizard.md#piste--temps-minimal-sur-le-groupe), la
  capture rejoint le groupe de 6 min ou plus, qui compte deux captures
  (−0,50 et −0,06 °C) contre une à 4 min ou moins (+0,06 °C) : les
  groupes se chevauchent.
- *Remplissage, mesuré.* 24,1 ml avec une pression avant la pompe de
  0,41 à 0,50 bar : le volume du matin ne se répète pas.
- *Galette, mesuré.* Plateau le plus rapide de la série à 92 °C
  (1,24 g/s), poussée à 2,1 g/s sur la fin à pression constante et 1,5 g
  de gouttes : la résistance cède en fin d'infusion, ce qui rejoint la
  galette dégradée et les sursauts du filet vus par David.

**Décision** (David). Réglages inchangés. La préparation (durée,
séchage) reste libre : on n'est pas au laboratoire, et elle ne devient
pas un levier. Trou de chauffe du remplissage : corrigé dans l'écran
0.4.13 (débit ancré sur l'écoulement en cours, 90 % jusqu'à ce qu'il soit
lu). Échantillons NTC invalides : les outils d'analyse acceptent une
lecture vieille de 200 ms à 1 s, sans copie de la capture.

**Attendus pour l'infusion suivante** (mêmes réglages, routine guidée) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) ; plus de gouttes avec un plateau plus rapide |
| Remplissage | 3,5 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 17 à 26 ml ; marge positive |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml |
| Tasse | avec 0.4.13, 91,2 à 92,2 °C, vers 91,7 °C (+1 °C en simulation sans trou de chauffe) ; avec 0.4.12, 90,6 à 92,0 °C ; modèle avec l'énergie SSR à ± 0,2 °C si la préparation dépasse ≈ 100 s |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C ; avec 0.4.13, commande à 90 % sans interruption de la fin de la précharge à la fin du remplissage, SSR jamais coupé plus de 0,2 s, et ≈ 1,9 kJ de plus au SSR sur le remplissage (≈ 7,3 kJ) |
| Goût | une acidité sous ≈ 5 ml de pré-infusion soutiendrait la piste du volume ; au-delà de ≈ 7 ml, l'infirmerait |

### 16 octobre, 9 h 34 — `261016-093425`

**Contexte.** Première infusion de l'écran 0.4.13 : débit ancré sur
l'écoulement en cours, chauffe à 90 % jusqu'à ce qu'il soit lu. Réglages
inchangés : consigne de 92 °C, cible de 22 g, mouture identique. Guide à
un rinçage, ouvert sous la consigne (NTC à 84,0 °C). Observation de
David : goût **bon dans l'ensemble**, mais la température **monte jusqu'à
97,2 °C après l'infusion**, ce qui le surprend.

**Routine** (clé `session` de la capture) :

| Étape | 16/10 9 h 34 | 15/10 14 h 46 |
| --- | --- | --- |
| NTC à l'ouverture du guide | **84,0 °C** | 87,7 °C |
| Rinçage du porte-filtre | 5,05 s, 18,3 ml, 24,5 g ; NTC 88,1 °C | 5,00 s, 19,7 ml, 32,5 g |
| Retour à la consigne | 207,0 s ; minimum 82,5 °C ; résistance active 54,7 s | 153,7 s ; 82,3 °C ; 43,7 s |
| Porte-filtre en place à la consigne | 601,1 s | 419,1 s |
| **Sur le groupe après le rinçage** | **808 s** | 573 s |
| **Préparation** | **92,1 s** | 102,2 s |

- La balance a manqué 17,9 s au total pendant la routine
  (`scale_lost_ms`), sans effet sur les étapes ni sur la pesée de
  l'infusion.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,02 s | — | — |
| Remplissage | 8,02 s | 6,61 s | 22,9 ml (3,46 ml/s) | 0,47 → 0,96 bar |
| Pré-infusion | 14,63 s | 1,99 s | 5,2 ml | 0,96 → 1,18 bar |
| Infusion | 16,62 s | 18,28 s | 43,5 ml | 9,15 bar max |

- Remplissage à 3,61 ml/s à 75 %, fin sur la pression : 0,87 bar au
  dernier paquet, passage à 0,96 bar.
- Pré-infusion finie sur la **goutte** à 1,99 s. Première goutte
  (0,3 g) à 28,1 ml : marge de remplissage de **5,2 ml** (6,4 ml la
  veille).
- Montée de 3,7 s jusqu'à 8 bar (3,8 s la veille).
- Depuis 8 bar : **14,7 s**, 1,21 g/s en tasse, 2,17 ml/s en amont (14,8 s,
  1,21 g/s et 2,25 ml/s la veille). Arrêt au poids à 21,5 g, 22,5 g
  final, 1,1 g de gouttes.
- Plateau (de 8 bar + 2 s à l'arrêt − 0,5 s) : 8,86 ± 0,09 bar, pompe à
  66 %, 1,22 g/s, soit **7,2 bar·s/g**, comme la veille. Fin d'écoulement
  à 1,46 g/s de 33 à 35 s, sans la poussée à 2,1 g/s de la veille.
- Après l'arrêt, la pression remonte à 9,37 bar.

**Thermique.**

| Repère | 16/10 9 h 34 | 15/10 14 h 46 |
| --- | ---: | ---: |
| NTC au départ | 92,10 °C, +0,10 °C | 92,11 °C, +0,11 °C |
| Pic avant l'infusion | 93,83 °C à 12,2 s, +1,83 °C | 93,74 °C à 12,3 s |
| Minimum pendant l'infusion | 91,17 °C à 20,8 s | 88,94 °C, à l'arrêt |
| **Commande à 0 % (remboursement)** | **aucune** | 10,9 s |
| **Tasse** | **93,16 °C**, +1,16 °C | 90,76 °C, −1,24 °C |
| NTC à l'arrêt | **95,71 °C**, en montée depuis 20,8 s | 88,94 °C |
| 20 s après l'arrêt | **99,55 °C** | 91,73 °C |
| Maximum après l'arrêt | **99,61 °C** à 52,4 s, 17,4 s après l'arrêt | — |

- Énergie, commande / SSR : précharge 8,51 / 8,33 kJ, remplissage 7,09 /
  **7,50** kJ, pré-infusion 1,54 / 1,56 kJ, infusion **13,76 / 12,40**
  kJ (5,73 / 5,45 la veille).
- Remplissage : commande à 90 % de bout en bout, sauf un pas à 84,3 % à
  9,1 s (débit déjà lu à 3,32 ml/s). Le SSR est coupé au plus 0,19 s
  (9,11 à 9,30 s).
- **Remise à zéro du régulateur** à 15,06 s, en pré-infusion : la lecture
  NTC a 204 ms d'âge, au-delà du seuil de fraîcheur de 200 ms (2 × la
  période de 100 ms), et le pas thermique de 250 ms tombe dessus. Le
  régulateur tient la mesure pour invalide et appelle `reset()`, qui
  efface la loi d'infusion et sa dette de précharge (712 %·s). La commande
  passe à 0 % de 15,06 à 15,29 s, puis la loi de débit reprend **sans
  dette** : l'infusion reçoit tout son appoint.
- 97,2 °C est atteint à 42,6 s, 7,6 s après l'arrêt ; la NTC monte
  ensuite jusqu'à 99,61 °C.
- Modèle : 92,35 °C en tasse avec la commande réelle, 92,38 °C avec
  l'énergie SSR, soit **−0,76 °C** sous la mesure.

**Attendus vérifiés** (entrée du 15 octobre, 14 h 46).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | 7,2 bar·s/g, 14,7 s : atteint |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) | 22,5 g, 1,1 g de gouttes : atteint |
| Remplissage | 3,5 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 17 à 26 ml ; marge positive | 3,61 ml/s, 6,61 s, 22,9 ml, marge de 5,2 ml : atteint |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge | goutte à 5,2 ml : atteint |
| Tasse | avec 0.4.13, 91,2 à 92,2 °C, vers 91,7 °C | **93,16 °C : non atteint**, remboursement absent |
| Chauffe | modèle avec l'énergie SSR entre −1,2 et +0,5 °C ; commande à 90 % de la précharge à la fin du remplissage, SSR jamais coupé plus de 0,2 s, ≈ 7,3 kJ au SSR sur le remplissage | −0,76 °C : atteint ; 90 % sauf un pas à 84 %, SSR coupé 0,19 s au plus, 7,50 kJ : atteint |
| Goût | acidité sous ≈ 5 ml de pré-infusion pour soutenir la piste du volume | bon, sans acidité relevée, à 5,2 ml : ni soutenue ni infirmée |

**Lecture.**

- *Chauffe, mesuré.* La correction 0.4.13 fait ce qu'elle doit : plus de
  trou au début du remplissage, 2,1 kJ de plus au SSR que la veille.
- *Dépassement, mesuré.* Il ne vient pas de 0.4.13 mais de la dette
  effacée : l'infusion reçoit 12,40 kJ au SSR au lieu de ≈ 5,5 kJ, soit
  ≈ 7 kJ de trop, ≈ 4,7 °C dans la chaudière (C = 1,47 kJ/K). La NTC
  monte dès 20,8 s et continue 17 s après l'arrêt, la chaleur commandée
  mettant ≈ 11 s à la rejoindre.
- *Dépassement, simulé.* Avec la dette rendue sur la même capture (0 %
  pendant 11,1 s dès l'infusion, 7,56 kJ de moins au SSR), la tasse perd
  0,69 °C (énergie SSR) à 0,79 °C (commande), et le maximum après l'arrêt
  passe de ≈ 98–99 °C à ≈ 93 °C. Avec l'écart du modèle de ce jour, la
  tasse aurait été vers **92,4 °C**, au-dessus de la plage attendue.
- *Lectures NTC tardives, mesuré.* Le 14 à 14 h 48 (18,10 s, 200 ms) et le
  15 à 9 h 08 (20,05 s, 211 ms), une lecture invalide tombait déjà en
  pleine infusion. Le pas thermique ne l'a pas vue (la commande reste à
  0 %, le remboursement va à son terme). Trois des quatre dernières
  captures ont une lecture tardive pendant l'écoulement ; seule sa
  coïncidence avec un pas thermique, avant la fin du remboursement,
  efface la dette.

**Décision** (David). Option 1 : pendant la précharge et l'écoulement,
une lecture NTC manquante d'au plus 1 s coupe la chauffe pour ces pas sans
remettre la loi d'infusion à zéro ; la dette est conservée, et ces pas
n'y ajoutent rien. Au-delà de 1 s, la remise à zéro reste. Écran 0.4.14.

**Attendus pour l'infusion suivante** (0.4.14, mêmes réglages, routine
guidée) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) |
| Remplissage | 3,5 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 17 à 26 ml ; marge positive |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml |
| Chauffe | commande à 90 % de la précharge à la fin du remplissage ; **0 % pendant 10,5 à 12,5 s dès l'infusion**, même si une lecture NTC invalide tombe dans la capture ; ≈ 5,5 kJ au SSR sur l'infusion ; modèle avec l'énergie SSR entre −1,2 et +0,5 °C |
| Tasse | 91,2 à 92,4 °C ; maximum après l'arrêt sous ≈ 94 °C |
| Goût | une acidité sous ≈ 5 ml de pré-infusion soutiendrait la piste du volume ; au-delà de ≈ 7 ml, l'infirmerait |

### 16 octobre, 14 h 10 — `261016-141025`

**Contexte.** Seconde infusion de l'écran 0.4.13, avant le passage à
0.4.14. Réglages inchangés. Guide à un rinçage, ouvert sous la consigne
(NTC à 87,2 °C). Observation de David : goût **impeccable**, il est
très content du résultat. Les variations de goût viennent maintenant du
moulin : le café du matin est un peu plus amer, la rétention du moulin
gardant de la mouture de la veille.

**Routine** : rinçage de 5,03 s, 19,7 ml, 31,3 g (NTC 89,1 → 88,7 °C) ;
retour à la consigne en 321,4 s (minimum 82,5 °C, résistance active
47,4 s) ; porte-filtre en place 228,6 s à la consigne, **550 s sur le
groupe** après le rinçage ; **préparation de 93,5 s**. La balance a
manqué 20,1 s au total pendant la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | — |
| Remplissage | 8,01 s | 6,63 s | 23,2 ml (3,50 ml/s) | 0,48 → 0,92 bar |
| Pré-infusion | 14,63 s | 1,71 s | 4,1 ml | 0,92 → 1,11 bar |
| Infusion | 16,35 s | 17,46 s | 42,9 ml | 9,09 bar max |

- Pré-infusion finie sur la goutte ; première goutte (0,3 g) à 29,0 ml,
  marge de **5,8 ml**.
- Montée de 4,3 s jusqu'à 8 bar.
- Depuis 8 bar : **13,2 s**, 1,31 g/s en tasse, 2,14 ml/s en amont.
  22,7 g final, 1,3 g de gouttes.
- Plateau : 8,82 bar, pompe à 66 %, 1,28 g/s, soit **6,9 bar·s/g**, le
  plus rapide depuis le 11 octobre.

**Thermique.**

| Repère | 16/10 14 h 10 | 16/10 9 h 34 | 15/10 14 h 46 |
| --- | ---: | ---: | ---: |
| NTC au départ | 92,13 °C | 92,10 °C | 92,11 °C |
| Pic avant l'infusion | 93,75 °C à 12,5 s | 93,83 °C | 93,74 °C |
| Commande à 0 % (remboursement) | **10,3 s** | aucune | 10,9 s |
| Appoint repris avant l'arrêt | 6,9 s | — | 7,4 s |
| Pente de la NTC pendant l'appoint | −0,17 °C/s | — | −0,31 °C/s |
| **Tasse** | **90,77 °C**, −1,23 °C | 93,16 °C | 90,76 °C |
| Minimum | 89,25 °C, à l'arrêt | 91,17 °C | 88,76 °C |
| 20 s après l'arrêt | 91,14 °C | 99,55 °C | 91,73 °C |

- Énergie, commande / SSR : précharge 8,44 / 8,47 kJ, remplissage 7,15 /
  **7,28** kJ, pré-infusion 1,54 / 1,52 kJ, infusion 4,85 / 4,55 kJ.
  Total SSR jusqu'à l'arrêt : 21,82 kJ (21,48 kJ la veille).
- Remplissage : commande à 90 %, SSR coupé au plus 0,20 s.
- Une lecture NTC de 205 ms à 16,75 s, en début de remboursement : le pas
  thermique ne tombe pas dessus, le remboursement va à son terme.
- Modèle : 91,35 °C avec la commande réelle, 91,27 °C avec l'énergie SSR,
  soit **−0,50 °C** sous la mesure (−0,06 °C la veille).

**Attendus vérifiés** (entrée du 16 octobre, 9 h 34 ; ceux du remboursement
visent 0.4.14, mais s'appliquent dès qu'aucune dette n'est effacée).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | 6,9 bar·s/g, 13,2 s : en limite basse |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) | 22,7 g, 1,3 g : atteint |
| Remplissage | 3,5 à 3,7 ml/s ; 5 à 7,5 s et 17 à 26 ml ; marge positive | 3,50 ml/s, 6,63 s, 23,2 ml, 5,8 ml : atteint |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge | goutte : atteint |
| Chauffe | 90 % sur le remplissage ; 0 % pendant 10,5 à 12,5 s ; ≈ 5,5 kJ au SSR sur l'infusion ; modèle entre −1,2 et +0,5 °C | 90 % : atteint ; **10,3 s, en limite** ; 4,55 kJ, infusion plus courte ; −0,50 °C : atteint |
| Tasse | 91,2 à 92,4 °C ; maximum après l'arrêt sous ≈ 94 °C | **90,77 °C : non atteint** ; 91,16 °C : atteint |
| Goût | acidité sous ≈ 5 ml de pré-infusion pour soutenir la piste du volume | impeccable, sans acidité, à 4,1 ml : **infirme** la piste du volume |

**Lecture.**

- *Chauffe, mesuré.* Le remplissage reçoit 1,87 kJ de plus que la veille
  à 14 h 46, comme prévu. Mais la tasse est identique (90,77 contre
  90,76 °C) : le gain de +1 °C de la simulation ne se voit pas.
- *Chauffe, simulé.* Le modèle avec l'énergie SSR place cette infusion
  0,56 °C au-dessus de celle de la veille (91,27 contre 90,71 °C) ; la
  mesure ne bouge pas. L'écart au modèle passe de −0,06 à −0,50 °C.
  L'infusion plus courte (17,5 s contre 18,5 s, 6,9 bar·s/g) est déjà dans
  le modèle ; elle n'explique pas l'écart.
- *Porte-filtre, hypothèse.* −0,50 °C, c'est l'écart du 14 à 14 h 48
  (642 s sur le groupe, 70 s de préparation). Ici, 550 s et 93 s : la
  préparation ne sépare plus les deux groupes. Non testé.
- *Une seule capture.* L'effet de 0.4.13 sur la tasse reste à confirmer
  sur plusieurs infusions ; celle-ci ne le soutient pas. À 90,77 °C, le
  goût est impeccable : la tasse n'a pas besoin du degré attendu.
- *Goût, observé par David.* La chauffe n'est plus la source des écarts
  de goût. Le café du matin, un peu plus amer, sort avec la rétention de
  la veille dans le moulin : une variable hors machine, que ni la capture
  ni le modèle ne voient.

**Décision** (David). Passage à l'écran 0.4.14 (lecture NTC manquante
d'au plus 1 s sans remise à zéro de la loi d'infusion). Réglages
inchangés.

**Attendus pour l'infusion suivante** (0.4.14, mêmes réglages, routine
guidée) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) |
| Remplissage | 3,5 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 17 à 26 ml ; marge positive |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml |
| Chauffe | 90 % sur le remplissage ; **0 % pendant 10 à 12,5 s dès l'infusion**, même avec une lecture NTC invalide dans la capture ; modèle avec l'énergie SSR entre −1,2 et +0,5 °C |
| Tasse | 90,6 à 92,4 °C (élargi vers le bas : le gain de 0.4.13 n'est pas confirmé) ; maximum après l'arrêt sous ≈ 94 °C |
| Goût | impeccable à mouture égale ; noter si le moulin a servi le jour même (rétention de la veille le matin, café un peu plus amer) |

### 17 octobre, 10 h 36 — `261017-103625`

**Contexte.** Première infusion de l'écran 0.4.14 (lecture NTC manquante
d'au plus 1 s sans remise à zéro de la loi d'infusion). Réglages
inchangés. Guide à un rinçage, ouvert sous la consigne (NTC à 89,9 °C).
Observation de David : il est satisfait du goût, même s'il y a eu **un
peu de channelling** pendant l'infusion.

**Routine** : rinçage de 5,02 s, 19,1 ml, 27,2 g (NTC 91,6 → 90,9 °C) ;
retour à la consigne en 102,1 s (minimum 85,2 °C, résistance active
23,6 s) ; porte-filtre en place 236,8 s à la consigne, **339 s sur le
groupe** après le rinçage ; **préparation de 78,5 s**. La balance a
manqué 31,3 s au total pendant la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | — |
| Remplissage | 8,01 s | 7,24 s | 24,9 ml (3,44 ml/s) | 0,38 → 0,85 bar |
| Pré-infusion | 15,25 s | 3,10 s | 6,7 ml | 0,85 → 1,25 bar |
| Infusion | 18,35 s | 17,02 s | 44,9 ml | 9,16 bar max |

- Pré-infusion finie sur la goutte ; première goutte (0,3 g) à 33,0 ml,
  marge de **8,1 ml**.
- Montée de 3,7 s jusqu'à 8 bar.
- Depuis 8 bar : **13,4 s**, 1,32 g/s en tasse, **2,40 ml/s** en amont,
  le débit amont le plus élevé de la synthèse. 21,6 g à l'arrêt, 22,9 g
  final, 1,3 g de gouttes.
- Plateau : 8,98 bar, pompe à 71 %, 1,37 g/s, soit **6,6 bar·s/g**, le
  plus rapide depuis le 11 octobre à 13 h 40 (6,2).
- Pression tenue entre 8,86 et 9,05 bar après le dépassement à 9,16 bar
  à 22,5 s : aucune chute dans la capture.

**Thermique.**

| Repère | 17/10 10 h 36 | 16/10 14 h 10 | 15/10 14 h 46 |
| --- | ---: | ---: | ---: |
| NTC au départ | 92,06 °C | 92,13 °C | 92,11 °C |
| Pic avant l'infusion | 93,14 °C à 11,3 s | 93,75 °C à 12,5 s | 93,74 °C |
| NTC au début de l'infusion | 92,62 °C | 92,23 °C | 91,65 °C |
| Commande à 0 % (remboursement) | **10,6 s** | 10,3 s | 10,9 s |
| Appoint repris avant l'arrêt | 6,3 s | 6,9 s | 7,4 s |
| Pente de la NTC pendant l'appoint | −0,32 °C/s | −0,17 °C/s | −0,31 °C/s |
| **Tasse** | **92,42 °C**, +0,42 °C | 90,77 °C | 90,76 °C |
| Minimum | 89,87 °C, 3,8 s après l'arrêt | 89,25 °C, à l'arrêt | 88,76 °C |
| 20 s après l'arrêt | 92,63 °C, encore en hausse (+0,05 °C/s) | 91,14 °C | 91,73 °C |

- Énergie, commande / SSR : précharge 8,49 / 8,62 kJ, remplissage 7,76 /
  7,66 kJ, pré-infusion 2,44 / 2,11 kJ, infusion 5,21 / 5,17 kJ. Total
  SSR jusqu'à l'arrêt : 23,56 kJ (21,82 kJ la veille), dont 18,39 kJ
  avant l'infusion (17,27 kJ).
- Remplissage : commande à 90 %, SSR coupé au plus 0,17 s.
- Pendant le remboursement, la NTC **monte** de 92,62 à 93,25 °C (24,0 s),
  puis redescend ; la veille, elle descendait dès le début de l'infusion.
- Deux lectures NTC invalides : 222 ms à 0,20 s (précharge), 234 ms à
  15,40 s (pré-infusion). Sur les deux échantillons, la commande reste à
  sa valeur (90 %, puis 89,3 %) : aucun pas thermique ne tombe dessus. Le
  16 à 9 h 34, le pas tombait sur la lecture (commande à 0 %) et la dette
  était effacée. **La tolérance de 0.4.14 n'a pas encore servi** : la même
  infusion aurait eu la même chauffe en 0.4.13.
- Modèle : 91,80 °C avec la commande réelle, 91,73 °C avec l'énergie SSR,
  soit **+0,71 °C** au-dessus de la mesure (−0,51 °C la veille).

**Attendus vérifiés** (entrée du 16 octobre, 14 h 10).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 7,0 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | **6,6 bar·s/g : non atteint** ; 13,4 s : atteint |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) | 22,9 g, 1,3 g : atteint |
| Remplissage | 3,5 à 3,7 ml/s ; 5 à 7,5 s et 17 à 26 ml ; marge positive | 3,44 ml/s, en limite basse ; 7,24 s, 24,9 ml, 8,1 ml : atteint |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml | goutte à 8,1 ml : atteint |
| Chauffe | 90 % sur le remplissage ; 0 % pendant 10 à 12,5 s, même avec une lecture NTC invalide ; modèle avec l'énergie SSR entre −1,2 et +0,5 °C | 90 % : atteint ; 10,6 s : atteint, mais aucune lecture invalide sur un pas : **non vérifié** ; **+0,71 °C : non atteint** |
| Tasse | 90,6 à 92,4 °C ; maximum après l'arrêt sous ≈ 94 °C | 92,42 °C : en limite haute ; 92,63 °C en fin de capture, montée ralentie : atteint |
| Goût | impeccable à mouture égale ; noter si le moulin a servi le jour même | satisfaisant malgré un peu de channelling ; usage du moulin non noté |

**Lecture.**

- *Chauffe, mesuré.* Même NTC au départ, même loi, même durée à 0 % que la
  veille, et pourtant **+1,65 °C** en tasse. Le SSR donne 1,12 kJ de plus
  avant l'infusion, dont 0,59 kJ sur une pré-infusion plus longue (3,10
  contre 1,71 s, toujours à ≈ 90 %).
- *Chauffe, simulé.* Le modèle avec l'énergie SSR ne place cette infusion
  que 0,46 °C au-dessus de celle de la veille (91,73 contre 91,27 °C) ;
  ≈ 1,2 °C de l'écart mesuré n'est pas dans le modèle. L'écart au modèle
  passe de −0,51 à +0,71 °C, proche du 16 à 9 h 34 (+0,76 °C, dette
  effacée).
- *Porte-filtre, hypothèse.* 339 s sur le groupe, le plus court depuis le
  15 à 9 h 08. Écart mesure − modèle (énergie SSR) selon le temps sur le
  groupe : 274 s +0,06 °C, 339 s +0,71 °C, 374 s +0,50 °C, 550 s
  −0,51 °C, 573 s +0,06 °C. La tendance va dans le sens d'un porte-filtre
  plus chaud quand il reste moins longtemps, mais elle n'est pas
  monotone. Non testé.
- *Machine avant le guide, hypothèse.* NTC à 89,9 °C à l'ouverture du
  guide, la plus haute des routines guidées (84,0 à 88,8 °C avant), retour
  à la consigne en 102 s avec 23,6 s de résistance (321 s et 47,4 s la
  veille). Un groupe ou une chaudière plus chauds en dehors de la NTC
  donneraient une eau plus chaude à commande égale ; la capture ne le
  mesure pas. Non testé.
- *Channelling, estimé.* David a vu un peu de channelling. La capture
  montre une galette moins résistante : 6,6 bar·s/g, la pompe à 71 % pour
  tenir 8,98 bar (66 % pour 8,82 bar la veille), 2,40 ml/s en amont. Elle
  ne montre ni chute de pression ni saut de débit : elle ne distingue pas
  un canal d'une galette simplement plus perméable.
- *Goût, observé par David.* Satisfaisant à 92,42 °C malgré le
  channelling.

**Décision.** Aucun changement annoncé par David : écran 0.4.14, réglages
inchangés.

**Attendus pour l'infusion suivante** (0.4.14, mêmes réglages, routine
guidée) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 6,5 à 8,5 bar·s/g (élargi vers le bas après 6,6) ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) |
| Remplissage | 3,4 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 17 à 26 ml ; marge positive |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml |
| Chauffe | 90 % sur le remplissage ; 0 % pendant 10 à 12,5 s dès l'infusion ; si une lecture NTC invalide tombe sur un pas pendant la précharge ou l'écoulement : 0 % sur ce pas, loi et dette conservées ; modèle avec l'énergie SSR entre −1,2 et +0,8 °C |
| Tasse | 90,6 à 92,6 °C ; maximum après l'arrêt sous ≈ 94 °C |
| Goût | noter le channelling vu et si le moulin a servi le jour même ; un channelling sous ≈ 7 bar·s/g soutiendrait la lecture d'une galette moins résistante |

### 17 octobre, 13 h 57 — `261017-135720`

**Contexte.** Seconde infusion de l'écran 0.4.14. Réglages inchangés.
Guide à un rinçage, ouvert sous la consigne (NTC à 89,8 °C). Observation
de David : café **clairement acide**, sans channelling ; le moulin avait
servi le matin, il ne gardait donc pas de mouture de la veille.

**Routine** : rinçage de 5,04 s, 19,4 ml, 28,4 g (NTC 90,7 → 90,1 °C) ;
retour à la consigne en 136,9 s (minimum 84,7 °C, résistance active
21,1 s) ; porte-filtre retiré **3,6 s** après la consigne, soit **140 s
sur le groupe** après le rinçage, la durée la plus courte des routines
guidées ; **préparation de 139,4 s**. La balance a manqué 19,0 s au total
pendant la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,09 s | — | — |
| Remplissage | 8,09 s | **8,24 s** | **28,4 ml** (3,45 ml/s) | 0,40 → 0,94 bar |
| Pré-infusion | 16,34 s | 3,07 s | 7,0 ml | 0,94 → 1,36 bar |
| Infusion | 19,41 s | 17,39 s | 45,2 ml | 9,03 bar max |

- Remplissage le plus long de la synthèse en volume (26,1 ml au plus
  jusqu'ici) : 1 bar n'est atteint qu'à 29,9 ml (27,8 ml le matin). La
  pression avant la pompe est normale (0,40 bar). Cause non identifiée.
- Pré-infusion finie sur la goutte ; première goutte (0,3 g) à 36,2 ml,
  marge de **7,8 ml**.
- Montée de 3,6 s jusqu'à 8 bar.
- Depuis 8 bar : **13,8 s**, 1,29 g/s en tasse, 2,41 ml/s en amont.
  21,5 g à l'arrêt, 22,7 g final, 1,2 g de gouttes.
- Plateau : 8,84 bar, pompe à 69 %, 1,29 g/s, soit **6,9 bar·s/g**, comme
  le 16 à 14 h 10 (impeccable).

**Thermique.**

| Repère | 17/10 13 h 57 | 17/10 10 h 36 | 16/10 14 h 10 |
| --- | ---: | ---: | ---: |
| NTC au départ | 91,75 °C | 92,06 °C | 92,13 °C |
| Pic avant l'infusion | 92,75 °C à 11,0 s | 93,14 °C à 11,3 s | 93,75 °C à 12,5 s |
| NTC au début de l'infusion | **90,25 °C** | 92,62 °C | 92,23 °C |
| Commande à 0 % (remboursement) | 10,7 s | 10,6 s | 10,3 s |
| Appoint repris avant l'arrêt | 6,5 s | 6,3 s | 6,9 s |
| Pente de la NTC pendant l'appoint | −0,22 °C/s | −0,32 °C/s | −0,17 °C/s |
| **Tasse** | **90,50 °C**, −1,50 °C | 92,42 °C | 90,77 °C |
| Minimum | 89,02 °C, 3,6 s après l'arrêt | 89,87 °C | 89,25 °C, à l'arrêt |
| 20 s après l'arrêt | 91,55 °C, encore en hausse | 92,63 °C | 91,14 °C |

- Énergie, commande / SSR : précharge 8,58 / 8,71 kJ, remplissage 8,77 /
  8,28 kJ, pré-infusion 2,47 / 2,25 kJ, infusion 5,38 / **4,68** kJ.
  Total SSR jusqu'à l'arrêt : 23,92 kJ, dont 19,24 kJ avant l'infusion
  (18,39 kJ le matin).
- Remplissage : commande à 90 %, SSR coupé au plus 0,21 s. La NTC y
  descend (91,88 → 91,33 °C) ; le matin, elle montait (92,31 → 92,47 °C).
- Eau admise avant l'infusion : 35,4 ml, contre 31,6 ml le matin.
- Deux lectures NTC invalides en pré-infusion : 206 ms à 16,85 s, 201 ms
  à 17,16 s. La commande reste à sa valeur sur les deux échantillons
  (78,7 % et 74,4 %) : aucun pas thermique ne tombe dessus. La tolérance
  de 0.4.14 n'a toujours pas servi.
- Modèle : 91,38 °C avec la commande réelle, 91,00 °C avec l'énergie SSR,
  soit **−0,50 °C** sous le modèle (+0,71 °C le matin).

**Attendus vérifiés** (entrée du 17 octobre, 10 h 36).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 6,5 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | 6,9 bar·s/g, 13,8 s : atteint |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) | 22,7 g, 1,2 g : atteint |
| Remplissage | 3,4 à 3,7 ml/s ; 5 à 7,5 s et 17 à 26 ml ; marge positive | 3,45 ml/s : atteint ; **8,24 s, 28,4 ml : non atteint** ; 7,8 ml : atteint |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge | goutte à 7,8 ml : atteint |
| Chauffe | 90 % sur le remplissage ; 0 % pendant 10 à 12,5 s ; lecture invalide sur un pas : 0 %, loi et dette conservées ; modèle avec l'énergie SSR entre −1,2 et +0,8 °C | 90 % : atteint ; 10,7 s : atteint ; aucune lecture sur un pas : **non vérifié** ; −0,50 °C : atteint |
| Tasse | 90,6 à 92,6 °C ; maximum après l'arrêt sous ≈ 94 °C | **90,50 °C : non atteint** de 0,10 °C ; 91,55 °C en fin de capture : atteint |
| Goût | noter le channelling et l'usage du moulin | **clairement acide** ; sans channelling ; moulin utilisé le matin |

**Lecture.**

- *Chauffe, mesuré.* −1,92 °C en tasse sur le matin, à galette proche
  (6,9 contre 6,6 bar·s/g). La NTC arrive à l'infusion 2,37 °C plus bas :
  départ plus bas de 0,31 °C, et 3,8 ml d'eau froide de plus avant
  l'infusion, venus du remplissage long.
- *Chauffe, simulé.* Le modèle avec l'énergie SSR place cette infusion
  0,73 °C sous celle du matin (91,00 contre 91,73 °C) ; la mesure en perd
  1,92. L'écart au modèle passe de +0,71 à −0,50 °C.
- *Température seule, mesuré.* 90,50 °C n'est que 0,13 à 0,27 °C sous
  trois infusions jugées bonnes : 90,63 °C (15/10 9 h 08, le plus
  équilibré), 90,76 °C (15/10 14 h 46), 90,77 °C (16/10 14 h 10,
  impeccable). La NTC tasse n'explique pas seule l'acidité.
- *Porte-filtre, hypothèse.* Les deux seules routines guidées avec
  l'acidité relevée sont aussi les deux plus courtes sur le groupe :

  | Routine | Sur le groupe | Préparation | Mesure − modèle (énergie SSR) | NTC tasse | Goût |
  | --- | ---: | ---: | ---: | ---: | --- |
  | 17/10 13 h 57 | **140 s** | 139 s | −0,50 °C | 90,50 °C | clairement acide |
  | 14/10 9 h 42 | **148 s** | 143 s | +0,06 °C | 90,89 °C | légèrement acide |
  | 15/10 9 h 08 | 274 s | 134 s | +0,06 °C | 90,63 °C | le plus équilibré |
  | 17/10 10 h 36 | 339 s | 79 s | +0,71 °C | 92,42 °C | satisfaisant, un peu de channelling |
  | 16/10 14 h 10 | 550 s | 93 s | −0,51 °C | 90,77 °C | impeccable |
  | 15/10 14 h 46 | 573 s | 102 s | +0,06 °C | 90,76 °C | assez bon |
  | 14/10 14 h 48 | 642 s | 70 s | +0,50 °C | 91,65 °C | très bon |
  | 16/10 9 h 34 | 808 s | 92 s | +0,76 °C | 93,16 °C | bon |

  À préparation égale (134 à 143 s), 274 s sur le groupe donne la tasse
  la plus équilibrée, 140 et 148 s une tasse acide. Un porte-filtre resté
  moins de ≈ 2,5 min sur le groupe serait plus froid que la NTC ne le
  dit ; la capture ne le mesure pas, et l'écart au modèle ne suit pas (−0,50
  et +0,06 °C). Deux captures seulement ; les deux sont aussi les plus
  longues préparations, à 5 à 9 s près de celle du 15 à 9 h 08. Non testé.
  Cette infusion et celle du matin sont sous 7,0 bar·s/g : elles n'entrent
  pas dans la règle de quantification de
  [wizard.md](wizard.md#piste--temps-minimal-sur-le-groupe).
- *Remplissage, mesuré.* 28,4 ml, 3,5 ml de plus que le matin, à débit
  égal : la galette met plus longtemps à monter à 0,8 bar. Une galette
  moins haute (dose, tassage) ou un panier plus froid sont possibles ;
  rien dans la capture ne les sépare. Non testé.
- *Moulin, observé par David.* Le moulin avait servi le matin : pas de
  rétention de la veille, la piste du moulin (café du matin plus amer,
  16 octobre) ne s'applique pas. Sans channelling, la galette n'explique
  pas non plus l'acidité.

**Décision.** Aucune annoncée par David : écran 0.4.14, réglages
inchangés.

**Attendus pour l'infusion suivante** (0.4.14, mêmes réglages, routine
guidée) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 6,5 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) |
| Remplissage | 3,4 à 3,7 ml/s à 75 % ; 5 à 8,5 s et 17 à 29 ml (élargi après 28,4 ml) ; marge positive |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml |
| Chauffe | 90 % sur le remplissage ; 0 % pendant 10 à 12,5 s dès l'infusion ; si une lecture NTC invalide tombe sur un pas pendant la précharge ou l'écoulement : 0 % sur ce pas, loi et dette conservées ; modèle avec l'énergie SSR entre −1,2 et +0,8 °C |
| Tasse | 90,4 à 92,6 °C ; maximum après l'arrêt sous ≈ 94 °C |
| Goût | une acidité après plus de ≈ 4,5 min sur le groupe infirmerait la piste du porte-filtre ; une tasse acide de nouveau sous ≈ 2,5 min la soutiendrait |

### 18 octobre, 12 h 40 — `261018-124031`

**Contexte.** Troisième infusion de l'écran 0.4.14. Réglages inchangés.
Premier essai d'un **porte-filtre à bec**, avec le même panier que le
porte-filtre sans fond. Le porte-filtre sans fond se pose à plat sur le
tapis de tassage ; celui à bec demande un support troué, et le tassage
peut être un peu moins fort (David). Guide à un rinçage, ouvert sous la
consigne (NTC à 86,9 °C). Observations de David : la pompe n'a tourné que
23 s ; café **légèrement plus acide**.

**Routine** : rinçage de 5,02 s, 19,1 ml, 24,4 g (NTC 89,9 → 89,6 °C) ;
retour à la consigne en 118,7 s (minimum 84,2 °C, résistance active
27,2 s) ; porte-filtre en place 124,6 s à la consigne, **243 s sur le
groupe** après le rinçage ; **préparation de 168,6 s**, la plus longue
des routines guidées. La balance a manqué 0,1 s pendant la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,01 s | — | — |
| Remplissage | 8,01 s | 7,30 s | 25,2 ml (3,46 ml/s) | 0,39 → 0,87 bar |
| Pré-infusion | 15,31 s | 3,79 s | 8,1 ml | 0,87 → 1,42 bar |
| Infusion | 19,09 s | **12,17 s** | 33,6 ml | 8,90 bar max |

- Remplissage dans la plage : 1 bar à 26,7 ml, en pré-infusion.
- Pré-infusion finie sur la goutte ; première goutte (0,3 g) à 33,3 ml,
  marge de **8,1 ml**.
- Montée de 3,9 s jusqu'à 8 bar, avec **6,7 g** déjà en tasse (3,5 g le
  17 à 13 h 57).
- Depuis 8 bar : **8,4 s**, 1,71 g/s en tasse, 2,53 ml/s en amont.
  21,6 g à l'arrêt, 23,6 g final, **2,0 g de gouttes**.
- Plateau : 8,72 bar, pompe à 72 %, 1,78 g/s, soit **4,9 bar·s/g**, la
  galette la moins résistante depuis le 10 octobre à 11 h 01 (5,9).
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 18/10 12 h 40 (bec) | **0,7 s** | 3,3 s | 5,9 s | 11,4 s |
  | 17/10 13 h 57 | 1,3 s | 4,5 s | 8,8 s | 16,3 s |
  | 17/10 10 h 36 | 1,7 s | 4,9 s | 8,9 s | 16,1 s |
  | 16/10 14 h 10 | 1,8 s | 4,7 s | 8,7 s | 16,3 s |

- Pompe en marche 23,3 s (28,7 s le 17 à 13 h 57) : la différence est
  entièrement dans l'infusion, 5,4 s depuis 8 bar.

**Thermique.**

| Repère | 18/10 12 h 40 | 17/10 13 h 57 | 17/10 10 h 36 |
| --- | ---: | ---: | ---: |
| NTC au départ | 91,83 °C | 91,75 °C | 92,06 °C |
| Pic avant l'infusion | 93,15 °C à 12,3 s | 92,75 °C à 11,0 s | 93,14 °C à 11,3 s |
| NTC au début de l'infusion | **88,36 °C** | 90,25 °C | 92,62 °C |
| Commande à 0 % (remboursement) | **9,8 s** | 10,7 s | 10,6 s |
| Appoint repris avant l'arrêt | 2,4 s | 6,5 s | 6,3 s |
| Pente de la NTC pendant l'appoint | −0,14 °C/s | −0,22 °C/s | −0,32 °C/s |
| **Tasse** | **90,44 °C**, −1,56 °C | 90,50 °C | 92,42 °C |
| Minimum | 88,36 °C, au début de l'infusion | 89,02 °C | 89,87 °C |
| 20 s après l'arrêt | 91,37 °C, encore en hausse | 91,55 °C | 92,63 °C |

- Énergie, commande / SSR : précharge 8,32 / 8,55 kJ, remplissage 7,79 /
  7,75 kJ, pré-infusion 2,82 / 2,33 kJ, infusion 1,90 / 1,70 kJ. Total
  SSR jusqu'à l'arrêt : 20,33 kJ, dont 18,63 kJ avant l'infusion
  (19,24 kJ la veille).
- Remplissage : commande à 90 %, SSR coupé moins de 0,3 s. NTC
  92,30 → 91,73 °C.
- Pré-infusion : la NTC perd **3,37 °C en 3,79 s** (91,73 → 88,36 °C),
  contre 1,08 °C en 3,07 s la veille. L'eau admise avant l'infusion est
  pourtant plus faible : 33,3 ml contre 35,4 ml.
- Cinq lectures NTC invalides : 229 ms à 0,21 s, 205 ms à 15,72 s,
  267 et 323 ms à 19,22–19,28 s, 216 ms à 31,68 s. Celle de 15,72 s tombe
  sur un pas de pré-infusion : **commande à 0 % sur ce pas**, puis reprise
  à 79,2 % sur la pente de la loi (88,1 % avant). Le remboursement a lieu
  ensuite en entier : la dette est conservée. Celle de 0,21 s est
  compatible avec un premier pas de précharge sauté (90 % à 0,32 s, contre
  0,15 s la veille ; précharge de 8,32 kJ contre 8,58 kJ). Celles de 19,2 s
  tombent pendant le 0 % du remboursement.
- Modèle : 91,80 °C avec la commande réelle, 91,83 °C avec l'énergie SSR,
  soit **−1,39 °C** sous le modèle (−0,50 °C la veille). L'écart se creuse
  pendant la pré-infusion : −1,25 °C à 16 s, **−3,05 °C** à 20 s, puis
  −0,62 °C à 30 s.

**Attendus vérifiés** (entrée du 17 octobre, 13 h 57).

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 6,5 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | **4,9 bar·s/g, 8,4 s : non atteint** |
| Rendement | 22,3 à 23,0 g final (gouttes de 0,7 à 1,5 g) | **23,6 g, 2,0 g : non atteint** |
| Remplissage | 3,4 à 3,7 ml/s à 75 % ; 5 à 8,5 s et 17 à 29 ml ; marge positive | 3,46 ml/s, 7,30 s, 25,2 ml, 8,1 ml : atteint |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml | goutte à 8,1 ml : atteint |
| Chauffe | 90 % sur le remplissage ; 0 % pendant 10 à 12,5 s dès l'infusion ; lecture invalide sur un pas : 0 %, loi et dette conservées ; modèle avec l'énergie SSR entre −1,2 et +0,8 °C | 90 % : atteint ; **9,8 s : non atteint** de 0,2 s ; lecture à 15,72 s : **atteint**, première vérification ; **−1,39 °C : non atteint** |
| Tasse | 90,4 à 92,6 °C ; maximum après l'arrêt sous ≈ 94 °C | 90,44 °C : atteint de justesse ; 91,37 °C en fin de capture : atteint |
| Goût | une acidité après plus de ≈ 4,5 min sur le groupe infirmerait la piste du porte-filtre ; une tasse acide de nouveau sous ≈ 2,5 min la soutiendrait | 243 s sur le groupe, entre les deux seuils, et porte-filtre changé : **non concluant** |

**Lecture.**

- *Durée de pompe, mesuré.* La pompe s'arrête au poids en tasse. Un bec
  qui retarde l'arrivée du café en tasse allongerait donc l'infusion. Ici,
  le café arrive **plus tôt** que les jours précédents (1 g à 0,7 s contre
  1,3 à 1,8 s), et la pré-infusion, finie sur la goutte, ne dure que 0,7 s
  de plus que la veille, à marge comparable (8,1 ml ; 9,3 ml le 15 à
  9 h 08). Le bec n'explique pas les 23 s. Elles viennent de la galette :
  1,78 g/s au plateau contre 1,30 g/s, à pression et pompe égales.
- *Bec, mesuré.* Le bec retient du café et le rend après l'arrêt : 2,0 g
  de gouttes contre 1,2 à 1,3 g avec le porte-filtre sans fond, d'où
  23,6 g final pour 21,6 g à l'arrêt.
- *Tassage, hypothèse de David.* Le panier étant le même, un tassage moins
  fort sur le support troué est la cause la plus probable. Les deux
  galettes les plus rapides avant celle-ci ont la même signature : 5,9
  bar·s/g et 9,1 s depuis 8 bar le 10 octobre à 11 h 01 (débit trop
  élevé), 6,2 bar·s/g et 11,9 s le 11 octobre à 13 h 40 (« tassage à
  travailler »). Au plateau, le rapport du débit en tasse au débit amont
  vaut 0,69 g/ml, comme ces deux infusions (0,77 et 0,73), contre 0,52 à
  0,58 du 13 au 17 octobre : il suit la perméabilité de la galette, il ne
  signale pas un défaut du débitmètre. Le tassage n'est pas mesuré ; non
  testé.
- *Goût, estimé.* 8,4 s depuis 8 bar, contre 13 à 16 s pour les tasses
  jugées bonnes : l'extraction courte suffit à expliquer l'acidité. La
  NTC tasse (90,44 °C) est dans la plage des tasses bonnes à impeccable
  (90,63 à 90,77 °C) : la température seule ne l'explique pas.
- *Porte-filtre sur le groupe, non concluant.* 243 s sur le groupe, entre
  les seuils de 2,5 et 4,5 min, avec un autre porte-filtre et une galette
  hors plage : cette infusion ne compte pas pour la piste.
- *Chute de la NTC en pré-infusion, mesuré.* 3,37 °C perdus en pré-infusion
  avec moins d'eau admise que la veille et une énergie SSR comparable :
  l'écart au modèle, −3,05 °C à 20 s, est le plus grand des routines
  guidées. Le modèle ne connaît que l'eau comptée par le débitmètre et la
  chaleur de la résistance ; la chute vient d'ailleurs. Hypothèse de
  David : le porte-filtre à bec, plus lourd, a une inertie thermique plus
  grande ; il tirerait plus de chaleur du groupe, d'autant plus après
  168,6 s de préparation. La capture ne mesure pas le porte-filtre. Les galettes
  rapides du 10 et du 11 octobre étaient aussi plus de 1 °C sous le modèle
  (modèle à +2,26 °C le 10, à +1,10 °C avec l'énergie SSR le 11) ; rien ne dit que la cause est la même. Non testé.
- *Remboursement, mesuré.* 9,8 s à 0 %, 0,5 s de moins que le plus court
  jusqu'ici (10,3 s le 16 à 14 h 10). Le remboursement suit le débitmètre :
  2,76 ml/s sur la phase d'infusion contre 2,60 ml/s la veille, et une
  dette un peu plus faible (précharge de 8,32 kJ contre 8,58 kJ). Il finit
  donc plus tôt, comme le prévoit la loi.

**Décision.** Aucune annoncée par David : écran 0.4.14, réglages
inchangés.

**Attendus pour l'infusion suivante** (0.4.14, mêmes réglages, routine
guidée, porte-filtre à bec tassé sur son support) :

| Point | Attendu |
| --- | --- |
| Hydraulique | 6,5 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar. Sous 6 bar·s/g avec un tassage soigné, le tassage ne suffirait plus à expliquer la galette rapide |
| Rendement | 21,6 g à l'arrêt ; ≈ 2 g de gouttes avec le bec, soit 23,0 à 24,0 g final (22,3 à 23,0 g sans fond) |
| Remplissage | 3,4 à 3,7 ml/s à 75 % ; 5 à 8,5 s et 17 à 29 ml ; marge positive |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml |
| Chauffe | 90 % sur le remplissage ; 0 % pendant 9,5 à 12,5 s dès l'infusion (élargi après 9,8 s à débit fort) ; lecture NTC invalide sur un pas : 0 % sur ce pas, loi et dette conservées ; modèle avec l'énergie SSR entre −1,2 et +0,8 °C. Une chute de la NTC de plus de 2 °C en pré-infusion avec le bec, à galette dans la plage, désignerait le porte-filtre |
| Tasse | 90,4 à 92,6 °C ; maximum après l'arrêt sous ≈ 94 °C |
| Goût | acidité absente à galette dans la plage : tassage ; acidité restée à galette dans la plage : chercher du côté du porte-filtre (masse, température) |

### 20 octobre, 14 h 29 — `261020-142940`

**Contexte.** Première infusion de l'écran 0.4.15 (lecture du SHT40 du
boîtier écran, loi de chauffe inchangée). Réglages inchangés. Deux
changements depuis le 18 octobre : **mouture plus fine** et **retour au
porte-filtre sans fond** ; rien dans la capture ne sépare leurs effets
sur la galette (voir la lecture). Guide à un rinçage, ouvert machine en
chauffe (NTC à **67,2 °C**). Observations de David : goût comme attendu,
**beaucoup plus acide, mais pas désagréable** ; après la fin de la
capture, la NTC a continué de monter, jusqu'à **94,1 °C**.

**Routine** : rinçage 100,7 s après l'ouverture du guide, de 5,02 s,
18,8 ml, 33,3 g (NTC 87,7 → 87,5 °C, sous la consigne) ; retour à la
consigne en 498,3 s (minimum 80,5 °C, résistance active 71,0 s) ;
porte-filtre en place 463,4 s à la consigne, **962 s sur le groupe**
après le rinçage, la durée la plus longue des routines guidées (808 s le
16 à 9 h 34) ; préparation de 92,4 s. La balance a manqué 9,8 s pendant
la routine.

**Hydraulique.**

| Phase | Début | Durée | Volume | Pression |
| --- | ---: | ---: | ---: | --- |
| Précharge | 0,00 s | 8,10 s | — | — |
| Remplissage | 8,10 s | 6,24 s | 21,4 ml (3,44 ml/s) | 0,57 → 0,86 bar |
| Pré-infusion | 14,34 s | 3,75 s | 8,1 ml | 0,86 → 1,35 bar |
| Infusion | 18,09 s | **22,32 s** | 49,0 ml | 9,61 bar max |

- Pression au repos avant la pompe : 0,50 bar piégés (0,39 à 0,40 bar
  les 17 et 18 octobre).
- Premier paquet au-dessus de 0,8 bar à 14,09 s (20,9 ml, 6,0 s après le
  début du remplissage), passage en pré-infusion à 14,34 s : 250 ms. Le
  remplissage le plus court en volume depuis le 13 octobre à 13 h 29
  (21,7 ml) ; 1 bar à 24,3 ml, en pré-infusion.
- Pré-infusion finie sur la goutte, 0,25 s avant l'échéance ; première
  goutte (0,3 g) à 30,1 ml, marge de **8,7 ml**.
- Montée de 3,6 s jusqu'à 8 bar, avec 2,8 g en tasse (6,7 g le 18). La
  pression dépasse ensuite la cible : **9,61 bar** à 22,08 s, pompe à
  78 %, 0,4 s après 8 bar (9,03 bar au plus le 17 à 13 h 57).
- Depuis 8 bar : **18,8 s**, 0,98 g/s en tasse, 2,01 ml/s en amont.
  21,5 g à l'arrêt (au poids, l'échéance de 27 s n'est pas atteinte),
  22,3 g final, **0,8 g de gouttes**. Pompe en marche 32,3 s.
- Plateau : 9,08 ± 0,08 bar, pompe à 67 %, 1,00 g/s, soit **9,0 bar·s/g**,
  la galette la plus résistante de la synthèse (8,5 le 12 octobre à
  9 h 12, mouture affinée). Rapport du débit en tasse au débit amont :
  0,50 g/ml, le plus bas (0,52 à 0,58 du 13 au 17 octobre, 0,69 le 18) ;
  il suit toujours la perméabilité de la galette.
- Arrivée en tasse, depuis le début de l'infusion :

  | Infusion | 1 g | 5 g | 10 g | 20 g |
  | --- | ---: | ---: | ---: | ---: |
  | 20/10 14 h 29 (mouture fine) | 1,9 s | 6,0 s | 11,3 s | **20,8 s** |
  | 18/10 12 h 40 (bec) | 0,7 s | 3,3 s | 5,9 s | 11,4 s |
  | 17/10 13 h 57 | 1,3 s | 4,5 s | 8,8 s | 16,3 s |
  | 16/10 14 h 10 | 1,8 s | 4,7 s | 8,7 s | 16,3 s |

**Thermique.**

| Repère | 20/10 14 h 29 | 18/10 12 h 40 | 17/10 13 h 57 |
| --- | ---: | ---: | ---: |
| NTC au départ | 92,00 °C | 91,83 °C | 91,75 °C |
| Pic avant l'infusion | **94,38 °C** à 13,9 s | 93,15 °C à 12,3 s | 92,75 °C à 11,0 s |
| NTC au début de l'infusion | **93,70 °C** | 88,36 °C | 90,25 °C |
| Commande à 0 % (remboursement) | 12,1 s | 9,8 s | 10,7 s |
| Appoint repris avant l'arrêt | **10,2 s** | 2,4 s | 6,5 s |
| Pente de la NTC pendant l'appoint | −0,17 °C/s | −0,14 °C/s | −0,22 °C/s |
| **Tasse** | **92,57 °C**, +0,57 °C | 90,44 °C | 90,50 °C |
| Minimum | 90,44 °C, 2,6 s après l'arrêt | 88,36 °C | 89,02 °C |
| 20 s après l'arrêt | 93,73 °C, encore en hausse ; **94,1 °C** au plus ensuite (David) | 91,37 °C | 91,55 °C |

- Énergie, commande / SSR : précharge 8,59 / 8,61 kJ, remplissage 6,71 /
  6,36 kJ, pré-infusion 2,92 / 2,70 kJ, infusion 6,97 / **6,08** kJ.
  Total SSR jusqu'à l'arrêt : 23,75 kJ, dont 17,67 kJ avant l'infusion
  (18,63 kJ le 18).
- Remplissage : commande de 83 à 90 %, SSR coupé ≈ 0,2 s au plus. La NTC
  y **gagne 2,25 °C** (92,12 → 94,37 °C) ; elle en perdait 0,57 le 18.
  La pré-infusion ne lui retire que 0,67 °C (3,37 °C le 18).
- Eau admise avant l'infusion : 29,6 ml, contre 33,3 ml le 18.
- Deux lectures NTC invalides (211 et 209 ms) à 40,62 et 40,94 s, après
  l'arrêt de la pompe, commande déjà à 0 % : aucun pas thermique ne tombe
  dessus.
- Modèle : 92,06 °C avec la commande réelle, 91,72 °C avec l'énergie SSR,
  soit **+0,85 °C** au-dessus du modèle (−1,39 °C le 18). L'écart se
  forme pendant le remplissage : +1,58 °C à 14 s, +1,96 °C à 20 s, puis
  +0,32 °C à 30 s et +0,01 °C à 35 s. En fin de capture, le modèle avec
  l'énergie SSR est à 92,41 °C, la mesure à 93,71 °C.

**Attendus vérifiés** (entrée du 18 octobre, 12 h 40). Ils visaient le
porte-filtre à bec à mouture inchangée : les deux ont changé.

| Point | Attendu | Mesure |
| --- | --- | --- |
| Hydraulique | 6,5 à 8,5 bar·s/g ; 13 à 18 s depuis 8 bar | **9,0 bar·s/g, 18,8 s : dépassé**, mouture affinée |
| Rendement | 21,6 g à l'arrêt ; 23,0 à 24,0 g final avec le bec, 22,3 à 23,0 g sans fond | 21,5 g, 22,3 g final sans fond : atteint, gouttes de 0,8 g |
| Remplissage | 3,4 à 3,7 ml/s à 75 % ; 5 à 8,5 s et 17 à 29 ml ; marge positive | 3,44 ml/s, 6,24 s, 21,4 ml, 8,7 ml : atteint |
| Pré-infusion | goutte jusqu'à ≈ 8,5 ml de marge, échéance à partir de ≈ 11 ml | goutte à 8,7 ml : atteint |
| Chauffe | 90 % sur le remplissage ; 0 % pendant 9,5 à 12,5 s ; lecture invalide sur un pas : 0 %, loi et dette conservées ; modèle avec l'énergie SSR entre −1,2 et +0,8 °C ; chute de plus de 2 °C en pré-infusion avec le bec | 90 % : atteint ; 12,1 s : atteint ; aucune lecture sur un pas : **non vérifié** ; **+0,85 °C : non atteint** de 0,05 °C ; sans bec : **non vérifié** |
| Tasse | 90,4 à 92,6 °C ; maximum après l'arrêt sous ≈ 94 °C | 92,57 °C : atteint, en haut de plage ; **94,1 °C : non atteint** de 0,1 °C |
| Goût | à galette dans la plage : acidité absente, tassage ; acidité restée, porte-filtre | galette hors plage, porte-filtre changé : **non concluant** |

**Lecture.**

- *Galette, mesuré.* 9,0 bar·s/g, contre 6,9 à 7,2 bar·s/g pour les
  galettes sans fond du 16 et du 17 octobre : +25 à +30 %. Avec la
  mouture et le porte-filtre changés ensemble, rien ne sépare les deux
  effets ; le panier est le même, et la mouture plus fine suffit à
  l'expliquer. Le dépassement à 9,61 bar au passage de 8 bar va avec la
  galette dure (estimé) : à 78 %, la pompe fait monter la pression plus
  vite que la régulation ne la ramène.
- *Gouttes, mesuré.* 0,8 g, sous les 1,2 à 1,3 g du sans fond les 16 et
  17 octobre : le débit en tasse à l'arrêt (≈ 1,0 g/s contre ≈ 1,3 g/s)
  laisse moins de café en chemin. Le retour au sans fond retire bien les
  2,0 g du bec.
- *Remplissage court, mesuré.* 21,4 ml jusqu'à 0,8 bar, contre 25,2 ml le
  18 : la galette fine monte en pression plus tôt, comme le 12 octobre à
  9 h 12 (18,6 ml après l'affinage). Moins d'eau froide passe sous la
  précharge, et la NTC monte pendant le remplissage. Le même lien se lit
  sur les captures précédentes : +2,25 °C à 21,2 ml le 6 octobre, +1,64 °C
  à 21,7 ml le 13 à 13 h 29, de −0,57 à +1,12 °C entre 24,1 et 28,4 ml
  depuis le 14 octobre.
- *Tasse chaude, mesuré.* La NTC arrive à l'infusion à 93,70 °C, 5,34 °C
  au-dessus du 18. La tasse, à 92,57 °C, est la plus chaude depuis le 16
  à 9 h 34 (dette effacée). Le modèle ne reproduit pas la hausse du
  remplissage, comme le 13 à 13 h 29 (« sous la mesure dès le
  remplissage ») : l'écart de +0,85 °C vient de là, il se referme dès
  30 s.
- *Rebond après l'arrêt, estimé.* 18,8 s depuis 8 bar laissent 10,2 s
  d'appoint avant l'arrêt, le plus long des routines guidées (9,6 s le
  13 à 8 h 18), et 6,08 kJ au SSR pendant l'infusion (1,70 kJ le 18).
  La chaleur encore en route dans la chaudière à l'arrêt arrive à la NTC
  sans eau à chauffer : +3,7 °C du minimum (90,44 °C) au maximum relevé
  par David (94,1 °C). Le simulateur place ce maximum 29,5 s après
  l'arrêt ; la capture s'arrête à 20 s.
- *Acidité, mesuré.* Les deux causes avancées les jours précédents ne
  s'appliquent pas ici : l'extraction est longue (18,8 s depuis 8 bar,
  contre 8,4 s le 18), et la NTC tasse (92,57 °C) est au-dessus de toutes
  les tasses très bonnes à impeccable (90,63 à 91,96 °C, du 13 au
  16 octobre) ; seule celle du 16 à 9 h 34, jugée bonne, était plus chaude
  (93,16 °C).
  Le porte-filtre est resté 962 s sur le groupe : l'attendu du 17 octobre
  (une acidité après plus de ≈ 4,5 min infirmerait la piste du
  porte-filtre) est rempli, à mouture changée. L'infusion la plus proche
  en hydraulique, le 12 octobre à 9 h 12 (8,5 bar·s/g, 18,7 s), était
  jugée très bonne à 88,75 °C de NTC tasse.
- *Acidité, observation de David.* Empiriquement, une mouture fine à
  débit faible donne un goût plus acide. Deux explications, non testées :
  1. aucun channelling, donc aucune amertume parasite pour couvrir
     l'acidité ;
  2. celle que David privilégie : à faible débit, l'arrêt à 22 g ne laisse
     passer que la première partie de l'extraction, acide ; la partie plus
     sucrée, plus tardive, n'arrive pas en tasse.

  Le 12 octobre à 9 h 12, à galette proche (8,5 bar·s/g, 18,7 s depuis
  8 bar), la tasse était très bonne avec un arrêt à 23 g : 1 g de plus
  d'extraction, ce qui va dans le sens de la seconde explication. Mais la
  NTC tasse y était inférieure de 3,8 °C, et la routine n'était pas
  guidée : rien ne sépare ces écarts. Seul un arrêt plus tardif à mouture
  égale testerait la seconde explication.

**Décision** (David). Mouture rendue moins fine ; écran 0.4.15, réglages
inchangés (cible 22 g).

**Attendus pour l'infusion suivante** (0.4.15, mêmes réglages, mouture
moins fine, porte-filtre sans fond, routine guidée). Le pas de mouture
n'est pas chiffré : un pas valait ≈ 0,8 bar·s/g le 8 octobre (estimé sur
ce seul pas).

| Point | Attendu |
| --- | --- |
| Hydraulique | 7,5 à 8,5 bar·s/g ; 14 à 18 s depuis 8 bar ; pic de pression au passage de 8 bar entre 9,0 et 9,6 bar |
| Rendement | ≈ 21,5 g à l'arrêt, 0,7 à 1,3 g de gouttes, soit 22,2 à 22,8 g final |
| Remplissage | 3,4 à 3,7 ml/s à 75 % ; 5 à 7,5 s et 20 à 26 ml ; marge positive |
| Pré-infusion | goutte jusqu'à ≈ 9 ml de marge, échéance au-delà |
| Chauffe | 90 % sur le remplissage ; NTC +0,5 à +2,3 °C pendant le remplissage, d'autant plus que celui-ci est court ; 0 % pendant 10 à 12,5 s ; appoint de 6 à 10 s ; lecture NTC invalide sur un pas : 0 % sur ce pas, loi et dette conservées ; modèle avec l'énergie SSR entre −1,2 et +1,0 °C (élargi : il ne suit pas la hausse du remplissage) |
| Tasse | 91,0 à 92,8 °C ; maximum après l'arrêt de 93,0 à 94,3 °C, ≈ 30 s après l'arrêt, hors d'une capture de 20 s |
| Goût | acidité moindre (observation de David). Une acidité aussi marquée sous 8 bar·s/g et 16 s depuis 8 bar infirmerait le lien avec le débit faible |

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

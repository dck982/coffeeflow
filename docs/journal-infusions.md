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
| Thermique | NTC pondérée par la tasse (`analyze_hf_capture.py`, critère de la consigne) ; minimum pendant l'écoulement ; minimum total et son instant ; NTC à l'arrêt et 20 s après |
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

## Pistes pour le goût

Recette : grain ViCafé (100 % arabica), 8,0 g en panier. Routine de
David avant chaque infusion : purge de 5 à 6 s sans porte-filtre dans la
tasse (rinçage du circuit, tasse chauffée), puis aussitôt une seconde
purge de 5 à 6 s avec le porte-filtre, dans la même tasse ; attente que
la NTC revienne à la consigne, porte-filtre en place avec l'eau restée
dedans ; porte-filtre vidé et séché, galette préparée, tasse vidée et
rincée, infusion. La durée de l'attente à la consigne varie et n'est pas
enregistrée. Les pistes
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

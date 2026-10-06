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
| Précharge | 8 s à 90 % | 4 octobre |
| Consigne | 90 °C | — |
| Remplissage | pompe à 75 %, fin à **0,8 bar** (10 s au plus) | 5 octobre, 7 h 59 |
| Pré-infusion | pompe à **50 %**, fin à 4 s ou à la première goutte | 5 octobre, 7 h 59 |
| Infusion | pompe jusqu'à 100 %, cible 9 bar, arrêt à 22 g ou 27 s, sans ramp-down | — |

Le numéro de firmware est celui du dernier flash connu ; la capture ne
l'enregistre pas.

## Méthode

### Commandes

```sh
uv run firmware/tools/describe_hf_capture.py captures/<capture>.json
uv run firmware/tools/analyze_hf_capture.py captures/<capture>.json
uv run firmware/tools/simulate_boiler.py --capture captures/<capture>.json
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

Valeurs du rapport (définitions ci-dessus). Les infusions avant le
5 octobre finissaient le remplissage sur +0,1 bar au-dessus du plancher
(schéma 8) et pré-infusaient à 60 % (35 % le 4 octobre à 13 h 24).

| Infusion | Remplissage | Pré-infusion | 1re goutte | Montée à 8 bar | Depuis 8 bar | Tasse | NTC tasse |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: |
| 30/09 14 h 25 | 4,05 s, 13,6 ml, 0,42 bar | 3,30 s, 11,0 ml, goutte | 24,6 ml | 5,3 s | 9,3 s, 1,47 g/s, 1,90 ml/s | 22,3 g | 89,60 °C |
| 01/10 8 h 29 | 4,40 s, 14,8 ml, 0,42 bar | 1,30 s, 4,1 ml, goutte | 18,8 ml | 6,4 s | 7,3 s, 1,59 g/s, 2,06 ml/s | 22,5 g | 88,42 °C |
| 04/10 10 h 01 | 3,00 s, 10,1 ml, 0,33 bar | 4,00 s, 12,5 ml, échéance | 38,0 ml | 4,6 s | 29,8 s, 0,71 g/s, 0,91 ml/s | 22,2 g | 92,50 °C |
| 04/10 13 h 24 | 4,50 s, 15,4 ml, 0,39 bar | 4,00 s, 2,0 ml, échéance | 35,1 ml | 6,4 s | 17,8 s, 1,10 g/s, 1,40 ml/s | 22,2 g | 87,58 °C |
| 05/10 7 h 26 | 7,00 s, 24,1 ml, 1,19 bar | 1,20 s, 1,2 ml, goutte | 25,8 ml | 4,5 s | 13,3 s, 1,28 g/s, 1,85 ml/s | 22,4 g | 88,64 °C |
| **06/10 7 h 19** | 6,10 s, 21,2 ml, 0,90 bar | 2,10 s, 4,1 ml, goutte | 25,5 ml | 4,8 s | 11,8 s, 1,35 g/s, 1,89 ml/s | 22,6 g | 88,47 °C |

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

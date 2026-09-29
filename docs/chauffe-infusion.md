# Chauffe pendant l'infusion

Ce document décrit la loi de chauffe appliquée pendant une infusion, de la
précharge à l'arrêt de la pompe (version écran **0.3.28**), et les mesures qui
la justifient. Le code est dans `firmware/screen/main/core/brew_heating.h`.

## Périmètre

Deux lois de chauffe coexistent dans le firmware écran :

| Loi | Code | Phases |
| --- | --- | --- |
| **Loi d'infusion** (ce document) | `BrewHeating`, `BrewEndEstimator` dans `brew_heating.h` | précharge, remplissage, pré-infusion, infusion, rampe de fin |
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
   final.** Le minimum de la NTC survient pendant l'infusion ; la chaleur
   envoyée ensuite arrive après lui. En simulation, couper 8 ou 11 s avant
   l'arrêt ne change pas le minimum et retire 1,7 à 3,7 °C au rebond.

Conséquences :

- la commande pendant l'écoulement suit le débit mesuré, sans dépendre de la
  NTC ;
- la précharge sert à **devancer** l'eau froide, pas à équilibrer un bilan ;
- la chauffe est coupée peu avant l'arrêt prévu de la pompe.

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
| Débit non mesurable (conditions ci-dessus) | **45 %** (repli) |
| Arrêt de la pompe prévu dans **11 s** ou moins | 0 % jusqu'à l'arrêt, même si l'estimation remonte |
| Mesure invalide, chauffe désactivée, NTC > 105 °C | 0 % (coupures générales) |

Le seuil de sécurité est volontairement large : le pic de NTC dû à la
précharge (≈ +1,5 à +2,5 °C) ne doit pas retirer l'appoint.

### Coupure de fin

Le temps restant avant l'arrêt de la pompe n'est estimé que pendant
l'infusion, rampe de fin comprise. Il est inconnu (NaN) pendant le
remplissage et la pré-infusion.

| Condition d'arrêt de la machine | Estimation |
| --- | --- |
| Temps cible (pas de balance au départ du cycle) | `target_time_s` − temps écoulé depuis le début du remplissage |
| Poids cible (balance présente au départ) | (poids d'arrêt − poids en tasse) / débit en tasse |
| Arrêt manuel | aucune : la chauffe continue jusqu'à l'arrêt |

Le poids d'arrêt est celui qu'applique la machine
(`Machine::stop_weight_g()`) : `target_weight_g` avec une rampe au poids,
sinon `target_weight_g − rampdown_lead_weight_g`. Le poids en tasse est
compté depuis le début du cycle. Le débit en tasse est calculé sur les
**2 dernières secondes** de mesures, prises à chaque pas du régulateur
(250 ms).

L'estimation au poids reste inconnue, et la chauffe continue :

- tant que la tasse n'a pas reçu **3 g** ;
- tant que 2 s de mesures ne sont pas disponibles ;
- si le débit en tasse est inférieur à **0,3 g/s** ;
- si la balance n'est plus présente.

Ce choix privilégie une fin plus chaude à un creux plus profond. Rejouée sur
les deux captures de référence, l'estimation coupe **0,1 s après** le
moment idéal (7 h 40, infusion courte de 14,4 s, coupure à 4,5 g) et
**1,4 s avant** (13 h 01). Une seconde d'erreur vaut 0,3 à 0,6 °C sur
l'état final, sans effet sur le minimum.

Sur une infusion courte, la coupure tombe tôt : à 7 h 40, 3,4 s après le
début de l'infusion, pendant la montée en pression.

## Réglages

| Réglage | Emplacement | Valeur |
| --- | --- | --- |
| Durée de précharge | NVS `heating.brew_preheat_time_s`, 4e page des réglages | 0 à 15 s ; 8 s au prochain essai |
| Puissance de précharge | `BrewHeating::kPreheatPowerPct` | 90 % |
| Bande de suppression de la précharge | `BrewHeating::kPreheatAboveTargetBandC` | +0,5 °C |
| Maintien | `BrewHeating::kHoldPowerPct` | 3,5 % |
| Pente débit → commande | `BrewHeating::kWaterHeatPctPerMlS` | 4,18 × 70,5 / 12 ≈ 24,56 % par ml/s |
| Plafond pendant l'écoulement | `BrewHeating::kPowerLimitPct` | 90 % |
| Repli sans débit | `BrewHeating::kFlowFallbackPct` | 45 % |
| Avance de la coupure de fin | `BrewHeating::kEndCutLeadS` | 11 s |
| Sécurité au-dessus de la consigne | `BrewHeating::kSafetyAboveTargetC` | +4 °C |
| Fenêtre, poids et débit minimaux de l'estimation | `BrewEndEstimator::kRateWindowMs`, `kMinimumCupWeightG`, `kMinimumCupRateGPerS` | 2 s, 3 g, 0,3 g/s |

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
neuve ; le prochain essai règle **8 s**.

## Limites connues

- **La NTC n'est pas l'eau au groupe.** Tout ce qui précède optimise la
  courbe de la sonde. Une sonde rapide à la sortie du groupe, pendant
  l'écoulement, dirait si le creux de la NTC est aussi celui de la tasse.
- **Le débitmètre est en amont de la pompe.** Une recirculation par l'OPV
  gonflerait le débit mesuré, donc la commande.
- **Premier instant du remplissage.** Le débit mesuré reste proche de 0 pendant
  la première seconde environ (≈ 0,07 ml/s alors que 2 ml sont déjà
  passés) : la commande y tombe vers 5 %, après la précharge. Les
  simulations incluent ce retard, puisqu'elles utilisent le débit mesuré.
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
| `monitor-heating-20260928-095631-275212.json` | — | ajustement ; montée sans écoulement depuis 80 °C |
| `260928-083730.json` | 5 s | exclue (fenêtre SSR de 5 s) |

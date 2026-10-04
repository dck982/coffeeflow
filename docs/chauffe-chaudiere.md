# Régulation de la chaudière

Ce document décrit d'abord **l'état actuel** (version **0.3.31**) :
machine, mesure, configuration et loi de chauffe. Viennent ensuite les
**points à considérer**, le **prochain essai**, puis le **journal des
essais**, qui conserve les mesures et le raisonnement ayant conduit aux
réglages actuels. Les procédures de mesure et le remplacement prévu de la
sonde terminent le document.

Les fichiers `captures/` cités sont locaux et ignorés par Git.

## Machine et ordres de grandeur

La machine est une **Profitec GO** : chaudière de **400 ml**, résistance de
**1 200 W**. Quelques ordres de grandeur utiles pour lire les captures :

| Grandeur | Valeur |
| --- | --- |
| 1 % de commande pendant 1 s | 12 J |
| Capacité thermique de l'eau de la chaudière | ≈ 1,67 kJ/K (400 g × 4,18), plus le métal |
| Montée maximale à 100 %, sans pertes ni écoulement | ≤ 0,72 °C/s, soit ≤ 0,0072 °C par %·s |
| Eau du réservoir | ≈ température de la pièce, 24–25 °C (estimation, non mesurée) |
| Remplacer 10 ml d'eau à 90 °C par de l'eau à 24,5 °C | ≈ −1,6 °C une fois mélangé |
| Chauffer 1 ml/s d'eau de 24,5 à 95 °C | ≈ 295 W, soit ≈ 25 % |

Le remplissage et la pré-infusion admettent environ 30 ml en 9 s : sans
chauffe, la chaudière perdrait environ 5 °C une fois l'eau mélangée.
Maintenir la température demanderait environ **88 %** à 3,6 ml/s, contre
**29 à 34 %** à 1,2–1,4 ml/s pendant l'infusion.

## Chaîne de commande

L'écran lit la NTC chaudière et calcule la puissance demandée, avec une
résolution de 0,1 %. Le module capteurs transforme cette puissance en temps de
marche du SSR sur une fenêtre fixe de **1 s**. L'écran renouvelle un bail de
**1 500 ms** toutes les 500 ms : une perte de l'écran ou du CAN coupe le
chauffage à l'expiration du bail. Une fenêtre isolée ne peut produire moins de
100 ms de chauffe ; un accumulateur répartit les faibles consignes sur
plusieurs fenêtres (1 % = 100 ms toutes les 10 s, 0,6 % ≈ 100 ms toutes les
16,7 s en moyenne).

Une fois la sortie éteinte dans une fenêtre, **aucune hausse de consigne ne
la rallume avant la fenêtre suivante** (`HeatingPwm::set_power`, module
capteurs). La fenêtre court en continu, y compris au repos : une chauffe
forte qui démarre juste après l'impulsion de repos attend jusqu'à 1 s. À
14 h 24, cela a coûté environ 2 kJ sur 23,8 kJ demandés ; voir
[l'infusion de 14 h 24](#infusion-de-14-h-24-29-septembre--précharge-de-10-s).

Depuis **0.3.30**, l'écran pose un flag `restart_window` sur la **première
commande non nulle de la précharge** et sur celle **de l'écoulement**
(début du remplissage) : le module capteurs ouvre alors une fenêtre neuve à
la réception (`HeatingPwm::restart`). Le flag n'est envoyé qu'une fois par
phase : répété, il relancerait la fenêtre toutes les 500 ms et chaufferait
en continu. Il n'est envoyé qu'à un module capteurs qui annonce la capacité
(`heating.window_restart_capable` dans `/telemetry`) ; perdu, il laisse le
comportement antérieur. Les hausses en cours d'écoulement (par exemple
après le creux de pré-infusion) attendent toujours la fenêtre suivante.

Les pourcentages des captures sont les **consignes demandées** par l'écran,
pas une mesure électrique de la puissance dissipée. Le champ `heater_on` des
captures HF donne l'état réel du SSR.

## Mesure de température

### Conversion actuelle

Le pont NTC utilise une résistance fixe de **4 676 Ω** entre A1 et GND
depuis le 2 octobre 2026 (écran 0.3.33) ; c'était **2 193 Ω** avant, valeur à
utiliser pour les captures antérieures. Il est alimenté par le 3V3 du port
Sensor AD du Waveshare. La résistance de la NTC vaut
`R_NTC = 4676 × (A0_raw/A1_raw − 1)` en ohms ; `A0_raw` et `A1_raw`
sont des codes ADS1115, pas des volts. Le PGA reste à ±4,096 V sur A0 et A1.

Depuis **0.3.34**, la conversion utilise **47 kΩ / 3 930 K** pour estimer la
température locale de la sonde, puis soustrait **10,5 °C** pour obtenir la
**température utilisateur**. Le Beta vient de l'étalonnage sur banc du
2 octobre 2026 ; l'offset passe de −10 à −10,5 °C pour qu'une même consigne
garde la même cible physique, à 0,1 °C près entre 90 et 110 °C sonde. Les constantes sont dans
`firmware/screen/main/core/calibration_machine.h`.

| Domaine | Utilisation | Exemple |
| --- | --- | --- |
| Utilisateur | écran, consigne, régulation, captures HF, `temperature.boiler.c`, `heating.target_c` | 90 °C |
| Sonde | `temperature.boiler.sensor_c`, `heating.target_sensor_c` | 100,5 °C ≈ 3,28 kΩ |

La coupure de chauffe à **105 °C utilisateur** représente **115,5 °C estimés
à la sonde**, soit la même résistance qu'avec la courbe précédente à 0,1 °C
près.

Ce réglage vient des essais de *flashing* du 27 septembre : un crépitement
devient perceptible à partir de **94 °C affichés**. Avec 967,7 hPa de
pression atmosphérique brute, l'ébullition est calculée à 98,72 °C ; cette
consigne correspond à environ 104 °C à la sonde avec la courbe 3 950 K
(104,5 °C avec 3 930 K) et 98 à 98,5 °C en sortie.
La conversion sonde et l'offset restent indicatifs : ils ne mesurent pas la
température de l'eau au groupe.

Le café jugé bon à 87 °C avec l'ancien offset de −14 °C visait environ
101 °C à la sonde ; son équivalent avec l'offset de −10 °C est une consigne
de **91 °C**. Une consigne NVS existante n'est pas migrée : une valeur de
87 °C reste affichée 87 °C et sa cible physique baisse de 4 °C.

### Historique des courbes

| Version écran | Courbe | Offset | Origine |
| --- | --- | ---: | --- |
| ≤ 0.3.7 | 27,29 kΩ / 3 728 K | 0 | points résistance/affichage Gicar (0.3.5 : essai à −18 °C, vapeur en purge) |
| 0.3.8 | 47,2 kΩ / 3 922 K | 0 | ajustement des mesures directes de la sonde |
| 0.3.9–0.3.13 | 47 kΩ / 3 950 K | 0 | valeurs nominales proches |
| 0.3.14–0.3.15 | 47 kΩ / 4 630 K | 0 | rejoindre Gicar près de 90 °C |
| 0.3.16–0.3.17 | 47 kΩ / 3 950 K | −14 °C | courbe sonde, affichage proche de Gicar |
| 0.3.18–0.3.33 | 47 kΩ / 3 950 K | −10 °C | *flashing* du 27 septembre |
| depuis 0.3.34 | 47 kΩ / 3 930 K | −10,5 °C | banc du 2 octobre ; même cible physique |

### Diagnostics ADS1115

Depuis 0.3.1, l'écran diffuse un `LOG` CAN à la première erreur ADS1115, puis
au plus toutes les 5 s si la même erreur persiste. `BOILER_ADC_NOT_FOUND`,
`BOILER_ADC_I2C_ERROR`, `BOILER_ADC_CONVERSION_TIMEOUT` et
`BOILER_NTC_INVALID_READING` distinguent les causes ; `BOILER_ADC_RECOVERED`
indique le retour d'une mesure valide et la durée de l'interruption. Le
décodeur `coffeetool` affiche l'adresse, l'étape I²C, le code ESP ou les
valeurs ADC brutes selon le cas.

Depuis 0.3.5, une erreur de lecture I²C entraîne un nouvel essai après 200 ms,
puis 400 ms, 800 ms et 1 s si elle persiste. Une lecture réussie remet ce
délai à 200 ms.

## Configuration

La configuration NVS v7 contient :

| Clé | Défaut | Plage | Réglage |
| --- | --- | --- | --- |
| `heating.brew_temperature_c` | 90 °C | 50 à 100 °C, pas de 0,5 °C | deuxième page des réglages |
| `heating.brew_preheat_time_s` | 2,5 s | 0 à 15 s, pas de 0,5 s ; 0 désactive | quatrième page des réglages |
| `heating.enabled` | `true` | — | `POST /config` uniquement |

La borne basse à 50 °C sert aux essais de calibration ; elle ne garantit pas
50 °C réels dans la chaudière. `firmware/tools/set_heating.py true` ou `false`
modifie `heating.enabled` via `GET /config` puis `POST /config`, avec
`COFFEEFLOW_HTTP_TOKEN` et `COFFEEFLOW_IP` dans l'environnement. La purge
reste possible quand la chauffe est désactivée.

Toute migration depuis une configuration v1–v5 persiste
`heating.enabled=false` avant le démarrage des tâches ; une installation neuve
conserve `true`. Une migration v6 conserve ce choix et initialise la
précharge à 2,5 s. Une consigne supérieure à 100 °C restée en NVS est ramenée
à 100 °C au démarrage.

L'infusion exige un module capteurs compatible et une mesure fraîche dans la
bande **consigne ±1 °C** pendant **3 s** (`kBrewTemperatureToleranceC`).

## Loi de régulation actuelle

Trois lois, toutes appelées par `Controller::step()`
(`firmware/screen/main/core/thermal_control.h`) :

| Situation | Loi | Document |
| --- | --- | --- |
| Repos | prédicteur de retard (depuis **0.3.29**) | [chauffe-repos.md](chauffe-repos.md) |
| Précharge, remplissage, pré-infusion, infusion | loi d'infusion, `brew_heating.h` (depuis **0.3.28**) | [chauffe-infusion.md](chauffe-infusion.md) |
| Purge, récupération | base commune ci-dessous | ce document |

Les trois lois partagent seulement l'historique de commande : la
récupération s'en sert pour estimer la chaleur en transit après une
infusion ou une purge. Régler l'une ne change pas les autres.

### Base commune de la purge et de la récupération

La commande de base vaut `3,5 % + 8 × erreur prédite`, avec :

- **maintien** nominal de **3,5 %** à la consigne ;
- **pente** de la NTC filtrée sur **8 s**, extrapolée sur **20 s** (10 s en
  récupération) seulement si elle est positive ;
- **chaleur en transit** : la commande au-dessus de 3,5 % demandée durant les
  **22 dernières secondes**, multipliée par **0,007 °C par %·s**, bornée à
  **1,5 °C** (6 °C en récupération) ;
- **filtre de sortie** de 5 s sur les baisses ; une commande nulle coupe
  immédiatement. Pendant une purge, les hausses sont appliquées sans délai.

L'intégrale de l'ancien régulateur n'agissait qu'au repos ; elle a été
retirée de cette base en 0.3.29.

Au-dessus de **105 °C**, sur mesure invalide ou si la chauffe est
désactivée, la commande tombe à zéro et le contrôleur est réinitialisé. Un
changement de consigne le réinitialise aussi.

### Par phase

| Phase | Commande |
| --- | --- |
| Repos | prédicteur de retard, jusqu'à 100 %, voir [chauffe-repos.md](chauffe-repos.md) |
| Précharge (`thermal_preheat`) | loi d'infusion : **90 %** pompe arrêtée pendant `brew_preheat_time_s`, sauf si la NTC dépasse la consigne de 0,5 °C ou plus ; hors chrono hydraulique |
| Remplissage, pré-infusion, infusion | loi d'infusion : `min(90 %, 3,5 % + 24,56 % × débit en ml/s)` ; depuis 0.3.37, 0 % dès l'entrée en infusion jusqu'à avoir retenu l'énergie de la précharge ; depuis 0.3.39, sans coupure de fin ; voir [chauffe-infusion.md](chauffe-infusion.md) |
| Purge près de la consigne | base commune plafonnée à **35 %** ; appoint de **18 %** jusqu'à la consigne, réduit linéairement à 0 à consigne + 2 °C (13,5 % à +0,5 °C, 9 % à +1 °C) ; plus l'appoint de débit, soit **45 %** au maximum |
| Purge lancée à ≥ 5 °C de la consigne | **0 %** jusqu'à la fin de la purge, même si la NTC traverse la consigne (purge de réglage thermique) |
| Récupération (30 s après l'écoulement) | base commune plafonnée à **35 %**, pente négative ignorée ; prédiction sur 10 s, retenant le **plus grand** effet entre pente montante et chaleur en transit |

**Appoint de débit de la purge** (depuis 0.3.14, purge seule depuis
0.3.28) : 0 point à 2 ml/s ou moins, 5 points à 3 ml/s, 10 points à 4 ml/s ou
plus, avec interpolation linéaire. Il décroît comme l'appoint de 18 %. Il
exige une mesure de débit fraîche, une impulsion de 500 ms au plus et la
pompe confirmée en marche. Le débitmètre est en amont de la pompe : lors
d'une recirculation par l'OPV, il peut surestimer le débit sorti au groupe.

Au début et à la fin de l'écoulement, la pente est remise à zéro.

### Loi d'infusion

Décrite dans [chauffe-infusion.md](chauffe-infusion.md) : appoint
proportionnel au débit, précharge à 90 % remboursée pendant l'infusion,
simulation et réglages. La coupure de fin (0.3.28 à 0.3.38) y est décrite
avec la raison de sa suppression.

### Pompe à l'entrée de l'infusion

La transition de la pré-infusion vers l'infusion (depuis 0.3.18) augmente la
pompe par pas de 5 points en environ 2,5 s. La boucle de pression attend la
fin de cette montée, sauf si la pression entre plus tôt dans sa bande
d'activation : elle reprend alors la commande courante sans saut. Depuis
0.3.32, la montée part d'au moins `kMinimumBrewPumpPct` (50 %) : après une
pause de pré-infusion à 35 %, sous le seuil de débit de la pompe, elle ne
perd pas son premier tiers.

### Captures

Depuis **0.3.32**, une capture HF d'infusion conserve **20 s** après l'arrêt
de la pompe (30 s en 0.3.27–0.3.31, 20 s en 0.3.25–0.3.26) ; une purge ou une
commande de banc, **5 s**. La précharge y apparaît comme `thermal_preheat`.
Pour une purge, lancer `record_heating.py --mode monitor` juste après si le
maximum du rebond compte.

Sur les cinq infusions du 29 septembre au
2 octobre, la NTC varie de moins de 0,2 °C entre 20 et 29 s après l'arrêt,
et le poids en tasse est stable 0,5 à 9,1 s après l'arrêt. Ces infusions
avaient la coupure de fin (0.3.28 à 0.3.38). Depuis 0.3.39, la chauffe suit
le débit jusqu'à l'arrêt, comme en 0.3.27 : le maximum du rebond peut
reculer au-delà des 20 s, qui ne couvrent pas non plus toute la récupération
(`kRecoveryDurationMs`, 30 s).

Argument de 0.3.27 pour 30 s : à 13 h 01, la NTC plafonnait déjà vers 20 s
après l'arrêt (+0,09 °C sur les 2 dernières secondes) ; 30 s devaient couvrir
le maximum du rebond. 25 s auraient suffi de justesse. À 7 h 40, la NTC
est à 0,06 °C de son maximum dès 20 s après l'arrêt de la pompe. Le modèle
de [simulation](#simulation-de-la-piste-1-29-septembre) place le maximum à
24,4–24,6 s après l'arrêt pour les deux infusions, et la montée sans
écoulement du 28 septembre à 22,5 s après la coupure. 25 s ne laisseraient
donc que 0 à 2,5 s de marge, et une loi qui chauffe jusqu'à l'arrêt de la
pompe repousse ce maximum. Les 30 s couvraient aussi toute la phase de
récupération (`kRecoveryDurationMs`).

## Points à considérer

### Incohérences du code

Les points 1, 5, 6 et 8 décrivent la loi d'infusion jusqu'à 0.3.27. Ils sont
résolus en 0.3.28 par la [loi d'infusion](chauffe-infusion.md) séparée ; le
point 2 ne concerne plus que la récupération. Ils sont conservés pour
l'historique.

1. **Le plancher d'infusion court-circuite l'anticipation.** Pendant
   l'écoulement, `power = max(power, 45 %)` s'applique tant que la *mesure*
   reste sous consigne + 1 °C. Le commentaire « Un dépassement prédit coupe
   immédiatement » est donc faux en infusion : seule la mesure coupe, avec
   environ 8 s de retard. À 13 h 01, la chauffe est restée à 45 % pendant
   toute la remontée de l'infusion, puis la NTC a gagné 3,8 °C après l'arrêt
   de la pompe, commande à zéro.
2. **La précharge est invisible pour la prédiction.** Hors récupération, la
   chaleur en transit est bornée à **1,5 °C**. Dix secondes à 100 %
   représentent pourtant 965 %·s au-dessus du maintien, soit environ 7 à
   9 °C selon le gain retenu.
3. **Le gain thermique est sous-estimé.** Le code utilise
   **0,007 °C par %·s**. Les deux montées sans écoulement du 28 septembre
   donnent **0,0086** (1 459 %·s, +12,6 °C du minimum au maximum) et
   **0,0090** (4 021 %·s, +36 °C), avant correction des pertes. Ces valeurs
   dépassent aussi la limite physique de **0,0072** calculée pour 1 200 W et
   400 ml d'eau, sans même compter le métal : voir la piste 6.
4. **Le commentaire sur le délai est obsolète.** Il décrit une réponse
   commande→température d'environ 22 s. Les mesures du 28 septembre
   distinguent un **retard d'environ 8 s** avant le retournement de la NTC et
   une **traîne d'environ 22 s** après la coupure. La fenêtre de 22 s décrit
   la traîne, pas le délai.
5. **La transition précharge → remplissage passe par le filtre de sortie.**
   Pendant la précharge, la commande filtrée vaut 100 %. En début de
   remplissage, elle est plafonnée à 60 % puis décroît vers 45 % avec la
   constante de 5 s (60 % à 10,1 s, 57,8 % à 11,0 s à 13 h 01). Ce
   comportement n'est pas voulu explicitement : il dépend du plafond, du
   filtre et de la durée de précharge.
6. **Les seuils du plancher et de l'appoint de débit diffèrent.** Le plancher
   disparaît à consigne + 1 °C, l'appoint de débit à consigne + 2 °C. Entre
   les deux, il ne reste que l'appoint : 8,3 % à 14 s dans la capture de
   13 h 01, alors que la NTC montait.
7. **La précharge n'est pas figée au départ.** La condition « NTC à moins de
   +0,5 °C de la consigne » est réévaluée à chaque pas. Avec 8 s de retard,
   elle ne coupe qu'une précharge déjà commencée au-dessus de la consigne ;
   c'est acceptable, mais ce n'est pas une décision prise au départ.
8. **L'appoint de débit est inversé par rapport au besoin.** Il est nul sous
   2 ml/s, ne vaut que 10 points au maximum, et la majeure partie de la
   commande vient d'un plancher fixe. À 13 h 01, la commande valait environ
   60 % pendant le remplissage à 3,6 ml/s et **45 %** pendant l'infusion à
   1,2–1,5 ml/s. Le besoin estimé est de 88 % et de 29–37 % : on chauffe
   trop peu au remplissage et trop pendant l'infusion.

9. **Une hausse de commande après l'extinction attend la fenêtre suivante.**
   Le module capteurs refuse de rallumer le SSR dans une fenêtre où il s'est
   déjà éteint. À 14 h 24, le SSR a démarré **1,0 s** après la précharge
   (90 % demandés à 0,31 s, SSR à 1,31 s), **0,55 s** après le retour du
   débit au remplissage et **0,5 s** après un creux de commande en
   pré-infusion : environ 2 kJ perdus, 10 % de la commande. La perte de la
   précharge dépend de la phase de la fenêtre au départ (0 à 1 s, soit 0 à
   1,1 kJ ≈ 0 à 0,7 °C) : la précharge effective varie d'une infusion à
   l'autre sans que la consigne change. **Résolu en partie en 0.3.30** pour
   le départ de la précharge et de l'écoulement (flag `restart_window`, voir
   la [chaîne de commande](#chaîne-de-commande)).

10. **La capture HF échantillonnait à ~130 ms, pas à 100 ms.** Les captures
    annoncent `sample_period_ms: 100`. À 14 h 25 le 30 septembre, les
    écarts entre échantillons valent **100 ms** (187 fois) ou **150 ms**
    (257 fois), sans échantillon perdu : ≈ 130 ms en moyenne, 149 ms en
    médiane. Même répartition à 14 h 24 le 29 septembre et à 9 h 43 le
    30 septembre. Cause dans `tick_hf_capture()` (`core.cpp`) : l'échéance
    suivante était posée à `now + 100 ms`. Or la boucle de télémétrie tourne
    à 50 ms (`vTaskDelay` de 5 ticks à 100 Hz), réveillée sur les frontières
    de tick, et `now` est lu après un travail de durée variable (requêtes
    CAN, `tick_machine`, `tick_thermal`). Deux tours plus tard, si ce travail
    a été un peu plus court, `now` tombait juste avant l'échéance :
    l'échantillon attendait un tour de plus, soit 150 ms. Les analyses
    interpolent sur `t_ms` et n'étaient pas faussées. **Résolu en 0.3.31**
    (`core/sample_schedule.h`) : l'échéance avance d'une période sur la
    grille de départ, un échantillon est accepté jusqu'à un demi-tour de
    boucle (25 ms) avant son échéance, et la grille repart de maintenant si
    le retard atteint une période moins cette avance. Attendu : des écarts
    de 100 ms à quelques millisecondes près ; 1 152 échantillons couvrent
    115 s. **Vérifié** à 8 h 29 le 1er octobre : les 548 écarts valent
    98 à 102 ms.

L'ancienne version de ce document annonçait aussi une bande « prête » de
±0,5 °C ; le code utilise ±1 °C.

### Pistes à explorer

1. **Appoint proportionnel au débit.** Remplacer le plancher fixe et
   l'appoint en escalier par
   `P = débit × 4,18 × (T_chaudière − T_eau) / 1 200 W`. Avec une eau à
   la température de la pièce (24–25 °C) et une chaudière vers 95 °C réels,
   cela donne environ 88 % à 3,6 ml/s, 34 % à 1,4 ml/s et 29 % à 1,2 ml/s.
   Le débitmètre étant en amont, une recirculation par l'OPV surestimerait
   l'appoint. La température d'eau du réservoir n'est pas mesurée : une
   constante suffit probablement, le capteur XDB401 mesurant son propre
   boîtier. **Simulé le 29 septembre** : l'appoint seul équilibre l'énergie,
   mais pas le creux de la NTC, car l'eau froide atteint la sonde avant la
   chaleur. Voir la [simulation](#simulation-de-la-piste-1-29-septembre).
2. **Dimensionner la précharge comme un budget d'énergie.** Le remplissage de
   13 h 01 admet environ 30 ml avant l'infusion, soit ≈ 8,8 kJ à apporter,
   l'équivalent de **7,4 s à 100 %**. La précharge pourrait alors se
   calculer à partir du volume attendu au remplissage, plutôt que d'être
   une durée fixe. **Partiellement contredit à 7 h 40** : le budget a été
   respecté à 0,6 kJ près et la NTC a quand même perdu 4,6 °C. La précharge
   sert surtout à **devancer** l'eau froide, pas à équilibrer le bilan.
3. **Anticiper la fin de l'infusion.** Le rebond vient de la chaleur fournie
   pendant les ~20 dernières secondes d'écoulement. Quand la fin est
   prévisible (poids ou volume cible), réduire la commande environ 8 s avant
   l'arrêt prévu. La simulation confirme : couper 8 à 11 s avant l'arrêt
   retire 1,7 à 3,7 °C au rebond sans toucher au minimum.
4. **Recaler le modèle de chaleur en transit.** Prendre un gain voisin de
   0,009 °C par %·s, compter la précharge dans l'estimation pendant
   l'écoulement, et vérifier la fenêtre de 22 s sur les deux montées du
   28 septembre.
5. **Simuler avant de flasher.** Fait avec
   `firmware/tools/simulate_boiler.py` : voir la
   [simulation](#simulation-de-la-piste-1-29-septembre). Le modèle reste à
   recaler à chaque capture qui enregistre l'état du SSR.
6. **Comprendre pourquoi la NTC monte plus vite que la physique.** Mesurer
   la tension secteur et la résistance à froid de l'élément (≈ 44 Ω attendus
   pour 1 200 W à 230 V). Autres hypothèses : une stratification autour de
   la sonde sans écoulement, ou un Beta trop faible, qui dilate les écarts de
   température : vers 3,3 kΩ, il faudrait un Beta d'au moins 4 500 K environ
   pour ramener le gain sous la limite physique, ce qui contredirait les
   mesures directes de la sonde. Une mesure indépendante de la température de la chaudière
   départagerait ces hypothèses.
7. **Remplacer la sonde par une PT1000.** Voir la
   [dernière section](#remplacement-prévu-par-une-pt1000).
8. **Démarrer une fenêtre SSR au début d'une chauffe forte.** Voir le point à
   considérer 9. Deux variantes, côté module capteurs, rejouées sur les
   commandes des trois infusions avec un modèle Python de `HeatingPwm`,
   pour toutes les phases de fenêtre :
   - ouvrir une fenêtre neuve quand la consigne passe de moins de 50 % à
     50 % ou plus : récupère la perte au départ de la précharge et rend
     l'énergie indépendante de la phase (22,86 kJ à 14 h 24 quelle que soit
     la phase, contre 20,9–22,8 kJ) ;
   - ne jamais retoucher la fenêtre en cours, la consigne s'appliquant à la
     fenêtre suivante : récupère aussi les pertes en cours d'écoulement,
     mais retarde toute baisse d'une seconde au plus.

   Le modèle Python ne reproduit l'énergie mesurée qu'à environ 1 kJ près :
   ces chiffres comparent les variantes entre elles, pas au SSR réel.

   **Retenu en 0.3.30** : une fenêtre neuve sur demande de l'écran, au
   premier paquet de la précharge et au premier paquet du remplissage.
   Rejoué de la même façon : 22,32 kJ à 14 h 24 quelle que soit la phase
   (20,89–22,84 kJ auparavant), 20,88–21,00 kJ à 7 h 40, 25,80–26,04 kJ à
   13 h 01. Le flag du remplissage ne change rien sur ces trois captures,
   la fenêtre étant déjà recalée par la précharge ; il sert quand la
   précharge est nulle ou supprimée.

## Prochain essai : sans coupure de fin, remplissage à 1 bar (0.3.40)

Premier essai du remboursement à
[13 h 24 le 4 octobre](#infusion-de-13-h-24-4-octobre--précharge-de-8-s-remboursement) :
**87,58 °C** en tasse, chauffe à 0 % pendant 22 des 24 s d'infusion. La
coupure de fin, que la précharge ne finance plus depuis le remboursement,
s'enchaînait au remboursement. L'écran **0.3.39** la supprime (voir
[chauffe-infusion.md](chauffe-infusion.md#coupure-de-fin-supprimée-en-0339)).

Réglages : `heating.brew_preheat_time_s` à **8 s**, pré-infusion à **35 %**
(pause), consigne de 90 °C. L'écran **0.3.40** termine aussi le remplissage
à **1 bar absolu** (10 s au plus) au lieu de +0,1 bar : ≈ 10 ml de plus
passent dans le remplissage, avec l'appoint complet au lieu d'être retenus
par le remboursement (≈ +2,9 kJ). Avec les deux changements, une tasse plus
chaude qu'à 13 h 24 ne départage pas leurs effets ; la simulation
ci-dessous ne compte que la coupure de fin. Noter le volume en fin de
remplissage et la durée de la pré-infusion.

Attendu, d'après la simulation sur l'hydraulique de 13 h 24 :

| Point | Attendu |
| --- | --- |
| Commande à 0 % dès l'entrée en infusion | ≈ 700 %·s retenus, soit ≈ 8,4 kJ, puis appoint au débit **jusqu'à l'arrêt de la pompe** |
| Moyenne NTC pondérée par la tasse | inchangée par la suppression (±0,02 °C en simulation) ; l'écart de −2 °C de 13 h 24 reste inexpliqué |
| Minimum | +0,2 à +1,3 °C par rapport à 0.3.38 à hydraulique égale |
| État 20 à 30 s après l'arrêt | +2 à +3,5 °C par rapport à 0.3.38, proche de la consigne ou au-dessus |

Si la moyenne en tasse reste vers −2 °C, le défaut n'est pas la coupure de
fin : comparer la pause de pré-infusion (13 h 24 : 2 ml en 4 s) aux
infusions où le débit a continué pendant la pré-infusion.

Si ce n'est pas déjà fait, relever 3 min de surveillance au repos avant
l'infusion : points à vérifier dans
[chauffe-repos.md](chauffe-repos.md#à-vérifier-sur-la-prochaine-capture).

Pour l'analyse :

```sh
uv run firmware/tools/analyze_hf_capture.py captures/<capture>.json
uv run firmware/tools/simulate_boiler.py --capture captures/<capture>.json
```

Comparer aussi l'énergie commandée et l'énergie SSR (`heater_on`) par phase :
le simulateur utilise la commande. Les contrôles de la fenêtre SSR neuve
(0.3.30) sont décrits dans l'essai de
[9 h 43](#infusion-de-9-h-43-30-septembre--précharge-de-8-s) ; les refaire si
le firmware du module capteurs change.

## Journal des essais

### Calibration de la sonde NTC

#### Sonde installée et hypothèses initiales (24 septembre)

La sonde chaudière est la
[Profitec P6036](https://links.imagerelay.com/cdn/2615/ql/423575fef2a046eb97c8b5af763d79b5/Pro-600-Parts-Diagram.pdf),
référencée comme NTC 1/8″. Sa pointe mesure **24 mm de long pour 3 mm de
diamètre**. Sa courbe n'est pas publiée ; les marquages
`1408504` et `16/18` n'ont pas été interprétés. À froid, la NTC débranchée
mesurait **42,1 kΩ**, ce que les constantes de l'époque (`27 290 Ω / 3 728 K`)
convertissaient en environ 15 °C, alors que la pièce était à 24,8 °C et le
métal de la chaudière à 26,7 °C au thermomètre IR après trois heures d'arrêt.

Hypothèses nominales envisagées avant la dépose : `47 kΩ / 4 700 K`
(combinaison d'un
[catalogue Panasonic](https://industrial.panasonic.com/cdbs/www-data/pdf/AUA0000/AUA0000C10.pdf)),
`50 kΩ / 4 800 K`, puis `47 kΩ / 4 400–4 450 K`. Le PID d'origine porte
l'inscription **`NTC 3K3`**, qui pourrait désigner une résistance fixe de
3,3 kΩ dans son pont ; ni le schéma ni la sonde ne le confirment. Son module
d'alimentation `prim BV 202 0154` (6 V / 0,5 VA), un STMicro L4941BV et un
condensateur ont été repérés près de l'entrée NTC.

**2,89 kΩ** est la résistance mesurée au multimètre après lecture de 90 °C sur
le Gicar, extinction et débranchement, sans durée de stabilisation connue.

#### Essai de résistances sur le PID Gicar d'origine

Des résistances mesurées ont été branchées à la place de la NTC sur l'entrée
du PID Gicar alimenté séparément ; son affichage est arrondi au degré :

| Résistance | Température affichée |
| ---: | ---: |
| 23,44 kΩ | 28 °C |
| 22,90 kΩ | 29 °C |
| 21,91 kΩ | 30 °C |
| 14,61 kΩ | 41 °C |
| 9,93 kΩ | 52 °C |
| 5,70 kΩ | 68 °C |
| 4,68 kΩ | 74 °C |
| 2,196 kΩ | 101 °C |
| 0,989 kΩ | 132 °C |

Un ajustement Beta donne environ **`R25 = 27,2 kΩ` et `Beta = 3 710 K`** pour
la *courbe d'affichage du PID* : 42 kΩ y valent environ 15 °C, et 90 °C
affichés correspondent à **2,92 kΩ**. L'accord avec les anciennes constantes
du firmware vient de ce que celles-ci avaient été déduites de l'affichage du
PID ; il ne valide pas la température réelle. Les écarts à une courbe Beta
unique restent de l'ordre du degré, sans rupture à basse température. Le PID
affiche `UP` pendant la montée en température, ce qui masque son écart à
froid.

#### Mesures directes de la sonde démontée

La NTC a été démontée et mesurée à plusieurs températures. Les points chauds
ont été pris pendant un refroidissement de 0,1 à 0,2 °C/s ; la sonde répond
plus lentement que le thermomètre digital, ce qui explique une partie de la
dispersion. Le point à 24,7 °C utilise un autre thermomètre, avant immersion.

| Température relevée | Résistance NTC |
| ---: | ---: |
| 20,1 °C | 57,7 kΩ |
| 21,6 °C | 54,6 kΩ |
| 24,7 °C | 49,1 kΩ |
| 24,9–25,0 °C | 47,6 kΩ |
| 43 °C | 21,42 kΩ |
| 49 °C | 18,70 kΩ |
| 74 °C | 7,60 kΩ |
| 77,7 °C | 6,66 kΩ |
| 78 °C | 5,97 kΩ |
| 86,2 °C | 4,95 kΩ |
| 89,5–89,8 °C | 4,73 kΩ |
| 91,0–91,3 °C | 4,31 kΩ |

Le point stabilisé près de 25 °C soutient fortement une **NTC nominale
47 kΩ**. L'ajustement des douze points donne **`R25 ≈ 47,2 kΩ` et
`Beta ≈ 3 920 K`** (`47,3 kΩ / 3 944 K` sans les deux derniers points). Avec
`R25 = 47 kΩ` fixé, les trois points chauds impliquent séparément des Beta
d'environ **3 940**, **3 840** et **3 920 K**. L'hypothèse `47 kΩ / 4 425 K`
est exclue : elle prévoit 3,8 kΩ à 86,2 °C contre 4,95 kΩ mesurés. Un Beta de
4 050 K attribuerait aux trois points chauds 84,2, 85,7 et 88,6 °C, soit 2 à
4 °C de moins que le bain ; le sens de cet écart ne peut être établi pendant
un refroidissement.

Pour les douze résistances, la courbe Gicar affiche **11,7 à 16,0 °C de
moins** que la température relevée, **13,6 °C en moyenne**. Ce décalage
presque constant pourrait être volontaire, pour afficher une température
d'infusion plutôt que la température locale de la chaudière ; aucune
documentation Gicar ne le confirme. Selon l'ajustement de la sonde, 90 °C
affichés par Gicar correspondent à environ **105 °C locaux**, et les
2,89 kΩ mesurés à environ 104,5 °C avec 3 950 K (102 °C avec 4 050 K). Pour
que 2,89 kΩ représentent 90 °C avec `R25 = 47 kΩ`, il faudrait un Beta
d'environ 4 646 K.

Les courbes sont comparables dans [l'explorateur NTC](ntc_curve_explorer.html).
Les anciens couples de
[`ntc_ads1115_calibration.md`](ntc_ads1115_calibration.md) utilisent la
température affichée par le PID : ce n'est pas une mesure indépendante.

#### Sensibilité du pont

Vers 90 °C locaux, la sonde vaut environ 4,5 kΩ. Une résistance fixe de
4,7 kΩ donnerait 24,5 mV/°C contre **21,7 mV/°C** avec les 2,193 kΩ d'origine,
soit 196 contre **173 codes par degré** : un gain de 13 % en sensibilité, pas
en justesse, négligeable devant l'incertitude de la courbe. Ce n'est pas la
raison du passage à 4676 Ω le 2 octobre 2026 : la résistance du banc PT1000 a
été reprise telle quelle.

#### Chaîne de mesure ADS1115 et multimètre

Le montage initial alimentait le pont par un LDO AMS1117 et retournait la
résistance fixe au Wago GND de l'alimentation 5 V. Le GND de l'ADS1115 se
trouvait **10–13 mV** au-dessus de ce Wago : une NTC proche de 50 kΩ était lue
à plus de 52 kΩ. Le pont utilise désormais **3V3 et GND du port Sensor AD du
Waveshare**, sans LDO. La première capture après correction (`t5.json`,
locale) donne `A0_raw = 26305`, `A1_raw = 1151`, soit **47,926 kΩ** calculés,
contre **47,9 kΩ** au multimètre immédiatement après : écart de 0,05 %. La
lecture de résistance est validée à ce point froid. Le détail figure dans
[la calibration NTC](ntc_ads1115_calibration.md).

#### Mesures au panier avec l'ancienne courbe (24 septembre)

Les températures d'eau mesurées en tasse après purges à froid, de 28,50 à
26,69 °C, confortent l'écart à froid de l'ancienne courbe
([`143100`](../captures/260924-143100.json),
[`143456`](../captures/260924-143456.json),
[`143921`](../captures/260924-143921.json),
[`145454`](../captures/260924-145454.json)).

Avec le firmware 0.3.7, à température café, les mesures dans le panier après
purge donnent **81,6 °C** pour une consigne de 80 °C
([capture](../captures/260924-162535.json)) et **86,7 puis 88,4 °C** pour
90 °C ([avant mode écoulement](../captures/260924-164051.json),
[après](../captures/260924-165337.json)). À consigne 65 °C, deux purges
donnent **72,6 puis 69,6 °C**
([première](../captures/260924-170809.json),
[seconde](../captures/260924-171142.json)). Aucune vapeur n'a été constatée à
90 °C affichés par Gicar. Relevés longs :
[`163749`](../captures/monitor-heating-20260924-163749-565936.json),
[`165225`](../captures/monitor-heating-20260924-165225-542907.json) (90 °C),
[`170713`](../captures/monitor-heating-20260924-170713-740281.json),
[`171044`](../captures/monitor-heating-20260924-171044-990745.json) (65 °C).

Ces mesures après purge ne séparent ni la température de l'eau stagnante à
la NTC ni celle de l'eau au groupe.

#### Écart à haute consigne (25 septembre, courbe 3 950 K sans offset)

Après stabilisation, le panier était proche de la consigne entre **50 et
70 °C**, mais vers **75 °C** pour une consigne de **90 °C**. Garder
`R25 = 47 kΩ` et expliquer 75 °C par la courbe demanderait un Beta d'environ
4 923 K, qui contredirait l'accord à 60–70 °C et les mesures directes ; un
offset uniforme abîmerait aussi la plage 50–70 °C. Ce calcul suppose que le
panier après purge donne la température de la NTC au repos, ce qui n'est pas
vérifié.

La limite de consigne a été portée **temporairement à 110 °C**. La
[purge](../captures/260925-145404.json) part de **109,1 °C** NTC et descend à
**93,8 °C** en fin d'enregistrement (16,35 s) ; l'eau est principalement
liquide, avec un panache visible au-dessus de la tasse, et le panier indique
environ **88 °C**. En mode purge, la chauffe montait de 18 % à 35 % vers
7,5 s. La [capture de surveillance](../captures/monitor-heating-20260925-144616-533930.json)
confirme une NTC proche de 111 °C avant la purge. Cette limite a été retirée
en 0.3.14 ; elle ne constitue pas un mode vapeur validé.

Avec le panier de simulation limité à 1,2 ml/s (valeur annoncée), deux purges
depuis 109 °C ont été comparées :

| Depuis le début | Panier précédent, faible pression | [Panier limité](../captures/260925-151230.json), pression montante |
| ---: | ---: | ---: |
| 8 s | NTC ≈ 104,2 °C ; ≈ 0,6 bar | NTC ≈ 105,2 °C ; ≈ 1,7 bar |
| 12 s | NTC ≈ 97,0 °C ; ≈ 1 bar | NTC ≈ 99,9 °C ; ≈ 6,4 bar |
| Arrêt de la pompe | 96,3 °C à 12,5 s | 97,0 °C à 16,25 s |
| Fin de capture | 93,8 °C à 16,35 s | 96,8 °C à 20,1 s |

Le panier limité était à **89 °C** 2–3 s après l'arrêt, contre une NTC à
96,8 °C. Une [troisième purge](../captures/260925-151508.json), sans vider le
panier, atteint 9,6 bar dès 5 s ; à volume compté voisin de **35 ml**, la NTC
vaut 103,7 et 103,4 °C dans les deux purges limitées. **La chute suit donc
surtout le volume d'eau froide admis**, pas la pression. Les 87 °C mesurés
après la troisième purge incluent l'eau refroidie de la précédente.

Ordre de grandeur : porter 4 ml/s de 25 à 110 °C demande environ 1,42 kW,
davantage que la résistance ; à 1,2 ml/s, environ 0,43 kW.

#### Courbe d'essai 47 kΩ / 4 630 K (0.3.14–0.3.15)

L'hypothèse de travail était que la conversion Gicar dans la plage café
correspondait à une calibration utile pour l'infusion. `47 kΩ / 4 630 K`
place 90 °C à **2,917 kΩ**, presque les 2,924 kΩ de la courbe Gicar, tout en
donnant 27,1 °C pour 42,1 kΩ. À résistance inchangée, les anciens points de
60 et 70 °C deviennent 54,4 et 62,6 °C. Une valeur Beta unique n'est qu'une
approximation ; pour une plage plus large, les fabricants utilisent des
tables ou une loi de Steinhart–Hart
([Vishay](https://www.vishay.com/docs/33001/seltherm.pdf),
[Analog Devices](https://www.analog.com/en/resources/analog-dialogue/articles/thermistor-temperature-sensing-system-part-1.html)).

Le [premier essai](../captures/260925-152754.json), panier chaud mais vide,
donne **90 °C dans le panier** pour une consigne de 90 °C. La NTC part de
91,86 °C, la pompe s'arrête vers 10,5 s après environ 40 ml comptés, avec une
NTC à 85,6 °C, puis 83,4 °C 3,9 s plus tard. La
[purge suivante](../captures/260925-153350.json) part de 90,06 °C et donne
**87,8 °C** dans le panier.

La moyenne de la NTC pondérée par les incréments de volume,
`Σ[ΔV × (T_avant + T_après)/2] / ΣΔV`, vaut **90,11 °C sur 40,0 ml** pour le
premier essai et **88,55 °C sur 42,61 ml** pour le second (88,77 °C à 40 ml).
La baisse au panier (2,2 °C) suit celle de la moyenne pondérée (1,56 °C). Le
débitmètre est en amont : c'est une estimation, pas une mesure à la sortie.

```sh
uv run firmware/tools/analyze_hf_capture.py captures/260925-152754.json
```

Le script affiche la moyenne jusqu'à l'arrêt de la pompe et sur la capture
entière, signale les échantillons perdus et refuse les segments où le volume
augmente alors que la NTC est invalide.

### Dynamique thermique

#### Purges à 90 °C (24 septembre)

Sur une purge de 8 s avec l'ancienne loi, la NTC descend à 80,9 °C environ
15 s après le début ; la commande atteint 93,9 % vers 23 s et la NTC culmine à
**96,6 °C** vers 50 s. Le panier indique 86,7 °C. Avec le plafond de 35 % en
récupération, la même purge donne un minimum de 79,4 °C, un maximum de
**88,2 °C** sur 90 s et 88,4 °C dans le panier. Le départ (90,6 contre
89,7 °C) et le volume (33 contre 31 ml) diffèrent : la comparaison n'isole
pas l'effet du logiciel.

#### Infusions des 25 et 26 septembre

La [capture du 25 septembre](../captures/260925-182131.json) (0.3.13, sans
appoint de débit) montre une chauffe à 18 % jusqu'à une NTC de 87 °C environ,
35 % à 84,54 °C (11,55 s), et une NTC à 78,4 °C en fin de capture, 3,8 s
après la pompe.

La [capture du 26 septembre](../captures/260926-073127.json) (courbe 4 630 K)
commence à 90,62 °C. La puissance reste à 10 % au début, car le plancher de
l'époque (30 %) ne s'appliquait qu'à 90,5 °C ou moins ; elle atteint 41,6 %
au maximum, et la NTC descend à 78,05 °C vers 20,35 s. Le plancher de 45 %
et le plafond de 60 % ont été introduits ensuite, en 0.3.17.

#### Retard de la NTC (28 septembre)

Deux montées sans écoulement donnent le même retard entre la mise en marche
réelle du SSR et le minimum de la NTC :

| Capture | SSR ON | Minimum NTC | Retard |
| --- | ---: | ---: | ---: |
| [Depuis 53 °C](../captures/monitor-heating-20260928-094947-505812.json) | 22,637 s | 30,619 s | **7,982 s** |
| [Depuis 80 °C](../captures/monitor-heating-20260928-095631-275212.json) | 13,121 s | 21,101 s | **7,980 s** |

La hausse dépasse le bruit 9,5 à 10 s après l'activation. La régulation
retient **8 s comme constante thermique effective de la NTC installée** ;
cette mesure porte sur l'ensemble résistance, eau et sonde.

Ce retard est distinct de la **traîne** : dans le second essai, la commande
tombe à zéro à 32,063 s avec une NTC à 83,14 °C, qui monte encore jusqu'à
**91,99 °C à 54,585 s**, soit 8,85 °C de plus pendant 22,52 s. Rapportée à
l'énergie injectée, la hausse vaut **0,0086 °C par %·s** dans cet essai et
**0,0090** dans le premier (voir les points à considérer).

#### Infusion de 8 h 37 (28 septembre) — précharge de 5 s

La [capture](../captures/260928-083730.json) commence à 90,54 °C. La commande
atteint 100 % vers 0,9 s et y reste jusqu'à environ 4,9 s ; la pompe démarre
vers 5 s. La NTC descend jusqu'à **82,32 °C à 22,61 s** (−8,22 °C). Avec le
retard de 8 s, la précharge commence à agir vers 9 à 11 s, et la chauffe du
remplissage vers 13 à 15 s ; l'arrivée d'eau froide masque encore la hausse
jusqu'au minimum. Décision : essayer **10 s**, en laissant la loi pendant
l'écoulement inchangée.

Cette capture précède le passage de la fenêtre du SSR de 5 s à 1 s (09 h 13) :
l'état du SSR n'y est pas enregistré, et le bilan énergétique ne ferme pas
(environ 3,6 kJ de moins que les captures suivantes). Elle est exclue de la
[simulation](#simulation-de-la-piste-1-29-septembre).

#### Infusion de 13 h 01 (28 septembre) — précharge de 10 s

[Capture brute](../captures/260928-130104.json) et
[graphique](../captures/260928-130104.html). Chronologie en temps brut de la
capture :

| Temps | Phase | NTC | Commande | Débit amont |
| ---: | --- | ---: | ---: | ---: |
| 0–10 s | précharge | 89,97 → 89,63 °C | 100 % | 0 |
| 10,1 s | remplissage | 89,68 °C | 60 %, puis décroît vers 45 % (+ appoint) | 0 → 3,6 ml/s |
| 13,5–14,5 s | remplissage | 91,6 → 92,2 °C | ≈ 8 % : plancher retiré à +1 °C, appoint de débit seul | 3,6 ml/s |
| 14,5–17 s | fin du remplissage, pré-infusion (14,8 s) | pic **92,45 °C** à 15,6 s | **0 %** | 3,6 ml/s |
| 18 s | pré-infusion | 90,7 °C | 55 % (plancher de retour) | 3,8 ml/s |
| 18,8–38,5 s | infusion | minimum **87,83 °C** vers 30 s, puis 89,2 °C | 49–53 %, puis **45 %** dès 24 s | 3,5 → 1,2–1,5 ml/s |
| 38,5–58,3 s | récupération | 89,2 → **93,00 °C**, encore en hausse | 0 % | 0 |

Le déficit maximal par rapport au départ n'est que de 2,14 °C, contre 8,22 °C
à 8 h 37, mais l'amplitude pic–creux est de **4,62 °C**. Le bruit de montée
en pression caractéristique de la chauffe forte a été entendu 4 à 5 s après
son démarrage ; en décalant la température de 5 s vers la gauche, son pic se
situe près du début du remplissage.

Lecture : la chaleur de la précharge arrive au début du remplissage et fait
dépasser consigne + 1 °C ; le plancher disparaît et la commande reste à 8 %
au plus de 13,5 à 18 s. Ce trou revient sur la sonde environ 8 s plus tard et
contribue au creux du début de l'infusion. Pendant l'infusion, les 45 %
(540 W) dépassent le besoin à 1,2–1,5 ml/s (≈ 350–440 W) : la NTC remonte,
puis la chaleur encore en transit à l'arrêt de la pompe produit le rebond.

Bilan énergétique : **26,1 kJ** injectés sur la capture (11,8 kJ en
précharge, 2,4 kJ en remplissage, 0,8 kJ en pré-infusion, 11,0 kJ en
infusion). La NTC finit 3 °C au-dessus de son départ, soit environ 4 à 6 kJ
de trop dans la chaudière, l'équivalent de 4 à 5 s à 100 %.

#### Infusion de 7 h 40 (29 septembre) — précharge de 6 s

[Capture brute](../captures/260929-074002.json) et
[graphique](../captures/260929-074002.html), version 0.3.27. Seule la durée de
précharge change par rapport à 13 h 01, mais le débit d'infusion aussi : pour
le même volume (68 ml) et le même poids (22,6 g), l'infusion dure 14,4 s au
lieu de 19,7 s, à **2,0 ml/s** en régime au lieu de 1,4 ml/s. La mouture ou
le tassage ont varié.

| Indicateur | 13 h 01 (10 s) | 7 h 40 (6 s) |
| --- | ---: | ---: |
| Pic avant infusion | 92,45 °C | **90,69 °C** à 12,1 s |
| Commande de 13,5 à 18 s | ≤ 8 %, dont 0 % | 53 à 64 % |
| Minimum | 87,83 °C | **86,07 °C** à 23,6 s |
| Amplitude pic–creux | 4,62 °C | **4,62 °C** |
| 20 s après l'arrêt de la pompe | 93,00 °C | 90,82 °C |
| 30 s après l'arrêt | — | 90,88 °C (+0,95 °C sur le départ) |
| Énergie SSR sur la capture | 26,1 kJ | 21,3 kJ |

Le but de l'essai est atteint : la NTC ne dépasse jamais consigne + 1 °C et
le plancher de 45 % n'est jamais retiré. L'amplitude pic–creux ne change
pourtant pas.

Commande et besoin par phase (besoin = maintien + débit amont × 4,18 ×
70,5 K) :

| Phase | Débit | Commande moyenne | Besoin moyen | Déficit |
| --- | ---: | ---: | ---: | ---: |
| Remplissage, 6,1–11,4 s | 3,6 ml/s | 61 % | 92 % | 1,9 kJ |
| Pré-infusion, 11,4–15,5 s | 3,5 ml/s | 57 % | 88 % | 1,5 kJ |
| Montée en pression, 15,5–20,5 s | 3,3 ml/s | 53 % | 80 % | 1,6 kJ |
| Infusion en régime, 20,5–29,9 s | 2,0 ml/s | 45,5 % | 52,5 % | 0,8 kJ |
| **Total** | | | | **5,9 kJ** |

À 13 h 01, le déficit était de 6,7 kJ jusqu'à la montée en pression, suivi
d'un **excédent** de 1,2 kJ en infusion (45 % contre 38 % nécessaires à
1,4 ml/s). À 7 h 40, l'appoint de débit est nul à 2 ml/s et le plancher de
45 % se retrouve **sous** le besoin (point à considérer 8).

Lecture :

- **La température finale suit le bilan « précharge − déficit ».** 6,5 kJ de
  précharge contre 5,9 kJ de déficit laissent +0,6 kJ, soit +0,95 °C mesurés.
  À 13 h 01, environ +4 kJ donnaient +3 °C.
- **Le creux ne vient pas d'un manque d'énergie totale** : le bilan cumulé
  reste positif pendant toute la capture. Il vient d'un décalage : la NTC
  commence à baisser **5 s** après le début du débit (7,2 → 12,2 s ; environ
  4 s à 13 h 01), mais ne réagit à la chauffe qu'après **8,2 s** (SSR à
  0,7 s, hausse visible à 8,9 s). La chaleur fournie pendant l'écoulement
  arrive trop tard pour compenser l'eau froide.

#### Simulation de la piste 1 (29 septembre)

`firmware/tools/simulate_boiler.py` ajuste un modèle linéaire de la NTC sur
les infusions de 13 h 01 et de 7 h 40 et sur la
[montée depuis 80 °C](../captures/monitor-heating-20260928-095631-275212.json),
puis rejoue des lois candidates sur l'hydraulique des deux infusions
(débit, durées de phase, récupération à 0 %) :

```sh
uv run firmware/tools/simulate_boiler.py          # paramètres enregistrés
uv run firmware/tools/simulate_boiler.py --fit    # réajuster
```

Le modèle superpose deux chemins, chacun avec un retard pur suivi de trois
premiers ordres, plus une part locale qui se mélange en 4,7 s :

| Chemin | Retard pur | Constante × 3 | Délai moyen |
| --- | ---: | ---: | ---: |
| Chauffe → NTC | 3,95 s | 4,56 s | ≈ 17,6 s |
| Eau admise → NTC | 2,0 s | 1,47 s | ≈ 6,4 s |

La capacité effective vaut **1,47 kJ/K**, soit 0,0082 °C par %·s ; les
pertes ajustées sont presque nulles sur une minute. L'écart RMS est de
0,46 à 0,48 °C sur chacun des trois enregistrements. En validation croisée
(une infusion retirée de l'ajustement), le minimum est prévu à **±1 °C**
près (88,87 contre 87,84 °C ; 85,75 contre 86,08 °C). Le modèle sous-estime
le pic de 13 h 01 (91,5 contre 92,45 °C).

Résultats, sur l'hydraulique de 13 h 01 et de 7 h 40 : voir
[chauffe-infusion.md](chauffe-infusion.md#simulation). En résumé :

- l'appoint proportionnel au débit, **sans précharge**, laisse la NTC
  descendre à **82,4–82,5 °C** : l'énergie est juste (89,5–89,6 °C 30 s
  après l'arrêt), mais le froid arrive environ 11 s avant la chaleur ;
- chaque seconde de précharge à 90 % relève le minimum d'environ **0,7 °C**
  et l'état final d'environ **0,7 °C** ;
- couper la chauffe *c* secondes avant l'arrêt de la pompe baisse l'état
  final sans modifier le minimum, tant que la coupure reste dans
  l'infusion en régime ;
- la commande « besoin du débit **11 s à l'avance** », irréalisable
  telle quelle, borne ce qu'on peut attendre : minimum 88,6–88,7 °C, état
  final 89,4–89,5 °C, 20 kJ. Une précharge de 8 à 10 s suivie d'une coupure
  11 s avant l'arrêt en est l'approximation réalisable.

#### Infusion de 14 h 24 (29 septembre) — précharge de 10 s

[Capture brute](../captures/260929-142427.json) et
[graphique](../captures/260929-142427.html). Première infusion avec la
[loi d'infusion](chauffe-infusion.md) de 0.3.28 : précharge de 10 s à 90 %,
appoint proportionnel au débit, coupure de fin. Consigne 90 °C, départ à
89,92 °C. 64,6 ml comptés, 22,4 g en tasse ; infusion de 17,3 s, dont 5 s à
3–3,5 ml/s pendant la montée en pression, puis 1,2–2 ml/s en régime.

| Temps | Phase | NTC | Commande | Débit amont |
| ---: | --- | ---: | ---: | ---: |
| 0,3–10,0 s | précharge | 89,9 °C, stable | 90 % (SSR à partir de 1,31 s) | 0 |
| 10,0–11,3 s | début du remplissage | 89,9 → 90,3 °C | 45 %, puis ≈ 5 % : débit pas encore mesuré | 0 → 3,3 ml/s |
| 11,3–14,9 s | remplissage | → 92,4 °C | 87–89 % | 3,5 ml/s |
| 14,9–18,9 s | pré-infusion | pic **92,53 °C** à 15,5 s, puis 91,6 °C | 60–90 %, selon le débit | 2,3–3,5 ml/s |
| 18,9–24,5 s | début de l'infusion, montée en pression | 91,6 → 91,2 °C | 76–90 % | 3–3,5 ml/s |
| 24,5 s | coupure de fin, ≈ 5,8 g en tasse | 91,0 °C | **0 %** | 2,3 ml/s |
| 24,5–36,3 s | infusion en régime | minimum **90,49 °C** à 26,8 s, puis **92,06 °C** à 34,5 s | 0 % | 1,2–2 ml/s |
| 36,3–66 s | récupération | 91,0 → 91,36 °C (49,6 s) → **90,86 °C** 30 s après l'arrêt | 0 % | 0 |

| Indicateur | Prévu | Mesuré |
| --- | --- | --- |
| Pic avant l'infusion | ≈ 92,5 °C | 92,53 °C |
| Minimum pendant l'écoulement | 89,3–89,6 °C (±1 °C) | 90,49 °C |
| Coupure de fin | ≈ 11 s avant l'arrêt | 11,8 s avant |
| 30 s après l'arrêt | 91,5–93,4 °C | 90,86 °C |
| Moyenne NTC pondérée par le volume | — | **91,46 °C** (13 h 01 : 89,98 ; 7 h 40 : 88,81) |

Lecture :

- **Plus de creux.** La chaleur de la précharge atteint la sonde vers
  10,3 s, 9 s après le démarrage réel du SSR, en même temps que l'eau
  froide, et la couvre entièrement. La NTC ne passe jamais sous la consigne
  pendant l'écoulement : la précharge de 10 s est **trop longue** pour cette
  mouture.
- **La deuxième bosse** (92,06 °C à 34,5 s) vient de la chauffe à 76–90 %
  pendant la montée en pression, au fort débit, qui atteint la sonde environ
  10 s plus tard, alors que la commande est déjà à 0 %.
- **La coupure de fin fonctionne** : l'état final est à +0,9 °C de la
  consigne, sous la fourchette prévue.

Énergie commandée et énergie réellement fournie par le SSR (`heater_on`) :

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 10,49 kJ | 9,42 kJ |
| Remplissage | 4,25 kJ | 3,66 kJ |
| Pré-infusion | 3,74 kJ | 3,30 kJ |
| Infusion | 5,33 kJ | 5,16 kJ |
| **Total** | **23,8 kJ** | **21,5 kJ** |

Il manque 10 % de la commande (13 h 01 : 26,1 contre 26,1 kJ ; 7 h 40 : 22,2
contre 21,3 kJ). Environ 2 kJ s'expliquent par trois attentes de fenêtre SSR
(point à considérer 9) : 1,0 s au départ de la précharge, 0,55 s au retour
du débit au remplissage, 0,5 s à 17,8 s en pré-infusion. La précharge
effective n'a donc été que d'environ **8,7 s** à 90 %.

Le simulateur, qui utilise la commande, en est affecté. Avec les paramètres
enregistrés, il prévoit 93,2 °C 30 s après l'arrêt, contre 90,86 °C
mesurés, et son écart RMS sur cette capture est de 1,30 °C. Rejoué avec
l'état du SSR, il retrouve l'état final (91,4 °C) mais sous-estime le milieu
de l'infusion de 1,2 à 1,7 °C. Un réajustement incluant cette capture
(C = 1,82 kJ/K, écarts RMS de 0,54 à 0,83 °C) donne les mêmes écarts entre
durées de précharge, à 0,1 °C près ; les paramètres enregistrés n'ont pas été
changés. La moyenne pondérée prévue pour 10 s (91,2–91,3 °C) est proche de
la mesure (91,46 °C).

Moyenne NTC pondérée par le volume prévue, coupure 11 s avant l'arrêt :

| Précharge | 14 h 24 | 13 h 01 | 7 h 40 | Minimum à 14 h 24 |
| ---: | ---: | ---: | ---: | ---: |
| 7 s | 89,5 | 88,9 | 88,8 | 88,2 |
| 8 s | 90,1 | 89,4 | 89,3 | 89,0 |
| 9 s | 90,6 | 90,0 | 89,9 | 89,7 |
| 10 s | 91,2 (mesuré 91,46) | 90,6 | 90,4 | 90,3 (mesuré 90,49) |

Chaque seconde de précharge vaut environ 0,55 °C sur la moyenne pondérée et
0,7 °C sur le pic. Décision : une fenêtre SSR neuve au départ de la
précharge et de l'écoulement (0.3.30), et **8 s** de précharge réglée.

#### Infusion de 9 h 43 (30 septembre) — précharge de 8 s

[Capture brute](../captures/260930-094308.json) et
[graphique](../captures/260930-094308.html). Première infusion en 0.3.30
(fenêtre SSR neuve au départ de la précharge et de l'écoulement). Consigne
90 °C, départ à 90,16 °C. 67,5 ml comptés, 22,4 g en tasse. Mouture jugée
trop fine : pompe en marche 29,9 s, infusion de 21,7 s, 1,3–1,9 ml/s en
régime.

Contrôles de la fenêtre SSR neuve, tous réussis :

| Contrôle | Attendu | Mesuré |
| --- | --- | --- |
| Départ de la précharge → premier `heater_on` | ≤ 0,2 s | 0,0 s (même échantillon, 0,15 s) |
| Début du remplissage → premier `heater_on` | ≤ 0,2 s | 0,0 s (8,05 s) |
| Énergie SSR / commande, précharge | ≥ 95 % | 99,9 % (8,52 kJ pour 8,53 kJ) |
| Énergie SSR / commande, capture entière | — | 94 % (22,6 kJ pour 24,0 kJ) |

| Temps | Phase | NTC | Commande | Débit amont |
| ---: | --- | ---: | ---: | ---: |
| 0,15–8,05 s | précharge | 90,16 °C, stable | 90 % | 0 |
| 8,05–9,4 s | début du remplissage | 90,2 → 90,3 °C | 45 %, puis ≈ 5 % : débit pas encore mesuré | 0 → 3,6 ml/s |
| 9,4–12,1 s | remplissage | → 91,6 °C | 90 % | 3,6 ml/s |
| 12,1–16,1 s | pré-infusion | pic **92,31 °C** à 14,8 s | 74–90 % | 2,9–3,5 ml/s |
| 16,1–20,9 s | début de l'infusion, montée en pression | 92,0 → 91,3 °C | 78–90 % | 3,1–3,6 ml/s |
| 20,9–27,5 s | infusion en régime | minimum **90,76 °C** à 23,6 s | 32–44 % | 1,3–1,5 ml/s |
| 27,5 s | coupure de fin, 10,5 s avant l'arrêt, 10,4 g en tasse | 92,0 °C | **0 %** | 1,5 ml/s |
| 27,5–38,0 s | fin de l'infusion | → **94,8 °C** à l'arrêt | 0 % | 1,2–1,9 ml/s |
| 38–68 s | récupération | 94,8 → **92,84 °C** 30 s après l'arrêt | 0 % | 0 |

| Indicateur | Prévu | Mesuré |
| --- | --- | --- |
| Pic avant l'infusion | ≈ 92,0 °C | 92,31 °C |
| Minimum pendant l'écoulement | ≈ 90,0 °C | 90,17 °C (début du remplissage) ; 90,76 °C en infusion |
| Coupure de fin | ≈ 11 s avant l'arrêt | 10,5 s |
| 30 s après l'arrêt | ≈ 90,5 °C | 92,84 °C |
| Moyenne NTC pondérée par le volume | ≈ 91,1 °C | 92,00 °C |
| Moyenne NTC pondérée par la tasse | — | **92,55 °C** |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 8,53 kJ | 8,52 kJ |
| Remplissage | 3,35 kJ | 2,64 kJ |
| Pré-infusion | 3,94 kJ | 3,60 kJ |
| Infusion | 8,14 kJ | 7,86 kJ |
| **Total** | **24,0 kJ** | **22,6 kJ** |

Lecture :

- **Le remplissage est couvert** : la NTC ne descend pas sous la consigne
  pendant l'écoulement.
- **La bosse de fin vient de l'appoint du fort débit.** Entre 9,4 et
  20,5 s, ≈ 12 kJ sont envoyés à 75–90 %. Ils atteignent la sonde entre ~27
  et 38 s, alors que la chauffe est coupée et le débit faible. L'eau du
  remplissage est chauffée deux fois : par la précharge, puis par l'appoint.
  La coupure de fin retire ≈ 4,6 kJ, et la précharge en apporte 8,5 kJ.
  Analyse dans [chauffe-infusion.md](chauffe-infusion.md#simulation).
- **L'énergie par ml est celle de 14 h 24** (335 contre 333 J/ml SSR). La
  différence tient à la répartition dans le temps : ici, le fort débit dure
  jusqu'à 20,9 s et l'infusion est plus longue, donc plus de cette chaleur
  arrive avant l'arrêt de la pompe.
- **Le simulateur sous-estime cette infusion** : 90,63 °C prévus en tasse
  avec la commande réelle, contre 92,51 °C mesurés. Même réajusté sur les
  quatre infusions, il ne reproduit pas à la fois 13 h 01 et 9 h 43
  (±1,3 °C).

Rapport entre débit amont et débit en tasse, après les 5 premiers grammes :
1,40 ici, contre 1,21 à 1,37 pour les trois infusions précédentes. Rien
n'indique une recirculation par l'OPV qui gonflerait la commande.

Décision : la consigne vise la moyenne au groupe, avec la moyenne pondérée
par la tasse comme critère (voir
[chauffe-infusion.md](chauffe-infusion.md#objectif)). Précharge de
**5,5 s** au prochain essai.

#### Infusion de 14 h 25 (30 septembre) — précharge de 5,5 s

[Capture brute](../captures/260930-142555.json) et
[graphique](../captures/260930-142555.html). Firmware 0.3.30 inchangé ;
précharge réglée à 5,5 s et mouture grossie. Consigne 90 °C, départ à
89,99 °C. 61,4 ml comptés, 22,5 g en tasse ; pompe en marche 21,7 s,
infusion de 14,4 s, 1,5–2,8 ml/s en régime. Café jugé bon, baisse de
température jugée acceptable.

| Temps | Phase | NTC | Commande | Débit amont |
| ---: | --- | ---: | ---: | ---: |
| 0,15–5,5 s | précharge | 89,99–90,02 °C | 90 % (SSR dès 0,15 s) | 0 |
| 5,5–6,5 s | début du remplissage | 90,0 °C | 45 %, puis 90 % dès 5,9 s | premières impulsions lues à 10–71 ml/s |
| 6,5–9,55 s | remplissage | → 90,3 °C | 86–90 % | 3,4–3,6 ml/s |
| 9,55–12,85 s | pré-infusion | pic **90,36 °C** à 9,9 s | 70–90 % | 2,7–4,0 ml/s |
| 12,85–17,1 s | début de l'infusion, montée en pression | 90,1 → 89,85 → 90,14 °C | 80–90 % | 3,3–3,7 ml/s |
| 17,1 s | coupure de fin, 10,2 s avant l'arrêt, 5,8 g en tasse | 90,14 °C | **0 %** | 3,4 ml/s (8 bar à 18,1 s) |
| 17,1–27,25 s | infusion | minimum **88,84 °C** à 22,0 s, puis 89,65 °C à l'arrêt | 0 % | 1,5–2,8 ml/s |
| 27,25–57,25 s | récupération | minimum **88,65 °C** à 32,9 s → **89,03 °C** 30 s après l'arrêt | 0 %, puis ≤ 8,3 % dès 36,9 s | 0 |

| Indicateur | Prévu | Mesuré |
| --- | --- | --- |
| Moyenne NTC pondérée par la tasse | ≈ 90 °C (±1 °C) | **89,60 °C** |
| Moyenne NTC pondérée par le volume | — | 89,88 °C |
| Pic avant l'infusion | ≈ 90,5 °C | 90,36 °C |
| Minimum pendant l'écoulement | ≈ 88,0–88,5 °C | 88,84 °C |
| Coupure de fin | ≈ 11 s avant l'arrêt | 10,2 s |
| 30 s après l'arrêt | ≈ 90,5–91 °C | 89,03 °C |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 5,78 kJ | 5,52 kJ |
| Remplissage | 4,15 kJ | 4,38 kJ |
| Pré-infusion | 3,37 kJ | 3,00 kJ |
| Infusion | 4,48 kJ | 4,56 kJ |
| **Total jusqu'à l'arrêt** | **17,8 kJ** | **17,5 kJ** |

Lecture :

- **Le critère est tenu** : −0,40 °C sur la moyenne en tasse, dans la
  fourchette prévue. Le creux (−1,16 °C pendant l'écoulement) est jugé
  acceptable au goût.
- **La bosse de fin a presque disparu** (+0,8 °C entre le minimum et
  l'arrêt, contre +4,0 °C à 9 h 43). L'infusion est plus courte et le débit
  plus élevé : la chaleur du fort débit atteint la sonde en grande partie
  après l'arrêt de la pompe, sans aller en tasse.
- **L'état final est 1,5 °C sous la prévision.** La coupure, tombée pendant
  la montée en pression, a retiré l'appoint d'un débit encore fort. La
  récupération a démarré de 88,65 °C.
- **Pas de trou de commande au début du remplissage, par hasard.** À 9 h 43,
  le débit restait proche de 0 pendant ≈ 1 s et la commande tombait vers
  5 %. Ici, les premières impulsions après le silence sont lues à 10–71 ml/s :
  la commande passe au plafond de 90 % dès 5,9 s. La
  [limite connue](chauffe-infusion.md#limites-connues) du premier instant
  du remplissage dépend donc de la phase des premières impulsions.

Décision : garder **5,5 s** de précharge.

#### Infusion de 8 h 29 (1er octobre) — précharge de 5,5 s

[Capture brute](../captures/261001-082959.json) et
[graphique](../captures/261001-082959.html). Écran en 0.3.31, module
capteurs en 0.3.30 ; précharge de 5,5 s. Mouture un peu moins fine qu'à
14 h 25, porte-filtre moins chauffé que d'habitude. Consigne 90 °C, départ à
89,95 °C. 56,8 ml comptés et 21,5 g en tasse à l'arrêt de la pompe (22,5 g
ensuite) ; pompe en marche 19,3 s, infusion de 13,7 s, 1,9–2,5 ml/s en
régime.

| Temps | Phase | NTC | Commande | Débit amont |
| ---: | --- | ---: | ---: | ---: |
| 0,12–5,5 s | précharge | 89,92–89,97 °C | 90 % (SSR dès 0,12 s) | 0 |
| 5,5–6,7 s | début du remplissage | 89,9 °C | 45 %, puis **5 %** de 6,1 à 6,7 s ; SSR éteint de 6,1 à 6,8 s | 0,06 ml/s lus, 2,3 ml passés |
| 6,7–9,9 s | remplissage | → 90,47 °C | 89–90 % | 3,5 ml/s |
| 9,9–11,2 s | pré-infusion, premières gouttes après ≈ 4 ml | → 90,70 °C | 84–90 % | 3,3–3,6 ml/s |
| 11,2–15,7 s | début de l'infusion, montée en pression | pic **90,71 °C** à 11,6 s, puis 89,23 °C | 80–90 % | 3,1–3,9 ml/s |
| 15,7 s | coupure de fin, 9,2 s avant l'arrêt, 5,6 g en tasse | 89,23 °C | **0 %** | 3,5 ml/s (3,5 bar ; 8 bar à 17,6 s) |
| 15,7–24,9 s | infusion | minimum **87,03 °C** à 21,8 s, puis 87,20 °C à l'arrêt | 0 % | 3,5 → 1,9–2,5 ml/s |
| 24,9–54,9 s | récupération | minimum **86,66 °C** à 30,8 s → **87,76 °C** 30 s après l'arrêt | 0 %, puis ≤ 17,2 % dès 31,7 s | 0 |

| Indicateur | Prévu | Mesuré |
| --- | --- | --- |
| Moyenne NTC pondérée par la tasse | 89,6 °C ±0,7 °C | **88,39 °C** |
| Moyenne NTC pondérée par le volume | — | 89,25 °C |
| Pic avant l'infusion | — | 90,71 °C |
| Minimum pendant l'écoulement | ≈ 88,8 °C | 87,03 °C |
| Coupure de fin | ≈ 11 s avant l'arrêt | 9,2 s |
| 30 s après l'arrêt | ≈ 89 °C | 87,76 °C |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 5,84 kJ | 6,12 kJ |
| Remplissage | 3,98 kJ | 4,08 kJ |
| Pré-infusion | 1,36 kJ | 1,32 kJ |
| Infusion | 4,77 kJ | 4,92 kJ |
| **Total jusqu'à l'arrêt** | **16,0 kJ** | **16,4 kJ** |

Lecture :

- **Hors de la fourchette** : −1,61 °C sur la moyenne en tasse, 1,21 °C
  sous 14 h 25 aux mêmes réglages. La mouture et la chauffe du porte-filtre
  ayant changé, l'essai ne tranche pas la répétabilité de la précharge
  fixe.
- **Rondelle plus rapide, comme attendu avec une mouture moins fine.** Les
  premières gouttes arrivent après ≈ 4 ml de pré-infusion (≈ 11 ml à
  14 h 25), la pré-infusion ne dure que 1,3 s (3,3 s) et la pression monte
  plus tôt. La coupure tombe donc en pleine montée en pression, avec encore
  3,5 ml/s, et le creux descend à 2,97 °C sous la consigne (1,16 °C à
  14 h 25). Aucune bosse de fin : la NTC baisse encore 6 s après l'arrêt.
- **Le modèle ne sépare pas les deux infusions.** Rejoué avec la commande
  réelle (`simulate_boiler.py`), il prévoit 87,71 °C en tasse à 8 h 29 et
  87,81 °C à 14 h 25, pour 88,39 et 89,60 °C mesurés. Il prévoit bien le
  minimum de 8 h 29 (86,81 contre 86,66 °C), mais l'écart entre les deux
  infusions ne vient ni de l'hydraulique ni de la commande telles qu'il les
  représente : c'est le facteur manquant déjà relevé dans
  [chauffe-infusion.md](chauffe-infusion.md#simulation). Le porte-filtre
  moins chauffé refroidit l'eau en tasse, mais n'agit pas directement sur
  la NTC ; le panier de mesure le verra.
- **Trou de commande au début du remplissage**, comme à 9 h 43 : le débit
  lu reste à 0,06 ml/s pendant ≈ 0,9 s et le SSR s'éteint 0,7 s, soit
  ≈ 0,8 kJ manquants (≈ 0,5 °C sur la moyenne en tasse). Voir la
  [limite connue](chauffe-infusion.md#limites-connues).
- **Récupération lente** : la commande ne dépasse pas 17,2 % et la NTC
  reste 2,2 °C sous la consigne 30 s après l'arrêt.

Décision : consigner sans rien changer ; garder **5,5 s** de précharge.

#### Infusion de 10 h 01 (4 octobre) — précharge de 5,5 s, mouture trop fine

[Capture brute](../captures/261004-100128.json) et
[graphique](../captures/261004-100128.html). Première infusion capturée
avec l'écran en 0.3.32 ou plus : la fin du remplissage par hausse de
pression relative le confirme. Le pont de 4 676 Ω reproduit la température
enregistrée ; l'échantillon ne distingue pas 0.3.33 de 0.3.34, que le
changement de courbe laisse à la même cible physique à 0,1 °C près.
Nouveau paquet du même café, mouture devenue plus fine sans changement de
réglage. Pré-infusion à **60 %** : la migration du schéma 7 au schéma 8
a conservé la valeur enregistrée, le défaut de 35 % ne vaut que pour une
configuration neuve. Consigne 90 °C, départ à 90,09 °C. 65,5 ml comptés,
21,5 g en tasse à l'arrêt de la pompe (22,2 g ensuite) ; pompe en marche
41,4 s, infusion de 34,4 s, 0,8–1,3 ml/s en régime.

| Temps | Phase | NTC | Commande | Débit amont |
| ---: | --- | ---: | ---: | ---: |
| 0,03–5,58 s | précharge | 90,02–90,11 °C | 90 % (SSR dès 0,18 s) | 0 |
| 5,58–7,0 s | début du remplissage | 90,1 °C | 90 %, puis **4,6 %** de 6,0 à 7,0 s | 0,04–0,05 ml/s lus, 2,6 ml passés |
| 7,0–8,58 s | remplissage, fin par hausse de pression (+0,12 bar sur un plancher de 0,21 bar) à 9,9 ml | → 90,45 °C | 90 % | 3,6 ml/s |
| 8,58–12,58 s | pré-infusion à 60 %, **fin au bout des 4 s** sans goutte en tasse | pic **91,07 °C** à 10,7 s | 67–90 % | 2,6–3,8 ml/s |
| 12,58–17,3 s | début de l'infusion, montée en pression ; 8 bar à 17,2 s, premières gouttes à 17,3 s après **38 ml** | 90,97 → 88,92 °C | 73–90 % | 2,9–3,6 ml/s |
| 17,3–36,08 s | infusion | minimum **88,71 °C** à 19,6 s, puis 94,02 °C | 69 % à 17,6 s, puis 20–45 % dès 19,3 s | 2,6 → 0,6–1,6 ml/s |
| 36,08 s | coupure de fin, 10,9 s avant l'arrêt, 12,7 g en tasse | 94,02 °C | **0 %** | 0,84 ml/s |
| 36,08–46,98 s | infusion | maximum **94,45 °C** à 41,1 s, puis 93,90 °C à l'arrêt | 0 % | 0,8–1,3 ml/s |
| 46,98–66,88 s | récupération (20 s capturées) | → **91,97 °C** 20 s après l'arrêt | 0 % | 0 |

| Indicateur | 14 h 25 (30/09) | Mesuré |
| --- | --- | --- |
| Moyenne NTC pondérée par la tasse | 89,60 °C | **92,52 °C** |
| Moyenne NTC pondérée par le volume | 89,88 °C | 91,21 °C |
| Pic avant l'infusion | 90,36 °C | 91,07 °C |
| Minimum pendant l'écoulement | 88,84 °C | 88,71 °C |
| Coupure de fin | 10,2 s avant l'arrêt | 10,9 s |
| État final | 89,03 °C 30 s après l'arrêt | 91,97 °C 20 s après l'arrêt |

| Phase | Commande | SSR |
| --- | ---: | ---: |
| Précharge | 5,94 kJ | 5,88 kJ |
| Remplissage | 2,26 kJ | **1,44 kJ** |
| Pré-infusion | 4,01 kJ | 3,96 kJ |
| Infusion | 11,15 kJ | 11,17 kJ |
| **Total jusqu'à l'arrêt** | **23,4 kJ** | **22,5 kJ** |

Durées de phase comparées aux infusions précédentes :

| | 14 h 25 (30/09) | 8 h 29 (01/10) | 7 h 35 (02/10) | **10 h 01 (04/10)** | 9 h 43 (30/09), trop fin |
| --- | ---: | ---: | ---: | ---: | ---: |
| Remplissage | 4,05 s, 13,6 ml | 4,40 s, 14,5 ml | 4,70 s, 15,7 ml | **3,00 s, 9,9 ml** | 4,05 s, 13,6 ml |
| Pré-infusion | 3,3 s, 10,4 ml | 1,3 s, 4,1 ml | 1,6 s, 4,6 ml | **4,0 s, 12,5 ml** | 4,0 s, 12,2 ml |
| Volume aux premières gouttes | 24,6 ml | 19,7 ml | 22,0 ml | **38,0 ml** | 32,5 ml |
| Démarrage de la pompe → 8 bar | 12,6 s | 12,1 s | 12,1 s | **11,6 s** | 12,3 s |
| Infusion | 14,4 s | 13,7 s | 14,2 s | **34,4 s** | 21,7 s |
| Pompe en marche | 21,7 s | 19,3 s | 20,4 s | **41,4 s** | 29,7 s |
| Tasse / volume amont, à partir de 8 bar + 2 s | 0,73 g/ml | 0,70 | 0,72 | **0,79** | 0,73 |

Lecture :

- **Seul le régime à 9 bar change.** Le temps jusqu'à 8 bar reste à
  11,6–12,6 s ; le débit en pression est divisé par deux et l'infusion
  dure 2,4 fois plus longtemps. Le remplissage raccourcit à cause de la
  0.3.32, pas de la mouture : 9,9 ml, dans les 7,5–16,2 ml rejoués lors de
  ce changement.
- **La pré-infusion n'a pas fait de pause.** À 60 %, elle a admis 12,5 ml
  et appelé ≈ 4 kJ de chauffe ; aucune goutte n'est arrivée avant la fin
  de ses 4 s. Avec la pause à 35 %, la commande serait restée à 3,5 %.
- **Double chauffe du remplissage, comme à 9 h 43.** La chaleur du fort
  débit (précharge, remplissage, pré-infusion, montée en pression) atteint
  la sonde vers 25–37 s, pompe toujours en marche : la NTC remonte de
  5,7 °C au-dessus de son minimum, alors que ≈ 19 des 21,3 g de la tasse
  tombent entre 19 et 47 s. La moyenne en tasse, **+2,52 °C**, rejoint
  celle de 9 h 43 (+2,55 °C, précharge de 8 s) avec une précharge de
  5,5 s. La coupure de fin ne retire que ≈ 3 kJ d'appoint sur 11 s, à
  ≈ 25 % de commande.
- **Pas de recirculation par l'OPV** : la tasse reçoit 0,79 g par ml admis
  en régime, dans la plage des autres infusions.
- **Trou de commande au début du remplissage**, comme à 9 h 43 et 8 h 29 :
  débit lu à 0,04–0,05 ml/s pendant ≈ 1 s, 0,8 kJ non fournis par le SSR.
  La règle des 3,5 % de la 0.3.32 ne vaut que pour la pré-infusion.
- **Le modèle sous-estime l'infusion** : rejoué avec la commande réelle
  (`simulate_boiler.py`), il prévoit 91,31 °C en tasse (mesuré 92,52 °C) et
  un minimum de 87,31 °C (mesuré 88,71 °C).

Décision : régler la pré-infusion à **35 %** et grossir la mouture ; garder
**5,5 s** de précharge.

#### Infusion de 13 h 24 (4 octobre) — précharge de 8 s, remboursement

[Capture brute](../captures/261004-132439.json) et
[graphique](../captures/261004-132439.html). Écran en 0.3.37 ou 0.3.38 :
premier essai du remboursement de la précharge. Configuration
`configs/261004-102724.json` : précharge de 8 s, remplissage de 6 s au
plus avec sortie à +0,1 bar, pré-infusion de 4 s à 35 % avec sortie au
poids, arrêt à 22 g.

| Phase | Durée | Volume | Commande | SSR | NTC |
| --- | ---: | ---: | ---: | ---: | --- |
| Précharge | 8,05 s | — | 703 %·s | 8,40 kJ | 89,55 → 89,90 °C |
| Remplissage | 4,50 s | 15,4 ml | 316 %·s | 3,36 kJ | → 92,06 °C |
| Pré-infusion | 4,00 s | 2,0 ml | 139 %·s | 1,80 kJ | → 91,75 °C |
| Infusion | 24,10 s | 46,4 ml | **89 %·s** | **0,84 kJ** | → 84,17 °C |

Lecture :

- **Moyenne en tasse de 87,58 °C** (−2,42 °C), 90,17 °C pondérée par le
  volume. Minimum de 82,79 °C, 10 s après l'arrêt de la pompe.
- **Chauffe à 0 % pendant 22 des 24 s d'infusion.** Remboursement de
  16,6 à 28,4 s, appoint à ≈ 37 % jusqu'à 30,6 s, puis coupure de fin,
  10,2 s avant l'arrêt. Voir
  [chauffe-infusion.md](chauffe-infusion.md#coupure-de-fin-supprimée-en-0339).
- **Le modèle ne reproduit pas la tasse** : 89,57 °C prévus avec la
  commande réelle. La coupure de fin explique la récupération basse, pas
  cet écart.
- **Le remplissage s'est arrêté sur la pression**, à 4,45 s et 15,4 ml,
  avant les 6 s : plancher de 0,26 bar, +0,10 bar à 12,4 s.
- **La pause de pré-infusion n'a rien admis** : 0,40–0,42 bar, 2 ml en
  4 s, aucune goutte. Le 1er et le 2 octobre, après 14,8 et 16,2 ml de
  remplissage, la goutte était tombée en 1,3 et 1,6 s. Ici, la première
  goutte arrive à 35,1 ml et 3,9 bar, pendant la montée en pression.

Décision : supprimer la coupure de fin (0.3.39) ; terminer le remplissage
à 1 bar absolu, avec 10 s de secours (0.3.40).

## Procédures de mesure

### Vérification de la conversion à chaud

Après stabilisation à la consigne, sauvegarder plusieurs réponses
`GET /telemetry` **avant extinction**, sans purge juste avant. Vérifier
`temperature.boiler.valid` et relever ensemble `temperature.boiler.c`,
`temperature.boiler.sensor_c`, `temperature.boiler.ntc_a0_raw` et
`temperature.boiler.ntc_a1_raw`. Avec la courbe actuelle, une consigne de
90 °C correspond à environ **3,28 kΩ**. Éteindre ensuite, débrancher la sonde
et mesurer rapidement sa résistance au multimètre ; une légère hausse est
attendue avec le refroidissement. Cette comparaison vérifie la chaîne ADC,
pas la conversion résistance/température : il faut pour cela une température
indépendante, stabilisée, au voisinage de la NTC.

Pour l'écart jusqu'au panier, mesurer la température de l'eau **pendant
l'écoulement** avec une sonde rapide placée au plus près de la sortie, à
débit et montage identiques ; la température du métal au repos ou du panier
après purge ne la remplace pas.

### Vérification à froid

Laisser `heating.enabled=false` et la machine au repos toute la nuit. Au
redémarrage, vérifier que la chauffe est toujours désactivée. Avant toute
purge, relever la température ambiante près de la chaudière et celle de l'eau
du réservoir, puis sauvegarder la télémétrie :

```sh
curl -fsS -H "Authorization: Bearer ${COFFEEFLOW_HTTP_TOKEN}" \
  "http://${COFFEEFLOW_IP}/telemetry" > captures/cold-before.json
uv run firmware/tools/purge.py 8
uv run firmware/tools/download_hf_capture.py
```

Faire deux purges identiques de 8 s, en mesurant le panier immédiatement
après chacune et en le vidant entre les deux. `purge.py` n'enregistre que les
valeurs affichées dans le terminal. `record_heating.py --mode monitor`
requiert `heating.enabled=true` et ne convient pas à cet essai. Comparer
`temperature.boiler.sensor_c` à la température stabilisée de la machine
(42,1 kΩ ≈ 27,5 °C à la sonde).

### Montée depuis l'ambiante

`firmware/tools/record_heating.py` automatise les étapes 3 et 4 ci-dessous,
coupe ensuite la chauffe et écrit un JSON dans `captures/` (`--output` pour
choisir le fichier). Il lit `COFFEEFLOW_HTTP_TOKEN` et `COFFEEFLOW_IP`.

1. Envoyer `POST /config` avec `{"version":7,"heating":{"enabled":false}}`,
   puis laisser refroidir.
2. Allumer la machine et activer le Wi-Fi. La valeur `false` reste persistée.
3. Envoyer `POST /config` avec `{"version":7,"heating":{"enabled":true}}`.
4. Interroger `GET /telemetry` toutes les 500 ms et enregistrer `uptime_ms`,
   `temperature.boiler.c`, sa validité et son âge, `heating.power_pct`,
   `heating.accepted_power_pct`, `heating.on` et `heating.target_c`, avec un
   horodatage client. L'écho de puissance et `heating.on` viennent du module
   capteurs ; une absence d'écho frais doit apparaître dans l'analyse.

Le mode vapeur reste à ajouter : il devra sélectionner une cible propre,
bloquer l'infusion et garder la purge disponible.

## Remplacement prévu par une PT1000

La NTC installée est physiquement assez grande. On lui attribuait un retard de
mesure d'environ 8 s, mais les captures ne séparent pas son temps de réponse
propre du transport de l'eau et de la chaleur dans la chaudière. Le chemin eau
admise → NTC ne dure que 6,4 s en moyenne, transport compris. Sur banc, la
PT1000 répond avec t63 ≈ 2,3 s et t90 ≈ 5–6 s. La NTC passera par le même banc
avant qu'on choisisse la sonde à monter : voir
[l'essai du 29 septembre](ntc_ads1115_calibration.md#essai-pt1000-sur-banc-29-septembre-2026).

Le remplacement prévu est une **PT1000 iOVEO 012EF02202**, filetée
**G 1/8**, avec une partie immergée en acier inoxydable de **9 mm de long et
5,5 mm de diamètre**, deux fils silicone. L'immersion directe et la faible
longueur devraient réduire le retard ; la constante de temps dépendra encore
de la construction interne, de la gaine et de la circulation d'eau.

Une PT1000 IEC 60751 nominale vaut environ 1,347 kΩ à 90 °C, 1,385 kΩ à
100 °C et 1,400 kΩ à 104 °C. La résistance basse du pont sera remplacée par
une **4,7 kΩ** pour limiter l'autoéchauffement : sous 3,3 V et à 100 °C, le
courant passera d'environ 0,92 mA à **0,54 mA** et la dissipation dans la
sonde d'environ 1,2 mW à **0,41 mW**. A1 sera proche de 2,55 V.

Le premier montage **conserve l'ADS1115** plutôt que d'ajouter un MAX31865.
Avec le PGA ±4,096 V (125 µV par pas), le pont fournira environ 1,6 mV/°C
autour de 100 °C, soit près de 13 pas par degré (≈ 0,08 °C par pas), ce qui
suffit à la régulation. Les contrôles de validité bruts acceptent les
tensions prévues. La valeur réelle de la résistance fixe devra être mesurée
et reportée dans `kBoilerNtcFixedOhm` ; une résistance métal de précision
évitera la dérive du pont avec l'échauffement du boîtier.

Le MAX31865 reste une possibilité si la détection matérielle des fils ouverts
ou en court-circuit devient prioritaire ; son mode deux fils ne retire pas la
résistance des conducteurs. Autour de 100 °C, la pente est proche de
3,8 Ω/°C : 1 Ω aller-retour crée environ 0,26 °C d'erreur. Les 40 à 50 cm de
fil aller-retour représentent quelques centièmes à un dixième d'ohm, donc
quelques centièmes de degré ; les contacts peuvent peser autant. Le firmware
devra remplacer le modèle Beta par la loi Callendar–Van Dusen. Avant ce
changement, relever les codes ADS1115 à froid et à chaud, puis comparer le
retard des deux sondes sur une purge identique.

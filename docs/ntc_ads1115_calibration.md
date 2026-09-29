# Calibration NTC chaudière via ADS1115

> Remplacement prévu : PT1000 iOVEO 012EF02202, G 1/8, immersion inox
> 9 × 5,5 mm, deux fils silicone. Le montage ADS1115 peut être réutilisé, mais
> la conversion Beta décrite dans ce document devra être remplacée par une loi
> Callendar–Van Dusen. Avec 20 à 25 cm de câble, la résistance des deux fils
> devrait être négligeable ; elle sera simplement vérifiée à la calibration.

Le montage conservera l'ADS1115, mais remplacera la résistance fixe mesurée de
2,193 kΩ par une **4,7 kΩ** de précision. Autour de 100 °C, la sensibilité
calculée restera d'environ 1,6 mV/°C, soit près de 13 codes par degré avec le
PGA ±4,096 V. Le courant de mesure sera ramené d'environ 0,92 à 0,54 mA et la
dissipation dans la PT1000 d'environ 1,2 à 0,41 mW. Un MAX31865 n'est pas
nécessaire à cette résolution. Sur la longueur prévue, une compensation des
fils n'apporterait qu'un gain négligeable. Il reste intéressant uniquement si ses diagnostics
RTD dédiés justifient une carte et une liaison SPI supplémentaires.

La valeur exacte de la nouvelle résistance devra être mesurée puis utilisée
dans la formule ratiométrique et dans <code>kBoilerNtcFixedOhm</code> ; le firmware
doit rester réglé sur 2,193 kΩ tant que le remplacement physique n'est pas fait.

## Contexte

L'objectif est de mesurer la température de la chaudière d'une machine à
café avec la sonde NTC 1/8" déjà installée, en utilisant un ADS1115 I²C
relié à un ESP32-S3-WROOM.

L'alimentation principale est en 5 V. L'ESP32 du Waveshare est alimenté par
sa carte ; le breakout ADS1115 reçoit **3,3 V du connecteur I2C du Waveshare**.
Le pont NTC reçoit le **3V3 et le GND du port Sensor AD du même écran**.
Le LDO AMS1117 dédié au pont dans le montage initial a été retiré.

La plage utile est principalement \~80--100 °C en mode café et jusqu'à
\~120--130 °C en mode vapeur.

## Schéma de montage

Le rail 3,3 V du port Sensor AD est mesuré par A0 afin de rendre le
calcul de résistance ratiométrique. A1 mesure le point milieu du pont NTC.

``` text
Waveshare Sensor AD 3V3 ----+---- A0 ADS1115
                            |
                           NTC
                            |
                            +---- A1 ADS1115
                            |
                        R = 2.193 kΩ
                            |
Waveshare Sensor AD GND ----+---- GND proche de l'ADS1115

Waveshare I2C 3V3 / GND ---------- alimentation ADS1115
```

Notes :

-   `A0` mesure la tension réelle du 3V3 qui alimente le pont.
-   `A1` mesure la tension au point milieu NTC / résistance fixe.
-   La résistance fixe nominale est 2,2 kΩ ; sa valeur mesurée est
    **2,193 kΩ**.
-   Le GPIO/AD du port Sensor n'est pas utilisé.
-   Le bas de la résistance fixe et le GND de l'ADS1115 doivent avoir une
    référence de potentiel proche. Un simple raccordement électrique entre
    deux GND éloignés ne garantit pas l'absence de chute de tension sous charge.
-   Pour mesurer directement \~3,3 V sur A0, configurer l'ADS1115 avec
    une plage compatible, typiquement **±4,096 V**.

## Calcul de la résistance NTC

Avec :

-   `R_FIXED = 2193 Ω`
-   `V_SUPPLY = tension mesurée sur A0`
-   `V_DIV = tension mesurée sur A1`

le pont est :

``` text
V_SUPPLY -> NTC -> V_DIV -> R_FIXED -> GND
```

La résistance de la NTC est donc :

``` text
R_NTC = R_FIXED * (V_SUPPLY / V_DIV - 1)
```

Si A0 et A1 sont lus avec exactement le même réglage PGA de l'ADS1115,
le rapport peut être calculé directement à partir des valeurs ADC :

``` text
R_NTC = 2193 * (ADC_A0 / ADC_A1 - 1)
```

Cela rend la mesure pratiquement indépendante de la valeur absolue du
3,3 V et de la précision absolue de la référence interne de l'ADS1115.
L'erreur de gain de l'ADS1115 s'annule dans le rapport ; restent la valeur
de `R_FIXED`, qui entre en facteur direct dans la résistance calculée, et le
GND de retour de la résistance. Une erreur de 0,1 % sur `R_FIXED` vaut environ
0,3 °C avec la PT1000 : utiliser une résistance à 0,1 %, pas le multimètre,
dont l'incertitude est plus grande que celle de la résistance.

### Correction du retour GND et vérification à froid

Le montage initial utilisait un LDO AMS1117 pour le pont et retournait le
bas de la résistance fixe au Wago GND de l'alimentation 5 V. Le GND de
l'ADS1115, amené par le port I2C du Waveshare, se trouvait **10–13 mV** au-dessus
de ce Wago sous tension. Le point milieu mesurait environ **0,143 V** par
rapport au GND de l'alimentation, mais environ **0,133 V** par rapport au GND
de l'ADS1115 ; ce dernier lisait donc correctement sa propre tension A1,
tout en surestimant la résistance de la NTC à cause de la référence GND du pont.

Le pont utilise maintenant **3V3 et GND du port Sensor AD** du Waveshare, sans
le LDO. Après modification, un relevé `/telemetry` à froid donne
`boiler_ntc_a0_raw = 26305`, `boiler_ntc_a1_raw = 1151`, soit **3,288125 V**
sur A0, **0,143875 V** sur A1 et **47,926 kΩ** calculés. La NTC débranchée
et mesurée au multimètre juste après était à **47,9 kΩ** : l'écart est
d'environ **26 Ω**, soit **0,05 %**. Cette concordance valide la lecture
de résistance du montage corrigé à ce point froid ; elle ne valide pas à
elle seule la courbe résistance/température à chaud.

## Conversion résistance -\> température

Le modèle Beta d'une NTC est :

``` text
1/T = 1/T0 + (1/B) * ln(R/R0)
```

où :

-   `T` est la température en kelvins ;
-   `T0 = 298.15 K` pour 25 °C ;
-   `R` est la résistance NTC mesurée ;
-   `R0` est la résistance extrapolée à 25 °C ;
-   `B` est la constante Beta de la sonde.

Pour obtenir directement la température en °C :

``` text
T_C = 1 / (1/T0 + ln(R_NTC/R0)/B) - 273.15
```

### Paramètres compilés de 0.3.9 jusqu'à l'essai du 25 septembre 2026

Les mesures directes de la sonde démontée, détaillées dans
[`chauffe-chaudiere.md`](chauffe-chaudiere.md#mesures-directes-de-la-sonde-démontée),
ont été ajustées par un modèle Beta (`R25 ≈ 47,2 kΩ`, `B ≈ 3 922 K`). Le
firmware utilisait des valeurs nominales proches de cet ajustement :

``` text
R0 = 47000 Ω
B  = 3950 K
T0 = 298.15 K
offset = 0 °C
```

Cette conversion estime la température locale de la sonde ; les points chauds,
relevés pendant le refroidissement, limitent sa précision. Avec la même
consigne enregistrée de 90 °C, la chaudière sera moins chaude qu'avec
l'ancienne courbe proche de l'affichage Gicar : 2,92 kΩ représentent environ
104 °C avec la nouvelle courbe, contre 90 °C auparavant. La consigne et les
gains de chauffe devront être validés sur la machine.

Le firmware 0.3.8 compilait l'ajustement direct `47 200 Ω / 3 922 K`. À
résistance identique, les valeurs nominales de 0.3.9 indiquent environ
0,7 °C de moins vers 90 °C et 1,2 °C de moins vers 130 °C.

L'[essai du 25 septembre 2026](chauffe-chaudiere.md)
conserve `R25 = 47 kΩ` mais porte `Beta` à **4 630 K** pour rapprocher la
cible café de l'affichage Gicar. Les valeurs ci-dessus décrivent la version
0.3.9 et les captures prises avant cet essai.

### Paramètres historiques jusqu'au firmware 0.3.7

Les mesures actuelles, en excluant volontairement l'ancienne mesure très
incertaine à \~45 °C, donnent approximativement :

``` text
R0 ≈ 27.3 kΩ @ 25 °C
B  ≈ 3730 K
```

Valeurs provisoires utilisables pour le développement :

``` text
R0 = 27290 Ω
B  = 3728 K
T0 = 298.15 K
```

Ces constantes ont été remplacées en 0.3.8 après les mesures directes de
la sonde démontée.

**Essai firmware 0.3.5 :** un décalage de `-18 °C` a été essayé pour comparer
la température en tasse ; de la vapeur est sortie pendant la purge. L'offset
est revenu à `0 °C` en 0.3.6. Les mesures de résistance et les constantes
ci-dessous n'ont pas été modifiées.

## Mesures de la NTC

Mesures effectuées manuellement sur la sonde existante de la machine :

    Température   Résistance NTC Remarque
  ------------- ---------------- ---------------------------
          29 °C         22.55 kΩ mesure basse température
          32 °C         20.78 kΩ mesure basse température
          36 °C         18.00 kΩ mesure basse température
          80 °C          3.85 kΩ
          89 °C          3.02 kΩ
          90 °C          2.89 kΩ
         100 °C          2.27 kΩ
       \~115 °C          1.48 kΩ température approximative

La mesure précédemment faite autour de 45 °C a été supprimée car sa
température était estimée à ±5 °C et elle était nettement moins fiable.

### Limites des mesures

Pour les mesures à chaud, la procédure était :

1.  lire la température entière affichée par la machine ;
2.  éteindre la machine ;
3.  débrancher la NTC ;
4.  connecter le multimètre ;
5.  mesurer sa résistance.

La chaudière peut perdre environ 0,5 °C pendant cette manipulation. La
température affichée par la machine n'a par ailleurs pas de décimales.

Les points doivent donc être considérés comme des données de calibration
expérimentales avec une incertitude non négligeable, et non comme des
références métrologiques.

## Détermination historique de la courbe Gicar

Les points situés entre 80 et 115 °C sont cohérents avec un modèle NTC
Beta proche de :

``` text
B ≈ 3728 K
R25 ≈ 27.29 kΩ
```

Une régression excluant la mesure peu fiable à \~45 °C avait donné un
résultat très proche :

``` text
B ≈ 3734 K
R25 ≈ 27.43 kΩ
```

Cette proximité indique qu'un modèle provisoire autour de :

``` text
B ≈ 3730 K
R25 ≈ 27.3 kΩ
```

est raisonnable pour commencer l'implémentation.

La résistance fixe de **2,193 kΩ** est bien adaptée : elle est proche de
la résistance de la NTC autour de 100 °C, ce qui donne une bonne
sensibilité du pont dans la zone principale de régulation de la
chaudière.

Une régression définitive devra comparer au minimum :

1.  le modèle Beta ;
2.  éventuellement une courbe Steinhart-Hart si elle améliore
    significativement les résidus sur la plage 25--130 °C.

## Étalonnage de la sonde en bains, avant installation

À faire avec le vrai montage `screen` (même ADS1115, même 4,7 kΩ, mêmes canaux
et même PGA, sonde au bout de rallonges sur le port Sensor AD), pas avec un
montage proto dont le retour GND serait différent. Enregistrer les **codes bruts**
`ntc_a0_raw` / `ntc_a1_raw`, pas seulement la température : le rapport
`a0/a1 − 1` vaut `R_sonde / R_fixe` et permet de recalculer l'étalonnage si
`R_fixe` change.

```sh
COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=… uv run firmware/tools/record_probe.py
```

`record_probe.py` interroge `/telemetry` à 5 Hz jusqu'à Ctrl-C et écrit
`captures/probe-<date>.json`. Plonger la sonde dans les bains pendant
l'enregistrement, puis retrouver les plateaux sur un tracé des codes. Écarter les
échantillons répétés grâce à `age_ms`.

| Bain | Rôle | Remarques |
|---|---|---|
| Glace pilée et un peu d'eau | point d'ajustement à 0 °C | remuer, sonde entièrement immergée sans toucher le fond |
| Eau bouillante | point d'ajustement | le point d'ébullition dépend de la pression locale absolue (environ −1 °C par 300 m) ; en pleine eau, sans toucher le fond |
| Eau à température ambiante, deux thermomètres | validation, pas ajustement | peu informatif sur la pente |

Pas de point vers 60 °C : l'eau refroidit en continu et la température n'est
jamais stable. Ne pas immerger la sortie des fils silicone si elle n'est pas
scellée.

Si `R_fixe` change ensuite (carte ADS1115 dédiée), refaire au moins le point à
0 °C sur la carte finie : une erreur sur `R_fixe` est un défaut de gain que ce
seul point corrige au premier ordre.

### Essai PT1000 sur banc (29 septembre 2026)

Sonde hors de la machine, sur le pont de l'écran : PT1000 entre le 3V3 du port
Sensor AD et A1, une 4,7 kΩ (tolérance non marquée) entre A1 et GND, PGA
±4,096 V. Le firmware restait réglé pour la NTC : il marquait la température
`missing`, parce que la loi Beta donne environ 179 °C pour 1,1 kΩ, au-delà de
la limite de 160 °C. Seuls les codes bruts ont servi. Conversion :
Callendar–Van Dusen IEC 60751, `R = R_fixe × (a0/a1 − 1)`.

| Capture `captures/probe-20260929-…` | Condition | Référence | Lecture (R_fixe = 4676 Ω) |
|---|---|---|---|
| `204302-235101` | air calme, 60 s | Netatmo 25,4 °C | 25,4 °C (A1 ≈ 21303) |
| `204855-502215` | eau du robinet, pointe seule | thermomètre 22,9 °C | 23,2 °C (A1 ≈ 21330) |
| `210506-779353` | glace pilée remuée, pointe seule | 0 °C (thermomètre −0,1 °C) | A1/A0 = 0,82376–0,82389 |
| `212531-670526` | ébullition, 966,1 hPa | 98,68 °C (thermomètre 100,1 °C) | 98,25–98,43 °C |

**R_fixe = 4676 Ω**, déduite du point de glace par
`R_fixe = 1000 × r / (1 − r)`, avec `r = A1/A0`. On a retenu le plateau le plus
bas, car la conduction par le corps ne peut que faire lire trop haut. Supposer
le bain à −0,1 °C donnerait 4672 Ω, soit 0,06 °C à 100 °C. Le multimètre
indique 4,68 kΩ, ce qui est cohérent, mais sa résolution ne suffit pas pour
étalonner : 10 Ω valent environ 0,8 °C à 100 °C. Avec la valeur nominale de
4700 Ω, toutes les lectures étaient trop hautes d'environ 1,5 °C.

**Point d'ébullition : −0,3 °C**, dans la tolérance de la classe B (±0,8 °C à
100 °C). La pente est donc validée sans autre correction entre 0 et 100 °C.
L'écart-type par échantillon atteint 1,1 à 1,5 °C dans l'eau bouillante, à
cause des bulles et de la turbulence, contre environ 0,2 °C en bain calme.
Le thermomètre de cuisine lit environ +1,4 °C trop haut à l'ébullition et
reste juste à 0 °C : les points fixes servent de référence, pas lui.

**Bruit à 5 Hz, bain calme :** environ 2,5 codes d'écart-type sur A1, soit
environ 0,2 °C, contre 0,8 code sur A0. Il vient donc du côté de la sonde :
captation par les fils libres ou mouvements d'air. Une moyenne sur 1 s
suffit.

**Temps de réponse.** Les durées sont mesurées depuis le début du geste
d'immersion, qui est compris dedans, pointe inox seule immergée et bain
agité à la main.

| Échelon | t63 | t90 |
|---|---|---|
| 24 °C → glace, corps à l'ambiante | ≈ 2 s | ≈ 5 s, puis traîne d'environ 40 s sur les 3 derniers °C |
| ≈ 7 °C → glace, corps encore froid | — | ≈ 8 s jusqu'au plateau |
| 24 °C → ébullition | 3,2 s | 6,7 s |
| 66 à 75 °C → ébullition, trois replongées | 2,3–2,4 s | 4,8–5,6 s |

La traîne vient du filetage resté à l'air ambiant, qui conduit la chaleur
vers la pointe. Sur la chaudière, le filetage est serré métal contre métal
et suit la température de la paroi, pas celle de l'eau : cette traîne de banc
n'y existe pas sous cette forme. En revanche, une paroi plus froide que l'eau
biaiserait la mesure vers le bas.

**Pièges constatés pendant l'essai :**

- Une sonde mouillée sortie du bain reste proche de la température de l'eau,
  parce que l'eau s'évapore à sa surface. Pour observer la reprise, il faut
  la sécher.
- Ajouter de l'eau à un bain de glace qui n'a plus assez de glace le fait
  remonter à 2–5 °C. Ce palier n'est plus un point de référence.

**Comparaison avec la NTC, décision en attente.** Chaque démontage de la sonde
sur la chaudière impose de vider la chaudière et de refaire le joint. On ne
monte donc aucune sonde à titre d'essai : on choisit sur banc, puis on monte
une seule fois. Au prochain démontage, prévu pour remplacer le Loxeal 53-14
par du 58-11 alimentaire, la NTC passe sur le même banc avec sa 2,193 kΩ. Le
firmware n'a pas besoin de changer, puisque les codes bruts suffisent. Dans la
glace, A1 descend vers 360 codes (≈ 158 kΩ) avec environ 18 codes/°C.

Le protocole est identique à celui de la PT1000 : même repère d'immersion
(pointe seule), sonde sèche stabilisée 2 min à l'air avant chaque échelon,
agitation similaire, au moins 90 s par bain, et une capture par bain. On
enregistre deux échelons, ambiante → glace et ambiante → ébullition, plus au
moins trois replongées depuis environ 70 °C, qui sont les échelons les plus
propres. On compare t63 et t90.

Les modèles des captures d'infusion bornent la constante propre de la NTC à
quelques secondes : le chemin eau admise → NTC ne dure que 6,4 s en moyenne,
hydraulique comprise. Les retards de régulation de 17,6 s en infusion et de
23,5 s au repos viennent surtout de la chaudière, pas de la sonde. Si la NTC
répond en moins d'environ 3 s, la PT1000 plus courte n'apporte que quelques
pourcents de retard en moins, pour le risque qu'elle touche moins bien l'eau.
Si la NTC répond en 8 s ou plus, le remplacement se justifie.

Si la PT1000 est retenue, le firmware doit passer à `R_fixe = 4676 Ω` et à
la loi Callendar–Van Dusen, qui s'inverse directement pour T ≥ 0 °C. Il faut
aussi retirer le décalage NTC de −10 °C
(`kBoilerNtcTemperatureOffsetC`) et le remesurer sonde montée.

## Carte ADS1115 prévue

Une carte dédiée alimentée par le seul câble I2C du Waveshare : ADS1115, 100 nF de
découplage, **4,7 kΩ à 0,1 %** entre A1 et GND, `A0` relié à `I2C_VCC`, XH 4 pôles
pour l'I2C et XH 2 pôles pour la sonde. Le pont est alors alimenté par `I2C_VCC`
et non plus par le port Sensor AD ; le firmware et le calcul ratiométrique ne
changent pas. Le GND de la résistance et celui de l'ADS1115 sont communs par
construction. **Aucune pull-up I2C** : le connecteur du Waveshare en a déjà
(voir `docs/cablage.md`). Un XH 3 pôles (`A2`, 3V3, GND) peut être prévu pour le
XDB401 analogique.

## Validation sur la machine

Effectuer une mesure à froid après stabilisation complète de la machine
et de la chaudière à température ambiante.

Procédure souhaitée :

1.  laisser la machine éteinte plusieurs heures jusqu'à l'équilibre
    thermique ;
2.  mesurer aussi précisément que possible la température réelle près du
    raccord 1/8" de la NTC ;
3.  pour une mesure IR sur une pièce métallique, mesurer de préférence
    une petite surface noire mate/ruban noir ayant eu le temps de
    prendre la température du raccord ;
4.  mesurer la résistance de la NTC ;
5.  noter le couple exact `température / résistance`.

Comparer ensuite les codes ADS1115 et la résistance calculée à une mesure
directe connue, puis contrôler une montée en température et la coupure de
chauffe avec la nouvelle courbe. Un point stabilisé à chaud permettrait de
réviser `R0` et `B` si nécessaire.

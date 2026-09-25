# Calibration NTC chaudière via ADS1115

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

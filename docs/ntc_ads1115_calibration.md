# Calibration NTC chaudière via ADS1115

## Contexte

L'objectif est de mesurer la température de la chaudière d'une machine à
café avec la sonde NTC 1/8" déjà installée, en utilisant un ADS1115 I²C
relié à un ESP32-S3-WROOM.

L'alimentation principale est en 5 V. L'ESP32 du Waveshare est alimenté par
sa carte ; le breakout ADS1115 reçoit **3,3 V du connecteur I2C du Waveshare**.
Un LDO AMS1117 dédié fournit un autre rail 3,3 V pour le pont de mesure de
la NTC. Les masses sont communes.

La plage utile est principalement \~80--100 °C en mode café et jusqu'à
\~120--130 °C en mode vapeur.

## Schéma de montage

Le rail 3,3 V issu de l'AMS1117 est mesuré par A0 afin de rendre le
calcul de résistance ratiométrique. A1 mesure le point milieu du pont
NTC.

``` text
                         +---------------- ESP32-S3-WROOM
                         |
5 V ---------------------+
                         |
                         +---- AMS1117 ----+---- A0 ADS1115
                                          |
                                      LDO 3.3 V
                                          |
                                         NTC
                                          |
                                          +---- A1 ADS1115
                                          |
                                      R = 2.193 kΩ
                                          |
GND --------------------------------------+---- GND commun
```

Notes :

-   `A0` mesure la tension réelle de sortie du LDO.
-   `A1` mesure la tension au point milieu NTC / résistance fixe.
-   La résistance fixe nominale est 2,2 kΩ ; sa valeur mesurée est
    **2,193 kΩ**.
-   La sortie du LDO AMS1117 mesurée au multimètre est **3,316 V**.
-   Le LDO du pont et le VDD de l'ADS1115 sont deux rails distincts ; leurs
    tensions ont été vérifiées sur la machine. La valeur **3,316 V** est un
    relevé de diagnostic, pas une constante nécessaire au calcul : celui-ci
    utilise le rapport des mesures A0 et A1.
-   `LDO_V = 3.316 V` est conservé comme valeur de référence/diagnostic
    ; le calcul normal doit utiliser A0.
-   Toutes les masses doivent être communes.
-   Pour mesurer directement \~3,3 V sur A0, configurer l'ADS1115 avec
    une plage compatible, typiquement **±4,096 V**.

## Calcul de la résistance NTC

Avec :

-   `R_FIXED = 2193 Ω`
-   `V_LDO = tension mesurée sur A0`
-   `V_DIV = tension mesurée sur A1`

le pont est :

``` text
V_LDO -> NTC -> V_DIV -> R_FIXED -> GND
```

La résistance de la NTC est donc :

``` text
R_NTC = R_FIXED * (V_LDO / V_DIV - 1)
```

Si A0 et A1 sont lus avec exactement le même réglage PGA de l'ADS1115,
le rapport peut être calculé directement à partir des valeurs ADC :

``` text
R_NTC = 2193 * (ADC_A0 / ADC_A1 - 1)
```

Cela rend la mesure pratiquement indépendante de la valeur absolue du
3,3 V et de la précision absolue de la référence interne de l'ADS1115.

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

### Paramètres provisoires

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

Ces constantes devront être recalculées après la prochaine mesure
stabilisée.

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

## Détermination provisoire de la courbe

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

## Next step : mesure stabilisée

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

Une fois ce point obtenu, refaire la régression sur l'ensemble des
données fiables, déterminer les constantes définitives (`R0`, `B`, ou
coefficients Steinhart-Hart), puis remplacer les constantes provisoires de
`firmware/screen/main/core/calibration_machine.h` et valider la conversion
ADS1115 -\> résistance -\> température sur la machine.

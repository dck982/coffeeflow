# Calibration NTC chaudière via ADS1115

> Remplacement prévu : PT1000 iOVEO 012EF02202, G 1/8, immersion inox
> 9 × 5,5 mm, deux fils silicone. Le montage ADS1115 peut être réutilisé, mais
> la conversion Beta décrite dans ce document devra être remplacée par une loi
> Callendar–Van Dusen. Avec 20 à 25 cm de câble, la résistance des deux fils
> devrait être négligeable ; elle sera simplement vérifiée à la calibration.

Le montage conservera l'ADS1115 et la résistance fixe **4,7 kΩ** déjà en
place (voir ci-dessous). Autour de 100 °C, la sensibilité
calculée restera d'environ 1,6 mV/°C, soit près de 13 codes par degré avec le
PGA ±4,096 V. Le courant de mesure sera ramené d'environ 0,92 à 0,54 mA et la
dissipation dans la PT1000 d'environ 1,2 à 0,41 mW. Un MAX31865 n'est pas
nécessaire à cette résolution. Sur la longueur prévue, une compensation des
fils n'apporterait qu'un gain négligeable. Il reste intéressant uniquement si ses diagnostics
RTD dédiés justifient une carte et une liaison SPI supplémentaires.

**Changement de résistance fixe, 2 octobre 2026.** La 4,7 kΩ du banc PT1000
(tolérance non marquée, **4676 Ω** déduits du point de glace, voir
[Essai PT1000 sur banc](#essai-pt1000-sur-banc-29-septembre-2026)) a remplacé
la 2,193 kΩ dans le pont, la NTC restant branchée.
<code>kBoilerNtcFixedOhm</code> vaut 4676 depuis l'écran 0.3.33. Les codes A1
bruts des captures antérieures se convertissent avec **2193 Ω**, les suivants
avec **4676 Ω** ; la courbe Beta et l'offset de −10 °C n'ont pas changé.
Relevés à froid, chaudière éteinte : `a0 = 26305`, `a1 = 2439`, soit
45,76 kΩ et 25,6 °C, relevé sous 0.3.32 et recalculé à la main, puis
`a0 = 26303`, `a1 = 2382` après flash de 0.3.33, soit 46,96 kΩ et 25,0 °C
lus par le firmware.

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
                        R = 4.676 kΩ (2.193 kΩ avant le 2026-10-02)
                            |
Waveshare Sensor AD GND ----+---- GND proche de l'ADS1115

Waveshare I2C 3V3 / GND ---------- alimentation ADS1115
```

Notes :

-   `A0` mesure la tension réelle du 3V3 qui alimente le pont.
-   `A1` mesure la tension au point milieu NTC / résistance fixe.
-   La résistance fixe nominale est 4,7 kΩ ; sa valeur étalonnée est
    **4676 Ω**. Jusqu'au 2 octobre 2026, c'était une 2,2 kΩ mesurée à
    **2,193 kΩ**.
-   Le GPIO/AD du port Sensor n'est pas utilisé.
-   Le bas de la résistance fixe et le GND de l'ADS1115 doivent avoir une
    référence de potentiel proche. Un simple raccordement électrique entre
    deux GND éloignés ne garantit pas l'absence de chute de tension sous charge.
-   Pour mesurer directement \~3,3 V sur A0, configurer l'ADS1115 avec
    une plage compatible, typiquement **±4,096 V**.

## Calcul de la résistance NTC

Avec :

-   `R_FIXED = 4676 Ω` (2193 Ω avant le 2 octobre 2026)
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
R_NTC = 4676 * (ADC_A0 / ADC_A1 - 1)
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
le LDO. Après modification, avec la 2,193 kΩ d'alors, un relevé `/telemetry` à froid donne
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

### Paramètres compilés depuis 0.3.34

``` text
R0 = 47000 Ω
B  = 3930 K
T0 = 298.15 K
offset utilisateur = −10,5 °C
```

`B = 3930 K` arrondit les ajustements de l'[essai NTC sur banc](#essai-ntc-sur-banc-2-octobre-2026)
(3 916 à 3 929 K sur trois bains, 3 923 K à l'ébullition seule). À résistance
égale, la lecture sonde monte de 0,40 °C vers 90 °C, 0,51 °C vers 104 °C et
0,68 °C vers 125 °C par rapport à 3 950 K. L'offset passe de −10 à −10,5 °C
pour compenser ce décalage dans la zone d'infusion : 94 °C utilisateur
désignent la même température physique qu'en 0.3.33.

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

La résistance fixe de **2,193 kΩ** d'origine était choisie proche de la
résistance de la NTC autour de 100 °C selon cette courbe. Avec la courbe
actuelle (47 kΩ / 3950 K), la NTC vaut environ 3,3 kΩ à 100 °C et 2,85 kΩ à
105 °C : la 4676 Ω en place donne environ 180 codes/°C dans cette zone.

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
échantillons répétés grâce à `age_ms`. `analyze_probe.py` le fait et résume
chaque capture : moyenne, écart-type, dérive et tranches pour repérer les
plateaux, avec une fenêtre et une référence optionnelles.

```sh
uv run firmware/tools/analyze_probe.py captures/probe-….json --start 55 --end 130 --reference-c 0
```

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
une seule fois. La pointe de la NTC mesure 24 mm × 3 mm de diamètre, contre 9 × 5,5 mm
pour la partie immergée de la PT1000. La NTC a été démontée le 2 octobre 2026
et passée sur le même banc, avec la même 4676 Ω que la PT1000 (voir
[Essai NTC sur banc](#essai-ntc-sur-banc-2-octobre-2026)). Le firmware n'a pas besoin de
changer, puisque les codes bruts suffisent. Repères attendus avec
`A0 ≈ 26 304` et la courbe 47 kΩ / 3950 K :

| Bain | R_NTC | A1 | Sensibilité |
|---|---:|---:|---:|
| glace | ≈ 158 kΩ | ≈ 756 codes | 39 codes/°C |
| ≈ 22 °C | ≈ 54 kΩ | ≈ 2 100 codes | 88 codes/°C |
| 100 °C | ≈ 3,3 kΩ | ≈ 15 460 codes | 181 codes/°C |

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

Si la PT1000 est retenue, `R_fixe = 4676 Ω` est déjà en place ; le firmware
doit passer à la loi Callendar–Van Dusen, qui s'inverse directement pour T ≥ 0 °C. Il faut
aussi retirer le décalage NTC de −10 °C
(`kBoilerNtcTemperatureOffsetC`) et le remesurer sonde montée.

### Essai NTC sur banc (2 octobre 2026)

NTC démontée de la chaudière, sur le pont de l'écran 0.3.33 : 4676 Ω entre A1
et GND, PGA ±4,096 V, courbe compilée 47 kΩ / 3950 K. Pointe seule immergée,
sonde séchée entre les plongées. Températures lues sur les codes bruts par
`analyze_probe.py`, sur le rapport moyen de la fenêtre.

| Capture `captures/probe-20261002-…` | Condition | Référence | Fenêtre | Lecture NTC | R / courbe |
|---|---|---|---|---|---|
| `162026-852765` | air, juste après manipulation | Netatmo 24,6 °C à 1 m | 0–60 s | 26,78 °C, dérive −0,34 °C/min | −9,2 % |
| `162450-810375` | air, 4 min plus tard | Netatmo 24,6 °C | 0–60 s | 25,41 °C, dérive −0,43 °C/min | −3,5 % |
| `162813-033751` | air, avant immersion | Netatmo 24,6 °C | 15–24 s | 24,61 °C | +2,7 % |
| `162813-033751` | eau, 1ʳᵉ plongée | thermomètre 25,2 °C | 55–70 s | 25,34 °C | −0,6 % |
| `162813-033751` | eau, 2ᵉ plongée | thermomètre 25,2 °C | 95–118 s | 25,32 °C | −0,55 % |
| `163646-733273` | glace, 1ʳᵉ immersion | 0 °C (thermomètre −0,2 °C) | 55–130 s | 0,26 °C, A1 ≈ 766 | −1,4 % |
| `163646-733273` | glace, 2ᵉ immersion | 0 °C | 175–250 s | 0,30 °C, A1 ≈ 768 | −1,6 % |
| `164815-029749` | ébullition, 1ʳᵉ plongée, 973,7 hPa | 98,89 °C | 186–197 s | 98,30 °C, dérive +1,6 °C/min | +1,7 % |
| `164815-029749` | ébullition, 2ᵉ plongée | 98,89 °C | 237–249 s | 98,24 °C, dérive +0,9 °C/min | +1,9 % |
| `164815-029749` | ébullition, 3ᵉ plongée | 98,89 °C | 282–291 s | 98,23 °C, dérive +2,2 °C/min | +1,9 % |
| `170021-938097` | eau vers 90 °C sur plaque de cuisson, indicatif | thermomètre 88 à 91 °C, 90,5 °C à la sortie | 98–105 s | 92,38 °C, dérive +0,07 °C/min | — |

**À l'air,** la sonde a mis plus de 8 minutes à rejoindre la Netatmo après
la manipulation. Les deux premières captures ne sont pas à l'équilibre et ne
servent pas à l'étalonnage. Le bruit à l'air est d'environ 0,02 °C sur 10 s,
cinq fois moins qu'avec la PT1000 : la NTC est bien plus sensible.

**À 25 °C,** la NTC lit 0,12 à 0,14 °C au-dessus du thermomètre, ce qui est
inférieur à l'incertitude de ce dernier. Ce bain valide la mesure ; il ne
sert pas à ajuster la courbe.

**Dans la glace,** le plateau oscille de 0,0 à 0,6 °C par tranches de 5 s,
avec un écart-type de 0,23 à 0,25 °C sur les moyennes de 1 s, contre 0,06 °C
dans le bain à 25 °C. Ces variations sont thermiques : bain inhomogène, ou
apport variable du corps resté à l'air selon l'agitation. Comme pour la
PT1000, on retient le plateau le plus bas, puisque la conduction ne peut que
faire lire trop haut : les tranches les plus basses lisent **−0,16 à 0,0 °C**,
soit la courbe nominale à 0 °C. La moyenne du plateau lirait +0,3 °C.

Le Beta qui relie 0 °C au bain à 25,2 °C vaut **3 968 K** avec le plateau le
plus bas et **3 920 K** avec le plateau moyen ; les 3 950 K compilés sont
entre les deux. **La courbe 47 kΩ / 3950 K est donc validée à environ 0,3 °C
entre 0 et 25 °C.**

**À l'ébullition,** le capuchon était protégé par un doigt de gant et les
plongées ont été limitées à 20–40 s. La fin de chaque plongée lit **0,6 à
0,66 °C sous** le point d'ébullition (98,89 °C à 973,7 hPa), mais monte encore
de 1 à 2 °C/min : c'est donc une borne haute de l'écart. Le thermomètre lisait
98,5 à 98,9 °C près de la sonde. Le bruit par échantillon atteint 0,25 à
0,29 °C, à cause des bulles. Le Beta qui fait lire 98,89 °C avec `R25 = 47 kΩ`
vaut **3 923 K**. Un ajustement Beta sur les trois bains donne
**47,07 kΩ / 3 929 K** avec le plateau de glace le plus bas, et
**46,77 kΩ / 3 916 K** avec le plateau moyen. Il retrouve l'ajustement de
septembre (`47,2 kΩ / 3 922 K`). Par rapport aux 47 kΩ / 3950 K compilés,
le firmware lit au plus **0,5 °C trop bas vers 90 °C, 0,6 à 0,7 °C vers
104 °C et 0,8 à 1,0 °C vers 125 °C**. L'offset utilisateur de −10 °C a été
réglé sur la courbe compilée : changer la courbe déplacerait la consigne
physique, sauf si l'offset est réajusté en même temps.

**Le bain vers 90 °C n'est qu'un repère de la zone café**, pas un point
d'étalonnage. L'eau chauffait sur une plaque de cuisson et le thermomètre
variait de 88 à 91 °C selon l'endroit et le moment. La NTC y lit 92,4 °C
(92,8 °C avec 3 930 K). Un écart de cet ordre est attendu dans un bain chauffé
par le fond, sans brassage contrôlé : ce point ne contraint pas le Beta. **Décision : 47 kΩ / 3 930 K avec un offset de −10,5 °C, écran
0.3.34.**

**Temps de réponse vers la glace.** Durées mesurées depuis la première baisse
détectable (−0,2 °C), donc sans le geste d'immersion, contrairement aux
valeurs de la PT1000 :

| Échelon | t63 | t90 | t95 |
|---|---|---|---|
| 23,8 °C → glace, corps à l'ambiante | 3,2 s | 7,3 s | 9,6 s |
| 9,6 °C → glace, après une sortie | 3,1 s | 6,6 s | 8,2 s |

La PT1000 donnait t63 ≈ 2 s et t90 ≈ 5 s sur l'échelon équivalent, suivis
d'une traîne d'environ 40 s sur les 3 derniers °C. La NTC n'a pas cette
traîne vers la glace : sa pointe de 24 mm éloigne l'élément sensible du
filetage.

**Temps de réponse vers l'ébullition,** mesurés de la même façon, avec comme
valeur finale la moyenne des 4 dernières secondes de chaque plongée :

| Échelon | t63 | t90 | t95 |
|---|---|---|---|
| 21 °C → ébullition | 6,7 s | 10,5 s | 13,1 s |
| 47 °C → ébullition | 3,6 s | 7,4 s | 9,5 s |
| 55 °C → ébullition | 3,2 s | 6,8 s | 8,4 s |

Le premier échelon est probablement allongé par une approche lente dans la
vapeur : la lecture monte déjà de 1,8 °C avant l'immersion. Les replongées
de la PT1000 depuis 66–75 °C donnaient t63 = 2,3–2,4 s et t90 = 4,8–5,6 s,
geste compris. La NTC est donc plus lente d'environ 1 s sur t63 et de 2 s sur
t90, légèrement au-dessus du seuil de 3 s et loin des 8 s qui justifieraient
le remplacement.

**Piège retrouvé :** sortie du bain à 25 °C, la sonde mouillée descend à
24,3–24,4 °C, sous la température de l'air, à cause de l'évaporation.

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

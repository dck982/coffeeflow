# Sonde SCACE

Mesure de la température de l'eau à la sortie du groupe, dans le panier, à la
manière d'un Scace. Elle remplacera la NTC chaudière comme critère de réglage
de la chauffe et permettra de recaler l'offset de −10 °C (voir
[chauffe-infusion.md](chauffe-infusion.md)).

## Matériel

| Élément | Choix |
| --- | --- |
| Sonde | EPCOS B57861S, NTC 10 kΩ à 25 °C, B25/100 = 3988 K, tolérance 0,3 %. Perle de 2,41 mm, fils de 0,5 mm. |
| Puck | `print/parts/sim_puck.py`, PETG, Ø 57 × 13,5 mm. La perle affleure au sommet, en sortie du canal incliné à 45°, scellée au silicone Loctite SI 5366. Les fils sortent sous le puck, entourés d'un joint torique. |
| Panier | Panier de simulation Decent Espresso, aveugle, trou central de 0,3 mm (un second de 0,2 mm existe). La restriction fait monter la pression. Deux trous de 0,6 mm laissent passer les fils de la sonde. |
| Boîtier | `print/parts/scace_box.py` : LDO 5 V → 3,3 V, ADS1115, résistance 1 kΩ, Wago 5 et 3 pôles. Les fils de la sonde y sont prolongés en 0,25 mm². |
| Lecteur | M5Stack Core2 sur batterie, port A (Grove) : jaune = SCL = GPIO32, blanc = SDA = GPIO33. Pull-ups de 10 kΩ sur le breakout ADS1115. |

## Câblage

``` text
Grove 5 V ------------ LDO VIN
Grove GND ---+-------- LDO GND
             |
LDO VOUT ----+-- WAGO5_VCC --+-- ADS1115 VDD
             |               +-- ADS1115 A0
             |               |
             |             R 1 kΩ
             |               |
             |          WAGO3_NTC --+-- ADS1115 A1
             |                      |
             |                     NTC (puck)
             |                      |
WAGO5_GND ---+----------------------+-- ADS1115 GND

Grove jaune -- ADS1115 SCL      Grove blanc -- ADS1115 SDA
```

A0 lit le rail, A1 le point milieu. La mesure est **ratiométrique** : le
rapport des deux codes ne dépend pas de la tension exacte du LDO.

```
R_ntc = R_fixe × A1 / (A0 − A1)
```

## Le pont diviseur

**Le montage est validé tel quel**, résistance côté 3,3 V et NTC côté GND.

Mettre la résistance en aval, côté GND, ne changerait **ni la précision, ni
le courant dans la sonde** : le même courant traverse les deux éléments en
série, et la mesure reste ratiométrique. La seule différence est le potentiel
des fils qui descendent dans le panier.

- **Montage actuel** : un fil de la sonde est au GND, l'autre est derrière la
  1 kΩ. Si un fil touche le panier, relié à la masse de la machine, le
  courant reste limité à 3,3 mA par la résistance. Le Core2 sur batterie est
  de toute façon flottant ; le cas ne se pose qu'avec le Core2 branché à un
  PC.
- **Montage inversé** : un fil de la sonde serait directement sur le 3,3 V,
  sans résistance pour limiter un contact avec le panier.

### Ordres de grandeur

Avec 3,3 V et le PGA ±4,096 V de l'ADS1115 (125 µV par code) :

| T | R NTC | A1 | Sensibilité | Courant | Puissance dans la NTC |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 0 °C | 34,0 kΩ | 3,206 V | 39 codes/K | 0,09 mA | 0,30 mW |
| 25 °C | 10,0 kΩ | 3,000 V | 98 codes/K | 0,30 mA | 0,90 mW |
| 60 °C | 2,45 kΩ | 2,344 V | 195 codes/K | 0,96 mA | 2,24 mW |
| 90 °C | 913 Ω | 1,575 V | 199 codes/K | 1,73 mA | 2,72 mW |
| 100 °C | 680 Ω | 1,336 V | 182 codes/K | 1,96 mA | 2,62 mW |

La 1 kΩ place la sensibilité maximale entre 85 et 90 °C, là où se fait la
mesure. A0 se lit vers 26 400 codes, sous la pleine échelle.

### Sources d'erreur négligeables

- **Fils** : environ 0,1 Ω en tout, soit 0,01 % de 913 Ω (moins de 0,01 K).
- **Impédance d'entrée de l'ADS1115** : plusieurs MΩ face à une source de
  moins de 1 kΩ, soit moins de 0,02 %.
- **Fuite entre les fils par l'eau** : en parallèle sur la NTC, elle fait lire
  trop chaud. 1 MΩ de fuite donne +0,03 K à 90 °C, 100 kΩ donnent +0,3 K.
  C'est l'étanchéité du silicone qui la tient basse.

### Auto-échauffement

La NTC dissipe environ 2,7 mW entre 60 et 100 °C, neuf fois plus qu'à 0 °C.
L'élévation dépend de l'évacuation de la chaleur par l'eau autour de la perle
et du silicone. Elle peut atteindre quelques dixièmes de degré. L'eau glacée,
à 0,3 mW, ne la révèle pas ; l'eau bouillante, à 2,6 mW et avec la sonde
montée dans le puck, la contient presque entièrement. C'est pourquoi la
calibration utilise les deux points.

**Le 5 V du port A est coupé par défaut.** Le firmware démarre pont hors
tension et l'alimente sur demande, pour que la sonde ne chauffe pas hors des
mesures.

## Conversion

Loi Beta, référencée à 25 °C :

```
1/T = 1/298,15 + ln(R_ntc / 10 000) / B
```

Constantes dans `firmware/scace/main/main.cpp` : `kFixedOhm = 1000`,
`kNtcR25Ohm = 10000`, `kNtcBetaK = 3988`. Les deux premières sont nominales
jusqu'à la calibration.

Une erreur de 1 % sur la 1 kΩ décale la lecture de 0,33 K à 90 °C. Une erreur
de 1 % sur B la décale d'environ 1,2 K à 90 °C si le seul point recalé est
0 °C. Un seul point ne suffit donc pas.

## Calibration

Deux points, sonde montée dans le puck comme en service :

1. **Point de glace, 0 °C.** Glace pilée et eau, bien mélangées, puck immergé
   au moins 2 minutes après stabilisation. Relever A0 et A1 sur la console.
   Ce point fixe le rapport R_ntc / R_fixe : on recale `kFixedOhm` en gardant
   `kNtcR25Ohm` nominal, car seul leur rapport compte.
2. **Point d'ébullition.** Eau à gros bouillons, puck immergé sans toucher le
   fond de la casserole. La température d'ébullition se calcule à partir de
   la pression atmosphérique brute, comme pour l'essai du 27 septembre
   (967,7 hPa donnaient 98,72 °C). Ce point fixe `kNtcBetaK`.

Avec ces deux points, la courbe passe par 0 °C et par environ 99 °C. Elle
intègre l'auto-échauffement en eau agitée et le décalage de la 1 kΩ. Entre
les deux, l'écart résiduel de la loi Beta face à la table du fabricant est le
seul terme non corrigé. S'il dépasse 0,1 K lors d'un contrôle à une
température intermédiaire, on passera à Steinhart–Hart à trois coefficients.

## Firmware `firmware/scace`

ESP-IDF 6.1, comme les deux autres cartes, avec **M5Unified** et **M5GFX**
tirés par le component manager. M5Unified gère la puce d'alimentation du
Core2 (AXP192 sur v1.0, AXP2101 sur v1.1), le port A et les touches. M5GFX
dessine sans LVGL. La cible est `esp32`, pas `esp32s3`.

### Comportement, version 1

- **Démarrage pont hors tension.** L'écran affiche « Sonde hors tension ».
- **Touche A** : bascule le 5 V du port A. À l'allumage, le firmware attend
  50 ms, ouvre le bus I²C du port A et cherche l'ADS1115 de 0x48 à 0x4B. À
  l'extinction, il libère le bus avant de couper le 5 V : un maître qui tient
  SDA et SCL hauts alimenterait l'ADS1115 éteint par ses diodes de
  protection.
- **Touche C maintenue 1 s** : coupe le 5 V puis éteint le Core2. Un simple
  effleurement ne suffit pas, pour ne pas interrompre une mesure. Branché en
  USB, le Core2 peut rester allumé.
- **Batterie trop faible** : le Core2 v1.1 refuse le 5 V sur batterie
  presque vide (8 % ou moins) sans USB. L'écran l'indique.
- **Mesure à 10 Hz** : A0 puis A1 en single-shot à 128 SPS, soit environ
  16 ms de conversion par paire. Une seule boucle de 10 ms, sans tâche de
  mesure : l'ADS1115 convertit seul pendant que l'écran se redessine.
- **Écran** : température en grand, avec deux décimales ; min et max sur une
  fenêtre glissante de 5 s ; codes et tensions A0 et A1 ; R_ntc. NTC ouverte
  (A1 ≈ A0) et court-circuit (A1 ≈ 0) sont affichés comme tels.
- **Console USB** : une ligne CSV par mesure, `ms,a0,a1,ohm,celsius`, à
  115 200 bauds, pour enregistrer les essais de calibration au PC.
- **Erreur I²C** : la mesure s'arrête, le firmware recherche l'ADS1115 toutes
  les 500 ms et compte les erreurs à l'écran.

Il n'y a pas d'OTA : l'image tient dans une partition unique, flashée par
USB.

### Construire et flasher

```sh
source ~/.espressif/tools/activate_idf_v6.1.sh
cd firmware/scace
idf.py build
idf.py -p /dev/cu.wchusbserialXXXX flash monitor
```

### Version 2 : serveur BLE

Le Core2 devient périphérique GATT. L'écran s'y connecte comme il le fait déjà
avec la balance, et reçoit la température du panier à 10 Hz pour l'aligner
sur les captures. Choix par défaut :

- un service, une caractéristique en notification, une trame par mesure ;
- trame de 8 octets, petit-boutiste : `uint16` numéro de séquence,
  `int16` température en centièmes de degré, `uint8` état (ok, ouverte,
  court-circuit, pont hors tension), `uint8` réservé, `uint16` temps en ms
  modulo 65 536 ;
- la définition du service (UUID, trame) vit dans `firmware/common`,
  compilée par l'écran et par la sonde ;
- les UUID seront tirés au moment de l'implémentation.

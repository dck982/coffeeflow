# Sonde SCACE

Mesure de la température de l'eau à la sortie du groupe, dans le panier, à la
manière d'un Scace. Elle remplacera la NTC chaudière comme critère de réglage
de la chauffe et permettra de recaler l'offset de −10 °C (voir
[chauffe-infusion.md](chauffe-infusion.md)).

## Matériel

| Élément | Choix |
| --- | --- |
| Sonde | EPCOS B57861S0103F, NTC 10 kΩ ±1 % à 25 °C, courbe R/T 8016, B25/100 = 3988 K ±0,3 %. Perle de 2,41 mm, fils de 0,5 mm. |
| Puck | `print/parts/sim_puck.py`, PETG, Ø 57 × 13,5 mm. La perle affleure au sommet, en sortie du canal incliné à 45°, scellée au silicone Loctite SI 5366. Les fils sortent sous le puck, entourés d'un joint torique. |
| Panier | Panier de simulation Decent Espresso, aveugle, trou central de 0,3 mm (un second de 0,2 mm existe). La restriction fait monter la pression. Deux trous de 0,6 mm laissent passer les fils de la sonde ; rendus étanches au Loctite par-dessous le 9 octobre 2026, après la fuite de la [première purge](#première-purge-9-octobre-2026). |
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
| 0 °C | 32,65 kΩ | 3,202 V | 39 codes/K | 0,10 mA | 0,31 mW |
| 25 °C | 10,0 kΩ | 3,000 V | 96 codes/K | 0,30 mA | 0,90 mW |
| 60 °C | 2,49 kΩ | 2,354 V | 194 codes/K | 0,95 mA | 2,23 mW |
| 90 °C | 918 Ω | 1,579 V | 204 codes/K | 1,72 mA | 2,72 mW |
| 100 °C | 680 Ω | 1,336 V | 184 codes/K | 1,96 mA | 2,62 mW |

R selon la table R/T 8016 du fabricant (voir [Conversion](#conversion)).

La 1 kΩ place la sensibilité maximale entre 85 et 90 °C, là où se fait la
mesure. A0 se lit vers 26 400 codes, sous la pleine échelle.

### Sources d'erreur négligeables

- **Fils** : environ 0,1 Ω en tout, soit 0,01 % de 918 Ω (moins de 0,01 K).
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

**Sur USB, le port A reste alimenté.** Sur le Core2, la broche qui coupe le
boost 5 V ferme en même temps l'interrupteur qui relie le VBUS de l'USB au
bus 5 V : câble branché, le pont reste sous tension malgré la touche A, et
la LED du LDO reste allumée. Seule la batterie permet de couper vraiment
(vérifié le 7 octobre 2026). C'est accepté tel quel : les mesures se font
sur batterie.

## Conversion

**Steinhart–Hart ajustée sur la table R/T 8016** de la fiche TDK S861, de
−10 à 110 °C, R en ohms :

```
1/T = A + B·ln R + C·(ln R)³
A = 1,12476949e-3   B = 2,34824463e-4   C = 8,50854659e-8
```

L'écart à la table reste sous 0,02 K, l'ordre de son arrondi : un
quatrième coefficient ou une autre plage n'y changent rien.

**La loi Beta ne convient pas.** B25/100 = 3 988 K ne la rend exacte qu'à 25
et 100 °C. À 0 °C, elle donne 34 013 Ω au lieu des 32 650 Ω de la table :

| T réelle | Lecture par la loi Beta | Écart |
| ---: | ---: | ---: |
| 0 °C | 0,77 °C | +0,77 K |
| 60 °C | 59,61 °C | −0,39 K |
| 90 °C | 89,81 °C | −0,19 K |

Le premier bain de glace l'a montré : la loi Beta lisait +0,75 °C, la table
−0,01 °C.

**Deux corrections, une par point de calibration**, sur le modèle de
tolérance de TDK :

```
R_ntc = kFixedOhm · A1 / (A0 − A1)
R_ntc = R_table(T) · exp(kBetaShiftK · (1/T − 1/273,15))
```

- `kFixedOhm` absorbe la résistance fixe réelle **et** la tolérance de
  ±1 % sur R25 : seul leur rapport compte. ±1 % donne ±0,2 K à 0 °C et
  ±0,33 K à 90 °C.
- `kBetaShiftK` est l'écart de B à 3 988 K. La tolérance de ±0,3 %, soit
  ±12 K, laisse encore ±0,35 K à 90 °C une fois la glace recalée. Il est
  référencé à 0 °C : il ne déplace pas le point de glace, et les deux points
  se calibrent indépendamment.
- Le firmware retire le décalage de B par trois itérations : 1/T dépend de
  lui-même, mais la pente vaut environ 0,003.

Constantes dans `firmware/scace/main/main.cpp` : `kShA`, `kShB`, `kShC`
(fixes), `kFixedOhm` (999,5, calibré au point de glace) et `kBetaShiftK`
(0, contrôlé au point d'ébullition).

La résistance fixe a été mesurée au multimètre à **989 Ω** le 7 octobre 2026.
Un multimètre courant ne fait pas mieux que ±1 % sur ce calibre, et
`kFixedOhm` inclut aussi la tolérance de R25 : la valeur sert de contrôle
grossier, pas de calibration.

## Calibration

Deux points, sonde montée dans le puck comme en service, Core2 **sur
batterie**. Chaque point s'enregistre avec `firmware/tools/scace_ble_log.py`
(voir [Serveur BLE](#serveur-ble)), puis se calcule avec
`firmware/tools/scace_calibration.py` sur la fenêtre du palier. L'outil
reprend la conversion du firmware ; un test vérifie que les coefficients sont
les mêmes. Il signale un palier qui dérive de plus de 0,01 K/min.

1. **Point de glace, 0 °C.** Surtout de la glace pilée, bien tassée, avec
   juste assez d'eau pour combler les vides, dans un récipient isolé. L'eau
   la plus dense est à 4 °C : avec trop d'eau, elle coule au fond et le bain
   n'est plus homogène (premier essai : 0,35 K d'écart entre deux zones).
   La glace sortie du congélateur est sous 0 °C : attendre qu'elle soit
   humide. Sonde au centre, immobile, 2 minutes de palier à moins de
   0,01 K/min. À 0 °C, un code vaut 0,026 K (39 codes/K) : la moyenne sur
   2 minutes, soit 1 200 mesures, rend le bruit de l'ADS1115 négligeable.

   ```sh
   uv run firmware/tools/scace_calibration.py ice --capture captures/scace-….csv --start … --end …
   ```

   Donne `kFixedOhm = R_table(0 °C) / (R/R_fixe moyen)`, avec
   R_table(0 °C) = 32 652 Ω selon le modèle.
2. **Point d'ébullition.** Eau à gros bouillons, puck immergé sans toucher le
   fond de la casserole, où l'eau est surchauffée. Sonde immobile au moins
   60 s : le puck finit de chauffer avec une constante de temps d'environ
   8 s, et chaque déplacement fait plonger la lecture. La température d'ébullition se calcule à partir de
   la pression atmosphérique brute, comme pour l'essai du 27 septembre
   (967,7 hPa donnaient 98,72 °C).

   ```sh
   uv run firmware/tools/scace_calibration.py boil --capture … --start … --end … \
       --celsius 98.72 --fixed-ohm …
   ```

   Donne `kBetaShiftK`. Une valeur au-delà de ±12 K sortirait de la
   tolérance de la NTC : elle signalerait un auto-échauffement plus fort que
   prévu, une fuite par l'eau ou une erreur de température d'ébullition.

Ce point intègre l'auto-échauffement en eau agitée. L'écart résiduel du
modèle face à la table, sous 0,02 K, est le seul terme non corrigé.

`scace_calibration.py convert --ratio … --fixed-ohm … --beta-shift …` donne
la température que lirait le firmware, pour contrôler un point
intermédiaire.

### Résultat du point de glace, 7 octobre 2026

**`kFixedOhm = 999,5 Ω`**, retenu dans le firmware 0.3.1.

| Bain | Capture | Fenêtre | R/R_fixe | `kFixedOhm` |
| --- | --- | --- | ---: | ---: |
| 1, trop d'eau, non homogène | `scace-20261007-171548.csv` | 201,7–210,9 s, zone la plus froide | 32,674 | 999,3 |
| 2, glace tassée | `scace-20261007-173013.csv` | 75–110 s, dérive −0,006 K/min | 32,6675 ± 0,0026 | 999,5 |
| 2 | idem | 80–112 s, dérive +0,005 K/min | 32,6694 ± 0,0026 | 999,5 |

- Le palier du second bain ne dure que 35 s, au lieu des 2 minutes de la
  procédure. Les deux bains et toutes les fenêtres valides concordent à
  0,2 Ω près, soit 0,004 K : on ne refait pas le bain.
- Le changement est presque nul : 1 000 → 999,5 Ω déplace la lecture
  d'environ 0,01 K à 0 °C et 0,02 K à 90 °C. Avec la table et les constantes
  nominales, la sonde lisait déjà −0,01 °C.
- Le multimètre donnait 989 Ω, 1 % plus bas. C'est dans les tolérances
  cumulées de R25 (±1 %) et du multimètre (±1 %) ; le point de glace fait
  foi.
- σ = 0,028 K, soit 1 code : à 0 °C, la sensibilité n'est que de
  39 codes/K.

### Contrôle au point d'ébullition, 7 octobre 2026

**`kBetaShiftK` reste à 0.** Pression absolue de 956,5 hPa (station
Netatmo intérieure), soit une ébullition à 98,39 °C. Capture
`scace-20261007-175059.csv`, sonde dans l'eau bouillante environ 90 s.

- Aucun palier : après deux déplacements de la sonde, la lecture remonte
  lentement de 98,0 à 98,3 °C, puis finit à 98,45–98,52 °C sur les 12
  dernières secondes, avec encore +0,35 K/min de dérive.
- Selon la fenêtre, `kBetaShiftK` vaudrait de −0,4 à +2,1 K, loin de la
  tolérance de ±12 K. Une valeur de +2 K ne déplacerait la lecture que
  d'environ 0,06 K, moins que l'incertitude de l'essai.
- La sonde lit donc juste à environ 0,1 à 0,2 K près vers 98 °C, mieux que
  les ±0,35 K que la seule tolérance de B laissait à 90 °C.

### Bilan

Les constantes retenues sont **celles de la fiche** : la table R/T 8016 et B
nominal. `kFixedOhm` = 999,5 Ω reste à 0,05 % du nominal. Il absorbe la R
fixe réelle et la tolérance de R25, qui se compensent presque ; le
multimètre (989 Ω) suggère une R25 environ 1 % sous le nominal, sans
pouvoir le séparer de sa propre erreur. Ce qui a vraiment compté, c'est le
modèle : la loi Beta lisait +0,77 K à 0 °C.

| Domaine | Incertitude absolue |
| --- | --- |
| 0 °C | ~0,01 K (point de glace) |
| vers 98 °C | ~0,1 à 0,2 K (contrôle à l'ébullition, sans palier) |
| entre les deux | même ordre, écart du modèle à la table sous 0,02 K |

Pour fixer B un jour : refaire l'ébullition avec la sonde immobile, loin du
fond, au moins 60 s, et lancer `scace_calibration.py boil`.

### Thermomètre de cuisine

TFA Dostmann 30.1061, donné à ±1 °C de −20 à 150 °C. Il lit −0,2 K à 0 °C
(zone la plus froide du premier bain) et +0,05 K à l'ébullition (98,4–98,5
pour 98,39 °C). Il est donc bien meilleur que sa fiche en ces deux points,
mais son erreur entre les deux reste inconnue : il sert de contrôle, pas de
référence pour B.

## Calibration de la consigne chaudière

But : mesurer l'eau au panier pour recaler l'offset de −10,5 °C entre la NTC
chaudière et la température utilisateur (voir
[chauffe-infusion.md](chauffe-infusion.md#objectif)). Le choix entre un
offset unique, un offset et une pente sur plusieurs consignes, ou une
séparation entre perte statique et dynamique de la loi d'infusion attend
les données.

Les purges se lancent **depuis l'écran, en mode machine** : en Wi-Fi, le BLE
de l'écran est éteint et la capture HF n'a ni la sonde ni le poids.
`scace_ble_log.py` tourne en parallèle sur le Mac pour couvrir l'avant et
l'après-purge avec les codes bruts. La capture HF est ensuite téléchargée
en Wi-Fi.

### Retard de la NTC à la mise en chauffe, 9 octobre 2026

Capture `monitor-heating-20261009-104655-351980.json` : chauffe activée à
100 % depuis 31,14 °C sonde (machine froide), bruit de 0,009 K.

| Repère | Après la mise en marche du SSR |
| --- | ---: |
| +0,05 K | 11,6 s |
| +0,1 K | 12,1 s |
| +1 K | 14,2 s |
| Retard pur ajusté, plus 3 premiers ordres de 1,67 s | 9,9 s |
| Délai moyen (asymptote de la rampe) | 14,9 s |

- Le retard pur de 10 s rejoint celui des montées du 28 septembre depuis 53
  et 80 °C (voir
  [chauffe-chaudiere.md](chauffe-chaudiere.md#retard-de-la-ntc-28-septembre)).
- La rampe atteint 1,37 K/s à la sonde, près du double des 0,72 K/s d'une
  chaudière chauffée uniformément : à froid, l'eau chaude monte vers la NTC.
  Le délai moyen est donc plus court que les 17,6 s du modèle ajusté à
  chaud, qui reste la référence près de la consigne.
- Conséquence pour la calibration : une NTC stable ne prouve pas
  l'équilibre ; NTC et SCACE se comparent en moyenne ou en palier, pas
  instant par instant.

### Première purge, 9 octobre 2026

Purge de 10 s à pompe 100 %, consigne de 90 °C, panier 0,3 mm. Captures
`261009-111018.json` (HF) et `scace-20261009-110844.csv` (Mac) ; les deux
concordent à la seconde, aucune trame perdue. Puck, panier et porte-filtre
à l'ambiante. **Balance non connectée** : `weight_g` reste à 0, bit de
validité absent.

**L'eau coulait le long des fils** sous le panier. Si les fils sont nus
(0,5 mm dans 0,6 mm), chaque trou laisse un anneau de 0,086 mm², soit
0,17 mm² pour les deux contre 0,071 mm² pour le trou de mesure : la fuite
dominerait le débit. Hypothèse, la section des fils n'a pas été vérifiée.

| Instant | SCACE | NTC utilisateur | Pression | Volume amont |
| --- | ---: | ---: | ---: | ---: |
| Porte-filtre verrouillé (−11 s) | 23,3 °C | — | — | — |
| Début de la purge | 40,7 °C | 89,7 °C | 0,3 bar | 0 ml |
| 3 s | 64,1 °C | 89,7 °C | 0,3 bar | 11,6 ml |
| Arrêt de la pompe, 10 s | 73,4 °C | 83,8 °C | 5,8 bar | 37,1 ml |
| Maximum SCACE, 13 s | **75,3 °C** | 80,6 °C | 6,2 bar | 38,3 ml |
| 64 s | 69,2 °C | — | — | — |

- Débit amont de 4,0 ml/s pendant 5 s, puis 3,25 ml/s à l'arrêt ; la
  pression n'atteint que 5,8 bar en 10 s (fuite et air au-dessus du puck).
- Après l'arrêt, la pression monte encore de 5,5 à 6,9 bar en 5 s, pompe à
  0 % et SSR à 35 %. Probablement la chaudière isolée par la vanne trois
  voies, qui se dilate en chauffant ; non vérifié.
- La NTC commence à baisser 5 s après le début de la purge (89,7 → 79,4 °C à
  15 s), dans l'ordre des 6,4 s du chemin eau admise → NTC.
- **Première piste d'offset : environ 15 K.** Au maximum de la SCACE
  (75,3 °C à 13 s), la NTC lit 80,6 °C utilisateur, soit 91,1 °C sonde :
  15,8 K d'écart, contre 10,5 K pour l'offset actuel. C'est l'ordre de
  l'offset d'usine du Gicar, d'environ 15 °C (−14 °C retenus en 0.3.16 pour
  un affichage proche de Gicar).
- L'écart dépend de l'instant comparé :

  | Instant comparé | NTC sonde | SCACE | Écart |
  | --- | ---: | ---: | ---: |
  | Début de la purge | 100,2 °C | 75,3 °C (maximum) | 24,9 K |
  | Arrêt de la pompe | 94,3 °C | 73,4 °C | 20,9 K |
  | Maximum SCACE, même instant | 91,1 °C | 75,3 °C | 15,8 K |

  Deux biais de sens opposés : le puck, le panier et le porte-filtre froids
  font lire la SCACE trop bas (elle montait encore de 0,5 K/s, sans
  palier), ce qui gonfle l'écart ; le retard de la NTC sur l'eau admise
  (≈ 6 s) fait comparer au même instant une NTC déjà refroidie, ce qui le
  réduit. L'essai donne l'ordre de grandeur, pas l'incertitude.

Suite, une fois l'étanchéité sèche (24 h) : purges de **15 s**, balance
connectée, puck et porte-filtre chauffés par une ou deux purges jetées.

## Firmware `firmware/scace`

ESP-IDF 6.1, comme les deux autres cartes, avec **M5Unified** et **M5GFX**
tirés par le component manager. M5Unified gère la puce d'alimentation du
Core2 (AXP192 sur v1.0, AXP2101 sur v1.1), le port A et les touches. M5GFX
dessine sans LVGL. **NimBLE** porte le serveur GATT. La cible est `esp32`,
pas `esp32s3`.

### Comportement

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
  16 ms de conversion par paire. Une tâche dédiée, prioritaire, sur le
  cœur 1, tourne toutes les 5 ms. Elle est seule à toucher le bus du port A
  et envoie chaque mesure sur la console et en BLE. La boucle principale
  garde les touches, la puce d'alimentation (bus interne) et l'écran. Pour
  couper le 5 V, elle demande d'abord à la tâche de libérer le bus et attend
  sa réponse, 200 ms au plus.
- **Pourquoi une tâche** : un dessin complet de l'écran prend 73 ms (63 ms
  sans la température), surtout pour envoyer l'image de 150 Ko en PSRAM.
  Dans une boucle unique redessinée à 10 Hz, chaque paire de conversions
  enjambait un dessin et la mesure tombait à 5 Hz (relevé du 7 octobre
  2026).
- **Version** : `kFirmwareVersion` dans `main.cpp`, affichée en haut de
  l'écran après « SCACE ». Elle est indépendante de celle des cartes de la
  machine et s'incrémente à chaque image flashée. 0.1.0 était la version
  sans BLE, 0.2.0 ajoute le serveur GATT, 0.2.1 la tâche de mesure, 0.2.2
  décale l'adresse de l'ADS, 0.3.0 remplace la loi Beta par la table, 0.3.1
  fixe `kFixedOhm` au point de glace.
- **Écran**, redessiné à 4 Hz, assez pour un humain : la mesure fine passe
  par le BLE. Température en grand, avec deux décimales ; min et max sur une
  fenêtre glissante de 5 s ; codes et tensions A0 et A1 ; R_ntc. NTC ouverte
  (A1 ≈ A0) et court-circuit (A1 ≈ 0) sont affichés comme tels. Le
  pied de page indique `BLE n` sous la touche B quand n centraux sont
  connectés.
- **BLE** : chaque mesure part en notification ; voir
  [Serveur BLE](#serveur-ble).
- **Console USB** : une ligne CSV par mesure, `ms,a0,a1,ohm,celsius`, à
  115 200 bauds. Le BLE la remplace pour les enregistrements : ouvrir le port
  série avec DTR ou RTS actif réinitialise le Core2, qui redémarre pont hors
  tension.
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

`sdkconfig` est généré et n'est pas suivi par Git. Après une modification de
`sdkconfig.defaults`, le supprimer avant de reconstruire : une option déjà
présente dans `sdkconfig`, même désactivée, l'emporte sur les défauts.

## Serveur BLE

Le Core2 est périphérique GATT. Il remplace la console USB pour la
calibration et alimente les captures de l'écran, qui s'y connecte comme à la
balance et horodate chaque trame à sa réception : aucune synchronisation
d'horloge n'est nécessaire.

La définition du service vit dans
`firmware/common/include/common/scace_ble.hpp`, compilée par la sonde et par
l'écran. Le script Mac en reprend les UUID ; un test vérifie
qu'ils n'ont pas divergé.

| Élément | Valeur |
| --- | --- |
| Nom annoncé | `SCACE`, dans la réponse au scan |
| Service | `5017120a-7fa7-46d8-a22e-61b24b1201fe`, dans le paquet d'annonce |
| Caractéristique | `84fef574-1171-4c9a-b221-6f17aea79a38`, lecture et notification |
| Connexions | 2 à la fois : l'écran et le Mac. L'annonce reprend tant qu'il reste une place. |
| Sécurité | aucune, ni appairage ni chiffrement |

### Trame

12 octets, petit-boutiste, une par mesure à 10 Hz. Elle tient dans le MTU
par défaut (20 octets de valeur), sans négociation.

| Octet | Type | Champ |
| ---: | --- | --- |
| 0 | `uint16` | numéro de séquence, incrémenté à chaque trame |
| 2 | `uint16` | temps sonde en ms modulo 65 536, à la fin de la conversion A1 |
| 4 | `int16` | code A0 (rail) |
| 6 | `int16` | code A1 (point milieu) |
| 8 | `int16` | température en centièmes de °C, constantes de la sonde ; −32 768 si absente |
| 10 | `uint8` | état : 0 sans mesure, 1 ok, 2 NTC ouverte, 3 court-circuit |
| 11 | `uint8` | réservé, 0 |

- **Les codes bruts accompagnent la température.** Une capture reste
  recalculable après chaque nouvelle calibration.
- **Le temps sonde** permet au central de retirer le jitter de l'intervalle
  de connexion : l'écart entre deux trames est celui des mesures, pas celui
  des réceptions.
- **Sans mesure** (pont hors tension, ADS1115 introuvable, erreur I²C), la
  sonde envoie une trame d'état 0 par seconde, codes à 0 : le central sait
  qu'elle est là.
- **Une trame perdue** faute de tampon n'est pas renvoyée ; elle se voit au
  numéro de séquence.
- Un décodeur ignore les octets au-delà de 12 et refuse un état inconnu.

### Enregistrement sur le Mac

```sh
uv run firmware/tools/scace_ble_log.py
uv run firmware/tools/scace_ble_log.py --duration 600 --window 120
```

Le script cherche le service, s'abonne et écrit
`captures/scace-AAAAMMJJ-HHMMSS.csv` jusqu'à Ctrl-C ou `--duration`. Il se
reconnecte seul si la liaison tombe. Colonnes : `unix_ms` (réception sur le
Mac), `seq`, `probe_ms`, `a0`, `a1`, `celsius` (vide sans température),
`status`. Toutes les 2 s, une ligne d'état donne, sur la fenêtre glissante
(`--window`, 60 s par défaut), la température moyenne, son écart-type, la
dérive en K/min et la moyenne de R/R_fixe, ainsi que le nombre de trames
perdues.

### Côté écran

Depuis l'écran 0.3.47, voir « Sonde SCACE » dans [firmware.md](firmware.md) :

- l'écran se connecte à la sonde si `scace.enabled` est vrai
  (`uv run firmware/tools/set_scace.py true`), se reconnecte après une perte,
  et la cherche 1 s toutes les 10 s tant que la balance est connectée ;
- la capture HF porte `scace_temperature_c` (température seule, au
  centième), `null` sans mesure correcte depuis 300 ms. Chaque échantillon
  prend la dernière trame reçue, sans dater par le temps sonde : le décalage
  reste sous 100 ms plus l'intervalle de connexion, contre plusieurs
  secondes de retard pour la NTC chaudière ;
- `analyze_hf_capture.py` et `describe_hf_capture.py` en donnent la moyenne
  pondérée par la tasse et la courbe ;
- `record_probe.py --scace` lit la sonde en BLE depuis le Mac, en parallèle
  de `/telemetry`.

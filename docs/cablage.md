# Câblage — détail fil par fil

Vue d'ensemble et choix des composants : `../README.md`. Ce fichier tient le détail :
quel fil, quelle couleur, quelle connectique, d'où à où. Le brochage de chaque connecteur
de la carte `combined` est dans `../pcb/combined/combined_design.md` (« Connectors »).

Tout le 230 V reste dans le compartiment technique. Le câble de l'écran (5 V et CAN) et le
câble de la sonde NTC rejoignent le boîtier de l'écran en façade.

```text
interrupteur principal ── L/N ── Wago 221-415 (boitier_ps_24v) ─┬─ LED de façade
                                                                ├─ alimentation 24 V (VH)
                                                                └─ combined J2 ─ J3 ─ vanne

alimentation 24 V ── bornier ── rouge/noir ── combined J7 ─┬─ embouts doubles ─ pompe (rouge/noir)
                                                           └─ buck 5 V ─┬─ J1 écran (5 V + CAN)
                                                                        ├─ J6 Digmesa
                                                                        ├─ J8 SSR chaudière
                                                                        └─ J10 XDB401
combined J9 ── fils de signal de la pompe (consigne, tachymètre)
```

La carte `combined` a deux SSR à distinguer : le triac de la carte commande la **vanne**
(J3), tandis que le **Keysolu/Maxwell KS53 D-24Z20N-LQ** (« mini-puck »), fourni avec la
machine, commande la **résistance de chaudière** ; `combined` ne lui envoie que sa
commande 5 V (J8). Il possède deux bornes d'entrée DC **+ / −** (plage **4–24 VDC
indiquée pour l'exemplaire**) et deux bornes de puissance **230 VAC**. La [fiche de la
série KS53](https://manage.keysolu.com/upload/product/file/20241204/KS53_EN.pdf) donne une
plage générique d'entrée de 4–32 VDC ; le 5 V du montage appartient aux deux plages.

## Câble

| Domaine | Référence | Section |
| --- | --- | --- |
| 230 V : arrivée, LED, alimentation, `combined`, vanne | **Helutherm 145** (`datasheets/helutherm145.pdf`) | **0,75 mm²** |
| 230 V — puissance chaudière | câblage de la machine | **1 mm²** |
| 24 V, alimentation → J7 | **Helutherm 145** rouge / noir | **0,75 mm²** |
| Commande du SSR chaudière (J8) | **Helutherm 145** | **0,75 mm²**, imposé par les cosses rouges 4,8 mm |
| Digmesa (J6) | **Helutherm 145** | **0,5 mm²**, imposé par les contacts VH3.96 |
| Écran, 5 V et CAN (J1) | **Helutherm 145** | **0,25 mm²** |
| XDB401 (J10) | câble intégré à la sonde | inconnue, probablement moins de 0,25 mm² |
| Pompe (J7, J9) | câble intégré à la pompe | rouge et noir Ø 2 mm, fils de signal Ø 1,5 mm sur l'isolant |

Le Helutherm 145 tient 145 °C en continu ; l'enceinte de la machine est estimée à
40–50 °C.

### Compatibilité électromagnétique

La vanne inductive est la principale source d'EMI. La règle est d'**annuler la surface
de boucle** : aller et retour strictement contigus.

- **230 V** : chaque paire phase-neutre est maintenue par de la **gaine thermo** sur toute
  sa longueur. Pas besoin de torsader.
- **CAN** : **paire torsadée** CANH / CANL.
- **Digmesa** : SIGNAL et GND torsadés.
- **XDB401** : la gaine porte une fine feuille de blindage, **sans continuité avec le GND**
  et sans fil de drain : elle reste flottante.

---

## 230 V

### Entrée — interrupteur principal → `boitier_ps_24v`

Deux **cosses FASTON 6,3 × 0,8 mm isolées** prises sur l'interrupteur principal de la
machine. Tant qu'il n'est pas enfoncé (clic mécanique), rien n'est sous tension : le mod
n'est jamais alimenté machine éteinte.

Les deux fils, **0,75 mm²**, entrent dans `boitier_ps_24v` et vont dans ses deux **Wago
221-415**, une pour L, une pour N. Phase brun, neutre bleu.

### Départs des Wago

Trois paires 0,75 mm² repartent des deux Wago :

| Vers | Arrivée |
| --- | --- |
| **LED de façade** | cosses FASTON d'origine. Ce départ est obligatoire : les cosses de la LED ont été réutilisées pour prendre l'alimentation |
| **Alimentation 24 V** | boîtier **JST VH 3 positions**, position 2 vide : L et N à 7,92 mm |
| **`combined` J2** | embouts 0,75 mm² dans un boîtier **HT3.96 2 pôles** : broche 1 L, broche 2 N |

Le fusible T2A de l'alimentation ne protège qu'elle-même. Le départ vers `combined` est
protégé par F1 (0,5 A rapide) sur la carte, juste après J2 ; le câblage en amont, par le
disjoncteur de la maison.

### Vanne — `combined` J3

Deux fils **0,75 mm² bleu / jaune**, embouts dans un boîtier **HT3.96 2 pôles** côté
`combined`, **FASTON 6,3 × 0,8 mm** côté vanne solénoïde trois voies OLAB 08252L50-A14-1A-G
(bobine 08000BH-J5IV, 15 VA). J3 : broche 1 N (bleu), broche 2 L commutée (jaune). Seule
la phase est commutée ; le neutre traverse la carte de J2 à J3.

### SSR de chaudière fourni avec la machine

Le SSR est **en série** dans le circuit de puissance de la chaudière, en **1 mm²**,
câblage d'origine :

```text
phase de l'interrupteur principal → borne AC du SSR chaudière
→ autre borne AC du SSR → résistance de chaudière
→ protection / relais thermique → neutre de l'interrupteur principal
```

Ses deux bornes DC **+ / −** reçoivent la commande de `combined` J8, décrite plus bas.

### Résumé des paires 230 V

1. **L + N** interrupteur principal → Wago de `boitier_ps_24v`
2. **L + N** Wago → LED de façade
3. **L + N** Wago → alimentation 24 V
4. **L + N** Wago → `combined` J2
5. **N + L commutée** `combined` J3 → vanne

Circuit de chauffe distinct : **phase interrupteur → SSR chaudière → résistance →
protection thermique → neutre interrupteur** (conducteurs de puissance en **1 mm²**).

---

## 24 V — alimentation, `combined` et pompe

Deux fils **0,75 mm² rouge (+24 V) / noir (GND)** partent du **bornier à vis** de
l'alimentation (+24 V côté LED, GND contre le flanc) vers **J7** de `combined` : broche 1
+24V, broche 2 GND.

Côté J7, chaque fil est serti avec le fil de même polarité de la pompe dans un **embout
double 2 × 0,75 mm²**, serré dans le boîtier **HT3.96 2 pôles** :

| J7 | Embout double |
| --- | --- |
| broche 1, +24V | rouge de l'alimentation + rouge de la pompe |
| broche 2, GND | noir de l'alimentation + noir de la pompe |

La pompe est donc alimentée par J7. Son courant (≈ 1,1 A au plus jusqu'à l'OPV) passe d'un
fil à l'autre dans l'embout et la vis du bornier, sans traverser les pistes de la carte.
Débrancher J7 coupe à la fois `combined`, l'écran et la pompe. **J7 ne se branche jamais
sous tension** : le buck n'a pas de condensateur électrolytique d'entrée pour amortir la
surtension d'un branchement à chaud (voir `combined_design.md`, « 24 V input and 5 V
buck »).

Le GND de `combined` est le noir de l'alimentation : le buck n'est pas isolé.

### Signaux de la pompe — `combined` J9

Les quatre fils de signal de la pompe sont sertis directement dans un boîtier **JST XH
4 pôles** (contacts SXH-001T-P0.6, isolant de 1,5 mm au maximum du contact) :

| J9 | Fil de la pompe | Signal |
| --- | --- | --- |
| 1 | vert | GND_P, référence de la consigne |
| 2 | marron | SPEED_P, consigne 0–5 V |
| 3 | bleu | TACHO_P, tachymètre |
| 4 | jaune | +5V_P, 5 V fourni par la pompe |

Le **blanc** (sens) n'est pas branché, extrémité isolée sous gaine. Ces quatre fils ne
touchent que le côté pompe des deux optocoupleurs : ils ne rejoignent jamais une masse ni
une alimentation de la carte. Détail dans [pompe-engrenage.md](pompe-engrenage.md),
« Interface électrique ».

---

## 5 V et CAN — `combined` J1 → écran

Le buck de `combined` fait le 5 V de toute la basse tension : XIAO, CAN, Digmesa, commande
du SSR chaudière, XDB401 (par le 3,3 V du XIAO) et écran.

**Quatre fils Helutherm 145 0,25 mm²** dans un boîtier **JST XH 4 pôles** sur J1 :

| J1 | Signal | Fil |
| --- | --- | --- |
| 1 | GND | noir |
| 2 | +5V | rouge |
| 3 | CAN_L | orange |
| 4 | CAN_H | gris |

- La **paire CAN est torsadée**. La **paire 5 V / GND** court droite le long de la paire
  CAN, attachée à elle par des bouts de gaine thermo.
- Le câble traverse `boitier_ps_24v` sur le fond, sous les fils 24 V, et **sort par le
  trou de l'ancien bouton brew** (Ø 16 mm), équipé du passe-câble imprimé
  (`print/parts/passe_cable.md`, `ecrou_passe_cable.md`).

### Côté écran

- **CAN** : un **JST SM 2 pôles** relie la paire torsadée au petit câble **JST PH 2.0
  2 pôles** qui se branche sur le PCB du Waveshare (inchangé). Le transceiver TJA1051T/3
  y est intégré : CAN sur **GPIO20 (TX) / GPIO19 (RX)**, `CAN_SEL` (EXIO5 du CH422G) à
  l'état haut.
- **5 V** : deux **Wago 221-412** relient la paire 5 V / GND à un câble USB-C, branché
  dans le câble USB-C coudé qui entre dans le Waveshare (inchangé).

### Bus CAN

Un seul segment, deux nœuds, **120 Ω à chaque extrémité** : R18 soudée sur `combined`,
jumper sur le Waveshare.

---

## Capteurs et commande de la chaudière

### Câble du Digmesa (J6 de `combined`)

Trois fils **Helutherm 145 0,5 mm²**. Le 0,5 mm² est imposé par la section minimale des
contacts VH3.96 côté capteur. **SIGNAL et GND sont torsadés**, le 5 V court le long de la
paire, et le tout est tenu par des bouts de gaine thermorétractable.

| Fil | Signal | Côté `combined` : J6 |
| --- | --- | --- |
| **blanc** | SIGNAL | broche 3, FLOW_PULSE → **GPIO 44** |
| **rouge** | 5 V | broche 2, +5V |
| **noir** | GND | broche 1, GND |

- **Côté Digmesa** : boîtier **VH3.96 3 pôles**.
- **Côté PCB** : embouts de câble dans un **boîtier HT3.96 3 pôles** (clone VH), enfiché
  dans J6.

**Ordre des broches côté capteur.** Le connecteur du Digmesa est un **PANCOM**. Un boîtier
VH3.96 s'y enfiche dans l'ordre inverse de celui qu'on attendrait d'un VH standard. Il faut
placer les fils d'après les **symboles moulés sur le connecteur du capteur**, pas d'après
l'ordre habituel. Le 9 septembre 2026, l'ancien câble inversait SIGNAL et VCC : GPIO 44
restait à environ 2,6 V au repos au lieu de 3,3 V, et aucune impulsion n'était comptée.

**Contrôle avant de brancher le capteur.** Câble sur J6, carte alimentée, mesurer côté
capteur par rapport à l'alvéole GND : environ **5 V** dans l'alvéole 5 V et environ
**3,3 V** dans l'alvéole SIGNAL.

Le Digmesa est un **collecteur ouvert** : il tire la ligne à la masse mais ne la monte
jamais. Sur `combined`, un **1 kΩ vers le 3V3** fixe le niveau haut, et un **10 nF vers
GND** filtre les transitoires. Le capteur peut donc être alimenté en 5 V sans jamais
présenter plus de 3,3 V au GPIO. GPIO en `INPUT`, **pull-up interne éteinte**.

### Commande du SSR de chaudière (J8 de `combined`)

Deux fils **0,75 mm²** : embouts 0,75 mm² dans un boîtier **HT3.96 2 pôles** côté
`combined`, **cosses rouges 4,8 mm** sur les languettes DC du SSR « mini-puck ». Les
cosses rouges demandent cette section ; elles n'entrent pas dans un contact XH.

| J8 | Signal | SSR |
| --- | --- | --- |
| 1 | GND | borne **−** |
| 2 | BOILER_5V | borne **+** |

`combined` commute le 5 V sur la broche 2 (Q5 / Q6) : **HEATER_CMD sur GPIO 4, actif
HIGH**. Des résistances tiennent la sortie coupée tant que le GPIO flotte (démarrage,
reset, XIAO absent). Le circuit de puissance du SSR est décrit dans « SSR de chaudière
fourni avec la machine » ci-dessus.

### Pression — XDB401 analogique (J10 de `combined`)

Sonde **XDB401 0,4–2,4 V, 0–12 bar, alimentée en 3,3 V**. Son câble intégré est serti
directement dans un boîtier **JST XH 3 pôles** :

| J10 | Fil de la sonde | Signal |
| --- | --- | --- |
| 1 | rouge | +3V3 |
| 2 | noir | GND |
| 3 | jaune | PRESSURE, 0,4–2,4 V |

L'ADS1115 de `combined` (U6, adresse 0x49) lit PRESSURE sur AIN1 derrière un filtre
1 kΩ / 10 µF (16 Hz) ; AIN0 surveille le 3,3 V. La sortie de la sonde n'est pas
ratiométrique. L'écran reçoit la pression par le CAN. Circuit :
`../pcb/sensors/sensors_design.md`, « U6 / J10 ».

Les deux sous-sections suivantes décrivent le passage de la sonde I2C à la sonde
analogique, en octobre 2026 ; le suivi est dans [xdb401_swap.md](xdb401_swap.md).

### Calibration de la version analogique

Les deux versions du XDB401 sortent **calibrées d'usine**. Elles partagent la cellule
céramique et la puce de conditionnement ; seule la sortie diffère. La fiche du fabricant
(xidibei.com, XDB401) annonce, en % de la pleine échelle (PE) :

| Caractéristique | Valeur | Pour la pièce 0–12 bar |
| --- | --- | --- |
| Précision (non-linéarité comprise) | 1 % PE | ±0,12 bar |
| Dérive thermique, zéro et sensibilité | ≤ 0,03 % PE/°C | ±0,13 bar pour +35 °C |
| Plage de température compensée | −20 à 80 °C | |
| Temps de réponse | ≤ 4 ms | |
| Surpression admissible | 150 % PE | 18 bar |
| Pression d'éclatement | 300 % PE | 36 bar |

La conversion est donc une **droite fixe** : 0,4 V à 0 bar, 2,4 V à 12 bar, soit
6 bar/V. La surpression de 18 bar couvre l'OPV (~10 bar) et la pompe vibrante en filtre
aveugle (~15 bar). Au-delà de 12 bar, la mesure est écrêtée ; la régulation n'en a pas besoin.

Seules les erreurs **ajoutées entre la sonde et l'ADS1115** restent à traiter :

| Source | Ordre de grandeur | Traitement |
| --- | --- | --- |
| Gain de l'ADS1115 | ≤ 0,15 %, ≤ 0,014 bar à 9 bar | négligé |
| Écart de masse sonde ↔ ADS1115 | 10 mV = 0,06 bar (écart déjà mesuré sur l'ancien pont NTC) | masse prise sur le port de l'ADS1115, puis réglage du zéro |
| Sortie ratiométrique au 3,3 V | ±3 % sur le 3,3 V → jusqu'à ±0,36 bar à 12 bar | test ci-dessous, puis correction par A0 si nécessaire |
| Dérive thermique du zéro | ~0,1 bar entre froid et chaud | réglage du zéro à chaud |

Générer une pression connue au banc n'est pas faisable (raccord 1/8" et source de pression
nécessaires). La caractérisation se fait donc ainsi :

1. **Test ratiométrique, au banc, sans pression.** Alimenter la sonde sous 3,0 V puis sous
   3,3 V et relever la sortie. Si elle passe d'environ 0,40 V à environ 0,36 V, la sortie suit
   l'alimentation : la conversion se fait alors sur le rapport A2/A0. Si elle reste à 0,40 V,
   elle ne dépend pas de l'alimentation : A0 n'intervient pas.
2. **Zéro, sur la machine.** Lire la sonde machine chaude, circuit dépressurisé. L'écart
   à 0,4 V devient l'offset ; il absorbe le zéro d'usine, l'écart de masse et la dérive à
   chaud.
3. **Pente.** Départ sur la pente d'usine (6 bar/V), puis alignement sur le XDB401 I2C au
   plateau de l'OPV, décrit ci-dessous.

### Point de comparaison : plateau de l'OPV en panier aveugle

**But : continuité, pas exactitude.** Les consignes de pression et les captures existantes
ont été établies avec le XDB401 I2C. La sonde analogique doit rendre **la même valeur
numérique** au même point, même si cette valeur s'écarte de la pression vraie.

Le tarage de l'OPV (~10 bar) ne bouge pas entre les deux sondes. Une **purge à 100 % avec
panier aveugle** monte jusqu'à l'ouverture de l'OPV et y forme un plateau, le même avant et
après le remplacement.

- **Avant le démontage du XDB401 I2C** : machine chaude, plusieurs purges à 100 % en panier
  aveugle. Relever pour chacune la lecture à pression atmosphérique juste avant, et le
  plateau. Noter aussi la valeur lue sur le manomètre à aiguille, pour mémoire seulement.
  Retenir la moyenne des **plateau − zéro**, et l'écart entre les purges comme
  répétabilité. Conversion actuelle : pleine échelle 16 bar, offset 0
  (`firmware/screen/main/core/calibration_machine.h`).
- **Après le montage de la version analogique** : mêmes purges, mêmes conditions (commande
  pompe, consigne chaudière, OPV non touchée). Relever la tension au zéro et au plateau.
- **Alignement.** Le zéro reste celui mesuré à pression atmosphérique. La pente est fixée
  pour que le plateau donne la valeur de la sonde I2C :
  pente = (plateau − zéro)<sub>I2C</sub> / (V<sub>plateau</sub> − V<sub>zéro</sub>), en bar/V.
- **Contrôle avant d'aligner.** Les deux sondes sont annoncées à 1 % PE (±0,16 bar et
  ±0,12 bar). La pente d'usine doit donc donner au plateau la valeur de la sonde I2C à
  **0,3 bar** près, plus la dispersion entre purges. Au-delà, chercher d'abord un défaut de
  chaîne (zéro, test ratiométrique, masse) avant d'aligner : l'alignement masquerait le
  défaut au plateau et le laisserait ailleurs sur la plage.

Consigner ici les valeurs relevées : zéro et plateau I2C, manomètre, tensions de la sonde
analogique et pente retenue.

La température interne fournie par le XDB401 I2C disparaît avec lui. C'est la température
du corps de la sonde, utilisée par sa puce pour compenser la pression ; elle ne mesure pas
l'eau et ne sert pas à la régulation.

### Sonde NTC et ADS1115 dans le boîtier de l'écran

La sonde NTC vissée en **G1/8** dans la chaudière rejoint directement le boîtier **screen** voisin, afin de raccourcir son cheminement et de limiter les interférences. L'**ADS1115 16 bits est alimenté en 3,3 V par le connecteur I2C** du Waveshare, sur le bus partagé avec notamment le CH422G et le contrôleur tactile GT911. Le **3V3 du port Sensor AD** rejoint la Wago NTC2, qui dessert une patte de la NTC et **A0** de l'ADS1115. L'autre patte rejoint la Wago NTC1, qui dessert **A1** et la résistance fixe de **4676 Ω** (4,7 kΩ du banc PT1000, 2,193 kΩ jusqu'au 2 octobre 2026). L'autre extrémité de cette résistance retourne au **GND du port Sensor AD**, proche de la référence GND de l'ADS1115. Le GPIO/AD du port Sensor n'est pas utilisé.

Le pont utilisait auparavant un LDO AMS1117 distinct, et sa résistance fixe retournait au GND de l'alimentation 5 V. Un écart mesuré de **10–13 mV** entre ce GND et celui de l'ADS1115 faussait la lecture ratiométrique : A1 indiquait environ 0,133 V côté ADS pour environ 0,143 V au point milieu rapporté au GND de l'alimentation. Le raccordement du pont au port Sensor AD a supprimé cet écart dans le calcul : **47,926 kΩ** par l'ADS contre **47,9 kΩ** au multimètre lors du relevé à froid. La [fiche ADS1115](datasheets/ads1115.pdf) décrit les limites électriques et la programmation de ces entrées.

Sur le Waveshare, le connecteur I2C (H7) est séparé de l'ESP32-S3 par un translateur de
niveau à MOSFET (NDC7002N). Côté connecteur, **deux 4,7 kΩ tirent SDA et SCL vers `I2C_VCC`**,
et deux autres côté ESP32 vers le 3V3 ; le bus du connecteur est donc déjà tiré, et l'ADS1115
n'a pas besoin de pull-ups. `I2C_VCC` est fixé par le cavalier H8 sur **3V3 (défaut)** ou 5 V ;
le montage suppose 3V3. Le port Sensor AD (J6) est câblé 3V3 / GND / AD, directement au rail
de la carte. Les deux ports sont supposés sur le même rail et le même plan de masse (ils sont
voisins sur la carte) ; ce n'est pas vérifié sur le schéma.

Le schéma du pont, les mesures de calibration et le calcul de température sont dans [ntc_ads1115_calibration.md](ntc_ads1115_calibration.md). Les échanges I2C sont décrits en §7.5 de la [fiche ADS1115](datasheets/ads1115.pdf). La lecture de l'ADS1115 est intégrée au firmware `screen` et publiée par `GET /telemetry`.

Une future **PT1000 iOVEO 012EF02202** remplacera la NTC dans le même raccord
G 1/8 de chaudière. Sa partie immergée en inox mesure 9 × 5,5 mm et sa sortie
comporte deux fils silicone de 20 à 25 cm. Le câblage restera donc un pont à
deux fils vers A1. Sur seulement 40 à 50 cm aller-retour, la résistance des
conducteurs devrait produire une erreur de quelques centièmes de degré ; une
mesure sonde montée suffira à vérifier qu'aucune compensation n'est nécessaire.
La résistance fixe est déjà une **4,7 kΩ** (4676 Ω étalonnés), montée avec la
NTC le 2 octobre 2026 à la place de la 2,193 kΩ. Le courant dans la PT1000 sera
ainsi voisin de 0,54 mA vers 100 °C, contre 0,92 mA avec l'ancien pont.

---

## Connectique — récapitulatif

| Type | Où |
| --- | --- |
| **FASTON 6,3 × 0,8 mm** isolées nylon | côté machine : interrupteur principal, LED de façade, vanne. Pas de piggyback |
| **FASTON 4,8 mm** (cosses rouges) | SSR de chaudière fourni avec la machine : commande DC depuis J8 |
| **Wago 221-415** | `boitier_ps_24v` : L et N (arrivée, LED, alimentation, `combined`) |
| **Wago 221-412** | boîtier de l'écran : 5 V / GND vers le câble USB-C |
| **HT3.96** (clone VH), embouts de câble | côté `combined` : J2 (230 V), J3 (vanne), J6 (Digmesa), J7 (24 V, embouts doubles avec la pompe), J8 (SSR chaudière) |
| **VH3.96** | côté Digmesa ; entrée secteur de l'alimentation 24 V (3 positions, centrale vide) |
| **Bornier à vis 5 mm** | sortie 24 V de l'alimentation |
| **JST XH 2,54 mm** | côté `combined` : J1 (écran), J9 (signaux de la pompe), J10 (XDB401) |
| **JST SM 2 pôles** | jonction débrochable du CAN côté écran |
| **JST PH 2.0** | entrée CAN du Waveshare |
| **USB-C** | alimentation 5 V de l'écran, câble coudé |

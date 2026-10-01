# Câblage — détail fil par fil

Vue d'ensemble et choix des composants : `../README.md`. Ce fichier tient le détail :
quel fil, quelle couleur, quelle connectique, d'où à où.

Tout le 230 V reste dans le compartiment technique. Le CAN et le câble de la sonde NTC rejoignent le boîtier de l'écran en façade.

Deux SSR ont des rôles distincts : le **M5Stack Unit SSR** dans `boitier_ac` commande la **vanne**, tandis que le **Keysolu/Maxwell KS53 D-24Z20N-LQ**, fourni avec la machine, commande la **résistance de chaudière**. Le second possède deux bornes d'entrée DC **+ / −** (plage **4–24 VDC indiquée pour l'exemplaire**) et deux bornes de puissance **230 VAC**. La [fiche de la série KS53](https://manage.keysolu.com/upload/product/file/20241204/KS53_EN.pdf) donne une plage générique d'entrée de 4–32 VDC ; le 5 V du montage appartient aux deux plages.

## Câble

| Domaine | Référence | Section |
| --- | --- | --- |
| 230 V | **Helutherm 145** (`datasheets/helutherm145.pdf`) | **0,75 mm²** |
| 230 V — puissance chaudière | câblage de la machine | **1 mm²** |
| 5 V, signaux, CAN | **Helutherm 145** | **0,25 mm²** |

Le **0,75 mm²** décrit les conducteurs ajoutés pour l'alimentation, la vanne et la pompe. Le circuit de puissance de la **résistance de chaudière** est en **1 mm²** ; son chemin est décrit ci-dessous. Le Helutherm 145 tient 145 °C en continu ; l'enceinte de la machine est estimée à 40–50 °C.

### Compatibilité électromagnétique

Le dimmer à triac et la vanne inductive sont des sources d'EMI. La règle est
d'**annuler la surface de boucle** : aller et retour strictement contigus.

- **230 V** : chaque paire phase-neutre est maintenue par de la **gaine thermo** sur toute
  sa longueur. Pas besoin de torsader.
- **CAN** : **paire torsadée** CANH / CANL.
- **XDB401** : la gaine porte une fine feuille de blindage, **sans continuité avec le GND**
  et sans fil de drain : elle ne peut pas être reliée à la masse et reste flottante. Un
  blindage flottant ne protège presque pas du couplage capacitif ; il le relaie et ajoute de
  la capacité aux lignes. Les erreurs I2C fréquentes de R1 s'expliquent ainsi, voir
  [Interférences sur la mesure de pression](#interférences-sur-la-mesure-de-pression).

---

## 230 V

### Entrée — interrupteur principal → `boitier_ps`

Deux **cosses FASTON 6,3 × 0,8 mm isolées** prises sur l'interrupteur principal de la
machine. Tant qu'il n'est pas enfoncé (clic mécanique), rien n'est sous tension : le mod
n'est jamais alimenté machine éteinte.

Les deux fils entrent dans `boitier_ps` par le **sud-ouest** et vont dans ses deux Wago :

| Fil | Couleur | Wago |
| --- | --- | --- |
| Phase | brun | Wago **du bas** |
| Neutre | bleu | Wago **du haut** |

### Sortie — LED de façade

Depuis les mêmes Wago, phase et neutre **ressortent par le sud-ouest** vers la LED de
façade (brun / bleu). Ce départ est obligatoire : les cosses FASTON de la LED ont été
réutilisées pour prendre l'alimentation.

### Sortie — alimentation RECOM

Depuis les mêmes Wago, phase et neutre font un **arc vers le nord** jusqu'à la RECOM
RAC05-05SK/277/W, logée juste au nord dans `boitier_ps`. Brun / bleu.

### Sortie — `boitier_ac`

Quatre conducteurs quittent `boitier_ps` en **deux paires** phase-neutre, chacune tenue par
de la gaine thermo, et entrent dans `boitier_ac` par sa **face est** :

| Destination | Phase | Neutre |
| --- | --- | --- |
| SSR | **jaune** | bleu |
| Dimmer | **violet** | bleu |

### Sorties de `boitier_ac` vers les actionneurs

| Départ | Vers | Phase | Neutre |
| --- | --- | --- | --- |
| SSR | **Vanne solénoïde** OLAB 08252L50-A14-1A-G (bobine 08000BH-J5IV, 15 VA) | jaune | bleu |
| Dimmer | **Pompe vibratoire** OLAB Silent Green 35 W | violet | bleu |

### SSR de chaudière fourni avec la machine

Le SSR est **en série** dans le circuit de puissance de la chaudière, en **1 mm²** :

```text
phase de l'interrupteur principal → borne AC du SSR chaudière
→ autre borne AC du SSR → résistance de chaudière
→ protection / relais thermique → neutre de l'interrupteur principal
```

Les **deux bornes DC + / −** rejoignent le `boitier_pid` et la sortie du HW-399 décrite plus bas. Ce SSR est distinct de celui de la vanne situé dans `boitier_ac`. La protection thermique reste dans le chemin de retour vers le neutre.

### Résumé des paires 230 V

1. **L + N** interrupteur principal → `boitier_ps`
2. **L + N** `boitier_ps` → LED de façade
3. **L + N** `boitier_ps` → alim RECOM (interne au boîtier)
4. **L + N** `boitier_ps` → SSR (`boitier_ac`)
5. **L + N** `boitier_ps` → dimmer (`boitier_ac`)
6. **L + N** SSR → vanne
7. **L + N** dimmer → pompe

Circuit de chauffe distinct : **phase interrupteur → SSR chaudière → résistance → protection thermique → neutre interrupteur** (conducteurs de puissance en **1 mm²**).

---

## 5 V

La RECOM sort **VCC 5 V et GND** dans les **Wago 3 poles au nord de `boitier_ps`**. Deux
départs partent de là, en **0,25 mm² rouge (5 V) / noir (GND)** :

| Départ | Vers | Arrivée |
| --- | --- | --- |
| 1 | **Écran Waveshare** | bornier adaptateur **USB-C** |
| 2 | **`boitier_dc`** | Wago du compartiment **sud-ouest** |

Le 5 V / GND est aussi distribué au **`boitier_pid`** (Wago de distribution pour le HW-399 et la commande du SSR chaudière). Le pont NTC du boîtier **screen** est alimenté par le **3V3 et le GND du port Sensor AD du Waveshare**. Le LDO AMS1117 ajouté auparavant pour ce pont a été retiré après la correction de sa référence de masse.

### Distribution dans `boitier_dc`

Compartiment **sud-ouest**, deux Wago :

- **3 poles à gauche = 5 V**
- **2 poles à droite = GND**

En repartent :

- **5 V + GND vers le XIAO**, dans le **bornier 2 poles** soudé sur les pastilles 5 V / GND
  du Grove Shield (une patte du condensateur du filtre RC partage cette borne).
- **5 V vers la Wago 3 poles du compartiment nord-ouest.**

La Wago **nord-ouest** distribue le 5 V à deux consommateurs :

- le **SSR** — fil du câble Grove dont le VCC a été coupé à ras côté XIAO, puis dénudé et
  repris ici ;
- le **Digmesa** — fil rouge de son câble JST SM.

---

## Bus CAN

Un seul segment, deux nœuds, **120 Ω activée par jumper aux deux extrémités**.

### Côté XIAO — Adafruit CAN Pal (TJA1051T/3)

Monté au **nord de `boitier_dc`**.

- **Quatre pastilles de gauche** (VCC, GND, RX, TX de gauche à droite) : **bornier à vis
  2,54 mm** soudé, relié au port Grove **R3** par un câble dénudé d'un côté, Grove de
  l'autre. **TX → fil blanc → GPIO 7**, **RX → fil jaune → GPIO 8** (port Grove **D8/D9**
  du silkscreen Seeed — le D-number, pas le GPIO natif : voir la note ci-dessous).
- **Deux pastilles de droite** (CANH, CANL de gauche à droite) : **connecteur PCB femelle
  JST XH 2,54 mm**.
- **Broche `SLNT`** (pastille suivante du header, entre TX et CANH) : reliée au **GND**
  (vis GND du bornier CAN). Sur ce clone, `SLNT` flottant se retrouve tiré haut (mode
  Silent, émetteur coupé) au lieu de rester bas comme le prévoit le pull-down interne du
  TJA1051

### La ligne

**Paire torsadée Helutherm 145 0,25 mm²** :

| Signal | Couleur |
| --- | --- |
| **CANL** | orange |
| **CANH** | gris |

Elle chemine dans la machine et **sort par le trou de l'ancien bouton brew** (Ø 16 mm),
équipé du passe-câble imprimé (`print/parts/passe_cable.md`, `ecrou_passe_cable.md`).

### Côté écran

Un **JST SM 2 poles** relie la paire torsadée à un câble **JST PH 2.0 2 poles** qui se
branche sur le PCB du Waveshare. Le transceiver TJA1051T/3 y est intégré : CAN sur
**GPIO15 (TX) / GPIO16 (RX)**, `CAN_SEL` (EXIO5 du CH422G) à l'état haut.

---

## Ports Grove du XIAO

Shield tenu XIAO en bas : colonne **gauche = L**, colonne **droite = R**, port **1** au plus
près du XIAO, **4** au plus loin.

**Le silkscreen du Grove Shield XIAO numérote en `Dn` (D-number Seeed), pas en GPIO natif
de l'ESP32-S3.** Au-delà de D5 les deux numérotations divergent : D6/D7 partent sur
GPIO43/44 (réservés à l'UART0), ce qui décale tout ce qui suit — D8→GPIO7, D9→GPIO8,
D10→GPIO9. Le tableau ci-dessous donne les deux, `Dn` étant ce qui est effectivement
imprimé sur le PCB du shield. Erreur découverte et corrigée le 2026-09-08, au multimètre
puis par un auto-test de bouclage transceiver (`firmware/can-selftest`), après un bring-up
phase 2 resté silencieux sur le bus faute de cette traduction.

| Port | Périphérique | Signaux (D-number) | GPIO natif | Alim |
| --- | --- | --- | --- | --- |
| **R1** | XDB401 (pression) | I2C — SDA D4, SCL D5 | SDA **GPIO 5**, SCL **GPIO 6** | 3,3 V |
| **R2** | Digmesa (débit) | impulsions — D7 | **GPIO 44** | 5 V, hors du câble Grove |
| **R3** | CAN Pal | TX D8, RX D9 | TX **GPIO 7**, RX **GPIO 8** | 3,3 V |
| **R4** | Unit SSR | commande — D10 (fil jaune) | **GPIO 9** | 5 V, hors du câble Grove |
| **L2** | HW-399 → SSR chaudière | commande — D2 | **GPIO 3** | 5 V côté sortie du HW-399 |
| **L4** | Dimmer DimmerLink | I2C — SDA D4, SCL D5 | SDA **GPIO 5**, SCL **GPIO 6** | 3,3 V |

**L4 est une copie de R1** : le dimmer et le capteur de pression partagent le même bus I2C.
Le dimmer n'a **pas de pull-up** ; ce sont les **4,7 kΩ du XDB401** qui tiennent SDA et SCL
pour tout le bus. Retirer le capteur de pression laisse les lignes sans tirage et le dimmer
muet (sauf câble très court, sur les pull-ups internes de l'ESP32).

Ces pull-ups sont au bout des ~30 cm de câble, côté sonde. Pour le niveau statique, leur
position est indifférente. Le temps de montée reste dans la norme : 4,7 kΩ avec ~100 pF
(câble blindé, câbles Grove, dimmer) donnent ~0,4 µs, sous la limite de 1 µs à 100 kHz
(`scl_speed_hz = 100000` dans `firmware/sensors`). **Doubler les pull-ups est permis**, et
même favorable contre le bruit, car une ligne tirée plus fort résiste mieux aux injections.
La limite est le courant qu'un composant doit absorber pour tenir la ligne basse : 3 mA en
mode standard, soit une résistance équivalente d'au moins **1,1 kΩ** sous 3,3 V. Deux 4,7 kΩ
en parallèle font 2,35 kΩ, soit 1,4 mA ; une 2,2 kΩ en parallèle des 4,7 kΩ du XDB401 fait
1,5 kΩ, soit 2,2 mA. Les deux sont acceptables.

Lors du remplacement prévu du XDB401 I2C par sa version analogique, ajouter au niveau du
XIAO **une résistance de 4,7 kΩ entre SDA et 3,3 V et une autre entre SCL et 3,3 V**. Les
pull-ups internes de l'ESP32 ne constituent pas un remplacement robuste.

Souder ces résistances sur les pastilles du shield est écarté : la pastille 3,3 V est déjà
utilisée. Un réseau de deux résistances à point commun exigerait une plaque de prototypage.
Les résistances iront donc dans un **petit connecteur Grove à quatre broches** (SDA → 4,7 kΩ
→ 3,3 V, SCL → 4,7 kΩ → 3,3 V) branché sur une prise I2C du shield, par exemple celle du
XDB401 (R1) une fois libérée. Ce module doit être en place **dès que le XDB401 est débranché** :
sans lui, le dimmer ne répond plus, sans autre symptôme. Le laisser branché en même temps
que le XDB401 I2C est sans danger (2,35 kΩ, voir plus haut).

### Câble du XDB401 (R1)

Câble **Grove à clip** de quelques centimètres, sur lequel est serti un **JST SM 4 poles**.
Le XDB401 porte le SM complémentaire au bout de son propre câble : la connexion se fait
**hors du boîtier**, à proximité immédiate, et la sonde se débranche sans rien ouvrir.

Fils du capteur : **rouge VCC, noir GND, vert SDA, blanc SCL**. Alimenté en **3,3 V** —
l'emballage annonce 5 V, mais l'émulation XDB401 est complète en 3,3 V (vérifié au banc).

### Câble du Digmesa (R2)

**Piège rencontré et corrigé (2026-09-09), après une session entière de diagnostic
électrique qui a d'abord fait suspecter le firmware/le filtre RC** : le connecteur du
capteur est un **PANCOM** (introuvable dans le commerce pour ce remplacement), sur lequel
un câble **VH3.96 3 pôles classique s'enfiche à l'envers** — les pastilles du connecteur
sensé être complémentaire ne sont pas dans le même ordre. Conséquence, côté capteur,
**les couleurs ne portent pas les signaux qu'on attendrait d'un VH3.96 standard** :

| Fil (côté capteur, VH3.96) | Signal réel |
| --- | --- |
| **rouge** | SIGNAL |
| **noir** | GND |
| **jaune** | VCC |

Ce câble VH3.96 se termine sur un **connecteur JST SM 3 pôles mâle fait maison** (serti à
la main, pas un pigtail acheté). Il s'enfiche dans un **JST SM 3 pôles femelle**, lui aussi
fait maison, au bout d'un tronçon de câble Grove : c'est **à cette jonction femelle qu'a
été appliqué le correctif** — rouge et jaune y ont été **intervertis** pour compenser
l'inversion en amont, afin que le reste de la chaîne (Grove → shield, fil vers la Wago)
retrouve la bonne identité de signal malgré le VH3.96 câblé à l'envers côté capteur.

En aval de ce correctif, la chaîne redonne exactement ce qu'attend le firmware :

| Fil (après le JST SM femelle) | Va vers |
| --- | --- |
| **noir** (GND) | serti dans le connecteur **Grove** → port R2 |
| **jaune** (signal) | serti dans le même connecteur Grove → **GPIO 44** (D7 sur le silkscreen) |
| **rouge** (5 V) | seul, dans la **Wago du compartiment nord-ouest** |

Le câble Grove utilisé pour ce tronçon a 4 fils de base (noir/rouge/blanc/jaune) ; seuls
**noir et jaune sont sertis dans le connecteur Grove**, **blanc et rouge sont coupés à ras**
à cet endroit (le VCC de ce câble-ci ne sert pas, le 5 V arrive par le fil rouge séparé
jusqu'à la Wago, pas par le connecteur Grove).

**Symptôme observé avant correction** : tension de repos anormale sur GPIO 44 (~2,64 V au
lieu des ~3,3 V attendus du filtre RC), et surtout **aucune impulsion jamais comptée**
malgré une turbine visiblement en rotation — le signal réel (rouge) était en fait câblé sur
la broche VCC en aval, et le VCC réel (jaune) sur la broche signal, avant l'inversion
corrective au JST SM femelle. Voir `docs/firmware-implementation.md` pour le détail de la
session de diagnostic (comparaison avec `tests/test_flowmeter.py`, tentative sur GPIO2/pull-up
interne, mesures ADC) qui a fini par isoler ce câblage plutôt qu'un bug logiciel.

**Filtre RC**, soudé sur les pastilles à gauche du XIAO (1 = 5 V, 2 = GND, 3 = 3V3,
D7 = GPIO 44) :

```
        3V3 (pastille 3)
             │
            1 kΩ
             │
GPIO 44 ─────┼────── signal Digmesa (fil jaune, collecteur ouvert NPN)
        (pastille D7)
             │
           10 nF
             │
        GND (pastille 2)
```

Le Digmesa est un **collecteur ouvert** : il tire la ligne à la masse mais ne la monte
jamais. C'est le 1 kΩ vers 3,3 V qui fixe le niveau haut — le capteur peut donc être
alimenté en 5 V sans jamais présenter plus de 3,3 V au GPIO. Le 10 nF filtre les
transitoires. GPIO en `INPUT`, **pull-up interne éteinte**.

### Câble du SSR de vanne (R4)

Câble **Grove 10 cm** dont le **VCC est coupé à ras côté XIAO**, dénudé et repris dans la
**Wago nord-ouest** (5 V). Le **fil jaune** porte la commande vers le SSR, sur **GPIO 9**
(D10 sur le silkscreen du Grove Shield). Le SSR est donc alimenté en 5 V par la Wago, pas
par le port Grove.

### Commande du SSR de chaudière (L2)

Le port Grove **L2** du Shield (broche **D2** sur le PCB, **GPIO 3** natif du XIAO) porte la commande du chauffage. Le GPIO commande **IN4** du HW-399 ; la commande est inversée côté logiciel : **LOW active le chauffage, HIGH l'arrête**. Le GPIO en 3,3 V ne commande pas directement le SSR Keysolu/Maxwell, dont l'entrée de l'exemplaire est indiquée pour 4–24 VDC.

Le câble Grove mène au connecteur **XH 2 pôles** du HW-399 dans `boitier_pid` : **GND + IN4**. La voie **IN4 / OUT4** du module optocoupleur est utilisée. Le côté sortie reçoit **5 V et GND** des Wago de distribution ; il sort par un **XH 3 pôles GND, VCC, OUT4**. Le SSR est câblé en mode absorption : **5 V (VCC) → borne `+` du SSR ; borne `−` du SSR → OUT4**. Quand GPIO 3 / IN4 est LOW, le transistor de sortie tire OUT4 vers GND et le SSR s'allume ; quand GPIO 3 est HIGH, OUT4 remonte à 5 V et le SSR s'éteint. Le SSR fourni avec la machine porte des **languettes mâles FASTON 4,8 mm**. Son circuit de puissance est décrit dans la section « SSR de chaudière fourni avec la machine » ci-dessus.

### Sonde NTC et ADS1115 dans le boîtier de l'écran

La sonde NTC vissée en **G1/8** dans la chaudière rejoint directement le boîtier **screen** voisin, afin de raccourcir son cheminement et de limiter les interférences. L'**ADS1115 16 bits est alimenté en 3,3 V par le connecteur I2C** du Waveshare, sur le bus partagé avec notamment le CH422G et le contrôleur tactile GT911. Le **3V3 du port Sensor AD** rejoint la Wago NTC2, qui dessert une patte de la NTC et **A0** de l'ADS1115. L'autre patte rejoint la Wago NTC1, qui dessert **A1** et la résistance fixe mesurée de **2,193 kΩ**. L'autre extrémité de cette résistance retourne au **GND du port Sensor AD**, proche de la référence GND de l'ADS1115. Le GPIO/AD du port Sensor n'est pas utilisé.

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
La résistance fixe actuelle de 2,193 kΩ sera remplacée par une **4,7 kΩ** de
précision, dont la valeur réelle sera mesurée et utilisée dans le firmware.
Le courant dans la PT1000 sera ainsi voisin de 0,54 mA vers 100 °C, contre
0,92 mA avec le pont actuel.

### Évolution prévue : XDB401 analogique sur l'ADS1115

Une version du XDB401 à sortie **0,4–2,4 V**, alimentée en **3,3 V**, pourra utiliser
l'entrée **A2** du même ADS1115. A0 restera la mesure du 3,3 V, A1 celle du pont de
température et A3 restera libre. La plage unipolaire 0,4–2,4 V est compatible avec
l'alimentation 3,3 V de l'ADS1115 et avec son réglage actuel à ±4,096 V. À ce gain, un
code vaut 125 µV. La pièce commandée couvre **0–12 bar** sur 2,0 V, soit une résolution
brute d'environ **0,00075 bar (0,75 mbar) par code**, avant bruit et calibration.

La référence de la pièce commandée est « 3.3V OUT 0.4-2.4V, 0-1,2 MPa, G1/8 » : elle
s'alimente bien en 3,3 V. D'autres annonces de la même famille XDB401 0,4–2,4 V indiquent
5–12 V ; ne pas les confondre. **Aucune résistance n'est nécessaire** entre la sortie et A2 :
l'entrée de l'ADS1115 est de haute impédance. Seul le filtre RC optionnel ci-dessous peut
s'ajouter.

Relier la masse de la sonde au GND du même port que l'ADS1115, et acheminer la sortie et la
masse ensemble, torsadées si possible et à l'écart du 230 V. Un petit filtre RC au plus près
de A2 pourra être ajouté si les captures montrent du bruit.

Ce montage raccourcit le trajet analogique et supprime le XDB401 I2C ainsi que ses attentes
de conversion sur le bus du XIAO. Le dimmer reste toutefois en I2C et nécessite alors les
deux pull-ups de 4,7 kΩ décrites plus haut. La mesure se fait aussi là où tourne la
régulation (`core` sur l'écran), sans passer par le CAN. Le câblage actuel avec le XDB401 I2C
reste la référence jusqu'à la réception et à la caractérisation de la nouvelle pièce.

#### Calibration de la version analogique

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

#### Point de comparaison : plateau de l'OPV en panier aveugle

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

### Interférences sur la mesure de pression

**Constat.** Le câble I2C du XDB401 (R1) fait ~30 cm. Il passe près des câbles 230 V du SSR
de chaudière, et son blindage ne peut pas être relié à la masse. Des erreurs I2C
(`I2C_ERROR`, `XDB401_TIMEOUT`) y apparaissent régulièrement.

**Mécanisme.** L'I2C est asymétrique et à drain ouvert : l'état haut n'est tenu que par une
pull-up de 4,7 kΩ. Les fronts rapides voisins s'y injectent par couplage capacitif : la
commutation du SSR, la découpe de phase du dimmer et la pompe, qui est une charge inductive.
Le champ magnétique 50 Hz du courant de chaudière compte peu. Le blindage flottant ne
protège presque pas. Une injection donne un NACK, un octet faux ou une ligne SDA bloquée. Le
bus est **partagé avec le dimmer** : une ligne bloquée prive aussi la pompe de commande.

Trois options :

| | A — analogique sur l'ADS1115 de l'écran | B — I2C sur le connecteur du Waveshare | C — I2C actuel amélioré |
| --- | --- | --- | --- |
| Montage | sortie 0,4–2,4 V sur A2, câble plus court | XDB401 I2C sur H7, câble plus court | inchangé |
| Nature du signal | tension pilotée par la sonde (faible impédance) | lignes tenues par pull-ups | lignes tenues par pull-ups |
| Pull-ups | dimmer : module 4,7 kΩ ajouté sur le XIAO | 4,7 kΩ Waveshare ∥ 4,7 kΩ sonde = 2,35 kΩ | 2,2 kΩ côté XIAO ∥ 4,7 kΩ sonde = 1,5 kΩ |
| Effet d'un parasite | un échantillon faussé, filtré par l'ADS1115, un RC ou un médian | transaction perdue ou bus bloqué | transaction perdue ou bus bloqué |
| Bus exposé au câble | aucun : l'ADS1115 est dans le boîtier | bus de l'écran : CH422G, GT911 tactile, ADS1115 (NTC chaudière) | bus du XIAO : dimmer |
| Gamme, précision | 0–12 bar, ±0,12 bar | 0–16 bar, ±0,16 bar | 0–16 bar, ±0,16 bar |
| Température de la sonde | perdue | conservée | conservée |
| Trajet jusqu'à la régulation | direct | direct | par le CAN |
| Pièce ou travail | sonde commandée, firmware screen, module pull-ups | adresse 0x7F à vérifier par scan du bus, firmware screen | pull-ups, routage, firmware sensors |

Mesures communes aux trois options : croiser les câbles 230 V à 90° plutôt que de les
longer, et garder chaque signal contre sa masse. Pour B et C, côté firmware : récupération
du bus (9 impulsions SCL) et nouvel essai après une erreur.

### Câble du dimmer (L4)

Câble **Grove** simple, alimentation **3,3 V**, I2C sur GPIO 5 / 6 (SDA/SCL, D4/D5 sur le
silkscreen). En mode DimmerLink,
le Cortex du module gère la détection de passage par zéro et le triac ; le XIAO ne fait que
de l'I2C. **Sans secteur sur le dimmer, le module reste en `Calibrating...`** et n'accepte
aucune commande.

---

## Connectique — récapitulatif

| Type | Où |
| --- | --- |
| **FASTON 6,3 × 0,8 mm** isolées nylon | côté machine : interrupteur principal, LED, pompe, vanne. Pas de piggyback |
| **FASTON 4,8 mm** mâles | sur le SSR de chaudière fourni avec la machine |
| **Wago 221** (412 / 415 / 423) | toutes les dérivations, 230 V et 5 V |
| **Grove** | XIAO ↔ périphériques |
| **JST SM** | jonctions débrochables : XDB401 (4 p.), Digmesa (3 p.), CAN (2 p.) |
| **JST XH 2,54 mm** | sortie CANH / CANL du CAN Pal ; HW-399 : entrée 2 pôles (GND, IN4) et sortie 3 pôles (GND, VCC, OUT4) |
| **JST PH 2.0** | entrée CAN du Waveshare |
| **VH3.96** | côté Digmesa |
| **Bornier à vis 2,54 mm** | pastilles VCC/GND/RX/TX du CAN Pal ; pastilles 5 V/GND du Grove Shield |
| **Bornier adaptateur USB-C** | alimentation 5 V de l'écran |

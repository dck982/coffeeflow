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
- **XDB401** : la gaine porte une fine feuille de blindage, **sans continuité avec le GND**.
  La relier à la masse **uniquement côté ESP32** ; côté sonde, coupée et isolée — jamais les
  deux, ça ferait une boucle.

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

Le 5 V / GND est aussi distribué au **`boitier_pid`** (Wago de distribution pour le HW-399 et la commande du SSR chaudière). Dans le boîtier **screen**, le 5 V alimente également un **LDO AMS1117** équipé d'un connecteur XH, dédié au 3,3 V du pont NTC. Le PCB Waveshare ne présente pas de reprise 3,3 V facilement accessible sur une pastille ou un connecteur séparé.

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
  TJA1051 — voir `docs/canpal-findings.md` pour le diagnostic complet et la validation
  (2026-09-09).

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
muet (sauf câble très court, sur les pull-ups internes de l'ESP32). Ne pas ajouter un second
4,7 kΩ côté MCU tant que le XDB401 est là.

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

La sonde NTC vissée en **G1/8** dans la chaudière rejoint directement le boîtier **screen** voisin, afin de raccourcir son cheminement et de limiter les interférences. L'**ADS1115 16 bits est alimenté en 3,3 V par le connecteur I2C** du Waveshare, sur le bus partagé avec notamment le CH422G et le contrôleur tactile GT911. Un **AMS1117** distinct, avec connecteur XH, transforme le 5 V de l'alimentation en 3,3 V stabilisé pour le **pont NTC**. Ce 3,3 V va à **A0** de l'ADS1115 et, via une **Wago**, à une borne de la NTC. Le retour de la NTC va à **A1** et à une résistance mesurée de **2,193 kΩ** vers **GND**. Les masses sont communes.

Les deux rails 3,3 V ont des sources différentes ; leurs tensions ont été vérifiées sur la machine. La valeur ponctuelle mesurée en sortie du LDO n'est pas une constante de calcul : l'ADS1115 lit **A0 et A1**, puis le firmware utilise leur **rapport** pour obtenir la résistance NTC. La [fiche ADS1115](datasheets/ads1115.pdf) décrit les limites électriques et la programmation de ces entrées.

Le schéma du pont, les mesures de calibration et le calcul de température sont dans [ntc_ads1115_calibration.md](ntc_ads1115_calibration.md). Les échanges I2C sont décrits en §7.5 de la [fiche ADS1115](datasheets/ads1115.pdf). La lecture de l'ADS1115 n'est pas encore intégrée au firmware `screen`.

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
| **JST XH 2,54 mm** | sortie CANH / CANL du CAN Pal ; HW-399 : entrée 2 pôles (GND, IN4) et sortie 3 pôles (GND, VCC, OUT4) ; connecteur du LDO AMS1117 |
| **JST PH 2.0** | entrée CAN du Waveshare |
| **VH3.96** | côté Digmesa |
| **Bornier à vis 2,54 mm** | pastilles VCC/GND/RX/TX du CAN Pal ; pastilles 5 V/GND du Grove Shield |
| **Bornier adaptateur USB-C** | alimentation 5 V de l'écran |

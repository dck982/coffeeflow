# coffeeflow

Modification d'une **Profitec Go** pour du *flow profiling* et du *brew by weight* : l'infusion se lance depuis un écran tactile, la pression se module, et elle s'arrête toute seule au poids cible.

## Machine d'origine

- Vanne solénoïde trois voies + pompe vibratoire 35 W + boiler
- Petit boîtier PID : consigne / mesure de température du boiler, chrono
- Vanne OPV pour limiter la pression
- Plomberie en **1/8"**
- Connexions électriques internes : cosses à languette **FASTON 6,3 × 0,8 mm** (pas de piggyback : la dérivation se fait dans un boîtier imprimé)
- Deux compartiments : **technique** (hydraulique, boiler, groupe, électricité) et **réservoir d'eau**
- Bouton **brew** en façade (Ø 16 mm) : démarrait vanne + pompe. **Supprimé** — le trou sert de passe-câble CAN vers l'écran
- LED de façade : ses cosses FASTON ont été réutilisées, la LED est réalimentée depuis le boîtier PS

## Ce que la modification ajoute

Le 230 V de la pompe et de la vanne est repris dans un boîtier dédié, sans remplacer le câblage d'origine :

1. **Pompe** — un dimmer AC 4 A en mode **DimmerLink** (I2C) fait varier la pression.
2. **Vanne solénoïde** — un SSR ouvre et ferme le circuit d'infusion.

Plus de bouton brew, plus de relais NC de « mode manuel » : la commande passe par l'écran. Le système est alimenté dès que l'**interrupteur principal** de la machine est enfoncé.

Deux retours capteurs, sur le module interne :

- **Pression** sur la plomberie, en amont de la vanne (I2C 3,3 V)
- **Débit** en amont de la pompe, côté basse pression (le capteur ne tient que 3 bar)

Le **poids** vient d'une balance **Acaia Lunar** en **BLE**, lue par l'écran. La pesée du drip tray par cellule de charge est **abandonnée pour l'instant** (peser quelques grammes sur un plateau d'un kilo) — pièces et cotes conservées dans `print/parts/*_pesage*` et `docs/driptray.md`.

## Architecture électronique

**Quatre boîtiers imprimés, deux nœuds ESP32-S3.** Le 230 V ne sort pas de la machine.

```
             interrupteur principal (230 V L/N)
                          │
                    [boitier_ps]      intérieur, face ouest, le long du réservoir
                  alim RECOM 5 W · Wago            │
                          │                        └── L/N → LED de façade
              ┌───────────┴───────────┐
          5 V / GND                 L/N ×2
              │                       │
   ┌──────────┴──────────┐      [boitier_ac]   accolé à boitier_dc
   │                     │       dimmer · SSR
[boitier_dc]        [screen_*]        │    │
 XIAO ESP32-S3       Waveshare 4,3"   │    └── L/N → vanne solénoïde
 + Grove Shield      ESP32-S3         └────── L/N → pompe vibratoire
 + CAN Pal           UI · Wi-Fi · BLE
      │                   │
      └──── bus CAN ──────┘   paire torsadée, par le trou Ø16 de l'ex-bouton brew
      │
      ├── I2C 3,3 V ─→ dimmer (DimmerLink)
      ├── GPIO ──────→ SSR
      ├── I2C 3,3 V ←─ capteur de pression XDB401
      └── GPIO ──────← débitmètre Digmesa
```

| Nœud | Alias | Où | Rôle |
| --- | --- | --- | --- |
| **Waveshare 4,3" LCD Touch** (ESP32-S3-WROOM) | *screen*, *écran*, *waveshare* | façade, dans `screen_base` + `screen_wedge` | Interface utilisateur, algorithme d'infusion (flow control, stop on weight), **Wi-Fi et BLE** (Acaia Lunar) |
| **Seeed XIAO ESP32-S3** sur Grove Shield | *xiao*, *module interne*, *module capteurs*, *contrôleur* | intérieur, `boitier_dc`, zone froide entre le module PID et le cadran manomètre | Capteurs (XDB401, Digmesa) et actionneurs (dimmer, SSR) |

Les deux nœuds parlent **CAN**. L'écran est le seul à utiliser la radio.

Maquette Three.js de la machine (ouvrir dans le navigateur) : `docs/profitec_go.html`.

### Pourquoi un XIAO à l'intérieur

L'Atom Echo S3R de l'ancienne architecture est spécifié à **40 °C**. Le XIAO ESP32-S3 tient **85 °C**. Le compartiment technique monte à 45–50 °C : la marge de l'Atom est nulle, celle du XIAO est utilisable. L'écran Waveshare reste en façade, hors de cette enceinte.

---

## Boîtiers

| Boîtier | Pièces `print/parts/` | Où | Contenu |
| --- | --- | --- | --- |
| **PS** | `boitier_ps`, `couvercle_ps` | intérieur, contre la face **gauche** vue de face, le long de la séparation avec le réservoir | Alim RECOM, Wago 230 V (entrée + LED façade), Wago 5 V / GND |
| **DC** | `boitier_dc` | intérieur, **zone froide** entre le module PID et le cadran manomètre | XIAO + Grove Shield, Adafruit CAN Pal, Wago 5 V |
| **AC** | `boitier_ac` | accolé au DC | Dimmer 4 A DimmerLink, M5Stack Unit SSR |
| **UI** | `screen_base`, `screen_wedge` | façade de la machine | Écran Waveshare 4,3" |

Les boîtiers **AC et DC sont côte à côte et reliés par leur couvercle**, qui est commun aux deux (`couvercle_acdc`, assemblage `ensemble_boitiers`).

---

## Composants

| Rôle | Matériel retenu | Fiche |
| --- | --- | --- |
| UI / algo / radio | **Waveshare ESP32-S3-Touch-LCD-4.3** — 800 × 480, TJA1051T/3 CAN intégré, alim USB-C 5 V | `docs/datasheets/waveshare-4-3-lcd-touch.md` |
| MCU interne | **Seeed Studio XIAO ESP32-S3** sur **Grove Shield for XIAO** (8 ports Grove) | `docs/datasheets/xiao.pdf`, `docs/datasheets/grove-expansion-board.md` |
| Transceiver CAN côté XIAO | **Adafruit CAN Pal** (TJA1051T/3), [produit 5708](https://www.adafruit.com/product/5708) | — |
| Dimmer pompe | **RBDimmer AC Dimmer 4 A 1 canal**, logique 3,3 V, mode **[DimmerLink](https://www.rbdimmer.com/docs/dimmerlink-overview)** ([boutique](https://www.rbdimmer.com/shop/ac-dimmers-1/ac-dimmer-module-4a-1-channel-33v5v-logic-ac-400v-4a-6?attribute_values=48)) | — |
| SSR vanne | **M5Stack Unit SSR** (2 A), commande 3,3–5 V, zero-crossing MOC3043M | `docs/datasheets/m5stack-unit-ssr.md` |
| Pression | **Yufavor XDB401**, I2C, filetage **G1/8** | `docs/datasheets/xidibei_xdb401.pdf` |
| Débit | **Digmesa FHKSC 932-9525-B**, buse **1,00 mm** | `docs/datasheets/flowmeter-digmesa.pdf`, `docs/debitmetres.md` |
| Alimentation | **RECOM RAC05-05SK/277/W** — 5 W, 5 V / 1 A, encapsulée, 85–305 VAC, version fils | `docs/datasheets/RAC05-K_277.pdf` |
| Poids | **Acaia Lunar**, BLE, lue par l'écran | — |
| Câble | **Helutherm 145** — 0,75 mm² en 230 V, 0,25 mm² en 5 V / signaux / CAN | `docs/datasheets/helutherm145.pdf` |

Actionneurs d'origine pilotés : vanne solénoïde **OLAB 08252L50-A14-1A-G** (bobine 08000BH-J5IV, 15 VA, 220/230 V 50/60 Hz, orifice Ø 1,4 mm, joint FKM, laiton CW510L, NSF) et pompe vibratoire **OLAB Silent Green 35 W**.

---

## Le module interne (XIAO)

### Ports Grove du Shield

Le Shield expose **deux colonnes de quatre connecteurs**. XIAO en bas : colonne de **gauche = L**, colonne de **droite = R** ; le port le plus proche du XIAO est **1**, le plus éloigné **4**.

| Port | Périphérique | Bus / GPIO | Alim | Câble |
| --- | --- | --- | --- | --- |
| **R1** | Capteur de pression XDB401 | I2C — SDA **GPIO 4**, SCL **GPIO 5** | 3,3 V | Grove à clip (quelques cm) → JST SM 4 poles, débrochable **hors du boîtier** |
| **R2** | Débitmètre Digmesa | impulsions — **GPIO 7** | 5 V (voir ci-dessous) | Grove 2 fils (noir + jaune) → JST SM 3 poles → VH3.96 côté Digmesa |
| **R3** | Adafruit CAN Pal | TWAI — TX **GPIO 8** (fil blanc), RX **GPIO 9** (fil jaune) | 3,3 V | Grove → fils dénudés dans le bornier à vis du CAN Pal |
| **R4** | M5Stack Unit SSR | commande — **GPIO 10** (fil jaune) | 5 V, repris hors du câble | Grove 10 cm, VCC coupé à ras côté XIAO |
| **L4** | Dimmer DimmerLink | I2C — SDA **GPIO 4**, SCL **GPIO 5** | 3,3 V | Grove |

**L4 est câblé en copie de R1** pour exposer une deuxième prise I2C. Le dimmer et le capteur de pression sont donc **sur le même bus** : le dimmer n'a pas de pull-up, il profite des **4,7 kΩ du XDB401**. Conséquence : retirer le capteur de pression laisse SDA / SCL sans tirage et le dimmer ne répond plus (sauf câble très court, sur les pull-ups internes de l'ESP32). Ne pas empiler un second 4,7 kΩ côté MCU tant que le XDB401 est sur le bus.

### Pastilles du Shield

Le Shield porte une rangée de pastilles à **gauche du XIAO** (board tenu XIAO en bas, ports vers le haut). Les trois premières sont **5 V**, **GND**, **3V3** ; la septième est **GPIO 7**.

- Un **bornier 2 poles** est soudé sur les pastilles 5 V et GND : il alimente le XIAO depuis le boîtier PS et sert de point de reprise.
- **Filtre RC du débitmètre** : résistance **1 kΩ** entre la pastille 3 (3V3) et la pastille 7 (GPIO 7), condensateur **10 nF** à cheval entre la pastille 2 (GND) et la pastille 7. Le Digmesa est un collecteur ouvert NPN : il tire la ligne à la masse mais ne la monte jamais, c'est le tirage vers 3,3 V qui fixe le niveau haut, alors même que le capteur est alimenté en 5 V. GPIO en `INPUT`, pull-up interne éteinte.

### Bus CAN

Le CAN Pal est monté au **nord de `boitier_dc`**.

- Un **bornier à vis 2,54 mm** est soudé sur ses **quatre pastilles de gauche** (VCC, GND, RX, TX de gauche à droite) et rejoint le port **R3** par un câble dénudé d'un côté, Grove de l'autre.
- Les **deux pastilles de droite** (CANH, CANL) portent un **connecteur PCB femelle JST XH 2,54 mm**.
- La ligne est une **paire torsadée Helutherm 145 0,25 mm²** : **CANL = orange, CANH = gris**. Elle chemine dans la machine et sort par le trou de l'ancien bouton brew, équipé du passe-câble imprimé (`passe_cable` + `ecrou_passe_cable`).
- À l'autre bout, un **JST SM 2 poles** la relie à un câble **JST PH 2.0 2 poles** qui se branche sur le PCB du Waveshare (transceiver TJA1051T/3 intégré, CAN sur **GPIO15 TX / GPIO16 RX**, `CAN_SEL` = EXIO5 à l'état haut).
- Les **résistances de terminaison 120 Ω sont activées par jumper des deux côtés**.

---

## Alimentation

L'alim **RECOM RAC05-05SK/277/W** vit dans `boitier_ps` et alimente **tout** : les deux modules, les capteurs et les actionneurs basse tension.

### 230 V

- **Phase et neutre** viennent de deux **cosses FASTON isolées** prises sur l'**interrupteur principal** de la machine : tant qu'il n'est pas enfoncé (clic mécanique), rien n'est sous tension.
- Les deux fils (**Helutherm 145 0,75 mm²**) entrent dans `boitier_ps` par le **sud-ouest** et vont dans les deux Wago (**phase en bas, neutre en haut**).
- De ces mêmes Wago, phase et neutre **ressortent par le sud-ouest** pour alimenter la **LED de façade** — nécessaire, ses cosses FASTON ayant été réutilisées. **Phase brun, neutre bleu.**
- Phase et neutre ressortent aussi en **arc vers le nord** pour alimenter la RECOM, juste au nord dans le même boîtier. **Phase brun, neutre bleu.**
- Deux paires 0,75 mm² sortent vers `boitier_ac`, entrant par sa **face est**, chaque paire phase-neutre maintenue par de la gaine thermo (surface de boucle nulle) : **SSR (jaune/bleu)** et **dimmer (violet/bleu)**.
- Du **SSR** repartent phase et neutre vers la **vanne solénoïde** — **phase jaune, neutre bleu**.
- Du **dimmer** repartent phase et neutre vers la **pompe** — **phase violet, neutre bleu**.

### 5 V

- **VCC 5 V et GND** sortent de la RECOM vers les **Wago 3 poles au nord de `boitier_ps`**.
- De là, deux fils **0,25 mm² rouge / noir** vont à l'**écran**, via un **bornier adaptateur USB-C**.
- Deux autres fils rouge / noir vont à `boitier_dc`, dans les deux Wago du **compartiment sud-ouest** : **3 poles à gauche = 5 V**, **2 poles à droite = GND**.
- De ces bornes partent (a) le 5 V / GND du **XIAO**, par le bornier soudé aux pastilles du Shield, et (b) le 5 V vers la **Wago 3 poles du compartiment nord-ouest**.
- La Wago nord-ouest distribue le 5 V au **SSR** (fil du câble Grove dont le VCC a été coupé à ras côté XIAO, dénudé et repris ici) et au **Digmesa** (fil rouge du câble JST SM).

### Le câble du Digmesa

Côté JST SM : **3 poles, 3 fils — rouge, noir, jaune**.

- **noir + jaune** sont sertis dans un connecteur **Grove** → port **R2**
- **rouge** part seul dans la **Wago du compartiment nord-ouest** → 5 V

Côté capteur, le câble est en **VH3.96** : rouge = VCC, noir = GND, jaune = signal.

---

## Capteurs

### Pression — Yufavor XDB401

Quatre fils : **rouge VCC**, **noir GND**, **vert SDA**, **blanc SCL**. Alimenté en **3,3 V** (l'emballage annonce 5 V ; l'émulation XDB401 est complète en 3,3 V, vérifié au banc). Pull-up **4,7 kΩ** sur SDA et SCL, non débrayables — ce sont elles qui tiennent tout le bus. Filetage **G1/8**, d'où le choix (plomberie machine en 1/8").

Gaine silicone avec une **fine feuille de blindage** : **pas de continuité feuille ↔ GND**. La relier à la masse **uniquement côté ESP32** ; côté sonde, coupée et isolée, pas de boucle.

Le connecteur **JST SM** est à l'extérieur du boîtier, à proximité immédiate : la sonde se débranche sans ouvrir.

### Débit — Digmesa FHKSC 932-9525-B (buse 1,00 mm)

| | 932-9525-B (retenu) | 932-9521-A (ancien) |
| --- | --- | --- |
| Buse | **1,00 mm** | 1,20 mm |
| Sens de montage | 0° | 0° |
| Impulsions | **2382 imp/L (0,42 g)** | 1925 imp/L (0,519 g) |
| Plage linéaire | **0,033 – 0,40 L/min** | 0,075 – 0,569 L/min |
| Pré-infusion 0,5 g/s | bas de plage | sous le linéaire |
| Perte de charge | ~0,48 bar vers 0,40 L/min | ~0,42 bar à 0,6 L/min |
| **Pression max** | **3 bar à 20 °C** | 3 bar à 20 °C |

Deux conséquences qui ne relèvent pas du câblage :

- **Le capteur va en amont de la pompe**, entre le réservoir et son entrée. La machine infuse à 9 bar et la pompe monte plus haut avant l'OPV : 3 bar de tenue interdisent le circuit haute pression.
- Le **1,00 mm** met une extraction de 36 g / 28 s (0,077 L/min) à 2,3 × son minimum, et 0,5 g/s au début du linéaire. Plafond 0,40 L/min : une chasse pompe ouverte peut saturer. Comparatif complet : `docs/debitmetres.md`.

---

## Câblage

| Domaine | Fil | Connectique |
| --- | --- | --- |
| 230 V | **Helutherm 145 0,75 mm²**, paires L/N sous gaine thermo | Wago 221 à leviers ; FASTON 6,3 × 0,8 mm côté machine |
| 5 V / signaux / CAN | **Helutherm 145 0,25 mm²** | Grove, JST SM (débrochable), JST XH / PH 2.0, borniers à vis 2,54 mm, Wago 221 |

Détail fil par fil, couleurs et cheminement : `docs/cablage.md`. Passage intérieur → façade : le trou **Ø 16 mm** de l'ancien bouton brew, avec le passe-câble imprimé.

## Firmware

- **`boitier_dc` (XIAO ESP32-S3)** — I2C (DimmerLink, XDB401), GPIO SSR, comptage d'impulsions du débitmètre, TWAI/CAN. Pas de radio. Il exécute ce que l'écran lui demande et remonte la télémétrie.
- **UI (Waveshare ESP32-S3)** — affichage et commandes, **algorithme d'infusion** (flow control, stop on weight), Wi-Fi, BLE vers l'Acaia Lunar.

Le mode DimmerLink retire tout besoin d'ISR zero-cross / PSM côté ESP32 : le Cortex du dimmer gère la détection de passage par zéro et le triac, le XIAO ne voit que de l'I2C. Sans **secteur** sur le dimmer, le module reste en `Calibrating...` et n'accepte pas les commandes.

Conception, protocole CAN et sécurité : `docs/firmware.md`. Ordre de réalisation : `docs/firmware-implementation.md`.

Le firmware n'est pas encore dans le dépôt. `sound_test/` (sketch Arduino Atom S3) est un reliquat.

## Disposition mécanique

- **`boitier_ps`** — intérieur, contre la face gauche vue de face, le long de la séparation avec le réservoir.
- **`boitier_dc` + `boitier_ac`** — intérieur, zone froide entre le module PID et le cadran manomètre ; côte à côte, couvercle commun.
- **UI** — façade, à l'emplacement des boutons. Le wedge cadre l'écran (vissé par l'arrière), la base accueille le wedge (`screen_assembly`).

Pas de perçage du châssis : aimants Ø8 × 3 mm à l'intérieur, trou brew existant vers l'extérieur.

### Pièces imprimées (`print/`)

Projet [nurb](https://pypi.org/project/nurb/) : lancer `nurb` depuis `print/`. Export préféré : **3MF** dans `print/build/`. Sur cette machine, pour tout script Python (y compris hors `print/`) : **`uv run`**, pas le `python` système — détail dans `print/README.md`.

| Pièce | Où | Rôle |
| --- | --- | --- |
| `boitier_ps` / `couvercle_ps` | intérieur, face ouest | Alim RECOM, Wago 230 V et 5 V |
| `boitier_dc` | intérieur, zone froide | XIAO + Grove Shield, CAN Pal, Wago 5 V |
| `boitier_ac` | accolé au DC | Dimmer, SSR |
| `couvercle_acdc` | — | Couvercle unique des deux boîtiers |
| `ensemble_boitiers` | — | Assemblage DC + AC |
| `screen_wedge` / `screen_base` | façade | Cadre de l'écran Waveshare (vis M2.5 à l'arrière) et son berceau |
| `passe_cable` / `ecrou_passe_cable` | façade | Traversée du trou Ø 16 mm ex-bouton brew |
| `canal` | intérieur | Guidage des fils |
| `base_pesage` / `plateau_pesage` | drip tray | Pesée par cellule — **en pause** |

### Wago

Côté machine : FASTON 6,3 × 0,8 mm isolées nylon. Côté mod : Wago 221 (412, 415, 423 selon le compartiment). Pas de piggyback. Vis M3, inserts laiton M2.5 pour les cartes. Aimants 8 × 3 mm. Wago sans contact avec le fond ni les parois.

## Matière d'impression

**Les boîtiers sont imprimés en PLA HT recuit** — après recuit, la tenue en température monte à **140 °C**, très au-dessus des 40–50 °C du compartiment technique et sans le fluage du PLA standard à partir de ~45 °C. La machine tourne 10–15 min, deux fois par jour ; le reste du temps tout est à température ambiante et hors tension.

Le recuit fait retirer les pièces : les cotes fit-critiques (puits d'aimant, logements Wago, inserts) se vérifient **après** recuit, pas sur la pièce sortie du plateau.

*Atelier : Bambu Lab A1 Mini.*

## Dépôt

```
coffeeflow/
  README.md          ← cette vue d'ensemble
  docs/              ← câblage, débitmètres, drip tray, maquettes Three.js ; datasheets/
  print/             ← impressions 3D (nurb)
  tests/             ← scripts MicroPython de banc
  sound_test/        ← sketch Arduino de test Atom S3 (reliquat)
  tts/               ← génération WAV (Gemini TTS) pour le sketch
```

Contraintes et cotes d'une pièce : `print/parts/<nom>.md` + `print/measurements.toml`.

## Banc

Multimètre : **M5Stack Atom Echo S3R** + **Unit VMeter** (Grove PORT.A, ±36 V DC, bornes vis). Script `tests/test_vmeter.py`. Ne pas mesurer au GPIO de l'ESP32 (3,3 V max, pas 5 V).

## Sketch de test (`sound_test/`)

Reliquat Atom S3 Voice (Echo), ESP32-S3, USB-CDC natif, `M5.Speaker`, bouton `M5.BtnA`. FQBN : `m5stack:esp32:m5stack_atoms3`.

```
ls /dev/cu.usbmodem*
arduino-cli compile --fqbn m5stack:esp32:m5stack_atoms3 sound_test
arduino-cli upload -p /dev/cu.usbmodem2201 --fqbn m5stack:esp32:m5stack_atoms3 sound_test
```

Câble USB-C **data**. Si le port n'apparaît pas : autre câble, pas de hub, reset latéral. Le suffixe `usbmodem*` change à chaque reconnexion.

## TTS (`tts/`)

```
cd tts
uv run generate_tts.py "Espresso ready" -o startup.wav
```

Clé `GOOGLE_API_KEY` dans `tts/.env` (non commité). Pour embarquer le WAV :

```
xxd -i tts/startup.wav | sed 's/tts_startup_wav/startup_wav/; s/unsigned char/const uint8_t/; s/unsigned int/const unsigned int/' > sound_test/startup_wav.h
```

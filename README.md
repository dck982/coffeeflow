# coffeeflow

Modification d'une **Profitec Go** pour du *flow profiling* et du *brew by weight* : l'infusion se lance depuis un contrôleur ESP32, la pression se module, et elle s'arrête toute seule au poids cible.

## Machine d'origine

- Vanne solénoïde + pompe vibratoire 35 W + boiler
- Petit boîtier PID : consigne / mesure de température du boiler, chrono
- Vanne OPV pour limiter la pression
- Plomberie en **1/8"**
- Connexions électriques internes : cosses à languette **FASTON 6,3 × 0,8 mm** (pas de piggyback : la dérivation se fait dans un boîtier imprimé)

## Ce que la modification ajoute

Deux dérivations 230 V, sans remplacer le câblage d'origine :

1. **Circuit brew** (vanne + pompe) — un ESP32 peut lancer une infusion en parallèle du bouton machine.
2. **Circuit pompe** — un dimmer AC RobotDyn 4 A fait varier la pression. En manuel, la pompe continue de fonctionner via un relais NC.

Alimentation du système de mod : uniquement quand la machine est allumée (phase prise sur le relais boiler du PID).

Trois capteurs en feedback :

- **Poids** (cellule de contrainte via Unit Weight-I2C)
- **Pression** sur la plomberie, en amont de la vanne
- **Débit** en amont de la pompe, côté basse pression (le capteur ne tient que 3 bar)

L'Atom Control coupe l'infusion au poids cible, d'après les valeurs que l'Atom Sensor envoie en UART.

## Architecture électronique

Trois nœuds ESP32 en chaîne UART, deux dedans/dessus la machine :

```
[Atom Sensor]  --LiYCY (UART + 5 V)-->  [Atom Control]  --UART-->  [Waveshare LCD]
 dans la machine                        sur la machine             sur la machine
 débit, poids, pression                 230 V, dimmer              interface
```

- **Atom Sensor** lit les trois capteurs et pousse les valeurs en UART.
- **Atom Control** pilote le 230 V (ACSSR, relais NC, dimmer) et relaie tout à l'écran.
- **Écran** affiche et renvoie la commande **départ**.

Wifi sur l'écran (remontée backend) : hors scope.

### BOM — Atom Sensor (dans la machine)

| Rôle | Matériel | Liaison | Fils |
| --- | --- | --- | --- |
| MCU | M5Stack Atom Echo S3R | USB-C (bornier 5 V/GND) + UART header | LiYCY : 5 V/GND → HNP-1205, TX/RX → Atom Control |
| Éclateur I2C | M5Stack Unit Hub | Grove sur **Port.A** | GND, 5 V, SDA, SCL — 3 ports en parallèle |
| Poids | M5Stack Unit Weight-I2C + cellule | I2C Grove sur le Hub — alim 5 V, pull-up 4,7 kΩ internes | Grove 4 fils ; cellule en HT3.96-4P |
| Pression | XDB401 3,3 V | I2C sur le **même bus**, Grove **sans fil 5 V** | 3,3 V header + SCL/SDA/GND |
| Débit | Digmesa FHKSC PVDF **932-9521-A** | collecteur ouvert NPN, sur **header** | 5 V, G38, GND (via Wagos) |

Un seul bus I2C, sur Port.A. Le Hub le duplique : le Weight-I2C s'y branche en 5 V comme n'importe quelle unit ; le XDB401 s'y greffe en 3,3 V par un câble Grove dont le fil rouge n'est pas connecté. Les tirages SDA/SCL sont ceux, internes, du Weight-I2C — rien à câbler côté pression. Débrancher le Weight-I2C laisse le bus sans pull-up.

Le débitmètre est en collecteur ouvert : il tire la ligne à la masse mais ne la monte jamais. R3 = **1 kΩ** vers le 3,3 V et C1 = **100 nF** vers la masse (passe-bas ≈ 1,6 kHz). `G38` en `INPUT`, pull-up interne éteinte. Le capteur est alimenté en 5 V ; c'est le tirage qui fixe le niveau haut à 3,3 V, donc l'entrée GPIO est en sécurité. Détail et schéma : `docs/atom_sensor.html`.

#### Débitmètre 932-9521-A

| Paramètre | Valeur |
| --- | --- |
| Buse | 1,20 mm |
| Sens de montage | 0° |
| Impulsions | 1925 imp/L, soit 0,519 g/impulsion |
| Plage linéaire | 0,075 – 0,569 L/min |
| Perte de charge | ~0,42 bar à 0,6 L/min |
| **Pression max** | **3 bar à 20 °C** |

Deux conséquences qui ne relèvent pas du câblage :

- **Le capteur va en amont de la pompe**, entre le réservoir et son entrée. La machine infuse à 9 bar et la pompe monte plus haut avant l'OPV : 3 bar de tenue interdisent le circuit haute pression. C'est la position habituelle d'un débitmètre sur une espresso, mais c'est une contrainte de plomberie qui ne se rattrape pas au montage.
- **Une extraction se déroule au plancher de la plage linéaire.** 36 g en 28 s font 0,077 L/min contre une limite basse à 0,075. En cumul c'est exploitable — 69 impulsions par tasse, 0,52 g de résolution, 1,4 % — mais le débit instantané à 2,5 Hz est grossier pour du profilage : il faudra lisser sur plusieurs secondes ou mesurer les intervalles entre fronts. En pré-infusion (~1 Hz) on passe sous la plage, la valeur devient indicative.

Le suffixe `-A` se distingue du `-B`, plus courant, par l'angle de sortie des raccords — sans effet sur le câblage.

### BOM — Atom Control (sur la machine)

| Rôle | Matériel | Liaison | Fils |
| --- | --- | --- | --- |
| MCU | M5Stack Atom Echo S3R | USB-C depuis l'alim 5 V | + GND, RX, TX sur header vers l'écran |
| Extension GPIO | Unit PbHub | I2C sur **Port.A** | GND, 5 V, SDA, SCL — donne six Grove GPIO + 5 V |
| Circuit brew (vanne + pompe) | Unit ACSSR | GPIO sur PbHub | GND, 5 V, REL |
| Bypass dimmer en mode manuel | Unit Relay **NC** | GPIO sur PbHub | GND, 5 V, REL |
| Variation de pression | RobotDyn AC Dimmer 4 A | **header** Atom | GND, 5 V, ZC (zero-cross), PSM (modulation) |
| Alimentation | HNP-MOD1205, 5 V / 2 A | 230 V en entrée | — |
| Protection | Fuse box, fusible **315 mA** | sur la phase, en entrée de l'alim | — |

Le PbHub existe pour une raison simple : l'Atom n'a pas assez de GPIO pour l'ACSSR, le relais NC *et* le dimmer, dont les deux lignes temps réel (ZC, PSM) doivent rester en direct sur le header.

### BOM — Écran (sur la machine)

Waveshare LCD tactile 4,3" avec ESP32-S3-VROOM : alimenté en USB-C, RX/TX de son port UART vers le header de l'Atom Control.

### Câblage

| Domaine | Fil | Connectique |
| --- | --- | --- |
| 230 V | Helutherm 145, **0,75 mm²**, Ø 2,2 mm | Wago 221 à leviers ; FASTON 6,3 × 0,8 mm côté machine |
| 5 V / signaux | **0,25 mm²** | Grove, JST, header |
| Sensor → Control | **LiYCY 4×0,25 mm²**, paires droites, blindé, 30–40 cm | 5 V/GND → HNP-1205 et bornier USB-C côté Sensor ; TX/RX → GPIO des deux Atom |

Le facteur limitant du LiYCY est son **rayon de courbure**, pas sa longueur : il doit remonter d'un canal et entrer dans un boîtier fermé sans forcer sur le bornier USB-C.

Les deux canaux séparent physiquement le 230 V du 5 V (un de chaque côté du réservoir) pour éviter que le dimmer, qui découpe la sinusoïde, ne pollue les signaux capteurs.

## Firmware

- **Atom Sensor** : Rust `no_std` — I2C (Weight-I2C, XDB401), comptage d'impulsions du débitmètre, émission UART
- **Atom Control** : Rust `no_std` — dimmer et UART en temps réel, GPIO via PbHub
- **Écran tactile** : Rust + lib graphique, UART vers l'Atom Control

Le dépôt contient aujourd'hui un test Arduino (`sound_test/`) sur Atom S3 Voice. Le firmware Rust n'y est pas encore.

## Disposition mécanique

**Dans la machine** (châssis ~40–50 °C, aimants, pas de perçage) : le boîtier de dérivation 230 V, le boîtier Atom Sensor, et deux canaux le long du réservoir d'eau.

**Sur la machine**, posé sur le plateau chauffant : `screen_base` (modules 230 V, alim 5 V, fusible, Atom Control) surélevée de 5 mm par une grille, et `screen_wedge` (Atom Voice + écran Waveshare) emboîté dessus. L'ensemble est `screen_assembly`.

Traversée intérieur → extérieur : **quatre fils 230 V** dans un canal, **le LiYCY Sensor → Control** dans l'autre.

### Les six objets imprimés (`print/`)

Projet [nurb](https://pypi.org/project/nurb/) : lancer `nurb` depuis `print/`. Export préféré : **3MF** dans `print/build/` (Studio peut avertir sur la spec 1.41 ; l’objet s’importe). Sur cette machine, pour tout script Python (y compris hors `print/`) : **`uv run`**, pas le `python` système — détail dans `print/README.md`.

| # | Pièce | Où | Matière | Rôle | Statut |
| --- | --- | --- | --- | --- | --- |
| 1 | `boitier_2w` / `boitier_4w` | intérieur, ~10 cm du boiler | PETG | Dérivation 230 V. FASTON nylon côté machine, Wago 221 côté mod. Évite les cosses piggyback. | existe |
| 2 | `canal` (× 2) | intérieur, ~20 cm du boiler | PETG | Guidage des fils le long du réservoir. Un pour le 230 V, un pour le 5 V — séparation pour les interférences. | existe |
| 3 | Boîtier **Atom Sensor** | intérieur | PETG | Atom Echo S3R, Unit Hub, Weight-I2C, XDB401, débitmètre et leur câblage. | à faire |
| 4 | `screen_wedge` | dessus, le plus loin de la machine | **PLA** | Cadre de l'écran Waveshare. Sert aussi de couvercle à `screen_base`. | en validation |
| 5 | `screen_base` | dessus, sur le plateau | PETG | Atom Control, PbHub, ACSSR, relais NC, RobotDyn, alim 5 V, fuse box, câblage. | à dimensionner |
| 6 | Grille d'entretoise 5 mm | dessus, contact plateau | PETG | Surélève `screen_base` pour l'isoler thermiquement et faire passer des fils dessous. | à faire |

Le volume de `screen_base` n'est pas encore fixé : il découle du placement réel des modules à l'intérieur.

### Quatre Wago 230 V

1. **Phase machine allumée** — sur le relais boiler du PID (alimentée dès le bouton on/off)
2. **Neutre** — pris sur la pompe
3. **Phase brew** — sortie de la vanne (infusion parallèle)
4. **Phase pompe** — entrée de la pompe (relais NC *ou* dimmer, selon manuel / contrôlé)

Vis M3. Aimants 8 × 3 mm. Wago sans contact avec le fond ni les parois.

## Matières d'impression

Le châssis mesure **40–50 °C** en fonctionnement, le plateau supérieur un peu plus (une tasse posée dessus tiédit). La machine tourne 10–15 min, deux fois par jour ; le reste du temps tout est à température ambiante et hors tension.

| | PLA | PETG |
| --- | --- | --- |
| Transition vitreuse | 55–60 °C | 78–85 °C |
| Limite pratique sans charge | ~45 °C | ~65 °C |
| Limite pratique sous charge continue (fluage) | ~40 °C | ~60 °C |

**PETG partout, sauf `screen_wedge`.** Le risque du PLA n'est pas la fonte mais le fluage : une déformation lente sous charge permanente à partir de ~45 °C. Les pièces 1, 2, 3, 5 et 6 sont soit dans l'air chaud enfermé de la machine, soit en contact avec le plateau, soit — pour la grille — les deux plus une charge en compression permanente. Le boîtier de dérivation a en plus un argument de sécurité : il tient des bornes secteur.

**`screen_wedge` est en PLA**, choisi pour le rendu : l'*ironing* de la face supérieure donne un fini que le PETG ne sait pas produire (il ne réagit pas bien à l'ironing). Le cycle thermique le permet — 10–15 min de chauffe deux fois par jour, et c'est la pièce la plus éloignée de la source, en haut de l'empilement mais séparée du plateau par la grille et par `screen_base`. Le PLA n'a pas le temps d'atteindre l'équilibre thermique sur un cycle aussi court.

Un point à surveiller malgré tout : `screen_base` contient l'alimentation, qui chauffe toute seule, et la chaleur monte vers le wedge. Si le cadre finit par gondoler après quelques mois, la réponse est PETG et l'abandon de l'ironing.

*Atelier : Bambu Lab A1 Mini. PETG HF Black 33102 pour le PETG.*

## Dépôt

```
coffeeflow/
  README.md          ← cette vue d'ensemble
  docs/              ← schémas de câblage (HTML, sans JS)
  print/             ← impressions 3D (nurb)
  sound_test/        ← sketch Arduino de test Atom S3
  tts/               ← génération WAV (Gemini TTS) pour le sketch
```

Contraintes et cotes d'une pièce : `print/parts/<nom>.md` + `print/measurements.toml`.

## Sketch de test (`sound_test/`)

Atom S3 Voice (Echo), ESP32-S3, USB-CDC natif, `M5.Speaker`, bouton `M5.BtnA`. FQBN : `m5stack:esp32:m5stack_atoms3`.

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

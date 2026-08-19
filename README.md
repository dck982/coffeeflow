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

- **Température du groupe** (sonde collée au groupe, pas l'eau du boiler)
- **Pression** sur la plomberie, en amont de la vanne
- **Débit** en ligne

Une **balance Acaia en BLE** (balance = master, Atom = client) donne le poids dans la tasse. L'Atom coupe l'infusion au poids cible.

## Architecture électronique

Trois nœuds ESP32 en chaîne UART, deux dedans/dessus la machine :

```
[Atom Sensor]  --UART + 5V (USB-C)-->  [Atom Control]  --UART-->  [Waveshare LCD]
 dans la machine                        sur la machine             sur la machine
 débit, température, pression           230 V, dimmer, BLE         interface
```

- **Atom Sensor** lit les trois capteurs et pousse les valeurs en UART.
- **Atom Control** pilote le 230 V (ACSSR, relais NC, dimmer) et relaie tout à l'écran.
- **Écran** affiche et renvoie la commande **départ**.

Le BLE Acaia est porté par l'Atom Control, puisque c'est lui qui coupe l'infusion au poids cible — *à confirmer au moment du firmware*.

Wifi sur l'écran (remontée backend) : hors scope.

### BOM — Atom Sensor (dans la machine)

| Rôle | Matériel | Liaison | Fils |
| --- | --- | --- | --- |
| MCU | M5Stack Atom Echo S3R | USB-C | GND, 5 V (depuis l'alim du haut), TX, RX (vers Atom Control) |
| Température groupe | M5Stack KISOMeter | I2C sur **Port.A** — alim 5 V, signal 3,3 V | GND, 5 V, SDA, SCL |
| Pression | XDB401 3,3 V | I2C sur **header** — alim et signal 3,3 V | GND, 3,3 V, SDA, SCL |
| Débit | Digmesa FHKSC PVDF *(à confirmer à réception)* | collecteur ouvert NPN, sur **header** | GND, 5 V, Signal |

Deux bus I2C à des tensions différentes : le KISOMeter sur Port.A (alim 5 V), le XDB401 sur le header en 3,3 V pur.

Le débitmètre est en collecteur ouvert : il tire la ligne à la masse mais ne la monte jamais. En première version, la ligne est tenue par la **résistance de pull-up interne de l'ESP32-S3** (`INPUT_PULLUP` sur le GPIO), pas de composant externe. Elle est faible (~45 kΩ typ.), donc le front montant est mou ; sur les quelques dizaines de cm de fil du boîtier Sensor et aux fréquences d'impulsion du FHKSC, ça passe. Si les fronts sont sales ou qu'on voit des impulsions doublées, le correctif est une pull-up externe de 4,7–10 kΩ vers 3,3 V.

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
| Sensor → Control | probablement un câble **USB-C data blindé** (alim + UART), 30–40 cm | USB-C |

Le facteur limitant du câble USB-C est son **rayon de courbure**, pas sa longueur : il doit remonter d'un canal et entrer dans un boîtier fermé sans forcer sur le connecteur.

Les deux canaux séparent physiquement le 230 V du 5 V (un de chaque côté du réservoir) pour éviter que le dimmer, qui découpe la sinusoïde, ne pollue les signaux capteurs.

## Firmware

- **Atom Sensor** : Rust `no_std` — I2C (KISOMeter, XDB401), comptage d'impulsions du débitmètre, émission UART
- **Atom Control** : Rust `no_std` — dimmer et UART en temps réel, GPIO via PbHub, client BLE Acaia
- **Écran tactile** : Rust + lib graphique, UART vers l'Atom Control

Le dépôt contient aujourd'hui un test Arduino (`sound_test/`) sur Atom S3 Voice. Le firmware Rust n'y est pas encore.

## Disposition mécanique

**Dans la machine** (châssis ~40–50 °C, aimants, pas de perçage) : le boîtier de dérivation 230 V, le boîtier Atom Sensor, et deux canaux le long du réservoir d'eau.

**Sur la machine**, posé sur le plateau chauffant : `screen_base` (modules 230 V, alim 5 V, fusible, Atom Control) surélevée de 5 mm par une grille, et `screen_wedge` (Atom Voice + écran Waveshare) emboîté dessus. L'ensemble est `screen_assembly`.

Traversée intérieur → extérieur : **quatre fils 230 V** dans un canal, **le câble USB-C Sensor → Control** dans l'autre.

### Les six objets imprimés (`print/`)

Projet [nurb](https://pypi.org/project/nurb/) : lancer `nurb` depuis `print/`. Export préféré : **3MF** dans `print/build/` (Studio peut avertir sur la spec 1.41 ; l’objet s’importe). Sur cette machine, pour tout script Python (y compris hors `print/`) : **`uv run`**, pas le `python` système — détail dans `print/README.md`.

| # | Pièce | Où | Matière | Rôle | Statut |
| --- | --- | --- | --- | --- | --- |
| 1 | `boitier_2w` / `boitier_4w` | intérieur, ~10 cm du boiler | PETG | Dérivation 230 V. FASTON nylon côté machine, Wago 221 côté mod. Évite les cosses piggyback. | existe |
| 2 | `canal` (× 2) | intérieur, ~20 cm du boiler | PETG | Guidage des fils le long du réservoir. Un pour le 230 V, un pour le 5 V — séparation pour les interférences. | existe |
| 3 | Boîtier **Atom Sensor** | intérieur | PETG | Atom Echo S3R, KISOMeter, XDB401, débitmètre et leur câblage. | à faire |
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

# coffeeflow

Modification d'une **Profitec Go** pour du *flow profiling* et du *brew by weight* : l'infusion se lance depuis un écran tactile, le débit et la pression se pilotent par la vitesse d'une **pompe à engrenage**, et l'infusion s'arrête toute seule au poids cible.

## Machine d'origine

- Vanne solénoïde trois voies + pompe vibratoire OLAB Silent Green 35 W (**remplacée** par une pompe à engrenage, voir plus bas) + boiler
- Petit boîtier PID : consigne / mesure de température du boiler, chrono
- Vanne OPV pour limiter la pression
- Plomberie en **1/8"**
- Connexions électriques internes : cosses à languette **FASTON 6,3 × 0,8 mm** (pas de piggyback : la dérivation se fait dans un boîtier imprimé)
- Deux compartiments : **technique** (hydraulique, boiler, groupe, électricité) et **réservoir d'eau**
- Bouton **brew** en façade (Ø 16 mm) : démarrait vanne + pompe. **Supprimé** — le trou sert de passe-câble vers l'écran
- LED de façade : ses cosses FASTON ont été réutilisées, la LED est réalimentée depuis le boîtier de l'alimentation

## Ce que la modification ajoute

La modification pilote la pompe, la vanne et la résistance de chaudière :

1. **Pompe** — la pompe vibratoire est remplacée par une **pompe à engrenage SOVOFLO MPB003** (moteur BLDC 24 V, 40 W, 0,1 ml par tour), installée le 12 novembre 2026. Volumétrique et sans pulsation, son débit suit la vitesse, commandée par une consigne 0–5 V et relue sur son tachymètre. Détail : [pompe-engrenage.md](docs/pompe-engrenage.md).
2. **Vanne solénoïde** — un triac sur la carte `combined` ouvre et ferme le circuit d'infusion.
3. **Chaudière** — le SSR Keysolu/Maxwell fourni avec la machine commande la résistance de chauffe ; `combined` lui envoie sa commande 5 V.

Plus de bouton brew, plus de relais NC de « mode manuel » : la commande passe par l'écran. Le système est alimenté dès que l'**interrupteur principal** de la machine est enfoncé.

Deux retours capteurs, sur la carte `combined` :

- **Pression** sur la plomberie, en amont de la vanne (XDB401 analogique, 0,4–2,4 V)
- **Débit** en amont de la pompe, côté basse pression (le capteur ne tient que 3 bar)

La pompe donne en plus sa **vitesse** par son tachymètre, donc le volume déplacé, au glissement près.

La **température de la chaudière** vient de la sonde NTC vissée en G1/8 dans la chaudière. Son câble rejoint le boîtier de l'écran, proche de la sonde, où un ADS1115 la mesure via un pont alimenté par le **3V3 et le GND du port Sensor AD du Waveshare**. Voir [la calibration NTC/ADS1115](docs/ntc_ads1115_calibration.md).

Le **poids** vient d'une balance **Acaia Lunar** en **BLE**, lue par l'écran. La pesée du drip tray par cellule de charge est **abandonnée pour l'instant** (peser quelques grammes sur un plateau d'un kilo) — notes dans `docs/driptray.md` ; les pièces imprimées ont été retirées (commit 1aa13ed).

## Architecture électronique

**Une alimentation 24 V, une carte maison, deux nœuds ESP32-S3.** Le 230 V ne sort pas de la machine.

```
             interrupteur principal (230 V L/N)
                          │
                  [boitier_ps_24v]   intérieur, face ouest, le long du réservoir
               Wago L/N · alim 24 V 96 W
                  │            │    └── L/N → LED de façade
               L/N (J2)     24 V (J7) ──────────────→ pompe à engrenage MPB003
                  │            │                      (embouts doubles dans J7)
             [boitier_ac + boitier_dc]
              carte combined : XIAO ESP32-S3 · buck 24 → 5 V · CAN
                  │
                  ├── J3  triac ────────→ vanne solénoïde (230 V)
                  ├── J8  5 V ──────────→ SSR chaudière ─→ résistance
                  ├── J9  2 optocoupleurs ⇄ consigne et tachymètre de la pompe
                  ├── J10 ADS1115 ←──── capteur de pression XDB401 (0,4–2,4 V)
                  ├── J6  GPIO ←──────── débitmètre Digmesa
                  └── J1  5 V + CAN ───→ [screen_*] Waveshare 4,3" ESP32-S3
                                          UI · Wi-Fi · BLE
                                          └── I2C ← ADS1115 ← NTC chaudière
```

| Nœud | Alias | Où | Rôle |
| --- | --- | --- | --- |
| **Waveshare 4,3" LCD Touch** (ESP32-S3-WROOM) | *screen*, *écran*, *waveshare* | façade, dans `screen_base` + `screen_wedge` | Interface utilisateur, algorithme d'infusion, **Wi-Fi et BLE** (Acaia Lunar) ; raccordement de la NTC chaudière via ADS1115 |
| **Seeed XIAO ESP32-S3** sur la carte **`combined`** | *xiao*, *combined*, *module capteurs*, *contrôleur* | intérieur, `boitier_ac` + `boitier_dc`, zone froide entre le module PID et le cadran manomètre | Capteurs (XDB401, Digmesa, tachymètre) et actionneurs (consigne de la pompe, vanne, commande du SSR chaudière) |

Les deux nœuds parlent **CAN**. L'écran est le seul à utiliser la radio.

Maquette Three.js de la machine (ouvrir dans le navigateur) : `docs/profitec_go.html`.

### Pourquoi un XIAO à l'intérieur

L'Atom Echo S3R de l'ancienne architecture est spécifié à **40 °C**. Le XIAO ESP32-S3 tient **85 °C**. Le compartiment technique monte à 45–50 °C : la marge de l'Atom est nulle, celle du XIAO est utilisable. L'écran Waveshare reste en façade, hors de cette enceinte.

---

## Boîtiers

| Boîtier | Pièces `print/parts/` | Où | Contenu |
| --- | --- | --- | --- |
| **PS 24 V** | `boitier_ps_24v`, `couvercle_ps_24v` | intérieur, contre la face **gauche** vue de face, le long de la séparation avec le réservoir | Alimentation 24 V, deux Wago 221-415 (L, N) ; le câble de l'écran le traverse |
| **AC + DC** | `boitier_ac`, `boitier_dc`, `couvercle_acdc` | intérieur, **zone froide** entre le module PID et le cadran manomètre | Carte `combined` : bande secteur dans `boitier_ac`, basse tension et aile 24 V dans `boitier_dc` |
| **UI** | `screen_base`, `screen_wedge` | façade de la machine | Écran Waveshare 4,3", ADS1115 et raccordement NTC ; emplacement AMS1117 inutilisé |

Les boîtiers **AC et DC sont côte à côte et reliés par leur couvercle**, qui est commun aux deux (`couvercle_acdc`, assemblage `ensemble_boitiers`). `boitier_pid` ne loge plus rien : c'est un pont qui tient le `cache_pid` contre l'ouverture de la machine.

La pompe est fixée par son équerre d'origine dans deux des trous de 6 mm de la plaque du fond de la machine, à 40 mm d'entraxe.

---

## Composants

| Rôle | Matériel retenu | Fiche |
| --- | --- | --- |
| UI / algo / radio | **Waveshare ESP32-S3-Touch-LCD-4.3** — 800 × 480, TJA1051T/3 CAN intégré, alim USB-C 5 V | `docs/datasheets/waveshare-4-3-lcd-touch.md` |
| Carte interne | **`combined`** (maison, JLCPCB) : **Seeed Studio XIAO ESP32-S3** enfiché, buck 24 → 5 V, TJA1051T/3, triac de vanne, interface optocouplée de la pompe, ADS1115 de pression | `pcb/combined/combined_design.md`, `docs/datasheets/xiao.pdf` |
| Pompe | **SOVOFLO MPB003BAAA** — engrenage, BLDC 24 V 40 W, 0,1 ml/tour, 12 bar au plus, raccords G1/8 | [pompe-engrenage.md](docs/pompe-engrenage.md) |
| Alimentation | Module ouvert **24 V, 4 A** (Aideepen, 96 W), fusible T2A interne | [pompe-engrenage.md](docs/pompe-engrenage.md#alimentation), [boitier-ps-24v.md](docs/boitier-ps-24v.md) |
| SSR chaudière | **Keysolu/Maxwell KS53 D-24Z20N-LQ**, fourni avec la machine ; en série entre la phase de l'interrupteur principal et la résistance, suivie de la protection thermique puis du neutre ; commande DC +/− depuis `combined` J8, FASTON mâles 4,8 mm | [Câblage](docs/cablage.md#ssr-de-chaudière-fourni-avec-la-machine) |
| Température chaudière | Sonde **NTC G1/8**, pont sur le port Sensor AD et breakout **ADS1115 16 bits** sur l'I2C de l'écran | [Calibration NTC/ADS1115](docs/ntc_ads1115_calibration.md) |
| Pression | **XDB401** analogique, 3,3 V, sortie 0,4–2,4 V, 0–12 bar, filetage **G1/8** | `docs/xdb401_swap.md` |
| Débit | **Digmesa FHKSC 932-9525-B**, buse **1,00 mm** | `docs/datasheets/flowmeter-digmesa.pdf`, `docs/debitmetres.md` |
| Poids | **Acaia Lunar**, BLE, lue par l'écran | — |
| Câble | **Helutherm 145** — 0,75 mm² en 230 V et en 24 V, 0,5 mm² pour le Digmesa, 0,25 mm² pour le 5 V et le CAN de l'écran ; puissance chaudière en 1 mm² | `docs/datasheets/helutherm145.pdf` |

Actionneur d'origine piloté : vanne solénoïde **OLAB 08252L50-A14-1A-G** (bobine 08000BH-J5IV, 15 VA, 220/230 V 50/60 Hz, orifice Ø 1,4 mm, joint FKM, laiton CW510L, NSF).

---

## La carte `combined`

Une seule carte réunit la partie 230 V (vanne) et la basse tension (XIAO, CAN, capteurs, commande de la pompe et du SSR chaudière). Elle reçoit le 24 V de l'alimentation et fait son propre 5 V, qu'elle renvoie à l'écran avec le CAN.

| Connecteur | Type | Vers |
| --- | --- | --- |
| **J1** | XH 4 | écran : GND, +5V, CAN_L, CAN_H |
| **J2** | HT3.96 2 | entrée 230 V, L et N |
| **J3** | HT3.96 2 | vanne solénoïde |
| **J6** | HT3.96 3 | Digmesa |
| **J7** | HT3.96 2 | entrée 24 V ; la pompe y est alimentée par des embouts doubles |
| **J8** | HT3.96 2 | commande du SSR chaudière |
| **J9** | XH 4 | signaux de la pompe : référence, consigne, tachymètre, 5 V de la pompe |
| **J10** | XH 3 | XDB401 |

J4 et J5, les prises I2C Grove de la carte `sensors`, n'existent plus. Brochage, schéma et choix des composants : `pcb/combined/combined_design.md`. Détail fil par fil : [cablage.md](docs/cablage.md).

**Pompe.** La consigne passe par un optocoupleur et un filtre RC (PWM à 1 kHz), le tachymètre revient par un second optocoupleur : les fils de signal de la pompe ne touchent jamais une masse ni une alimentation de la carte. Sans commande (démarrage, reset, XIAO absent), la consigne retombe à 0 et la pompe reste à l'arrêt.

**Bus CAN.** Paire torsadée de J1 jusqu'à l'écran, par le trou de l'ancien bouton brew. **Terminaisons 120 Ω aux deux bouts** : soudée sur `combined`, par jumper sur le Waveshare.

---

## Alimentation

L'**alimentation 24 V, 96 W** vit dans `boitier_ps_24v` et alimente **tout** : la pompe directement, et toute la basse tension par le buck de `combined`.

### 230 V

- **Phase et neutre** viennent de deux **cosses FASTON isolées** prises sur l'**interrupteur principal** de la machine : tant qu'il n'est pas enfoncé (clic mécanique), rien n'est sous tension.
- Les deux fils (**Helutherm 145 0,75 mm²**) vont dans les deux **Wago 221-415** de `boitier_ps_24v`, une pour L, une pour N.
- De ces Wago repartent trois paires 0,75 mm² : la **LED de façade**, l'**alimentation 24 V** (JST VH, position centrale vide) et **`combined` J2** (HT3.96).
- De **J3** repartent phase commutée et neutre vers la **vanne solénoïde** — **bleu / jaune**, FASTON 6,3 mm côté vanne.

### 24 V

- Deux fils **0,75 mm² rouge / noir** vont du bornier de l'alimentation à **`combined` J7**.
- Dans le boîtier HT3.96 de J7, chaque fil est serti avec le fil de même polarité de la pompe dans un **embout double 2 × 0,75 mm²** : la pompe est alimentée par J7, sans que son courant traverse les pistes de la carte.
- Le buck de `combined` fait le **5 V** du XIAO, du CAN, du Digmesa, de la commande du SSR chaudière et de l'**écran**, par J1 : paire 5 V / GND le long de la paire CAN, puis deux Wago 221-412 et un câble USB-C côté écran.
- Le pont NTC du boîtier de l'écran utilise le **3V3 / GND du port Sensor AD du Waveshare**.

---

## Capteurs

### Température chaudière — NTC G1/8

La sonde rejoint directement le boîtier de l'écran, voisin de la chaudière. Le **3V3 du port Sensor AD du Waveshare** rejoint la Wago NTC2, la NTC et l'entrée **A0** de l'ADS1115 ; le retour NTC rejoint la Wago NTC1, **A1** et une résistance de **4,7 kΩ** (4676 Ω étalonnés) vers le **GND du même port Sensor AD**. L'ADS1115 16 bits est sur le port I2C du Waveshare. A0 permet de mesurer la tension réelle d'alimentation du pont. Schéma et paramètres de conversion : [calibration NTC/ADS1115](docs/ntc_ads1115_calibration.md).

### Pression — XDB401 analogique

Trois fils : **rouge 3,3 V**, **noir GND**, **jaune sortie 0,4–2,4 V**, sertis en JST XH sur **J10** de `combined`. Sortie non ratiométrique, 6 bar/V, lue par l'ADS1115 de la carte ; l'écran reçoit la pression par le CAN. Filetage **G1/8**, d'où le choix (plomberie machine en 1/8"). Gaine silicone avec une fine feuille de blindage, sans continuité avec le GND : elle reste flottante. Passage de la sonde I2C à la sonde analogique : `docs/xdb401_swap.md`.

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
- Le **1,00 mm** met une extraction de 36 g / 28 s (0,077 L/min) à 2,3 × son minimum, et 0,5 g/s au début du linéaire. Plafond 0,40 L/min : la pompe le dépasse à basse pression au-delà de 4 000 tr/min. Le débit se lit alors sur le tachymètre ; le Digmesa sert surtout à mesurer le glissement de la pompe. Comparatif complet : `docs/debitmetres.md`.

---

## Câblage

| Domaine | Fil | Connectique |
| --- | --- | --- |
| 230 V — arrivée, LED, alimentation, `combined`, vanne | **Helutherm 145 0,75 mm²**, paires L/N sous gaine thermo | Wago 221-415 ; FASTON 6,3 × 0,8 mm côté machine ; HT3.96 côté `combined` |
| 230 V — puissance chaudière | **1 mm²** | SSR chaudière fourni avec la machine, FASTON mâles 4,8 mm |
| 24 V et commande du SSR chaudière | **Helutherm 145 0,75 mm²** | HT3.96 à embouts ; bornier de l'alimentation ; cosses 4,8 mm côté SSR |
| Digmesa | **Helutherm 145 0,5 mm²** | HT3.96 côté `combined`, VH3.96 côté capteur |
| 5 V / CAN de l'écran | **Helutherm 145 0,25 mm²** | JST XH, JST SM, JST PH 2.0, Wago 221-412, USB-C |
| Pompe (signaux), XDB401 | câbles intégrés | JST XH serti sur les fils d'origine |

Détail fil par fil, couleurs et cheminement : `docs/cablage.md`. Passage intérieur → façade : le trou **Ø 16 mm** de l'ancien bouton brew, avec le passe-câble imprimé.

## Firmware

- **`combined` (XIAO ESP32-S3)** — consigne PWM et tachymètre de la pompe, commande de la vanne et du SSR chaudière, ADS1115 de pression, comptage d'impulsions du débitmètre, TWAI/CAN. Le crate garde le nom `sensors`, la carte qu'il pilotait avant `combined`.
- **UI (Waveshare ESP32-S3)** — affichage et commandes, **algorithme d'infusion** (flow control, stop on weight), Wi-Fi, BLE vers l'Acaia Lunar. L'ADS1115 sur son I2C mesure la température chaudière par une NTC.

**Langage : Rust** (`no_std`, esp-hal, embassy) sur les deux cartes, sous `firmware/rust/` ; l'interface de l'écran est en **Slint**. Le firmware C++/ESP-IDF d'origine est archivé dans `archive/firmware/`, toujours compilable : il reste la référence de comportement sur le bus CAN et en HTTP. L'outil Mac `coffeetool` (Rust) remplace les scripts Python, archivés eux aussi.

**État du firmware (2026-11-12) :** écran et `combined` en **0.5.0**, installés sur la machine avec la pompe à engrenage. Démarrage, CAN et montée en température vérifiés ; la première purge attend le séchage du Loxeal des raccords de la pompe. Voir [le contrat firmware](docs/firmware.md) et [la calibration NTC](docs/ntc_ads1115_calibration.md).

Conception, protocole CAN et sécurité : `docs/firmware.md`. Build, flash et banc : `docs/firmware-build.md`. Style et structure de l'interface tactile : `docs/ui.md`, maquette à l'échelle `docs/ui-mockup.html`.

## Disposition mécanique

- **`boitier_ps_24v`** — intérieur, contre la face gauche vue de face, le long de la séparation avec le réservoir.
- **`boitier_dc` + `boitier_ac`** — intérieur, zone froide entre le module PID et le cadran manomètre ; côte à côte, couvercle commun ; ils logent la carte `combined`.
- **Pompe** — équerre d'origine vissée dans deux trous de 6 mm de la plaque du fond, à 40 mm d'entraxe.
- **UI** — façade, à l'emplacement des boutons. Le wedge cadre l'écran (vissé par l'arrière), la base accueille le wedge (`screen_assembly`).

Pas de perçage du châssis : aimants Ø8 × 3 mm à l'intérieur, trou brew existant vers l'extérieur, trous existants de la plaque du fond pour la pompe.

### Pièces imprimées (`print/`)

Projet [nurb](https://pypi.org/project/nurb/) : lancer `nurb` depuis `print/`. Export préféré : **3MF** dans `print/build/`. Sur cette machine, pour tout script Python (y compris hors `print/`) : **`uv run`**, pas le `python` système — détail dans `print/README.md`.

| Pièce | Où | Rôle |
| --- | --- | --- |
| `boitier_ps_24v` / `couvercle_ps_24v` | intérieur, face ouest | Alimentation 24 V, Wago 230 V |
| `boitier_dc` | intérieur, zone froide | Partie basse tension et aile 24 V de `combined` |
| `boitier_ac` | accolé au DC | Bande secteur de `combined` (J2, J3) |
| `couvercle_acdc` | — | Couvercle unique des deux boîtiers |
| `ensemble_boitiers` | — | Assemblage DC + AC |
| `boitier_pid` / `cache_pid` | compartiment technique | Pont qui tient le cache contre l'ouverture du module PID |
| `screen_wedge` / `screen_base` | façade | Cadre de l'écran Waveshare (vis M2.5 à l'arrière) et son berceau |
| `passe_cable` / `ecrou_passe_cable` | façade | Traversée du trou Ø 16 mm ex-bouton brew |
| `canal` | intérieur | Guidage des fils |

### Wago

Côté machine : FASTON 6,3 × 0,8 mm isolées nylon. Côté mod : Wago 221 (412, 415 selon l'emplacement) et connecteurs HT3.96 sur `combined`. Pas de piggyback. Vis M3, inserts laiton M2 et M2.5 pour les cartes. Aimants 8 × 3 mm. Wago sans contact avec le fond ni les parois.

## Matière d'impression

**Les boîtiers sont imprimés en PLA HT recuit** — après recuit, la tenue en température monte à **140 °C**, très au-dessus des 40–50 °C du compartiment technique et sans le fluage du PLA standard à partir de ~45 °C. La machine tourne 10–15 min, deux fois par jour ; le reste du temps tout est à température ambiante et hors tension.

Le recuit fait retirer les pièces : les cotes fit-critiques (puits d'aimant, logements Wago, inserts) se vérifient **après** recuit, pas sur la pièce sortie du plateau.

*Atelier : Bambu Lab A1 Mini.*

## Dépôt

```
coffeeflow/
  README.md          ← cette vue d'ensemble
  docs/              ← câblage, débitmètres, drip tray, maquettes Three.js ; datasheets/
  firmware/rust/     ← firmwares écran et capteurs, protocole CAN, coffeetool (Rust)
  archive/firmware/  ← firmware C++/ESP-IDF d'origine et outils Python (référence)
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

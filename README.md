# coffeeflow

Modification d'une **Profitec Go** pour du *flow profiling* et du *brew by weight* : l'infusion se lance depuis un écran tactile, la pression se module, et elle s'arrête toute seule au poids cible.

## Machine d'origine

- Vanne solénoïde trois voies + pompe vibratoire 35 W + boiler
- Petit boîtier PID : consigne / mesure de température du boiler, chrono
- Vanne OPV pour limiter la pression
- Plomberie en **1/8"**
- Connexions électriques internes : cosses à languette **FASTON 6,3 × 0,8 mm** (pas de piggyback : la dérivation se fait dans un boîtier imprimé)
- Deux compartiments : **technique** (hydraulique, boiler, groupe, électricité) et **réservoir d'eau**
- Bouton **brew** en façade (Ø 16 mm) : démarrait vanne + pompe. **Supprimé** — le trou sert de passe-câble DC vers l'écran

## Ce que la modification ajoute

Le 230 V de la pompe et de la vanne est repris dans un boîtier dédié, sans remplacer le câblage d'origine :

1. **Pompe** — un dimmer AC RobotDyn 4 A (option DimmerLink I2C) fait varier la pression.
2. **Vanne solénoïde** — un SSR ouvre et ferme le circuit d'infusion.

Plus de bouton brew parallèle, plus de relais NC de « mode manuel » : la commande passe par l'écran. Alimentation du système de mod : uniquement quand la machine est allumée (phase prise sur le relais boiler du PID).

Trois retours capteurs, centralisés dans le boîtier DC :

- **Pression** sur la plomberie, en amont de la vanne (I2C 3,3 V)
- **Débit** en amont de la pompe, côté basse pression (le capteur ne tient que 3 bar)
- **Poids** — soit une cellule via HX711 (pas encore décidé), soit une balance Acaia Lunar en BLE depuis l'écran

L'ESP32 du boîtier DC coupe l'infusion au poids cible. L'écran affiche et envoie les commandes.

## Architecture électronique

Trois boîtiers, deux nœuds ESP32. Le 230 V ne sort plus de la machine.

```
230 V L/N (machine allumée)
        │
   [Boîtier AC]                          intérieur, face ouest, près pompe / vanne
    dimmer · SSR · alim 5 V
        │              │              │
   L/N pompe      L/N vanne       5 V + GND
                                      │
                                 [Boîtier DC]    intérieur, zone froide
                                  XIAO ESP32-S3 · capteurs · CAN
                                      │              │
                              I2C dimmer      GPIO SSR
                              (vers AC)       (vers AC)
                                      │
                              5 V + CAN  ── un câble depuis DC,
                                      │     ou alim depuis AC + CAN depuis DC
                                 [Boîtier UI]    façade, trou Ø16 mm (ex-brew)
                                  wedge + base · Waveshare 4,3"
```

- **Boîtier AC** — tout ce qui touche au 230 V. Pas de MCU.
- **Boîtier DC** — commande le dimmer et le SSR, lit les capteurs, parle CAN à l'écran.
- **Boîtier UI** — affichage et commandes. **Seul module à utiliser la radio** (Wi‑Fi / BLE).

Maquettes Three.js (ouvrir dans le navigateur) : machine `docs/profitec_go.html`, boîtier AC `docs/ac_box.html`.

Les schémas `docs/atom_sensor.html` et `docs/atom_control.html` décrivent l'ancienne chaîne de trois Atom en UART. Ils sont **OUTDATED**.

### Pourquoi plus d'Atom

L'Atom Echo S3R est spécifié à **40 °C**. Le XIAO ESP32-S3 tient **85 °C**. À l'intérieur de la machine (45–50 °C dans le compartiment technique), la marge de l'Atom est nulle ; celle du XIAO est utilisable. L'écran Waveshare reste en façade, hors de cette enceinte.

---

### Boîtier AC

Tout le 230 V. Fixé par aimants contre la face **ouest** du compartiment technique, au plus proche de la pompe et de la vanne.

**Entrées / sorties**

| Domaine | Sens | Contenu |
| --- | --- | --- |
| AC | entrée | Phase + neutre (machine allumée) |
| AC | sortie | Paire L/N vers la pompe |
| AC | sortie | Paire L/N vers la vanne solénoïde trois voies |
| DC | entrée | Commande dimmer (I2C 3,3 V) et commande SSR (high/low) |
| DC | sortie | 5 V + GND pour les composants DC |

**Composants**

| Rôle | Matériel retenu | Notes |
| --- | --- | --- |
| Dimmer pompe | **RobotDyn AC Dimmer 4 A**, option **DimmerLink I2C** (rbdimmer.com) | Banc 2026-08-30 : **déjà en I2C** (plus d'UART). Logique **3,3 V**. **Pas de pull-up.** Header, module face à soi, gauche → droite : **VCC, GND, SDA, SCL**. Le Cortex du DimmerLink gère ZC / triac ; l'ESP32 ne voit que l'I2C. Sans **mains** sur le dimmer, le module reste en `Calibrating...` et n'accepte pas les commandes. Sur quelques centimètres, les pull-ups internes de l'ESP32 suffisent ; pas sur le câble DC↔AC. |
| SSR vanne | **M5Stack Unit SSR** (2 A) | Alim **5 V**, commande un signal low/high. |
| Alimentation DC | **RECOM RAC05-05SK-277-W** | 5 W, 5 V / 1 A, version **fils**. Encapsulée, 85–305 VAC. |

**Alternatives dimmer** (pas considérées pour l'instant) : RobotDyn AC Dimmer **8 A « raw »**, pins zero-cross + dim, comme dans l'ancienne architecture Atom.

**Alternatives SSR** (pas retenues) :

- SSR à borniers basé sur **Omron G3MB-202P** — certifié 240 VAC, mais le fabricant écrit *« for safety reasons, operate only with a maximum of 48 VAC »*
- Puck-style **Tru-Components TC-VSR8-10DA48Z** — 10 A, encombrement imposant

---

### Boîtier DC

Centralise les capteurs et commande les deux modules AC. Intérieur, **zone froide** : entre le module PID et le cadran de pression à aiguille, contre la face qui donne sur la cavité de purge de la vanne solénoïde, plus bas que le boiler en Z.

**Entrées / sorties**

| Sens | Contenu |
| --- | --- |
| Entrée | 5 V + GND depuis l'alim du boîtier AC |
| Sortie | I2C 3,3 V vers le DimmerLink |
| Sortie | High/low vers le SSR |
| Sortie | CAN vers l'écran (et éventuellement 5 V dans le même câble) |

**Composants**

| Rôle | Matériel | Liaison |
| --- | --- | --- |
| MCU | **Seeed Studio XIAO ESP32-S3** | Enfiché sur son **extension Grove** (4 ports Grove). Alim **5 V** sur les broches 5 V. Logique 3,3 V. |
| Dimmer | vers DimmerLink du boîtier AC | I2C 3,3 V — pas de pull-up sur le dimmer (voir banc ci-dessus) |
| Vanne | vers Unit SSR du boîtier AC | GPIO high/low ; le SSR s'alimente en 5 V |
| Débit | Digmesa FHKSC effet Hall, impulsions | Collecteur ouvert NPN, pull-up 1 kΩ vers 3,3 V + 100 nF (voir ci-dessous) |
| Pression | **Yufavor** I2C, filetage **G1/8** (compat. protocole Xidibei XDB401) | Même bus I2C que le dimmer. VCC **3,3 V** (l'emballage dit 5 V ; émulation XDB401 complète en 3,3 V, vérifié au banc). |
| Pesée | **HX711** | Pas encore décidé. Alternative : Acaia Lunar en BLE depuis l'écran — dans ce cas le HX711 disparaît |
| Climat (optionnel) | Température / humidité **dans le boîtier** | OneWire |
| CAN | **M5Stack Unit CAN** d'abord (encombrant, RX/TX → TWAI), puis **Adafruit CAN Pal** | 120 Ω incluse dans les deux cas |

Un seul bus I2C 3,3 V pour le dimmer et la pression. Le HX711, s'il vient, a ses propres lignes (DT/SCK). Le débitmètre est une GPIO d'impulsions, pas de l'I2C.

Les **4,7 kΩ du Yufavor** tiennent SDA et SCL pour tout le bus (dimmer compris). Le DimmerLink n'en a pas. Conséquence : retirer le capteur de pression laisse les lignes sans tirage — le dimmer seul ne parlera plus, sauf câble très court (pull-ups internes ESP32). Ne pas empiler un second 4,7 kΩ côté MCU tant que le Yufavor est sur le bus.

#### Pression Yufavor (compat. XDB401)

Quatre fils : **rouge VCC**, **noir GND**, **vert SDA**, **blanc SCL**. Gaine type silicone, **fine feuille de blindage** : **pas de continuité feuille ↔ GND** — relier la feuille à la masse **uniquement côté ESP32** (côté sonde : coupée et isolée, pas de boucle). Pull-up **4,7 kΩ** sur SDA et sur SCL, non débrayable. Filetage **G1/8**, d'où le choix (plomberie machine en 1/8").

#### Débitmètre Digmesa FHKSC

En service : **932-9521-A**, buse **1,20 mm**. Commandé : **932-9525-B**, buse **1,00 mm** (même famille, même collecteur ouvert NPN, PVDF/NSF). Fiche : `docs/datasheets/flowmeter-digmesa.pdf`.

| | 932-9521-A | 932-9525-B |
| --- | --- | --- |
| Buse | 1,20 mm | 1,00 mm |
| Sens de montage | 0° | 0° |
| Impulsions | 1925 imp/L (0,519 g) | **2382 imp/L (0,42 g)** |
| Plage linéaire | 0,075 – 0,569 L/min | **0,033 – 0,40 L/min** |
| Pré-infusion 0,5 g/s | sous le linéaire | **bas de plage** |
| Perte de charge | ~0,42 bar à 0,6 L/min | ~0,48 bar vers 0,40 L/min |
| **Pression max** | **3 bar à 20 °C** | **3 bar à 20 °C** |

Deux conséquences qui ne relèvent pas du câblage :

- **Le capteur va en amont de la pompe**, entre le réservoir et son entrée. La machine infuse à 9 bar et la pompe monte plus haut avant l'OPV : 3 bar de tenue interdisent le circuit haute pression.
- **Le 1,20 mm pose l'extraction au plancher** (36 g / 28 s = 0,077 L/min contre 0,075). Le **1,00 mm** met 0,077 L/min à 2,3 × le minimum, et 0,5 g/s (0,03 L/min) au début du linéaire. Plafond 0,40 L/min : une chasse pompe ouverte peut saturer. Comparaison OOTDTY : `docs/debitmetres.md`.

Sortie collecteur ouvert : il tire la ligne à la masse mais ne la monte jamais. R = **1 kΩ** vers le 3,3 V et C = **100 nF** vers la masse (passe-bas ≈ 1,6 kHz). GPIO en `INPUT`, pull-up interne éteinte. Le capteur est alimenté en 5 V ; c'est le tirage qui fixe le niveau haut à 3,3 V. Comparaison avec un second capteur : `docs/debitmetres.md`. Le schéma Atom (`docs/atom_sensor.html`, `docs/capteur_debit_digmesa_atom.md`) reste juste pour le RC — plus pour le MCU.

---

### Boîtier UI

Écran **Waveshare 4,3" LCD tactile**, module **ESP32-S3-WROOM**. En façade, à la place des boutons de commande.

Deux pièces imprimées, pour la fixation (l'écran est vissé à l'**arrière** du cadre) :

- **`screen_wedge`** — cadre de l'écran
- **`screen_base`** — accueille le wedge

**Entrées**

| Signal | Origine | Câble |
| --- | --- | --- |
| 5 V + GND | alim RECOM (via AC, éventuellement via DC) | Un seul câble depuis le boîtier DC, **ou** deux paires : alim depuis AC, CAN depuis DC |
| CAN | transceiver du boîtier DC | idem |

Traversée intérieur → façade : le trou **Ø 16 mm** de l'ancien bouton brew.

**Radio.** Le Waveshare est le seul nœud Wi‑Fi / BLE. BLE si le HX711 est abandonné au profit de l'Acaia Lunar. Wi‑Fi (remontée backend) : hors scope pour l'instant.

---

## Câblage

| Domaine | Fil | Connectique |
| --- | --- | --- |
| 230 V | Silicone **0,75 mm²**, paires L/N sous gaine (surface de boucle nulle) | Wago 221 à leviers ; FASTON 6,3 × 0,8 mm côté machine |
| 5 V / signaux / CAN | **0,25 mm²** | Grove, header, LiYCY si blindage |

Trois paires 230 V, tout dans le compartiment technique :

1. **L+N** machine allumée → boîtier AC
2. **L+N** boîtier AC → pompe
3. **L+N** boîtier AC → vanne

Le 230 V ne traverse plus vers la façade. Côté DC, I2C et GPIO de commande relient AC et DC ; 5 V et CAN relient DC (et éventuellement AC) à l'écran.

Notes de câblage 230 V plus anciennes (options 6 vs 8 conducteurs, dont le bouton brew encore en 230 V) : `docs/cablage.md`. L'option 8 conducteurs tombe avec la suppression du bouton.

## Firmware

Deux nœuds, plus trois :

- **Boîtier DC** (XIAO ESP32-S3) : Rust `no_std` — I2C (DimmerLink, pression), GPIO SSR, impulsions débitmètre, TWAI/CAN, logique de shot. Pas de radio.
- **Boîtier UI** (Waveshare ESP32-S3-WROOM) : affichage, commandes CAN, **seul** à monter Wi‑Fi / BLE.

Le dimmer I2C retire le besoin d'ISR zero-cross / PSM sur l'ESP32 (c'était le cœur de `docs/firmware_control.md`, encore écrit pour un Atom et un dimmer « raw »).

Le dépôt contient un test Arduino (`sound_test/`) sur Atom S3 Voice — reliquat. Le firmware Rust n'y est pas encore.

## Disposition mécanique

**Boîtier AC** — intérieur, compartiment technique, face ouest, aimants, au plus proche de la pompe et de la vanne.

**Boîtier DC** — intérieur, zone froide, entre PID et cadran de pression, contre la face qui donne sur la cavité de purge de la vanne, plus bas que le boiler en Z.

**Boîtier UI** — façade, emplacement des boutons. Le wedge cadre l'écran (vissé par l'arrière) ; la base accueille le wedge. Ensemble : `screen_assembly`.

Pas de perçage du châssis : aimants à l'intérieur, trou brew existant vers l'extérieur.

### Pièces imprimées (`print/`)

Projet [nurb](https://pypi.org/project/nurb/) : lancer `nurb` depuis `print/`. Export préféré : **3MF** dans `print/build/` (Studio peut avertir sur la spec 1.41 ; l’objet s’importe). Sur cette machine, pour tout script Python (y compris hors `print/`) : **`uv run`**, pas le `python` système — détail dans `print/README.md`.

| Pièce | Où | Matière | Rôle | Statut |
| --- | --- | --- | --- | --- |
| `boitier_2w` / `boitier_4w` | intérieur, ~10 cm du boiler | PETG | Dérivation 230 V. FASTON nylon côté machine, Wago 221 côté mod. Évite les cosses piggyback. | existe |
| `canal` (+ angle) | intérieur | PETG | Guidage des fils. | existe |
| **Boîtier AC** | intérieur, baie 70 mm, face ouest | PETG | Dimmer, SSR, alim RECOM. | à faire (`print/parts/ac_box` ignoré) |
| **Boîtier DC** | intérieur, zone froide | PETG | XIAO + Grove, capteurs, CAN. | à faire |
| `screen_wedge` | façade | **PLA** | Cadre de l'écran Waveshare, vis M2.5 à l'arrière. | en validation |
| `screen_base` | façade | PETG | Accueille le wedge. Plus de modules 230 V dedans. | existe (berceau de bureau) ; **fixation façade à reprendre** |

La grille d'entretoise 5 mm sous le plateau chauffant n'a plus lieu : l'UI n'est plus posée sur la machine.

### Wago 230 V

Côté machine : FASTON 6,3 × 0,8 mm isolées nylon. Côté mod : Wago. Pas de piggyback. Vis M3. Aimants 8 × 3 mm. Wago sans contact avec le fond ni les parois.

## Matières d'impression

Le châssis mesure **40–50 °C** en fonctionnement. La machine tourne 10–15 min, deux fois par jour ; le reste du temps tout est à température ambiante et hors tension. La façade (UI) est hors de l'enceinte chaude.

| | PLA | PETG |
| --- | --- | --- |
| Transition vitreuse | 55–60 °C | 78–85 °C |
| Limite pratique sans charge | ~45 °C | ~65 °C |
| Limite pratique sous charge continue (fluage) | ~40 °C | ~60 °C |

**PETG partout, sauf `screen_wedge`.** Le risque du PLA n'est pas la fonte mais le fluage sous charge permanente à partir de ~45 °C. Les boîtiers AC / DC et les dérivations 230 V sont dans l'air chaud enfermé ; le boîtier AC tient en plus des bornes secteur.

**`screen_wedge` est en PLA**, pour le rendu : l'*ironing* de la face supérieure donne un fini que le PETG ne sait pas produire. L'écran est en façade, et l'alimentation 5 V vit dans le boîtier AC — plus dans la base sous le cadre. Si le cadre gondole malgré tout, la réponse est PETG et l'abandon de l'ironing.

*Atelier : Bambu Lab A1 Mini. PETG HF Black 33102 pour le PETG.*

## Dépôt

```
coffeeflow/
  README.md          ← cette vue d'ensemble
  docs/              ← schémas et notes (atom_*.html = OUTDATED) ; datasheets/
  tests/             ← scripts MicroPython de banc (Atom Echo S3R)
  print/             ← impressions 3D (nurb)
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

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

Deux capteurs en feedback :

- **Température du groupe** (sonde collée au groupe, pas l'eau du boiler)
- **Pression** sur la plomberie, en amont de la vanne

Une **balance Acaia en BLE** (balance = master, Atom = client) donne le poids dans la tasse. L'Atom coupe l'infusion au poids cible.

## Électronique (écosystème M5Stack)

| Rôle | Matériel |
| --- | --- |
| Contrôle temps réel (GPIO, dimmer, UART, BLE) | M5Stack Atom S3 |
| Infusion parallèle (vanne + pompe) | ACSSR, GPIO |
| Pompe toujours dispo en manuel | Unit Relay **NC**, en parallèle du circuit pompe |
| Variation de pression | RobotDyn AC Dimmer 4 A, GPIO, en parallèle du relais NC |
| Température groupe | M5Stack KISOMeter (I2C) |
| Pression | capteur I2C |
| Bus capteurs | Hub I2C M5Stack → un seul Grove I2C vers l'Atom |
| UI (débit, pression, poids final) | Waveshare LCD tactile 4,3" (ESP32-S3-VROOM), UART vers l'Atom |

L'Atom envoie à quelques hertz, sur l'UART, les valeurs capteurs + balance. L'écran envoie la commande **départ**.

Wifi sur l'écran (remontée backend) : hors scope.

## Firmware

- **Atom S3** : Rust `no_std` — dimmer et UART en temps réel, client BLE Acaia
- **Écran tactile** : Rust + lib graphique, UART vers l'Atom

Le dépôt contient aujourd'hui un test Arduino (`sound_test/`) sur Atom S3 Voice. Le firmware Rust n'y est pas encore.

## Disposition mécanique

**Dans la machine** (châssis ~40–50 °C, aimants, pas de perçage) : seulement les deux boîtiers de dérivation.

**Hors machine**, posés sur le dessus : boîtier Atom (+ ACSSR, relais, RobotDyn) et boîtier écran triangulaire (angle 60°), emboîté sur le boîtier Atom.

Traversée intérieur → extérieur : **quatre fils 230 V** + **un câble Grove I2C**. Des goulottes imprimées sont possibles pour les guider.

### Boîtiers 3D (`print/`)

Projet [nurb](https://pypi.org/project/nurb/) : lancer `nurb` depuis `print/`. Export préféré : **3MF** dans `print/build/` (Studio peut avertir sur la spec 1.41 ; l’objet s’importe). Sur cette machine, pour tout script Python (y compris hors `print/`) : **`uv run`**, pas le `python` système — détail dans `print/README.md`.

| Pièce | Où | Rôle |
| --- | --- | --- |
| Boîtier de dérivation **230 V** | intérieur | FASTON nylon côté machine, Wago 221-423 côté mod. Évite les cosses piggyback. Quatre Wago **verticaux**. |
| Boîtier de dérivation **DC** | intérieur | KISOMeter + hub I2C. Les fils des deux capteurs y arrivent ; un seul Grove I2C en sort. |
| Boîtier Atom | extérieur | Atom S3, ACSSR, Unit Relay, RobotDyn |
| Boîtier écran | extérieur | Triangle 60°, posé sur le boîtier Atom |
| Goulottes | selon besoin | Guidage des 4 × 230 V + Grove |

#### Quatre Wago 230 V

1. **Phase machine allumée** — sur le relais boiler du PID (alimentée dès le bouton on/off)
2. **Neutre** — pris sur la pompe
3. **Phase brew** — sortie de la vanne (infusion parallèle)
4. **Phase pompe** — entrée de la pompe (relais NC *ou* dimmer, selon manuel / contrôlé)

Fils : Helutherm 145, 0,75 mm², Ø 2,2 mm. Vis M3. Aimants 8 × 3 mm. Wago sans contact avec le fond ni les parois.

Atelier : Bambu Lab A1 Mini, PETG HF Black 33102.

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

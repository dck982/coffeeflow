# Migration vers Rust — document de travail

Ce document est un **document de travail**. Il remplace les notes de session
qui ont précédé l'essai. Il décrit l'environnement de build, la cible, la
règle de déploiement et les premières étapes. Il doit suffire pour reprendre
le travail sans autre contexte.

`docs/firmware.md` reste la référence du firmware en service, écrit en C++ sur
ESP-IDF 6.1. Rien ici ne le modifie tant qu'une étape n'a pas été validée sur
la carte.

---

## Règle n° 1 : ne pas casser le firmware écran

L'écran est en service dans la machine. Toute expérience Rust doit pouvoir
être annulée par une simple bascule d'`otadata`, sans reflasher coffeeflow. En
cas de doute sur une commande, on ne la lance pas.

Concrètement, un essai **n'écrit jamais** ailleurs que dans le slot OTA
inactif, et ne touche que deux zones :

- le slot de test, via `parttool.py write_partition` ;
- `otadata`, uniquement via `otatool.py switch_ota_partition`.

Les zones suivantes sont **intouchables** : bootloader (`0x0`), table de
partitions (`0x8000`), `nvs`, `phy_init`, `factory`, le slot qui porte
coffeeflow, `ota_staging` et `assets`.

**NVS : ni lecture, ni sauvegarde, ni effacement.** NVS contient les
identifiants Wi-Fi. Une copie (`read_flash`, `parttool read_partition`,
`nvs_tool`, etc.) les exposerait en clair sur le disque. On ne fait donc pas de
« sauvegarde de précaution » de NVS. La protection consiste à ne jamais
l'écrire.

**À partir de l'étape 8 (décision du 2026-10-09), la règle est levée en
partie sur l'écran**, et sur lui seul. Il est sur le bureau, en USB, et
coffeeflow se reconstruit depuis Git : la phase devient destructive. Sont
alors permis, et seulement dans le cadre de l'étape 8 :

- `esp-storage` et les écritures en flash depuis Rust : slot inactif,
  `otadata`, et la zone de test NVS décrite à l'étape 8 ;
- la réécriture du bootloader de l'écran, une fois, en USB (étape 8,
  préalable) ;
- `nvs` lu, effacé ou restauré **uniquement par l'outil `nvs-vault`**, qui
  écrit la sauvegarde hors du dépôt et n'en affiche que la taille et le
  SHA-256. Claude ne lit jamais ce fichier.

`factory`, la table de partitions, `assets` et `ota_staging` restent
intouchables. Sur `sensors`, rien ne change : on n'y écrit qu'à travers la
chaîne OTA C++ existante (étape 9).

### Commandes interdites sur l'écran

| Commande | Pourquoi |
|---|---|
| `cargo run` (runner `espflash flash --monitor` posé par esp-generate) | écrit le bootloader et la table d'espflash, puis l'app dans la première partition app, c'est-à-dire `factory` |
| `espflash flash`, `espflash write-bin`, `espflash erase-*` | même effet, ou un effacement |
| `probe-rs run`, `probe-rs download`, `cargo embed` | flashent l'ELF avec leur propre mise en page |
| `idf.py flash`, `idf.py app-flash`, `esptool write_flash` sans partition ciblée | réécrivent bootloader, table et `ota_data_initial`, ou posent l'app dans `factory` |
| crate `esp-storage`, ou tout code qui écrit en flash | les tests n'en ont pas besoin, et une écriture dans `nvs` ou `assets` serait irréversible |
| lire, copier ou sauvegarder `nvs` | identifiants Wi-Fi |

Dans chaque projet d'essai, le runner de `.cargo/config.toml` est **remplacé**
par `espflash monitor` (voir plus bas). Un `cargo run` réflexe ne peut alors
plus flasher.

### Ce que l'essai interrompt, et c'est voulu

Pendant un essai, coffeeflow ne tourne pas. Le module capteurs ne reçoit plus
rien par CAN : après expiration du bail, il coupe pompe, vanne et chauffe
(voir « Sécurité » dans `docs/firmware.md`). Les essais se font donc **machine
au repos**, jamais pendant une infusion ou une chauffe qu'on veut garder.

Le CH422G est une puce séparée. Il **garde son registre de sortie** à travers
un reset de l'ESP32-S3, mais pas à travers une coupure d'alimentation. Au
démarrage d'un essai, CAN_SEL, DISP et les resets sont donc dans l'état laissé
par coffeeflow. Tout essai qui parle au CH422G écrit l'**octet complet** du
registre de sortie (`0x38`), jamais un bit isolé. Les essais Rust gardent
**CAN_SEL bas** : ils n'utilisent pas le CAN, et le transceiver reste ainsi
désélectionné.

---

## État de la carte (lu le 2026-10-08)

Lu en USB, en lecture seule : la table de partitions à `0x8000` et les deux
copies d'`otadata`. NVS n'a pas été lu.

```
# Name,      Type, SubType, Offset,   Size
nvs,         data, nvs,     0x9000,   24K
phy_init,    data, phy,     0xf000,   4K
otadata,     data, ota,     0x10000,  8K
factory,     app,  factory, 0x20000,  1000K    pont USB-série ↔ CAN, jamais réécrit
ota_0,       app,  ota_0,   0x120000, 3500K
ota_1,       app,  ota_1,   0x490000, 3500K
ota_staging, data, 0x40,    0x7fb000, 1M
assets,      data, spiffs,  0x8fb000, 2976K
```

| Copie `otadata` | Séquence | État |
|---|---|---|
| 0 | `0x51` | VALID |
| 1 | `0x52` | VALID |

| Slot | Descripteur d'app |
|---|---|
| `ota_0` | `screen` `314a68a-dirty`, 7 oct. 2026, IDF v6.1 (l'image précédente) |
| `ota_1` | `screen` `5aac3e0-dirty`, 8 oct. 2026, IDF v6.1 — **en cours d'exécution** (séquence la plus haute, `(0x52 − 1) mod 2 = 1`) |

Conséquence : **à cette date, le slot de test est `ota_0`.** L'écrire efface
l'image coffeeflow précédente, qui servait de repli après la dernière mise à
jour. C'est accepté : `ota_1` est `VALID` et confirmé, `factory` reste
intacte, et l'image précédente se reconstruit depuis Git.

Une mise à jour OTA de coffeeflow fait alterner les slots. **Il faut donc
relire `otadata` avant chaque session**, et non supposer que le slot de test
est toujours `ota_0`. Le slot de test est par définition celui qui ne porte
*pas* coffeeflow.

Les deux copies sont `VALID`. `switch_ota_partition` ne réécrit que la
séquence et le CRC de l'autre copie, pas son état. La bascule vers le test
puis le retour se font donc sans délai de rollback ni `POST /firmware/confirm`.
Si une copie passe un jour en `INVALID` ou `ABORTED`, le bootloader l'ignore et
redémarre sur coffeeflow : l'essai ne démarre pas, mais rien n'est cassé. Dans
ce cas, on s'arrête et on regarde, sans « réparer » `otadata` à la main.

Une image Rust ne marque jamais son slot comme valide, puisqu'elle n'a pas
`esp-storage`. Le bootloader installé ne fait pas passer les images en
`PENDING_VERIFY`. Il n'y a donc **pas de retour automatique** : le retour se
fait toujours à la main avec `otatool`. C'est fiable, parce que le mode
téléchargement ROM reste accessible par le port UART, même si l'essai plante
en boucle.

---

## Environnement de build (vérifié le 2026-10-08)

### Ce qui est installé

| Outil | Version | Rôle |
|---|---|---|
| `rustup` + toolchain `stable` aarch64 | — | outils hôte, futur `coffeetool` |
| toolchain `esp` (installée par `espup`) | rustc 1.97.0-nightly fork Espressif, avec `rust-src` | cible `xtensa-esp32s3-none-elf` |
| GCC Xtensa | `esp-16.2.0_20260914` | éditeur de liens (`-nostartfiles`) |
| `espup` | 0.18.0 | installe et met à jour la toolchain `esp` |
| `espflash` | 4.6.0 | `save-image` et `monitor` **seulement** |
| `esp-generate` | 1.4.0 | squelettes de projet (esp-hal ~1.2.2, esp-rtos 0.4.0, esp-bootloader-esp-idf 0.6.0, esp-println 0.18.0, esp-alloc 0.11.0, esp-radio 1.0.0-beta.1) |
| `probe-rs` | 0.32.0 | facultatif, voir plus bas |
| ESP-IDF 6.1 (`~/.espressif/v6.1`) | — | fournit `otatool.py` et `parttool.py` |

Installation, pour mémoire :

```sh
cargo install espup --locked && espup install     # écrit ~/export-esp.sh
cargo install espflash --locked
cargo install esp-generate --locked
cargo install probe-rs-tools --locked             # binaire probe-rs
# facultatif, interface TUI de configuration :
# cargo install esp-config --features=tui --locked
```

`~/export-esp.sh` ajoute le GCC Xtensa au `PATH` et pose `LIBCLANG_PATH`. Il
faut le sourcer **dans chaque shell** qui compile :

```sh
source ~/export-esp.sh
```

`otatool.py` et `parttool.py` ne sont pas dans le `PATH` par défaut. On active
l'environnement IDF dans le shell qui flashe :

```sh
source ~/.espressif/tools/activate_idf_v6.1.sh
```

### Vérifications faites

- `examples/hello_world` du clone local d'esp-hal (`tmp/esp-hal`, esp-hal
  1.2.0, commit `76a0e71d5`) compile pour l'esp32s3 en release en 54 s à
  froid.
- Un squelette `esp-generate` (esp32s3, PSRAM octale, embassy, alloc, log)
  compile en 42 s avec les versions de crates.io.
- `espflash save-image --chip esp32s3` produit une image au format IDF
  (108 Ko pour hello_world). Grâce à `esp_bootloader_esp_idf::esp_app_desc!()`,
  le bootloader IDF déjà installé l'accepte.
- Un seul avertissement de l'éditeur de liens : « LOAD segment with RWX
  permissions ». Il est sans conséquence sur cette cible.

### Références

- Le clone `tmp/esp-hal` sert de **référence de lecture** : sources du driver
  DPI (`esp-hal/src/lcd_cam/lcd/dpi.rs`), PSRAM
  (`esp-hal/src/psram/esp32s3.rs`), exemples BLE (`examples/ble/scanner`,
  trouble-host 0.8) et Wi-Fi (`examples/wifi/embassy_access_point`). Les
  projets, eux, dépendent des **versions publiées sur crates.io**, celles que
  choisit `esp-generate`, et pas de chemins vers le clone.
- `examples/qa/src/bin/lcd-dpi.rs` n'est **pas** un point de départ pour
  l'écran. Sa configuration 800×480 vise une autre carte (ESP32-S31-Korvo), et
  il pousse en boucle 32 octets d'une couleur unie avec `dma_loop_buffer!`,
  sans framebuffer, sans PSRAM et sans bounce buffer.

### probe-rs : pas utile au départ

Sur l'ESP32-S3, l'USB-JTAG natif est sur GPIO19/20, les broches que le mux
FSUSB42 bascule vers le CAN quand CAN_SEL est haut. Avec CAN_SEL bas, le port
« USB » de la Waveshare redevient l'USB-JTAG, et `probe-rs attach` pourrait
servir (RTT, inspection mémoire, sans flasher). Ce n'est utile que si ce
connecteur est accessible dans la machine, et seulement en `attach` : jamais
`run` ni `download`. Les essais utilisent la console UART.

---

## Cible finale

À terme, tout le code de la machine est en Rust, dans un workspace Cargo
unique sous `firmware/rust/` :

```
firmware/rust/
  Cargo.toml          workspace
  protocol/           no_std, sans alloc : le contrat CAN
  screen/             app Waveshare (xtensa-esp32s3-none-elf)
  sensors/            app XIAO (xtensa-esp32s3-none-elf)
  coffeetool/         binaire hôte macOS (std), remplace firmware/tools/coffeetool
  trials/NN-nom/      essais isolés, décrits plus bas
```

- **`protocol`** reprend `firmware/common/` : identifiant 11 bits
  (type/dest/src), types de messages, charges utiles (`messages.hpp`), CRC,
  cadrage série et OTA (`framing.hpp`), version, codes de log. La source de
  vérité des codes de log reste `common/codegen/log_codes.yaml`. Le Rust est
  généré depuis ce fichier, comme le C++ et le Python aujourd'hui. Le crate
  compile en `no_std` pour les deux cartes et en `std` pour `coffeetool`. Ses
  tests tournent sur l'hôte (`cargo test -p protocol`).
- **Compatibilité binaire.** Pendant la migration, C++ et Rust coexistent sur
  le bus. `protocol` doit donc être identique au bit près à `common/`. Avant
  tout usage sur le bus, il se teste contre des vecteurs produits par les
  tests C++ existants (`common/test`).
- **`coffeetool`** reprend le décodeur, le transport série/WebSocket, le
  client de flash et l'enregistreur de l'outil Python, en partageant
  `protocol` au lieu de le réimplémenter.
- **`screen` et `sensors`** s'exécutent dans les slots OTA. L'image `factory`
  de chaque carte reste celle d'aujourd'hui, en C++ : elle n'est jamais
  réécrite, et le filet de récupération ne change pas. Les apps Rust gardent le
  bootloader IDF (`esp_app_desc!`) et doivent, à terme, implémenter la
  confirmation OTA et le rollback décrits dans `docs/firmware.md`. Ce sera la
  première étape qui ajoute `esp-storage`, et elle se conçoit à part.
- **Avant la production : refaire l'image `factory` de l'écran.** D'après
  ce qu'on en sait, elle ne met pas à jour correctement l'état OTA :
  l'image resterait en `NEW`. C'est à vérifier dans son code avant de la
  refaire. On profitera d'une session où l'écran est branché en USB pour
  la réécrire. Ce sera la seule exception à « `factory` n'est jamais
  réécrite ». Elle se prépare à part, avec sa propre procédure, puisque
  la règle n° 1 interdit d'écrire dans `factory` pendant les essais.
- **Ordre.** `protocol` et `coffeetool` d'abord, sans risque matériel. Puis
  `screen`, la carte la plus risquée en technique (LCD, BLE, Wi-Fi), sur
  laquelle portent les essais ci-dessous. `sensors` en dernier : c'est la
  carte la plus simple, mais elle est enfermée dans la machine et se met à
  jour uniquement à travers l'écran.

### `coffeetool` : un seul outil (demande du 2026-10-08)

Le nouvel outil remplace la plupart des scripts de `firmware/tools`, pour
ne plus écrire de commandes `curl` ni lancer un script par tâche.

**Transport par défaut : HTTP.** L'outil vise `http://$COFFEEFLOW_IP`, avec
le jeton `$COFFEEFLOW_HTTP_TOKEN` en `Authorization: Bearer`, comme les
scripts actuels. S'il manque une variable, il s'arrête en la nommant.
`--ip` et `--token` les remplacent pour une commande. **`--usb [PORT]`**
passe par le port série (pont USB-série ↔ CAN de l'image `factory`). Par
défaut, le port est celui de l'écran. Seules les commandes qui ont un
sens sur le bus CAN acceptent l'USB.

| Commande | Remplace | HTTP | `--usb` |
|---|---|---|---|
| `flash <image> --target screen\|sensors` | `coffeetool flash` | `POST /firmware?target=` | `FLASH_CTRL`/`FLASH_DATA` |
| `confirm` | `confirm_fw.sh` | `POST /firmware/confirm` | — |
| `monitor [--out <fichier>]` | `coffeetool monitor`, `coffeetool record` | WebSocket `/ws` (miroir CAN) | flux série |
| `send`, `replay` | `coffeetool send`, `replay` | WebSocket | série |
| `telemetry` | `curl GET /telemetry` | `GET /telemetry`, JSON indenté | — |
| `telemetry --record [--rate] [--duration] [--scace]` | `record_probe.py`, `record_heating.py` | `/telemetry` en boucle, dans `captures/` | — |
| `config save [fichier]` | `save_config.py` | `GET /config`, écrit dans `configs/` | — |
| `config restore <fichier>` | `restore_config.py` | `POST /config` | — |
| `set heating on\|off` | `set_heating.py` | `GET` puis `POST /config` | — |
| `set scace on\|off` | `set_scace.py` | `GET` puis `POST /config` | — |
| `download-hf` | `download_hf_capture.py` | dernière capture HF, dans `captures/` | — |
| `purge <s>` | `purge.py` | idem | — |

- **`set`** garde le protocole des scripts : relire `/config` pour sa
  `version`, n'envoyer que la clé changée avec cette version, puis
  vérifier la réponse. Une nouvelle clé s'ajoute au tableau des réglages
  de l'outil ; il n'y a pas de `set <chemin> <valeur>` générique.
- **`telemetry --record`** remplace `record_probe.py` et
  `record_heating.py`, qui font la même chose à la fréquence et au tri
  près. Chaque échantillon garde **toute** la télémétrie
  (`{elapsed_s, received_at_utc, telemetry}`), et un instantané de `/config`
  est pris au départ : il remplace la consigne que relevait
  `record_heating.py`. Le fichier garde l'enveloppe des scripts :
  `started_at_utc`, `interval_s`, `max_duration_s`, `samples`,
  `stop_reason`, `ended_at_utc`, plus `config`. Par défaut : `--rate` 5 Hz,
  et sans `--duration`, l'enregistrement dure jusqu'à Ctrl-C. Une ligne
  d'état toutes les 10 s donne la température chaudière et la consigne,
  la puissance, la pression, le rapport a0/a1, et la SCACE si elle est
  lue. Taille : un enregistrement ne dépasse jamais 5 min, soit environ
  3 Mo à 5 Hz pour une réponse de 2 Ko. Garder toute la télémétrie ne
  pose donc pas de problème.
- **`--scace`** lit la SCACE en BLE depuis le Mac, comme `record_probe.py`
  avec bleak. En Rust, c'est `btleplug`, la seule dépendance BLE côté hôte.
  Chaque échantillon porte alors `scace_c` et `scace_age_ms`.
- **`monitor --out`** enregistre le flux CAN décodé, comme l'ancien
  `coffeetool record`. Le fichier garde son format, que `replay` relit.
- **Formats de fichier :** `analyze_probe.py` lit les codes A0/A1 dans
  `sample["telemetry"]` pour les nouveaux fichiers, et à la racine de
  l'échantillon pour les anciens. Les captures HF ne changent pas.
- **Restent en Python**, hors de `coffeetool` : les outils d'analyse et de
  tracé, qui n'ont jamais parlé à la machine et s'appuient sur matplotlib,
  numpy ou scipy. Ce sont `analyze_hf_capture.py`, `analyze_probe.py`,
  `describe_hf_capture.py`, `plot_hf_capture.py`, `replay_brew_pi.py`,
  `simulate_boiler.py` et `scace_calibration.py`. Restent aussi
  `scace_ble_log.py` (BLE Mac ↔ SCACE seul, pour la calibration) et
  `check_screen_strings.py`, propre à l'UI LVGL.

---

## Approche : des essais petits et isolés

Chaque essai est un crate autonome sous `firmware/rust/trials/NN-nom/`, créé
avec `esp-generate`. Il ne prouve qu'une seule chose, mesure ce qu'il doit
mesurer, puis on revient sur coffeeflow. Les essais restent dans le dépôt :
ils servent de référence quand on écrira `screen`.

### Créer un essai

```sh
cd firmware/rust/trials
esp-generate --headless -s -o esp32s3 -o esp32s3-wroom-1-octal-psram \
  -o unstable-hal -o alloc -o embassy -o log -o esp-backtrace NN-nom
rm -rf NN-nom/.git            # esp-generate fait un git init imbriqué
```

Cargo refuse un nom de paquet qui commence par un chiffre. Dans
`Cargo.toml`, `name` (paquet et `[[bin]]`) devient `trial-NN-nom` ; le dossier
garde `NN-nom`. Les chemins `target/.../release/` qui suivent utilisent donc
`trial-NN-nom`.

Puis, dans chaque essai :

- **Runner :** dans `.cargo/config.toml`, remplacer
  `runner = "espflash flash --monitor --chip esp32s3"` par
  `runner = "espflash monitor --chip esp32s3 --port /dev/cu.usbmodem5B790235091 --elf"`.
  `cargo run` ne fait alors plus qu'ouvrir la console, avec décodage des
  backtraces.
- **Console :** `esp-println` avec la feature `uart`, et non `auto` ni
  `jtag-serial`. Le port « UART » de la Waveshare (pont CH343,
  `/dev/cu.usbmodem5B790235091`) est câblé sur UART0, GPIO43/44, où sortent
  aussi les logs du bootloader IDF. Le port USB natif est partagé avec le CAN.
- **PSRAM :** le modèle `-octal-psram` ne configure pas la PSRAM. Les
  essais qui en ont besoin l'initialisent en `PsramMode::OctalSpi` à 80 MHz
  (8 Mo, comme coffeeflow) et l'ajoutent au tas avec
  `esp_alloc::psram_allocator!`.
- **Brochage :** reprendre celui de `firmware/screen/main/board.h` et
  `service_screen.cpp`, rappelé plus bas, jamais celui d'un exemple.

### Déployer un essai

Les étapes 1 à 3 se font **une fois par session**, et l'étape 3 avant toute
écriture.

```sh
source ~/export-esp.sh
source ~/.espressif/tools/activate_idf_v6.1.sh
PORT=/dev/cu.usbmodem5B790235091

# 1. Construire et convertir au format d'image IDF
cargo build --release
espflash save-image --chip esp32s3 \
  target/xtensa-esp32s3-none-elf/release/trial-NN-nom /tmp/trial.bin   # ≤ 3500K

# 2. Lire otadata : quel slot porte coffeeflow ?
otatool.py -p $PORT read_otadata
#    slot courant = (séquence la plus haute − 1) mod 2 ;
#    TEST = l'autre. Au 2026-10-08 : coffeeflow = ota_1, TEST = ota_0.
#    Si une copie est INVALID/ABORTED : s'arrêter.
#    read_otadata n'affiche que séquence et CRC, pas l'état. Pour l'état :
#    parttool.py read_partition --partition-name otadata (otadata, jamais nvs),
#    puis lire le mot de 32 bits à l'octet 24 de chaque copie (0x0, 0x1000) :
#    2 = VALID, 3 = INVALID, 4 = ABORTED.

# 3. Écrire l'essai dans le slot de test, et seulement là
parttool.py -p $PORT write_partition --partition-name TEST --input /tmp/trial.bin

# 4. Basculer (une seule fois par session ; ensuite, réécrire TEST suffit)
otatool.py -p $PORT switch_ota_partition --name TEST

# 5. Console (voir l'avertissement ci-dessous)
espflash monitor --chip esp32s3 --port $PORT --elf target/xtensa-esp32s3-none-elf/release/trial-NN-nom

# 6. Fin de session : retour sur coffeeflow
otatool.py -p $PORT switch_ota_partition --name COFFEEFLOW
```

**Ouvrir le port redémarre la carte.** Sous macOS, l'ouverture comme la
fermeture du port du pont CH343 basculent DTR/RTS, même depuis un lecteur
pyserial qui les force à zéro. Chaque lecture de console relance donc l'essai,
et un phénomène observé à l'écran peut n'être qu'un redémarrage. Pendant une
observation longue, on ne touche pas au port. `espflash monitor --no-reset`
ne règle rien : il se connecte au bootloader ROM et laisse la puce en mode
téléchargement. On la relance avec `espflash reset`, qui n'écrit rien.
`parttool.py write_partition` redémarre aussi la carte à la fin.

Après le retour (étape 6), on vérifie que coffeeflow a bien redémarré :
`GET /telemetry`, `firmware.screen.ota.running_partition`. Le test laissé dans
le slot inactif ne gêne pas : la prochaine mise à jour OTA de coffeeflow
l'écrasera.

Ces étapes seront regroupées dans un script `firmware/rust/trial-ota.sh`
qui refuse d'écrire si le slot visé est le slot courant, ou si une copie
d'`otadata` n'est pas `VALID`. Tant que ce script n'existe pas, on suit les
étapes à la main.

### Rappel du matériel (Waveshare ESP32-S3-Touch-LCD-4.3)

- ESP32-S3, flash 16 Mo, PSRAM octale 8 Mo.
- **CH422G** sur l'I2C SDA=GPIO8, SCL=GPIO9. Adresse `0x24` = registre de mode,
  `0x38` = registre de sortie. Bits de sortie : TP_RST=1, DISP=2
  (dalle et rétroéclairage), LCD_RST=3, SD_CS=4, CAN_SEL=5. On écrit toujours
  l'octet complet, et CAN_SEL reste à 0 pendant les essais.
- **Mise sous tension de la dalle** (comme `board::panel_power_on()`) : DISP
  haut, LCD_RST et TP_RST bas pendant 100 ms, puis hauts pendant 200 ms avant
  le RGB et le GT911.
- **RGB 800×480 RGB565**, PCLK 16 MHz sur front descendant. HSYNC
  pulse/back/front = 48/88/40, VSYNC = 3/32/13. Broches : HSYNC=46, VSYNC=3,
  DE=5, PCLK=7. Données D0..D15 = 14, 38, 18, 17, 10, 39, 0, 45, 48, 47, 21,
  1, 2, 42, 41, 40.
- **GT911** sur le même I2C, IRQ=GPIO4 (piloté bas pendant le reset pour
  l'adresse `0x5D`), lu en polling.

### Ce que coffeeflow a appris sur l'affichage (à ne pas réapprendre)

- Un trame dure (800+48+88+40) × (480+3+32+13) = 976 × 528 = 515 328 PCLK,
  soit **≈ 31 Hz** à 16 MHz.
- coffeeflow utilise un framebuffer de 768 000 octets en PSRAM et deux bounce
  buffers internes de 40 lignes (64 000 octets chacun). Un bounce buffer se
  vide en 40 × 976 / 16 MHz ≈ **2,44 ms**. Le remplissage depuis la PSRAM doit
  donc tenir **≥ 26 Mo/s en continu**, interruptions comprises.
- Le décalage d'image et les sauts venaient de la **contention PSRAM** :
  l'ISR de recopie tournait sur un cœur pendant que LVGL écrivait le
  framebuffer depuis l'autre. Le correctif a été de mettre l'ISR LCD et le
  dessin **sur le même cœur**, et de ne redessiner que ce qui change.
  `docs/screen-issue.md` donne le détail.
- Les écritures en flash coupent le cache PSRAM et affament l'ISR. Les essais
  n'écrivent pas en flash, et la question reviendra avec l'OTA en Rust. Le
  correctif d'IDF (`CONFIG_SPIRAM_XIP_FROM_PSRAM`, `docs/screen-issue.md`)
  n'a pas d'équivalent dans esp-hal 1.2.2 ; l'essai 04b l'écrit
  (`src/xip.rs`, étape 4). De plus, `esp-storage`
  fait chaque effacement et chaque écriture en section critique, et exige
  que l'autre cœur soit arrêté pendant ce temps. Voir la section « Écritures
  en flash ».
- Le double framebuffer avec `avoid_tearing` a été essayé dans IDF puis
  abandonné (écran instable).

### Écritures en flash : à concevoir à part

Toutes les écritures en flash de l'écran relèvent de cette question, et pas
seulement l'OTA :

- l'image `screen` écrite dans le slot inactif ;
- le relais du flash `sensors`, écrit dans `ota_staging` au fil des blocs
  reçus ;
- la confirmation et le rollback, qui réécrivent `otadata` ;
- le stockage persistant : identifiants Wi-Fi, `/config` et calibrations,
  aujourd'hui en NVS. Il faudra soit une implémentation NVS en Rust
  compatible avec les données existantes, soit un autre format avec une
  migration au premier démarrage. Le choix se fait avec le reste de cette
  section.

Le problème est matériel : pendant un effacement ou une écriture de la flash,
le cache de la mémoire externe (flash et PSRAM) est coupé. La recopie des
bounce buffers ne peut plus lire le framebuffer, et l'affichage reçoit des
lignes périmées. Rust ne le fait pas disparaître.

Chiffres à vérifier sur la puce de l'écran, mais typiques : effacement d'un
secteur de 4 Ko en ~45 ms (jusqu'à plusieurs centaines), programmation
d'une page de 256 octets en ~1 ms. `esp-storage` fait chaque opération sous
section critique : l'interruption de recopie est bloquée pendant tout ce
temps, soit environ 37 recopies perdues par secteur effacé.

Différence avec coffeeflow avant son correctif : le contrôle au VSYNC
détecte une trame incomplète et relance le DMA en phase. Une écriture flash
devrait donc donner une ou deux trames fausses, puis l'image se rétablit,
sans décalage permanent. À vérifier.

Pistes, à trancher avant l'OTA en Rust :

- accepter un écran figé ou un écran fixe « mise à jour » pendant l'OTA, et
  des écritures NVS rares, groupées et brèves ;
- programmer les pages pendant le blanking vertical (2,9 ms). Cela ne marche
  pas pour un effacement de secteur, trop long ;
- la suspension d'effacement de la flash (`CONFIG_SPI_FLASH_AUTO_SUSPEND`
  côté IDF), si la puce la supporte et si esp-storage peut l'utiliser ;
- l'équivalent de `XIP_FROM_PSRAM` : copier le code et les rodata en PSRAM
  et remapper la MMU. Écrit et validé par l'essai 04b pour l'affichage
  (étape 4). Reste à vérifier ce qu'il permet pendant une écriture : le code
  ne lit plus la flash, mais `esp-storage` coupe encore le cache et les
  interruptions.

`esp-storage` exige aussi que l'autre cœur soit arrêté pendant l'opération :
avec la radio sur le cœur 1, il faudra le suspendre le temps de chaque
écriture.

**Mesure possible, reportée (décision du 2026-10-08), puis prévue à
l'étape 8a (décision du 2026-10-09).** L'affichage de
l'essai 03 tournerait pendant qu'on efface puis réécrit un secteur, avec
les compteurs de retards et de recalages, et un contrôle visuel. Cela
confirmerait ou non que l'image se rétablit seule après une ou deux trames.
Mais cela demande d'écrire en flash, ce que la règle n° 1 interdit aux
essais. La seule zone envisageable serait la fin du slot de test, au-delà de
l'image de l'essai. Cette exception n'est pas accordée : la question attend
l'étape OTA en Rust, et rien n'est fait d'ici là.

---

## Viabilité : ce qui reste à démontrer

Trois questions décident si Rust convient au projet. Les étapes ci-dessous y
répondent en partie ; le reste se conçoit à part.

1. **La charge radio sur l'écran.** Le BLE avec l'affichage est validé par
   l'étape 4 : scan, puis deux connexions GATT simultanées, avec le code en
   PSRAM. Le Wi-Fi est validé par l'étape 5, seul puis avec les deux liens
   BLE, à la marge de recopie près à 20 lignes. La vraie question est
   **UI + BLE + Wi-Fi** : elle ne se tranche qu'avec Slint, à l'étape 7.
   Le passage d'un mode radio à l'autre ne se vérifie que si cette
   combinaison ne tient pas.
2. **La stratégie d'UI : Slint, décidé le 2026-10-08** (section suivante).
   Reste à mesurer son rendu sur notre affichage (étape 6).
3. **La stratégie d'écriture en flash** : OTA, relais vers `ota_staging`,
   `otadata` et stockage persistant (section « Écritures en flash »).
   Étapes 8 (écran) et 9 (`sensors`).

Trois autres points à couvrir, qui ne remettent pas en cause la viabilité :

- **La couche réseau applicative** : serveur HTTP, WebSocket (miroir CAN),
  SNTP, client HTTP vers `coffeetracker`, provisioning par point d'accès.
  C'est surtout une question d'effort ; les serveurs WebSocket `no_std` sont
  les crates les moins mûres de la liste. L'étape 5 ne teste qu'un petit
  serveur HTTP.
- **Le partage du bus I2C de l'écran** entre tâches (CH422G, GT911,
  ADS1115) : le bus partagé d'`embassy-embedded-hal` suffit.

Ce qui ne présente pas de difficulté particulière :

- **Le CAN.** esp-hal 1.2.2 fournit un pilote TWAI. Sur l'écran, le CAN
  passe par GPIO19/20, les broches USB : esp-hal coupe le pad USB quand il
  les reprend (`usb_pad_enable`), et CAN_SEL reste à tenir haut par le
  CH422G. La compatibilité binaire est assurée par `protocol` et les
  vecteurs de test de `common/test`. Seul le relais du flash `sensors`
  demande une mesure, et il relève de la stratégie d'écriture en flash.
- **L'I2C** : GT911 lu en polling, ADS1115 (A0, A1, A2 à 16 SPS), CH422G
  (déjà fait), DimmerLink côté capteurs. Ce ne sont que des registres. Pour
  le dimmer, garder l'interdiction de la commande `0x03`, qui écrit dans son
  EEPROM.
- **Le reste de `sensors`** : l'interruption GPIO du débitmètre, la chauffe
  modulée sur une période d'une seconde, le verrou de 60 s en mémoire RTC
  persistante (`#[ram(unstable(rtc_fast, persistent))]`) et la cause du
  reset existent dans esp-hal.

---

## UI : Slint

**Décision du 2026-10-08 : l'UI de `screen` sera écrite en Slint.** Pas de
bindings vers LVGL.

### Pourquoi pas `embedded-graphics` seul

L'UI de coffeeflow est simple à dessiner : environ 2 400 lignes de LVGL,
des labels, des rectangles arrondis, deux boutons, quelques rangées en flex,
et une frise faite de rectangles et de chiffres. La police est Inter, en
7 tailles de 18 à 104 px, anticrénelée sur 4 bits. Ce que LVGL apportait
au-delà du dessin, c'est la couche du dessus : objets retenus, mise en page,
suivi des zones à redessiner, texte anticrénelé, routage du toucher.
`embedded-graphics` ne donne que des primitives en mode immédiat et des
polices bitmap 1 bit. Toute cette couche serait donc à écrire et à
maintenir. Slint la fournit.

### Ce que Slint apporte

- un langage déclaratif (`.slint`) avec des layouts horizontaux, verticaux
  et en grille, des propriétés liées aux données et des animations ;
- un rendu logiciel `no_std` qui ne redessine que les zones modifiées
  (`RepaintBufferType::ReusedBuffer`), ou qui gère deux framebuffers
  (`SwappedBuffers`) ;
- les polices TTF pré-rendues au build (`EmbedForSoftwareRenderer`) : Inter
  s'utilise directement, sans conversion ;
- le même `.slint` qui tourne sur le Mac avec aperçu en direct, ce qui permet
  de mettre l'UI au point sans flasher.

Le cœur de Slint ne dépend pas d'esp-hal : aucun couplage de versions.

### Règles d'intégration

- **On garde notre pilote d'affichage** (bounce buffers, recopie par
  interruption, essais 02b à 04). Slint dessine dans notre framebuffer
  PSRAM avec `SoftwareRenderer::render(fb, 800)`, depuis le cœur 0, comme
  le dessin de l'essai 03.
- **On ne reprend pas le pilote d'affichage de `mcu-board-support`** pour
  les cartes ESP32-S3 RGB (`esp32_s3_lcd_ev_board`). Il envoie le
  framebuffer PSRAM en DMA direct par `Dpi::send`, l'approche que l'essai 02
  a écartée. De ce crate, on ne reprend que la façon d'implémenter
  `Platform`, le temps et le toucher.
- **Double framebuffer** avec `SwappedBuffers` : l'échange est un changement
  de pointeur, fait quand la recopie repasse au bloc 0 (étape 3). Coût :
  768 Ko de PSRAM.
- **Limites du rendu logiciel** : pas d'ombre portée, pas de rotation ni de
  mise à l'échelle, pas de `clip` combiné à `border-radius`, pas de contour
  de texte. L'UI actuelle n'en utilise aucune.

### Références dans le clone `tmp/rust/slint`

Exemples et démos sous licence MIT, donc réutilisables :

- `examples/mcu-embassy` : 800×480, embassy, tâche de rendu, double
  framebuffer en `SwappedBuffers`, et un simulateur sur PC
  (`bin/ui_simulator.rs`) qui partage l'UI et le contrôleur avec la version
  MCU. C'est le gabarit de l'essai 6.
- `demos/energy-monitor` : tuiles (`widgets/tile.slint`) et histogrammes
  (`widgets/bar_chart.slint`), proches de nos tuiles et de la frise ;
  fenêtre MCU dédiée (`mcu_window.slint`) ; `cache-rendering-hint`.
  `bar_chart.slint` utilise `clip` avec `border-radius`, à ne pas copier.
- `demos/printerdemo_mcu` : Inter embarquée au build (`ui/fonts`), et
  `bench.rs`, qui mesure le rendu complet et ligne par ligne sur le Mac.
- `api/rs/slint/mcu.md` : guide de Slint sur microcontrôleur (`Platform`,
  boucle, rendu).

### Licence

Le framework Slint est sous GPLv3, sous licence commerciale, ou sous une
licence gratuite « Royalty-free » qui exclut les systèmes embarqués
(`LICENSE.md` du clone). La page de prix de Slint indique toutefois
qu'aucune redevance n'est due pour un projet personnel, non commercial ou
open source. Coffeeflow est un projet personnel, et Slint ne limite rien
techniquement : la question ne se pose pas. **À reprendre si le projet
devait un jour être distribué**, ce qui n'est pas prévu.

---

## Étapes initiales

Chaque étape a un critère de réussite. On ne passe à la suivante qu'une fois
ce critère atteint. Les résultats (chiffres, captures, impasses) se notent
dans la section « Journal » en bas de ce document.

### 1. `01-hello-uart` — console série

Squelette `esp-generate` sans modification, à part le runner et
`esp-println/uart`. Il affiche un compteur chaque seconde.

- **Prouve :** la chaîne complète (build, `save-image`, écriture dans le slot
  de test, bascule, démarrage par le bootloader IDF installé, console, retour
  sur coffeeflow).
- **Réussi si :** le compteur s'affiche dans `espflash monitor`, et le retour
  sur coffeeflow redémarre l'écran normalement. Le décodage d'une backtrace
  sera vérifié au premier vrai panic.
- **Ne touche pas** au CH422G.

### 2. `02-lcd-static` — contenu fixe à l'écran

Init du CH422G (octet complet, CAN_SEL=0), mise sous tension de la dalle,
PSRAM, framebuffer 800×480 RGB565 en PSRAM, puis affichage d'une mire fixe :
bandes de couleur, dégradés R, V, B et gris, quadrillage tous les 40 px,
cadre d'un pixel sur les bords, carré de couleur dans chaque coin.

- **Prouve :** le brochage, les timings, la PSRAM octale, et l'alimentation du
  DPI d'esp-hal depuis un framebuffer PSRAM.
- **Réussi si :** la mire est stable pendant une minute, sans décalage ni
  saut, cadre intact sur les quatre bords.

**Résultat (2026-10-08) : DMA direct depuis la PSRAM écarté.** L'essai
`02-lcd-static` lit le framebuffer en DMA, par une chaîne circulaire de
191 descripteurs de 4 032 octets (rafales de 64 octets), sans bounce buffer.
Brochage, couleurs, ordre des bits et timings sont justes. Mais **chaque
défaut de cache sur le code en flash fait perdre des pixels pour toujours**.
La flash (en DIO) et la PSRAM partagent le bus : pendant un chargement de
ligne de cache, le DMA attend, la FIFO du LCD (16 entrées, ~0,5 µs à 16 MHz)
se vide, et la chaîne circulaire, qui ne se recale jamais, garde le retard.
Un premier `info!` a coûté ~40 px. La preuve est dans l'essai : avec le code
de la boucle préchargé en cache, la mire reste alignée, puis saute quand un
témoin exécute du code froid à 30 s. Dans le vrai firmware, les défauts de
cache sont permanents (piles radio, rodata, polices). Un recalage à chaque
VSYNC limiterait chaque perte à une trame, mais l'image scintillerait.

Ce qui reste acquis de cet essai :

- correspondance des timings IDF → `FrameTiming` d'esp-hal :
  `horizontal_blank_front_porch` = pulse + back porch d'IDF (48 + 88),
  `horizontal_total_width` = 976 ; en vertical, 3 + 32 et 528. PCLK sur front
  descendant = `Phase::ShiftHigh`, `Polarity::IdleLow`. HSYNC et VSYNC au
  repos haut, DE au repos bas ;
- `lcd_always_out_en`, posé par IDF et pas par esp-hal, se pose par le PAC
  `esp32s3` ;
- **`Dpi::send` démarre dans le mauvais ordre** : il lance le DMA puis vide
  la FIFO du LCD, ce qui jette les premiers pixels. Il faut refaire le
  démarrage dans l'ordre d'IDF (`lcd_rgb_panel_start_transmission`) : LCD
  arrêté, FIFO vidée, DMA remis à zéro puis relancé, 1 µs, LCD démarré
  (`restart_in_phase` dans l'essai) ;
- le CPU écrit le framebuffer à travers le cache : il faut un writeback
  (`rom_Cache_WriteBack_Addr`) avant que le DMA le lise.

### 2b. `02b-lcd-bounce` — bounce buffers en SRAM

Décision du 2026-10-08 : l'architecture de coffeeflow, à savoir le
framebuffer en PSRAM et **deux bounce buffers de 20 lignes** en SRAM interne
(2 × 32 000 octets). Le DMA ne lit plus que la SRAM, que le bus flash/PSRAM ne
bloque pas. Une interruption de fin de buffer recopie les 20 lignes suivantes
du framebuffer, avec 1,22 ms de marge (744 interruptions/s). On recale à
chaque VSYNC par sécurité : un retard ne gâche alors qu'une trame.

Il faut écrire un `DmaTxBuffer` maison (anneau de descripteurs sur les deux
buffers, EOF sur chacun), l'interruption de recopie, le suivi de la ligne en
cours et le recalage au VSYNC. Pour le recalage, IDF sur ESP32-S3 relance le
DMA à 17 pixels (`LCD_LL_FIFO_DEPTH + 1`) après le début de la trame, parce
que la FIFO du LCD en contient déjà. On compte les recopies en retard.

- **Réussi si :** même mire que `02-lcd-static`, stable, y compris avec le
  témoin de code froid, et zéro recopie en retard.

**Résultat (2026-10-08) : validé, 20 lignes retenues.** Mire alignée dès le
démarrage, immobile, y compris quand le témoin de code froid passe à 30 s.
Console sur 70 s : 31 trames/s, zéro recalage, recopie la plus lente à
607 µs sur 1 220 µs disponibles (~53 Mo/s lus depuis la PSRAM à travers le
cache). Un seul « retard » compté avant 10 s, puis plus aucun. L'essai 04 a
montré que ce compteur donnait de fausses alarmes (voir le résultat de
l'étape 4) : ce retard en était très probablement une.

Coût de 20 lignes : 64 Ko de SRAM interne, et au pire 24 × 31 × 607 µs ≈ 45 %
d'un cœur pour les recopies. Ce compromis SRAM/CPU est retenu. Pour alléger
le CPU, deux pistes restent ouvertes : précharger dans le cache le bloc
suivant, comme le fait IDF (`cache_hal_preload`), ou faire la recopie par un
DMA mémoire à mémoire. Pour l'instant, les interruptions et la recopie
s'exécutent depuis la flash.

### 3. `03-lcd-dynamic` — contenu qui change

Même base, avec un compteur et un rectangle qui se déplace, redessinés en
continu. Le dessin se fait avec `embedded-graphics` sur le framebuffer. On
compte les sous-alimentations détectées (bounce buffer en retard, ou EOF
manqué).

- **Prouve :** la marge de bande passante sous charge réelle, l'affinité de
  cœur (dessin et ISR sur le même cœur, comme coffeeflow), et la
  resynchronisation au VSYNC si la recopie prend du retard.
- **Variante :** la même chose sur deux cœurs, pour vérifier que le Rust
  reproduit le problème connu, et donc que la mesure le détecte.
- **Réussi si :** aucun décalage visible et zéro sous-alimentation en
  une minute, avec redessin continu de plusieurs zones.
- Le choix d'une bibliothèque d'UI se fait **après** cette étape, une fois
  les chiffres connus. Fait le 2026-10-08 : Slint (section « UI : Slint »).

**Résultat (2026-10-08) : validé sur un cœur.** L'essai `03-lcd-dynamic`
reprend l'affichage de 02b. Trois rectangles rebondissent sans pause sur la
mire, dont le fond est restauré à chaque pas, et un compteur texte
(`embedded-graphics`, FONT_10X20) est réécrit à chaque pas.

| | Même cœur (défaut) | Deux cœurs (`--features two-cores`) |
|---|---|---|
| Trames/s | 31 | 31 |
| Recopies en retard | 0 | ~46/s (≈ 6 %) |
| Recalages | 0 | 0 |
| Recopie la plus lente | 885 µs sur 1 220 | 1 320 µs |
| Pas de dessin/s | 38 | 67 |
| Écriture | ~5,2 Mo/s | ~9 Mo/s |

- Sur **un cœur**, la mire reste immobile, le compteur est net, et aucune
  recopie n'est en retard, même dès le démarrage. Cela confirme que le retard
  isolé de 02b venait du journal : un `info!` qui attend la FIFO de l'UART
  bloque les interruptions, sous la section critique d'esp-println. Il faut
  donc des lignes courtes et espacées tant que l'affichage tourne.
- La recopie passe de 607 à 885 µs, alors que l'interruption a priorité sur
  le dessin. Hypothèse : le dessin remplit le cache de lignes modifiées, que
  la recopie doit réécrire en PSRAM avant de lire. Il reste 27 % de marge.
- La recopie prend 45 à 65 % du cœur. Le débit de dessin mesuré est donc
  celui d'un demi-cœur, avec une restauration de fond volontairement lente
  (calcul de la mire pixel par pixel). Ce n'est pas un débit de référence
  pour la PSRAM.
- Sur **deux cœurs**, le problème connu de coffeeflow est reproduit, et le
  compteur de retards est validé. La mire le masque presque : une recopie en
  retard montre les lignes situées 40 px plus haut, et la mire se répète
  tous les 40 px. Sur du texte, ce seraient des bandes qui sautent.
- Les rectangles déchirent : c'est l'effet d'un seul framebuffer. Avec les
  bounce buffers, un double framebuffer ne demande qu'un changement de
  pointeur, au moment où la recopie repasse au bloc 0 (pas au VSYNC : les
  blocs 0 et 1 de la trame suivante sont déjà chargés). Coût : 768 Ko de
  PSRAM. À décider avec le choix de la bibliothèque d'UI.

**Répartition des cœurs retenue :** cœur 0 pour l'affichage (recopie et
dessin de l'UI), cœur 1 pour la radio (BLE, ou Wi-Fi et HTTP). L'UI de
coffeeflow anime peu : la frise pendant l'infusion, quelques valeurs. À
vérifier à l'étape 6 : les défauts de cache du code radio, sur le cœur 1,
partagent le bus avec la recopie, comme le dessin ici. Si la marge de 27 %
ne suffit pas : 30 ou 40 lignes, ou le code de recopie en SRAM. Reste aussi
à voir si esp-radio peut être cantonné au cœur 1.

### 4. `04-ble-display` — BLE avec l'affichage

Décision du 2026-10-08 : le scan BLE se teste directement avec l'affichage
de l'essai 03, pour gagner une étape. On mesure ainsi tout de suite ce que la
radio coûte à la recopie, au lieu d'attendre `06-all`.

L'essai part d'une copie de `03-lcd-dynamic`, variante un cœur, sans
changement d'affichage. Le cœur 0 garde la recopie et le dessin continu des
rectangles et du compteur : c'est le pire cas de charge PSRAM, et on le garde
volontairement. On y ajoute `esp-radio` + `trouble-host` en central, sur le
modèle de `examples/ble/scanner`, **sur le cœur 1**.

- **Cœur de la radio.** On regarde d'abord dans les sources d'esp-rtos et
  d'esp-radio comment la cantonner au cœur 1. Si c'est impossible, on lance
  l'essai quand même, on note sur quel cœur tournent les tâches et
  l'interruption radio, et la mesure dit si c'est acceptable.
- **Scan actif**, pour recevoir les noms dans les réponses de scan. Mêmes
  critères que coffeeflow : un nom contenant `ACAIA` ou `LUNAR`, sans tenir
  compte de la casse, pour la balance (`ble_scale.cpp`). Le nom exact
  `SCACE` pour la sonde (`common/scace_ble.hpp`). Les autres annonces sont
  ignorées.
- **Journal.** Au plus une ligne par appareil reconnu toutes les 10 s (nom,
  adresse, RSSI), plus la ligne de compteurs de l'essai 03. Les lignes
  restent courtes et espacées : un `info!` qui attend la FIFO de l'UART
  bloque les interruptions (étape 3).
- **Mémoire** (étape 6). Au démarrage puis toutes les 10 s : SRAM interne
  libre et plus grand bloc libre (`esp_alloc::HEAP.stats()`), taille de
  l'image. Le tas interne prend la taille des exemples BLE d'esp-radio
  (72 Ko), en plus des 64 Ko de bounce buffers.
- **Prouve :** que la pile BLE Rust démarre sur cette carte et voit les deux
  appareils réels, et que l'affichage tient pendant le scan.
- **Réussi si :** l'Acaia Lunar et la SCACE apparaissent. Pendant le scan,
  les critères de l'étape 3 tiennent : mire immobile, 0 recopie en retard,
  0 recalage. La recopie la plus lente est notée à côté des 885 µs de
  l'essai 03.
- **Si l'affichage ne tient pas :** on ne corrige pas tout de suite. On note
  les chiffres, puis on choisit entre plus de lignes de bounce buffer, le
  code de recopie en SRAM, et le placement de la radio.
- **Puis :** connexions GATT **simultanées** à la Lunar et à la SCACE, comme
  coffeeflow, avec lecture de leurs notifications et le heartbeat de
  l'Acaia tenu. Mêmes compteurs, et mêmes critères d'affichage pendant la
  connexion et pendant la réception.

Matériel : balance et SCACE allumées, à portée. Machine au repos.

**Résultat du scan (2026-10-08) : validé en régime établi.** L'Acaia
(`ACAIAL-C1EF4C9`) et la SCACE apparaissent, avec 70 à 150 annonces reçues
par seconde. La mire reste immobile.

- **Radio sur le cœur 1.** `BleConnector::new` est appelé sur le cœur 1 :
  esp-hal y active l'interruption `RWBLE` (lu dans la matrice
  d'interruptions ; `BT_BB` n'est activée sur aucun cœur). La tâche du
  contrôleur y est épinglée par `Config::with_task_cpu(Cpu::AppCpu)`, et
  l'hôte trouble-host tourne dans un exécuteur embassy du cœur 1. Seule la
  tâche « timer » d'esp-radio n'est pas épinglée, et rien ne permet de
  l'épingler sans patcher le crate. Le cœur 1 ne journalise rien lui-même :
  esp-println écrit sous une section critique partagée par les deux cœurs.
- **Régime établi :** 31 trames/s, 0 recopie en retard et 0 recalage sur
  2 min 30. Recopie la plus lente entre 882 et 940 µs (885 µs pour
  l'essai 03), soit environ 280 µs de marge sur 1 220. L'interruption entre
  à l'heure, à moins de 100 µs près. Les PC interrompus relevés tombent tous
  dans le dessin : aucune section critique ne la masque.
- **Démarrage :** 6 vrais retards pendant l'initialisation BLE (calibration
  PHY, démarrage du contrôleur). La recopie monte jusqu'à 1 757 µs, et
  l'interruption entre jusqu'à ~600 µs en retard. Cela ne se produit qu'une
  fois. Parade pour `screen` : démarrer la radio avant l'affichage.
- **Le détecteur de retard des essais 02b et 03 était faux.** Malgré la doc
  du PAC (« descripteur en cours »), `OUT_DSCR` a **deux descripteurs
  d'avance** sur celui que lit le DMA. Un histogramme relevé à l'entrée de
  l'interruption de fin de buffer le montre : sur 114 000 entrées, toutes
  sauf 4 (celles du démarrage) trouvent `OUT_DSCR` au troisième descripteur
  de l'autre buffer. L'ancien détecteur sonnait dès que la recopie dépassait
  6 × 152 ≈ 912 µs, et non 1 220 µs. Le détecteur corrigé prend le
  descripteur situé deux crans avant `OUT_DSCR`. Les durées de recopie
  mesurées dans 02b et 03 restent valables.
- **Code de recopie en SRAM : sans effet.** L'interruption, la recopie et le
  VSYNC passés en `#[esp_hal::ram]` (1 420 octets d'IRAM) laissaient la
  recopie à 900–980 µs. Le surcoût de la radio ne vient donc pas des défauts
  de cache sur le code de la recopie, mais de la lecture de la PSRAM, sur un
  cache et un bus partagés avec le cœur 1. Retour en flash, pour garder la
  SRAM. À remesurer sous la charge de LVGL.
- **Mémoire.** Le BLE prend 35,8 Ko du tas interne de 72 Ko (37,9 Ko
  restent libres), dont deux piles de tâches de 8 Ko (contrôleur et timer).
  Le plus grand bloc libre n'est pas mesurable avec esp-alloc 0.11. La pile
  du cœur 1 utilise 2,7 Ko sur 16 Ko. Image : 369 Ko.
- **Répartition de la SRAM interne.** esp-hal donne à la pile du cœur 0
  **tout ce qui reste** (195 Ko), dont elle n'utilise que 1,6 Ko, alors que
  le tas est un bloc fixe. Le reste : 64 Ko de bounce buffers, 43,5 Ko de
  code en IRAM (dont 25 Ko de correctifs du contrôleur BLE), 17 Ko de
  `.data`, 16 Ko de pile du cœur 1, et le tas de 72 Ko pris sur la zone du
  bootloader. Le libre comparable aux 70 à 80 Ko de coffeeflow (BLE actif,
  avec LVGL) est donc d'environ 38 + 193 ≈ 230 Ko. Pour l'étape 6, il faudra
  rendre une partie de la pile du cœur 0 au tas, pour mesurer.

**Résultat des connexions GATT (2026-10-08) : validé, avec le code en
PSRAM.** L'essai `04b-ble-gatt` reprend 04. Un scan actif trouve la Lunar et
la SCACE, puis s'arrête. Chaque lien se connecte ensuite par adresse : le
contrôleur cherche l'appareil lui-même, sans scan de l'hôte, et le lien se
reconnecte seul. Intervalle demandé : 30 à 50 ms, comme NimBLE ; les deux
appareils acceptent 50 ms.

- **BLE.** Client GATT de trouble-host 0.7 : découverte par UUID, abonnement,
  écritures sans réponse. La Lunar répond au service legacy `0x1820/0x2A80`,
  avec 9 notifications/s et le heartbeat tenu toutes les 2,5 s. La SCACE
  envoie 10 trames/s, sans trame manquée d'après les numéros de séquence.
  Balance éteinte : elle envoie `0x20` avant de couper. Appareil éteint :
  chaque tentative expire en 10 s, puis le lien revient seul. La file de
  notifications de trouble-host n'a qu'une place : une trame non lue à temps
  serait écrasée, d'où le compteur de trames manquées.
- **Affichage, code en flash : échec.** Avec les deux liens et le dessin
  continu de 03, 0,1 à 0,25 recopie en retard par seconde, et la recopie la
  plus lente monte à 1 944 µs (940 µs avec le scan seul). L'interruption
  entre à l'heure : c'est la recopie qui ralentit. Le cache est partagé par
  les deux cœurs (datasheet, §3.1.5), et la flash et la PSRAM partagent le
  même bus. Or un défaut de cache servi par la flash DIO occupe ce bus
  environ 1,9 µs, contre environ 0,4 µs en PSRAM octale. Avec les
  connexions, le code radio du cœur 1 fait bien plus de défauts que pendant
  le scan.
- **Code en PSRAM.** coffeeflow exécute son code depuis la PSRAM
  (`CONFIG_SPIRAM_XIP_FROM_PSRAM`, et `SPIRAM_FETCH_INSTRUCTIONS` et
  `SPIRAM_RODATA`). esp-hal 1.2.2 n'en a pas d'équivalent ;
  `src/xip.rs` le fait au démarrage, avant tout autre usage de la PSRAM. Il
  copie les pages de flash de l'app au début de la PSRAM, puis redirige leurs
  entrées MMU, caches suspendus, depuis l'IRAM. Le bus d'instructions et
  celui de données partagent la table MMU : une redirection déplace le code
  et les rodata de la page. La fonction ROM `Cache_Flash_To_SPIRAM_Copy`
  n'est pas utilisée : IDF corrige un bug de la ROM sur le compte des pages,
  et la dernière entrée sert au bootloader. 7 pages (448 Ko) en 42 ms ; le
  framebuffer suit.
- **Dessin à 10 Hz.** Le dessin continu de 03 (5,2 Mo/s écrits, doublés par
  l'allocation en écriture du cache) n'est pas représentatif. La scène
  avance désormais à 10 Hz, soit 1,35 Mo/s, l'ordre de grandeur de l'UI
  pendant une infusion. `draw-continuous` rend le pire cas ;
  `exec-from-flash` désactive le XIP.
- **Résultat :** 0 retard et 0 recalage, y compris pendant l'initialisation
  BLE (7 retards avant). Recopie la plus lente : 870 à 892 µs, avec ou sans
  liens, soit environ 330 µs de marge avec 20 lignes. Le XIP et le dessin à
  10 Hz ont changé ensemble, et leurs parts ne sont pas séparées : ce n'est
  pas utile, puisque le XIP est retenu de toute façon. **Les bounce buffers
  restent à 20 lignes**, pour garder la SRAM interne.
- **Mémoire.** Tas interne libre inchangé (38 Ko), pile du cœur 1 à 7,4 Ko
  sur 16. Image : 426 Ko.
- Les essais d'affichage suivants partent de 04b, XIP compris.

### 5. `05-wifi-ap` — Wi-Fi

Décision du 2026-10-08 : comme pour le BLE à l'étape 4, le Wi-Fi se teste
**avec l'affichage qui tourne**, puis **combiné au BLE**.

L'essai part d'une copie de `04b-ble-gatt` : affichage, XIP, scène à 10 Hz
et rapport inchangés sur le cœur 0, radio sur le cœur 1. Il se fait en deux
temps, choisis par une feature :

1. **Wi-Fi seul** (par défaut). Point d'accès avec un SSID et un mot de
   passe **propres à l'essai**, écrits dans le source de l'essai, sans
   rapport avec ceux de coffeeflow et sans lecture de NVS. DHCP, et un petit
   serveur HTTP sur le modèle de `examples/wifi/embassy_access_point`
   (détail plus bas). Tas interne à la taille de l'exemple Wi-Fi
   d'esp-radio (100 Ko), pris sur la pile du cœur 0, qui n'utilise que
   3 Ko de ses 189.
2. **BLE + Wi-Fi** (feature `ble`). Les deux liens GATT de 04b et le point
   d'accès en même temps, avec la coexistence d'esp-radio (feature `coex`,
   modèle : `examples/wifi/embassy_coex`). Tas interne de 128 Ko, la taille
   des exemples pour les deux radios.

**Serveur HTTP et charge.** Le client est l'iPhone, connecté au point
d'accès. Pas le Mac : en rejoignant l'AP, il perdrait l'accès à Internet,
et la session Claude Code avec. Le serveur sert deux chemins :

- `GET /` : une page HTML, avec son JavaScript, écrite dans le source de
  l'essai. Elle appelle `GET /stats` toutes les 200 ms (5 Hz) et affiche la
  dernière réponse, le nombre de requêtes, d'erreurs et de délais dépassés
  (1 s), et la latence moyenne et maximale. Un bouton passe en mode « sans
  pause » : la requête suivante part dès que la précédente a répondu.
- `GET /stats` : un petit JSON avec les compteurs du rapport (affichage,
  tas interne, liens BLE en combiné).

Mesures, dans le rapport et dans `/stats` : les compteurs d'affichage de
04b, le tas interne libre et son minimum, la pile du cœur 1, la taille de
l'image ; côté BLE, les mêmes lignes qu'en 04b ; côté HTTP, les requêtes
servies par seconde. La charge dure au moins une minute à 5 Hz, puis une
minute sans pause.

- **Prouve :** la pile Wi-Fi et réseau (esp-radio + embassy-net) sur cette
  carte, sans dépendre des identifiants du réseau domestique, et ce que le
  Wi-Fi, seul puis avec le BLE, coûte à la recopie et à la SRAM interne.
- **Réussi si :** dans les deux temps, 0 recopie en retard et 0 recalage,
  hors démarrage de la radio, pendant la charge HTTP, et la page ne compte
  ni erreur ni délai dépassé à 5 Hz. En combiné, la SCACE garde ses
  10 trames/s sans trame manquée, et la balance ses pesées et son heartbeat,
  toujours pendant la charge HTTP.
- **Si l'affichage ne tient pas :** noter les chiffres, sans corriger tout
  de suite (même règle qu'à l'étape 4). Les 40 lignes de bounce buffer
  restent possibles, mais elles coûtent 64 Ko de SRAM interne, celle que
  réclame justement la combinaison.
- **Si la combinaison ne tient pas en SRAM :** noter où, et ce qu'il manque.
  Le fonctionnement actuel de coffeeflow, une radio à la fois, reste la
  référence (bascule, étape 7).
- **Portée du temps combiné (décision du 2026-10-08).** BLE et Wi-Fi
  ensemble, c'est préférable mais pas exigé : il est acceptable qu'ils ne
  tiennent pas ensemble, faute de SRAM. Le temps combiné de l'étape 5 ne
  tranche donc pas, que la combinaison passe ou non : la mise en page de la
  SRAM changera avec Slint (tas, tampons de rendu). La réponse viendra de
  l'étape 7, avec Slint. L'étape 5 sert à mesurer les coûts, sans décider.
- Un mode station viendra plus tard. Il faudra alors décider comment fournir
  les identifiants sans les lire depuis NVS (fichier ignoré par Git, comme
  `secrets.h`).

**Réalisation (2026-10-08).** L'essai `05-wifi-ap` reprend 04b. Le code
BLE de 04b est passé tel quel dans `src/bin/ble.rs`, compilé seulement avec
la feature `ble`. Le point d'accès est dans `src/bin/wifi.rs`, et
`autobins = false` empêche Cargo de prendre ces deux fichiers pour des
binaires.

- **Point d'accès :** SSID `coffeeflow-essai-05`, WPA2, mot de passe
  `essai-rust-05`, canal 1, au plus 4 stations. Adresse fixe `192.168.2.1/24`,
  serveur DHCP d'`edge-dhcp` comme dans l'exemple. Pas de DNS : l'iPhone
  signale « pas d'Internet », ce qui ne gêne pas.
- **HTTP :** port 80, 4 connexions servies en parallèle, en keep-alive.
  `/stats` rend les compteurs cumulés : trames, retards, recalages, recopie
  la plus lente, tas libre et minimum, pile du cœur 1, stations, requêtes,
  erreurs HTTP. En combiné, il y ajoute l'état de chaque lien BLE. La page
  compte ses propres requêtes, erreurs, délais dépassés et latences, et
  affiche les retards et recalages survenus depuis son ouverture. Le bouton
  « Remise à zéro » repart de zéro après un changement de mode.
- **Ordre de démarrage, sur le cœur 1 :** le BLE d'abord, en combiné, comme
  l'exemple `embassy_coex`, puis le contrôleur Wi-Fi. Ce dernier épingle sa
  tâche sur le cœur qui le crée (`wifi_task_core_id = Cpu::current()`).
- **Interruption Wi-Fi : à vérifier.** Sur ESP32-S3, `set_intr` d'esp-radio
  beta.1 force la matrice du **cœur 0** (`intr_matrix_set(0, …)`), puis
  `set_isr` active `WIFI_MAC` et `WIFI_PWR` sur le cœur courant, par
  esp-hal. La ligne `pile0` du rapport affiche donc la matrice des deux
  cœurs pour `mac` et `pwr` (16 = désactivée). Si le cœur 0 reçoit
  l'interruption, cela se verra dans les retards de recopie.
- **Tas interne :** 72 Ko pris sur la zone du bootloader, plus 28 Ko
  (Wi-Fi seul) ou 56 Ko (combiné) en `.bss`, pris sur la pile du cœur 0.
- **Images :** 637 Ko pour le Wi-Fi seul, 878 Ko pour le combiné. Le blob
  Wi-Fi place 32 Ko de code en IRAM (`.rwtext.wifi`), et 51 Ko en combiné.
  `.bss` : 151 Ko pour le Wi-Fi seul, 190 Ko pour le combiné.

### 6. `06-slint-display` — Slint sur notre affichage

Décision du 2026-10-08 : l'ancienne étape 6 (« pression mémoire ») est
retirée. Chaque essai a relevé la mémoire au fil de l'eau, et le combiné
BLE + Wi-Fi de l'étape 5 en est le pire cas radio. La seule question qui
lui restait propre, le passage d'un mode radio à l'autre, ne se pose que si
l'étape 7 ne tient pas : elle y est décrite.

Slint se mesure seul, pour que l'étape 7 sache ce qui revient à l'UI et ce
qui revient à la radio. L'essai reprend l'affichage de l'essai 04b (XIP
compris), sans la radio, sur la structure de `examples/mcu-embassy`, avec
le simulateur sur Mac dès le départ. Slint dessine dans le framebuffer
PSRAM une maquette de l'accueil : la barre du haut, six tuiles, de grands
chiffres Inter en 104 px, et la frise. Les valeurs changent à 10 Hz, comme
pendant une infusion. Un bouton fait basculer entre deux écrans, pour
mesurer un redessin complet.

- **Mesure :** le temps de rendu partiel (mise à jour des valeurs) et
  complet (changement d'écran), les compteurs de recopie de l'essai 04, la
  taille de l'image, et le tas utilisé (interne et PSRAM). D'abord en
  `ReusedBuffer`, puis en `SwappedBuffers`, pour les déchirures.
- **Réussi si :** 0 recopie en retard et 0 recalage pendant une minute ; une
  mise à jour des valeurs rendue en moins de 50 ms, ce qui laisse tenir
  10 Hz avec de la marge ; un changement d'écran rendu en moins de 300 ms ;
  une image qui tient dans le slot de 3 500 Ko.
- **Si le rendu est trop lent :** noter les chiffres avant de corriger.
  Leviers dans l'ordre : réduire les zones redessinées (valeurs isolées
  dans leurs composants, `cache-rendering-hint`), puis placer le tas de
  Slint en SRAM interne plutôt qu'en PSRAM.

**Réalisation (2026-10-08).** L'essai `06-slint-display` reprend 05 sans la
radio : affichage, XIP et rapport de 04b, tout sur le cœur 0. Slint 1.18.1
(crates.io) dessine avec `SoftwareRenderer::render(fb, 800)` dans le
framebuffer PSRAM que lit la recopie.

- **Deux écrans au lieu d'un.** La maquette reprend les deux écrans réels
  de coffeeflow, plutôt qu'un accueil composite où les 104 px ne tiennent
  pas avec la frise et les tuiles. L'écran d'infusion porte la barre du
  haut, la frise, les six tuiles et la tuile de fonctionnement ; l'accueil
  porte la barre du haut et le poids en 104 px. « accueil » et « infuser »
  changent d'écran : l'ancien est détruit, le nouveau créé et redessiné en
  entier. Une infusion simulée de 50 s (`src/demo.rs`) fait avancer toutes
  les valeurs, la frise comprise, à 10 Hz.
- **Inter** : `Inter-24pt-Regular.ttf` de `printerdemo_mcu` (OFL, sous-ensemble
  Latin-1), en 18, 26, 28, 32 et 104 px. Slint pré-rend chaque taille en
  alpha 8 bits, mais seulement pour l'ASCII et les caractères des littéraux
  du `.slint` : « ° » et « É », que seul le code Rust affiche, sont portés
  par des valeurs par défaut de `Live`. Les rodata passent à 547 Ko.
- **Tactile** : GT911 lu en polling toutes les 20 ms, sur le bus du CH422G,
  avec le décalage de 10 px de coffeeflow.
- **Tas** : la région PSRAM (après le code et les framebuffers) est ajoutée
  en premier, et esp-alloc sert chaque allocation depuis la première région
  qui a la place : Slint vit en PSRAM. Les 72 Ko récupérés du bootloader
  suivent. Pas de tas en `.bss`.
- **ReusedBuffer** (par défaut) : Slint dessine dans le framebuffer
  affiché, d'où des déchirures possibles. **SwappedBuffers** (feature
  `swapped-buffers`) : un second framebuffer, échangé par l'interruption au
  bloc 0 de la trame suivante ; la boucle de rendu attend l'échange avant de
  réécrire l'ancien.
- **Mesures du rapport** : rendus partiels (nombre, moyenne, maximum,
  pixels redessinés) et complets (dernier, maximum) ; un changement d'écran
  compte un rendu complet en ReusedBuffer, deux en SwappedBuffers. Puis le
  tas PSRAM et interne, et la pile du cœur 0.
- **Simulateur** : `sim/`, crate hôte à part (toolchain stable, cible du
  Mac) qui compile le même `.slint` et le même `demo.rs`. `cargo run
  --release` ouvre la fenêtre ; `snapshot` écrit une image PPM ;
  `bench` mesure le rendu sur le Mac : 0,4 ms en complet, 0,24 ms en
  partiel sur l'écran d'infusion, avec 157 000 pixels redessinés en
  moyenne (41 % de l'écran). Slint fusionne les zones modifiées en
  quelques rectangles : la frise, la barre du haut et les tuiles
  deviennent de grandes bandes.
- **Image** : 989 Ko en ReusedBuffer.

**Résultat en ReusedBuffer (2026-10-08) : la recopie tient, le rendu de
l'écran d'infusion est trop lent, et l'écran clignote.** Essai dans `ota_0`
(`otadata` `0x5f`/`0x5e`), 105 s d'observation, 6 changements d'écran au
doigt.

| Critère | Mesuré | Verdict |
|---|---|---|
| Recopie : 0 retard, 0 recalage | 0 et 0 ; recopie la plus lente 921 µs, 31 trames/s | tenu |
| Rendu partiel < 50 ms, infusion | moyenne 90 à 106 ms, maximum 141 ms, 190 000 à 200 000 px par rendu | échec |
| Rendu partiel < 50 ms, accueil | moyenne 10 à 43 ms, maximum 45 ms en régime établi | tenu, de justesse |
| Changement d'écran < 300 ms | 130 ms au premier rendu, ~72 ms ensuite | tenu |
| Image < 3 500 Ko | 989 Ko, 16 pages de XIP | tenu |

- **Clignotement.** En ReusedBuffer, Slint repeint le fond de chaque zone
  modifiée, puis les éléments, dans le framebuffer affiché. Avec 90 à
  140 ms par rendu sur la moitié de l'écran, la recopie envoie à l'écran
  des zones vides ou à moitié dessinées. C'est le défaut attendu de ce mode,
  amplifié par la lenteur du rendu.
- **Pourquoi c'est lent.** Slint ne garde que 3 rectangles modifiés par
  rendu (`PHYSICAL_REGION_MAX_SIZE = 3`). La barre du haut, la frise et les
  tuiles, qui changent toutes à 10 Hz, se fusionnent en grandes bandes : la
  moitié de l'écran par rendu, au lieu de quelques chiffres. Le débit est
  d'environ 2 Mpx/s, dessin direct en PSRAM, crates compilées en
  `opt-level = "s"`. Sur le Mac, `bench` mesurait le même ordre de zone
  (157 000 px).
- **Tactile** : 6 appuis comptés, 0 erreur.
- **Mémoire.** Tas PSRAM : 52 Ko au plus. Tas interne : jamais utilisé.
  Pile du cœur 0 : 8,5 Ko au plus.
- **Défaut de l'essai, corrigé depuis.** La boucle de rendu attendait un
  `Ticker` de 10 ms. Quand un rendu dure plus longtemps, le `Ticker` est en
  retard, rend la main tout de suite et ne cède jamais l'exécuteur : le
  rapport a sauté de 20 s à 44 s. Chaque tour se termine désormais par
  `Timer::after(10 ms)`. Les chiffres relevés restent valables.

**Variantes préparées pour la reprise (décision du 2026-10-08).** Trois
leviers, chacun une feature de l'essai, mesurés avec les mêmes lignes de
rapport que ReusedBuffer. On reprendra par ces trois variantes ; l'ordre
reste à choisir.

| Variante | Feature | Ce qu'on en attend |
|---|---|---|
| 1. SwappedBuffers | `swapped-buffers` | Plus de clignotement : Slint dessine hors écran. Chaque rendu reprend aussi la zone du rendu précédent, donc il devrait être plus lent. |
| 2. Ligne par ligne | `line-by-line` | Slint compose chaque ligne dans un tampon de 1,6 Ko en SRAM interne, recopié d'un bloc dans le framebuffer affiché. Plus de clignotement avec un seul framebuffer : la recopie ne voit que des lignes anciennes ou nouvelles, au pire une déchirure. Le dessin ne lit plus la PSRAM ; la recopie de 400 Ko par rendu de 200 000 px reste à payer. Le gain de vitesse n'est pas acquis : seule la carte tranchera. |
| 3. Fréquence réduite | `reduced-rate` | Les valeurs se mettent à jour moins souvent. Le rendu n'est pas plus rapide, mais il y en a moins, et chacun couvre moins de surface. |

- **Fréquence réduite : ce que fait la variante.** La table
  `REDUCED_RATES` de `src/demo.rs` donne une période et un décalage par
  groupe de valeurs : barre du haut (et poids de l'accueil), frise, six
  tuiles, tuile de fonctionnement. Par défaut, chaque groupe passe à
  2,5 Hz, avec des décalages tels qu'un rendu ne couvre qu'un groupe.
  Slint ne garde que trois zones modifiées par rendu : un seul groupe par
  rendu évite la fusion en grandes bandes. **La fréquence se décidera
  valeur par valeur** (décision métier) : en infusion, 2 à 4 Hz peuvent
  suffire. La table sert à essayer d'autres choix, sans toucher au reste.
- **La variante 3 se combine** avec les deux autres (`--features
  "reduced-rate,line-by-line"`, par exemple). Les variantes 1 et 2
  s'excluent.
- **Sur le Mac** (`cd sim && cargo run --release -- bench`), l'écran
  d'infusion :

  | Mode | Rendu partiel moyen | Maximum | Zone moyenne |
  |---|---|---|---|
  | `render`, 10 Hz | 250 µs | 479 µs | 157 000 px |
  | `render_by_line`, 10 Hz | 280 µs | 897 µs | 157 000 px |
  | `render`, fréquence réduite | 71 µs | 342 µs | 20 000 px |
  | `render_by_line`, fréquence réduite | 78 µs | 388 µs | 20 000 px |

  Le rendu ligne par ligne coûte ~10 % de plus sur le Mac, où le
  framebuffer est en RAM ordinaire ; sur la carte, il évite la PSRAM, donc
  le résultat peut s'inverser. La fréquence réduite divise la zone par
  huit. `cargo run --release -- reduced-rate` ouvre la fenêtre à
  fréquence réduite, pour juger à l'œil.
- **Images prêtes** sous `target/variants/` de l'essai, avec l'ELF
  correspondant pour `espflash monitor --elf` :
  `reused`, `swapped`, `line`, `reduced` (`.bin` et `.elf`), de 989 à
  998 Ko. `reused` est ReusedBuffer avec le correctif du `Ticker`, et
  `reduced` combine ReusedBuffer et la fréquence réduite. Pour une autre
  combinaison, `cargo build --release --features "…"`, puis
  `espflash save-image`. `target/` n'est pas versionné : après un
  `cargo clean`, il faut reconstruire. À la reprise : relire
  `otadata`, écrire le `.bin` choisi dans `ota_0`, basculer vers `ota_0`,
  puis ouvrir la console avec l'`.elf` du même nom.
- **Méthode** : celle de ReusedBuffer. Une minute sur l'écran d'infusion
  sans toucher, environ 5 changements d'écran, une minute sur l'accueil.
  Observer en plus le clignotement et les déchirures. En SwappedBuffers,
  relever `attente=`, l'attente de l'échange.

**Fin de session (2026-10-08) : retour sur coffeeflow.** `otadata`
`0x5f`/`0x60`, les deux `VALID`, `ota_1` actif. L'essai reste dans `ota_0` :
pour le reprendre, une bascule vers `ota_0` suffit.

**Résultat en SwappedBuffers (2026-10-09) : plus de clignotement, mais le
rendu de l'écran d'infusion reste trop lent.** Image reconstruite avant
l'essai (`main.rs` plus récent que `swapped.bin`), écrite dans `ota_0`,
`otadata` `0x61`/`0x60`, les deux `VALID`. 111 s d'observation, 8 appuis.

| Critère | Mesuré | Verdict |
|---|---|---|
| Recopie : 0 retard, 0 recalage | 0 et 0 ; recopie la plus lente 922 µs, 31 trames/s | tenu |
| Rendu partiel < 50 ms, infusion | moyenne 94 à 117 ms en régime établi, maximum 145 ms, 206 000 à 229 000 px par rendu | échec |
| Rendu partiel < 50 ms, accueil | moyenne 33 à 35 ms, maximum 45 ms, ~112 000 px | tenu, de justesse |
| Changement d'écran < 300 ms | rendu complet de 72 à 131 ms, puis au plus 32 ms d'attente de l'échange | tenu |

- **Clignotement** : aucun. Slint dessine hors écran, la recopie ne voit
  que des images finies.
- **Zone redessinée** : ~10 % de plus qu'en ReusedBuffer sur l'écran
  d'infusion, puisque chaque rendu reprend la zone du précédent. Sur
  l'accueil, ~112 000 px alors que seul le poids change. Le débit ne bouge
  pas : ~2 Mpx/s.
- **Attente de l'échange** : jusqu'à 32 ms, une trame entière, à chaque
  rendu. En infusion, il ne reste que ~7 rendus par seconde au lieu de 10.
- **Tactile** : 8 appuis, 0 erreur, mais 7 changements d'écran d'après les
  rendus complets (2 par changement, 16 au total, 2 au démarrage).
- **Mémoire** : tas PSRAM 52 Ko au plus, tas interne jamais utilisé, pile
  du cœur 0 8,6 Ko au plus. Comme en ReusedBuffer.

**Résultat ligne par ligne (2026-10-09) : plus de clignotement, mais pas
de gain de vitesse.** `line.bin` écrit dans `ota_0` (`otadata`
`0x61`/`0x60`, les deux `VALID`, pas de nouvelle bascule). 112 s
d'observation, 9 appuis.

| Critère | Mesuré | Verdict |
|---|---|---|
| Recopie : 0 retard, 0 recalage | 0 et 0 ; recopie la plus lente 913 µs, 31 à 32 trames/s | tenu |
| Rendu partiel < 50 ms, infusion | moyenne 92 à 108 ms en régime établi, maximum 147 ms, 188 000 à 204 000 px par rendu | échec |
| Rendu partiel < 50 ms, accueil | moyenne 29 à 35 ms, maximum 49 ms | tenu, de justesse |
| Changement d'écran < 300 ms | 77 ms, 133 ms au premier rendu | tenu |

- **Clignotement** : aucun. Une petite cassure au milieu des grands
  chiffres quand ils changent vite : la déchirure attendue, jugée
  acceptable.
- **Vitesse** : ~1,9 Mpx/s, comme en ReusedBuffer. Composer en SRAM
  interne n'accélère pas le dessin : la PSRAM n'était pas le goulot, ou la
  recopie des lignes reprend ce qu'on gagne. La zone est celle de
  ReusedBuffer, sans le report de SwappedBuffers.
- **Tactile** : 9 appuis, 9 changements d'écran (10 rendus complets, dont
  celui du démarrage), 0 erreur.
- **Mémoire** : tas PSRAM 77 Ko au plus (52 Ko dans les deux autres
  modes), tas interne jamais utilisé, pile du cœur 0 8,9 Ko au plus. Un
  seul framebuffer : 768 Ko de PSRAM de plus pour le tas qu'en
  SwappedBuffers.
- **Face à SwappedBuffers** : même absence de clignotement, même vitesse,
  sans l'attente de l'échange ni le second framebuffer, au prix d'une
  déchirure occasionnelle.

**Résultat fréquence réduite + ligne par ligne (2026-10-09) : le critère
tient.** Le levier se combine avec le rendu ligne par ligne plutôt qu'avec
ReusedBuffer, puisque les deux dessinent à la même vitesse : la combinaison
mesure le même gain, sans clignotement. Essai d'abord à 2,5 Hz : rendu net,
mais trop lent à l'œil. La démo avance donc par pas de 50 ms, et chaque
groupe change tous les 5 pas : 4 Hz, toujours un groupe par rendu, un pas
sur cinq sans rendu. Le mode sans fréquence réduite reste à 10 Hz. 4 Hz
est jugé tout à fait utilisable. `reduced-line.bin` dans `ota_0` (`otadata`
`0x61`/`0x60`, les deux `VALID`), 90 s d'observation, 7 appuis.

| Critère | 2,5 Hz | 4 Hz | Verdict à 4 Hz |
|---|---|---|---|
| Recopie : 0 retard, 0 recalage | 0 et 0, 913 µs au plus | 0 et 0, 913 µs au plus | tenu |
| Rendu partiel < 50 ms, infusion | moyenne 16 à 27 ms, 13 000 à 28 000 px, maximum 65 ms | moyenne 14 à 25 ms, 10 000 à 26 000 px, maximum 66 ms | tenu en moyenne |
| Rendu partiel < 50 ms, accueil | moyenne 34 à 41 ms, maximum 49 ms, ~100 000 à 125 000 px | moyenne 36 à 37 ms, maximum 47 ms, ~115 000 à 120 000 px | tenu, de justesse |
| Changement d'écran < 300 ms | 93 ms, 133 ms au premier rendu | 77 à 81 ms, 133 ms au premier rendu | tenu |

- **Rendus par seconde** : 14,7 à 15,1 sur l'écran d'infusion, pour 16
  prévus. Les rendus de plus de 50 ms dépassent le pas : le minuteur part
  en retard, et deux groupes tombent parfois dans le même rendu. Accueil :
  3,8 à 3,9 par seconde, pour 4.
- **Pics** : 123 ms quand le cycle de 50 s reprend, où la frise et toutes
  les valeurs repartent de zéro. Une vraie infusion fera de même à son
  départ : un rendu lent, une fois.
- **Accueil** : chaque rendu couvre ~120 000 px, alors que seuls le poids
  et la barre du haut changent. Le poids en 104 px et la barre du haut
  forment une grande zone : c'est l'écran le plus près de la limite.
- **Tactile** : 7 appuis, 7 changements d'écran, 0 erreur.
- **Mémoire** : tas PSRAM 77 Ko au plus, tas interne jamais utilisé, pile
  du cœur 0 8,9 Ko au plus.

### 7. `07-slint-radio` — Slint, BLE et Wi-Fi ensemble

Décision du 2026-10-08 : c'est le cas réaliste qui résume les essais. Il
répond à la question qui décide de la viabilité : l'UI tient-elle avec le
BLE et le Wi-Fi ensemble ?

L'essai réunit l'affichage Slint de l'étape 6 (cœur 0) et la radio de
l'étape 5, en combiné (cœur 1) : les deux liens GATT, le point d'accès, le
DHCP et le serveur HTTP, avec la même page de charge et le même `/stats`.
Il n'y a pas de variante Wi-Fi seul ni BLE seul : l'étape 5 les a déjà
mesurées.

**Un essai, pas le début de `screen` (décision du 2026-10-08).** Trois
raisons :

- la viabilité n'est pas acquise : cette étape et la stratégie d'écriture
  en flash peuvent encore changer l'architecture ;
- l'ordre prévu (« Cible finale ») fait passer `protocol` et `coffeetool`
  avant `screen` ;
- un essai peut prendre des raccourcis sans laisser de dette : point
  d'accès de test, page de charge, valeurs simulées.

`screen` commencera après, en reprenant les essais comme référence, et non
comme base de code.

- **UI :** la maquette de l'étape 6, dans sa configuration retenue : rendu
  ligne par ligne, fréquence réduite à 4 Hz. La **température réelle de la
  SCACE** remplace le poids simulé du chiffre en 104 px de l'accueil, et la
  température simulée de la barre du haut. Elle est rafraîchie à **4 Hz**,
  dans le rendu de la barre du haut (décision du 2026-10-09, au lieu des
  5 Hz prévus : 5 Hz tombe tous les 4 pas de 50 ms, et rejoindrait
  régulièrement le rendu d'un autre groupe). Le rendu lit la dernière
  trame reçue ; la SCACE en envoie 10 par seconde. Les autres valeurs
  restent simulées. Sonde débranchée ou lien coupé : le chiffre affiche
  « – », comme coffeeflow sans température.
- **Bounce buffers :** 20 lignes au départ. L'étape 5 a montré de rares
  recopies en retard à 20 lignes sous trafic Wi-Fi ; on s'attend à les
  retrouver. Une fois les mesures à 20 lignes notées, on passe à
  **30 lignes si la SRAM le permet** : le tas libre minimum reste au-dessus
  de la réserve de coffeeflow (32 Ko), et la pile du cœur 0 garde au moins
  16 Ko de marge après les 32 Ko de plus. Sinon, on reste à 20 (décision du
  2026-10-08 : en mode Wi-Fi, l'écran de coffeeflow est une page fixe).
- **Charge :** les deux liens abonnés, puis l'iPhone sur le point d'accès,
  une minute à 5 Hz et une minute sans pause, comme à l'étape 5.
- **Mesures :** celles de l'étape 5 (affichage, tas, piles, liens, HTTP,
  matrice d'interruptions), plus les temps de rendu de Slint de l'étape 6,
  et la taille de l'image.
- **Réussi si :** pendant toute la charge HTTP, 0 recalage et 0 recopie en
  retard (à 30 lignes si la SRAM l'a permis) ; la SCACE garde ses 10 trames
  par seconde sans trame manquée, et la balance ses pesées et son
  heartbeat ; la page ne compte ni erreur ni délai dépassé à 5 Hz ; la
  température change à l'écran au rythme de la sonde ; le rendu tient les
  critères de l'étape 6 ; le tas libre minimum reste au-dessus de 32 Ko.
- **Si l'affichage ne tient pas :** noter les chiffres avant de corriger.
  Leviers : 30 lignes, si ce n'est pas déjà fait, puis réduire les défauts
  de cache du cœur 1 (code réseau chaud en IRAM), puis la recopie par DMA
  mémoire à mémoire ou le préchargement du cache.
**Réalisation (2026-10-09).** L'essai `07-slint-radio` réunit l'affichage
de 06 (cœur 0) et `ble.rs`, `wifi.rs` de 05 (cœur 1). La configuration de
Slint est fixe : rendu ligne par ligne, valeurs à 4 Hz par groupe.

- **Deux tas (décision du 2026-10-09).** `esp_alloc::HEAP` ne contient que
  la SRAM interne, 128 Ko comme le combiné de l'étape 5 : les `malloc` des
  blobs radio y vont, et aucun ne peut déborder en PSRAM. Un second
  `EspHeap`, en PSRAM après le code et le framebuffer, porte Slint.
  L'allocateur global choisit selon le cœur qui alloue : PSRAM pour le
  cœur 0, SRAM interne pour le cœur 1. La libération suit l'adresse du
  bloc. Le tas libre minimum du rapport et de `/stats` est donc celui de la
  radio seule, comparable aux 33,7 Ko de l'étape 5.
- **Température** : affichée si le lien SCACE est abonné, que l'état de la
  trame est « mesure » (1) et que la dernière trame a moins de 1 s. Sinon,
  « – ».
- **Point d'accès** : le SSID et le mot de passe de l'essai 05, que le
  téléphone connaît déjà ; page de charge et `/stats` inchangés, plus
  `rendu_max_us`.
- **Bounce buffers** : 20 lignes par défaut, 30 avec la feature
  `bounce-30` (12 descripteurs par buffer, pour rester sous 4 095 octets).
- **Images** : 1 760 Ko, `bb20` et `bb30` sous `target/variants/`, avec
  leur `.elf`. `.bss` de 192 Ko, 52 Ko de code Wi-Fi en IRAM.
- **Clippy, `large_stack_frames`** : 28 signalements, les mêmes que sur
  l'essai 05, où ils n'avaient pas été examinés. Les estimations de clippy
  (jusqu'à 27 Ko) sont 4 à 15 fois au-dessus des trames réelles, relevées
  avec `-Z emit-stack-sizes` : `ble::link` 3,1 Ko, `radio_task` 2,6 Ko,
  `Interface::poll` de smoltcp 2,5 Ko, tâche `http` 0,7 Ko. Clippy
  additionne toutes les variables locales sans leur recouvrement, et compte
  dans une fonction `async` l'état de la future, qui vit dans l'arène de
  l'exécuteur, en `.bss`. L'essai 07 les autorise : `allow` avec raison
  dans `ble.rs`, `wifi.rs`, sur `report` et sur un module autour de
  `radio_task` (la fonction qu'engendre `#[embassy_executor::task]` ne
  reprend pas les attributs).
- **Pour `screen`** : garder `large_stack_frames` et son seuil de 1 Ko,
  utiles pour le code synchrone, et n'autoriser le lint qu'aux modules de
  tâches `async`, avec une raison. La garde réelle reste la pile peinte de
  chaque cœur, dans le rapport, et la mesure `-Z emit-stack-sizes` quand un
  doute se pose : `RUSTFLAGS="-C link-arg=-nostartfiles -Z
  emit-stack-sizes"`, dans un `CARGO_TARGET_DIR` à part, puis la section
  `.stack_sizes` de l'ELF (adresse, puis taille en ULEB128) croisée avec
  `nm`.

**Résultat à 20 lignes (2026-10-09) : tout tient, sauf 2 recopies en
retard sans pause.** `bb20` dans `ota_0` (`otadata` `0x61`/`0x60`, les deux
`VALID`). Deux passes : la première sans téléphone (81 s), la seconde avec
les deux liens abonnés, puis l'iPhone à 5 Hz et sans pause (122 s). Aucun
défaut visible à l'écran.

| Critère | Mesuré | Verdict |
|---|---|---|
| Recopie, à 5 Hz | 0 retard, 0 recalage, recopie la plus lente 1 121 µs | tenu |
| Recopie, sans pause | **2 retards**, 0 recalage, recopie la plus lente 1 252 µs | échec à 20 lignes |
| SCACE | 10 trames/s, 0 manquée, 0 invalide, 0 perte de lien | tenu |
| Balance | ~9 notifications/s, heartbeat toutes les 2,5 s, 0 perte | tenu |
| Page, 5 Hz | 179 requêtes, 5/s, latence 17 ms en moyenne, 60 ms au plus, 0 erreur | tenu |
| Page, sans pause | 4 076 requêtes, 128/s, 7 ms en moyenne, 61 ms au plus, 0 erreur | tenu |
| Rendu partiel, infusion | 14 à 26 ms en moyenne sans charge, ~30 ms sans pause ; 74 ms au plus en régime établi, 168 ms une fois sans pause | tenu en moyenne |
| Tas interne libre minimum | 34,0 Ko (44,3 Ko avant le téléphone) | tenu, de justesse |
| Piles | cœur 0 : 8,8 Ko sur 54,8 ; cœur 1 : 8,4 Ko sur 16 | tenu |

- **Les deux retards ne sont pas des retards d'entrée.** L'interruption
  entre à l'heure (écart de 1 220 µs, `dscr` 2 à l'entrée), mais la
  recopie dure 1 241 et 1 252 µs, au lieu de ~900 µs : plus que la période
  de 1 220 µs. À l'étape 5, c'était l'entrée qui prenait du retard. Ici, la
  lecture du framebuffer en PSRAM ralentit : le cœur 1 sert 128 requêtes/s
  avec son code en PSRAM (XIP), et le cœur 0 y écrit les lignes de Slint.
  Le PC interrompu (`Ord::min`, `spec_fill` de Slint) montre le cœur 0 en
  plein rendu.
- **Ce que 30 lignes changent.** La recopie reste à ~74 % de la période
  (900 µs sur 1 220, ~1 350 sur 1 830), mais la marge absolue passe de
  ~320 à ~480 µs. Les deux recopies lentes dépassaient de 20 à 30 µs.
- **Tas interne.** Le minimum tombe de 44,3 à 34,0 Ko quand les stations
  s'associent (le rapport en compte deux). Le DHCP et le serveur HTTP
  n'abaissent plus ce minimum ensuite. C'est l'équivalent des 33,7 Ko de
  l'étape 5 : Slint n'y prend rien.
- **Rendu sous charge** : environ 5 ms de plus en moyenne sans pause, et un
  pic à 168 ms. La contention sur la PSRAM ralentit aussi Slint.

**Décision du 2026-10-09 : on reste à 20 lignes, et l'étape 7 est
close.** Les deux retards n'apparaissent qu'à ~128 requêtes/s, un cas
limite qu'on ne rencontre qu'au téléchargement par Wi-Fi, et ils ne se
voient pas à l'œil. Pendant une infusion, le pire trafic HTTP est un
enregistrement de la sonde qui interroge `/telemetry` à 10 Hz, et la
plupart du temps il n'y en a aucun. À 5 Hz, aucun retard. `bb30` n'est
pas essayé : il ne ferait que gagner ~160 µs de marge, au prix de 32 Ko
sur la pile du cœur 0 (13,6 Ko de marge restante, sous les 16 Ko de la
règle). La feature `bounce-30` reste dans l'essai. **La combinaison
Slint + BLE + Wi-Fi tient en SRAM** : pas d'essai de bascule entre radios.

- **Si la combinaison ne tient pas en SRAM :** on revient au
  fonctionnement de coffeeflow, une radio à la fois. C'est acceptable
  (décision du 2026-10-08). Un essai de **bascule** s'ajoute alors. Avec
  l'UI qui tourne, il passe plusieurs fois de BLE à Wi-Fi, puis de nouveau
  à BLE. Il vérifie qu'esp-radio sait arrêter un contrôleur et en relancer
  un autre, et que le tas interne revient à la même valeur après chaque
  aller-retour. Critères : ceux de l'affichage, pendant chaque changement
  de mode.

### 8. `08-flash` — écritures en flash sur l'écran

Décision du 2026-10-09 : l'étape 7 close, il reste la troisième question
de viabilité, l'écriture en flash. Elle se tranche sur l'écran, en phase
destructive (règle n° 1, levée partielle).

**Le support Rust (relevé le 2026-10-09).**

| Besoin | Crate | État |
|---|---|---|
| Lire, écrire, effacer | `esp-storage` 0.10 (esp-rs) | Fonctions de la ROM, en section critique. `multicore_auto_park` suspend l'autre cœur pendant l'opération. `read`/`write` posent un secteur de 4 Ko sur la pile. |
| Partitions, `otadata`, OTA | `esp-bootloader-esp-idf` 0.6 (esp-rs) | `OtaUpdater`, états `New`/`PendingVerify`/`Valid`/`Invalid`/`Aborted`, changement de slot. À vérifier : `current_app_partition` prend la séquence la plus haute sans regarder l'état, là où le bootloader IDF écarte une copie `INVALID` ; l'app booté se lit par `booted_partition`. |
| NVS | `esp-nvs` 0.6.2 (tiers, un mainteneur, depuis 2025-11) | Format d'IDF, blobs multi-pages, migration de l'ancien format, CRC vérifiés. Le maillon faible : à valider dans les deux sens contre les outils d'IDF. Repli : notre propre format, migré au premier démarrage. |

**Préalable 1 : `nvs-vault`.** Script `firmware/rust/nvs-vault.sh`, sur le
modèle de `parttool.py` :

- `save <fichier>` : lit `nvs`, écrit le fichier en `chmod 600`, refuse un
  chemin dans le dépôt, affiche la taille et le SHA-256, rien d'autre ;
- `erase` : efface `nvs`, après `save` dans la même session ;
- `restore <fichier>` : réécrit `nvs`, relit, compare les SHA-256.

David garde le fichier ; Claude ne le lit pas. `save` passe avant toute
écriture USB de l'étape (bootloader compris).

**Préalable 2 : le bootloader de l'écran avec rollback.** Le bootloader
installé laisse les images en `NEW` ; celui de `sensors` les fait passer en
`PENDING_VERIFY` (relevé du 2026-10-09, plus bas). Pour répéter sur le
bureau le mécanisme de `sensors`, on réécrit le **bootloader** (à `0x0`),
et lui seul. Ni l'image `factory`, ni la table de partitions : la
réécriture de `factory` se prépare à part (« Cible finale »).

```sh
source ~/.espressif/tools/activate_idf_v6.1.sh
PORT=/dev/cu.usbmodem5B790235091
cd firmware/screen
# 1. Reconstruire : le bootloader.bin présent date du 2026-09-12.
#    sdkconfig : BOOTLOADER_APP_ROLLBACK_ENABLE=y, dio, 80m, 16MB.
idf.py bootloader
# 2. Garder l'ancien (0x0 à 0x8000, rien de secret), hors de build/.
esptool.py --chip esp32s3 -p $PORT read-flash 0x0 0x8000 ~/coffeeflow-bootloader-avant.bin
# 3. Écrire le nouveau, et rien d'autre.
esptool.py --chip esp32s3 -p $PORT write-flash --flash-mode dio \
  --flash-freq 80m --flash-size 16MB 0x0 build/bootloader/bootloader.bin
```

`nvs-vault save` passe avant l'étape 3. Contrôle : coffeeflow redémarre
sur `ota_1`, puis une OTA C++ de l'écran démarre en `PENDING_VERIFY`
(`firmware.screen.ota`) et se confirme. En cas d'échec, on réécrit
l'ancien bootloader avec la même commande. C'est aussi la correction que la
doc prévoit avant la production (« Cible finale »).

**Fait le 2026-10-09.** `nvs-vault save` d'abord (24 576 octets, SHA-256
`7c1e579d…afe48c`, fichier gardé par David). Le bootloader installé datait
du 2026-09-10, jour de l'écriture des images `factory`. Il est sauvegardé
dans `~/coffeeflow-bootloader-avant.bin`. Il a été remplacé par celui du
build du 2026-09-12 : `idf.py bootloader` n'a rien recompilé, et sa
configuration a le rollback. Relecture identique au build, et coffeeflow
0.3.47 redémarre sur `ota_1`. La console reste muette :
`ESP_CONSOLE_NONE`, bootloader compris.

Incohérence à lever au contrôle : le rollback est actif dans le `sdkconfig`
de l'écran depuis le 2026-09-07. L'ancien bootloader l'avait donc
probablement déjà, et l'état `NEW` observé en 0.2.64 aurait une autre
cause. On ne le saura plus : l'ancien est remplacé.

**Contrôle fait le 2026-10-09 : le rollback fonctionne.** Coffeeflow 0.3.48
envoyé par Wi-Fi démarre dans `ota_0` en `pending_verify`. L'écran étant
hors du bus (`sensors.alive: false`), la confirmation n'a pas été tentée :
après le délai de cinq minutes, l'image s'invalide et redémarre, et le
bootloader repart sur `ota_1` en 0.3.47, état `valid`. C'est le chemin dont
dépend la sonde de l'étape 9. Cette OTA a écrasé l'essai 07 dans `ota_0`.

**8a. La flash et l'affichage.** L'essai 07, avec en plus l'effacement
puis la réécriture de secteurs dans la fin du slot de test, au-delà de
l'image, radio suspendue par `multicore_auto_park`.

- **Mesures :** durée d'un effacement de secteur et d'une écriture de page
  sur la puce de l'écran ; retards et recalages de la recopie ; contrôle
  visuel ; liens BLE et HTTP après chaque série.
- **Réussi si :** l'image se rétablit seule après chaque opération, sans
  décalage permanent, et les liens tiennent. Les retards pendant une
  opération sont attendus : on les compte, ils décident de l'UI pendant
  une OTA (écran fixe « mise à jour » ou non).

**8b. Un cycle OTA complet, Rust et C++ mêlés.** Le slot inactif porte
coffeeflow : la Rust A l'écrase. C'est voulu, et c'est le meilleur test :

1. La Rust A, dans `ota_0`, reçoit **l'image C++ de coffeeflow** par
   `POST /firmware` (point d'accès de l'essai), l'écrit dans `ota_1`, pose
   `otadata` en `New` et redémarre. Coffeeflow démarre en
   `PENDING_VERIFY` et se confirme par `POST /firmware/confirm`. Rust
   installe donc du C++.
2. Coffeeflow installe une Rust B par sa route habituelle. La Rust B,
   en `PENDING_VERIFY`, joue tour à tour :
   - ne se valide pas, et redémarre après son délai : retour sur
     coffeeflow ;
   - panique au démarrage : retour sur coffeeflow ;
   - se bloque, watchdog RTC armé : retour sur coffeeflow ;
   - se valide : elle reste.

   Chaque cas se rejoue par une nouvelle OTA de coffeeflow.

- **Réussi si :** les quatre cas donnent le résultat attendu, `otadata`
  reste lisible par `otatool` et par coffeeflow après chaque écriture
  Rust, et l'affichage se comporte comme en 8a pendant la réception.
- **À relever :** ce que fait `esp-backtrace` après une panique (il doit
  redémarrer, pas s'arrêter), et si `esp_hal::init` coupe le watchdog RTC
  laissé par le bootloader.

**8c. NVS, d'abord sans le vrai NVS.**

1. Une image NVS générée sur le Mac par `nvs_partition_gen.py` d'IDF, avec
   de faux identifiants et les types de coffeeflow (chaînes, entiers,
   blobs), écrite dans la fin du slot de test. `esp-nvs` doit tout relire.
2. `esp-nvs` y écrit, modifie et efface des clés ; la zone relue sur le
   Mac doit se décoder avec `nvs_tool.py` d'IDF.
3. Seulement ensuite, le vrai NVS, après `nvs-vault save` : Rust lit les
   clés de coffeeflow et n'en affiche que les noms, types et longueurs.
   Puis Rust écrit une clé de test ; coffeeflow redémarre et doit relire
   tous ses réglages. `nvs-vault restore` à la fin, dans tous les cas.

- **Réussi si :** les deux sens se décodent sans perte, et coffeeflow
  relit son NVS après le passage de Rust. Sinon : notre propre format,
  avec migration au premier démarrage.

### 9. `09-sensors-probe` — première image Rust sur `sensors`, sans USB

Décision du 2026-10-09 : `sensors` passe à Rust par la chaîne OTA C++
actuelle, sans jamais ouvrir la machine. L'étape vient après la 8b, qui
aura éprouvé le même mécanisme sur l'écran.

**Le bootloader de `sensors` fait le rollback (relevé du 2026-10-09).**
`sensors` 0.3.48 journalise l'état OTA au démarrage dans `arg32` de
`OTA_PENDING_VERIFY`. Après une OTA par l'écran :
`OTA_PENDING_VERIFY etat=PENDING_VERIFY`, puis `CONFIRM_SENSORS_OTA` et
`OTA_VALIDATED` en 1,1 s. Une image qui ne se valide pas et redémarre
revient donc sur l'image précédente.

**Conséquence : la sonde n'a aucun code d'écriture en flash.** La chaîne
C++ (proxy de l'écran, `ota_staging`, récepteur de `sensors`) écrit l'image
Rust dans le slot inactif ; l'image C++ reste dans l'autre. La sonde ne se
valide jamais : le redémarrage la renvoie toujours sur le C++.

- **Sorties sûres d'abord :** SSR, vanne et dimmer au repos, avant toute
  autre initialisation, comme l'image C++.
- **Watchdog RTC** armé dès le début de `main` ; une panique redémarre.
- **CAN :** écoute, et un `LOG BOOT` puis un `LOG` toutes les 5 s, `arg32`
  au repère `0x52555354` (« RUST ») et à l'uptime. Ni `PING` ni `PONG` :
  l'écran voit la présence perdue, machine au repos. `CONFIRM_SENSORS_OTA`
  est ignoré.
- **Retour :** redémarrage volontaire après 30 s, comme le rollback C++.
- **Image :** ≤ 1 Mo (`ota_staging`), ≤ 1 500 Ko (slot de `sensors`).
- **Réussi si :** le moniteur `coffeetool` montre les `LOG` de la sonde,
  puis, après 30 s, le `PING` de `sensors` 0.3.48, `OTA_VALIDATED` absent
  (l'image C++ est déjà `VALID`), et les échos d'actionneurs reviennent.
- **Si la sonde ne revient pas :** 30 s, puis le watchdog, puis le
  rollback du bootloader au redémarrage suivant. Au-delà, l'image
  `factory` de `sensors`, jamais réécrite, reste le filet.

Seulement après : la vraie app Rust de `sensors`, avec sa propre
validation par CAN et la réception FLASH en Rust.

---

## Journal

Résultats datés, au fil des essais.

- **2026-10-08 :** environnement vérifié, `esp-generate` 1.4.0 et `probe-rs`
  0.32.0 installés, partitions et `otadata` de l'écran relus (coffeeflow dans
  `ota_1`, slot de test `ota_0`). Aucun essai flashé.
- **2026-10-08 :** `01-hello-uart` — chaîne complète validée. Build release
  41 s, image 99 488 octets. `otadata` relu : `0x51`/`0x52`, les deux `VALID`,
  coffeeflow dans `ota_1`. Essai écrit dans `ota_0` (`0x120000`, hash vérifié),
  bascule, le bootloader IDF v6.1 charge l'app depuis `0x120000`, et le compteur
  s'affiche chaque seconde sur `/dev/cu.usbmodem5B790235091`. Retour sur `ota_1`
  (`otadata` `0x53`/`0x54`, les deux `VALID`), coffeeflow redémarre normalement.
  Le décodage des adresses par `espflash monitor --elf` (fonction, fichier,
  ligne) a été vérifié à l'essai 04. La backtrace d'un panic se vérifiera au
  premier vrai panic, sans essai dédié.
- **2026-10-08 :** `02-lcd-static`, DMA direct depuis la PSRAM. Mire juste
  (couleurs, bits, timings), mais décalée horizontalement. Deux causes
  trouvées. D'abord, `Dpi::send` jette le contenu de la FIFO au démarrage :
  corrigé en redémarrant dans l'ordre d'IDF. Ensuite, chaque défaut de cache
  sur la flash affame le DMA, et la perte est définitive. Démontré par un
  témoin de code froid à 30 s, qui fait sauter une mire jusque-là alignée.
  Mode direct écarté, passage aux bounce buffers de 20 lignes
  (`02b-lcd-bounce`). Retour sur coffeeflow, `otadata` `0x55`/`0x56`, les deux
  `VALID`.
- **2026-10-08 :** `02b-lcd-bounce` validé. Deux bounce buffers de 20 lignes
  en SRAM, recopie par interruption, contrôle de phase au VSYNC. Mire stable,
  insensible au code froid. 31 trames/s, 0 recalage, recopie max 607 µs sur
  1 220. Un retard isolé au démarrage, à vérifier. 20 lignes retenues.
- **2026-10-08 :** `03-lcd-dynamic`. Sur un cœur : mire immobile, 0 retard,
  0 recalage, recopie max 885 µs, rectangles déchirés (un seul
  framebuffer). Sur deux cœurs : ~46 retards/s, recopie max 1 320 µs, ce qui
  reproduit le problème de coffeeflow et valide le compteur. Répartition
  retenue : affichage sur le cœur 0, radio sur le cœur 1. Retour sur
  coffeeflow, `otadata` `0x57`/`0x58`, les deux `VALID`.
- **2026-10-08 :** `04-ble-display`, scan BLE actif sur le cœur 1 avec
  l'affichage de 03 sur le cœur 0. Acaia et SCACE vues. En régime établi :
  0 retard, 0 recalage, recopie max 940 µs. Six vrais retards à
  l'initialisation BLE. Le détecteur des essais 02b et 03 était faux
  (`OUT_DSCR` a deux descripteurs d'avance) ; il est corrigé. Code de
  recopie en SRAM essayé, sans effet, puis retiré. Critère de stabilité
  ramené de 10 minutes à une minute. Essai dans `ota_0`, `otadata`
  `0x59`/`0x58`, les deux `VALID`. Reste la connexion GATT à la SCACE.
- **2026-10-08 :** `04b-ble-gatt`, connexions GATT simultanées à la Lunar et
  à la SCACE, reconnexion validée. Avec le code en flash, recopies en retard
  (jusqu'à 1 944 µs). Code et rodata passés en PSRAM (`src/xip.rs`) et
  dessin ramené à 10 Hz : 0 retard, 0 recalage, recopie max 892 µs, avec
  20 lignes. Essai laissé dans `ota_0` ; retour sur coffeeflow, `otadata`
  `0x5b`/`0x5c`, les deux `VALID`, coffeeflow redémarre normalement.
  Prochaine étape : 5, Wi-Fi avec l'affichage, puis combiné au BLE.
- **2026-10-08 :** UI de `screen` en Slint, décidé après comparaison avec
  `embedded-graphics` seul. Pas de bindings vers LVGL. Licence sans objet
  tant que le projet reste personnel. Essai `07-slint-display` ajouté.
- **2026-10-08 :** `05-wifi-ap` écrit et compilé, dans ses deux variantes
  (Wi-Fi seul, et `--features ble`). Pas encore flashé. BLE + Wi-Fi
  ensemble est préférable mais pas exigé. La réponse viendra de `06-all`
  avec Slint, qui passe donc après l'étape 7.
- **2026-10-08 :** `05-wifi-ap`, Wi-Fi seul, dans `ota_0` (`otadata`
  `0x5d`/`0x5c`). Premier démarrage en panic : la feature `dns`
  d'`embassy-net`, activée par `edge-nal-embassy`, réserve un socket de plus.
  Après correction, le point d'accès, le DHCP et le HTTP fonctionnent, de
  5 à 130 requêtes/s, sans erreur. Recopie la plus lente : 890 µs sans
  client, environ 1 050 µs dès qu'un iPhone est associé, et 1 100 à
  1 211 µs sous charge, quel que soit le débit. Une vraie recopie en retard
  en environ 3 min, 0 recalage. L'interruption reste à l'heure. Matrice :
  `WIFI_MAC`/`WIFI_PWR` sur l'interruption CPU 1 du cœur 1 ; le cœur 0 garde
  la valeur 0 écrite par esp-radio, sans effet visible. Tas : 100 Ko, dont
  38,5 Ko libres au minimum. Pile du cœur 1 : 6,3 Ko sur 16. Le compteur
  de stations ratait des déconnexions : il s'abonnait de nouveau à chaque
  événement. Corrigé après coup par un abonnement unique et un suivi par
  AID, sans nouveau passage sur la carte. **Décision :** les bounce buffers
  restent à 20 lignes, et 30 lignes reste une option. En mode Wi-Fi, l'écran
  de coffeeflow est une page fixe, sans animation. On passe donc à 30
  lignes seulement si la SRAM le permet, sinon on reste à 20.
- **2026-10-08 :** `05-wifi-ap --features ble`, BLE + Wi-Fi, dans `ota_0`.
  La Lunar et la SCACE sont abonnées en 10 s. Pendant la charge HTTP (5 Hz,
  puis 98 à 127 requêtes/s), la SCACE garde 10 trames/s sans trame
  manquée. La balance garde 9 notifications/s et un heartbeat toutes les
  2,5 s. Aucun échec ni perte de lien. Page : 16 ms de latence moyenne et
  30 ms au maximum à 5 Hz ; 8 ms et 150 ms sans pause. Recopie la plus
  lente : 870 à 911 µs sans requêtes, jusqu'à 1 258 µs sous charge.
  **4 vraies recopies en retard** en environ 90 s de charge, 0 recalage.
  L'interruption entre parfois jusqu'à environ 120 µs en retard (écart de
  1 340 µs). `RWBLE`, `WIFI_MAC` et `WIFI_PWR` sont servies par le cœur 1.
  Tas : 128 Ko, dont 33,7 Ko libres au minimum. Pile du cœur 1 : 8,4 Ko
  sur 16. La pile du cœur 0 garde 57 Ko, dont 3,3 Ko utilisés. Image :
  878 Ko, 13 pages de XIP. La combinaison tient en SRAM, et 30 lignes de
  bounce buffer (+32 Ko) y resteraient possibles. Retour sur coffeeflow :
  `otadata` `0x5d`/`0x5e`, les deux `VALID`, coffeeflow redémarre
  normalement. **Étape 5 close.** L'effet de 30 lignes est compris, sans
  essai dédié : la décision se prendra avec la SRAM disponible sous Slint.
- **2026-10-08 :** plan revu. L'étape « pression mémoire » est retirée :
  ses mesures sont faites au fil des essais, et le combiné de l'étape 5 en
  est le pire cas radio. Étape 6 : `06-slint-display`, Slint seul. Étape 7 :
  `07-slint-radio`, Slint avec BLE et Wi-Fi, la température SCACE affichée
  à 5 Hz. La bascule BLE ↔ Wi-Fi ne s'essaie que si l'étape 7 ne tient pas
  en SRAM. Les étapes 6 et 7 restent des essais : `screen` ne commence
  qu'ensuite.
- **2026-10-08 :** `06-slint-display` écrit et compilé, en ReusedBuffer et
  en SwappedBuffers, avec son simulateur sur Mac. Image de 989 Ko, rodata de
  547 Ko, surtout les glyphes d'Inter.
- **2026-10-08 :** `06-slint-display`, ReusedBuffer, dans `ota_0` (`otadata`
  `0x5f`/`0x5e`, les deux `VALID` avant la bascule). Recopie : 0 retard,
  0 recalage, recopie max 921 µs. Rendu partiel de l'écran d'infusion :
  90 à 106 ms en moyenne, 141 ms au maximum, sur ~200 000 px. Échec du
  critère des 50 ms, et l'écran clignote. Accueil : 45 ms au maximum.
  Changement d'écran : 72 à 130 ms. Défaut du `Ticker` de la boucle de
  rendu relevé. Leviers notés dans l'étape 6. Retour sur coffeeflow :
  `otadata` `0x5f`/`0x60`, les deux `VALID`.
- **2026-10-08 :** `06-slint-display`, trois variantes préparées pour la
  reprise, chacune une feature : SwappedBuffers, rendu ligne par ligne en
  SRAM interne, fréquence réduite par groupe de valeurs (2,5 Hz par défaut,
  à décider valeur par valeur). Correctif du `Ticker`. Images de 989 à
  998 Ko sous `target/variants/`. Sur le Mac, la fréquence réduite divise
  par huit la zone redessinée. Rien de flashé ; coffeeflow en service.
- **2026-10-09 :** `06-slint-display`, SwappedBuffers, dans `ota_0`
  (`otadata` `0x5f`/`0x60` avant, `0x61`/`0x60` après la bascule, les deux
  `VALID`). Aucun clignotement. Recopie : 0 retard, 0 recalage. Rendu
  partiel de l'écran d'infusion : 94 à 117 ms en moyenne, 145 ms au
  maximum, sur ~210 000 à 230 000 px. Échec du critère des 50 ms. Accueil :
  45 ms au maximum. Attente de l'échange : jusqu'à 32 ms par rendu.
- **2026-10-09 :** `06-slint-display`, ligne par ligne, dans `ota_0`
  (`otadata` `0x61`/`0x60`, les deux `VALID`). Aucun clignotement, une
  déchirure occasionnelle des grands chiffres, acceptable. Rendu partiel de
  l'écran d'infusion : 92 à 108 ms en moyenne, 147 ms au maximum, sur
  ~190 000 à 200 000 px : pas plus rapide qu'en ReusedBuffer, échec du
  critère des 50 ms. Accueil : 49 ms au maximum. Tas PSRAM 77 Ko au plus.
- **2026-10-09 :** `06-slint-display`, fréquence réduite à 2,5 Hz et ligne
  par ligne, dans `ota_0`. Rendu net, sans clignotement, mais trop lent à
  l'œil. Rendu partiel de l'écran d'infusion : 16 à 27 ms en moyenne, sur
  13 000 à 28 000 px, 65 ms au maximum en régime établi, 124 ms quand le
  cycle de 50 s reprend. Accueil : 34 à 41 ms en moyenne, 49 ms au maximum,
  ~100 000 à 125 000 px. Fréquence réduite passée à 4 Hz : pas de la démo
  de 50 ms, période de 5 pas, toujours un groupe par rendu.
- **2026-10-09 :** `06-slint-display`, fréquence réduite à 4 Hz et ligne
  par ligne, dans `ota_0` (`otadata` `0x61`/`0x60`, les deux `VALID`).
  4 Hz jugé tout à fait utilisable. Rendu partiel de l'écran d'infusion :
  14 à 25 ms en moyenne, 66 ms au maximum en régime établi, 123 ms quand le
  cycle reprend. Accueil : 47 ms au maximum. Critère des 50 ms tenu en
  moyenne sur les deux écrans.
- **2026-10-09 :** `07-slint-radio` écrit : affichage de 06 (ligne par
  ligne, 4 Hz) sur le cœur 0, radio de 05 en combiné sur le cœur 1, deux
  tas (SRAM interne pour la radio, PSRAM pour Slint), température SCACE à
  l'écran. Image de 1 760 Ko. Clippy : les 28 `large_stack_frames` de 05
  et 07 expliqués (trames réelles de 3,1 Ko au plus) et autorisés dans 07.
- **2026-10-09 :** `07-slint-radio`, 20 lignes, dans `ota_0` (`otadata`
  `0x61`/`0x60`, les deux `VALID`). Deux liens abonnés, SCACE à 10 trames/s
  sans perte, page à 5 Hz puis sans pause (128 requêtes/s), 0 erreur HTTP.
  0 recalage ; 0 retard à 5 Hz, 2 retards sans pause, par une recopie
  ralentie (1 252 µs) et non par une entrée tardive. Tas interne libre
  minimum 34,0 Ko. Rendu de l'infusion ~30 ms en moyenne sous charge.
  Rien de visible à l'écran. Décision : 20 lignes, 30 lignes non essayé.
  **Étape 7 close** : la combinaison tient en SRAM.
- **2026-10-09 :** support Rust de la flash relevé : `esp-storage` 0.10,
  `esp-bootloader-esp-idf` 0.6 (OTA complète), `esp-nvs` 0.6.2 (tiers,
  format IDF). Étapes 8 (écran : flash et affichage, cycle OTA Rust et
  C++, NVS) et 9 (sonde Rust sur `sensors` par la chaîne OTA C++) écrites.
  Règle n° 1 levée en partie sur l'écran pour l'étape 8.
- **2026-10-09 :** `sensors` 0.3.48 flashé par l'écran : il démarre en
  `PENDING_VERIFY`, puis `OTA_VALIDATED`. Le bootloader installé sur
  `sensors` fait le rollback ; celui de l'écran ne le fait pas.
- **2026-10-09 :** `firmware/rust/nvs-vault.sh` écrit (`save`, `erase`,
  `restore`, refus testés sans la carte). NVS de l'écran sauvegardé hors
  du dépôt. Bootloader de l'écran remplacé par celui du build IDF du
  2026-09-12, avec rollback ; l'ancien (2026-09-10) est gardé. Coffeeflow
  0.3.47 redémarre.
- **2026-10-09 :** contrôle du nouveau bootloader de l'écran. Coffeeflow
  0.3.48 par Wi-Fi démarre dans `ota_0` en `pending_verify` ; sans
  confirmation (écran hors du bus), retour seul sur `ota_1` en 0.3.47,
  `valid`, après cinq minutes. Le rollback de l'écran fonctionne, comme
  celui de `sensors`. Préalables de l'étape 8 faits.

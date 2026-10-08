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
- **Ordre.** `protocol` et `coffeetool` d'abord, sans risque matériel. Puis
  `screen`, la carte la plus risquée en technique (LCD, BLE, Wi-Fi), sur
  laquelle portent les essais ci-dessous. `sensors` en dernier : c'est la
  carte la plus simple, mais elle est enfermée dans la machine et se met à
  jour uniquement à travers l'écran.

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
  n'a pas d'équivalent trouvé dans esp-hal 1.2.2. De plus, `esp-storage`
  fait chaque effacement et chaque écriture en section critique, et exige
  que l'autre cœur soit arrêté pendant ce temps. Voir la section « Écritures
  en flash ».
- Le double framebuffer avec `avoid_tearing` a été essayé dans IDF puis
  abandonné (écran instable).

### Écritures en flash : à concevoir à part

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
  et remapper la MMU. Ce n'est pas fourni par esp-hal 1.2.2, donc ce serait
  à écrire.

`esp-storage` exige aussi que l'autre cœur soit arrêté pendant l'opération :
avec la radio sur le cœur 1, il faudra le suspendre le temps de chaque
écriture.

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
- **Réussi si :** la mire est stable pendant 10 minutes, sans décalage ni
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
cache). Un seul « retard » compté avant 10 s, puis plus aucun : soit le
démarrage à cache froid, soit le compteur lui-même, qui lit `out_dscr` sans
savoir s'il désigne le descripteur en cours ou le suivant. À vérifier à
l'essai 03.

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
  10 minutes, avec redessin continu de plusieurs zones.
- Le choix d'une bibliothèque d'UI (Slint, LVGL via des bindings, ou
  `embedded-graphics` seul) se fait **après** cette étape, une fois les
  chiffres connus.

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

### 4. `04-ble-scan` — BLE

`esp-radio` + `trouble-host` en central, sur le modèle de
`examples/ble/scanner`. Il scanne et affiche les annonces de l'Acaia Lunar et
de la sonde SCACE (nom, RSSI). Il ne se connecte pas encore en GATT.

- **Prouve :** que la pile BLE Rust démarre sur cette carte et voit les deux
  périphériques réels.
- **Puis :** connexion GATT à la SCACE et lecture d'une notification, puisque
  coffeeflow lit déjà la SCACE en BLE.

### 5. `05-wifi-ap` — Wi-Fi

Point d'accès avec un SSID et un mot de passe **propres à l'essai**, écrits
dans le source de l'essai, sans rapport avec ceux de coffeeflow et sans
lecture de NVS. DHCP, et un petit serveur HTTP qui renvoie les mesures
mémoire, sur le modèle de `examples/wifi/embassy_access_point`.

- **Prouve :** la pile Wi-Fi et réseau (esp-radio + embassy-net) sur cette
  carte, sans dépendre des identifiants du réseau domestique.
- Un mode station viendra plus tard. Il faudra alors décider comment fournir
  les identifiants sans les lire depuis NVS (fichier ignoré par Git, comme
  `secrets.h`).

### 6. Pression mémoire

Chaque essai, de l'étape 2 à l'étape 5, affiche sur la console, au démarrage
puis toutes les 10 s : SRAM interne libre, plus grand bloc interne libre,
PSRAM libre (`esp_alloc::HEAP.stats()`), et taille de l'image. Le critère
qui compte est la **dernière combinaison** : un essai `06-all` qui fait
tourner l'écran dynamique avec le scan BLE, puis avec l'AP Wi-Fi.

Comme aujourd'hui dans coffeeflow, **Wi-Fi et BLE ne tournent pas en même
temps**. Les faire coexister serait un plus, mais pas au prix d'un arbitrage
sur le code placé en SRAM : si la place manque, le code en SRAM passe
d'abord. À titre indicatif, les exemples d'esp-radio sur ESP32-S3 réservent
72 Ko de tas pour le BLE, 100 Ko pour le Wi-Fi et 128 Ko pour les deux.

- **Prouve :** que les deux bounce buffers internes (64 Ko, 20 lignes), la
  pile radio active et les tâches tiennent en SRAM interne, avec une marge,
  et sans dégrader l'affichage de l'étape 3.
- **Référence :** relever les mêmes grandeurs dans coffeeflow
  (`GET /telemetry`) avant de comparer.
- **Réussi si :** les critères de l'étape 3 tiennent avec BLE, puis avec
  Wi-Fi actif,
  et le plus grand bloc interne libre ne descend pas sous la réserve de
  coffeeflow (`CONFIG_SPIRAM_MALLOC_RESERVE_INTERNAL`, 32 Ko).

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
  Reste à faire pour clore l'étape : le panic volontaire avec backtrace décodée.
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

# Firmware — build, flash et banc

Conception, protocole, sécurité et mise à jour : `firmware.md`. Ce fichier
tient ce qu'il faut pour construire, flasher et reprendre le banc, ainsi que
les pièges déjà payés une fois. Le firmware est en Rust ; le C++/ESP-IDF
archivé (`archive/firmware/`) reste compilable et sert de référence de
comportement, ses commandes sont marquées « C++ archivé ».

## Environnement

Deux toolchains, selon le crate.

**Workspace hôte** (`firmware/rust` : `protocol`, `screen-core`, `screen-net`,
`sensors-logic`, `coffeetool`), toolchain `stable`, sans matériel :

```sh
cd firmware/rust
cargo test                            # tout le workspace
cargo +stable test -p screen-core     # un crate
cargo +stable clippy -p screen-core --all-targets -- -D warnings
```

Les tests HTTP de `screen-net` ouvrent un socket local.

**Simulateur de l'UI** (`firmware/rust/screen-sim`, hors workspace), depuis la
racine du dépôt :

```sh
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml -- --snapshots /tmp/coffeeflow-ui
cargo +stable test --manifest-path firmware/rust/screen-sim/Cargo.toml
```

Sans argument, une fenêtre de 800 × 480 utilise le vrai `screen-core` avec un
faux module interne. `--snapshots` écrit les 24 scénarios en PNG
(`--list` les énumère). Ce n'est pas une comparaison pixel par pixel avec
l'ancienne UI LVGL.

**Crates Xtensa** (`firmware/rust/screen`, `firmware/rust/sensors`),
toolchain `esp` (fichier `rust-toolchain.toml` du crate). Chaque nouveau
terminal doit charger l'environnement :

```sh
source ~/export-esp.sh
```

Chaque crate a son `.cargo/config.toml` et son `Cargo.lock`. Construire depuis
le répertoire du crate :

```sh
cd firmware/rust/sensors
cargo build --release
espflash save-image --chip esp32s3 --flash-size 8mb --skip-update-check \
  target/xtensa-esp32s3-none-elf/release/sensors /tmp/sensors-<variante>.bin

cd firmware/rust/screen
cargo build --release --offline
espflash save-image --chip esp32s3 --flash-size 16mb --skip-update-check \
  target/xtensa-esp32s3-none-elf/release/screen /tmp/screen-<variante>.bin
```

Le XIAO a 8 Mo de flash et l'écran 16 Mo. Une image `sensors` tient dans
`ota_staging` (1 Mo au plus ; 164 Ko en pratique) ; une image `screen` dans un
slot de 3 584 000 octets (2,7 Mo en pratique). Copier l'ELF à côté de l'image
(`/tmp/screen-<variante>.elf`) pour `espflash monitor --elf`.

Options de build de `screen`, par variables d'environnement :

- `REPORT_PERIOD_MS=<ms>` : période du rapport console (10 s par défaut, 1000
  au moins) ; il porte les piles des deux cœurs et des tâches radio.
- `RADIO_EXTRA_TASK_STACK=<octets>` : pile supplémentaire pour les tâches
  créées par les blobs radio.

Option de build de `sensors` : la feature `no-validate`
(`cargo build --release --features no-validate`) ignore `CONFIRM_SENSORS_OTA`
et redémarre après 30 s. C'est la première installation d'un `sensors` sans
risque : le rollback est garanti. Ne jamais la laisser dans une image de
service.

Contrôles Xtensa, sans matériel :

```sh
cargo clippy --release --lib --bin screen --offline -- -D warnings
```

Pour `screen`, jamais `--all-targets` : le harnais Rust `test` n'existe pas
sur cette cible, les modules testables sont exécutés sur l'hôte. Le linker GCC
signale un segment RWX, attendu pour les données et instructions en RAM de
cette cible.

**Secrets de `screen`.** `firmware/rust/screen/secrets.rs` (ignoré par Git)
porte le jeton HTTP et le mot de passe WPA2 du point d'accès de provisioning ;
`secrets.example.rs` est le modèle versionné. Sans `secrets.rs`, le build
utilise l'exemple, mais l'authentification HTTP et le provisioning sont
désactivés. Ne jamais lire ni afficher ces fichiers.

La table des codes `LOG` est générée depuis une source unique,
`firmware/rust/protocol/log_codes.yaml` ; `protocol/build.rs` en tire le Rust,
ne jamais éditer le code généré.

**C++ archivé.** `source ~/.espressif/tools/activate_idf_v6.1.sh`, puis
`idf.py build` depuis `archive/firmware/screen` ou `archive/firmware/sensors`
(version d'ESP-IDF : `firmware/IDF_VERSION.md`). Tests hôte archivés :
`archive/firmware/common/test/run_tests.sh` (protocole, C++) et
`archive/firmware/tools/test/run_tests.sh` (outils Python).

## Flash

Courant, en exploitation : par OTA, avec `coffeetool`, depuis `firmware/rust`
(`cargo build --release -p coffeetool` ; binaire dans
`firmware/rust/target/release/coffeetool`) :

```sh
coffeetool --ip <ip> flash <image> --target screen     # HTTP, écran
coffeetool --ip <ip> flash <image> --target sensors    # HTTP, puis CAN à travers l'écran
coffeetool flash --usb=<port> <image> --target screen  # pont USB↔CAN de l'écran
```

`--ip` et `--token` valent aussi `COFFEEFLOW_IP` et `COFFEEFLOW_HTTP_TOKEN`.
Le signe égal de `--usb=<port>` est obligatoire. Ne pas interrompre le pont ni
l'alimentation pendant un transfert. Une image `sensors` passe en blocs de 2 ko
acquittés (82 blocs pour 164 Ko) ; la confirmation finale puis le PING/PONG au
redémarrage valident l'image `PENDING_VERIFY`. L'écran se confirme avec
`coffeetool confirm` (`POST /firmware/confirm`) dans les cinq minutes ; sans
validation, l'image précédente revient. `sensors` est confirmé par l'écran lui-même
(`CONFIRM_SENSORS_OTA`), sans commande.

Le flash USB direct d'une image Rust est réservé à la première installation
sur une carte ou à la récupération. Méthode des essais : écrire l'image dans
l'offset de `ota_0` (`espflash write-bin … 0x120000` sur l'écran), ou par
`parttool.py`, puis `otatool.py -p <port> switch_ota_partition --name ota_0`.
Le retour à l'autre slot est `… --name ota_1`. Les images suivantes se
réécrivent dans le slot inactif par OTA, sans nouvelle bascule.

**C++ archivé.** `idf.py flash` n'est utilisable que pour écrire ou récupérer
l'image `factory` du C++ ; il réécrit la table de partitions, NVS et/ou
`factory` selon la configuration locale.

### Images factory

Les images de secours des deux cartes sont des **images C++** figées au commit
`451566c` (`v0.2.9`, ESP-IDF 6.1.0) et archivées, avec leurs SHA-256 et leur
procédure d'écriture, sous `firmware/factory-images/2026-09-09-451566c/`
(`screen-factory.bin`, `sensors-factory.bin`). Elles sont à l'offset `0x20000`
de leurs partitions `factory`. L'écran factory est le pont USB↔CAN : il sait
reflasher l'écran localement et le XIAO à travers le CAN. Son écran noir est
normal, puisque cette image ne porte pas d'interface graphique.

### Banc LCD jetable (C++ archivé)

`archive/firmware/screen-lcd-test` (sans CAN ni NVS applicative) s'écrivait par
`idf.py flash` complet. Sa table de partitions n'est pas celle de production
(`factory` 1408 KiB contre 1000 KiB). Après usage, restaurer la table et la
factory de l'écran par `esptool.py`, jamais par `idf.py flash` :

```sh
esptool.py --chip esp32s3 --port /dev/cu.wchusbserialXXXX write_flash \
  0x8000 archive/firmware/screen/build/partition_table/partition-table.bin \
  0x20000 firmware/factory-images/2026-09-09-451566c/screen-factory.bin
esptool.py --chip esp32s3 --port /dev/cu.wchusbserialXXXX \
  erase_region 0x10000 0x2000
```

Le second ordre efface seulement `otadata` : le bootloader choisit alors la
partition `factory`, d'où l'on relivre l'image fonctionnelle par OTA. Le même
effacement de `otadata` sert de récupération quand une image OTA redémarre en
boucle sans que le rollback reprenne le slot précédent.

### Sonde SCACE

`firmware/scace` tourne sur un M5Stack Core2 indépendant de la machine :
cible `esp32`, partition unique, sans OTA. `idf.py flash` complet est le
chemin normal. Conception et calibration : [sonde-scace.md](sonde-scace.md).

### Outils hôte

`coffeetool` (Rust, `firmware/rust/coffeetool`) remplace les outils Python,
archivés dans `archive/firmware/tools`. Le transport par défaut est HTTP ;
`--usb[=PORT]` passe par le port série pour les seules commandes qui ont un
sens sur le bus CAN. Sous-commandes :

| Commande | Rôle |
| --- | --- |
| `flash <image> --target screen\|sensors` | `POST /firmware`, ou `FLASH_CTRL` / `FLASH_DATA` avec `--usb` |
| `confirm` | `POST /firmware/confirm` |
| `monitor [--out <fichier>]` | décode le flux CAN en direct (WebSocket `/ws`, ou série avec `--usb`), enregistrement JSON Lines optionnel |
| `send` | envoie un message CAN à la main (`ping`, `pong`, `stop`, `reset`, `set`, `reqstatus`) |
| `replay <fichier>` | décode un fichier enregistré, ou le rejoue sur le bus (`--send`, `--usb`) |
| `telemetry [--record --rate <Hz> --duration <s> --scace --scace-address <adresse> --output <fichier>]` | `GET /telemetry`, ou son enregistrement dans `captures/` ; `--scace` lit aussi la sonde en BLE depuis le Mac |
| `config save [fichier]` / `config restore <fichier>` | `GET` / `POST /config` (fichiers dans `configs/`) |
| `set scace on\|off` | active ou désactive un réglage de `/config` |
| `download-hf` | télécharge la dernière capture haute fréquence dans `captures/` |
| `purge <secondes>` | purge Wi-Fi (60 s au plus, exacte si multiple de 5) |

Un seul processus peut ouvrir un port série à la fois ; le moniteur peut
afficher les trames avec retard lorsqu'il rattrape un buffer important.

## Reprise du banc

1. Vérifier `CAN OK`, pression/température et débit au repos sur l'écran.
2. Vérifier un PING/PONG via `coffeetool monitor` avant tout test applicatif.
   Les compteurs d'erreurs de bus ne servent plus : `twai_bus_errors` vaut
   toujours 0 en Rust.
3. Faire les essais SSR et dimmer sous 230 V avec une charge de test adaptée
   et les précautions décrites dans `firmware.md`.
4. Ne pas confondre validité du XDB401 et présence du XIAO : le premier enlève
   seulement les mesures de pression/température ; le second produit `CAN
   PERDU` après environ 3 s.
5. Un essai choisit l'un des deux montages : bureau (USB sans CAN) ou machine
   (CAN sans USB). Sur le bureau, la console UART de l'écran ne porte que du
   texte tant qu'aucune trame valide n'a été reçue du Mac ; les trames du pont
   ne partent ensuite.

## Pièges connus

### Écran

- **Registre de sortie du CH422G.** Il est partagé entre `CAN_SEL`/`USB_SEL`
  (EXIO5), `LCD_BL`, `LCD_RST`, `TP_RST` et `SD_CS`. Il n'a pas de lecture : le
  firmware garde un miroir et chaque écriture envoie l'octet complet. Une
  écriture directe (celle de l'exemple Waveshare par exemple) écrase les
  autres bits et coupe le transceiver CAN au moment d'allumer la dalle.
- **Timings du panneau RGB.** HSYNC `48/88/40`, VSYNC `3/32/13`, PCLK 16 MHz.
  À 12 MHz, la dalle boucle sur des mires de couleur.
- **Affinité des cœurs.** Cœur 0 : recopie d'affichage, rendu Slint, logique.
  Cœur 1 : radio, réseau, flash. La tâche `timer` d'esp-radio (rappels des
  minuteries des blobs Wi-Fi/BLE) est créée sans cœur par
  `esp-radio-rtos-driver` ; elle doit être épinglée au cœur de la radio, sinon
  l'ordonnanceur d'esp-rtos est corrompu (panique en 2 à 10 s avec BLE et Wi-Fi
  station ensemble, signatures diverses : garde de pile, `LoadProhibited`,
  `InstrProhibited` dans `TaskQueue::pop`). Le correctif est dans la copie
  vendorée d'esp-rtos.
- **Copies vendorées** (`firmware/rust/vendor`, `[patch.crates-io]` de
  `screen`) :
  - `esp-nvs` 0.6.2 mettait trois copies d'une page de 4 Ko sur la pile à
    l'ouverture (trame de 12,7 Ko) : débordement des 16 Ko de la pile du
    cœur 1. La copie met la page en tas et la lit en place (chemin NVS de
    ~18,1 à 6,6 Ko). Pile du cœur 1 laissée à 16 Ko.
  - `esp-hal` 1.2.2 : `Instant::now` lu depuis les deux cœurs peut reculer, et
    `Instant - Instant` panique alors (« Sub failed »), notamment dans le
    nettoyage du bus I2C quand l'ADS absent ne répond pas. La copie lit le
    compteur sous un verrou inter-cœurs et ne rend jamais moins que la valeur
    précédente.
  - `esp-rtos` 0.4.0 : épinglage de la tâche `timer` au cœur de la radio, et
    mesure de la pile des tâches radio (lignes `[pile] …` du rapport).
- **Ordre d'initialisation.** La lecture de l'identité du slot précède le
  remappage XIP du code en PSRAM, la radio démarre avant l'affichage, et les
  écritures flash gardent les interruptions actives. Le bootloader de l'écran
  (console IDF désactivée) remet GPIO43 en GPIO, sortie coupée : le firmware
  rebranche U0TXD avant le premier journal.
- **Piles et tas.** La pile du cœur 0 doit rester d'au moins 16 Ko
  (`stack-budget.x` le vérifie à l'édition de liens ; ~29 Ko en pratique, 18 Ko
  utilisés dès le démarrage). Tas radio interne de 120 Ko. Les tampons TCP et
  les en-têtes des quatre workers HTTP sont en PSRAM (56 Ko au plus) pour
  préserver la SRAM radio. Le rapport de `screen` donne `pile c0 max=…/… c1
  max=…/…`.
- **`cargo clippy` sur Xtensa** : `--lib --bin screen`, pas `--all-targets`.
- **`cargo fmt`** : à lancer dans le workspace et dans chaque crate autonome
  (`screen`, `sensors`, `screen-sim`) ; `cargo fmt --all --check` doit passer
  partout avant un commit.
- **CAN.** `TwaiConfiguration::new` prend RX puis TX. Sur l'écran, TX = GPIO20
  et RX = GPIO19 : passer GPIO20 puis GPIO19 inverse les deux et laisse l'écran
  sur « module interne injoignable » sans aucune trame des capteurs.
- **Watchdog RTC.** Interruption qui panique à 6 s, reset à 8 s. Les blocages
  observés masquent les interruptions : le reset est ce qu'on voit.
- **C++ archivé :** DIRAM (lier le contrôleur BLE coûtait ~18 Kio d'IRAM et
  faisait échouer les bounce buffers LCD), polices LVGL
  (`LV_USE_FONT_COMPRESSED`, signe moins Unicode absent) et ordre
  `service_screen::init()` avant `net_wifi::init()` n'ont plus d'objet en Rust.

### Radio

- BLE et Wi-Fi tournent ensemble (voir « Politique radio » dans `firmware.md`).
  Le C++ archivé les gardait exclusifs (`BLE_INIT: Malloc failed` signalait
  qu'ils avaient été chargés ensemble) : cette règle n'a plus d'objet.
- Une Lunar peut répartir l'UUID de service et le nom entre l'advertising et
  la scan response : le scan de la balance doit rester actif et sans
  déduplication.
- Une seule connexion BLE à la fois : pas de scan pendant une connexion.

### Module capteurs

- **UART.** Une entrée restée sur sa fonction IOMUX ne reçoit rien : forcer le
  pad en GPIO.
- **Digmesa.** GPIO44 (D7) compte les fronts descendants, pull-up interne
  désactivé, le filtre RC assurant la polarisation. C'est aussi l'entrée
  UART0 par défaut : `Input::new` le passe en GPIO, et cela doit se faire
  **avant** l'installation du gestionnaire d'interruption GPIO. Dans l'ordre
  inverse, la carte reste bloquée dans `Input::new` jusqu'au watchdog (retour
  à l'ancienne image trois secondes plus tard) ; le mécanisme exact n'est pas
  établi. Un comptage nul persistant venait d'un câble inversant VCC et SIGNAL
  (câblage final dans `cablage.md`).
- **Cœurs.** `sensors` tourne sur le seul cœur 0. Sans radio ni second cœur,
  les copies vendorées de `screen` n'y servent pas.
- **Premier démarrage d'une image qui se bloque.** Elle ne voit rien passer sur
  le CAN : le `main` ne cède la main qu'à la boucle de logique, donc le `PING`
  mis en file ne part pas si le blocage est en amont. Un retour de l'ancienne
  image à ~3,3 s est le watchdog (3 s) ; à 30 s, c'est le rollback
  `no-validate`.
- **XDB401** (ancien capteur I2C, archivé). Lire pression et température en
  deux transactions séparées ; une lecture groupée donne des pressions
  incohérentes. Appliquer la formule du fabricant à l'ordre big-endian des
  registres, même si le PDU est little-endian.
- **Pull-ups I2C.** Ce sont R13/R14 sur la carte `sensors` ; un dimmer muet
  au démarrage fait clignoter la LED à 1 Hz.
- **Dimmer.** Il lui faut le secteur pour devenir prêt : sans 230 V, son
  erreur de calibration est normale. Les flags viennent de `STATUS`
  (READY/ERROR), pas du seul registre `ERROR` ; `AC_FREQ` est informatif. Ne
  pas envoyer `RECALIBRATE` au boot. Une inversion phase/neutre sur son
  bornier a déjà été corrigée une fois. Registres : `dimmerlink-i2c.md`.

### OTA

- Un rollback n'est validé qu'avec une image réellement cassée ou une image
  `no-validate`.
- Après une erreur CRC de bloc côté `sensors`, une retransmission n'est pas
  distinguée explicitement d'un bloc suivant. Le CRC32 final protège l'image ;
  corriger ce point avant tout transport moins fiable qu'un CAN de banc.
- Le `PONG` est l'identité et la version observables d'une carte en boîte.
- Une écriture de configuration ou un reset NVS sur le cœur 0 attendait NVS
  pendant que l'OTA gardait la flash sur le cœur 1 en attendant le cœur 0 :
  interblocage. La configuration est donc refusée pendant un cycle et dès
  l'attente `STOP` d'un OTA.
- Infusion, purge, maintenance et régulation thermique restent exclues tant
  que l'écho `STOP` des capteurs n'est pas reçu pendant un OTA.

## Observations ouvertes

- **Perte de synchronisation RGB pendant un flash HTTP** (2026-09-11, C++).
  Elle revenait parfois seulement au rendu du L2 « mise à jour ». À revoir
  sur le Rust si le cas se reproduit : latence entre la recopie d'affichage
  et la pile Wi-Fi.
- **Écart connu** : `twai_bus_errors` vaut toujours 0 en Rust (le contrôleur
  n'expose pas ce compteur) ; le diagnostic de câblage par erreurs de bus du
  C++ est perdu.
- **NVS est rouvert à chaque requête** de l'écran.
- **Restent à valider sur la machine** : chauffe, purge, infusion (SSR vanne,
  dimmer en commande, chauffe GPIO4, comptage réel du débitmètre).

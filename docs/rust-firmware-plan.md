# Firmwares Rust — plan de réalisation

Document de travail pour écrire `protocol`, `coffeetool`, `screen` et `sensors`
en Rust. Les essais de `docs/rust-migration.md` sont clos : la viabilité est
acquise (affichage, BLE + Wi-Fi ensemble, flash, OTA, NVS). Ce plan est
supprimé une fois le travail fait.

## Avancement — 2026-10-09

Les statuts ci-dessous décrivent le code et les vérifications hors matériel.
Les essais de viabilité de `rust-migration.md` ne valent pas validation du
firmware assemblé : ils restent la référence technique pour sa mise en service.

| # | Statut | Vérification / suite |
|---|---|---|
| 1 | Intégrée | Tests hôte et Clippy du workspace verts. |
| 2 | Intégrée | 70 tests hôte ; Clippy propre. |
| 3 | Intégrée | 96 tests de logique ; builds Xtensa normal et `no-validate` réussis ; image normale 164 240 octets (< 1 Mo), Clippy propre. |
| 4 | Terminée et intégrée (`181a446`) | 124 tests + 1 doctest après assemblage ; Clippy propre. Parsing serde_json sans std, sortie compatible cJSON et clés HTTP conservées. Écarts de parsing documentés dans `screen-core/README.md`. |
| 5 | Intégrée | Carte, LCD, XIP, tactile et ADS constituent la base de l'assemblage. |
| 6 | Intégrée (`833acdf`, `c34a338`, `5703524`) | 9 tests simulateur, Clippy propre, 24 captures RGB565 examinées. Pas de comparaison pixel par pixel avec LVGL. |
| 7 | Intégrée (`4b43756`, `953a164`) | CAN, pont série, staging/proxy OTA, journaux UART sérialisés et récupération bus-off compilés pour Xtensa ; tests protocole/ACK/NOR/journaux verts. Brochage vérifié dans `archive/firmware/screen/main/board.h` : TX GPIO20, RX GPIO19. Rapport : `rust-task7.md`. |
| 8 | Intégrée (`8443b1b`, `4b6ec69`, `4363bc2`) | GATT compilé pour Xtensa, Clippy propre ; scan, arrêt provisioning, heartbeat, panne de l'hôte et horodatage revus. |
| 9 | Intégrée (`92163c3`, depuis `0de6bac`) | STA/AP, provisioning, HTTP/WS, SNTP, OTA locale/USB et NVS raccordés à `screen`. `screen-net` fait partie du workspace ; ses 7 tests passent. |
| 10 | Terminée hors matériel (`da486ae`) | UI ↔ cœur ↔ CAN/BLE/réseau/flash raccordés. Build release Xtensa complet, Clippy hôte/Xtensa et tests verts ; image OTA vérifiée et prête pour la session USB. |

La logique et l'UI tournent sur le cœur 0 ; les services radio et le propriétaire
flash sur le cœur 1. La lecture de l'identité du slot précède le remappage XIP,
la radio précède le LCD, et les écritures flash gardent les interruptions actives.
Les mesures ADS ne sont publiées qu'après une nouvelle tentative de lecture ;
les événements BLE conservent leur instant de réception. Le relâchement physique
de la purge reste indépendant de la durée de vie du bouton Slint.

### Problèmes rencontrés et résolus pendant l'assemblage

- **Intégration réseau.** Les fichiers `screen-net` préparatoires entraient en
  conflit avec la version complète de la tâche 9. Cette dernière est retenue,
  le workspace et le lockfile sont réunis, les dépendances dupliquées de
  `screen/Cargo.toml` supprimées et les modules réseau exportés par la bibliothèque.
- **Interblocage OTA/NVS.** Une écriture ou un reset de configuration pouvait
  attendre NVS sur le cœur 0 pendant que l'OTA gardait la flash sur le cœur 1
  et attendait le cœur 0. Configuration refusée pendant un cycle et dès
  l'attente STOP d'un OTA. La notification proxy `Finished` ne lève plus
  l'exclusion : seul `FinishUpload`, après libération de la flash, le fait.
- **Attente STOP des capteurs.** Une infusion, une purge, une commande de
  maintenance ou la régulation thermique pouvait repartir avant l'écho STOP.
  Elles restent maintenant exclues pendant cette attente ; un échec/délai OTA
  annule aussi l'attente, permettant un nouvel essai. Trois tests de régression
  couvrent configuration/OTA, reset pendant infusion et attente STOP/annulation.
- **WebSocket.** Les miroirs CAN/série préparatoires n'alimentaient pas les
  sessions réseau. Les réceptions CAN, émissions du cœur et émissions USB
  réussies alimentent maintenant les files indépendantes de chaque client WS.
- **Slint `no_std`.** Les éléments `Path` ne compilaient pas sans la feature
  de chemins ; son activation déclenchait une erreur `f32::rem_euclid` dans
  Slint 1.18.1 sans std. Les sept mêmes tracés sont conservés en SVG, pré-rendus
  au build et teintés par l'UI, sans moteur vectoriel à l'exécution.
- **SRAM.** L'édition de liens du firmware complet dépassait la SRAM de
  41 920 octets. Les tampons TCP RX/TX et en-têtes des quatre workers sont
  alloués explicitement en PSRAM (56 Ko au maximum), sans réduire la fenêtre
  TCP OTA ni le tas radio interne de 128 Ko. Le build final réussit avec
  20 912 octets de pile cœur 0 ; un script linker impose désormais au moins
  16 Ko. La pile cœur 1 reste interne, 16 Ko. Les estimations Clippy de pile
  sont documentées aux points concernés ; la marge réelle reste à observer
  sur carte.
- **Fin de démarrage réseau.** Un échec de création du contrôleur Wi-Fi
  publie maintenant `Off` et termine l'attente SNTP, au lieu de laisser l'UI
  attendre indéfiniment. La confirmation d'une image déjà valide reste
  idempotente, même sans capteurs ou STA.

### Vérifications et limites

Le workspace hôte passe **355 tests et 1 doctest** ; le simulateur passe
**9 tests** et produit les 24 captures RGB565. Les nouvelles icônes ont été
examinées sur l'accueil et les réglages. Ce n'est toujours pas une comparaison
pixel par pixel avec LVGL. Les tests HTTP hôte nécessitent un socket local :
ils passent hors de la restriction réseau du sandbox.

Build release Xtensa final réussi ; image IDF générée avec
`espflash save-image` uniquement, **2 730 208 octets**, sous la limite de slot
**3 584 000 octets**. Le vérificateur Rust `screen-net::image::verify` a relu
l'image et validé structure ESP32-S3, checksum XOR et SHA256. L'image est dans
`firmware/rust/screen/target/screen-0.4.0.bin` (artefact local non versionné).
Les commandes reproductibles sont dans `screen/NETWORK.md`.

Clippy du workspace hôte, du simulateur et du binaire/bibliothèque Xtensa est propre. Sur Xtensa, utiliser `--lib --bin screen`, pas
`--all-targets` : le harnais Rust `test` n'existe pas pour cette cible ; les
modules testables sont exécutés sur l'hôte. Le linker GCC signale un segment
RWX, attendu pour les données/instructions en RAM de cette cible.

**Préparation USB :** `screen/secrets.rs` a été généré localement depuis le
`secrets.h` C++ par un script ponctuel dans `/tmp`, sans consultation directe
ni affichage des valeurs. Le fichier est ignoré par Git et créé en mode 0600.
`AGENTS.md` interdit désormais explicitement la lecture directe de ces deux
fichiers et l'exposition de leurs valeurs. Le build release, Clippy Xtensa
et la vérification de l'image passent aussi avec ces constantes : HTTP et
provisioning peuvent désormais être exercés lors de la session USB.

Comme en C++, le jeton HTTP et le mot de passe WPA2 AP sont embarqués au
build ; les identifiants STA restent en NVS. Différence explicite : le C++
refuse de compiler sans `secrets.h`, tandis que le Rust compile en mode
réseau restreint et refuse `CHANGE_ME` comme jeton. Le code HTTP C++ ne
rejette pas explicitement son placeholder. Les constantes ont les
correspondances `kHttpBearerToken` → `HTTP_BEARER_TOKEN` et
`kProvisioningApPassword` → `PROVISIONING_AP_PASSWORD`, avec les mêmes valeurs
pour conserver les clients existants. La règle `.gitignore` manquante pour
`screen/secrets.rs` a été ajoutée.

Restent exclusivement matériels : démarrage réel, marge de pile/tas radio,
rendu et tactile, BLE/Wi-Fi simultanés, DHCP/reconnexion, HTTP/WS, persistance
NVS, OTA USB/HTTP et rollback. Aucun de ces points n'est validé par le build.

**Matériel : aucun flash, moniteur ou accès à la carte pendant ces travaux.**
Le point 10 est validé hors matériel : demander maintenant à David l'accès
à l'écran en USB sur le host. Reprendre alors les précautions,
les méthodes et les résultats de `rust-migration.md` avant tout essai ; le
premier parcours prévu reste l'écran sur le bureau, USB sans CAN, puis la machine.

## Première session USB — 2026-10-09

Écran sur le bureau, USB, sans CAN ni ADS ; balance et SCACE éteintes.
Image Rust installée par `parttool` dans `ota_0` puis `otatool
switch_ota_partition --name ota_0` : `otadata` `0x68`/`0x69`, les deux
`VALID`, donc pas de retour automatique. Coffeeflow 0.3.48 reste dans
`ota_1`. Retour, console fermée :
`otatool.py -p /dev/cu.usbmodem5B790235091 switch_ota_partition --name ota_1`.
Les images suivantes se réécrivent dans `ota_0` sans nouvelle bascule.

### Corrigé pendant la session (vérifié sur carte)

- **Pile du cœur 1 et NVS.** `esp_nvs` 0.6.2 mettait trois copies d'une page
  de 4 Ko sur la pile à l'ouverture (`scan_sectors`, trame de 12,7 Ko) :
  débordement des 16 Ko du cœur 1. Copie vendorée dans
  `firmware/rust/vendor/esp-nvs` (`[patch.crates-io]` de `screen`), page en
  tas et lue en place ; 126 tests de la crate verts. Chemin NVS ~18,1 → 6,6 Ko.
  Mesuré : cœur 1 à 8,6 Ko au plus (Wi-Fi), 6,3 Ko (BLE). Pile du cœur 1
  laissée à 16 Ko.
- **Pile du cœur 0.** 18,2 Ko utilisés dès le démarrage sur 20,9. Tas radio
  interne ramené de 128 à 120 Ko : la pile du cœur 0 passe à ~29 Ko. Tas
  interne libre minimal mesuré : 30 Ko avec Wi-Fi + BLE, 65 Ko Wi-Fi seul,
  77 Ko BLE seul.
- **Mesure de pile.** `screen/src/stack_watermark.rs` : motif peint au
  démarrage, ligne `pile c0 max=…/… c1 max=…/…` dans le rapport toutes les
  10 s. Pour le cœur 1, la fermeture de démarrage déposée par esp-hal au bas
  de la pile est sautée (sinon la mesure dit « pleine »).
- **Horloge non monotone entre les cœurs.** `esp_hal::time::Instant::now`,
  lu depuis les deux cœurs, peut reculer ; `Instant - Instant` panique alors
  (« Sub failed »), notamment dans le nettoyage de bus I2C d'esp-hal quand
  l'ADS absent ne répond pas, et dans l'interruption de recopie. Copie
  vendorée `firmware/rust/vendor/esp-hal` (1.2.2) : `time::now` lit le
  compteur sous `esp_sync::RawMutex` (verrou inter-cœurs, interruptions
  masquées, comme IDF) et ne rend jamais moins que la valeur précédente.
  Recopie et mesure de rendu en µs saturés. Recopie mesurée ensuite : 715 à
  1 083 µs, `ret=0`.
- **Console lisible.** Les trames du pont série ne partent qu'après la
  première trame valide reçue du Mac ; avant, l'UART ne porte que le texte.
- **Watchdog RTC.** Interruption qui panique à 6 s (trace du cœur 0), reset
  à 8 s. Jamais vue se déclencher : les blocages observés masquent les
  interruptions.
- **Points de passage** (retirés depuis, avec tout le code de diagnostic) :
  dernier fichier:ligne, nombre de passages et instant (ms) de chaque cœur,
  en RAM RTC lente, affichés au démarrage suivant. Ils n'ont jamais désigné
  une cause.

### Problème ouvert : BLE + Wi-Fi station plantent ensemble

| Variante (build) | Résultat |
|---|---|
| Wi-Fi station seul (`DIAG_NO_BLE=1`) | stable (30 s et plus) |
| BLE seul, scan sans appareil (`DIAG_NO_WIFI=1`) | stable (60 s et plus) |
| BLE + station | blocage ou panique entre 1,4 et 3,7 s |
| idem, station en `PowerSaveMode::Minimum` | identique : ce n'est pas la veille Wi-Fi |
| BLE lancé après l'association (`DIAG_BLE_AFTER_STA=1`) | associé à 2 685 ms, BLE à 2 774 ms, plantage aussitôt : régime établi, pas la phase d'association |

Les deux cœurs s'arrêtent à la même milliseconde. Panique double observée :
cœur 1 `LoadStoreError` dans `pm_process_tim` (blob Wi-Fi, traitement du TIM
des balises, propre au mode station), lecture d'octet à `0x42170428`, une
adresse de la zone de code ; cœur 0 `InstrProhibited` (PC 0) dans
`esp_rtos::TaskQueue::pop`, appelé par `cross_core_yield_handler`.
L'ordonnanceur d'esp-rtos est corrompu.

**Trou de couverture des essais :** 05, 07 et 08a combinaient BLE et Wi-Fi
en **point d'accès** ; 08c a utilisé la station **sans BLE**. BLE + station
n'a jamais tourné. Versions identiques à l'essai 07 et dernières publiées :
esp-radio 1.0.0-beta.1, esp-rtos 0.4.0, esp-hal 1.2.2, trouble-host 0.7.0.

**Prochaine étape proposée :** reproduire hors de `screen`, avec l'essai 07
passé en station (même affichage, même radio), puis la même chose sans le
remappage XIP. Ça sépare un défaut d'esp-radio/esp-rtos d'une interaction
avec le code en PSRAM (l'adresse fautive tombe dans le code remappé) ou avec
le placement des tampons du blob. Pistes ensuite : cœur des tâches radio
(contrôleur BLE sur `AppCpu`, tâche Wi-Fi sur le cœur qui crée le
contrôleur), allocations du blob (tas interne ou non).

### Essai 10 `10-sta-ble` (2026-10-09, après le commit précédent)

Reproducteur hors de `screen` : `firmware/rust/trials/10-sta-ble`, cœur 1
radio, cœur 0 quasi vide, identifiants relus en NVS en lecture seule,
esp-rtos 0.4.0 vendoré dans l'essai (patch `// Essai 10 :` : chaque pile de
tâche créée par les blobs est peinte, inscrite et mesurée ; pile
supplémentaire par `TRIAL_EXTRA_TASK_STACK` au build), contrôles de pile
d'esp-rtos tous actifs (plage, mot témoin, garde matérielle). Features
cumulables : `ble-controller`, `ble-host`, `ble-scan`, `no-wifi`, `ps-min`,
`psram-heap` (PSRAM + allocateur par cœur de `screen`), `xip`,
`core0-load` (interruption de niveau 3 à 820 Hz qui recopie 32 Ko
PSRAM → SRAM en parcourant un tampon de 768 Ko, tâche qui alloue et calcule,
réveils du cœur 0 depuis le cœur 1 par `Channel`).

| Variante | Résultat |
|---|---|
| `ble-scan` (scan + station) | **tient** (50 s et plus) ; tâches radio : `btController` 1 840/8 272 (22 %), `wifi` 2 992/6 736 (44 %), `timer` 1 232/8 272 (14 %, **non épinglée**, `cœur=-1`) ; pile c1 13 152/16 384 |
| `ble-scan,xip,core0-load`, 1er passage | tient, mais charge **inactive** (alarme TIMG non réarmée : `irq=1`) |
| idem, 2e passage | tient, mais recopie **supprimée par le compilateur** (`max=12us`, destination jamais relue) |
| idem, 3e passage (`black_box`, `memcpy` vérifié au désassemblage) | **tient** ; recopie réelle `max=978us` (screen : 715 à 1 083 µs), `irq=5146`, `evenements_c0=106` au relevé |

Conclusions admises : BLE scan + Wi-Fi station tiennent au minimum, avec
XIP, PSRAM, allocateur par cœur, réveils inter-cœurs et une recopie
d'affichage synthétique de durée comparable (978 µs, PSRAM → SRAM sur
768 Ko). Ce qui n'est **pas** établi (remarques de relecture) : que le
déclencheur soit dans le LCD, Slint ou la logique (la charge synthétique
n'est pas le DMA LCD ni le rendu Slint) ; que les piles radio aient de la marge **dans
`screen`** (pile du cœur 0 six fois plus petite, interruptions en plus).
Les points de passage ne donnent que le dernier endroit instrumenté : ils
n'ont jamais désigné une cause.

Défauts d'instrumentation :
- Corrigé : `screen/src/stack_watermark.rs`, `usage()` du cœur 1 saute au
  plus 512 octets (la fermeture de démarrage) et rend « invalide » sans
  motif retrouvé, au lieu de sauter sans borne.
- Corrigé dans la copie de `screen` (`vendor/esp-rtos`), pas dans celle de
  l'essai : `task_stacks()` pouvait mesurer une pile libérée puis
  réattribuée pendant le parcours ; un compteur de génération par case
  écarte la mesure.
- Non corrigé : `trials/10-sta-ble`, variante `scan-load` (sans PSRAM) :
  2 avertissements Clippy depuis la dernière modification de `src/load.rs`.

Corrigé dans `screen` : la boucle de scan BLE attend 20 ms avant son
`continue` quand `scan_can_start` est faux.

**Prochaine piste (décidée par David) : essais directement dans `screen`.**
Fait : patch d'esp-rtos porté dans `firmware/rust/vendor/esp-rtos`
(`radio_stacks.rs`, marqueur `// Coffeeflow :`), utilisé par `screen` ;
lignes `[pile] <tâche> taille=… demandé=… max=… (…%) cœur=…` dans le rapport
toutes les 10 s, et `[pile] création …` à chaque tâche créée par un blob ;
pile supplémentaire par `RADIO_EXTRA_TASK_STACK` au build ; contrôles de pile
d'esp-rtos tous actifs dans `screen/.cargo/config.toml` (y compris la
vérification de plage, qui paniquait sur l'état déjà corrompu : une panique
de ce contrôle est maintenant un résultat). Résultat : voir « Cause trouvée » ci-dessous.

### Cause trouvée : tâche `timer` d'esp-radio non épinglée (2026-10-09)

Essais dans `screen`, écran au bureau, rapport à 1 s (`REPORT_PERIOD_MS=1000`),
image écrite par `espflash write-bin … 0x120000` (offset d'`ota_0`, plus
rapide que `parttool`).

| Variante (build) | Affichage | Résultat |
|---|---|---|
| complète | actif | panique en 2 à 10 s, à chaque démarrage |
| `DIAG_NO_CAN`, `DIAG_NO_ADS` | actif | 3 plantages en 50 s, parfois avant l'association (1,3 s) |
| idem + `DIAG_NO_DISPLAY` | absent | tient 128 s (mais disposition mémoire différente) |
| complète, `timer` épinglée au cœur 1 | actif, 35 tr/s, `ret=0` | **tient 556 s**, puis 116 s sur un autre démarrage |
| même code, `timer` non épinglée | actif | 4 plantages en 120 s |

- Les piles radio ont de la marge, y compris dans le dernier relevé avant
  plantage (RAM RTC, 12 ms avant) : `btController` 1 856/8 272,
  `wifi` 2 976/6 736, `timer` 1 648/8 272.
- Les plantages ont des signatures diverses, toutes d'ordonnanceur corrompu :
  contrôle de plage d'esp-rtos sur le cœur 0 (tâche principale avec SP `0x1d`,
  puis tâche nulle) ; garde de pile matérielle du cœur 1 en double exception,
  SP passé d'un coup 16 Ko plus bas ; `LoadProhibited` sur le cœur 1.
- `esp-radio-rtos-driver` 0.4.2 crée sa tâche `timer` (rappels des
  minuteries des blobs) sans cœur : elle tourne alors sur l'un ou l'autre.
  Correctif dans `vendor/esp-rtos/src/esp_radio/mod.rs` : une tâche `timer`
  sans cœur est épinglée au cœur qui la crée, celui de la radio.
- Non établi : le mécanisme exact. La recopie d'affichage (≈1 ms au niveau
  3 toutes les 1,2 ms sur le cœur 0) semble élargir une course entre les
  rappels exécutés sur le cœur 0 et le reste de la radio sur le cœur 1 ;
  l'essai 10, avec une charge synthétique comparable, ne l'a pas déclenchée.
- Faux départ à ne pas refaire : espflash attribue `0x3fce04d0` à
  `g_shared_buffers`, symbole de la ROM ESP32-S2, sans rapport ici.
- Les variantes `nofill` (DMA sans recopie) et une première `pin` ont tourné
  sans affichage (`Display` lâché par erreur, `tr=0/s`) : résultats écartés.

Gardé dans `screen` : `REPORT_PERIOD_MS` (10 s par défaut, piles des deux
cœurs et des tâches radio en tête du rapport). Retiré : points de passage,
relevé RTC des piles radio, interrupteurs `DIAG_*`, repères `[diag]`.

### Premier essai sur la machine (2026-10-09)

- Avec l'image ci-dessus : écran sur « module interne injoignable » après
  1-2 s, aucune trame des capteurs. Cause : `TwaiConfiguration::new` prend
  RX puis TX, `screen` passait GPIO20 puis GPIO19, donc RX et TX inversés
  (C++ : TX = GPIO20, RX = GPIO19). `sensors` (RX = GPIO8, TX = GPIO7) était
  juste. Débit, filtre et `CAN_SEL` identiques au C++.
- Image corrigée, sans code de diagnostic, flashée par HTTP
  (`coffeetool flash --target screen`) dans `ota_1` à la place de coffeeflow
  C++ 0.3.48, puis confirmée. Trames capteurs reçues, `confirm` accepté.
- Pendant l'upload, l'écran est resté sur « module interne injoignable ».
  Tranché par David : « mise à jour » passe avant (`b099e7e`), le verrou de
  sécurité reste en tête. Vérifié sur la machine.
- Page diagnostic portée du C++ (pastilles d'état, erreurs CAN, poids, deux
  colonnes de versions, ligne SRAM ; les libellés étaient centrés sur la
  valeur faute de `y`) et version unique `protocol::FIRMWARE_VERSION`,
  passée à 0.4.1 (`9071370`). Référence visuelle C++ :
  `archive/firmware/screen/ui_sim/build/snapshots/`. Écran 0.4.1 flashé par
  HTTP et confirmé.
- Erreurs de bus côté capteurs (`twai_bus_errors`, cumul depuis le
  démarrage du module) : 1 285 623 puis 1 295 131 à 782 s d'uptime, CAN
  pourtant fonctionnel (~20/s). Pas comparé au C++ : à mesurer avec l'écran
  C++ ou en regardant la source côté `sensors` 0.3.48.

### Première installation de `sensors` Rust : échec (2026-10-09)

Image : `firmware/rust/sensors`, `cargo build --release --features
no-validate`, 0.4.1, 164 240 octets (`espflash save-image --chip esp32s3
--flash-size 8mb`). Envoyée par l'écran Rust 0.4.1 :
`coffeetool flash <image> --target sensors`.

Observé dans `coffeetool monitor` :
- Transfert complet : 82 `BLOCK_ACK`, `FLASH_PROGRESS` 164 240,
  `FLASH_DONE`, `END`. La chaîne écran Rust → `ota_staging` → CAN →
  récepteur C++ de `sensors` 0.3.48 fonctionne.
- Aucune trame de l'image Rust (ni `PING` ni `LOG BOOT`, envoyés pourtant
  par `Machine::start`).
- 3,6 s après `FLASH_DONE`, `sensors` **0.3.48** redémarre (`BOOT`,
  `READY`, uptime 0) : le bootloader est revenu au C++. L'écran a vu
  `PRESENCE_LOST` puis `PRESENCE_RESTORED`.

Analyse (rien de vérifié sur carte) :
- Le C++ redémarre 50 ms après `FLASH_DONE` (`main.cpp`, après
  `esp_ota_set_boot_partition`). Les ~3,5 s restantes correspondent au
  watchdog RTC de l'image Rust (`sensors/src/watchdog.rs`, `TIMEOUT` = 3 s,
  étape 0 = reset système). `no-validate` aurait tourné 30 s.
- Donc : l'image Rust a démarré puis s'est bloquée **avant d'atteindre la
  boucle `logic`** (qui nourrit le watchdog), ou dans son premier tour. Une
  panique redémarrerait tout de suite (`custom_halt`), pas en 3 s. Un rejet
  par le bootloader reviendrait au C++ sans délai. À confirmer : un
  bootloader qui refuse l'image peut aussi la faire tenter puis échouer.
- Comme `main` ne cède jamais la main avant `logic(machine).await`, la tâche
  CAN ne tourne pas avant : le `PING` mis en file par `machine.start()` ne
  part jamais si le blocage est en amont. L'absence de trame ne situe donc
  pas le blocage.
- En-tête d'image vérifié (`esptool image-info`) : ESP32-S3, DIO 40 MHz,
  8 Mo, révision min v0.0, SHA valide, descripteur d'app `sensors` 0.4.1.
  Bootloader C++ : `BOOTLOADER_APP_ROLLBACK_ENABLE=y`,
  `BOOTLOADER_WDT_ENABLE=y` (9 s), pas de secure boot.
- Étapes de `main` avant `logic`, candidates au blocage : `esp_hal::init`
  (160 MHz), sorties sûres, `watchdog::arm`, `clock::init`, logger +
  `UartTx` sur GPIO43 (esp-println `uart` attend la FIFO), `lockout::init`
  (RAM RTC rapide), `esp_rtos::start`, `FlashOta::new` (table de
  partitions, `booted_partition`), `pending_state` (otadata), TWAI
  (RX GPIO8, TX GPIO7 : juste), I2C du dimmer (`unwrap`), `flow::init`
  (interruption GPIO44, front descendant), `machine.start()`/`ready()`.
- `sensors` utilise esp-hal, esp-rtos et esp-storage du registre, **pas**
  les copies vendorées de `screen` (horloge monotone, piles radio) ; sans
  radio ni second cœur, ça n'a pas été jugé nécessaire.

Pistes pour la prochaine session (à choisir avec David ; pas de console USB
sur la machine) :
1. **Jalons sur le CAN — fait, pas encore essayé.** TWAI démarré juste
   après les sorties sûres et le watchdog, en bloquant ; `LOG BOOT`
   `arg16` = étape, `arg32` = 0x52555354 (« RUST »), 20 ms au plus chacun.
   Étapes : 1 TWAI, 2 console, 3 verrou, 4 esp-rtos, 5 table de partitions,
   6 otadata, 7 I2C, 8 `machine.start`, 9 débitmètre, 10 (par la tâche CAN
   asynchrone) boucle de logique lancée. Aucun jalon : l'émission CAN ou
   ce qui précède l'étape 1 est en cause. LED du XIAO (GPIO21, libre en
   C++ comme en Rust, visible du haut de la machine) : éteinte = image Rust
   pas démarrée ; fixe = démarrée, bloquée avant la boucle (puis éteinte au
   reset du watchdog) ; clignotante à 1 Hz = boucle de logique lancée. Image
   `/tmp/sensors-jalons-no-validate.bin` (version toujours 0.4.1).
2. **Jalons en RAM RTC** survivant au reset du watchdog : inutile seul, le
   C++ qui reprend la main ne les lit pas.
3. **Reproduire au bureau** sur un XIAO ESP32-S3 de rechange, console
   esp-println en `jtag-serial` (ou `auto`) pour la voir par l'USB natif.
4. Allonger le watchdog (ex. 8 s) pour distinguer un blocage d'une
   lenteur.

Prochaine action (David, après ce commit) : flasher
`/tmp/sensors-jalons-no-validate.bin` par `coffeetool --ip 192.168.2.196
flash … --target sensors`, `coffeetool monitor` ouvert, en regardant la LED
du XIAO. Relever : le dernier jalon `LOG BOOT arg16=N arg32=0x52555354`
reçu, l'état de la LED, un éventuel `PING` v0.4.1, et l'instant du retour de
0.3.48. Décision : les images d'essai de `sensors` restent en 0.4.1 (pas
d'incrément à chaque image tant que la première installation n'a pas
abouti).

État des cartes à ce moment (dépassé, voir « `sensors` Rust installé ») :
- `sensors` : C++ 0.3.48 actif et `VALID` ; l'autre slot contient l'image
  Rust `no-validate` (non validée, jamais rechoisie par le bootloader).
- Écran : 0.4.1 dans `ota_1`, actif et confirmé ; `ota_0` = build
  intermédiaire (priorité « mise à jour », 0.4.0). Plus de C++ sur l'écran.
- Images : `/tmp/screen-0.4.1.{bin,elf}` (depuis `9071370`),
  `/tmp/sensors-jalons-no-validate.{bin,elf}` (depuis `4802dfc`, `cargo
  build --release --offline --features no-validate` dans
  `firmware/rust/sensors`, puis `espflash save-image --chip esp32s3
  --flash-size 8mb --skip-update-check`). Perdues au redémarrage du Mac.

### `sensors` Rust démarre : blocage du débitmètre corrigé (2026-10-09)

Trois images `no-validate` (0.4.1) envoyées par `coffeetool flash … --target
sensors`, suivies dans `coffeetool monitor`. La LED du XIAO n'aide pas : la
LED du shield Grove clignote déjà à 1 Hz ; la nôtre passe à 5 Hz.

| Image | Dernier jalon | Retour à 0.3.48 |
|---|---|---|
| jalons 1-10 | 8 (`machine.start`) | 3,3 s après : watchdog |
| `flow::init` découpé (81 gestionnaire installé, 82 pad en GPIO, …) | 81 | 3,4 s après : watchdog |
| ordre inversé (`Input::new` avant le gestionnaire) + registres GPIO + garde-fou anti-tempête | jusqu'à 10, `PING` v0.4.1, `READY` | 30 s après : retour en arrière `no-validate` |

- Cause : installer le gestionnaire d'interruption GPIO **avant** de passer
  GPIO44 de U0RXD en GPIO bloque la carte dans `Input::new` jusqu'au
  watchdog. Dans l'ordre inverse : gestionnaire jamais appelé en 100 ms,
  aucune interruption GPIO en attente, `GPIO_PIN44_REG` = `0x2500` (front
  descendant, activée vers le cœur 0). Mécanisme non établi : le garde-fou
  n'a jamais tourné dans l'ordre fautif ; une tempête d'interruptions reste
  l'hypothèse.
- Correctif gardé (`flow.rs`) : pad en GPIO d'abord, puis gestionnaire et
  `listen`. Jalons, relevés de registres et garde-fou retirés ; TWAI de
  nouveau démarré directement en asynchrone.
- Image propre (164 720 octets) : démarrage 0,3 s après `FLASH_DONE`,
  `PING` v0.4.1, `BOOT`, `OTA_PENDING_VERIFY`, `READY`, `PONG` de l'écran,
  `STATUS_FLOW` / `_ACTUATORS` / `_HEATING` servis, `CONFIRM_SENSORS_OTA`
  ignoré, `OTA_ROLLBACK` à 30,0 s, 0.3.48 revenu. Écran sans redémarrage
  (uptime continu) ; la perte du WebSocket vue à l'essai précédent ne s'est
  pas reproduite.
- `sensors` Rust tourne sur le seul cœur 0 (second cœur jamais démarré) ;
  le C++ laisse ses trois tâches sans affinité sur deux cœurs, interruptions
  sur le cœur 0. Sans effet sur ce blocage, qui survient pendant
  l'initialisation, sur le cœur 0 dans les deux cas.

Comparé à une capture de 0.3.48 au repos, identique au C++ :
`DIMMER_MAINS_FREQ` toutes les 0,4 à 1 s, `STATUS_HEATING` toutes les
~300 ms pour 250 ms demandées, `CONFIRM_SENSORS_OTA` de l'écran chaque
seconde (même à une image déjà validée).

### `sensors` Rust installé et validé (2026-10-09)

- Image normale 0.4.1 reçue par le C++ 0.3.48 : `OTA_VALIDATED` 1,2 s après
  le démarrage, au premier `CONFIRM_SENSORS_OTA` ; toujours là 40 s après.
- Écran puis `sensors` passés en **0.4.2** : écran par HTTP (`ota_0`,
  confirmé) ; `sensors` 0.4.2 reçu par le **récepteur Rust** 0.4.1 :
  82 blocs, `FLASH_DONE`, démarrage 0,3 s après, `OTA_VALIDATED` 1,1 s
  après le démarrage. Le C++ n'est plus sur `sensors` : l'autre slot
  contient le Rust 0.4.1.
- `uptime_s` des capteurs dans `/telemetry` : les capteurs ne donnent leur
  uptime qu'au PING de démarrage et aux vérifications de présence, rares en
  service ; l'écran le laissait figé (0 en pratique), comme le C++. Écart
  voulu (0.4.2) : tant que les capteurs sont vivants, l'écran ajoute le
  temps écoulé depuis le dernier PING/PONG. La détection d'un redémarrage
  compare toujours la valeur reçue. Vérifié : 35 s.
- La LED à 5 Hz n'a pas été vue. Le clignotement à 1 Hz est sans doute le
  voyant de charge du shield (sans batterie) ; la LED utilisateur du XIAO
  (orange, GPIO21, près de l'USB-C) est petite et peut être masquée.
- Écart connu, non corrigé : `twai_bus_errors` vaut toujours 0 en Rust
  (`board.rs`) ; le contrôleur n'expose pas ce compteur, que le pilote IDF
  tenait par interruption. Le diagnostic de câblage du C++ (1,2 million
  d'erreurs relevées sous 0.3.48) est donc perdu.

État des cartes :
- `sensors` : Rust 0.4.2 actif et validé ; l'autre slot, Rust 0.4.1 validé.
  Plus de C++ sur `sensors`.
- Écran : 0.4.2 dans `ota_0`, actif et confirmé ; `ota_1` = 0.4.1.
- Images : `/tmp/screen-0.4.2.{bin,elf}`, `/tmp/sensors-0.4.2.{bin,elf}`.
  Perdues au redémarrage du Mac.

### Pour reprendre

- Build, depuis `firmware/rust/screen`, après
  `source ~/export-esp.sh` : `cargo build --release --offline`, plus au
  besoin `REPORT_PERIOD_MS=<ms>` ou `RADIO_EXTRA_TASK_STACK=<octets>`. Image :
  `espflash save-image --chip esp32s3 --flash-size 16mb --skip-update-check
  target/xtensa-esp32s3-none-elf/release/screen /tmp/screen-<variante>.bin`,
  ELF copié en `/tmp/screen-<variante>.elf` pour `espflash monitor --elf`.
- Gardés : `stack_watermark`, le watchdog en deux temps et les trois copies
  vendorées (esp-nvs, esp-hal, esp-rtos : mesure des piles radio et
  épinglage de la tâche `timer`).
- Défaut relevé, non corrigé : NVS est rouvert à chaque requête.
- État des cartes : voir la fin de « `sensors` Rust installé et validé ».

## Référence de comportement

Le C++ est dans `archive/firmware/` (`screen`, `sensors`, `common`, `tools`),
toujours compilable. **Le comportement à reproduire est celui du code
archivé**, au bit près sur le bus et à la clé près en HTTP (`/telemetry`,
`/config` schéma 10, `/action`, `/hf-capture`, `/firmware`,
`/firmware/confirm`, WebSocket `/ws`). `docs/firmware.md` et `docs/ui.md`
décrivent ce code ; en cas de désaccord, le code archivé fait foi, et l'écart
se note dans le rapport de la tâche.

Les essais `firmware/rust/trials/` sont la **référence technique** (pilote
d'affichage, XIP, radio, flash, OTA, NVS) : on en recopie les modules, on ne
dépend pas d'eux.

## Ce qui change par rapport au C++ (décisions du 2026-10-09)

- **Radio.** BLE et Wi-Fi tournent ensemble. Avec des identifiants Wi-Fi en
  NVS, l'écran est **toujours en STA**. Le « mode Wi-Fi » modal disparaît,
  et l'**infusion est permise Wi-Fi actif**. HTTP garde ses refus `409`
  pendant infusion et purge (`/config`, `/firmware`), et ne peut jamais
  lancer d'infusion.
- **Icône Wi-Fi des réglages** : bascule ON/OFF du Wi-Fi pour la session.
  Non mémorisée : chaque démarrage remet le Wi-Fi ON.
- **Sans identifiants** : point d'accès de provisioning avec page de saisie
  (comme le C++), plein écran « connectez-vous au réseau `<SSID>` et entrez
  vos identifiants » par-dessus toute l'UI, **BLE coupé** pendant ce mode.
  Une panne STA ne relance jamais l'AP ; seul « oublier le réseau » le fait.
- **Scan BLE** : 50 % du temps radio au plus (fenêtre = moitié de
  l'intervalle), jamais continu. Défaut : intervalle 60 ms, fenêtre 30 ms
  tant que la balance manque ; politique de `ble_scan_policy.h` pour le
  reste (pas de scan pendant infusion/purge balance connectée, ni pendant
  une connexion).
- **UI en Slint**, rendu ligne par ligne, valeurs rafraîchies à **4 Hz par
  groupe**, un groupe par rendu (essai 06/07). Mêmes écrans, textes, cotes et
  couleurs que l'UI LVGL archivée.
- **Versions** : les deux firmwares Rust partent de **0.4.0** (PING/PONG,
  `/telemetry`). `screen` exige toujours `sensors` ≥ 0.2.63 pour confirmer.

## Organisation du code

```
firmware/rust/
  Cargo.toml        workspace hôte : protocol, screen-core, coffeetool
  protocol/         no_std sans alloc ; feature `std` pour coffeetool
  screen-core/      no_std + alloc, logique de l'écran, testée sur le Mac
  coffeetool/       binaire macOS
  screen/           app Waveshare, crate xtensa autonome (hors workspace)
  sensors/          app XIAO, crate xtensa autonome (hors workspace)
  trials/           essais, inchangés
```

`screen` et `sensors` ont leur propre `.cargo/config.toml`, leur
`rust-toolchain.toml` (`esp`) et leur `Cargo.lock`, comme les essais ; ils
dépendent de `protocol` (et `screen` de `screen-core`) par chemin. Le
workspace hôte compile avec la toolchain `stable`.

- `log_codes.yaml` est dans `protocol/`. `protocol/build.rs` en génère le
  Rust. Le codegen archivé (`archive/firmware/common/codegen`) lit le même
  fichier.
- `screen-core` reprend `archive/firmware/screen/main/core/` : machine
  d'état, infusion, `thermal_control`, `brew_heating`, `shot_summary`,
  `sample_schedule`, `config` (validation, JSON, migrations), calibration,
  événements. Aucune dépendance au matériel : il reçoit des échantillons
  horodatés (horloge monotone en µs) et émet des commandes. Les tests de
  `archive/firmware/screen/test/` sont portés.
- `screen` = carte (CH422G, panneau, bounce buffers, XIP, GT911, ADS1115),
  CAN (TWAI GPIO19/20, CAN_SEL haut), BLE, réseau, OTA local, proxy OTA
  vers `sensors` (`ota_staging`), pont série UART0, NVS (`esp-nvs` par
  `flash.rs` de 08c), UI Slint.

Cœurs : cœur 0 = recopie d'affichage + rendu Slint + logique ; cœur 1 =
radio, réseau et flash (mode `comme-idf` de 08a, sans section critique).
Deux tas (essai 07) : SRAM interne pour la radio, PSRAM pour le reste.

À reprendre des essais, obligatoirement : `otadata.rs` (jamais `Ota`
d'esp-bootloader-esp-idf), `custom_halt` qui redémarre, watchdog RTC
réarmé au début de `main`, U0TXD rebranché sur GPIO43, radio démarrée avant
l'affichage, fenêtre TCP agrandie pour l'OTA.

## Règles pour toutes les tâches

- Commentaires en français, qui expliquent le pourquoi sans renvoyer à un
  fichier de `docs/`.
- Pas de flash, pas de moniteur série, pas d'accès à la carte : une tâche
  s'arrête à « compile, tests hôte verts, clippy propre ». Les essais sur
  carte sont décidés par David.
- Ne jamais consulter directement `secrets.h`, `secrets.rs` ni un fichier
  de secrets, ni afficher leurs valeurs. Une conversion locale explicitement
  demandée peut passer par un script sans exposition du contenu, comme défini
  dans `AGENTS.md`. Jeton HTTP et mot de passe de l'AP : un `secrets.rs` ignoré
  par Git, avec `secrets.example.rs` versionné ; les correspondances des noms
  C++/Rust sont documentées ci-dessus.
- Un commit par tâche, sur la branche de la tâche, message en anglais.

## Tâches

| # | Tâche | Dépend de | Modèle |
|---|---|---|---|
| 1 | `protocol` + workspace hôte | — | Sonnet |
| 2 | `coffeetool` | 1 | Sonnet |
| 3 | `sensors` | 1 | Sonnet |
| 4 | `screen-core` | 1 | Sonnet |
| 5 | `screen` : squelette, carte, affichage, Slint vide | 1 | Sonnet |
| 6 | UI Slint + simulateur | 4 | Sonnet |
| 7 | `screen` : CAN, pont série, proxy OTA sensors | 4, 5 | Sonnet |
| 8 | `screen` : BLE (Acaia, SCACE) | 4, 5 | Sonnet |
| 9 | `screen` : Wi-Fi, provisioning, HTTP, WS, SNTP, OTA local, NVS | 4, 5 | Sonnet |
| 10 | Assemblage `screen` (UI ↔ cœur ↔ services) | 6–9 | Sonnet |

Puis les essais sur carte, dans l'ordre : `screen` sur le bureau (USB, sans
CAN : UI, BLE, Wi-Fi, OTA) ; `screen` sur la machine (CAN, ADS1115) ;
première installation de `sensors` construite **sans validation** (feature
`no-validate`, rollback garanti), puis la version qui se valide.

### `sensors` : points imposés

Reprend `archive/firmware/sensors/main/` entier (bail, présence, verrou 60 s
en RAM RTC persistante levé seulement au power-on, chauffe modulée 1 s et
`restart_window`, garde de passage par zéro du dimmer, `MAINTENANCE_VALVE`,
REQSTATUS, canal pression gardé sans capteur, débitmètre en interruption,
réception `FLASH` par blocs de 2 Ko acquittés, confirmation par
`CONFIRM_SENSORS_OTA`, rollback après 30 s). Sorties sûres avant toute autre
initialisation. Jamais la commande `0x03` du DimmerLink. Image ≤ 1 Mo
(`ota_staging`). Feature `no-validate` : ignore `CONFIRM_SENSORS_OTA` et
redémarre après 30 s.

### `coffeetool` : périmètre

Le tableau « `coffeetool` : un seul outil » de `docs/rust-migration.md`,
tel quel (HTTP par défaut, `--usb`, `telemetry --record`, `--scace` par
`btleplug`, formats de fichiers inchangés). Les scripts Python remplacés
restent dans `archive/firmware/tools`.

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

## Première session USB — 2026-10-09 (en cours, bloquée)

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
- **Points de passage.** `screen/src/breadcrumb.rs` et la macro `crumb!` :
  dernier fichier:ligne, nombre de passages et instant (ms) de chaque cœur,
  en RAM RTC lente ; affichés au démarrage suivant (`[diag] avant reset,
  cœur N : …`). Posés aux têtes de boucle et dans le BLE et la boucle de
  rendu.

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
| idem, 3e passage (`black_box`, `memcpy` vérifié au désassemblage) | flashé en dernier, **résultat non relevé** |

Conclusions admises : BLE scan + Wi-Fi station tiennent au minimum, avec
XIP, PSRAM, allocateur par cœur et réveils inter-cœurs. Ce qui n'est **pas**
établi (remarques de relecture) : que le déclencheur soit dans le LCD,
Slint ou la logique (la charge mémoire de l'affichage n'a pas encore été
reproduite fidèlement) ; que les piles radio aient de la marge **dans
`screen`** (pile du cœur 0 six fois plus petite, interruptions en plus).
Les points de passage ne donnent que le dernier endroit instrumenté : ils
n'ont jamais désigné une cause.

Défauts d'instrumentation à corriger :
- `screen/src/stack_watermark.rs`, `usage()` du cœur 1 : saute tous les
  mots non-motif sans borne ; une pile entièrement écrasée s'afficherait
  « 0 utilisé ». Sauter une zone de taille connue (la fermeture de
  démarrage) et signaler « invalide » sans motif restant.
- `trials/10-sta-ble/vendor/esp-rtos/src/trial_stacks.rs` : `task_stacks()`
  peut lire une pile libérée entre la lecture de l'adresse et le parcours
  (sans effet tant que les tâches radio ne sont pas détruites).
- `trials/10-sta-ble`, variante `scan-load` (sans PSRAM) : 2 avertissements
  Clippy depuis la dernière modification de `src/load.rs`.

Corrigé dans `screen` : la boucle de scan BLE attend 20 ms avant son
`continue` quand `scan_can_start` est faux.

**Prochaine piste (décidée par David) : essais directement dans `screen`.**
Y porter le patch d'esp-rtos de l'essai (piles des tâches radio peintes,
mesurées dans le rapport, `TRIAL_EXTRA_TASK_STACK`), contrôles de pile tous
actifs, un ELF conservé par variante (`/tmp/screen-<variante>.elf`). Puis,
selon la mesure : grossir les piles radio, épingler la tâche `timer`
d'esp-radio au cœur 1, ou isoler LCD/Slint/logique.

### Pour reprendre

- Build de diagnostic actuel, depuis `firmware/rust/screen` :
  `ESP_RTOS_CONFIG_STACK_POINTER_RANGE_CHECK=false cargo build --release --offline`,
  plus l'un des interrupteurs `DIAG_NO_BLE`, `DIAG_NO_WIFI`,
  `DIAG_BLE_AFTER_STA`. La vérification de plage d'esp-rtos est coupée
  parce qu'elle paniquait sur l'état déjà corrompu ; la garde matérielle de
  pile reste active. Image : `espflash save-image --chip esp32s3
  --flash-size 16mb --skip-update-check target/xtensa-esp32s3-none-elf/release/screen /tmp/screen-diag.bin`,
  ELF copié en `/tmp/screen-diag.elf` pour `espflash monitor --elf`.
- Le code de diagnostic (`crumb!`, `[diag]`, interrupteurs `DIAG_*`,
  repères de construction du cœur) est à retirer une fois le problème
  réglé ; `stack_watermark`, le watchdog en deux temps et les deux copies
  vendorées sont à garder.
- Défauts relevés, non corrigés : la boucle de scan BLE fait `continue` sans
  `await` quand `scan_can_start` est faux (la tâche peut monopoliser le
  cœur 1) ; NVS est rouvert à chaque requête.
- Carte laissée sur l'essai 10 `scan-xip-load` dans `ota_0` (actif,
  `otadata` `0x6a`/`0x6b`, les deux `VALID`) ; coffeeflow dans `ota_1`.
  Images et ELF des variantes dans `/tmp/trial-10-*.{bin,elf}` et
  `/tmp/screen-diag.{bin,elf}` (perdus au redémarrage du Mac).

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

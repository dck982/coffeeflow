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
| 4 | Terminée et intégrée (`181a446`) | 117 tests + 1 doctest ; Clippy propre. Parsing serde_json sans std, sortie compatible cJSON et clés HTTP conservées. Écarts de parsing documentés dans `screen-core/README.md`. |
| 5 | Intégrée | Carte, LCD, XIP, tactile et ADS constituent la base de l'assemblage. |
| 6 | Intégrée (`833acdf`), derniers détails en intégration | 9 tests simulateur, Clippy propre, 24 captures RGB565 examinées. Pas de comparaison pixel par pixel avec LVGL. |
| 7 | Intégrée (`4b43756`, `953a164`) | CAN, pont série, staging et proxy OTA compilent pour Xtensa ; tests protocole/ACK/NOR/journaux verts. Rapport : `rust-task7.md`. |
| 8 | Intégrée (`8443b1b` et correctifs suivants) | Services GATT compilés pour Xtensa, Clippy propre ; scan, arrêt provisioning, maintien, panne de l'hôte et horodatage revus. |
| 9 | En validation dans le worktree réseau | STA/AP, HTTP/WS, SNTP, OTA locale/USB et NVS écrits ; vérifications finales avant intégration. |
| 10 | En cours | Liaison du cœur, de l'UI et des services ; compilation complète et contrôles finaux à faire. |

La logique et l'UI tournent sur le cœur 0 ; les services radio et le propriétaire
flash sur le cœur 1. La lecture de l'identité du slot précède le remappage XIP,
la radio précède le LCD, et les écritures flash gardent les interruptions actives.
Les mesures ADS ne sont publiées qu'après une nouvelle tentative de lecture ;
les événements BLE conservent leur instant de réception. Le relâchement physique
de la purge reste indépendant de la durée de vie du bouton Slint.

**Matériel : aucun flash, moniteur ou accès à la carte pendant ces travaux.**
Une fois le point 10 validé hors matériel, demander à David l'accès à l'écran
en USB sur le host ou à la machine par Wi-Fi. Reprendre alors les précautions,
les méthodes et les résultats de `rust-migration.md` avant tout essai ; le
premier parcours prévu reste l'écran sur le bureau, USB sans CAN, puis la machine.

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
- Ne jamais lire `secrets.h` ni un fichier de secrets. Jeton HTTP et mot de
  passe de l'AP : un `secrets.rs` ignoré par Git, avec `secrets.example.rs`
  versionné (mêmes noms que `secrets.example.h`).
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

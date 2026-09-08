# Firmware — séquence d'implémentation

## Où on en est

**Phase 0 (socle) et phase 1 (outil Mac) faites.**

Phase 1 :

- Cadrage série défini et figé dans `firmware/common/` (source commune, comme le reste) : PDU `id/dlc/data/crc16` + COBS sur le fil USB, WebSocket transportant le PDU nu. Voir `firmware/common/include/common/framing.hpp` pour le détail et le pourquoi (zéros d'une image OTA, resynchronisation après un octet perdu). Tests hôte C++ inclus dans `common/test/`, au vert.
- `firmware/tools/coffeetool/` : portage Python à la main de `common/` (protocole, messages, CRC, cadrage) — même pattern que `log_codes` mais sans génération commune, donc des tests croisés dédiés (`test_framing.py::test_golden_vectors_from_cpp` compare des trames encodées côté C++ octet à octet).
- Décodeur de trames en texte lisible et horodaté (`decoder.py`), utilisant la table `LOG` générée en phase 0.
- Deux transports derrière la même interface (`transport.py`) : `SerialTransport` (COBS, USB) et `WebSocketTransport` (PDU nu) — seul l'USB a un consommateur réel avant la phase 3, le reste est écrit par avance comme demandé.
- Émission à la main (`send`) : `PING`, `PONG`, `STOP`, `RESET`, `SET`, `REQSTATUS`.
- Enregistrement / relecture (`recorder.py`) en JSON Lines, format qui deviendra la fixture des tests d'algorithme en phase 6.
- Client de flash (`flash_client.py`) : `BEGIN` / blocs de 2 ko acquittés / `END`, contre le layout `FLASH_CTRL` provisoire de `messages.hpp` — à revalider pour de vrai en phase 4, contre du matériel.
- `firmware/tools/test/` : 27 tests hôte, aucune dépendance matérielle, `run_tests.sh` régénère d'abord `log_codes.py` comme en phase 0.

Pas fait / à savoir avant de continuer :

- Le client de flash n'a jamais parlé à une vraie carte : la sémantique exacte d'un nouvel essai après un bloc refusé (qui, côté récepteur, doit rejouer quoi) n'est pas fixée — `messages.hpp` le dit déjà, mais ça vaut aussi pour la logique de réception à écrire en phase 4.
- `WebSocketTransport` n'a jamais été exercé contre un vrai serveur (aucun serveur avant la phase 6) : seul l'encodage/décodage du PDU est testé.

**Phase 0 (socle), pour mémoire.**

Fait :

- Version ESP-IDF figée : v6.1 (stable courante, la LTS v5.1 visée initialement était déjà en fin de vie), notée dans `firmware/IDF_VERSION.md`. Installée via `eim`, sélectionnée.
- **`idf.py build` vérifié pour de vrai sur les deux projets** (`sensors/` et `screen/`, cible `esp32s3`) : bootloader + image applicative générés sans erreur. Un bug de `common/CMakeLists.txt` a été corrigé au passage (l'`add_custom_command` de génération des codes LOG doit venir *après* `idf_component_register`, sinon ESP-IDF le rejette lors de sa phase de lecture en mode script).
- Squelettes des deux projets ESP-IDF (`firmware/sensors/`, `firmware/screen/`), chacun avec sa table de partitions (factory + ota_0/ota_1 + rollback) et son `sdkconfig.defaults`.
- `firmware/common/` : identifiant CAN (encode/decode 11 bits), charges utiles de tous les messages du protocole (`SET`, `PONG`, `REQSTATUS`, `STATUS_*`, `LOG`, `FLASH_CTRL`), CRC16/CRC32.
- Table de codes `LOG` générée depuis une source unique (`firmware/common/codegen/log_codes.yaml` → `.hpp` pour le C++, `.py` pour l'outil Mac à venir).
- Numéro de version (`common::kFirmwareVersion{Major,Minor,Patch}`), à incrémenter à chaque image.
- Tests hôte (`firmware/common/test/run_tests.sh`) : aller-retour pack/unpack de chaque message, ordre de priorité des ID CAN, vecteurs de test CRC16/CRC32. Tournent sur le Mac, sans matériel — **vérifiés, au vert**.

Pas fait / à savoir avant de continuer :

- Layout de `FLASH_CTRL` (sous-commandes `BEGIN`/`BLOCK_ACK`/`END`/`ABORT`) est une première proposition dans `common/messages.hpp` — `firmware.md` ne fige pas les octets exacts, à revalider en phase 4.
- Tailles des partitions posées large mais provisoires, comme prévu par le plan.
- Rien dans `main.cpp` des deux projets au-delà d'un `ESP_LOGI` de démarrage — normal pour la phase 0.

Prochaine étape : phase 2, les capteurs factory sur la table — première fois qu'une carte tourne du code.

**Phase 2, en cours (2026-09-08).**

Fait, vérifié sur le vrai matériel :

- Module CAN Pal AliExpress abandonné côté capteurs (sous-tension à 3,3 V, voir
  `docs/canpal-findings.md`) — remplacé par un **M5Stack Unit CAN** (CA-IS3050G isolé,
  même module que celui déjà utilisé côté `can-monitor`). Brochage GPIO7=RX/GPIO8=TX
  sur le XIAO, **inversé** par rapport à l'ancien CAN Pal (voir suite de
  `canpal-findings.md` : `CAN_TX`/`CAN_RX` du module sont nommés du point de vue du
  transceiver, pas une convention câble croisé). Validé par self-test TWAI en boucle
  isolée (`firmware/can-selftest`) puis sur le vrai bus.
- `firmware/sensors` tourne pour de vrai sur le XIAO : TWAI 500 kbit/s, `PING`/`PONG`
  avec version, `LOG` périodique des compteurs d'erreur TWAI. Terminaison 120 Ω
  confirmée aux deux bouts.
- Round-trip `PING`/`PONG` confirmé : `PONG` reçu avec `node=kSensors`,
  `version=0.1.0`, `uptime_s` croissant. Compteurs d'erreur TWAI observés à zéro en
  régime établi (une remontée transitoire liée aux flashs/resets répétés de la session
  de bring-up s'est résorbée).
- **`firmware/can-monitor` (Atom S3) devenu un vrai pont bidirectionnel série↔CAN**,
  même cadrage COBS+PDU+CRC16 que celui prévu pour l'écran en phase 3 (voir
  `firmware/common/include/common/framing.hpp`). `coffeetool` (phase 1) lui parle
  directement sur son port USB-C pour émettre `PING`/`SET`/`STOP`/`RESET`/`REQSTATUS`
  vers `sensors` sur le vrai bus — plus besoin d'attendre l'écran pour tester le
  protocole. Le `PING` périodique auto-émis par `can-monitor` (ajouté plus tôt dans le
  bring-up) a été retiré : redondant maintenant que `coffeetool` peut l'envoyer
  lui-même avec la bonne identité.
  - **Piège de mise en œuvre** : le port USB-C de l'Atom S3 est le périphérique
    USB_SERIAL_JTAG natif du chip, le même que celui qu'ESP-IDF utilise par défaut
    pour la console (`printf`/`ESP_LOGx`). Il a fallu désactiver la console
    (`CONFIG_ESP_CONSOLE_NONE` + `CONFIG_ESP_CONSOLE_SECONDARY_NONE` dans
    `sdkconfig.defaults`) et piloter le driver `usb_serial_jtag` directement
    (`usb_serial_jtag_read_bytes`/`write_bytes`, pas le VFS console qui traduit les
    fins de ligne — une donnée CAN quelconque peut contenir 0x0A/0x0D). Conséquence :
    `can-monitor` n'a plus aucune sortie texte de debug, tout passe par les trames
    `LOG` du protocole, décodées côté `coffeetool`.
  - **À retirer ou désactiver une fois la phase 3 en place** : un vrai écran sur le
    bus serait un second nœud `kScreen`, en conflit d'identité avec ce qui transite
    par le pont.
- **`LOG` au boot confirmé sur le bus** : reset à froid du XIAO (via `esptool`),
  `BOOT` puis `READY` vus par `can-monitor`/`coffeetool` dans la foulée.
- **`RESET` testé pour de vrai** via `coffeetool send reset` : `LOG REBOOT_REQUESTED`,
  puis `BOOT`/`READY`, puis nouveau `PONG` avec `uptime_s` reparti de zéro.
- **Bail (lease) vérifié** : `SET ssr=1 ttl_ms=2000` → `STATUS_ACTUATORS` immédiat
  (`ssr=on`, `bail_restant≈2000ms`) → ~2 s plus tard `LOG LEASE_EXPIRED` +
  `STATUS_ACTUATORS` (`ssr=off`), au bon délai.
- **Verrou 60 s déclenché pour de vrai** : `SET ssr=1 ttl_ms=65000` maintenu, `LOG
  RUNTIME_LOCKOUT_TRIGGERED` (`arg32=60100` ms) après ~60 s, `STATUS_ACTUATORS`
  `flags` avec le bit verrou posé.
- **Piège documenté confirmé** : verrou actif → `RESET` logiciel (`coffeetool send
  reset`) → nouveau boot avec `uptime_s` reparti de zéro → `SET` suivant refusé
  (`LOG COMMAND_REFUSED_LOCKED`, `STATUS_ACTUATORS` avec le bit verrou toujours posé).
  Le verrou survit bien à un reset logiciel.

- **Levée du verrou à la coupure secteur réelle, confirmée** (2026-09-08) : XIAO
  débranché/rebranché en USB (coupure d'alimentation réelle, pas juste un `RESET`
  logiciel), puis `SET ssr=1 ttl_ms=2000` accepté (`flags=0b010`, bit verrou à 0) et
  bail expiré au bon délai. Le verrou ne se lève bien qu'au démarrage à froid, comme
  documenté.
- **Présence, confirmée** (2026-09-08) : `SET ssr=1 ttl_ms=30000`, câble CAN débranché
  ~6 s (> `kPresenceTimeoutUs` = 3 s) puis rebranché, `STOP` envoyé pour lire l'état :
  `ssr=off`, `bail_restant=0ms` malgré le bail de 30 s encore valide — confirme que
  `force_actuators_off()` a bien coupé pendant la coupure du bus, indépendamment du
  bail restant.

**Phase 2 terminée.** Tous les points de la checklist et le piège documenté sont
vérifiés sur le vrai matériel. Le **rearmement** (deux activations de 30 s séparées de
plus de 2 s ne déclenchent rien) n'a pas été exercé spécifiquement, mais c'est un
test de la phase 5 (pas de la checklist phase 2), à faire là où il est prévu.

**Décision (2026-09-08) : réordonnancement délibéré, phase 4 (partie capteurs) avant
phase 3.** Le blocage pour tester le flash des capteurs par CAN n'est pas matériel
(le pont série↔CAN existe déjà, `firmware/can-monitor`) mais logiciel : `sensors`
n'a aucune logique de réception `FLASH_CTRL`/`FLASH_DATA`. Plutôt que d'attendre
l'écran (phase 3) pour un pont dont on a déjà l'équivalent fonctionnel, on avance la
partie capteurs de la phase 4 maintenant, contre `can-monitor`. Le pont de l'écran
restera à valider séparément en phase 3 — ce test-ci ne le remplace pas, il avance
le risque le plus important (flash/rollback des capteurs) sans dépendre du bring-up
de l'écran.

### Prochaine étape concrète : phase 4, partie capteurs, contre `can-monitor`

**Rien n'est encore écrit pour ça.** À faire dans `firmware/sensors/main/main.cpp` :

1. Layout des messages déjà figé dans `firmware/common/include/common/messages.hpp`
   (`FlashCtrlPayload`, `FlashSubCmd::{kBegin,kBlockAck,kEnd,kAbort}`) — c'est une
   première proposition jamais confrontée au matériel, donc c'est aussi le moment de
   la challenger si elle ne tient pas.
2. Séquence exacte déjà implémentée côté Mac dans
   `firmware/tools/coffeetool/flash_client.py` (`flash()`) — c'est la référence pour
   ce qu'attend le récepteur :
   - `FLASH_CTRL BEGIN` (taille de l'image) → le récepteur doit **effacer la
     partition OTA inactive avant d'acquitter**, puis répondre
     `FLASH_CTRL BLOCK_ACK block_number=0`.
   - Blocs de 2 ko = 256 trames `FLASH_DATA` de 8 octets bruts (pas d'en-tête, la
     trame CAN entière est la donnée) → écrire au fil de l'eau avec `esp_ota_write`
     (ne pas bufferiser l'image entière en RAM). Après 256 trames, répondre
     `FLASH_CTRL BLOCK_ACK` avec `block_number+1` et le CRC16 du bloc reçu — c'est
     le contrôle de flux, `flash_client.py` rejoue le bloc si le CRC ne correspond
     pas (3 essais max).
   - `FLASH_CTRL END` (CRC32 global) → vérifier contre les octets reçus, puis
     `esp_ota_set_boot_partition` + reboot. `LOG FLASH_DONE`/`FLASH_FAILED` déjà
     dans `log_codes.yaml`, à utiliser.
3. Après le reboot sur la nouvelle image : rester en `PENDING_VERIFY`
   (`CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y` déjà activé dans
   `sdkconfig.defaults`, table de partitions `factory`/`ota_0`/`ota_1`/`otadata`
   déjà posée dans `partitions.csv`) jusqu'à reconfirmation d'un `PING`/`PONG` sur
   le bus, puis appeler `esp_ota_mark_app_valid_cancel_rollback()`. **Écrire aussi
   le temporisateur d'invalidation** (`firmware.md` insiste : IDF ne redémarre pas
   tout seul une image jamais validée, ce timer doit exister explicitement) — sinon
   une image qui ne pingue jamais reste bloquée en attente pour toujours au lieu de
   rollback.
4. Les quatre essais de validation de la phase 4 (voir plus bas, section "Phase 4"),
   **à faire contre `can-monitor` plutôt que l'écran** :
   - Flasher une image saine (incrémenter `kFirmwareVersion*` pour le voir dans le
     `PONG` après coup) via `coffeetool flash` — commande CLI déjà branchée sur
     `flash_client.py` (`cli.py::cmd_flash`), jamais exercée contre du matériel.
   - Flasher une image sciemment cassée (crash au boot avant le premier ping/pong) →
     le rollback doit ramener l'image précédente sans intervention.
   - Débrancher le câble CAN en plein transfert → l'écriture doit s'annuler
     proprement, `otadata` ne doit pas bouger, redémarrage sur l'image précédente.
   - (Le 3ᵉ essai de la checklist officielle, "idem sur l'écran, avec son propre OTA
     local", reste hors scope ici — c'est `firmware/screen`, pas encore construit.)
5. Outillage pour identifier les ports au prochain démarrage de session : voir
   section "Environnement de build/flash (Mac)" juste en dessous. `can-monitor` est
   déjà flashé sur l'Atom S3 en pont série↔CAN fonctionnel (voir plus haut) ; `sensors`
   est déjà flashé sur le XIAO avec le brochage GPIO7=RX/GPIO8=TX correct. Le câblage
   physique (Unit CAN des deux côtés, terminaison 120 Ω aux deux bouts, câble CAN
   entre les deux cartes) est déjà en place et vérifié — pas besoin de repartir de
   zéro sur le bring-up matériel, seulement d'écrire et tester la logique de flash.

Une fois ce chantier bouclé : **phase 3** (l'écran factory Waveshare comme pont
USB↔CAN) reste la suite logique — à ce stade, `firmware/can-monitor` pourra être mis
de côté (son rôle de pont ad hoc est repris par le vrai pont de l'écran, voir la note
dans sa description ci-dessous), et il faudra rejouer les mêmes essais de flash à
travers ce pont-là pour de vrai avant de considérer la barrière C atteinte.

### Environnement de build/flash (Mac)

- **Activer l'environnement ESP-IDF** : ne pas utiliser `$IDF_PATH/export.sh` — installé via `eim` (voir `firmware/IDF_VERSION.md`), il cherche un venv Python à un chemin (`~/.espressif/python_env/...`) qu'`eim` ne peuple pas. Le bon script est celui qu'`eim` dépose lui-même :
  ```sh
  source ~/.espressif/tools/activate_idf_v6.1.sh
  ```
  Doit être **sourcé** (pas exécuté) dans le shell courant ; ne persiste pas d'un appel `Bash` à l'autre dans cet outil — à re-sourcer avant chaque `idf.py build/flash/monitor` si chaque commande part dans un nouveau process.
- **Build** : `idf.py build` depuis `firmware/sensors/`, `firmware/can-monitor/`, `firmware/can-selftest/`, chacun indépendamment (pas de build à la racine `firmware/`).
- **Identifier quel port série correspond à quelle carte** (macOS expose les deux comme `/dev/cu.usbmodemNNNN`, sans nom lisible) :
  ```sh
  python -m esptool --port /dev/cu.usbmodemXXXX chip-id
  ```
  Le `Chip type` renvoyé distingue les deux cartes de ce banc :
  - **XIAO ESP32-S3** (`sensors`) : `ESP32-S3 (QFN56)` — PSRAM embarquée, **flash externe** (pas de ligne "Embedded Flash").
  - **Atom S3** (`can-monitor`) : `ESP32-S3-PICO-1 (LGA56)` — flash **et** PSRAM embarquées (SiP).
  Les numéros `/dev/cu.usbmodemNNNN` eux-mêmes ne sont pas stables : à revérifier par ce moyen à chaque nouvelle session/rebranchement, ne pas supposer qu'un port garde son rôle.
- **Flasher** : `idf.py -p /dev/cu.usbmodemXXXX flash` depuis le dossier du projet concerné.
- **Lire la sortie série sans terminal interactif** (utile pour capturer une trace courte sans bloquer sur `idf.py monitor`) : script Python avec `pyserial` (déjà présent dans le venv IDF, dépendance d'`esptool`), `serial.Serial(port, 115200, timeout=...)` + `readline()` en boucle avec une échéance. Piège : les logs `ESP_LOGx` sortent bien sur ce port, mais les messages `LOG` du protocole (boot, ready, compteurs TWAI) partent sur le **bus CAN**, pas sur l'UART — invisibles ici tant qu'on n'a pas de pont vers `coffeetool`.

---

Conception, protocole et décisions : `firmware.md`. Ce fichier ne dit pas *comment* coder, il dit **dans quel ordre**, et ce qu'il ne faut pas oublier avant de passer à la suite.

L'objectif de bout en bout est un point précis : **débrancher l'USB**. Tout ce qui vient avant existe pour rendre ce moment sans risque ; tout ce qui vient après arrive par OTA.

## Le principe qui ordonne tout

On ne met **rien** en boîte, et on ne branche **rien** sur le 230 V, tant que le chemin qui permet de se rattraper n'est pas prouvé. Les phases sont donc construites à l'envers de l'envie : le protocole et le flash d'abord, les capteurs et l'UI ensuite.

Trois barrières, dans cet ordre :

| Barrière | Ce qu'elle autorise |
| --- | --- |
| **A** — les deux cartes se pinguent sur le bus | brancher les périphériques basse tension |
| **B** — bail, présence et verrou 60 s vérifiés | brancher le 230 V sur la pompe et la vanne |
| **C** — OTA prouvé dans les deux sens, rollback compris | fermer les boîtiers et débrancher l'USB |

---

## Phase 0 — Socle

Rien de visible, mais c'est ce qui évite de tout refaire en phase 4.

- Deux projets ESP-IDF, `sensors/` et `screen/`, et un composant `common/` partagé par les deux.
- Figer la version d'ESP-IDF et la noter dans le dépôt. Une montée de version en cours de route se paie sur la carte qu'on ne peut plus atteindre.
- Dans `common/` : définition des types de messages, des charges utiles, du CRC — **et la table de codes `LOG` générée depuis une source unique**, consommée par le C++ et par l'outil Python. C'est le point à ne pas rater : une table dupliquée dérive, et le premier message qu'on ne comprendra plus sera celui d'un crash.
- Un numéro de version par image, remonté dans le `PONG`. Discipline d'incrémentation dès maintenant : sur une carte en boîte, la seule façon de savoir ce qui tourne, c'est de le lui demander.
- Tests hôte sur `common/` : encoder / décoder chaque message, aller-retour. Ça tourne sur le Mac, sans matériel.

**Sortie :** `common/` compile pour les deux cibles et passe ses tests sur le Mac.

---

## Phase 1 — L'outil Mac

Avant les cartes, parce que tout le reste se débogue à travers lui.

- Décodeur de trames : lit le flux, imprime du texte lisible, horodate.
- Deux transports, même décodeur : **USB série** (phase 2 et 3) et **WebSocket** (phase 6). Écrire l'abstraction tout de suite, même si seul l'USB existe.
- Émission aussi, pas seulement lecture : pouvoir envoyer un `SET`, un `PING`, un `REQSTATUS` à la main est ce qui rend les phases 2 à 5 tenables.
- Enregistrement dans un fichier, et relecture. **Ce format devient la fixture des tests d'algorithme** en phase 6 : on enregistre un vrai shot, on le rejoue sur le Mac.
- Le client de flash (`BEGIN`, blocs, `END`) vit ici aussi.

**Sortie :** l'outil décode et rejoue un fichier de trames fabriqué à la main, sans matériel.

---

## Phase 2 — Capteurs factory, sur la table

Le XIAO seul, alimenté en USB, hors de la machine, sans aucun périphérique branché sauf le CAN Pal.

- TWAI à 500 kbit/s, réception en accept-all, aiguillage sur le type.
- `PING` / `PONG` avec identité et version.
- `LOG` au boot.
- `RESET`.
- **Exposer les compteurs d'erreur TWAI** dans un `LOG` périodique. C'est comme ça qu'on valide le câblage et la terminaison sans oscilloscope, et ça servira à chaque phase suivante.
- GPIO 9 tenu bas dès le démarrage, avant l'initialisation du CAN.
- Machine à états de sécurité **complète**, même sans actionneur branché : bail, présence, verrou 60 s en mémoire RTC. On la teste ici, à vide, où elle ne peut rien casser.
- Rien d'autre : ni I2C, ni débitmètre, ni logique d'infusion.

**Piège :** le verrou 60 s doit survivre à un `RESET` logiciel et ne se lever que sur un démarrage à froid. À vérifier explicitement, pas à supposer.

**Sortie :** l'outil Mac, branché sur un adaptateur CAN ou sur la seconde carte en phase 3, voit les pongs. Le verrou se déclenche à la commande et ne se lève qu'à la coupure d'alimentation.

### `firmware/can-monitor` — l'adaptateur CAN de secours

Un troisième firmware, indépendant de `sensors/` et `screen/`, pour avoir un moyen de parler au bus sans dépendre de l'écran (utile avant la phase 3, et comme filet ensuite).

**Devenu, en cours de phase 2, un vrai pont bidirectionnel série↔CAN** — pas juste un moniteur passif comme prévu initialement (voir "Phase 2, en cours" plus haut pour le pourquoi : tester `SET`/`STOP`/`RESET` sans attendre l'écran) :

- Cible : M5Stack Atom S3 + M5Stack Unit CAN (**CA-IS3050G isolé**, pas le TJA1051/3 — voir la suite de `docs/canpal-findings.md`), relié par le Port.A.
- GPIO déclarés **en haut du fichier**, modifiables sans fouiller le reste du code — valeurs par défaut `TX = GPIO 2`, `RX = GPIO 1` (Port.A de l'Atom S3 monté sur ce banc). `GPIO 26`/`GPIO 36` documentés initialement étaient faux pour cet exemplaire : confirmé au multimètre puis par un auto-test de bouclage transceiver (`firmware/can-selftest`) le 2026-09-08. Le Port.A peut varier d'un lot à l'autre — revalider avant de réutiliser ces valeurs sur un autre Atom S3.
- TWAI à 500 kbit/s, accept-all. Chaque trame reçue est encadrée (COBS+PDU+CRC16, `firmware/common/include/common/framing.hpp`) et écrite brute sur le port USB-C ; chaque trame encadrée reçue sur ce port est décodée et transmise sur le bus. Même cadrage que celui prévu pour l'écran en phase 3 : `coffeetool` s'en sert exactement pareil.
- Console désactivée (`CONFIG_ESP_CONSOLE_NONE`/`CONFIG_ESP_CONSOLE_SECONDARY_NONE`) : le port USB-C est le périphérique `usb_serial_jtag` natif du chip, piloté directement en octets bruts (pas le VFS console, qui traduirait des fins de ligne dans des données CAN binaires). Plus aucun `printf`/`ESP_LOGx` — les erreurs qui comptent passent par les trames `LOG` du protocole, décodées côté `coffeetool`.
- Pas d'identité propre, pas de participation au protocole : c'est un pont transparent, pas un nœud. `coffeetool` choisit lui-même la source (`--src screen`) de ce qu'il envoie.

**Usage :** flasher une fois, laisser branché sur le bus, utiliser `firmware/tools/coffeetool` contre son port USB-C (`--port /dev/cu.usbmodemXXXX`) exactement comme on le ferait contre l'écran en phase 3.

**À retirer ou désactiver une fois la phase 3 en place** : un vrai écran sur le bus est un second pont vers le même rôle logique `kScreen` — les deux en même temps sèmeraient la confusion, pas un conflit protocolaire à proprement parler (le pont n'a pas d'identité propre) mais deux sources concurrentes du point de vue de `sensors`.

---

## Phase 3 — Écran factory, le pont USB ↔ CAN

Le Waveshare seul, en USB, hors de la machine.

- `CAN_SEL` (EXIO5 du CH422G) tenu haut. **C'est la première chose à faire marcher** : sans ça le transceiver n'est pas sélectionné et il ne se passe rien, sans message d'erreur. Le bring-up CH422G de `tests/screen/hello_waveshare/` sert de référence.
- TWAI, même pile que les capteurs, depuis `common/`.
- CDC USB : pont bidirectionnel entre le port série et le bus.
- `PING` / `PONG` / `LOG` / `RESET`.
- Ni Wi-Fi, ni HTTP, ni LVGL, ni BLE. L'écran peut rester noir.
- Sauvegarder les deux images factory dans le dépôt ou à côté, avec leur version. Ce sont les seules qu'on reflashera un jour en USB.

**Puis :** relier les deux cartes par la paire CAN, terminaison activée aux deux bouts.

**Sortie — barrière A.** Les deux cartes se pinguent, l'outil Mac voit le trafic à travers le pont, les compteurs d'erreur TWAI restent à zéro sur plusieurs minutes.

---

## Phase 4 — Le flash, dans les deux sens

C'est le jalon. Tant qu'il n'est pas franchi, la suite est théorique.

- `FLASH_CTRL` / `FLASH_DATA` dans `common/`, donc identique des deux côtés.
- Table de partitions posée sur les deux puces : `factory`, `ota_0`, `ota_1`, `nvs`, `otadata`. Tailles provisoires, à réajuster en phase 6 quand on connaîtra le poids réel de l'application écran — mais **réajuster une table de partitions sur une carte en boîte n'est pas possible**, donc prévoir large dès maintenant.
- Effacement complet de la partition **avant** l'acquittement du `BEGIN`.
- Blocs de 2 ko acquittés avec CRC16, CRC32 global sur le `END`.
- Progression en `LOG`.
- Validation de l'image après démarrage, conditionnée au ping/pong CAN.
- **Temporisateur d'invalidation propre** : IDF ne redémarre pas tout seul une image en attente de validation. Écrire ce mécanisme, ne pas compter sur un rollback automatique.

Les quatre essais qui valident la phase, tous à faire pour de vrai :

1. Flasher une image saine dans les capteurs depuis le Mac, à travers l'écran. Elle démarre, elle pongue avec sa nouvelle version.
2. Flasher une image sciemment cassée dans les capteurs. **Le rollback ramène la précédente sans intervention.**
3. Idem sur l'écran, avec son propre OTA local.
4. Débrancher la paire CAN en plein transfert. L'écriture s'annule, `otadata` ne bouge pas, la carte redémarre sur l'image précédente.

**Sortie — barrière C (partielle).** Une carte se reflashe et se récupère sans USB. À ce stade seulement, la mise en boîte devient une option raisonnable.

---

## Phase 5 — Application capteurs, livrée par OTA

À partir d'ici, on ne flashe plus le XIAO en USB. Chaque itération passe par le bus : c'est la répétition générale de la vie en boîte, tant qu'on peut encore ouvrir.

Dans l'ordre, un périphérique à la fois, en vérifiant à chaque fois sur l'outil Mac :

1. **XDB401** — bus I2C, mutex, conversion déclenchée puis relâchement du mutex pendant les 50 ms d'attente. `STATUS_PRESSURE` avec les 5 octets bruts. `REQSTATUS` avec période.
2. **Débitmètre** — ISR sur front descendant, compteur 32 bits, horodatage du dernier front, `STATUS_FLOW`. Vérification à la main : souffler dans le capteur ou le faire tourner, compter les impulsions.
3. **Dimmer** — écriture du registre de niveau, lecture du statut et de l'erreur, remontés dans les flags. **Le dimmer exige le secteur pour sortir de `Calibrating...`** : c'est la première fois que 230 V et USB coexistent sur le plan de travail. **Laptop sur batterie, débranché du secteur** — sinon la masse du Mac rejoint celle de l'alim RECOM.
4. **SSR** — sortie GPIO, puis `SET` complet avec bail et `STATUS_ACTUATORS` en retour.

Puis la vérification de sécurité, **avant** de relier la pompe et la vanne :

- Bail : couper l'émission de `SET`, les actionneurs retombent en ~500 ms.
- Présence : débrancher la paire CAN, tout retombe, le streaming s'arrête.
- Verrou : commander 60 s d'affilée, vérifier la coupure, le code `LOG`, le refus des commandes suivantes, et que seul un cycle d'alimentation le lève.
- Rearmement : deux activations de 30 s séparées de plus de 2 s ne déclenchent rien.

**Sortie — barrière B.** Les actionneurs 230 V peuvent être reliés. Le module capteurs est complet et se met à jour par le bus.

---

## Phase 6 — Application écran, livrée par OTA

Le plus gros morceau, mais le moins risqué : l'écran reste atteignable en USB.

1. **Réseau** — provisioning Wi-Fi (point d'accès + page d'accueil suffit ; la saisie tactile peut attendre LVGL), identifiants en NVS, secret HTTP dans un en-tête non commité. Prévoir **un moyen d'effacer la NVS depuis l'image factory** : un SSID erroné enregistré rend l'écran injoignable en Wi-Fi, et c'est l'USB qui doit pouvoir rattraper ça.
2. **HTTP** — `GET` télémétrie, `POST` commandes, `POST` firmware avec cible. C'est le moment où le flash passe du câble série au réseau.
3. **WebSocket** — miroir du trafic CAN, **même format qu'en USB**. L'outil Mac ne change pas, il change de transport.
4. **Couche capteurs** — l'interface unique `{ horodatage, valeur brute, validité }` et les calibrations en NVS par-dessus. Les sources CAN d'abord.
5. **BLE** — client GATT vers l'Acaia Lunar, comme une source de plus.
6. **LVGL** — écran, tactile, et l'UI minimale : purge, départ d'infusion, arrêt.

**Sortie :** l'écran se flashe et flashe les capteurs par le réseau, et l'outil Mac voit tout par WebSocket.

---

## Phase 7 — Mise en boîte et débranchement

- Vérifier une dernière fois les deux images factory sauvegardées, avec leur version.
- Refaire les quatre essais de la phase 4, **par le réseau cette fois**, cartes hors boîte mais câblées comme en service.
- Monter le XIAO dans `boitier_dc`, l'écran dans `screen_wedge` / `screen_base`.
- Vérifier l'accès USB-C de l'écran une fois la façade montée, panneau latéral retiré. C'est le filet, il doit être praticable.
- Un flash complet des deux cartes, en boîte, par le réseau.
- **Débrancher l'USB.**

---

## Après

Tout ce qui suit arrive par OTA, dans la machine.

1. **Calibration** (`firmware.md`, section dédiée) — facteur K sous OPV, puis le sort de l'OPV au-dessus de son seuil, puis le point de décrochage, puis la carte dimmer → pression. C'est ce qui débloque les features suivantes, et c'est la première chose à faire une fois l'USB débranché.
2. **Purge / flush**, la plus simple, et celle qui exerce le chemin complet écran → CAN → actionneurs.
3. **Infusion au temps**, puis **au poids**.
4. **Pré-infusion**, une fois qu'on sait si le débitmètre sert à quelque chose en dessous de 1 ml/s.
5. **Flow control**, en dernier. L'OPV renvoyant à l'entrée de la pompe et restant fermée à 9 bar, le débitmètre lit bien le débit d'infusion ; la contrainte est sa résolution (~4 s de moyennage à 1 ml/s), donc pression et poids restent les signaux rapides.

---

## Ce qu'on oublie habituellement

- **La table de codes `LOG` en double.** Générée depuis une source unique, sinon elle dérive.
- **La table de partitions figée trop tard.** On ne la change plus une fois la carte inaccessible.
- **Le rollback jamais testé pour de vrai.** Une image sciemment cassée, ou ça ne compte pas.
- **Le verrou 60 s effacé par un reset logiciel.** Il doit être en mémoire RTC, levé seulement au démarrage à froid.
- **Les masses jointes.** Laptop sur batterie dès qu'un USB et le 230 V se croisent.
- **Le dimmer muet sans secteur.** Il reste en `Calibrating...` et refuse tout : ce n'est pas un bug du firmware.
- **Les pull-ups I2C qui vivent dans le XDB401.** Débrancher la sonde de pression rend le dimmer muet, en phase 5 comme en service.
- **Le mutex I2C tenu pendant les 50 ms de conversion.** Il ne doit pas l'être.
- **Les compteurs d'erreur TWAI jamais regardés.** C'est le seul diagnostic de câblage disponible sans oscilloscope.
- **La version qui ne bouge pas.** Sur une carte en boîte, le `PONG` est la seule façon de savoir ce qui tourne.
- **Les images factory non archivées.** Ce sont les seules qu'on reflashera en USB, il faut les retrouver.

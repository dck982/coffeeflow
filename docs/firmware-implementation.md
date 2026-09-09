# Firmware — séquence d'implémentation

## Où on en est

**Phase 5, débitmètre (Digmesa, GPIO 44/D7), en cours — bloqué à 0 impulsion,
pas encore résolu (2026-09-09).** `firmware/sensors/main/main.cpp` :
`init_flow()` configure GPIO44 en entrée, interruption front descendant,
pull-up interne éteinte (le filtre RC du shield en tient lieu). ISR
(`flow_isr_handler`) incrémente `g_flow_pulse_count` (32 bits) et mémorise
`g_flow_last_edge_us`. `on_reqstatus_received` gère `kStatusFlow` (pas de
plancher de période, contrairement au XDB401 — rien ne l'impose côté GPIO) ;
`tick_flow()` (dans `safety_task`, pas de tâche dédiée : lecture non
bloquante) publie `STATUS_FLOW` à la période demandée.

**Bug trouvé et corrigé en cours de route, mais qui ne suffit pas** : GPIO44
est le RX par défaut de la console UART0 (`CONFIG_ESP_CONSOLE_UART_DEFAULT`,
`sensors/sdkconfig`) — sans forcer le pad en `PIN_FUNC_GPIO`, il reste sur sa
fonction IOMUX de reset (`U0RXD`) et ne remonte jamais rien à l'ISR. **Exactement
le même piège que le bug 1 de la phase 3** (voir plus bas, `screen`), sur le
même GPIO, corrigé de la même façon (`gpio_func_sel(kGpioFlow,
PIN_FUNC_GPIO)` + `gpio_input_enable()`, `esp_private/gpio.h`). Après ce
correctif, un `ESP_LOGI` de debug direct (`gpio_get_level(kGpioFlow)`, lu en
USB natif sur `sensors`, pas via CAN) confirme que le pad est bien lisible en
GPIO simple (`level=1` au repos, cohérent avec un NPN open collector au
repos tiré haut) — **mais aucune impulsion n'est comptée malgré plusieurs
souffles francs dans le capteur, turbine visiblement en rotation.**
`g_flow_pulse_count` reste à 0, `level` reste figé à `1` tout du long
(aucune transition observée, même passagère).

Piste non encore vérifiée, à creuser en priorité à la reprise : tension
mesurée au multimètre sur le connecteur du débitmètre, **GND↔VCC = 5,14 V,
GND↔SIGNAL = 2,64 V au repos**. Le repos attendu d'après `docs/firmware.md`
("filtre RC du shield, 1 kΩ vers 3,3 V, 10 nF vers GND") serait proche de
3,3 V, pas 2,64 V — l'écart (~0,66 V) est trop grand pour n'être qu'un effet
de charge du multimètre. Deux hypothèses à trancher avant de reprendre le
firmware : (a) le port réellement câblé n'est pas R2/D7 tel que documenté
(mauvais port, mauvais GPIO), (b) le filtre RC du shield ne se comporte pas
comme documenté sur cet exemplaire (résistance/tension de pull-up
différentes, ou pull-up vers 5 V divisée plutôt que vers 3,3 V) — dans les
deux cas, ça pourrait expliquer une absence totale de transition si le
niveau ne bouge jamais assez bas/haut, ou si le vrai signal n'arrive
simplement pas sur GPIO44.

Debug temporaire laissé en place dans `tick_flow()` (`main.cpp`, commentaire
"DEBUG temporaire — bring-up débitmètre, à retirer une fois validé") : log
`ESP_LOGI` toutes les 500 ms avec `level`/`pulses`, à lire en USB direct sur
le port `sensors` (pas via CAN — voir la remarque déjà documentée plus bas,
"les messages `LOG` du protocole partent sur le bus CAN, pas sur l'UART").
**À retirer une fois le débitmètre validé**, ne pas oublier.

Bit `flags` "capteur valide" généralisé la même session (voir
`docs/firmware.md`, nouveau paragraphe "Convention `flags`") : bit0 réel sur
`STATUS_PRESSURE` (détection I2C), fixé à 1 sur `STATUS_FLOW` (non
détectable en GPIO seul). **Validé sur le vrai matériel** : XDB401 branché →
`flags=0b01` sur toutes les trames `STATUS_PRESSURE` ; débranché →
`flags=0b00` + `LOG I2C_ERROR arg16=127` en boucle, comme attendu.

**Phase 5, XDB401 (pression/température), fait et validé sur le vrai
capteur (2026-09-09).** `firmware/sensors/main/main.cpp` : bus I2C
(`esp_driver_i2c`, GPIO5 SDA / GPIO6 SCL, port R1), déclenchement de
conversion (écriture `0x0A` au registre `0x30`), attente de fin de
conversion par scrutation du bit Sco (bit 3 du registre 0x30, avec repli sur
un délai fixe de 50 ms si le bit ne retombe jamais), puis lecture en deux
transactions I2C séparées (3 octets à partir de `0x06` pour la pression, 2
octets à partir de `0x09` pour la température — pas une rafale unique des 5
octets, voir plus bas). `on_reqstatus_received` gère désormais
`kStatusPressure` (période avec plancher 100 ms, 0 = arrêt) ; une tâche
dédiée `pressure_task` (pas un tick de plus dans `safety_task`, pour ne pas
imposer ~50 ms de blocage à la boucle bail/présence/verrou) déclenche la
lecture et publie `STATUS_PRESSURE`.

Deux écueils rencontrés, tous les deux sur le vrai capteur, à travers le
pont écran (écran rebranché pour l'occasion — pas nécessaire pour flasher
`sensors`, qui reste accessible en USB direct, mais indispensable pour
échanger des messages protocolaires sur le bus) :

- **La lecture en une seule rafale de 5 octets (`0x06`→`0x0A`, repeated
  start) donnait une pression aberrante d'une lecture à l'autre alors que
  la température restait cohérente.** L'exemple du fabricant (datasheet
  XDB401) fait deux appels séparés (`I2C_ReadNByte(0x06, Pressure, 3)` puis
  `I2C_ReadNByte(0x09, Temp, 2)`), chacun avec son propre STOP — pas une
  transaction combinée. Reproduit tel quel, corrige le problème.
- **Faux positif de debug, à ne pas reproduire** : en lisant les valeurs
  décimales imprimées par `coffeetool monitor` à l'œil pendant le
  diagnostic, la pression semblait varier de façon aberrante (jusqu'à
  ±9 bar d'une lecture à l'autre, capteur au repos). En réalité `sensors`
  et `coffeetool` sont cohérents entre eux (même convention little-endian
  de bout en bout pour `pressure_raw`/`temperature_raw`, voir
  `StatusPressurePayload` dans `common/messages.hpp` et son miroir Python
  dans `coffeetool/messages.py`) — mais cette convention **diffère de
  l'ordre big-endian utilisé par la formule de la datasheet**
  (`m = x·65536 + y·256 + z`). Le nombre décimal affiché par `coffeetool`
  n'est donc pas directement comparable à la formule du fabricant sans
  reconvertir les octets. Une fois reconverti correctement, les lectures
  étaient stables (~0,013 bar au repos) depuis le début — la « panne »
  n'existait que dans la lecture manuelle des chiffres, pas dans le
  matériel ni le firmware. Documenté ici pour ne pas se refaire peur la
  prochaine fois : le brut part bien « tel quel » sur le CAN comme prévu
  (voir `docs/firmware.md`, "XDB401"), mais quiconque doit un jour
  interpréter ces octets en bar/°C doit appliquer la formule de la
  datasheet à l'ordre **big-endian d'origine des registres**, pas à
  l'entier `pressure_raw`/`temperature_raw` tel qu'assemblé par le
  firmware/`coffeetool`.

**Validé en soufflant physiquement dans le capteur** : capture de
`STATUS_PRESSURE` à 200 ms de période sur 10 s pendant deux souffles —
deux pics nets et cohérents dans le temps (~0,013 bar de repos → ~0,08 bar
au pic → retour), confirmant la chaîne complète (I2C → CAN → pont écran →
`coffeetool`) plutôt qu'une simple valeur statique plausible.

Pas fait / prochaine étape : le **débitmètre** (Digmesa, ISR front
descendant GPIO 44, voir `docs/firmware.md` section dédiée), puis dimmer et
SSR (ces deux derniers demandent le secteur, voir la note "Ce qui nécessite
le 230 V" plus bas dans ce fichier).

**Phase 4, partie capteurs, terminée (2026-09-08).** Réception `FLASH_CTRL`/
`FLASH_DATA` écrite dans `firmware/sensors/main/main.cpp` (écriture au fil de
l'eau via `esp_ota_write`, CRC16 par bloc, CRC32 global au `END`,
`PENDING_VERIFY` + timer d'invalidation 30 s réutilisant le mécanisme de
présence existant). Les trois essais applicables (sans écran) validés pour de
vrai contre `can-monitor` :

- **Image saine** : `v0.1.1` flashée par CAN, redémarre, `PONG v0.1.1`, `LOG
  OTA_VALIDATED` après confirmation `PING`/`PONG`.
- **Image cassée** (`v0.1.2`, `abort()` avant tout `PING`) : rollback
  automatique du bootloader (`CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE`), `PONG`
  revient à `v0.1.1` sans intervention.
- **Coupure CAN en plein transfert** (câble débranché au bloc 3/107) : le
  client abandonne après 3 échecs, aucun `END` n'est jamais reçu côté
  capteurs donc `esp_ota_set_boot_partition` n'est jamais appelé — `otadata`
  ne bouge pas, la carte ne reboote même pas, elle continue sur `v0.1.1` sans
  interruption. `LOG TWAI_ERROR_COUNTERS` confirme le pic d'erreurs pendant
  la coupure.

Deux bugs trouvés et corrigés côté client (`firmware/tools/coffeetool/flash_client.py`),
pas dans le firmware — le layout `FlashCtrlPayload`/`FlashSubCmd` de
`messages.hpp` n'a pas eu besoin de changer :

- `_wait_flash_ctrl` ne lisait qu'une seule trame par tentative et
  abandonnait si ce n'était pas un `FLASH_CTRL` — alors qu'un `LOG` précède
  systématiquement chaque acquittement sur le bus réel (`LOG FLASH_BEGIN`
  avant le `BLOCK_ACK` de `BEGIN`, `LOG FLASH_PROGRESS` avant chaque
  `BLOCK_ACK` de bloc). Corrigé : boucle jusqu'à trouver un `FLASH_CTRL` ou
  expiration du délai total.
- Les 256 trames `FLASH_DATA` d'un bloc partaient sans aucune pause entre
  elles, ce qui sature le pont `can-monitor` (sa file TWAI déborde, des
  trames sont perdues) — le premier bloc échouait systématiquement au CRC16.
  Corrigé par un espacement de 2 ms entre trames.

**Reste ouvert, documenté en commentaire dans `main.cpp`** (pas corrigé, hors
scope de cette session) : si un bloc échoue au CRC16 côté capteurs et que
`flash_client.py` le rejoue, le récepteur ne peut pas distinguer ce rejeu
d'un bloc suivant — le curseur d'écriture a déjà avancé, donc un vrai rejeu
casserait l'image (rattrapé par le CRC32 global du `END`, donc pas de risque
de flasher une image fausse, mais sans le vrai rattrapage bloc par bloc
annoncé par le protocole). Non observé sur le vrai bus : le CAN a son propre
CRC/ACK matériel, ce cas ne devrait se déclencher que sur un bug logiciel.

**Phase 3 : barrière A atteinte (2026-09-09).** Écran (Waveshare) et
capteurs (XIAO) branchés en USB, reliés par un vrai câble CAN,
`can-monitor` débranché.

- **Le câble USB de l'écran est branché sur le port "UART"** de la carte
  (bridge WCH CH343P externe, confirmé par le schéma officiel Waveshare —
  nets `ESP_TXD`/`ESP_RXD` vers GPIO43/44), **pas le port USB natif** de
  l'ESP32-S3 — d'où l'usage d'`UART_NUM_2` réaffecté sur GPIO43/44 via la
  matrice GPIO, plutôt que `usb_serial_jtag` (comme `can-monitor`) ou
  `UART_NUM_0` directement.
- **GPIO CAN corrigés : TX=GPIO20, RX=GPIO19**, pas 15/16 comme une première
  lecture de la fiche produit Waveshare l'avait suggéré — confirmé contre le
  code source officiel (`waveshareteam/ESP32-S3-Touch-LCD-4.3`, cloné dans
  `tmp/ESP32-S3-Touch-LCD-4.3/`, voir `examples/ESP-IDF/06_TWAItransmit` et
  `07_TWAIreceive`, `sdkconfig.defaults`). GPIO19/20 sont les broches USB
  natives D+/D- de l'ESP32-S3, basculées vers CAN_TX/CAN_RX par le mux
  analogique FSUSB42UMX quand `CAN_SEL` (EXIO5 du CH422G) passe haut — d'où
  le partage de bit avec `EXIO_USB_SEL` déjà repéré dans
  `tests/screen/hello_waveshare/`.
- **`PING`/`PONG` confirmés dans les deux sens à travers le vrai pont
  écran**, y compris l'injection de commandes depuis le Mac (pas seulement
  la lecture) : un `PING` envoyé par `coffeetool` obtient un `PONG` direct
  de `sensors`, testé 3/3 après reset propre.

**Deux bugs de bring-up UART trouvés et corrigés**, tous deux silencieux
(aucune erreur, juste une réception qui ne marche jamais) :

1. **Régression IDF v5.x → v6.1** dans `uart_set_pin()` : sa branche RX
   (`gpio_hal_matrix_in()`) ne force plus le pad en `PIN_FUNC_GPIO` comme le
   faisait v5.1 (la branche TX, elle, le fait toujours) — sans forcer
   nous-mêmes le pad, GPIO44 restait sur sa fonction IOMUX de reset
   (`U0RXD`), jamais lue par `UART_NUM_2`. Corrigé par
   `gpio_func_sel(rx, PIN_FUNC_GPIO)` + `gpio_input_enable()` +
   `gpio_pullup_en()` juste après `uart_set_pin()`.
2. **`uart_read_bytes(..., portMAX_DELAY)` ne se réveille jamais** sur cet
   `UART_NUM_2` réaffecté, même une fois le bug 1 corrigé et la FIFO
   effectivement pleine (confirmé par `uart_get_buffered_data_len()`) — un
   timeout court (20 ms) en boucle, comme le fait l'exemple officiel
   Waveshare (`05_UART_Test`), fonctionne à chaque essai.

Méthode de diagnostic, pour mémoire (utile si un bug similaire réapparaît
sur `screen/` ou un autre projet ESP-IDF v6.1) :

- Le binaire précompilé officiel Waveshare (`UART_Test.bin`, compilé en IDF
  **v5.3.1** d'après son en-tête `esptool image-info`) fonctionne de façon
  fiable — confirmé sans ambiguïté par un script Python `pyserial` qui lit
  le port directement (pas seulement `picocom`, qui peut faire de l'écho
  local trompeur). Ça a permis d'écarter le câble/port physique/brochage
  comme cause, et d'isoler la régression à la version d'IDF.
- Bissection décisive : `gpio_get_level()` en direct sur GPIO44 pendant que
  le Mac spamme `0x55` en continu → des centaines de milliers de
  transitions comptées, donc le signal physique arrive bel et bien au pad.
  Puis `uart_get_buffered_data_len()` en boucle → la FIFO UART2 se remplit
  bien (jusqu'à pleine, 2040/2048 octets). Ces deux mesures ont isolé le
  bug à l'appel bloquant `uart_read_bytes(..., portMAX_DELAY)` lui-même,
  pas au routage matrice GPIO ni à la réception matérielle.
- Beaucoup de fausses pistes explorées et écartées avant ça (ordre
  d'installation des drivers I2C/TWAI/UART, délais après l'écriture
  CH422G, `gpio_reset_pin()`, taille des buffers TX/RX, RTS/CTS matériel,
  `CONFIG_ESP_CONSOLE_NONE`) — aucune n'était la cause.

**`firmware/screen/main/main.cpp` est maintenant le vrai firmware du pont**
(pont série↔CAN + nœud `kScreen`, comme prévu depuis le début de la phase
3), avec les deux correctifs ci-dessus intégrés. `CMakeLists.txt` de
`screen/main` a gagné `esp_driver_gpio` (pour `gpio_func_sel` et
consorts).

**Test d'endurance TWAI fait (2026-09-09, 3 min, 07:34:20→07:37:23) :**
`tx_error_counter`/`rx_error_counter` restés à 0 tout du long,
`bus_error_count` figé sans jamais grimper sur les 3 minutes — zéro nouvelle
erreur bus pendant le test. Barrière A fermée sur ce critère.

Note en passant (pas un défaut, juste un comportement à connaître) : un
`LOG PRESENCE_LOST` revient toutes les ~3 s pendant ce test (60 fois sur les
3 minutes, toujours suivi d'un seul aller-retour `PING`/`PONG`). Normal à ce
stade : `sensors` ne pingue que quand il détecte une perte de présence
(`firmware/sensors/main/main.cpp`, `tick_presence()`), `screen` ne pingue
jamais de lui-même, et aucun trafic `STATUS`/`REQSTATUS` périodique n'existe
encore (phases 5/6). Le cycle silence 3 s → perte → probe → reprise est donc
attendu, pas un signe de câblage instable — à revérifier une fois du trafic
périodique en place.

Piège rencontré en faisant ce test, pour mémoire (détaillé dans la
checklist de mise en route ci-dessous) : un pont qui semblait totalement
muet ce matin (aucun `PING`/`PONG`, aucun `LOG`) s'est avéré fonctionner
parfaitement — la cause était un `coffeetool monitor` redirigé vers un
fichier puis tué avant que son buffer stdout ne soit vidé, pas une panne
matérielle. Un vrai power-cycle physique de l'écran (pas un reset logiciel)
a aussi été nécessaire à un moment, écho d'un piège déjà rencontré la veille.

**Essais OTA de la phase 4 rejoués à travers le vrai pont écran, tous les
trois réussis (2026-09-09)** — chemin complet USB Mac → écran → CAN →
capteurs, `can-monitor` totalement débranché pendant ces essais :

- **Image saine** (`v0.1.4` puis `v0.1.5`) : transfert des 107 blocs sans
  échec, reboot, séquence `BOOT` → `OTA_PENDING_VERIFY` → `READY` → premier
  `PING`/`PONG` échangé avec l'écran → `LOG OTA_VALIDATED`. Capturée en
  entier cette fois (contrairement à l'essai précédent contre
  `can-monitor`, où le tout premier tick post-boot avait échappé à la
  capture).
- **Image cassée** (`v0.1.6`, `abort()` en toute première ligne
  d'`app_main`, avant tout GPIO/CAN) : rollback automatique du bootloader
  déjà effectif avant même le début de la capture (aussi rapide qu'observé
  précédemment contre `can-monitor`) — confirmé a posteriori par un `PING`
  manuel : `PONG` répond `v0.1.5`, la dernière image validée, pas `v0.1.6`.
- **Coupure CAN en plein transfert** (câble débranché ~3-4 s au bloc
  2/107) : `flash_client.py` abandonne après 3 échecs (`bloc 2 refusé après
  3 essais`), câble rebranché ensuite, `sensors` interrogé par `PING` :
  `PONG` répond toujours `v0.1.7` (l'image précédente, déjà validée) avec
  un `uptime_s` continu, jamais retombé à zéro — confirme qu'aucun reboot
  n'a eu lieu, `otadata` n'a pas bougé, exactement le comportement déjà vu
  contre `can-monitor`.

**Barrière C atteinte pour la partie capteurs.** Le point 3 de la checklist
officielle de la phase 4 (« idem sur l'écran, avec son propre OTA local »)
reste hors scope : c'est un flash de `screen` par lui-même, pas encore fait.
`can-monitor` peut être mis de côté pour la suite du travail sur `sensors`
et `screen` — son rôle de pont ad hoc est repris pour de bon par l'écran.

Pas fait / à savoir avant de continuer :

- **OTA de l'écran lui-même** jamais exercé (`screen` n'a pas encore de
  logique `FLASH_CTRL`/`FLASH_DATA` réceptrice — jusqu'ici c'est toujours
  `sensors` qui reçoit un flash, `screen` n'étant que le pont). Reste à
  écrire si on veut fermer complètement le point 3 de la phase 4.

Prochaine étape : **rejouer les essais OTA de la phase 4 (`coffeetool
flash`) sur `sensors`, cette fois à travers le pont écran réel** (USB Mac →
écran → CAN → capteurs) plutôt que `can-monitor`. Les trois essais
matériels de la phase 4 (image saine, image cassée avec rollback, coupure
CAN en plein transfert) sont à revalider dans ce nouveau chemin avant de
considérer la barrière C atteinte et de mettre `can-monitor` définitivement
de côté.

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

### Phase 4, partie capteurs, contre `can-monitor` — terminée, détail ci-dessous pour mémoire

**Fait et vérifié sur le vrai matériel — voir le résumé en tête de fichier.**
Détail conservé tel qu'écrit avant l'implémentation, comme trace de ce qui
était prévu :

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

### Checklist de mise en route (nouvelle session)

Écrite après une session (2026-09-09 matin) où la moitié du temps est partie en
tâtonnement avant de découvrir que le pont marchait très bien depuis le
début — les points ci-dessous sont les pièges rencontrés, dans l'ordre où les
vérifier.

1. **Sourcer l'environnement IDF avant toute commande `idf.py`/`esptool`/`coffeetool`**,
   et le refaire à chaque nouvel appel `Bash` si l'outil ne garde pas l'état du
   shell entre deux appels :
   ```sh
   source ~/.espressif/tools/activate_idf_v6.1.sh
   ```
2. **Identifier quel port est quelle carte, ne jamais supposer** (les
   `/dev/cu.usbmodemNNNN` changent d'un rebranchement à l'autre) :
   - `python -m esptool --port /dev/cu.usbmodemXXXX chip-id` donne le type de
     puce, mais **XIAO et Waveshare sont tous les deux des `ESP32-S3
     (QFN56)`** — ça ne les distingue pas entre eux (seul l'Atom S3 du
     `can-monitor` ressort différent, `ESP32-S3-PICO-1`).
   - Utiliser plutôt `python -m esptool --port /dev/cu.usbmodemXXXX flash-id`
     et regarder `Detected flash size` : **8 Mo = XIAO (`sensors`), 16 Mo =
     Waveshare (`screen`)**. Fiable, contrairement au chip-id seul.
3. **Avant de conclure à une carte muette, vérifier qu'aucun autre process
   n'a déjà le port ouvert** (`ps aux | grep coffeetool`) — un `monitor` lancé
   en arrière-plan et pas proprement tué (voir point 5) reste accroché au
   port et fait sembler le nouveau essai silencieux, ou pire, fait lire du
   flux entrelacé entre deux processus sur le même port.
4. **`coffeetool monitor` redirigé vers un fichier (`> out.log &`) puis tué
   avant la fin peut paraître muet alors que le trafic existe bien** : la
   sortie de Python est bufferisée par bloc (pas ligne par ligne) dès qu'elle
   n'est plus un terminal, donc un `kill` avant que le buffer se vide perd
   tout ce qui n'a pas encore été flush. Lancer avec `PYTHONUNBUFFERED=1` en
   tête de commande dès qu'il faut rediriger + tuer plus tard. Ça a fait
   perdre du temps à tort conclure "aucune réponse du pont" alors que
   `PING`/`PONG` circulaient très bien.
5. **Toujours tuer proprement le process de capture** (`kill $PID; wait
   $PID`) avant d'en relancer un autre sur le même port — un `run_in_background`
   dont le script interne fait déjà son propre `&`/`sleep`/`kill` peut se
   terminer (et donc être vu comme "complété") avant que le sous-process
   detaché soit réellement mort, laissant un doublon actif (voir point 3).
6. **Un pont qui répondait hier soir et ne répond plus ce matin n'est pas
   forcément cassé** : un reset logiciel répété (RTS via `idf.py
   monitor`/`flash`) peut laisser l'écran dans un état incohérent qu'un
   `RESET` protocolaire ne rattrape pas. Avant de creuser côté firmware,
   **couper l'alimentation physiquement (débrancher l'USB) sur chaque carte
   à tour de rôle**, pas juste demander un reset logiciel.
7. **`firmware/can-selftest` (auto-test transceiver, `TWAI_MODE_NO_ACK` +
   boucle vers soi-même) exige le câble CAN débranché sur les deux cartes** —
   sinon on ne sait plus si un échec vient du transceiver local ou du câble/de
   l'autre carte. Vérifier aussi `PINOUT_CANPAL` et `TEST_MODE_TWAI` en tête de
   fichier avant de flasher : ce sont des `#define` de bring-up, pas figés,
   et le brochage actuel du XIAO est le **Unit CAN** (`PINOUT_CANPAL 0`,
   TX=8/RX=7), pas l'ancien CAN Pal.
8. **Un `LOG PRESENCE_LOST` répété toutes les ~3 s juste après un
   rebranchement n'est pas forcément une régression** : les compteurs
   d'erreur TWAI (`LOG TWAI_ERROR_COUNTERS`, `arg16` = rx/tx error counter,
   `arg32` = bus_error_count cumulé) mettent quelques dizaines de secondes à
   redescendre après une coupure/reprise du bus. Regarder si `arg16` décroît
   vers 0 et si `arg32` cesse de grimper avant de traiter ça comme un vrai
   problème de câblage/terminaison.

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

**Ce qui nécessite le 230 V et ce qui n'en a pas besoin** (question posée le
2026-09-09, à trancher avant d'attaquer la phase) :

- **Sans 230 V** : XDB401 (capteur de pression, I2C pur) et débitmètre (ISR
  GPIO pur) — les deux premiers points ci-dessous. Testables sur batterie,
  Mac au secteur, sans précaution particulière.
- **Avec 230 V** : dimmer (bloqué en `Calibrating...` sans secteur, voir le
  piège documenté) et donc tout ce qui suit dans l'ordre ci-dessous, y
  compris la vérification de sécurité (bail/présence/verrou/réarmement),
  qui porte sur les actionneurs.
- **Option envisagée, pas encore décidée** : avancer le WebSocket (phase 6,
  point 3 — miroir du trafic CAN, même format qu'en USB) *avant* de brancher
  le 230 V, pour pouvoir continuer à tester sans dépendre du câble USB une
  fois la carte en boîte. Ça découplerait "SSR/dimmer nécessitent le
  secteur" de "l'outil Mac nécessite l'USB" — mais suppose d'avoir déjà le
  Wi-Fi (phase 6, point 1) et un minimum de HTTP/WebSocket côté écran, donc
  une partie de la phase 6 avant la fin de la phase 5. À reconsidérer une
  fois les deux premiers points (XDB401, débitmètre) faits.

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

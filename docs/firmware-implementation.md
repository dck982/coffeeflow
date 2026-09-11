# Firmware — séquence d'implémentation

Conception, protocole et décisions de fond : `docs/firmware.md`. Plan détaillé
de la phase 6 : `docs/plan-phase6.md`. Ce fichier conserve l'état de mise en
oeuvre, les résultats de banc et les contraintes utiles pour reprendre le
travail.

## Où on en est

### Phase 6, lots 1 et 2 — faits (2026-09-09)

`firmware/screen/main/main.cpp` a été séparé en modules : carte (`board`),
CAN et présence (`can_link`), pont série (`serial_bridge`), OTA local et proxy,
écran de service, et coeur machine. Le registre de sortie du CH422G est
maintenu en RAM et modifié uniquement par `ch422g_set_bit()` : écrire les bits
directement ferait perdre l'état des autres sorties.

L'écran de service RGB 800x480, tactile GT911 et LVGL est fonctionnel. Le
panneau exige les timings Waveshare suivants : HSYNC `48/88/40`, VSYNC
`3/32/13`, PCLK 16 MHz. Les mises à jour LVGL sont stables lorsque
l'initialisation LCD et LVGL tournent sur le coeur 1, tandis que CAN, UART,
pont et OTA tournent sur le coeur 0. `CONFIG_LCD_RGB_RESTART_IN_VSYNC` reste
désactivé. Cette répartition est une contrainte d'architecture, pas une simple
optimisation.

Les régressions factory ont été rejouées : PING/PONG via le pont, OTA de
`sensors` par CAN et OTA local de `screen`.

### Phase 6, lot 3 — coeur et télémétrie, fait (2026-09-10)

`core/core.h/.cpp` maintient un instantané cohérent des capteurs, de leur
validité et âge, des versions, de l'état dimmer et des compteurs TWAI. Il est
alimenté par `STATUS_PRESSURE`, `STATUS_FLOW`, `STATUS_ACTUATORS`, `PONG` et
`LOG`. L'écran de service ne lit que cet instantané.

La tâche de télémétrie sur le coeur 0 demande les statuts au repos : pression
toutes les 500 ms, débit et actionneurs toutes les 1 000 ms. Le XIAO diffuse
aussi `STATUS_ACTUATORS` périodiquement. Le débit affiché est calculé sur une
fenêtre configurable `kFlowWindowPulses` (10 impulsions actuellement) et est
remis à zéro après 3 s sans front.

La présence est une machine à états sur les deux noeuds : toute trame valide
du pair la maintient. Après 1,5 s de silence, elle envoie trois `PING` à 500 ms
d'intervalle puis passe à `PRESENCE_LOST`; la première trame reçue la rétablit.
Le PING/PONG servant à valider une image OTA est distinct de cette présence.
Le bit 3 de `STATUS_ACTUATORS` signifie définitivement `dimmer_error_active`.

Validé sur le banc : builds ESP-IDF des deux projets, 27 tests hôte
`coffeetool`, flash USB hash-vérifié, `CAN OK` à l'écran et télémétrie au repos
(`0,02 bar`, `25,3 C`, `0,00 ml/s`). Débrancher le XDB401 retire les valeurs de
pression/température et les rétablit au rebranchement. Couper le XIAO donne
`CAN PERDU` après environ 3 s, puis `CAN OK` dès la reprise des trames. Le
dimmer sans secteur est attendu présent mais non prêt. Un souffle dans le
débitmètre a fait monter le compteur affiché à 176, confirmant que le coeur
reçoit et expose `STATUS_FLOW`. Le test SSR 30 s sous 230 V est reporté au
test général juste avant l'installation dans la machine.

### Phase 6, lot 4 — Wi-Fi, provisioning et face configuration, fait (2026-09-10)

NVS de configuration et credentials Wi-Fi, provisioning AP/STA, diagnostic
réseau et commandes de service. Les calibrations sont versionnées dans
l'image (`core/calibration_machine.h`), hors NVS.

**Contrainte d'init.** L'OTA `v0.2.22` a laissé la dalle noire (abort/reboot
en boucle) : `net_wifi::init()` allouait la pile Wi-Fi et httpd avant les
bounce buffers DMA du panneau RGB, d'où une pression sur la SRAM interne
(`lcd_rgb_panel_alloc_frame_buffers: no mem for bounce buffer`). L'ordre
obligatoire est `service_screen::init()` (coeur 1) puis `net_wifi::init()`
(coeur 0). Reproduit avec `firmware/screen-lcd-test`.

**Image `v0.2.23` livrée par OTA et validée.** 632/632 blocs vers le slot
inactif, PONG `écran v0.2.23` répété (pas de rollback), télémétrie au repos.
Écran de service stable, sans glitch. Provisioning vérifié : AP au premier
démarrage, formulaire `http://192.168.4.1`, association (`RESEAU: CONNECTE`),
persistance après coupure (AP temporaire non remonté), *Oublier le réseau* →
AP immédiat, reprovisioning réussi. SSID volontairement faux : l'écran reste
en STA (`RESEAU: COUPE`, événements `WIFI PERDU` répétés), sans repli AP ;
le bouton *Oublier le réseau* ramène l'AP. Lot 4 clos.

### Phase 6, lot 5 — serveur HTTP, fait (2026-09-10)

`net_http` démarre uniquement après l'association STA et s'arrête avec elle.
Il expose, derrière le bearer token défini dans `secrets.h`, `GET /telemetry`,
`GET` et `POST /config`, et `POST /action`. C'est un traducteur du coeur :
la télémétrie provient de son instantané, la configuration passe par sa
validation transactionnelle et la commande brute par sa face actions. Les
routes de provisioning AP restent, elles, sans bearer token.

**Image `v0.2.24` livrée par OTA et validée.** Build ESP-IDF réussi, puis PONG
de l'écran en `v0.2.24`. Les tests LAN sur `192.168.2.196` ont validé une
télémétrie fraîche et cohérente (pression, température, débit, présence CAN,
versions et réseau STA), la sauvegarde de `/config`, une modification
partielle (`brew.target_weight_g`), la restauration complète, et le rejet
atomique d'une valeur hors bornes avec `400` et
`brew.target_weight_g`. Une action à dimmer hors bornes est refusée avec `400`
et `dimmer`; une requête sans `Authorization` reçoit `401`. Aucun secret ne
figure dans les artefacts de test. Lot 5 clos.

### Phase 6, lot 6 — WebSocket miroir, fait (2026-09-10)

`GET /ws` est enregistré sur le même `httpd`, donc seulement en STA connecté,
et vérifie le même bearer token pendant le handshake. Il diffuse le PDU CAN nu
(sans COBS) dans les deux sens : réceptions TWAI, émissions locales de l'écran
et trames relayées depuis l'USB. Le miroir est lecture seule ; une trame envoyée
par un client ferme sa session. Il accepte jusqu'à trois clients, publie depuis
la tâche CAN par travail différé (sans E/S réseau), puis déconnecte un client
dont l'envoi échoue. Le WebSocket est fermé avec `net_http` à la perte STA.
`coffeetool monitor --ws … --token …` (ou `COFFEEFLOW_HTTP_TOKEN`) porte le
header `Authorization` et utilise toujours le décodeur PDU commun.

**Image `v0.2.24` livrée par OTA et WebSocket validé.** Le test LAN a affiché
en temps réel les `STATUS_PRESSURE`, `STATUS_FLOW`, `LOG FLOWMETER_SILENT` et
les `REQSTATUS` écran→capteurs, donc les deux sens du miroir et le décodage PDU
commun. La version de l'image n'a pas été incrémentée pour ce lot : elle reste
`v0.2.24`; le prochain artefact devra passer en `v0.2.25`.

### Phase 6, lot 7 — OTA réseau, fait (2026-09-10)

`POST /firmware?target=screen` reçoit l'image en streaming directement dans
le slot OTA inactif et répond avant le redémarrage. Pour `target=sensors`,
l'écran reçoit d'abord l'image dans `ota_staging` (repli transitoire sur
`assets` pour les cartes dont la table n'a pas encore été réécrite), puis une
tâche CAN envoie `BEGIN`, blocs de 2 ko acquittés et `END`, à 2 ms entre les
trames. Le coeur interdit le flash sans capteurs vivants ou avec un écho
actionneur frais indiquant une sortie active; l'absence d'écho est admise
quand le dimmer est sans secteur. Il coupe toujours les sorties et suspend
`REQSTATUS` pendant l'opération. La progression est exposée dans
`/telemetry` et l'écran de service.

Les deux cibles ont été livrées par HTTP : écran `v0.2.28`, puis capteurs.
La validation de l'image capteurs et la reprise des statuts ont été observées
sur CAN. Les deux nœuds ont ensuite été livrés en `v0.2.30` : PING transporte
désormais l'identité (nœud, version, uptime), comme PONG; chaque nœud en
envoie un au démarrage, le récepteur exploite immédiatement cette identité et
répond par PONG. Les PING DLC 0 des images anciennes restent acceptés. Après
un power cycle des capteurs, le banc a confirmé `PING capteurs v0.2.30`,
`PONG écran v0.2.30`, `BOOT`, `READY`, puis les trois `STATUS_*` frais, sans
PING de présence tant que les `REQSTATUS` circulent. Lot 7 clos.

## Historique compact des phases terminées

### Images factory — figées et écrites (2026-09-10)

Les images de secours des deux cartes sont figées au commit
`451566c980ccb6935284ebad8b9b5a400a2db413` (`v0.2.9`) et archivées, avec leurs
SHA-256 et procédure, sous `firmware/factory-images/2026-09-09-451566c/`.
Elles ont été écrites et hash-vérifiées à l'offset `0x20000` de leurs
partitions `factory`, sans toucher `otadata`, NVS ni les slots OTA. L'écran
factory est le pont USB↔CAN et sait reflasher l'écran localement et le XIAO à
travers CAN ; le XIAO factory est le filet CAN/OTA sûr des sorties.

### Phases 0 à 2 — protocole, bus et sécurité

- Le protocole commun (PDU, COBS et CRC16), `coffeetool`, le pont USB-C/CAN et
  le PING/PONG ont été validés sur les deux cartes.
- Le CAN de l'écran Waveshare utilise le transceiver sélectionné par `CAN_SEL`
  (EXIO5 du CH422G), avec TX GPIO20 et RX GPIO19. Le port série du pont est
  UART2 sur GPIO43/44 via le CH343P externe.
- Des entrées UART restées sur leur fonction IOMUX empêchaient la réception.
  La correction durable consiste à forcer le pad en GPIO et à activer son
  entrée après `uart_set_pin()`; les lectures UART emploient des délais courts,
  jamais `portMAX_DELAY`.
- Bail, perte de présence, verrou de marche continue à 60 s et réarmement ont
  été vérifiés sur matériel. Toute perte de présence force les actionneurs à
  l'arrêt; le verrou ne se lève qu'après une coupure d'alimentation réelle.

### Phase 4 — OTA dans les deux sens

Les deux cartes supportent `FLASH_CTRL` et `FLASH_DATA` avec blocs de 2 ko,
CRC16 par bloc, CRC32 final, partition inactive, `PENDING_VERIFY` et rollback
après temporisateur si l'image ne valide pas un PING/PONG CAN réel. Les quatre
scénarios ont été validés : image saine et image volontairement cassée sur
`sensors` et `screen`, ainsi qu'interruption de transfert sans modifier
`otadata`.

Limite connue : après une erreur CRC de bloc côté `sensors`, une retransmission
n'est pas distinguée explicitement d'un bloc suivant. Le CRC32 final protège
l'image flashée; corriger l'identification de retransmission avant de rendre
le transport moins fiable qu'un CAN de banc.

### Phase 5 — capteurs et actionneurs

**XDB401.** La conversion I2C utilise GPIO5/6. La pression et la température
doivent être lues dans deux transactions séparées, conformément au capteur;
une lecture groupée donnait des pressions incohérentes. Les octets sont
transportés tels quels dans le PDU little-endian, mais la formule physique du
fabricant doit être appliquée à l'ordre big-endian des registres. Testé par
souffle et par débranchement : `STATUS_PRESSURE.flags.bit0` indique la validité
réelle du capteur.

**Débitmètre Digmesa.** GPIO44/D7 compte les fronts descendants avec le pull-up
interne désactivé, le filtre RC du shield assurant la polarisation. Le GPIO44
était aussi l'entrée UART0 par défaut : le forcer en fonction GPIO et activer
l'entrée est indispensable. Un comptage nul persistant venait ensuite d'un
câble inversant VCC et SIGNAL sur le connecteur PANCOM; le câblage final est
documenté dans `docs/cablage.md`. Testé par souffle. `STATUS_FLOW.flags.bit0`
est fixé à 1, car ce capteur GPIO seul ne se détecte pas électriquement.

**SSR et dimmer.** La chaîne `SET` -> bail -> `STATUS_ACTUATORS` a été validée
sur charge de test. Le dimmer demande le secteur pour devenir prêt; sans 230 V,
son erreur de calibration est donc normale. Une inversion phase/neutre sur le
bornier du dimmer a été corrigée. L'état `STATUS` (READY/ERROR), et non le seul
registre `ERROR`, est la source des flags; `AC_FREQ` est informatif seulement.
Ne pas envoyer `RECALIBRATE` au boot. Une coupure secteur conserve le niveau
programmé, puis le dimmer le reprend après son recalibrage naturel.

La barrière B (capteurs, actionneurs et sécurité) et la barrière C (OTA) sont
atteintes. Les détails de registres dimmer sont dans `docs/dimmerlink-i2c.md`.

## Environnement de build et flash

Chaque nouveau terminal doit charger ESP-IDF 6.1 :

```sh
source ~/.espressif/tools/activate_idf_v6.1.sh
```

Construire depuis le répertoire du projet concerné :

```sh
idf.py build
```

`idf.py build` imprime une ligne par fichier compilé (souvent plusieurs
milliers). Pour ne garder que les erreurs et le résultat final, rediriger vers
un fichier puis filtrer, sans perdre le code de sortie réel :

```sh
idf.py build > /tmp/build.log 2>&1; ec=$?
grep -iE 'error|failed' /tmp/build.log
tail -n 10 /tmp/build.log
```

Identifier les ports avec `esptool.py --port <port> flash-id` : le XIAO a 8 Mo
de flash et l'écran 16 Mo. En exploitation, le flash USB direct est réservé à
l'écriture ou à la récupération de l'image `factory` :

```sh
idf.py -p /dev/cu.usbmodemXXXX flash
```

### Flash d'une image fonctionnelle de l'écran

Ne pas utiliser `idf.py flash` pour une image fonctionnelle : il réécrit la
table de partitions, NVS et/ou `factory` selon la configuration locale. Le
contrat est de toujours livrer l'écran par OTA, via son pont USB↔CAN, vers le
slot OTA inactif :

```sh
cd firmware/tools
source ~/.espressif/tools/activate_idf_v6.1.sh
python -m coffeetool.cli flash \
  --port /dev/cu.wchusbserialXXXX \
  --dest screen ../screen/build/screen.bin
```

Avec le rythme CAN volontairement limité à 2 ms par trame, l'image écran
d'environ 1,2 Mo prend environ huit minutes (632 blocs de 2 ko). Ne pas
interrompre le pont ni l'alimentation pendant ce transfert. La confirmation
finale puis le PING/PONG au redémarrage valident l'image `PENDING_VERIFY` ;
en l'absence de validation, ESP-IDF revient à l'image précédente.

Exception de banc : `firmware/screen-lcd-test` (firmware jetable, sans CAN ni
NVS applicative) peut être écrit par `idf.py flash` complet. Sa table de
partitions n'est pas celle de production (`factory` 1408 KiB contre 1000 KiB) :
restaurer d'abord `firmware/screen/build/partition_table/partition-table.bin`
à `0x8000`, puis l'image factory à `0x20000`, par `esptool.py write_flash` —
jamais `idf.py flash`. Cette exception ne s'applique jamais à l'image
fonctionnelle `screen`.

Après un flash direct du banc, restaurer et sélectionner la factory écran :

```sh
esptool.py --chip esp32s3 --port /dev/cu.wchusbserial5B790235091 write_flash \
  0x8000 firmware/screen/build/partition_table/partition-table.bin \
  0x20000 firmware/factory-images/2026-09-09-451566c/screen-factory.bin
esptool.py --chip esp32s3 --port /dev/cu.wchusbserial5B790235091 \
  erase_region 0x10000 0x2000
```

Le second ordre efface seulement `otadata` : le bootloader choisira donc la
partition `factory`. Son écran noir est normal, puisque cette image ne porte
pas LVGL ; le port USB↔CAN reste, lui, disponible.

Les outils hôte sont sous `firmware/tools/coffeetool` et s'exécutent avec
`python -m coffeetool.cli ...`. Un seul processus peut ouvrir un port série à
la fois; le moniteur peut afficher les trames avec retard lorsqu'il rattrape un
buffer important.

## Reprise du banc

1. Vérifier `CAN OK`, pression/température et débit au repos sur l'écran.
2. Vérifier les compteurs TWAI et un PING/PONG via `coffeetool` avant tout test
   applicatif.
3. Garder le test SSR/dimmer sous 230 V pour la répétition générale juste avant
   l'installation. Utiliser une charge de test adaptée et les précautions de
   banc décrites dans `docs/firmware.md`.
4. Ne pas confondre validité du XDB401 et présence du XIAO : le premier enlève
   seulement les mesures de pression/température; le second produit `CAN
   PERDU`.

## Suite de la phase 6

Le découpage précis et les critères de sortie sont dans
`docs/plan-phase6.md`. Les lots 0 à 9 sont implémentés ; la validation de banc
du lot 9 est volontairement regroupée après l'UI du lot 10. Ordre restant :

1. UI LVGL complète (lot 10), puis validation physique commune du lot 9 :
   shot, purge et sécurités depuis ses grandes cibles tactiles. LVGL et HTTP
   restent des clients sans accès direct aux sorties.
2. Geler la table de partitions et fermer la phase (lot 11).

Le code de référence Acaia est dans `docs/reference/acaia-ble/`; il faut porter le
protocole vers GATT ESP-IDF, non compiler le code Arduino tel quel. LVGL reste
en v9 via `espressif/esp_lvgl_port`, avec bounce buffer obligatoire sur le
panneau RGB. Les choix d'interface sont dans `docs/ui.md` et
`docs/ui-mockup.html`.

### Lot 8 — client BLE Acaia et politique radio, fait (2026-09-11)

L'image écran `v0.2.31`, qui initialisait LCD/LVGL, Wi-Fi puis NimBLE, a
redémarré en boucle dès le boot. Le rollback OTA n'a pas sélectionné le slot
précédent ; l'effacement de `otadata` a permis de redémarrer l'image factory.
L'écran noir de cette image `v0.2.9` est normal : elle est un pont USB↔CAN de
récupération, sans LCD/LVGL.

Le banc jetable `firmware/screen-lcd-test` isole désormais les séquences
`LCD`, `LCD→Wi-Fi`, `LCD→NimBLE`, `LCD→Wi-Fi→NimBLE` et une alternance
Wi-Fi/NimBLE. Le test complet affiche la mire puis échoue à l'initialisation
du contrôleur BLE : `BLE_INIT: Malloc failed`, pour une allocation interne de
4 KiB. À l'inverse, `LCD→NimBLE` démarre et reste vivant. Conclusion
partielle : le problème est la disponibilité ou la fragmentation de SRAM
interne après LCD/RGB et Wi-Fi, non le protocole Acaia ni CAN. La PSRAM ne
résout pas cette allocation : le contrôleur BLE exige lui-même de la SRAM
interne. L'alternance réelle, toutes les cinq secondes, a été validée le
2026-09-10 : Wi-Fi/httpd/netif s'arrêtent entièrement, puis NimBLE démarre ;
NimBLE et son contrôleur s'arrêtent entièrement, puis Wi-Fi repart. Aucun
reset ni échec d'allocation ne survient. Les avertissements IRK initiaux
(`rc=8`, fonctionnalité de persistance absente) venaient de la confidentialité
NimBLE activée par défaut, inutile au client central de banc ; elle est
désormais désactivée.

Le reboot de l'image principale avec les radios encore inactives a ensuite
montré une seconde limite, distincte. Les trames du pont ont localisé chaque
abort entre `LCD_INIT_STEP=2` et `LCD_INIT_STEP=3`, dans
`esp_lcd_new_rgb_panel()`. Lier le contrôleur BLE ajoute environ 18,2 Kio de
code en IRAM même sans appeler son initialisation. Il ne restait que 129 082
octets de DIRAM après le link, alors que le panneau demande deux bounce buffers
DMA internes de 64 000 octets, avant ses autres allocations. Le deuxième
buffer échouait et `ESP_ERROR_CHECK` redémarrait la carte ; la console texte
désactivée masquait le message. `Saved PC` pointait seulement sur
`esp_cpu_wait_for_intr()` de l'autre coeur, pas sur la cause.

Le correctif conserve les bounce buffers de 40 lignes validés pour la
stabilité vidéo et déplace les chemins rapides non indispensables vers la
flash : `CONFIG_ESP_WIFI_IRAM_OPT=n`, `CONFIG_ESP_WIFI_RX_IRAM_OPT=n` et
`CONFIG_BT_NIMBLE_LOW_SPEED_MODE=y`. Le build laisse alors 147 646 octets de
DIRAM après le link (+18 564), et l'image démarre sur la carte avec LCD/LVGL.
La priorité est la marge SRAM, pas le débit maximal : le Wi-Fi transporte
ordinairement environ 1 Kio et ne sert qu'occasionnellement au flash, tandis
que la balance publie au plus à 10 Hz.

**Décision d'architecture validée par le banc.** Wi-Fi et BLE ne seront plus
deux services permanents. Le coeur possède un mode radio exclusif
`machine` ou `wifi` :

- **Mode machine** (défaut) : BLE est chargé, la balance reste visible et
  `scale_present` choisit l'objectif poids/temps. Wi-Fi et httpd sont
  complètement désinitialisés.
- **Mode Wi-Fi** : demandé explicitement par un bouton de l'UI, il affiche un
  bandeau `WIFI MODE`, charge Wi-Fi/httpd et rend disponibles diagnostic,
  flash réseau, purge de banc et envoi différé du dernier shot au backend.
  L'infusion et toute action qui lancerait un cycle sont refusées par le
  coeur, pas seulement masquées dans l'UI.
- Quitter ce mode arrête et désinitialise Wi-Fi/httpd/netif avant de relancer
  BLE. Le pont USB↔CAN reste toujours disponible.

L'implémentation de la transition est en place dans `firmware/screen` :
l'écran de service demande `ACTIVER WIFI` ou `ACTIVER BLE` au coeur, qui
effectue l'arrêt complet de la pile opposée sur le coeur 0. L'image corrigée
a démarré le 2026-09-10 et les trois transitions de bring-up ont été
exercées. Mesures affichées (`libre`, plus grand `bloc`, minimum historique),
en octets :

| État | Libre | Plus grand bloc | Minimum |
| --- | ---: | ---: | ---: |
| aucune radio, après boot | 85 163 | 31 744 | 48 736 |
| Wi-Fi connecté | 23 275 | 14 336 | 18 744 |
| BLE initialisé | 33 459 | 18 432 | 14 520 |
| aucune radio, après arrêt | 75 479 | 22 528 | 14 520 |

**Image `v0.2.37` validée par OTA.** Une Lunar peut répartir l'UUID de
service et le nom entre l'advertising et la scan response. Le filtrage de
doublons par périphérique du contrôleur S3 masquait alors le second paquet au
client ; le scan ne déduplique donc plus les paquets. Le moniteur CAN expose
maintenant `BLE_SCAN_STARTED`, `BLE_SCALE_FOUND`, `BLE_CONNECTED`,
`BLE_SUBSCRIBED`, `BLE_DISCONNECTED` et `BLE_ERROR` : le bring-up est lisible
sans console ESP-IDF. La cadence de publication BLE est prête à passer de
1 Hz au repos à chaque notification avec le profil de télémétrie actif du lot
9. Le build ESP-IDF réussit. La validation sur la Lunar réelle a confirmé la
topologie legacy `0x1820` / `0x2A80` (`WRITE_NR|NOTIFY`) / CCCD `0x2902`, puis
le protocole applicatif et son battement. Le poids affiché par CoffeeFlow est
identique à celui de la balance; tare, poids négatif, déconnexion, reconnexion
et `scale_present` ont été vérifiés. Le passage en mode Wi-Fi arrête bien BLE
et déconnecte la Lunar, conformément à la politique radio exclusive.

Le premier essai `v0.2.35` a révélé une course NimBLE : `ble_gap_connect()`
était appelé pendant que le scan était actif, provoquant la séquence répétée
`BLE_SCALE_FOUND` puis `BLE_SCAN_STARTED`. `v0.2.37` annule le scan puis
demande immédiatement la connexion, comme le client NimBLE de référence ;
l'annulation retire le callback du scan et ne livre pas de
`BLE_GAP_EVENT_DISC_COMPLETE` à l'application. La connexion passe aussi
`nullptr` pour employer les paramètres NimBLE par défaut valides.

L'écran affiche à tort `RADIO: AUCUNE` en mode BLE : le libellé est encore
déduit de `NetworkState::kOff` au lieu de `RadioMode::kMachine`. C'est un
défaut d'observation de l'UI, pas la preuve que BLE n'a pas démarré. Le
tri-state `off`/`machine`/`wifi` est conservé, mais `off` ne doit plus être
l'état stable de boot : après LCD/LVGL, le firmware demande désormais le mode
`machine` et donc charge BLE par défaut. Les allocations Wi-Fi/LwIP préfèrent
aussi la PSRAM et le seuil des allocations ordinaires préférant la mémoire
interne passe de 16 Kio à 4 Kio ; la réserve DMA/interne reste à 32 Kio.
La connexion, les notifications et le retour Wi-Fi → BLE ont depuis été
validés sur la Lunar réelle.

Après le reboot persistant de cette image, le bring-up passe temporairement à
trois états : `sans radio` au démarrage, puis `BLE` ou `Wi-Fi` demandé sur
l'écran de service. Celui-ci affiche la SRAM interne libre, son plus grand
bloc contigu et son minimum historique avant l'activation de BLE. Ce banc
permet de séparer l'initialisation LCD/CAN de celle du contrôleur BLE ; il ne
remplace pas la politique finale à deux modes `machine`/`Wi-Fi`.

La face actions du lot 9 devra donc tester ce mode avant d'accepter un brew,
et l'UI du lot 10 le rendra visible ; HTTP et LVGL restent des clients du
coeur, sans accès direct aux sorties.

### Décalage fixe de l'affichage RGB (2026-09-10, corrigé)

L'image apparaissait environ 150 pixels trop à droite et légèrement trop bas,
alors que le GT911 renvoyait les coordonnées LVGL logiques correctes. La cause
était un démarrage désynchronisé du flux RGB/DMA avant que le premier rendu
LVGL soit complet. Une demande unique `esp_lcd_rgb_panel_restart()` une
seconde après la construction de l'UI remet l'image en place ; le correctif est
validé sur le matériel avec `v0.2.33`.

Ne pas compenser le GT911 et ne pas activer
`CONFIG_LCD_RGB_RESTART_IN_VSYNC` : le tactile n'était pas fautif et un restart
à chaque VSYNC avait auparavant provoqué des sauts. Le détail historique est
dans `docs/screen-issue.md`.

### Lot 9 — cœur, face actions, implémenté (2026-09-11)

`core/machine.h/.cpp` est une machine à états pure, sans ESP-IDF, couvrant
pré-infusion, extraction, ramp-down, arrêt temps ou poids, arrêt manuel,
perte/recul de balance et purge homme-mort plafonnée. Le cœur l'exécute toutes
les 50 ms, émet seul les `SET` avec un bail de 500 ms renouvelé à 10 Hz, et
coupe immédiatement à l'arrêt. `/action` expose désormais infusion, arrêt,
purge, tare et fermeture du résumé ; `/telemetry` expose le cycle et la
dernière infusion volatile.

Le résumé (poids, durée, débit, heure prise au départ si connue) est remplacé
par le shot suivant et effacé à la fermeture. Les transitions sont observables
par les LOG CAN `BREW_*` et `PURGE_*`. Les tests hôte de `core/machine` et le
build ESP-IDF de l'écran sont verts. Les essais physiques sont reportés après
le lot 10, conformément à la décision de validation commune.

**Image `v0.2.38` flashée et fonctionnelle sur le banc.** Le cycle et les
actions du lot 9 semblent fonctionner. Régression à conserver pour le lot 10 :
au démarrage, le contenu RGB est parfois décalé vers la droite. Le
`esp_lcd_rgb_panel_restart()` différé d'une seconde, ajouté pour resynchroniser
le DMA (`service_screen.cpp`), ne corrige pas le problème de façon fiable.
Ne pas investiguer dans ce lot ; reproduire et isoler le problème pendant le
travail UI.

### Lot 10, sous-lot 1 — dalle et jetons, validé (2026-09-11)

La mire LVGL expose les huit jetons de couleur, les sept corps Inter prévus
par `ui.md`, et un retour tactile. Les corps texte embarquent latin-1
(`U+00A0–U+00FF`) ; les corps héros ont un jeu de glyphes restreint aux valeurs
café. `LV_USE_FONT_COMPRESSED` doit rester activé : `lv_font_conv` compresse
les bitmaps par défaut et, sans le décodeur LVGL, toutes les étiquettes sont
invisibles alors que les formes restent visibles.

Validé sur la dalle : les accents français et le GT911 fonctionnent. Le titre
affiche la version d'image courante. Tant que la destination Wi-Fi de l'UI
n'est pas écrite (sous-lot 4), la mire conserve un bouton explicite pour
entrer/quitter le mode Wi-Fi ; il passe par le cœur, coupe BLE avant
Wi-Fi/httpd et montre l'état AP ou STA. Ce bouton est provisoire et sera
absorbé par la destination Wi-Fi finale.

### Lot 10, sous-lot 2 — bandeau L0 et repos statique, validé (2026-09-11)

`ui/ui_root.cpp` installe l'écran LVGL unique et `ui/ui_home.cpp` reproduit le
premier cadre de la maquette sur des valeurs figées : bandeau L0, filet,
cible poids encadrée de `−`/`+`, étiquette de paramètres et rangée
*infuser/purge/réglages*. Les cotes sont celles de `ui.md`; aucune télémétrie,
aucune configuration et aucune action d'infusion ne sont encore lues ou
déclenchées par cette étape.

La version d'image reste visible dans le bandeau. Le bouton de service `wifi`
est conservé jusqu'à la destination Wi-Fi du sous-lot 4 : il demande la
transition radio au cœur, qui arrête BLE avant Wi-Fi/httpd, et son second
appui demande le retour au mode machine. Build ESP-IDF vert et validation
visuelle sur dalle effectuée avec l'image `v0.2.43` : cotes du cadre 1,
texte, boutons, version et accès Wi-Fi sont lisibles et corrects. Le bouton
`-` emploie le tiret ASCII (le signe moins Unicode n'est pas dans la police).

### Lot 10, sous-lot 3 — modèle vivant, clos (2026-09-11)

L'arbre LVGL unique est maintenant rafraîchi à 10 Hz au plus depuis le
`core::Snapshot` cohérent : température, poids, présence de la balance et
cible poids/temps sont rendus sans lecture directe du CAN. La fraîcheur et la
validité restent celles du cœur ; une mesure manquante est affichée par le
tiret ASCII de repli de la police héros, jamais comme `0,0`.

La feuille `diagnostic` rend les mêmes données ligne à ligne, notamment l'état
`absent` de pression/température et du bus. Les priorités L2 boot, verrou,
module interne injoignable et mise à jour préemptent cet arbre. Build ESP-IDF
vert et banc validé : affichage stable, bascule temps/poids à la connexion de
la balance et poids réel rafraîchi. Le débranchement XDB401/CAN reste une
régression matérielle à rejouer, sans bloquer la clôture du sous-lot. Le
power cycle des capteurs a depuis validé le second cas : L2 `module interne
injoignable` apparaît pendant l'absence puis se ferme au retour du module.

**Observation de banc à suivre (2026-09-11).** Après deux flashs, l'écran est
arrivé sur L2 `module interne injoignable`; redémarrer l'écran ou les capteurs
a rétabli le fonctionnement. Ne pas conclure à un premier PING perdu : sur un
démarrage propre capturé à `19:36:10`, le PING écran, `PRESENCE_RESTORED`, PONG
capteurs et les trois `STATUS_*` ont tous été observés avant le premier
`LCD_INIT_STEP`, puis `READY` à `19:36:11.763`. Si le cas se reproduit,
conserver le log `coffeetool monitor` depuis le boot : il faut déterminer si
la présence n'est pas relancée, si les trames sont perdues, ou si seul le
rendu L2 reste figé malgré une présence restaurée.

**Observation de banc à différer (2026-09-11).** Lors d'un flash HTTP, le
panneau RGB peut perdre complètement sa synchronisation ; elle revient parfois
seulement lorsque L2 `mise à jour` est rendu. La piste principale est une
priorité/latence défavorable entre l'ISR LCD (cœur 1) et la pile Wi-Fi/httpd
(cœur 0), malgré l'affinité prévue. Ne pas ouvrir cette investigation pendant
le lot UI : la correction sera traitée après sa finalisation, dans une session
de stabilité LCD/radio dédiée.

### Lot 10, sous-lot 4 — interactions et actions locales, en cours (2026-09-11)

État de reprise : les pas de cible, pavé numérique, persistance NVS, purge
homme-mort et destination Wi-Fi existent. Restent à fermer avant de déclarer
le sous-lot fini :

- [x] rendre accessibles tous les paramètres tactiles de `core::Config` ;
- [x] confirmer l'entrée Wi-Fi comme la réinitialisation réseau ;
- [x] implémenter atténuation (4 min) et veille L4 (30 min), réveil sans
  action ;
- [x] générer et examiner les captures de régression hôte.
- [ ] validation sur dalle : persistance après coupure, purge au relâchement
  et au plafond, réveil L4 sans action.

Les cibles `-` et `+` modifient la cible active (poids lorsque la balance est
présente, temps sinon) par `core::put_config()` ; toucher sa valeur ouvre le
pavé numérique LVGL pour les grands écarts. La persistance, les bornes et le
refus pendant un cycle restent donc du ressort du cœur et de NVS, sans chemin
UI parallèle. La feuille `réglages` rend les réglages actifs de cible,
pré-infusion et purge, ainsi que l'entrée Wi-Fi ; l'effacement des credentials
exige une confirmation modale explicite.

`purge` est relié à `PurgePress` au poser du doigt et à `PurgeRelease` au
relâchement ou à la perte de contact. Son délai maximal reste appliqué par
`core::machine`, jamais par un timer LVGL. La destination Wi-Fi devient L2,
montre l'association ou l'adresse IP et son retour demande réellement le mode
machine au cœur. Le raccourci diagnostic est désormais le groupe de présence
du bandeau, conformément à `ui.md`.

La feuille `réglages` est répartie sur trois pages : cibles et pré-infusion,
rampe et niveaux de pompe, puis plafond de purge et réseau. Chaque appui passe
par la validation transactionnelle du cœur et avance la valeur dans sa plage.
Les seuils d'atténuation et de veille restent configurables par `/config`,
mais sont volontairement absents de cette feuille. Entrer en Wi-Fi et effacer
le réseau passent par une confirmation modale ambre, qui ne peut ni recouvrir
les lignes de réglages ni rester affichée au retour du mode machine.

Après `dim_after_s` (240 s par défaut), un calque atténue la façade. Après
`standby_after_s` (1800 s), le calque devient L4 et le bloc `coffeeflow · au
repos` se déplace localement, sans animation plein écran. Le premier toucher
est capturé par ce calque et ne peut donc pas déclencher le contrôle situé
dessous. Les snapshots hôte couvrent l'accueil, les trois pages de réglages,
l'atténuation, la veille et la confirmation Wi-Fi ; le build ESP-IDF reste
vert.

## Phase 7 — mise en boîte

- Archiver et vérifier les deux images factory avec leurs versions.
- Rejouer les essais OTA et rollback par le réseau, cartes encore accessibles.
- Monter le XIAO et l'écran, puis vérifier que l'USB-C de l'écran reste
  praticable avec le panneau de service retiré.
- Refaire un flash complet par le réseau avant de débrancher l'USB.

## Après la mise en boîte

1. Calibrer facteur K, OPV, point de décrochage et courbe dimmer/pression.
2. Implémenter purge, puis infusion au temps et au poids.
3. Ajouter pré-infusion et flow control après caractérisation du débitmètre à
   bas débit; pression et poids restent les signaux rapides.

## Rappels à ne pas perdre

- Une table de codes `LOG` doit être générée depuis une source unique.
- La table de partitions doit être dimensionnée avant la fermeture de la
  machine.
- Un rollback n'est validé qu'avec une image réellement cassée.
- Les pull-ups I2C sont dans le XDB401 : le débrancher peut aussi rendre le
  dimmer I2C muet.
- Ne pas garder le mutex I2C pendant les 50 ms de conversion XDB401.
- Le `PONG` est l'identité et la version observables d'une carte en boîte.

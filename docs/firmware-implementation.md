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
`docs/plan-phase6.md`. Les lots 0 à 7 sont clos. Ordre restant :

1. Client BLE GATT Acaia Lunar et politique radio (lot 8) : porter le
   protocole vers GATT ESP-IDF, valider les UUID et le bit de signe sur la
   balance réelle, puis exposer poids, tare et `scale_present` au coeur.
2. Face actions du coeur : infusion et purge sans écran (lot 9), puis UI LVGL
   complète (lot 10). LVGL et HTTP restent des clients sans accès direct aux
   sorties.
3. Geler la table de partitions et fermer la phase (lot 11).

Le code de référence Acaia est dans `reference/acaia-ble/`; il faut porter le
protocole vers GATT ESP-IDF, non compiler le code Arduino tel quel. LVGL reste
en v9 via `espressif/esp_lvgl_port`, avec bounce buffer obligatoire sur le
panneau RGB. Les choix d'interface sont dans `docs/ui.md` et
`docs/ui-mockup.html`.

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

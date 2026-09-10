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

### Phase 6, lot 4 — incident d'intégration LCD/Wi-Fi, résolu et validé par OTA (2026-09-10)

**Image `v0.2.23` livrée par OTA et validée (2026-09-10).** Après la
restauration factory ci-dessus, l'image `screen` corrigée (les deux
correctifs déjà présents dans `firmware/screen` — ordre LCD avant Wi-Fi,
affinité de cœur — étaient inchangés, seul le patch de version est passé de
22 à 23) a été renvoyée par le pont USB↔CAN vers le slot OTA inactif : 632/632
blocs, code de sortie 0. Confirmé par `coffeetool` après redémarrage :
`écran→capteurs PONG node=écran v0.2.23`, répété à intervalles réguliers —
l'image est validée (pas de rollback), télémétrie normale au repos
(`STATUS_PRESSURE`/`STATUS_FLOW` reçus, `REQSTATUS` réémis par l'écran).
**Lot 4 validé sur le banc (2026-09-10).** Écran de service stable, sans
glitch, boutons présents. Provisioning complet vérifié par David : AP au
premier démarrage, connexion au mot de passe de `secrets.h`, formulaire
`http://192.168.4.1`, association au vrai réseau (`RESEAU: CONNECTE`, IP
attribuée), persistance après coupure d'alimentation (AP temporaire non
remonté), bouton *Oublier le réseau* → AP immédiat, reprovisioning réussi.
Seul point du critère de sortie de `docs/plan-phase6.md` non rejoué ici : un
SSID volontairement faux qui ramène l'AP au bout du délai prévu — à couvrir
si l'occasion se présente, pas bloquant pour clore le lot. Lot 4 clos.

Le lot 4 est implémenté localement (NVS de configuration et credentials Wi-Fi,
provisioning AP/STA, diagnostic réseau et commandes de service), mais il n'est
pas validé sur le banc. L'image écran `v0.2.22` de 1,2 Mo a été envoyée le
2026-09-10 via le pont USB↔CAN vers le slot OTA inactif : les 632 blocs et le
CRC final ont été confirmés par `coffeetool`. Factory et NVS n'ont pas été
écrits. Après démarrage, la dalle est néanmoins entièrement noire, y compris
après un cycle d'alimentation. Le PING/PONG CAN a vraisemblablement validé
l'image `PENDING_VERIFY`, donc le cycle ne garantit plus un rollback.

L'hypothèse principale est une pression/fragmentation de RAM interne :
`net_wifi::init()` démarrait Wi-Fi puis HTTP avant l'allocation des buffers RGB
DMA. Le correctif local, déjà compilé mais **pas flashé**, initialise d'abord
`service_screen::init()` (cœur 1), puis Wi-Fi/HTTP (cœur 0) dans
`main/main.cpp`. Ce n'est qu'une hypothèse : ne pas lancer un second transfert
OTA de huit minutes sans l'avoir reproduite de façon réduite.

Le firmware réduit de diagnostic est maintenant préparé dans
`firmware/screen-lcd-test`. Il ne contient ni CAN, ni pont, ni NVS applicative,
ni credentials CoffeeFlow : il initialise CH422G, un AP Wi-Fi ouvert et un
`httpd` minimal, le panneau RGB/LVGL et une mire explicite à six bandes. La
variante par défaut reproduit l'ordre suspect `CH422G -> Wi-Fi/HTTP ->
LCD/LVGL`; la mire affiche cet ordre. La variante témoin inverse
`CH422G -> LCD/LVGL -> Wi-Fi/HTTP` est sélectionnable dans `menuconfig` ou se
construit sans modifier la configuration A avec
`sdkconfig.lcd-first.defaults` dans `build-lcd-first`. L'AP temporaire est
`CoffeeFlow-LCD-Diag` et `GET /` renvoie seulement un statut.

**Hypothèse confirmée sur matériel (2026-09-10).** La partition `factory` de
`screen-lcd-test/partitions.csv` a été agrandie à 1408 KiB (voir plus bas,
propre à ce projet de test) pour loger le binaire de diagnostic (~1,13 Mo). Un
premier essai a révélé un bug du firmware de diagnostic lui-même, sans lien
avec l'hypothèse testée : `nvs_flash_init()` n'était jamais appelé avant
`esp_wifi_init()`, qui l'utilise en interne pour la calibration radio, d'où un
`ESP_ERROR_CHECK` avorté immédiatement (`ESP_ERR_NVS_NOT_INITIALIZED`) et un
reboot en boucle sans rapport avec le LCD. Corrigé en ajoutant l'appel dans
`app_main()`, avant les deux ordres testés — comme le fait déjà
`storage::init()` dans `firmware/screen/main/main.cpp` avant `net_wifi::init()`.

Une fois ce bug corrigé, la variante suspecte (`CH422G -> Wi-Fi/HTTP ->
LCD/LVGL`) plante précisément à l'allocation du panneau RGB : `lcd.rgb:
lcd_rgb_panel_alloc_frame_buffers: no mem for bounce buffer`, puis abort et
reboot en boucle — rétroéclairage clignotant, contenu noir, pas d'AP visible.
La variante témoin (`CH422G -> LCD/LVGL -> Wi-Fi/HTTP`) boote proprement
jusqu'au bout : LVGL démarre, puis le Wi-Fi (`wifi:mode : softAP`,
`DHCP server started`), puis `mire prete; AP CoffeeFlow-LCD-Diag, HTTP /`. Le
correctif préparé pour `firmware/screen` (LCD/LVGL avant Wi-Fi/HTTP) est donc
validé par ce test réduit ; son ordre correspond déjà à celui de
`firmware/screen/main/main.cpp` (`service_screen::init()` avant
`net_wifi::init()`).

**Second bug de `screen-lcd-test` trouvé et corrigé par le même mécanisme que
le commit `4f74f52b` (2026-09-10).** Une fois la variante témoin bootée, le
texte de la mire était décalé à droite — même symptôme que le glitch de
`docs/screen-issue.md` : `app_main()` tourne sur le cœur 0
(`CONFIG_ESP_MAIN_TASK_AFFINITY_CPU0`) et y appelait directement
`esp_lcd_new_rgb_panel()`, installant l'ISR DMA sur le cœur 0, alors que LVGL
est épinglé au cœur 1 (`task_affinity = 1`). `screen-lcd-test` n'avait jamais
reçu le correctif appliqué à `service_screen.cpp` (tâche dédiée épinglée au
cœur 1 pour installer le panneau RGB). Corrigé à l'identique dans
`firmware/screen-lcd-test/main/main.cpp` : `init_lcd_and_lvgl_on_core1()`
exécute `init_lcd()` et `init_lvgl_and_pattern()` depuis une tâche pinnée au
cœur 1, `app_main` bloque sur un sémaphore. Vérifié sur matériel : texte
centré, AP `CoffeeFlow-LCD-Diag` visible.

**Restauration factory faite et vérifiée (2026-09-10).** Table de partitions
de production réécrite à `0x8000` (régénérée par `idf.py build` dans
`firmware/screen`, pour être sûr qu'elle correspond au `partitions.csv`
courant) puis `screen-factory.bin` (`v0.2.9`, hash re-vérifié avant écriture)
à `0x20000`, par `esptool.py write-flash` — jamais `idf.py flash`. `otadata`
était déjà vierge (`0xFF`, laissé par l'`ota_data_initial.bin` du flash
`screen-lcd-test` au même offset), donc le bootloader retombe sur `factory`
par défaut sans action supplémentaire. Un **power cycle des deux cartes a été
nécessaire** après l'écriture : le port du pont restait dans un état
résiduel avant, sans réponse à PING. Après power cycle, confirmé par
`coffeetool` : `écran→capteurs PONG node=écran v0.2.9`. La carte écran est
revenue à son filet de récupération figé ; prochaine étape, reprendre l'OTA
du lot 4 avec les deux correctifs validés (ordre d'init, affinité de cœur).

**Stratégie retenue (2026-09-10) : `idf.py flash` direct, puis restauration
factory.** L'exception précédente (écrire seulement le slot OTA et `otadata`)
est abandonnée au profit d'un cycle plus court : `screen-lcd-test` est un
firmware jetable, sans CAN ni NVS applicative ni credentials CoffeeFlow, donc
`idf.py flash` complet dessus est sans risque pour la machine — il n'y a rien
d'utile à préserver sur la carte pendant cette phase de diagnostic. Cela évite
les cycles OTA USB↔CAN de ~8 minutes à chaque itération.

Séquence :

1. `idf.py flash` avec `screen-lcd-test` (variante ordre suspect ou ordre
   inversé) directement sur la carte écran, autant d'itérations que
   nécessaire.
2. Une fois le diagnostic conclu, restaurer d'abord la table de partitions de
   production (`firmware/screen/build/partition_table/partition-table.bin`) à
   `0x8000`, puis l'image factory figée
   (`firmware/factory-images/2026-09-09-451566c/screen-factory.bin`, `v0.2.9`)
   à l'offset `0x20000`, par `esptool.py write_flash` — jamais `idf.py flash`
   pour cette étape. La table de `screen-lcd-test` diffère de celle de
   `screen` (`factory` agrandie à 1408 KiB pour loger le binaire de
   diagnostic, contre 1000 KiB en production) : ne pas supposer qu'elle
   correspond déjà, toujours réécrire la table de production avant l'image
   factory.
3. Sélectionner explicitement `factory` au boot (`otadata` ne pointe pas
   dessus par défaut après un usage OTA normal) puis vérifier PING/PONG par le
   pont USB↔CAN de l'écran factory, comme décrit dans le README de l'archive.
4. Reprendre l'OTA normal du lot 4 (image `screen` corrigée à partir de ce
   qui a été appris avec `screen-lcd-test`) vers le slot OTA inactif, selon la
   procédure habituelle de livraison.

Cette voie n'est légitime que parce que `screen-lcd-test` ne contient aucun
état à préserver ; elle ne s'applique pas à l'image fonctionnelle `screen`.

La console USB native (`/dev/cu.usbmodem5B790235091`) a montré le boot ROM puis
le passage au flux COBS du pont ; elle ne donne donc pas de logs texte après
`app_main`. Le port du pont testé est `/dev/cu.wchusbserial5B790235091`.

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

Exception de banc documentée au lot 4 : `screen-lcd-test`, firmware jetable
sans CAN ni NVS applicative, peut être écrit par `idf.py flash` complet pour
un cycle d'itération court, puis la carte est restaurée en réécrivant l'image
`factory` archivée à `0x20000` (voir plus haut, section lot 4). Cette
exception ne s'applique qu'à ce firmware de diagnostic et ne doit jamais être
utilisée sur l'image fonctionnelle `screen`, qui reste livrée exclusivement
par OTA vers un slot inactif.

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

Le découpage précis et les critères de sortie des lots restants sont dans
`docs/plan-phase6.md`. Ordre prévu :

1. Wi-Fi, provisioning et face configuration du coeur (lot 4, clos
   2026-09-10) : les calibrations sont désormais versionnées dans l'image
   écran via `core/calibration_machine.h`, plutôt que stockées en NVS. Le
   protocole de calibration est un outil hôte qui modifie ce fichier puis
   flashe l'image.
2. HTTP, puis WebSocket miroir du trafic CAN au même format que le pont USB
   (lots 5 et 6, prochaine étape).
3. OTA par le réseau pour l'écran et les capteurs, puis client BLE GATT Acaia
   Lunar (lots 7 et 8).
4. Face actions du coeur : infusion et purge sans écran, puis UI LVGL complète
   (lots 9 et 10). LVGL et HTTP restent des clients sans accès direct aux
   sorties.
5. Geler la table de partitions et fermer la phase (lot 11).

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

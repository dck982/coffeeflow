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

## Historique compact des phases terminées

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

Identifier les ports avec `esptool.py --port <port> flash-id` : le XIAO a 8 Mo
de flash et l'écran 16 Mo. Flasher en USB :

```sh
idf.py -p /dev/cu.usbmodemXXXX flash
```

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

1. Wi-Fi, provisioning et face configuration du coeur (lot 4).
2. HTTP, puis WebSocket miroir du trafic CAN au même format que le pont USB
   (lots 5 et 6).
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

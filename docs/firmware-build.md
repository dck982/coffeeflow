# Firmware — build, flash et banc

Conception, protocole, sécurité et mise à jour : `firmware.md`. Ce fichier
tient ce qu'il faut pour construire, flasher et reprendre le banc, ainsi que
les pièges matériels déjà payés une fois.

## Environnement

Chaque nouveau terminal doit charger ESP-IDF 6.1 :

```sh
source ~/.espressif/tools/activate_idf_v6.1.sh
```

Construire depuis le répertoire du projet concerné (`firmware/screen`,
`firmware/sensors`…) :

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

Tests hôte, sans matériel :

```sh
firmware/common/test/run_tests.sh   # protocole commun, C++
firmware/tools/test/run_tests.sh    # outils Python, régénère d'abord log_codes.py
```

La table des codes `LOG` est générée depuis une source unique,
`firmware/common/codegen/log_codes.yaml` ; ne jamais éditer les fichiers
générés.

## Flash

Identifier les ports avec `esptool.py --port <port> flash-id` : le XIAO a 8 Mo
de flash et l'écran 16 Mo.

**Le flash USB direct (`idf.py flash`) est réservé à l'écriture ou à la
récupération de l'image `factory`.** Il réécrit la table de partitions, NVS
et/ou `factory` selon la configuration locale. En exploitation, les deux
cartes sont livrées par OTA, soit par HTTP en mode Wi-Fi (voir « Mise à
jour » dans `firmware.md`), soit par le pont USB↔CAN de l'écran :

```sh
cd firmware/tools
python -m coffeetool.cli flash \
  --port /dev/cu.wchusbserialXXXX \
  --dest screen ../screen/build/screen.bin
```

Avec le rythme CAN volontairement limité à 2 ms par trame, l'image écran
d'environ 1,2 Mo prend environ huit minutes (632 blocs de 2 ko). Ne pas
interrompre le pont ni l'alimentation pendant ce transfert. La confirmation
finale puis le PING/PONG au redémarrage valident l'image `PENDING_VERIFY` ;
en l'absence de validation, l'image précédente revient.

### Images factory

Les images de secours des deux cartes sont figées au commit `451566c`
(`v0.2.9`) et archivées, avec leurs SHA-256 et leur procédure, sous
`firmware/factory-images/2026-09-09-451566c/`. Elles sont à l'offset
`0x20000` de leurs partitions `factory`. L'écran factory est le pont USB↔CAN :
il sait reflasher l'écran localement et le XIAO à travers le CAN. Son écran
noir est normal, puisque cette image ne porte pas LVGL.

### Banc LCD jetable

`firmware/screen-lcd-test` (sans CAN ni NVS applicative) peut être écrit par
`idf.py flash` complet. Sa table de partitions n'est pas celle de production
(`factory` 1408 KiB contre 1000 KiB). Après usage, restaurer la table et la
factory de l'écran par `esptool.py`, jamais par `idf.py flash` :

```sh
esptool.py --chip esp32s3 --port /dev/cu.wchusbserialXXXX write_flash \
  0x8000 firmware/screen/build/partition_table/partition-table.bin \
  0x20000 firmware/factory-images/2026-09-09-451566c/screen-factory.bin
esptool.py --chip esp32s3 --port /dev/cu.wchusbserialXXXX \
  erase_region 0x10000 0x2000
```

Le second ordre efface seulement `otadata` : le bootloader choisit alors la
partition `factory`, d'où l'on relivre l'image fonctionnelle par OTA. Le même
effacement de `otadata` sert de récupération quand une image OTA redémarre en
boucle sans que le rollback reprenne le slot précédent.

### Outils hôte

Les outils de protocole sont sous `firmware/tools/coffeetool` et s'exécutent
avec `python -m coffeetool.cli ...` (`monitor`, `send`, `record`, `replay`,
`flash`). Un seul processus peut ouvrir un port série à la fois ; le moniteur
peut afficher les trames avec retard lorsqu'il rattrape un buffer important.
`coffeetool monitor --ws … --token …` (ou `COFFEEFLOW_HTTP_TOKEN`) lit le
miroir WebSocket en mode Wi-Fi avec le même décodeur.

## Reprise du banc

1. Vérifier `CAN OK`, pression/température et débit au repos sur l'écran.
2. Vérifier les compteurs TWAI et un PING/PONG via `coffeetool` avant tout test
   applicatif.
3. Faire les essais SSR et dimmer sous 230 V avec une charge de test adaptée
   et les précautions décrites dans `firmware.md`.
4. Ne pas confondre validité du XDB401 et présence du XIAO : le premier enlève
   seulement les mesures de pression/température ; le second produit `CAN
   PERDU` après environ 3 s.

## Pièges connus

### Écran

- **Registre de sortie du CH422G.** Il est partagé entre `CAN_SEL`/`USB_SEL`
  (EXIO5), `LCD_BL`, `LCD_RST`, `TP_RST` et `SD_CS`. Il est maintenu en RAM et
  modifié uniquement par `ch422g_set_bit()` : une écriture directe (celle de
  l'exemple Waveshare par exemple) écrase les autres bits et coupe le
  transceiver CAN au moment d'allumer la dalle.
- **Timings du panneau RGB.** HSYNC `48/88/40`, VSYNC `3/32/13`, PCLK 16 MHz.
  À 12 MHz, la dalle boucle sur des mires de couleur.
- **Affinité des cœurs.** LCD et LVGL sur le cœur 1, CAN, UART, pont, Wi-Fi et
  BLE sur le cœur 0. C'est une contrainte, pas une optimisation : ailleurs,
  l'image saute. Ne pas activer `CONFIG_LCD_RGB_RESTART_IN_VSYNC`, qui
  provoquait aussi des sauts. Le décalage d'image au boot est suivi dans
  `screen-issue.md`.
- **Ordre d'initialisation.** `service_screen::init()` (cœur 1) avant
  `net_wifi::init()` (cœur 0) : les bounce buffers DMA du panneau doivent
  être alloués avant la pile Wi-Fi, sinon
  `lcd_rgb_panel_alloc_frame_buffers: no mem for bounce buffer` et reboot en
  boucle (image `v0.2.22`).
- **DIRAM.** Lier le contrôleur BLE coûte environ 18 Kio d'IRAM, même sans
  l'initialiser ; le second bounce buffer de 64 000 octets échouait alors dans
  `esp_lcd_new_rgb_panel()`. D'où `CONFIG_ESP_WIFI_IRAM_OPT=n`,
  `CONFIG_ESP_WIFI_RX_IRAM_OPT=n` et `CONFIG_BT_NIMBLE_LOW_SPEED_MODE=y`. Un
  `ESP_ERROR_CHECK` qui échoue avec la console texte désactivée ne laisse
  qu'un `Saved PC` sur `esp_cpu_wait_for_intr()` de l'autre cœur : chercher
  la cause dans les trames `LCD_INIT_STEP` du pont.
- **Polices LVGL.** `LV_USE_FONT_COMPRESSED` doit rester activé : sans lui,
  les étiquettes sont invisibles alors que les formes s'affichent. Le signe
  moins Unicode n'est pas dans les polices : utiliser le tiret ASCII.

### Radio

- Wi-Fi et BLE sont exclusifs (voir « Politique radio » dans `firmware.md`) :
  `BLE_INIT: Malloc failed` signale qu'ils ont été chargés ensemble.
- Une Lunar peut répartir l'UUID de service et le nom entre l'advertising et
  la scan response : le scan ne doit pas dédupliquer les paquets.
- Annuler le scan avant `ble_gap_connect()`, sinon la séquence
  `BLE_SCALE_FOUND` / `BLE_SCAN_STARTED` se répète. Passer `nullptr` pour les
  paramètres de connexion par défaut.
- La confidentialité NimBLE reste désactivée : inutile au client central, elle
  produisait des avertissements IRK `rc=8`.

### Module capteurs

- **UART.** Une entrée restée sur sa fonction IOMUX ne reçoit rien : forcer le
  pad en GPIO et activer son entrée après `uart_set_pin()`. Les lectures UART
  emploient des délais courts, jamais `portMAX_DELAY`.
- **XDB401.** Lire pression et température en deux transactions séparées ;
  une lecture groupée donne des pressions incohérentes. Appliquer la formule
  du fabricant à l'ordre big-endian des registres, même si le PDU est
  little-endian. Ne pas garder le mutex I2C pendant les 50 ms de conversion.
- **Pull-ups I2C.** Elles sont dans le XDB401 : le débrancher peut aussi
  rendre le dimmer muet.
- **Digmesa.** GPIO44 (D7) compte les fronts descendants, pull-up interne
  désactivé, le filtre RC assurant la polarisation. C'est aussi l'entrée
  UART0 par défaut : le forcer en GPIO. Un comptage nul persistant venait
  d'un câble inversant VCC et SIGNAL (câblage final dans `cablage.md`).
- **Dimmer.** Il lui faut le secteur pour devenir prêt : sans 230 V, son
  erreur de calibration est normale. Les flags viennent de `STATUS`
  (READY/ERROR), pas du seul registre `ERROR` ; `AC_FREQ` est informatif. Ne
  pas envoyer `RECALIBRATE` au boot. Une inversion phase/neutre sur son
  bornier a déjà été corrigée une fois. Registres : `dimmerlink-i2c.md`.

### OTA

- Un rollback n'est validé qu'avec une image réellement cassée.
- Après une erreur CRC de bloc côté `sensors`, une retransmission n'est pas
  distinguée explicitement d'un bloc suivant. Le CRC32 final protège l'image ;
  corriger ce point avant tout transport moins fiable qu'un CAN de banc.
- Le `PONG` est l'identité et la version observables d'une carte en boîte.

## Observations ouvertes

- **L2 « module interne injoignable » après un flash** (2026-09-11). Un
  redémarrage de l'écran ou des capteurs l'a levé. Un démarrage propre montre
  PING, `PRESENCE_RESTORED`, PONG et les trois `STATUS_*` avant le premier
  `LCD_INIT_STEP`. Si le cas revient, conserver le log `coffeetool monitor`
  depuis le boot : présence non relancée, trames perdues, ou rendu L2 figé
  malgré une présence restaurée.
- **Perte de synchronisation RGB pendant un flash HTTP** (2026-09-11). Elle
  revient parfois seulement au rendu du L2 « mise à jour ». Piste : latence
  entre l'ISR LCD (cœur 1) et la pile Wi-Fi/httpd (cœur 0).

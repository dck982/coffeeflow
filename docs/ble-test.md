# Diagnostic BLE Acaia sur AtomS3

## But

Créer un firmware ESP-IDF autonome pour un **M5Stack AtomS3** qui écrit son
diagnostic directement sur la console USB. Il doit permettre d'itérer rapidement
sur la découverte BLE de la balance Acaia Lunar sans reflasher l'écran
CoffeeFlow ni faire passer les détails GATT dans les petites trames CAN.

Ce firmware est jetable. Il peut remplacer temporairement `can-monitor` sur un
AtomS3 ; à la fin des essais, il suffit de reflasher `firmware/can-monitor`.

Le premier objectif est uniquement d'observer fidèlement la topologie BLE :

1. lister les annonces uniques reçues ;
2. sélectionner la Lunar par nom ou UUID annoncé ;
3. s'y connecter ;
4. lister tous ses services, caractéristiques et descripteurs ;
5. afficher les handles, UUID et propriétés sans filtrer selon nos hypothèses.

Ne pas envoyer de commande Acaia et ne pas écrire de CCCD dans la première
version. On ajoutera souscription et protocole applicatif après avoir obtenu le
premier inventaire GATT.

## Contexte du défaut actuel

Le firmware écran `v0.2.37` produit cette séquence :

```text
BLE_SCAN_STARTED
BLE_SCALE_FOUND arg16=65471       # RSSI signé : -65 dBm
BLE_CONNECTED
BLE_ERROR arg16=14
BLE_DISCONNECTED
BLE_SCAN_STARTED
```

Dans NimBLE, `14` est `BLE_HS_EDONE`. Ce n'est pas une panne radio : la
connexion GAP réussit, puis une procédure de découverte GATT arrive à sa fin
sans que le firmware ait accepté l'attribut recherché.

Le défaut le plus probable est dans
`firmware/screen/main/ble_scale.cpp` : le code cherche actuellement
`49535343-8841-43f4-a8d4-ecbe34729bb3`, puis exige sur cette même
caractéristique les propriétés écriture **et** notification. Les
implémentations Acaia communautaires indiquent deux caractéristiques pour les
Lunar récentes :

- écriture : `49535343-8841-43f4-a8d4-ecbe34729bb3` ;
- notification : `49535343-1e4d-4bd9-ba61-23c647249616`.

Pour une Lunar pré-2021, lecture et écriture peuvent à la place utiliser la
caractéristique courte `0x2A80`. Le firmware de diagnostic ne doit donc pas
filtrer la découverte avant de l'avoir imprimée.

Références communautaires, le protocole Acaia n'étant pas publié officiellement :

- <https://github.com/beat843796/acaia-lunar-ble>
- <https://github.com/tatemazer/AcaiaArduinoBLE>
- `docs/reference/acaia-ble/README.md`

## Pourquoi utiliser l'AtomS3

L'AtomS3 et l'écran sont tous deux basés sur un ESP32-S3 et utilisent le même
hôte NimBLE ESP-IDF 6.1. Pour découvrir la base GATT de la Lunar, les
particularités LCD, PSRAM, CAN et CH422G de l'écran ne sont pas pertinentes.

L'AtomS3 fournit en outre une console USB Serial/JTAG native. Il se reflashe en
quelques secondes et évite les contraintes de rollback et de partition OTA de
l'écran. Ne pas utiliser le XIAO capteurs en premier choix : il convient aussi,
mais l'AtomS3 dispose déjà dans ce dépôt d'une configuration ESP-IDF et d'un
chemin USB connus (`firmware/can-monitor`).

## Projet à créer

Créer un projet indépendant :

```text
firmware/ble-debug/
  CMakeLists.txt
  sdkconfig.defaults
  main/
    CMakeLists.txt
    main.cpp
```

Ne modifier ni `firmware/screen`, ni `firmware/can-monitor`. Ne lire ou copier
aucun secret de l'écran.

Le `CMakeLists.txt` racine peut reprendre la structure minimale des autres
projets :

```cmake
cmake_minimum_required(VERSION 3.16)
include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(ble-debug)
```

Le composant principal ne nécessite que NimBLE, NVS et FreeRTOS :

```cmake
idf_component_register(
  SRCS "main.cpp"
  INCLUDE_DIRS "."
  REQUIRES bt nvs_flash freertos
)
```

## Configuration ESP-IDF

Le fichier `sdkconfig.defaults` doit au minimum contenir :

```text
CONFIG_IDF_TARGET="esp32s3"
CONFIG_ESPTOOLPY_FLASHSIZE_8MB=y

CONFIG_BT_ENABLED=y
CONFIG_BT_BLUEDROID_ENABLED=n
CONFIG_BT_NIMBLE_ENABLED=y
CONFIG_BT_NIMBLE_ROLE_CENTRAL=y
CONFIG_BT_NIMBLE_ROLE_OBSERVER=y
CONFIG_BT_NIMBLE_ROLE_PERIPHERAL=n
CONFIG_BT_NIMBLE_ROLE_BROADCASTER=n
CONFIG_BT_NIMBLE_GATT_CLIENT=y
CONFIG_BT_NIMBLE_GATT_SERVER=n
CONFIG_BT_NIMBLE_SECURITY_ENABLE=n
CONFIG_BT_NIMBLE_HS_PVCY=n
CONFIG_BT_NIMBLE_MAX_CONNECTIONS=1

CONFIG_ESP_CONSOLE_USB_SERIAL_JTAG=y
CONFIG_ESP_CONSOLE_UART_DEFAULT=n
CONFIG_ESP_CONSOLE_SECONDARY_NONE=y

CONFIG_LOG_DEFAULT_LEVEL_DEBUG=y
CONFIG_LOG_MAXIMUM_LEVEL_DEBUG=y
```

Si Kconfig normalise certaines lignes lors du premier `idf.py reconfigure`,
conserver la sélection USB Serial/JTAG et vérifier dans `sdkconfig` que
`CONFIG_ESP_CONSOLE_USB_SERIAL_JTAG=y` est bien effectif.

## Comportement demandé

### Initialisation

Dans `app_main()` :

1. initialiser NVS avec `nvs_flash_init()` ;
2. attendre environ une seconde pour laisser la console USB se réénumérer ;
3. afficher un bandeau clair avec version ESP-IDF et raison du reset ;
4. appeler `nimble_port_init()` ;
5. installer `ble_hs_cfg.reset_cb` et `ble_hs_cfg.sync_cb` ;
6. démarrer l'hôte avec `nimble_port_freertos_init()`.

Après synchronisation NimBLE, appeler `ble_hs_util_ensure_addr(0)`, puis lancer
un scan actif et permanent avec `ble_gap_disc()`.

Utiliser `nullptr` pour les paramètres de connexion de `ble_gap_connect()` :
une `ble_gap_conn_params{}` remplie de zéros contient des valeurs HCI invalides.

### Annonces BLE

Configurer le scan actif :

```cpp
ble_gap_disc_params params{};
params.passive = 0;
params.filter_duplicates = 0;
```

Le filtrage contrôleur reste désactivé pour recevoir séparément advertising et
scan response. Afin de ne pas saturer la console, dédupliquer dans
l'application sur le triplet suivant :

```text
(adresse, event_type, hash du payload brut)
```

Pour chaque paquet jamais vu, imprimer :

- temps depuis le boot ;
- adresse et type d'adresse ;
- `event_type` (`ADV_IND`, `SCAN_RSP`, etc.) ;
- RSSI ;
- nom local complet ou abrégé ;
- listes des UUID 16, 32 et 128 bits ;
- payload publicitaire brut en hexadécimal.

Utiliser `ble_hs_adv_parse_fields()` et `ble_uuid_to_str()`. Toujours afficher
le payload brut même si son parsing échoue.

Détecter la cible si le nom contient `ACAIA` ou `LUNAR`, sans tenir compte de
la casse, ou si l'annonce contient le service connu :

```text
49535343-fe7d-4ae5-8fa9-9fafd205e455
```

Au premier match :

1. imprimer la raison du match (`name` ou `service UUID`) ;
2. copier l'adresse ;
3. appeler `ble_gap_disc_cancel()` ;
4. si l'annulation réussit, appeler immédiatement `ble_gap_connect()`.

Ne pas attendre `BLE_GAP_EVENT_DISC_COMPLETE`. Dans NimBLE ESP-IDF 6.1,
l'annulation retire l'état et le callback du scan ; cet événement n'est pas
remis à ce callback après une annulation explicite.

### Connexion et inventaire GATT

Sur `BLE_GAP_EVENT_CONNECT`, imprimer le status et le handle. En cas de succès,
lancer `ble_gattc_disc_all_svcs()` : surtout pas une découverte filtrée par
UUID.

Conserver les services dans un tableau borné, par exemple 32 entrées :

```cpp
struct ServiceRecord {
  uint16_t start_handle;
  uint16_t end_handle;
  ble_uuid_any_t uuid;
};
```

Pour chaque service, imprimer :

```text
SERVICE start=0x.... end=0x.... uuid=...
```

Lorsque le callback reçoit `BLE_HS_EDONE`, parcourir les services un par un
avec `ble_gattc_disc_all_chrs()`. Ne jamais lancer plusieurs procédures GATT
en parallèle sur la même connexion.

Conserver au moins 64 caractéristiques :

```cpp
struct CharacteristicRecord {
  uint16_t service_index;
  uint16_t def_handle;
  uint16_t value_handle;
  uint16_t end_handle;
  uint8_t properties;
  ble_uuid_any_t uuid;
};
```

Pour chaque caractéristique, imprimer son UUID et décoder les propriétés :

```text
CHR service=N def=0x.... value=0x.... props=0x.. [READ WRITE WRITE_NR NOTIFY INDICATE] uuid=...
```

Calculer la fin de sa plage de descripteurs avec le `def_handle` de la
caractéristique suivante moins un ; pour la dernière caractéristique du
service, utiliser le `end_handle` du service.

Après la fin de toutes les découvertes de caractéristiques, parcourir les
caractéristiques une par une avec `ble_gattc_disc_all_dscs()`. Imprimer :

```text
DSC char_value=0x.... handle=0x.... uuid=...
```

Le CCCD attendu porte l'UUID `0x2902`. Dans cette première version, seulement
le signaler clairement, sans l'écrire.

À la fin, imprimer un résumé comprenant :

- nombre de services, caractéristiques et descripteurs ;
- présence et propriétés de `…8841…` ;
- présence et propriétés de `…1e4d…` ;
- présence et propriétés de `0x2A80` ;
- caractéristique(s) `NOTIFY` et CCCD associé ;
- caractéristique(s) `WRITE` / `WRITE_NO_RSP`.

En cas d'erreur, toujours imprimer l'étape, le status NimBLE, le handle ATT et
sa signification au moins pour :

```text
14 = BLE_HS_EDONE
15 = BLE_HS_EBUSY
```

`BLE_HS_EDONE` doit être traité comme la fin normale d'une procédure ; il ne
devient une erreur fonctionnelle que si aucun élément attendu n'a été trouvé.

Sur `BLE_GAP_EVENT_DISCONNECT`, imprimer la raison, vider tous les tableaux et
relancer le scan.

### Robustesse du diagnostic

- Copier tout UUID conservé avec `ble_uuid_copy()` ; les structures fournies
  aux callbacks ne restent pas valides après leur retour.
- Borner tous les tableaux et signaler explicitement un dépassement.
- Ne pas utiliser d'allocation dynamique dans les callbacks.
- Ne pas appeler de procédure de découverte suivante pour chaque résultat :
  attendre le `BLE_HS_EDONE` de la procédure courante.
- Afficher la valeur de retour immédiate de chaque API NimBLE ainsi que le
  status asynchrone du callback.
- Ne jamais interpréter l'`I2C_ERROR` du firmware capteurs comme un problème
  BLE ; il est indépendant et n'existera pas sur ce firmware AtomS3.

## Build, flash et console

Identifier d'abord le port USB natif de l'AtomS3 :

```sh
ls /dev/cu.usbmodem*
```

Puis construire et flasher avec ESP-IDF 6.1 :

```sh
cd /Users/davidclerc/Projects/coffeeflow/firmware/ble-debug
source /Users/davidclerc/.espressif/tools/activate_idf_v6.1.sh
idf.py build
idf.py -p /dev/cu.usbmodemXXXX flash monitor
```

Quitter le moniteur avec `Ctrl-]`. Si le port disparaît pendant le reset,
relancer seulement :

```sh
idf.py -p /dev/cu.usbmodemXXXX monitor
```

Ce flash complet remplace temporairement le contenu de l'AtomS3. Il ne touche
ni l'écran ni le XIAO capteurs.

## Données à rapporter après le premier essai

Conserver la sortie complète depuis le bandeau de démarrage jusqu'au résumé
GATT. Le minimum indispensable est :

1. les deux paquets `ADV_IND` et `SCAN_RSP` de la Lunar ;
2. la ligne indiquant pourquoi la cible a été reconnue ;
3. tous les `SERVICE`, `CHR` et `DSC` ;
4. le résumé ;
5. toute ligne d'erreur ou de déconnexion.

À partir de cet inventaire, la deuxième itération pourra :

1. choisir la bonne caractéristique de notification ;
2. écrire `{0x01, 0x00}` dans son CCCD `0x2902` ;
3. écrire les commandes Acaia sur la caractéristique d'écriture ;
4. afficher les notifications brutes ;
5. seulement ensuite reporter la topologie confirmée dans
   `firmware/screen/main/ble_scale.cpp`.

## Restauration de l'AtomS3

Pour remettre le pont CAN après les essais :

```sh
cd /Users/davidclerc/Projects/coffeeflow/firmware/can-monitor
source /Users/davidclerc/.espressif/tools/activate_idf_v6.1.sh
idf.py build
idf.py -p /dev/cu.usbmodemXXXX flash
```

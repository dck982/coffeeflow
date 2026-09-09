# Client BLE Acaia — code de référence

`blescale.h` / `blescale.cpp` : implémentation à la main, écrite dans un
projet antérieur (`coffeetracker.ino`, Arduino-ESP32), d'un client BLE GATT
pour une balance Acaia (Lunar). Déplacé ici depuis `tmp/ble-sample/` pour
servir de **référence de protocole** au moment d'écrire le client BLE de
`firmware/screen` (phase 6, voir `docs/firmware-implementation.md`).

**Ce code ne compile pas tel quel dans l'arbre `firmware/`.** Il est écrit
contre la pile Arduino-ESP32 (`BLEDevice.h`, `BLEAdvertisedDevice.h`,
`loop()` Arduino), alors que `firmware.md` a tranché pour ESP-IDF 5.x/6.x et
ses tâches FreeRTOS (le client BLE de l'écran utilisera donc l'API GATT
native d'ESP-IDF — `esp_gattc_api.h` / NimBLE — pas Arduino-ESP32, même si
Arduino-ESP32 reste une option de secours "composant IDF" citée dans
`firmware.md`). Sa valeur est dans **la logique du protocole Acaia**, pas
dans le code BLE bas niveau qui l'entoure : découverte du service, cadrage
des messages, décodage poids/temps/boutons, et l'émission périodique
(heartbeat + demande de notifications) sans laquelle la balance arrête
d'envoyer.

## Ce que fait ce code

- **Scan** — recherche un périphérique annonçant le service GATT de la
  balance (UUID passé au constructeur `BLEScale`), connexion, découverte du
  service/de la caractéristique, activation des notifications.
- **Cadrage des trames entrantes** (`provideData` accumule dans `msgBuf`,
  `loop()` le consomme) : préfixe `0xEF 0xDD`, puis `cmd` (1 octet),
  `payloadLen` (1 octet), `payload`, cas particulier `cmd == 0x20` = mise
  hors tension de la balance.
- **Notification `cmd == 12`** (`processNotification`) : le payload est
  lui-même une suite de sous-messages `[type][data...]`, démultiplexés par
  `processMessage` :
  - `type 5`, ≥6 octets : poids. `decodeWeight` — entier 32 bits
    little-endian sur 4 octets, `msg[4]` sélectionne le diviseur
    (10/100/1000/10000, donc le nombre de décimales), `msg[5] & 2` est le
    signe.
  - `type 7`, ≥3 octets : temps du chronomètre balance. `decodeTime` —
    `minutes*6000 + secondes*100 + dixièmes*10` (unité 10 ms), ramené à 0 si
    ≤ 20 (bruit de démarrage).
  - `type 8` : bouton pressé sur la balance (`TARE`/`START`/`STOP`/`RESET`),
    codes bruts 0/8/9/10.
- **Notification `cmd == 8`** (`processSettings`) : bit0 de `payload[0]` =
  état du chronomètre (marche/arrêt) ; `payload[2]` encode le mode de la
  balance (poids seul, poids+temps, débit, débit+tare auto, etc.) — pas
  décodé plus loin ici.
- **Émission périodique obligatoire** (`loop()`, toutes les 2,5 s) :
  `send_heartbeat` (un ID fixe puis un heartbeat vide) et
  `send_request_notifications`. **Sans ce battement, la balance arrête
  d'émettre** — ce n'est pas une notification GATT passive une fois
  souscrite, il faut la relancer.
- **Cadrage des trames sortantes** (`send_fixedlen_payload` /
  `send_payload`) : même préfixe `0xEF 0xDD`, `cmd`, payload, et un checksum
  sur 2 octets (somme des octets pairs / impairs du payload) — pas un CRC
  standard, juste deux accumulateurs 8 bits.

## Ce qui ne sert à rien à réutiliser tel quel

- Toute la mécanique `BLEAdvertisedDeviceCallbacks` / `BLEClientCallbacks` /
  `BLEScan` : à réécrire avec l'API GATT native ESP-IDF.
- Le `g_scanOwner` global (pointeur C brut vers l'unique instance vivante,
  nécessaire parce que le callback de fin de scan Arduino-ESP32 est un
  pointeur de fonction C sans contexte utilisateur) : problème spécifique à
  cette pile, ne se pose sans doute pas avec l'API ESP-IDF si elle permet de
  passer un contexte (`void *arg`) au callback GATT.
- `Serial.print` partout : à remplacer par `ESP_LOGx`, ou par un `LOG` du
  protocole CAN si l'info doit remonter jusqu'au Mac (voir la convention
  déjà en place dans `docs/firmware.md`).

## À vérifier avant de coder le client définitif

- **UUID exacts du service/caractéristique Acaia** : passés en paramètre du
  constructeur dans ce code, pas codés en dur ici — à retrouver (capture
  BLE d'une vraie Lunar, ou doc communautaire type `pyacaia`/`acaia_ble`)
  avant d'écrire le client ESP-IDF.
- **`msg[5] & 2` pour le signe du poids** : bit isolé, pas documenté au-delà
  de ce que ce code fait — à confirmer par test réel (poids négatif après
  tare, par exemple) plutôt que supposé correct par lecture du code seul.
- Le mode de balance (`payload[2]` dans `processSettings`) et le décodage
  des boutons ne sont pas exploités par l'algorithme d'infusion prévu
  (`firmware.md` : poids + un décalage d'anticipation) — à garder en tête
  mais pas nécessairement à porter en premier.

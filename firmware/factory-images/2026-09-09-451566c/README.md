# Images factory — 2026-09-09

Source : commit [`451566c980ccb6935284ebad8b9b5a400a2db413`](../../../../.git/)
(`firmware phase 6, lot 1 : restructuration de screen/, vérifiée sur matériel`).
Version PONG : `0.2.9`. ESP-IDF : `6.1.0`.

Ces images sont le filet de récupération immuable, pas l'application
fonctionnelle :

- `screen-factory.bin` : pont UART USB ↔ CAN, PING/PONG et réception OTA
  locale de l'écran. Les trames destinées aux capteurs sont relayées sur CAN,
  ce qui permet de reflasher les deux cartes avec le seul USB de l'écran.
- `sensors-factory.bin` : CAN, PING/PONG, OTA et repli sûr des sorties.

Elles s'écrivent **uniquement** à l'offset `0x20000`, dans la partition
`factory`, avec la table de partitions correspondante. Ne jamais utiliser
`idf.py flash` avec l'application fonctionnelle : elle va à une partition OTA
et ne tient de toute façon pas dans factory.

| Fichier | SHA-256 |
| --- | --- |
| `screen-factory.bin` | `fb05203840294191584dfc962c79c965a151beace5699eac8de12299111fa4dd` |
| `sensors-factory.bin` | `9e0bbd94e9730a5df153b5f5eb24f17427ef0d293a7aa1e7fb517445f52ab929` |

## Écriture initiale

Écrites le 2026-09-10, avec vérification par hash après écriture :

| Cible | Port USB | MAC | Offset |
| --- | --- | --- | --- |
| XIAO capteurs, flash 8 Mo | `/dev/cu.usbmodem1101` | `a4:cb:8f:d2:67:64` | `0x20000` |
| Waveshare écran, flash 16 Mo | `/dev/cu.usbmodem5B790235091` | `44:1b:f6:ca:6a:58` | `0x20000` |

L'écriture ne modifie ni `otadata`, ni NVS, ni les partitions OTA : elle ne
fait donc pas démarrer factory immédiatement. Pour éprouver un jour le chemin
de récupération, sélectionner factory explicitement, vérifier PING/PONG par
l'USB de l'écran, puis reflasher une image OTA fonctionnelle avant de remettre
la machine en service.

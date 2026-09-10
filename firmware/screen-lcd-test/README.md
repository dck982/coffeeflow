# Diagnostic LCD / radio

Ce firmware réduit isole le démarrage de la phase 6 : CH422G, panneau RGB,
LVGL, point d'accès Wi-Fi / `httpd` minimal et hôte NimBLE. Il ne touche ni au
CAN, ni au pont, ni aux credentials ou réglages NVS CoffeeFlow.

L'image par défaut teste la séquence de production réduite :
`CH422G -> LCD/LVGL -> Wi-Fi/HTTP -> NimBLE`. La mire affiche la séquence qui
a été compilée et les logs UART annoncent chaque étape.

Dans `idf.py menuconfig`, `LCD / radio diagnostic > Sequence de demarrage
testee` sélectionne l'une des cinq images : LCD seul, LCD+Wi-Fi,
LCD+NimBLE, LCD+Wi-Fi+NimBLE, ou une alternance Wi-Fi/NimBLE toutes les cinq
secondes. Cette dernière déinitialise réellement chaque pile, contrôleur et
httpd inclus, avant d'initialiser l'autre. La première variante, sans modifier le
`sdkconfig`, se construit dans un répertoire distinct :

```sh
idf.py -B build-lcd-only -D SDKCONFIG_DEFAULTS='sdkconfig.defaults;sdkconfig.lcd-first.defaults' build
```

L'AP ouvert temporaire est `CoffeeFlow-LCD-Diag`; `GET /` retourne un statut.
Il sert seulement à faire allouer les piles réseau.

```sh
source ~/.espressif/tools/activate_idf_v6.1.sh
idf.py build
```

Ce firmware jetable se flashe directement par `idf.py -p <port> flash` sur le
banc — cycle court, sans passer par le CAN. Sa table de partitions n'est pas
celle de production : une fois le diagnostic terminé, restaurer la table de
partitions production à `0x8000`, puis l'image `factory` à `0x20000`, avant de
reprendre l'OTA normal, selon `docs/firmware-implementation.md`.

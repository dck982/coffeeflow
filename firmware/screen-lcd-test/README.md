# Diagnostic LCD/Wi-Fi

Ce firmware réduit isole l'incident du lot 4 : CH422G, point d'accès Wi-Fi et
`httpd` minimal, puis panneau RGB, LVGL et mire six couleurs. Il ne touche ni
au CAN, ni au pont, ni aux credentials ou réglages NVS CoffeeFlow.

L'image par défaut teste `CH422G -> Wi-Fi + HTTP -> LCD + LVGL`. Dans
`idf.py menuconfig`, `LCD/Wi-Fi diagnostic > Ordre d'initialisation teste`
permet de produire l'image témoin inverse. La mire indique l'ordre compilé.
Sans modifier `sdkconfig`, la variante inverse se construit dans un répertoire
distinct avec `idf.py -B build-lcd-first -D
SDKCONFIG_DEFAULTS='sdkconfig.defaults;sdkconfig.lcd-first.defaults' build`.

L'AP ouvert temporaire est `CoffeeFlow-LCD-Diag`; `GET /` retourne un statut.
Il sert seulement à faire allouer les piles réseau.

```sh
source ~/.espressif/tools/activate_idf_v6.1.sh
idf.py build
```

Ce firmware jetable se flashe directement par `idf.py -p <port> flash` sur le
banc — cycle court, sans passer par le CAN. Une fois le diagnostic terminé,
restaurer l'image `factory` archivée avant de reprendre l'OTA normal du lot 4,
selon la procédure de `docs/firmware-implementation.md`.

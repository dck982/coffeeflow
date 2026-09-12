# Captures UI LVGL

Ce simulateur hôte compile le même `ui_home.cpp` et les mêmes polices que
l'image écran, avec un coeur factice et `LV_USE_SNAPSHOT=1`. Il n'emploie pas
SDL : `lv_snapshot_take()` rend directement un framebuffer 800 × 480.

```sh
cmake -S firmware/screen/ui_sim -B firmware/screen/ui_sim/build
cmake --build firmware/screen/ui_sim/build -j 4
mkdir -p firmware/screen/ui_sim/build/snapshots
firmware/screen/ui_sim/build/ui_snapshot firmware/screen/ui_sim/build/snapshots/home.ppm
sips -s format png firmware/screen/ui_sim/build/snapshots/home.ppm --out firmware/screen/ui_sim/build/snapshots/home.png
```

Le format PPM est volontaire : il évite d'ajouter un encodeur PNG à LVGL.
`sips` est disponible sur macOS pour consultation ou intégration dans un test
visuel. `build/snapshots/` est ignoré par Git avec les autres artefacts de
build. Les scénarios disponibles sont l'accueil (défaut), `settings`,
`settings1`, `settings2`, `wifi-confirm`, `dim` et `standby`. Le simulateur ne
remplace pas un essai tactile ou RGB sur la dalle.

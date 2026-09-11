# Captures UI LVGL

Ce simulateur hôte compile le même `ui_home.cpp` et les mêmes polices que
l'image écran, avec un coeur factice et `LV_USE_SNAPSHOT=1`. Il n'emploie pas
SDL : `lv_snapshot_take()` rend directement un framebuffer 800 × 480.

```sh
cmake -S firmware/screen/ui_sim -B firmware/screen/ui_sim/build
cmake --build firmware/screen/ui_sim/build -j 4
firmware/screen/ui_sim/build/ui_snapshot /private/tmp/home.ppm
firmware/screen/ui_sim/build/ui_snapshot /private/tmp/settings.ppm settings
firmware/screen/ui_sim/build/ui_snapshot /private/tmp/settings-2.ppm settings2
sips -s format png /private/tmp/home.ppm --out /private/tmp/home.png
```

Le format PPM est volontaire : il évite d'ajouter un encodeur PNG à LVGL.
`sips` est disponible sur macOS pour consultation ou intégration dans un test
visuel. Le simulateur ne remplace pas un essai tactile ou RGB sur la dalle.

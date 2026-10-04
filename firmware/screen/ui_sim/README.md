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
build. Les scénarios disponibles sont l'accueil (défaut), `no-scale`, `settings`,
`settings1`, `settings2`, `settings3`, `diagnostic`, `diagnostic-errors`,
`diagnostic-states`, `heating-menu-on`, `heating-menu-off`, `valve-menu`,
`valve-menu-open`, `valve-menu-heating`, `wifi-confirm`,
`dim`, `standby`, `cold` (lecture sonde à l'ambiante) et `cold-capped`
(sonde entre 50 et 60,5 °C). L'écran d'infusion se capture avec
`brew-<t>` (infusion synthétique à `t` secondes, par exemple `brew-3`, `brew-15`,
`brew-30`, `brew-41`, `brew-60`) ou `replay=<csv>@<t>` ; `brew` et `preinfusion`
sont des alias de `brew-30` et `brew-10`. Voir `docs/ecran-infusion.md`. Le simulateur ne
remplace pas un essai tactile ou RGB sur la dalle. Le scénario `wifi-mode`
capture le plein écran du mode Wi-Fi.

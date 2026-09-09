# Firmware écran

## Labels LVGL

`lv_label_set_text()` invalide toujours le widget, même si la chaîne est
identique. Sur ce panneau RGB (framebuffer PSRAM, bounce buffer), chaque
invalidation dispute le bus à l’ISR DMA LCD — voir `docs/screen-issue.md`.

Ne l’appeler que si le texte a vraiment changé. Comparer à
`lv_label_get_text()` avant d’écrire (voir `set_label_if_changed()` dans
`main/service_screen.cpp`). Un `lv_label_set_text()` unique à la construction
d’un widget neuf n’a pas besoin de cette garde.

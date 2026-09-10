# Instabilité de synchronisation de l’écran

## Résultat final (2026-09-10, `v0.2.33`)

Le correctif `v0.2.20` a supprimé les sauts et décalages variables observés
pendant les rafraîchissements, mais la conclusion « écran parfaitement stable »
était trop forte. Un décalage fixe persistait : tout le contenu apparaissait
environ 150 pixels trop à droite et légèrement trop bas. Pour cliquer sur un
bouton affiché, il fallait toucher environ 150 pixels à sa gauche et un peu
plus haut.

Le test `v0.2.33` confirme que le GT911 renvoyait les coordonnées LVGL
correctes et que seule la trame RGB visible était décalée. Une reprise unique
du panneau avec `esp_lcd_rgb_panel_restart()`, une seconde après le premier
rendu LVGL, remet l'image en place. La cause est donc le démarrage
désynchronisé du flux RGB/DMA, pas les coordonnées tactiles ni un offset LVGL.

Suffisant, flashé et observé : LCD initialisé depuis le cœur 1 (ISR DMA
avec LVGL), pont TWAI/UART et validation OTA sur le cœur 0.

Ne pas appeler `lv_label_set_text()` si le texte est inchangé : bonne
pratique posée dans `service_screen.cpp` (`set_label_if_changed()`),
documentée dans `firmware/screen/AGENTS.md`.

PCLK resté à 16 MHz, CPU resté à 160 MHz. Les images 2 (240 MHz) et 3
(PCLK 14 puis 12 MHz) n’ont pas été nécessaires. `CONFIG_LCD_RGB_RESTART_IN_VSYNC`
reste désactivé.

## Diagnostic des sauts variables (confirmé)

Ce n’était ni un mauvais timing HSYNC/VSYNC, ni un manque global de CPU pour
traiter le CAN.

Le symptôme correspondait à une sous-alimentation ponctuelle du DMA LCD :

- image décalée horizontalement : le LCD continue d’avancer pendant que le DMA
  attend des données ;
- saut occasionnel : ESP-IDF détecte les bounce buffers manqués et
  resynchronise le DMA au VSYNC suivant.

La documentation ESP-IDF 6.1 décrit explicitement ce cas : Core 1 écrit le
framebuffer PSRAM pendant que l’ISR LCD sur Core 0 copie ce même framebuffer
vers les bounce buffers, ce qui peut provoquer un « screen shift ».

C’était précisément la configuration fautive :

- `app_main()` tourne sur Core 0 et appelait `service_screen::init()` :
  l’interruption LCD était donc installée sur Core 0 ;
- LVGL était épinglé sur Core 1 ;
- LVGL écrivait dans le framebuffer PSRAM pendant que Core 0 le lisait ;
- l’écran de service réécrivait six labels toutes les 200 ms, même lorsque
  leur texte était inchangé.

`screen-lcd-test` n’était pas une charge équivalente : il ne modifie qu’un
minuscule compteur. Il prouve les timings et le panneau, mais pas la marge de
bande passante du vrai écran.

## Correctif appliqué

### 1. LCD et LVGL sur le cœur 1 — fait

`service_screen::init()` lance une tâche temporaire épinglée sur le cœur 1,
qui appelle `esp_lcd_new_rgb_panel()`. L’interruption LCD est donc attachée
au même cœur que LVGL : elle préempte les écritures framebuffer au lieu que
les deux cœurs accèdent simultanément à la PSRAM.

TWAI/UART restent installés depuis `app_main` (cœur 0). `can2ser`, `ser2can`
et `ota_valid` sont épinglés sur le cœur 0. Répartition en vigueur :

- Core 0 : TWAI, UART, pont, plus tard Wi-Fi/BLE ;
- Core 1 : LCD, LVGL, contrôle machine.

### 2. Ne redessiner que lorsqu’une valeur change — fait

`set_label_if_changed()` compare à `lv_label_get_text()` et n’appelle
`lv_label_set_text()` que si la chaîne diffère. `tick_presence()` continue
de tourner toutes les 200 ms, indépendamment du dessin. La construction
initiale des labels (`build_ui`) écrit directement : le widget est neuf.

### 3. CPU à 240 MHz — non nécessaire

Le remplissage des bounce buffers est un `memcpy()` dans l’ISR ; 240 MHz
donnerait 50 % de marge de plus. Inutile une fois 1 et 2 en place.

### 4. PCLK à 12 ou 14 MHz — non nécessaire

Avec les porches actuels : 16 MHz ≈ 31 Hz, 14 MHz ≈ 27 Hz, 12 MHz ≈ 23 Hz.
16 MHz tient, une fois la contention PSRAM/ISR levée.

### 5. `CONFIG_LCD_RGB_RESTART_IN_VSYNC` désactivé — conservé

En ESP-IDF 6.1, le driver détecte déjà les EOF manqués et redémarre uniquement
en cas de sous-alimentation. Cette option force le redémarrage à chaque VSYNC,
ce qui avait aggravé les sauts. Ne pas la réactiver.

Un downgrade vers ESP-IDF 5.x n'est pas recommandé : le firmware isolé était
déjà stable sous 6.1, et le correctif est architectural (affinité des cœurs),
pas une incompatibilité d'IDF.

## Décalage fixe — diagnostic confirmé et correctif

Les timings en vigueur sont ceux du sketch Waveshare validé sur ce panneau :
HSYNC `48/88/40`, VSYNC `3/32/13`, PCLK 16 MHz sur front descendant. Le
framebuffer 800×480 RGB565 est en PSRAM ; le driver `esp_lcd` utilise deux
bounce buffers DMA internes de 40 lignes et `esp_lvgl_port` est en `bb_mode`.

Le premier essai ciblé a suffi : un timer LVGL demande une seule reprise du
panneau une seconde après la construction de l'UI. Le driver exécute la reprise
au VSYNC suivant et le moniteur reçoit `LCD_INIT_STEP=8`. L'image et les zones
tactiles coïncident ensuite sur le matériel.

Ce restart unique est différent de `CONFIG_LCD_RGB_RESTART_IN_VSYNC`. Cette
option reste désactivée : redémarrer à chaque trame avait aggravé les sauts.
Les timings HSYNC/VSYNC, PCLK, les deux bounce buffers de 40 lignes et la
configuration du GT911 ne changent pas. La bordure magenta ajoutée pour le
diagnostic a été retirée sans modifier le correctif.

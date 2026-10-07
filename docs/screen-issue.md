# Instabilité de synchronisation de l’écran

## Écritures flash et cache PSRAM (2026-10-07)

### Symptômes

- L'image glisse à chaque flash : c'est le cas le plus facile à reproduire.
- Un décalage fixe peut survivre à un reboot logiciel : après un flash,
  démarrage en mode Wi-Fi, puis `/confirm`, l'écran est toujours décalé.
- L'écran perd sa synchronisation pendant l'activité réseau.

### Cause probable

En mode « bounce buffer + framebuffer PSRAM », la documentation ESP-IDF 6.1
(`rgb_lcd.rst`) est explicite : le LCD **ne peut pas fonctionner** tant que le
cache de la mémoire externe est désactivé, ce qui arrive pendant toute
écriture ou tout effacement de la flash principale (OTA, NVS). L'ISR qui
remplit les bounce buffers lit le framebuffer à travers ce cache. Bloquée,
elle laisse le DMA LCD sans données, et l'image glisse. La même documentation
recommande `CONFIG_SPIRAM_XIP_FROM_PSRAM` : le code et les données en lecture
seule s'exécutent alors depuis la PSRAM, et le cache externe reste actif
pendant les écritures flash.

Le firmware écran écrit en flash alors que le flux RGB tourne :

- OTA écran par HTTP (`esp_ota_write`) et dépôt OTA capteurs dans
  `ota_staging` (effacement de la partition entière, puis écritures) ;
- `/confirm` : `esp_ota_mark_app_valid_cancel_rollback()` réécrit `otadata` ;
- démarrage Wi-Fi : la calibration PHY peut être enregistrée en NVS
  (`CONFIG_ESP_PHY_CALIBRATION_AND_DATA_STORAGE=y`) ;
- réglages et identifiants Wi-Fi (NVS).

Le décalage qui « survit » au reboot n'en serait pas un : chaque démarrage
en mode Wi-Fi refait une écriture flash pendant que le flux RGB tourne, et
recrée donc le décalage. Cela expliquerait aussi qu'une reprise à 1 s ne
suffisait pas et qu'une reprise à 5 s, après l'init radio, aide. Cette
explication reste une hypothèse : aucun log ne relie encore une écriture
précise au décalage.

### Ce que les exemples Waveshare apportent

Le README des exemples ESP-IDF `08_lvgl_v8_demo` et `09_lvgl_v9_demo` annonce
la bibliothèque `ESP32_Display_Panel`. C'est une couche C++ d'Espressif qui
décrit des cartes connues (brochage, timings, CH422G, GT911,
rétroéclairage) et appelle en dessous le même `esp_lcd_new_rgb_panel()` que
nous. Le code des exemples ne l'utilise d'ailleurs pas : il passe par un
`waveshare_rgb_lcd_port.c` local sur `esp_lcd`. Elle n'apporte rien de plus
au niveau matériel :

- le reset de la dalle par EXIO3 (LCD_RST) existe déjà chez nous ;
- les timings Waveshare ne sont pas cohérents d'une source à l'autre :
  `ESP32_Display_Panel` utilise HSYNC `4/8/8` et VSYNC `4/8/8` ; les exemples
  IDF proposent, sous `#if`, `30/145/170` et `2/23/12` ou bien `4/8/8` et
  `4/8/8`. Les nôtres (`48/88/40`, `3/32/13`) sont validés sur ce panneau et
  ne changent pas.

La différence utile est dans le `sdkconfig.defaults` de l'exemple v9 :
`CONFIG_SPIRAM_XIP_FROM_PSRAM=y`, avec une PSRAM à 120 MHz, des caches
agrandis (instructions 32 Kio, données 64 Kio en lignes de 64 octets dans
l'exemple v8) et une flash en QIO.

### Essai 1 : XIP seul

`CONFIG_SPIRAM_XIP_FROM_PSRAM=y`, qui entraîne `SPIRAM_FETCH_INSTRUCTIONS` et
`SPIRAM_RODATA`. Caches, vitesse PSRAM (80 MHz), timings, bounce buffers,
PCLK et reprise à 5 s ne changent pas.

Coût attendu : l'image est copiée en PSRAM au démarrage (environ 1,7 Mo sur
8 Mo), et chaque défaut du cache d'instructions lit désormais la PSRAM. Ces
lectures partagent la bande passante avec l'ISR des bounce buffers,
c'est-à-dire la contention déjà corrigée en septembre. Si de nouveaux sauts
apparaissent sous charge (infusion, balance connectée), c'est l'essai 2 qui
les traite : caches agrandis, à condition que la SRAM interne le permette.
La PSRAM à 120 MHz, expérimentale et sensible à la température, ne vient
qu'en dernier recours.

Pour relever la marge SRAM, l'écran de diagnostic et l'écran du mode Wi-Fi
affichent en bas la SRAM interne libre et le plus gros bloc allouable.

Protocole :

1. Flasher `0.3.43`. Ce flash ne teste rien : c'est l'ancienne image, sans
   XIP, qui écrit la flash, et l'image glisse comme avant.
2. Plusieurs démarrages en mode Wi-Fi, chacun suivi de `/confirm` : aucun
   décalage, avant comme après la reprise de 5 s. `/confirm` est la première
   écriture flash faite par l'image XIP.
3. Flasher de nouveau l'écran (même image ou suivante), puis le module
   capteurs, en regardant la dalle : c'est maintenant l'image XIP qui écrit,
   l'image ne doit plus glisser.
4. Une infusion avec la balance connectée : aucun saut. Relever les deux
   valeurs SRAM sur l'écran de diagnostic, puis en mode Wi-Fi.

### Résultats (`0.3.43` et `0.3.44`)

**Glissement pendant les écritures flash : corrigé.** Le flash de `0.3.44`,
écrit par `0.3.43` avec XIP, n'a plus fait glisser l'image du tout.

**Marge SRAM interne** (plus gros bloc 31 K dans tous les cas) :

| Mode | Libre |
|---|---|
| Machine (BLE), balance connectée ou non | 88 à 91 K |
| Wi-Fi | 77 K |

Les caches agrandis (essai 2) ne seront essayés que si des sauts
réapparaissent pendant une infusion.

**Décalage fixe au démarrage : toujours présent, indépendant de XIP.**

- Il apparaît au hasard, environ un démarrage sur 5 à 10. Une séquence
  isolée (par exemple `/confirm` suivi d'un redémarrage décalé) ne permet
  donc aucune conclusion.
- Il est déjà visible sur la bannière « coffeeflow », avant toute pile
  radio : ni le Wi-Fi, ni BLE, ni la transition entre les deux n'en sont la
  cause.
- Au dernier essai, la reprise manuelle (`esp_lcd_rgb_panel_restart()`,
  case « firmware », page 3 des réglages) ne remettait pas l'image en place.
  Cela contredit la conclusion de `v0.2.33` ; à revérifier sur plusieurs
  occurrences.

Un état que le redémarrage du flux DMA ne corrige pas, et qui survit à un
reboot logiciel, désigne plutôt la dalle que l'ESP32.

**Ce que montre le schéma Waveshare** (connecteur 40 broches de la dalle,
PORT1) :

- `LCD_RST` (EXIO3) arrive sur la broche 35, marquée `NC`. Le « reset
  dalle » de `board::panel_power_on()` ne réinitialise vraisemblablement
  rien. La dalle est alimentée directement en 3V3 : seule une coupure
  d'alimentation la réinitialise.
- `DISP` (EXIO2, `kCh422gDisp`) commande la broche 31 `DISP` de la
  dalle (veille / marche) **et** l'entrée EN du convertisseur de
  rétroéclairage MP3302, avec un tirage de 10 kΩ au 3V3. Couper le
  rétroéclairage met donc aussi la dalle en veille.

Notre démarrage met `DISP` à 0, démarre le flux RGB, puis remet `DISP` à 1
après le premier rendu LVGL : la dalle sort de veille à un instant
quelconque de la trame. Waveshare, de son côté, passe toutes les sorties du
CH422G à 1 dès son initialisation : `DISP` est haut avant le démarrage du
flux et n'est plus jamais modifié. Hypothèse, non vérifiée faute de
datasheet de la dalle : la sortie de veille au milieu d'une trame fige
parfois une mauvaise phase dans le contrôleur de la dalle, et seule une
nouvelle sortie de veille ou une coupure d'alimentation la libère.

**Le décalage est toujours exactement le même, en X comme en Y.** Ce n'est
donc pas une phase aléatoire : la dalle bascule entre deux états
déterministes. Les contrôleurs de ces dalles RGB 40 broches acceptent
deux modes : DE, où la position de l'image suit le signal DE, et HV (SYNC),
où le premier pixel est pris à un nombre fixe de PCLK après HSYNC et de
lignes après VSYNC, quels que soient nos porches. Si la dalle démarre
parfois en mode HV, le décalage vaut :

- en X : `hsync_pulse_width + hsync_back_porch` (48 + 88 = 136) moins le
  back porch fixe de la dalle ;
- en Y : `vsync_pulse_width + vsync_back_porch` (3 + 32 = 35) moins son
  back porch vertical fixe.

Les valeurs fixes de la dalle sont inconnues sans datasheet. À titre
d'exemple, les valeurs courantes des dalles 800×480 de ce type (46 PCLK, 23
lignes) donneraient 90 px vers la droite et 12 lignes vers le bas : même
sens que l'observation (« environ 150 px à droite, légèrement plus bas »,
estimé au toucher).

Si l'hypothèse tient, le correctif ne dépend pas de la cause du basculement :
choisir les porches pour que la somme impulsion + back porch soit égale au
back porch fixe de la dalle, en X comme en Y. Les deux modes produisent
alors la même image. Il faut pour cela mesurer le décalage exact en pixels
dans l'état décalé.

**Essai `0.3.45` : `DISP` haut dès la première écriture CH422G.** Avant,
`select_can()` remettait `DISP` à 0 à chaque démarrage (veille de la dalle,
y compris après un reboot logiciel), et `DISP` ne remontait qu'après le
premier rendu LVGL, flux RGB déjà lancé. Désormais `DISP` est à 1 dès
`select_can()` et n'est plus jamais modifié, comme chez Waveshare. La
séquence LCD_RST/TP_RST de `panel_power_on()` est inchangée. Le
rétroéclairage s'allume donc dès le démarrage : l'écran est blanc jusqu'au
démarrage du flux RGB, puis la bannière s'affiche. C'est accepté si le
décalage disparaît ; le résultat se juge sur plusieurs jours.

Mesurer le décalage exact (mire affichée avant la bannière, maintenue
jusqu'au premier toucher) ne sert que si cet essai échoue : elle permettrait
de confirmer le mode HV et de caler les porches pour que les deux modes
donnent la même image.

**Combien de démarrages pour conclure.** Avec un défaut tous les 5 à 10
démarrages (10 à 20 %), une série sans aucun décalage doit compter au moins
30 démarrages. Si le défaut est toujours là à 10 %, 30 démarrages propres
ont 4 % de chances d'arriver par hasard ; à 20 %, 0,1 %. Avec 20
démarrages, ces chances montent à 12 % et 1 %.

## Nouvel essai (2026-09-15)

Le décalage fixe reste observable sur certains démarrages malgré la reprise
unique ajoutée en `v0.2.33`. Un essai à PCLK 12 MHz a placé la dalle dans une
boucle de mires blanc/rouge/vert/bleu/noir ; PCLK reste donc à 16 MHz. La reprise
automatique est repoussée de 1 à 5 secondes afin qu'elle arrive après le
démarrage normal de la radio. La case « firmware » de la troisième page des
réglages permet aussi de demander manuellement une reprise au prochain VSYNC.

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

Le premier essai ciblé utilisait un timer LVGL une seconde après la construction
de l'UI. Le nouvel essai porte ce délai à cinq secondes. Le driver exécute la
reprise au VSYNC suivant et le moniteur reçoit `LCD_INIT_STEP=8`. Une demande
manuelle depuis les réglages émet `LCD_INIT_STEP=9`.

Ce restart unique est différent de `CONFIG_LCD_RGB_RESTART_IN_VSYNC`. Cette
option reste désactivée : redémarrer à chaque trame avait aggravé les sauts.
Les timings, les deux bounce buffers de 40 lignes et la configuration du GT911
ne changent pas. La bordure magenta ajoutée pour le
diagnostic a été retirée sans modifier le correctif.

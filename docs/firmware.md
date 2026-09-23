# Firmware

Deux cartes ESP32-S3 font tourner la machine. Le **Waveshare 4,3"** est en façade et il est le seul nœud qui parle au monde extérieur (tactile, Wi-Fi, BLE). Le **XIAO** vit dans `boitier_dc`, dans le compartiment technique. Les capteurs de pression et de débit et les commandes des actionneurs y sont raccordés ; la NTC de chaudière rejoint directement l'écran. Ils se rejoignent sur un bus CAN d'un seul segment, terminé 120 Ω aux deux bouts.

Vue d'ensemble du matériel : `../README.md`. Détail fil par fil : `cablage.md`.

Arborescence :

```
firmware/
  common/     protocole CAN, types de messages, table de codes LOG, CRC
  sensors/    projet ESP-IDF, XIAO ESP32-S3
  screen/     projet ESP-IDF, ESP32-S3-WROOM-1-N16R8
  tools/      décodeur Mac (USB série et WebSocket)
```

**Langage : C++ sur ESP-IDF 5.x pour les deux cartes**, tâches FreeRTOS, pas de `loop()` Arduino. Une seule langue parce que le codec du protocole et le cadrage OTA doivent être identiques des deux côtés, et parce que l'écran (CH422G, GT911, RGB via `esp_lcd`, LVGL, client GATT BLE, `esp_http_server`, NVS) n'est documenté qu'en C. Rust sur le module capteurs reste une réécriture légitime plus tard, une fois l'image factory ennuyeuse et prouvée. Arduino-ESP32 peut être tiré comme composant IDF si une bibliothèque l'exige.

## Répartition

L'**écran est le cerveau**. Il porte l'algorithme d'infusion, l'UI, l'Acaia Lunar (client BLE GATT), le LAN, et **toutes les calibrations**. Il ne pilote jamais la pompe ni la vanne directement.

Le module **capteurs est les mains**. Il exécute des commandes (SSR, dimmer), compte les impulsions du débitmètre, lit la pression et la température, et publie de la télémétrie quand on la lui demande. Pas de radio. Pas de calibration. Pas d'interprétation : il transporte des valeurs brutes.

Cette coupure est aussi thermique et électrique. Le XIAO est spécifié à 85 °C et vit dans un compartiment à 45–50 °C ; le Waveshare reste en façade. Le 230 V ne sort jamais de la machine. La paire CAN et le câble de la NTC rejoignent l'écran.

---

## État de mise en service (2026-09-16)

Les modules **capteurs** et **écran** sont montés dans la machine. Ils sont
branchés au débitmètre, au XDB401, à la pompe et à la vanne ; les premiers
essais portent maintenant sur l'étanchéité de la plomberie et la validation
des capteurs en conditions réelles.

Le chemin de test courant est le bouton **purge** de l'écran. Il alimente
pompe et vanne tant qu'il est maintenu ; puissance et plafond de sécurité se
réglent par pas de 5 % et de 5 s. Le plafond de purge reste appliqué par le
coeur, indépendamment de l'UI. L'écran de purge normal affiche pression, débit
calculé et puissance de pompe, mais pas le compteur brut. Le compteur cumulatif
du débitmètre apparaît sous la forme `n=<…>` uniquement dans l'écran de
service/diagnostic historique (`MAINTENIR PURGE`).

Le débit calculé peut légitimement rester à `0,0 ml/s` au début : il requiert
des fronts du débitmètre et il est remis à zéro après 3 s sans front. Pendant
les essais d'étanchéité, le compteur brut `n` est donc le premier indicateur à
observer ; s'il ne bouge pas alors que l'eau traverse bien la turbine, le
diagnostic porte sur le capteur, son câblage et son sens de montage, avant la
calibration du facteur K.

`GET /telemetry` contient déjà les éléments nécessaires à un relevé de banc :
`weight_g`, `pressure_bar`, `pressure_raw`, `temperature_c`,
`temperature_raw`, `flow_ml_s`, `volume_ml` et `flow_pulse_count`, avec les
validités, présences et âges associés. Depuis l'image écran 0.2.67,
`boiler_temperature_c`, `boiler_temperature_valid`,
`boiler_temperature_freshness`, `boiler_temperature_age_ms` et les codes
`boiler_ntc_a0_raw` / `boiler_ntc_a1_raw` décrivent la sonde NTC chaudière.
`xdb401_temperature_c` / `xdb401_temperature_raw` décrivent explicitement la
température amont chaudière. Dans `/telemetry`, `temperature_c` est un alias
de `boiler_temperature_c` ; `temperature_raw` reste le code brut XDB401.
La validité de la chaudière est indiquée par `boiler_temperature_valid`.
L'écran principal affiche uniquement la chaudière
NTC et un tiret si la mesure est absente ou périmée.
`GET /config` et `POST /config` permettent de régler `purge.pump_pct` et
`purge.max_s`; `POST /action` accepte
`purge_press` et `purge_release`. Un futur script de calibration peut donc
encadrer un essai par deux instantanés, conserver les deltas dans un JSON et
garantir un `purge_release` en sortie d'erreur. L'outil prévu est
`firmware/tools/calibration_purge.py`; il ne tente pas de lire le poids en
Wi-Fi et attend sa saisie après l'essai.

`GET /hf-capture` exporte la dernière session d'actionneur terminée, à 10 Hz,
sous forme JSON. Le buffer est remis à zéro au premier `SET dimmer>0` d'une
nouvelle session, y compris une commande brute `set_actuators`. Il conserve la
consigne et le niveau dimmer rapporté, les phases, les valeurs brutes XDB401 /
Digmesa et leurs valeurs calibrées. Dans les échantillons calibrés,
`volume_ml` est relatif à la capture et vaut toujours zéro au premier
échantillon. Après la confirmation de l'arrêt du dimmer, la capture conserve
quatre secondes d'échantillons `mode="cooldown"` afin d'inclure la fin de
l'écoulement en tasse. `?view=raw`, `?view=calibrated` ou
`?view=both` (défaut) sélectionne les colonnes. L'endpoint retourne
`409 capture_active` jusqu'à la confirmation de `dimmer=0` puis pendant ce
cooldown de quatre secondes; il retourne `404` avant toute capture terminée.
L'export est envoyé par morceaux, sans
construire le document entier en SRAM. L'image écran 0.2.67 produit le schéma
`coffeeflow.hf_capture.v2` avec les codes A0/A1, la température chaudière,
son âge et sa validité. L'outil de tracé accepte aussi les captures v1 et y
interprète `temperature_c` comme la température XDB401.

`firmware/tools/download_hf_capture.py` télécharge la capture et conserve le
JSON brut dans `captures/YYMMDD-HHMMSS.json`. Le fichier peut ensuite être
analysé par d'autres outils ou tracé avec matplotlib :
`COFFEEFLOW_HTTP_TOKEN=… COFFEEFLOW_IP=… uv run firmware/tools/download_hf_capture.py`, puis
`uv run firmware/tools/plot_hf_capture.py captures/YYMMDD-HHMMSS.json --output capture.png`.

**Limite actuelle :** HTTP nécessite le mode Wi-Fi, qui désinitialise le BLE
et déconnecte la balance Acaia. Les relevés HTTP de `weight_g` ne sont donc pas
une mesure de poids vivante pendant une purge en Wi-Fi. La première campagne
de calibration doit soit saisir le poids lu manuellement, soit faire évoluer
cette politique radio avant d'automatiser la comparaison débitmètre ↔ balance.

---

## Module capteurs (XIAO ESP32-S3)

Brochage Grove Shield, tel que câblé. **GPIO natif de l'ESP32-S3**, pas le
D-number du silkscreen Seeed : le Grove Shield XIAO numérote ses ports en
`Dn`, et `Dn` ne vaut **pas** `GPIOn` au-delà de D5 (D6/D7 partent sur
GPIO43/44 pour l'UART0, ce qui décale tout ce qui suit — D8→GPIO7,
D9→GPIO8, D10→GPIO9). Confirmé au multimètre le 2026-09-08 après un
bring-up phase 2 en échec faute de cette traduction :

| Port | Périphérique | Bus | D-number (silkscreen) | GPIO natif |
| --- | --- | --- | --- | --- |
| R1 | XDB401 pression / température | I2C | SDA D4, SCL D5 | SDA **GPIO 5**, SCL **GPIO 6** |
| L4 | Dimmer RBDimmer DimmerLink | I2C (même bus) | SDA D4, SCL D5 | SDA **GPIO 5**, SCL **GPIO 6** |
| R2 | Digmesa FHKSC 932-9525-B | impulsions, front descendant | D7 | **GPIO 44** |
| R3 | Adafruit CAN Pal (TJA1051T/3) | TWAI | TX D8, RX D9 | TX **GPIO 7**, RX **GPIO 8** |
| R4 | M5Stack Unit SSR | sortie GPIO | D10 | **GPIO 9** |
| L2 | HW-399 → SSR chaudière Keysolu/Maxwell | sortie GPIO | D2 | **GPIO 3** |

**CAN Pal validé de bout en bout le 2026-09-09**, après rework de la broche `SLNT`
(tirée au GND — le clone la laissait flottante sous les connecteurs rapportés,
d'où un mode Silent permanent). Un M5Stack Unit CAN avait servi de contournement pendant
l'investigation ; il n'est plus utilisé côté capteurs.

**I2C partagé.** Dimmer à `0x50`, XDB401 à `0x7F`. Les seules pull-ups du bus sont les 4,7 kΩ du XDB401 : retirer le capteur de pression rend le dimmer muet. Accès sérialisé par mutex. Ne pas empiler un second jeu de pull-ups tant que le XDB401 est là.

**Le mutex I2C ne couvre pas l'attente de conversion du XDB401.** Déclencher, relâcher le mutex, attendre ~50 ms, reprendre, lire. Sinon une rampe dimmer à 10 Hz se prend 50 ms de latence pour rien.

### SSR — vanne solénoïde

GPIO 9 (D10 sur le silkscreen du Grove Shield), HIGH = vanne ouverte, LOW = fermée. Le Unit SSR est zero-crossing (MOC3043) : pas d'ISR de passage par zéro, pas de timing. Défaut et repli : LOW, y compris au boot. Le 5 V du module vient de la Wago, pas du port Grove.

### Commande diagnostique de chaudière

Le matériel est câblé pour deux chemins : **GPIO 3 / D2 / L2** du XIAO → **IN4 du HW-399** → sortie absorbante **OUT4** → entrée DC du **SSR chaudière Keysolu/Maxwell KS53 D-24Z20N-LQ** ; et **NTC G1/8** → pont 3,3 V AMS1117 → **A0/A1 de l'ADS1115** sur l'I2C du Waveshare. La commande du SSR est câblée **5 V → SSR `+` → SSR `−` → OUT4**. La logique est inversée : **LOW sur GPIO 3 active la chauffe**, HIGH l'arrête. Schéma et calibration : [ntc_ads1115_calibration.md](ntc_ads1115_calibration.md).

Depuis v0.2.63, le firmware commande GPIO 3 via `SET_HEATING` pour une impulsion diagnostique de 1 à 30000 ms, sans renouvellement. Depuis v0.2.69, `SET_HEATING_POWER` porte une puissance de 0,0 à 100,0 % par pas de 0,1 % et un bail de 1500 ms, renouvelé par l'écran toutes les 500 ms. Le module capteurs réalise la modulation sur une période fixe de 5 s ; un renouvellement ne redémarre pas cette période. Sous 2 %, il répartit des impulsions de 100 ms sur plusieurs périodes. GPIO 3 est mis à HIGH (état inactif) dès son initialisation dans `app_main`, à l'expiration du bail, sur `STOP`, à la perte de présence et au début d'un flash ; il passe à LOW pour chauffer. Avant l'initialisation logicielle, notamment durant reset, l'état dépend du matériel et doit être vérifié. Le SSR existant dans `STATUS_ACTUATORS` reste celui de la **vanne**. `temperature_raw` concerne le **XDB401** ; `temperature_c` dans `/telemetry` concerne la NTC chaudière depuis l'image écran 0.2.67. GPIO 3 est aussi une broche de strapping pour le choix JTAG dans certaines configurations eFuse ; vérifier le chemin de récupération de la carte avec le HW-399 raccordé.

`POST /action` accepte `{"action":"set_heating","on":true,"duration_ms":1000}` puis `{"action":"set_heating","on":false}`. La réponse confirme l'acceptation ; `GET /telemetry` expose `heating_requested`, `heater_on` (`null` sans écho frais), `heating_freshness`, `heating_capable` et le bail restant. `set_brew_actuators` utilise `pump_pct` et `ttl_ms`. L'ancien `set_actuators` avec `dimmer` garde le même sens ; `dimmer_pct` reste un alias de `pump_pct` en télémétrie.

Revue des noms dans le code actuel :

| Endroit | Nom actuel | Sens actuel | Nom conseillé pour l'extension |
| --- | --- | --- | --- |
| `firmware/sensors/main/main.cpp` | `g_valve_open`, `apply_valve()`, `kGpioValve` | relais de vanne R4 | chauffage séparé sur L2 |
| `firmware/common/include/common/messages.hpp` | `StatusActuatorsPayload::valve_open` | état de la vanne | octet CAN 0 conservé |
| `firmware/common/include/common/messages.hpp` | `SetPayload::pump_pct` | puissance de pompe | octet CAN 0 conservé |
| `firmware/sensors/main/main.cpp` | `g_pump_pct`, `apply_dimmer()`, registres `kDimmer*` | niveau de pompe et pilote matériel DimmerLink | pilotes DimmerLink inchangés |
| `firmware/screen/main/core` | `snapshot.valve_open`, `snapshot.pump_pct`, `snapshot.heater_on` | vanne, pompe et chaudière | `dimmer_pct` reste un alias |
| HTTP (`net_http.cpp`) | `set_brew_actuators` avec `pump_pct` | commande de pompe | `set_actuators` avec `dimmer` reste un alias |

`on_set_received()` déduit l'état de la vanne de `payload.pump_pct > 0`. Le chauffage utilise sa propre commande et son propre état. Les octets des messages CAN existants et les champs HTTP historiques gardent leur sens.

### Dimmer — pompe

DimmerLink en I2C, pas UART. Le Cortex du module gère le passage par zéro et le triac ; le XIAO n'écrit que des registres.

- `0x10` niveau, 0–100 % — la seule commande du cycle d'infusion
- `0x00` statut et `0x02` erreur sont remontés dans `STATUS_ACTUATORS` pour que l'écran affiche « dimmer pas prêt » ou une erreur réelle au lieu de ne rien faire silencieusement. La fréquence secteur (`0x20`) reste informative et n'est pas un flag de santé.

**Sans secteur sur le dimmer, le module reste en `Calibrating...`** et refuse toute écriture. La commande `0x03` bascule le dimmer en UART **et l'écrit dans son EEPROM** : à ne jamais appeler depuis le firmware. Le seuil de calage de la pompe vibratoire est une calibration, mesurée sur la machine, stockée côté écran — pas une constante dans le source.

Code de banc existant : `../tests/test_rbi2c.py`.

### Débitmètre

Digmesa 932-9525-B, buse 1,00 mm, **2382 impulsions par litre** (0,42 ml par impulsion), collecteur ouvert NPN. Le filtre RC du shield (1 kΩ vers 3,3 V, 10 nF vers GND) fournit un front descendant 3,3 V sur GPIO 44 (D7 sur le silkscreen du Grove Shield) ; pull-up interne éteinte. Une ISR incrémente un compteur 32 bits et mémorise l'horodatage du dernier front.

**Le module capteurs ne calcule ni volume ni débit.** Il publie le compteur cumulé et la date de la dernière impulsion ; l'écran en dérive tout, avec le facteur K et la courbe de correction qu'il détient. Trois raisons :

- un compteur cumulé est idempotent : une trame perdue ne coûte rien, un delta perdu est un volume perdu pour toujours ;
- recalibrer devient rétroactif sur les shots déjà enregistrés ;
- l'horodatage du dernier front donne la période inter-impulsion réelle, indépendamment de la fréquence de publication.

**Ce que ce capteur peut et ne peut pas faire.** À 1–2 ml/s en extraction, il sort 2,4 à 4,8 impulsions par seconde. Un débit à ±10 % demande une dizaine d'impulsions, donc **~4 s de moyennage à 1 ml/s**. C'est un totaliseur de volume, pas un capteur de débit temps réel : il ne peut pas être la boucle rapide d'un flow control. Les signaux rapides sont la pression (50 ms) et le poids Acaia (10 Hz).

Le capteur est **en amont de la pompe** (3 bar de tenue), dans la ligne qui va de l'adaptateur du réservoir à la pompe en passant par le filtre.

**L'OPV ne renvoie pas au réservoir**, elle renvoie à l'entrée de la pompe, sur l'adaptateur qui presse contre la soupape de fond du bac. Le débitmètre est donc dans la boucle de recirculation, et il mesure le **débit de la pompe**, pas le débit net tiré du réservoir. Conséquence, et c'est la bonne :

- **OPV fermée** — toute la sortie de la pompe traverse la galette. Débit pompe = débit d'infusion, le capteur lit exactement ce qu'on veut.
- **OPV ouverte** — le capteur compte en plus l'eau recirculée, et il sur-lit.

Avec l'OPV réglée à 11 bar pour un fonctionnement à 9 bar, **elle ne s'ouvre jamais pendant une extraction**. Le seul régime où la lecture ne veut plus rien dire est la purge de backflush, où on ne mesure rien de toute façon. Un flow control fondé sur le débit reste donc légitime ; ce qui le limite est la résolution du capteur (ci-dessus), pas la plomberie. Voir `debitmetres.md`.

### XDB401 — pression et température

Même bus I2C. Déclencher une conversion (`0x30` / `0x0A`), attendre ~50 ms, lire 5 octets à partir de `0x06` : pression 24 bits, température 16 bits. Ces 5 octets partent **tels quels** sur le CAN. La pleine échelle est une propriété de la pièce (le script de banc suppose 10 bar, à confirmer sur l'exemplaire monté) et c'est une calibration : elle vit côté écran.

Mesure côté groupe, en amont de la vanne solénoïde.

---

## Module écran (Waveshare ESP32-S3-Touch-LCD-4.3)

Dalle RGB 800 × 480, tactile GT911, 16 Mo de flash, 8 Mo de PSRAM, TJA1051T/3 intégré. CAN sur GPIO 15 TX / 16 RX. `CAN_SEL` est l'EXIO5 du CH422G et **doit être tenu haut**, sinon le transceiver n'est pas sélectionné (cette ligne est aussi USB_SEL, actif bas). Notes de bring-up CH422G / GT911 : `../tests/screen/hello_waveshare/`.

Depuis v0.2.64, le reset du tactile maintient `TP_IRQ` (GPIO 4) à LOW pendant `TP_RST` pour fixer l'adresse I²C du GT911, comme dans l'exemple Waveshare. En cas d'échec à l'initialisation, `screen` essaie les deux adresses possibles puis réinitialise seulement le tactile et réessaie. La télémétrie publie `touch_ready` et `touch_press_count` : une pression physique doit incrémenter ce compteur avant de confirmer une nouvelle image. Après le flash OTA de la v0.2.64, le tactile a fonctionné dès le redémarrage et trois pressions ont été comptées.

Quatre travaux concurrents, qui ne sont pas chauds en même temps :

| Travail | Quand ça compte |
| --- | --- |
| UI LVGL | toujours, surtout pendant une infusion |
| Client BLE → Acaia Lunar | pendant une infusion (poids) |
| CAN → capteurs | pendant une infusion (commandes + télémétrie) |
| Wi-Fi / HTTP | configuration, flash, envoi du shot en fin de cycle |

Code de référence pour le protocole Acaia (cadrage des trames, décodage
poids/temps/boutons, heartbeat obligatoire) : `reference/acaia-ble/`.
C'est du code Arduino-ESP32 d'un projet antérieur, à ne pas compiler tel
quel dans `firmware/screen` (qui est en ESP-IDF pur) — voir le `README.md`
du dossier pour ce qui est réutilisable et ce qui ne l'est pas.

Le Wi-Fi est inactif en plein shot ; BLE + CAN + LVGL ne le sont pas. LCD, LVGL et la boucle d'infusion sur le **cœur 1** (l'ISR DMA du panneau RGB doit y vivre, sinon sauts d'image — `docs/screen-issue.md`) ; TWAI, UART, pont, Wi-Fi et pile BLE sur le **cœur 0**, là où Espressif met déjà les radios. Le CAN est interruption + file, vidée par la tâche qui porte le protocole.

### Les capteurs vus depuis l'écran

Tout ce qui est mesuré est derrière une seule interface, quel que soit le transport : pression, température, débit et volume arrivent par CAN, le poids par BLE, mais l'algorithme d'infusion ne le sait pas.

Une source expose un échantillon `{ horodatage, valeur brute, validité }` ; la calibration est une couche au-dessus. Deux bénéfices directs :

- l'algorithme se teste sur le Mac, sans machine ;
- **le dump de l'outil de monitoring est le format de fixture des tests** : on enregistre un vrai shot, on le rejoue dans l'algorithme.

### Réseau

Les identifiants Wi-Fi sont saisis via un point d'accès temporaire et une page
d'accueil, puis stockés en **NVS** (pas d'EEPROM sur ESP32). Leur existence
définit le mode : absents → AP, présents → STA ; une panne STA ne réactive
jamais l'AP, seul « oublier le réseau » efface explicitement les identifiants.
Ils n'apparaissent jamais dans le source. Un secret HTTP partagé vit dans un
en-tête non commité, utilisé en `Authorization`. Le serveur est en HTTP, pas
HTTPS : le secret évite juste que le LAN soit un jouet.

Première mouture de l'API :

- `GET` — dernière télémétrie (pression, température, débit, volume, dimmer, SSR) plus ce que l'écran sait seul (poids, état d'infusion)
- `GET /hf-capture` — dernière capture haute fréquence, seulement après arrêt confirmé
- `POST /action` — actions, dont le niveau dimmer
- `POST` image firmware, avec destination **screen** ou **sensors**
- `GET` / `POST` **`/config`** — toute la configuration en un seul objet JSON, voir ci-dessous
- WebSocket — miroir de tout le trafic CAN, brut. C'est le sniffer une fois les cartes en boîte.

#### L'heure vient du réseau, et ne sert jamais à mesurer

L'ESP32-S3 n'a **pas de pile de sauvegarde** : son RTC ne compte que sous
tension. Comme la machine est coupée à l'interrupteur entre deux sessions,
l'heure murale n'est pas décalée au démarrage suivant, elle est **absente**.
Elle est obtenue par **SNTP une seule fois, à l'association Wi-Fi**, avec le
fuseau suisse et ses règles d'heure d'été figés dans le firmware. Pas de
resynchronisation périodique : sur une session d'une trentaine de minutes, la
dérive de l'oscillateur RC du domaine RTC ne gêne rien.

**Tout ce qui mesure une durée utilise l'horloge monotone, jamais l'heure
murale** : chronomètre d'infusion, phases, bail, péremption des valeurs,
temporisateur d'invalidation OTA, seuils de veille. L'heure murale ne sert qu'à
*étiqueter* — un shot, un événement. La raison est concrète : l'association
Wi-Fi peut aboutir à n'importe quel moment, y compris en plein shot, et le pas
SNTP qui suit décalerait ou ferait reculer tout chronomètre qui s'y appuierait.

Tant qu'aucune synchronisation n'a abouti, un indicateur « heure connue » reste
faux et **rien n'affiche ni n'horodate avec une heure fausse**.

#### Envoi des shots vers `coffeetracker`

**Hors phase 6**, mais les décisions ci-dessous se prennent avant, parce
qu'elles coûtent cher à rattraper. **`coffeetracker`** — dépôt séparé, hors de
ce projet, dont il faut demander l'accès plutôt que le chercher — est un
magasin de shots déjà en service : FastAPI, un fichier JSON par shot,
`Authorization: Bearer` sur les écritures — le **même schéma que l'API de
l'écran**, avec les rôles inversés. C'est la première fois que l'écran est
*client* HTTP et pas seulement serveur.

- **La machine envoie `brewed_at`, en epoch Unix (ms, UTC).** Le backend
  horodate aujourd'hui à la réception (`received_at`) parce que le M5Core2 qui
  l'alimente n'a pas d'horloge. Il garde ce champ tel quel ; `brewed_at`
  s'ajoute et fait autorité quand il est présent. Les shots déjà stockés et le
  M5Core2 continuent de fonctionner sans rien changer.
- **L'horodatage est pris au *départ* du shot**, converti depuis l'horloge
  monotone à cet instant, et transporté avec lui. Il reste donc correct même
  si l'utilisateur attend avant l'envoi.
- **Une seule dernière infusion est conservée, en PSRAM uniquement.** L'écran
  la montre avant envoi (heure, durée, poids) afin que l'utilisateur sache ce
  qui partira. Un envoi accepté l'efface ; une coupure, un redémarrage ou une
  nouvelle infusion la remplace. Il n'y a ni tampon persistant, ni réessai,
  ni historique : sans réseau ou sans geste explicite d'envoi, l'infusion est
  simplement perdue.
- **L'URL et la clé du serveur vivent dans la configuration**, la clé en
  écriture seule — jamais renvoyée par `GET /config`, comme le mot de passe
  Wi-Fi.
- **Ne jamais réécrire les champs déclarés du profil avec des mesures.**
  `brew_temp_c` et `pressure_bar` du profil `coffeetracker` sont des valeurs
  *saisies par l'opérateur*, pas des mesures. L'écran, lui, aura la pression et
  la température **mesurées**. Les faire arriver sous les mêmes noms
  changerait le sens de ces colonnes au milieu du jeu de données, et rendrait
  incomparables les shots d'avant et d'après. Les mesures s'ajoutent sous
  leurs propres noms.

La politique radio joue déjà correctement ici : le Wi-Fi est coupé pendant le
shot, l'envoi a donc lieu après le cycle — ce que dit déjà le tableau des
travaux concurrents.

#### `/config` — toute la configuration en un objet JSON

**`GET /config` renvoie l'intégralité des réglages d'usage, `POST /config` le
remplace.** Rien de configurable ne doit exister uniquement dans l'écran
tactile : réglages d'infusion, profils, seuils de veille. Les calibrations ne
sont pas modifiables à l'exécution : le protocole hôte les écrit dans
`core/calibration_machine.h`, versionné avec l'image écran. Deux raisons, et
la première suffirait :

- **Sauvegarde et restauration depuis un hôte distant.** Ces valeurs vivent en
  NVS sur une carte qu'on aura fermée dans la façade. Un `curl > config.json`
  les rend récupérables.
- **Régler autre part qu'au doigt.** Les calibrations sont faites rarement par
  un protocole de mesure hôte et nécessitent de toute façon un build/flash ;
  elles ne justifient pas une UI ou un format NVS de courbes prématuré.

Règles qui rendent ça utilisable plutôt que dangereux :

- **`POST` partiel accepté** : les clés absentes gardent leur valeur. Une
  restauration complète est simplement un `POST` de tout l'objet.
- **Validation avant écriture, tout ou rien.** Chaque valeur est bornée
  (mêmes bornes que l'UI, voir `ui.md`) ; une seule clé hors bornes rejette
  l'objet entier avec `400` et le nom de la clé fautive. On n'écrit jamais une
  configuration à moitié appliquée en NVS.
- **`version` obligatoire en tête de l'objet**, incrémentée à chaque
  changement de schéma. Un `POST` d'une version inconnue est refusé plutôt
  qu'interprété de travers — un fichier de sauvegarde vieux d'un an ne doit
  pas pouvoir écrire une calibration dans le mauvais champ.
- **Les identifiants Wi-Fi n'y sont pas.** Ni en lecture, ni en écriture : ils
  restent au provisioning. Une sauvegarde de configuration ne doit pas
  contenir un mot de passe en clair, et un objet restauré ne doit pas pouvoir
  couper l'écran du réseau.
- **Refusé pendant une infusion ou une purge** (`409`) : on ne change pas les
  bornes sous les pieds de l'algorithme qui tourne.
- L'UI relit la configuration après un `POST` accepté — l'écran affiche
  toujours ce qui est réellement en NVS, jamais une copie divergente.

```json
{
  "version": 6,
  "brew": { "target_weight_g": 36.0, "target_time_s": 28, "target_pressure_bar": 9.0, "pump_pct": 100 },
  "heating": { "enabled": true, "brew_temperature_c": 90.0 },
  "filling": { "time_s": 3, "pressure_target_bar": 0.3, "pump_pct": 100 },
  "preinfusion": { "time": true, "pressure": false, "weight": false, "time_s": 4, "pressure_bar": 1.5, "pump_pct": 30 },
  "rampdown": { "mode": "none", "lead_time_s": 3.0, "lead_weight_g": 4.0, "pressure_drop_bar": 1.0 },
  "purge": { "pump_pct": 100, "max_s": 20 },
  "ui": { "dim_after_s": 240, "standby_after_s": 1800 },
  "profiles": []
}
```

`profiles` reste un tableau vide tant que la notion de profil n'existe pas :
le schéma est prévu pour, et l'ajouter ne changera pas le reste de l'objet.

### Politique radio

**Wi-Fi et BLE sont des modes exclusifs du coeur.** Ils partagent la radio
2,4 GHz et, sur cet écran RGB, leurs contrôleurs exigent aussi de la SRAM
interne. Cette exclusivité libère leurs allocations dynamiques, mais pas le
code IRAM lié dans l'image : ajouter le contrôleur BLE avait aussi retiré
environ 18 Kio à la DIRAM disponible avant même son initialisation, assez pour
faire échouer les bounce buffers LCD. Les optimisations IRAM Wi-Fi sont donc
désactivées et NimBLE utilise son mode basse vitesse et la PSRAM lorsque
possible. Le coeur possède un état radio explicite, pas une convention
implicite de l'UI.

| Mode | Wi-Fi / HTTP | BLE | Actions permises |
| --- | --- | --- | --- |
| **machine** (défaut) | complètement désinitialisés | actif ; poids à basse cadence au repos, pleine cadence pendant un cycle | infusion, purge, tare |
| **Wi-Fi** (bouton explicite) | actifs ; API, WebSocket et flash réseau ouverts | complètement désinitialisé | diagnostic, flash, purge de banc, envoi du dernier shot au backend ; **pas d'infusion** |

L'UI ouvre ce mode comme une destination depuis l'accueil si la place le
permet, sinon depuis les réglages. Son bouton retour est une vraie sortie de
mode, pas seulement une navigation : il arrête et désinitialise Wi-Fi/httpd/
netif avant de relancer BLE. Ce n'est pas un simple
`esp_wifi_stop()` : les buffers doivent être rendus à la SRAM interne. L'UI
affiche l'état (AP, association ou adresse IP), la dernière infusion à envoyer
le cas échéant, et la progression d'un flash réseau. Le pont USB reste
disponible dans les deux cas.

Le mode Wi-Fi est volontairement modal : une infusion ne peut pas démarrer
tant qu'il est actif et le coeur refuse cette action même si un client HTTP la
demande. Une purge de banc reste autorisée pour le diagnostic. L'envoi d'un
shot vers le backend se fait aussi dans ce mode, après la fin du cycle.

Le **même flux de trames** sort en USB série sur l'image factory (voir plus bas) : un seul décodeur côté Mac pour les deux transports.

---

## Sécurité

Une surpression n'est pas un problème de firmware : **c'est l'OPV qui la traite**, mécaniquement. Réglage visé **11 bar**, fonctionnement normal à **9 bar** — en usage courant l'OPV ne s'ouvre jamais, sauf sur la purge de backflush. C'est ce qui autorise le module capteurs à transporter la pression sans la comprendre.

Ce que le firmware doit garantir, c'est qu'on ne laisse pas la vanne ouverte et la pompe à fond. Trois mécanismes distincts, qui ne se remplacent pas :

### 1. Bail sur commande (~500 ms)

Chaque `SET` porte un TTL. À l'expiration, le module capteurs remet SSR à 0 et dimmer à 0. L'écran qui rampe le dimmer à 10 Hz renouvelle le bail sans y penser ; un écran **mort** coupe tout en un demi-tour de seconde.

C'est le mécanisme rapide, et le seul qui compte pendant un shot.

### 2. Présence (sonde uniquement en cas de silence)

Chaque nœud tient un `last_presence`. Toute trame valide attribuable au pair (adressée au nœud ou en broadcast) remet ce compteur à zéro. Après 1,5 s de silence, il entre dans `PRESENCE_CHECK`, envoie un `PING` toutes les 500 ms, puis déclare `PRESENCE_LOST` 1,5 s après le premier ping si aucune trame ne revient. Trois PING sans réponse ni autre trafic suffisent donc à conclure. Le PING est une sonde de silence, pas un trafic périodique redondant.

Perte de présence côté capteurs : arrêt du streaming, SSR bas, dimmer 0. Une carte seule sur la table pingue toutes les 1–2 s et ne voit jamais de pong : c'est le repos correct.

### 3. Plafond de temps de marche — 60 s, verrouillé jusqu'à coupure secteur

Si le SSR ou le dimmer restent actifs **plus de 60 s d'affilée**, le module capteurs coupe tout (SSR bas, dimmer 0) et **reste dans cet état jusqu'à une coupure d'alimentation**. Il continue de répondre au ping et de publier son état, il refuse toute commande d'actionneur et le dit par un code `LOG`.

Ce mécanisme protège contre un écran **fou** (bug qui commande en boucle), là où le bail protège contre un écran **mort**. Il n'est donc **pas renouvelable** par une rafale de `SET`.

Détails qui font que ça tient :

- **Rearmement.** Le compteur est un temps d'activation continu, remis à zéro par un passage OFF → ON. Deux shots de 30 s ne déclenchent rien. Pour fermer la porte à un bug qui commuterait toutes les 59 s, le rearmement exige **au moins 2 s d'arrêt** — jamais un problème pour un usage humain.
- **Le verrou survit à un reset logiciel.** Il est tenu en mémoire RTC et n'est levé que sur un vrai démarrage à froid (`ESP_RST_POWERON`). Une commande `RESET` venue du bus ne le lève pas : sinon l'écran fou l'effacerait lui-même.
- **Le gros bouton de façade est le reset.** Tout le mod est alimenté depuis l'interrupteur principal de la machine : couper la machine coupe le XIAO. La procédure de sortie de verrou est celle que n'importe qui applique déjà à une machine à café.
- **Ça vaut aussi pour la vanne.** La bobine OLAB fait 15 VA ; ouverte en continu elle mérite la même surveillance que la pompe. Même plafond.
- **60 s est aussi une limite matérielle.** La pompe vibratoire chauffe et finit par ouvrir son thermique. Aucun essai utile ne dure plus longtemps.

Le SSR est bas au boot, avant toute initialisation du CAN.

---

## Protocole CAN

**500 kbit/s, identifiants 11 bits, charge utile binaire, un seul message par trame — sauf le flash.**

TWAI est du CAN 2.0 classique : 8 octets par trame, pas de CAN FD. C'est trop peu pour de l'ASCII ; le texte se fabrique côté Mac, par un décodeur qui connaît les types. Le bus transporte des octets, pas des phrases.

### Identifiant

Le **type de message est dans l'ID**, pas dans la charge utile : l'ID est arbitré et filtré gratuitement, et les 8 octets restent disponibles pour les données.

```
bits 10..5   type   (64 valeurs) — valeur basse = priorité bus haute
bits  4..3   dest   (0 broadcast, 1 écran, 2 capteurs)
bits  2..0   src    (1 écran, 2 capteurs)
```

Le type **est** la priorité : pas de champ séparé. Un `STOP` gagne l'arbitrage contre un flash en cours, par construction.

À deux nœuds, le filtre d'acceptation ne sert à rien : pression 10 Hz + débit 5 Hz + actionneurs 2 Hz + ping ≈ 18 trames/s, soit ~0,5 % du bus. Accept-all, aiguillage sur le type. Un troisième nœud remettrait la question.

### Types

| Type | Message | Sens | Charge utile |
| --- | --- | --- | --- |
| `0x00` | `STOP` | S → X | vide |
| `0x01` | `SET` | S → X | actionneurs + bail |
| `0x02` | `RESET` | S → X | vide |
| `0x04` | `SET_HEATING` | S → X | commande chaudière + durée diagnostique |
| `0x06` | `SET_HEATING_POWER` | S → X | puissance chaudière par pas de 0,1 % + bail 1500 ms |
| `0x05` | `CONFIRM_SENSORS_OTA` | S → X | confirmation de nouvelle image |
| `0x08` | `PING` | ↔ | identité + uptime |
| `0x09` | `PONG` | ↔ | identité + uptime |
| `0x10` | `REQSTATUS` | S → X | quoi, à quelle période |
| `0x20` | `STATUS_PRESSURE` | X → S | XDB401 brut |
| `0x21` | `STATUS_FLOW` | X → S | compteur d'impulsions |
| `0x22` | `STATUS_ACTUATORS` | X → S | état + santé |
| `0x23` | `STATUS_HEATING` | X → S | état chaudière + bail |
| `0x30` | `LOG` | ↔ | code + arguments |
| `0x38` | `FLASH_CTRL` | ↔ | sous-commande |
| `0x39` | `FLASH_DATA` | ↔ | 8 octets bruts |

`PING` et `PONG` ont le même format de huit octets : nœud source, version
majeure/mineure/patch et uptime en secondes. Chaque nœud émet un `PING`
d'identité au démarrage; le récepteur met son instantané à jour puis répond
par `PONG`. Les images antérieures, qui émettent un `PING` DLC 0, restent
acceptées. La sonde de présence ne se déclenche qu'après silence : toute trame
valide du pair, y compris `REQSTATUS`, la réarme.

### Charges utiles

`SET` (0x01)

```
[0]     dimmer      0..100 ; 0 demande l'arrêt, >0 l'activation
[1..2]  ttl_ms      uint16, 0 = défaut (500)
[3..7]  réservé
```

`PONG` (0x09)

```
[0]     nœud        1 écran, 2 capteurs
[1..3]  version     majeure, mineure, correctif
[4..7]  uptime_s    uint32
```

`REQSTATUS` (0x10)

```
[0]     type visé   0x20 | 0x21 | 0x22 | 0x23
[1..2]  periode_ms  uint16, 0 = arrêt
[3..7]  réservé
```

Une période par capteur, pas une fréquence globale : la pression et le débit n'ont pas les mêmes besoins. `0` au boot pour tous ; le module ne streame jamais spontanément. Plancher utile côté XDB401 : 50 ms de conversion, donc pas en dessous de ~100 ms.

**Convention `flags` — bit0 « capteur valide ».** Chaque `STATUS_*` qui porte une lecture de capteur réserve un bit à la même question : est-ce que cette valeur vient d'être obtenue avec succès ? Le sens est générique, mais la capacité de détecter une absence ne l'est pas :

- **XDB401 (I2C)** : détection réelle. Une transaction I2C qui échoue (adresse muette, bus figé) ou un timeout de conversion mettent ce bit à 0 — la valeur brute qui l'accompagne reste la dernière connue, pas un zéro forcé (voir `docs/firmware-implementation.md`).
- **Débitmètre (GPIO seul)** : pas de détection possible. Une simple entrée GPIO ne dit rien sur la présence du capteur, seulement sur les fronts qu'elle reçoit — ce bit reste **toujours à 1** sur `STATUS_FLOW`. L'absence se devine autrement, indirectement, par une absence d'impulsions *attendues* (`LOG FLOWMETER_SILENT`), pas par ce bit.
- **Dimmer (I2C, pas encore câblé)** : même détection réelle que le XDB401, prévue mais pas encore implémentée — voir `STATUS_ACTUATORS` ci-dessous.

`STATUS_PRESSURE` (0x20) — recopie du registre `0x06`

```
[0..2]  pression brute      24 bits
[3..4]  température brute   16 bits
[5..6]  horodatage ms       uint16 (16 bits bas)
[7]     flags               bit0 capteur valide (détection I2C réelle), bit1 timeout conversion
```

`STATUS_FLOW` (0x21)

```
[0..3]  impulsions          uint32 cumulé depuis reset
[4..5]  dernier front ms    uint16 (16 bits bas)
[6]     flags               bit0 capteur valide — toujours 1, absence non détectable en GPIO seul
[7]     réservé
```

`STATUS_ACTUATORS` (0x22) — accusé de réception d'un `SET`, et statut
individuel diffusable périodiquement par `REQSTATUS`

```
[0]     ssr                 0 | 1
[1]     dimmer              0..100
[2..3]  bail restant ms     uint16
[4..5]  marche continue ms  uint16   (pour voir arriver les 60 s)
[6]     flags               bit0 verrou actif, bit1 dimmer prêt, bit2 dimmer valide (détection I2C),
                            bit3 erreur dimmer active (bit ERROR du registre 0x00)
[7]     réservé
```

Il n'y a pas d'acquittement séparé pour `SET` : l'écran compare ce qu'il a commandé à ce qui revient ici. Les commandes sont idempotentes, le dernier gagne ; le module n'empile pas de file de niveaux dimmer.

`SET_HEATING` (0x04) porte `[0] on` (0 ou 1) et `[1..2] duration_ms` en little endian. `on=1` est accepté pour 1 à 30000 ms, indépendamment du bail et de la marche de la pompe. `SET_HEATING_POWER` (0x06) porte `[0..1] power_permille` (0–1000, soit 0,0–100,0 %) et `[2..3] lease_ms` ; seul un bail de 1500 ms est accepté. Le format ancien à 3 octets (pourcentage entier puis bail) reste accepté par `sensors`. `STATUS_HEATING` (0x23) porte `[0] heater_on`, `[1..2] bail restant ms`, `[3] bit0 capacité diagnostique, bit1 capacité puissance, bit2 résolution 0,1 %`, `[4] partie entière du pourcentage`, `[5] dixième`. `screen` exige le bit2 pour la régulation.

`LOG` (0x30)

```
[0]     code
[1]     sévérité
[2..3]  arg16
[4..7]  arg32
```

Pas de texte sur le bus. **La table de codes est générée depuis une source unique** dans `common/`, consommée à la fois par le C++ et par le décodeur Python. Sinon la table du Mac dérive de celle du firmware, et le premier message qu'on ne comprendra plus sera celui d'un crash.

Codes de départ : boot, prêt, reboot demandé ; bail expiré, présence perdue, **verrou 60 s déclenché** (arg32 = ms d'activation), commande refusée car verrouillée ; erreur I2C (arg16 = adresse, arg32 = `esp_err_t` quand disponible), dimmer en calibration, erreur dimmer (arg16 = registre `0x02`), timeout XDB401 (arg32 = `esp_err_t`), débitmètre muet ; début / progression / fin / échec de flash ; OTA en attente de validation, validée, rollback.

### Flash — le seul cas de réassemblage

Une image de 1 Mo ne tient pas dans une trame, et c'est le seul message dans ce cas. Pas de numéro de séquence par trame : sur CAN, un émetteur unique délivre dans l'ordre sans duplication, et le seul mode de panne réel est la perte par débordement de la file RX.

1. `FLASH_CTRL BEGIN` porte la taille. Le récepteur **efface toute la partition avant d'acquitter** : plus aucun effacement pendant le flux, donc plus la cause principale de débordement.
2. `FLASH_DATA` : **8 octets de données pures**, aucun en-tête.
3. Tous les **2 ko** (256 trames), `FLASH_CTRL BLOCK_ACK` avec le numéro de bloc et le CRC16 calculé. L'émetteur attend. **L'acquittement est le contrôle de flux.**
4. Bloc faux → on rejoue 2 ko, pas 1 Mo.
5. `FLASH_CTRL END` porte le CRC32 global. Vérification, bascule d'`otadata`, reboot.
6. `FLASH_CTRL ABORT`, ou une perte de présence, annule l'écriture sans toucher à `otadata`.

Surcoût ~0,4 %. Une image de 1 Mo, c'est ~34 s de fil à 500 kbit/s, disons une minute avec les allers-retours. Le streaming est arrêté pendant un flash. Les actionneurs sont coupés.

500 kbit/s sur une paire courte et terminée est conservateur ; 1 Mbit/s est plausible plus tard. On commence à 500.

---

## Mise à jour

**Image factory jamais réécrite, deux emplacements OTA, rollback applicatif, et le CAN comme simple tuyau.**

```
nvs        data  nvs
otadata    data  ota
phy_init   data  phy
factory    app   factory   image de secours, jamais mise à jour
ota_0      app   ota_0     courante
ota_1      app   ota_1     suivante
```

Tailles ajustées par puce : XIAO 8 Mo, Waveshare 16 Mo (plus une partition de données pour les ressources LVGL).

### L'image factory de l'écran est un pont USB-série ↔ CAN

Le Waveshare reste atteignable en USB-C une fois monté : `screen_wedge` fait déboucher les canaux USB-C et UART sur la face extérieure, et à défaut la façade se démonte pour amener l'écran au Mac. C'est ce qui permet à son image factory d'être **minuscule** : TWAI, CDC, `FLASH`, ping/pong. **Ni Wi-Fi, ni HTTP, ni LVGL.**

Trois conséquences :

- l'outil Mac parle **le même protocole** en USB et en WebSocket — un seul décodeur ;
- le sniffer CAN existe dès le premier jour, avant la moindre ligne de Wi-Fi ;
- le provisioning Wi-Fi, le httpd, le WebSocket et le proxy OTA descendent dans `ota_0`, là où ils sont corrigibles.

Précaution d'usage : brancher l'USB pendant que la machine est sous tension relie la masse du laptop à celle de l'alim RECOM. **Laptop sur batterie, débranché du secteur.** C'est un chemin de dépannage et de bring-up, pas un usage courant.

L'image factory du **module capteurs** est le vrai filet : TWAI, `FLASH`, ping/pong, le verrou 60 s, GPIO 9 tenu bas. Aucune logique d'infusion. C'est la carte qu'on ne veut pas aller rechercher au fond de la machine.

### Flash local (l'écran)

`POST /firmware?target=screen`, en-tête `Authorization`, corps `.bin`. Écriture de l'emplacement OTA inactif, marquage bootable, reset. Après redémarrage, vérifier Wi-Fi, HTTP, `GET /telemetry` et la route `/firmware`, puis appeler `POST /firmware/confirm` avec le même jeton dans les cinq minutes. La confirmation exige Wi-Fi connecté, CAN vivant et écho des actionneurs frais. Sinon l'image revient en arrière. Une panne CAN laisse l'accès au mode Wi-Fi et le flash de `screen` disponibles ; le flash de `sensors` exige toujours un écho CAN sûr.

`GET /telemetry` expose `screen_ota_pending_verify`, `screen_running_partition`, `ota_staging_available` et `ota_staging_size` pour préparer le déploiement. Tant que l'image écran attend sa confirmation, un autre flash est refusé afin de conserver l'image de repli.

Pendant `NEW` ou `PENDING_VERIFY`, `screen` démarre directement en mode Wi-Fi après l'initialisation LCD. HTTP reste ainsi accessible si le tactile échoue au premier démarrage OTA. Vérifier `touch_press_count` après un appui réel, puis confirmer ; en cas d'échec du tactile, laisser expirer le délai de rollback.
Le bootloader actuellement installé sur l'écran a laissé l'image 0.2.64 en état `NEW` après un POST OTA (observé via `screen_ota_state`). Depuis 0.2.65, l'application traite aussi `NEW` comme une image à confirmer : elle garde le Wi-Fi actif et son délai de rollback applicatif, et `POST /firmware/confirm` peut la marquer `VALID`. Cette reprise applicative ne remplace pas le rollback du bootloader en cas de plantage avant le démarrage de l'application ; corriger le bootloader installé exigera un flash physique.

Attention à ce qu'IDF fait et ne fait pas : en `PENDING_VERIFY`, si l'application ne se valide pas, le bootloader revient en arrière **au prochain redémarrage** — mais rien ne redémarre tout seul. Il faut un temporisateur propre qui appelle l'invalidation-et-reboot, ou laisser le task watchdog frapper. Sans ça, une image qui démarre et ne se valide jamais reste en place indéfiniment.

### Flash distant (les capteurs, à travers l'écran)

Même route, `target=sensors`. Le proxy utilise exclusivement la partition `ota_staging` : si la table installée ne la contient pas, il refuse le transfert sans effacer `assets`. L'écran relaie `BEGIN`, des blocs numérotés et acquittés, puis `END`. Le rejeu d'un bloc est idempotent avec `sensors` depuis v0.2.63 ; avec une ancienne image, le proxy ne retente pas un bloc sans ACK. Après redémarrage, `screen` confirme automatiquement la nouvelle image par CAN seulement après version v0.2.63 ou supérieure, échos frais de la pompe/vanne et de la chaudière, pompe et vanne au repos. Depuis v0.2.69, une chauffe régulée peut continuer pendant cette confirmation si son bail est actif ; une impulsion diagnostique doit être terminée. Depuis v0.2.65, `sensors` accepte cette confirmation en état `NEW` ou `PENDING_VERIFY`. En l'absence de confirmation, `sensors` revient en arrière après 30 s. Il n'y a pas de POST de confirmation pour `sensors`.

### Pourquoi factory + deux OTA, et pas seulement deux OTA

Deux emplacements avec rollback couvrent « la nouvelle application plante ». Ils ne couvrent pas « la nouvelle application démarre, se valide, et a un bug CAN qui empêche de reflasher ». L'image factory ne se valide jamais elle-même ; elle ne parle que le protocole minimal. Elle reste petite.

---

## Ce que l'écran fera, après le bootstrap

Ce ne sont pas les premiers firmwares. C'est pourquoi l'interface capteurs a cette forme.

- **Purge / flush** — SSR ouvert, dimmer 100 %, tant que le bouton est tenu. C'est le seul cas où l'OPV s'ouvre normalement (backflush).
- **Infusion au poids** — jusqu'à la cible Acaia, moins un décalage d'anticipation pour les dernières gouttes du groupe.
- **Infusion au temps** — repli quand la balance manque.
- **Pré-infusion** — basse pression dans le ciel du groupe, pause sur la galette, puis rampe vers une cible qui peut être inférieure à 100 %. Le déclencheur sera vraisemblablement un **timer ou une détection de montée en pression** : à 0,5 ml/s le débitmètre est à ~1,2 impulsion par seconde, on verra à la calibration s'il apporte quelque chose.
- **Flow control** — si la pression s'effondre (canalisation), on lève le pied. Piloté par la pression et le poids, pas par le débitmètre.

Calibrations côté écran : facteur K du débitmètre et correction bas débit,
pleine échelle du XDB401, carte dimmer → pression, seuil de calage de la
pomme, grammes d'anticipation. Elles sont versionnées dans l'image écran après
le protocole de mesure hôte ; un remplacement de XIAO ne fait rien perdre.

---

## Calibration, dans la machine

Elle se fait sur la machine, pas au banc : c'est la plomberie réelle qu'on calibre, OPV comprise. Référence de mesure : la balance Acaia. Deux porte-filtres de simulation, percés d'un trou central, imposent une résistance connue.

Une réserve à respecter : **le point pompe libre ne calibre rien.** ~10 ml/s, c'est 0,6 L/min, au-dessus du plafond du Digmesa 1,00 mm (0,40 L/min). La turbine sature, on lira un chiffre faux sans que rien ne le signale. Les points utiles sont ceux des porte-filtres de simulation, à 1–3 ml/s (0,06–0,18 L/min), en plein linéaire.

Séquence :

1. **Facteur K.** Porte-filtre de simulation donnant une pression **sous** l'ouverture OPV. Impulsions comptées contre grammes lus à la balance : le rapport doit coller. C'est le K réel du montage.
2. **Seuil réel d'ouverture de l'OPV.** Même montage, on monte la pression jusqu'à ce que le compte d'impulsions décroche de la balance : à partir de là le capteur compte l'eau recirculée en plus (l'OPV renvoie à l'entrée de la pompe, donc en amont du débitmètre). Ce décrochage **est** la mesure du seuil d'ouverture, et il confirme au passage que le réglage 11 bar est bien celui qu'on croit. Au-dessus, la lecture de débit n'est plus exploitable — c'est attendu, pas une panne.
3. **Point de décrochage.** Descente à 0,5 puis 0,3 ml/s. Dit si la pré-infusion est mesurable ou si c'est un timer.
4. **Carte dimmer → pression** et **seuil de calage** de la pompe, avec le XDB401 comme lecture.

Chaque essai reste sous 60 s : au-delà la pompe chauffe et son thermique finit par s'ouvrir, et le verrou du module capteurs coupe de toute façon.

---

## Décisions déjà prises

| Sujet | Décision |
| --- | --- |
| Langage | C++ / ESP-IDF 5.x, les deux cartes, `common/` partagé |
| Encodage CAN | binaire, type dans l'ID, une trame par message |
| Endianness | little-endian pour tous les champs multi-octets des charges utiles |
| Réassemblage | uniquement pour `FLASH`, par blocs de 2 ko acquittés |
| Télémétrie | valeurs brutes, calibration côté écran |
| Surpression | traitée par l'OPV (11 bar), pas par le firmware |
| Sécurité firmware | bail 500 ms + présence 1–2 s + verrou 60 s jusqu'à coupure secteur |
| Factory écran | pont USB-série ↔ CAN, sans radio |
| Mise à jour | factory + `ota_0` / `ota_1`, rollback, CAN en tuyau |
| Calibration | sur la machine, balance Acaia, porte-filtres de simulation |

## Ce qui reste à trancher

- **Pleine échelle réelle du XDB401** monté (le banc suppose 10 bar).
- **Seuil réel d'ouverture de l'OPV** — étape 2 de la calibration. Le principe est connu (retour à l'entrée de la pompe), c'est la valeur qui manque.
- **Tailles exactes des partitions**, une fois qu'on connaît le poids de l'application écran avec LVGL et BLE.
- **Utilité du débitmètre en pré-infusion** — dépend du point de décrochage.
- Passage éventuel à **1 Mbit/s** sur le bus, après mise en boîte.
- ~~Stratégie de provisioning Wi-Fi~~ **Décidé (2026-09-09) : point d'accès
  temporaire + page d'accueil**, pas de saisie tactile LVGL. Raisons :
  testable dès le point 1 de la phase 6 (avant HTTP/WebSocket/BLE/LVGL), et
  sert aussi de filet de secours (un SSID erroné en NVS rend l'écran
  injoignable en Wi-Fi ; le bouton *oublier le réseau* sur l'écran ramène
  l'AP, contrairement au tactile qui suppose déjà LVGL en place). Stockage
  en NVS, déjà acquis. Validé lot 4 (2026-09-10) : pas de repli AP
  automatique.
- **Charte graphique / design de l'UI écran** (phase 6, LVGL) — à définir,
  session dédiée envisagée avec Opus.

Séquence d'implémentation : `firmware-implementation.md`.

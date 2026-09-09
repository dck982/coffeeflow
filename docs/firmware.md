# Firmware

Deux cartes ESP32-S3 font tourner la machine. Le **Waveshare 4,3"** est en façade et il est le seul nœud qui parle au monde extérieur (tactile, Wi-Fi, BLE). Le **XIAO** vit dans `boitier_dc`, dans le compartiment technique, et il est le seul à toucher les capteurs et les actionneurs 230 V. Ils se rejoignent sur un bus CAN d'un seul segment, terminé 120 Ω aux deux bouts.

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

Cette coupure est aussi thermique et électrique. Le XIAO est spécifié à 85 °C et vit dans un compartiment à 45–50 °C ; le Waveshare reste en façade. Le 230 V ne sort jamais de la machine, seule la paire CAN passe par le trou de l'ancien bouton brew.

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

**I2C partagé.** Dimmer à `0x50`, XDB401 à `0x7F`. Les seules pull-ups du bus sont les 4,7 kΩ du XDB401 : retirer le capteur de pression rend le dimmer muet. Accès sérialisé par mutex. Ne pas empiler un second jeu de pull-ups tant que le XDB401 est là.

**Le mutex I2C ne couvre pas l'attente de conversion du XDB401.** Déclencher, relâcher le mutex, attendre ~50 ms, reprendre, lire. Sinon une rampe dimmer à 10 Hz se prend 50 ms de latence pour rien.

### SSR — vanne solénoïde

GPIO 9 (D10 sur le silkscreen du Grove Shield), HIGH = vanne ouverte, LOW = fermée. Le Unit SSR est zero-crossing (MOC3043) : pas d'ISR de passage par zéro, pas de timing. Défaut et repli : LOW, y compris au boot. Le 5 V du module vient de la Wago, pas du port Grove.

### Dimmer — pompe

DimmerLink en I2C, pas UART. Le Cortex du module gère le passage par zéro et le triac ; le XIAO n'écrit que des registres.

- `0x10` niveau, 0–100 % — la seule commande du cycle d'infusion
- `0x00` statut, `0x02` erreur, `0x11` courbe, `0x20` fréquence secteur — remontés dans les flags de `STATUS_ACTUATORS` pour que l'écran affiche « dimmer pas prêt » au lieu de ne rien faire silencieusement

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

Le Wi-Fi est inactif en plein shot ; BLE + CAN + LVGL ne le sont pas. LVGL et la boucle d'infusion sur un cœur ; Wi-Fi et pile BLE sur l'autre, là où Espressif les met déjà. Le CAN est interruption + file, vidée par la tâche qui porte le protocole.

### Les capteurs vus depuis l'écran

Tout ce qui est mesuré est derrière une seule interface, quel que soit le transport : pression, température, débit et volume arrivent par CAN, le poids par BLE, mais l'algorithme d'infusion ne le sait pas.

Une source expose un échantillon `{ horodatage, valeur brute, validité }` ; la calibration est une couche au-dessus. Deux bénéfices directs :

- l'algorithme se teste sur le Mac, sans machine ;
- **le dump de l'outil de monitoring est le format de fixture des tests** : on enregistre un vrai shot, on le rejoue dans l'algorithme.

### Réseau

Les identifiants Wi-Fi sont saisis une fois — au tactile ou via un point d'accès temporaire et une page d'accueil — et stockés en **NVS** (pas d'EEPROM sur ESP32). Ils n'apparaissent jamais dans le source. Un secret HTTP partagé vit dans un en-tête non commité, utilisé en `Authorization`. Le serveur est en HTTP, pas HTTPS : le secret évite juste que le LAN soit un jouet.

Première mouture de l'API :

- `GET` — dernière télémétrie (pression, température, débit, volume, dimmer, SSR) plus ce que l'écran sait seul (poids, état d'infusion)
- `POST` — niveau dimmer et SSR
- `POST` image firmware, avec destination **screen** ou **sensors**
- `GET` / `POST` **`/config`** — toute la configuration en un seul objet JSON, voir ci-dessous
- WebSocket — miroir de tout le trafic CAN, brut. C'est le sniffer une fois les cartes en boîte.

#### `/config` — toute la configuration en un objet JSON

**`GET /config` renvoie l'intégralité de ce qui est réglable, `POST /config` le
remplace.** Rien de configurable ne doit exister uniquement dans l'écran
tactile : réglages d'infusion, profils, calibrations, luminosité, veille. Deux
raisons, et la première suffirait :

- **Sauvegarde et restauration depuis un hôte distant.** Toutes ces valeurs
  vivent en NVS sur une carte qu'on aura fermée dans la façade. Une calibration
  perdue (flash raté, NVS effacée pour rattraper un SSID erroné, carte
  remplacée) se remesure sur la machine, à la main, pendant une heure. Un
  `curl > config.json` la rend gratuite.
- **Régler autre part qu'au doigt.** Ajuster une carte dimmer → pression ou
  une courbe de correction bas débit à coups de `−`/`+` sur un 4,3" est une
  punition ; dans un éditeur de texte, c'est trivial.

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
  "version": 1,
  "brew": { "target_weight_g": 36.0, "target_time_s": 28, "pump_pct": 100 },
  "preinfusion": { "mode": "time", "time_s": 6, "pressure_bar": 4.0, "pump_pct": 30 },
  "rampdown": { "mode": "none", "lead_time_s": 3.0, "lead_weight_g": 4.0, "pressure_drop_bar": 1.0 },
  "purge": { "pump_pct": 100, "max_s": 20 },
  "ui": { "brightness_pct": 80, "dim_after_s": 300 },
  "calibration": {
    "flow_k_pulses_per_l": 2382, "flow_low_correction": [],
    "pressure_full_scale_bar": 10.0,
    "dimmer_to_pressure": [], "pump_stall_pct": 20, "weight_anticipation_g": 1.5
  },
  "profiles": []
}
```

`profiles` reste un tableau vide tant que la notion de profil n'existe pas :
le schéma est prévu pour, et l'ajouter ne changera pas le reste de l'objet.

### Politique radio

**Wi-Fi et BLE ne sont pas actifs en même temps quand ça compte.** Ils
partagent la même radio 2,4 GHz : les faire cohabiter pendant un shot, c'est
accepter des trous dans la pesée au moment précis où elle décide de l'arrêt.

| Moment | Wi-Fi | BLE |
| --- | --- | --- |
| Repos | actif | actif, mais **basse cadence** — on détecte la balance et on lit le poids à ~1 Hz, il n'y a rien à suivre |
| Infusion, purge | **coupé** | actif, pleine cadence |
| Fin d'infusion | réactivé | actif, basse cadence |

La coupure du Wi-Fi pendant une infusion est **délibérée et normale**, pas une
panne : l'UI l'affiche comme telle (`ui.md`, bandeau de statut). Le shot est
envoyé au réseau *après* le cycle, quand la radio est rendue — c'est déjà ce
que dit le tableau des travaux concurrents plus haut.

Le **même flux de trames** sort en USB série sur l'image factory (voir plus bas) : un seul décodeur côté Mac pour les deux transports.

---

## Sécurité

Une surpression n'est pas un problème de firmware : **c'est l'OPV qui la traite**, mécaniquement. Réglage visé **11 bar**, fonctionnement normal à **9 bar** — en usage courant l'OPV ne s'ouvre jamais, sauf sur la purge de backflush. C'est ce qui autorise le module capteurs à transporter la pression sans la comprendre.

Ce que le firmware doit garantir, c'est qu'on ne laisse pas la vanne ouverte et la pompe à fond. Trois mécanismes distincts, qui ne se remplacent pas :

### 1. Bail sur commande (~500 ms)

Chaque `SET` porte un TTL. À l'expiration, le module capteurs remet SSR à 0 et dimmer à 0. L'écran qui rampe le dimmer à 10 Hz renouvelle le bail sans y penser ; un écran **mort** coupe tout en un demi-tour de seconde.

C'est le mécanisme rapide, et le seul qui compte pendant un shot.

### 2. Présence (ping toutes les 1–2 s)

Chaque nœud tient un `last_presence`. Un `PONG` reçu **ou** un `PING` reçu comptent tous les deux : le pair est vivant. Sans rien pendant ~3 s, on émet un `PING` ; qui l'entend répond immédiatement. Le nœud qui **répond** remet aussi son propre compteur à zéro : TWAI ne boucle pas ses propres trames, c'est la réception du ping qui est le signal.

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
| `0x08` | `PING` | ↔ | vide |
| `0x09` | `PONG` | ↔ | identité + uptime |
| `0x10` | `REQSTATUS` | S → X | quoi, à quelle période |
| `0x20` | `STATUS_PRESSURE` | X → S | XDB401 brut |
| `0x21` | `STATUS_FLOW` | X → S | compteur d'impulsions |
| `0x22` | `STATUS_ACTUATORS` | X → S | état + santé |
| `0x30` | `LOG` | ↔ | code + arguments |
| `0x38` | `FLASH_CTRL` | ↔ | sous-commande |
| `0x39` | `FLASH_DATA` | ↔ | 8 octets bruts |

### Charges utiles

`SET` (0x01)

```
[0]     masque      bit0 = SSR, bit1 = dimmer
[1]     ssr         0 | 1
[2]     dimmer      0..100
[3..4]  ttl_ms      uint16, 0 = défaut (500)
[5..7]  réservé
```

`PONG` (0x09)

```
[0]     nœud        1 écran, 2 capteurs
[1..3]  version     majeure, mineure, correctif
[4..7]  uptime_s    uint32
```

`REQSTATUS` (0x10)

```
[0]     type visé   0x20 | 0x21 | 0x22
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

`STATUS_ACTUATORS` (0x22) — c'est l'accusé de réception d'un `SET`

```
[0]     ssr                 0 | 1
[1]     dimmer              0..100
[2..3]  bail restant ms     uint16
[4..5]  marche continue ms  uint16   (pour voir arriver les 60 s)
[6]     flags               bit0 verrou actif, bit1 dimmer prêt, bit2 dimmer valide (détection I2C),
                            bit3 secteur détecté côté dimmer (registre 0x20 plausible, 45-65 Hz)
[7]     réservé
```

Il n'y a pas d'acquittement séparé pour `SET` : l'écran compare ce qu'il a commandé à ce qui revient ici. Les commandes sont idempotentes, le dernier gagne ; le module n'empile pas de file de niveaux dimmer.

`LOG` (0x30)

```
[0]     code
[1]     sévérité
[2..3]  arg16
[4..7]  arg32
```

Pas de texte sur le bus. **La table de codes est générée depuis une source unique** dans `common/`, consommée à la fois par le C++ et par le décodeur Python. Sinon la table du Mac dérive de celle du firmware, et le premier message qu'on ne comprendra plus sera celui d'un crash.

Codes de départ : boot, prêt, reboot demandé ; bail expiré, présence perdue, **verrou 60 s déclenché** (arg32 = ms d'activation), commande refusée car verrouillée ; erreur I2C (arg16 = adresse), dimmer en calibration, erreur dimmer (arg16 = registre `0x02`), timeout XDB401, débitmètre muet ; début / progression / fin / échec de flash ; OTA en attente de validation, validée, rollback.

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

`POST /firmware?target=screen`, en-tête `Authorization`, corps `.bin`. Écriture de l'emplacement OTA inactif, marquage bootable, reset. Après redémarrage, la nouvelle image ne se valide **qu'une fois le ping/pong CAN reconfirmé**.

Attention à ce qu'IDF fait et ne fait pas : en `PENDING_VERIFY`, si l'application ne se valide pas, le bootloader revient en arrière **au prochain redémarrage** — mais rien ne redémarre tout seul. Il faut un temporisateur propre qui appelle l'invalidation-et-reboot, ou laisser le task watchdog frapper. Sans ça, une image qui démarre et ne se valide jamais reste en place indéfiniment.

### Flash distant (les capteurs, à travers l'écran)

Même route, `target=sensors`. L'écran ne lit pas l'image : `BEGIN`, blocs acquittés, `END`. Le module capteurs écrit l'emplacement inactif, vérifie le CRC32, bascule, remonte sa progression en `LOG` (que le WebSocket affiche), redémarre. Il doit ensuite répondre au ping avec sa nouvelle version ; l'écran attend ce pong. S'il ne vient pas, le rollback du module joue seul.

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

Calibrations, toutes en NVS côté écran : facteur K du débitmètre et correction bas débit, pleine échelle du XDB401, carte dimmer → pression, seuil de calage de la pompe, grammes d'anticipation. Un remplacement de XIAO ne fait rien perdre.

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
  injoignable en Wi-Fi — voir « Ce qu'on oublie habituellement » dans
  `firmware-implementation.md` — un AP de secours déclenché par bouton ou
  échec de connexion répété rattrape ça, contrairement au tactile qui suppose
  déjà LVGL en place). Stockage en NVS, déjà acquis.
- **Charte graphique / design de l'UI écran** (phase 6, LVGL) — à définir,
  session dédiée envisagée avec Opus.

Séquence d'implémentation : `firmware-implementation.md`.

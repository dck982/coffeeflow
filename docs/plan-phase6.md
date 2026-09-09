# Phase 6 — plan d'implémentation détaillé

Ce document découpe la phase 6 (`firmware-implementation.md`) en lots
exécutables, dans l'ordre où ils doivent être écrits, chacun avec son contenu,
ses pièges connus et **son critère de sortie vérifiable**. Il ne contient pas
de code : il dit quoi construire, dans quel ordre, et comment savoir que c'est
fini.

Documents dont il dépend, à lire avant d'écrire une ligne :

| Document | Ce qu'il fixe |
| --- | --- |
| `firmware.md` | protocole CAN, répartition écran/capteurs, API réseau, `/config`, politique radio, mise à jour |
| `ui.md` | style, cotes, `ui_model_t`, machine à états UI, cas limites, textes, réglages NVS, découpage en cinq lots |
| `firmware-implementation.md` | ordre général des phases, état réel du matériel, pièges de mise en route |
| `docs/reference/acaia-ble/README.md` | protocole Acaia (cadrage, décodage, heartbeat) |
| `docs/cablage.md` | câblage réel du banc |

**Ce plan ne couvre pas** : la calibration sur machine, les profils, le flow
control, la pré-infusion pilotée au débitmètre. Tout ça est la section
« Après » de `firmware-implementation.md`, postérieure au débranchement de
l'USB.

---

## L'idée qui structure tout : le cœur machine

La décision la plus importante de cette phase n'est pas un ordre de lots, c'est
une frontière. **Tout ce que la machine sait faire et tout ce qu'elle sait vit
derrière une interface unique, le cœur machine. LVGL en est un client, HTTP en
est un autre, et aucun des deux n'a de privilège.**

Conséquences directes, qui expliquent la moitié du découpage ci-dessous :

- La machine se teste **entièrement sans écran**, par `curl` et `coffeetool`,
  avant qu'une seule ligne de LVGL soit écrite.
- L'écran ne peut pas diverger du réseau : il n'y a pas deux vérités, il y a un
  instantané et deux façons de le regarder.
- Une règle d'éligibilité (« peut-on lancer une infusion maintenant ? ») est
  écrite **une fois**, dans le cœur. Ni `ui_home.c` ni le httpd ne la
  recalculent — ils demandent, et affichent la réponse.

Le cœur se compose de trois faces, construites dans cet ordre par les lots :

| Face | Ce que c'est | Lot |
| --- | --- | --- |
| **Sorties** | un instantané cohérent de tout ce que la machine sait | 3 |
| **Configuration** | lecture, écriture partielle validée, valeurs par défaut, bornes | 4 |
| **Actions** | ce que la machine sait faire, chacune acceptée ou refusée avec un motif | 9 |

Une quatrième face, transverse : un **flux d'événements** (transitions,
progression d'un flash, fin de shot avec son résumé, codes `LOG`), auquel le
WebSocket, l'écran de service et l'UI s'abonnent au lieu de scruter.

### Sorties

Un seul instantané, pris sous verrou, jamais lu champ par champ par un
consommateur. C'est `ui_model_t` tel que `ui.md` le définit, **étendu** avec ce
dont la page de diagnostic et la télémétrie HTTP ont besoin : valeurs brutes
(`pressure_raw`, `temperature_raw`, impulsions cumulées), âge de chaque
grandeur, compteurs d'erreur TWAI, versions et uptimes des deux nœuds,
progression d'un flash en cours. Les champs déjà documentés dans `ui.md` ne
changent pas.

Chaque grandeur porte sa **validité** et son **âge**, et la règle de péremption
de `ui.md` (douteuse au-delà de deux périodes, absente au-delà de 3 s) vit
ici — pas dans l'UI, qui n'est pas seule à en avoir besoin.

### Actions

La liste complète de ce que la machine sait faire. Chaque appel répond
**accepté** ou **refusé avec un motif** (verrou de sécurité, infusion en cours,
dimmer pas prêt, module injoignable, valeur hors bornes) ; aucun appelant n'a à
deviner s'il avait le droit.

| Action | Argument | Refusée quand |
| --- | --- | --- |
| démarrer une infusion | — | verrou, dimmer pas prêt, bus perdu, infusion ou purge en cours, flash en cours |
| arrêter l'infusion | — | aucune infusion en cours |
| purge appuyée | — (à renouveler, homme-mort) | verrou, dimmer pas prêt, bus perdu, infusion en cours |
| purge relâchée | — | — |
| tarer la balance | — | balance absente |
| fermer le résumé | — | pas dans l'état terminé |
| commande d'actionneur brute | SSR, niveau dimmer, bail | verrou, bus perdu, infusion ou purge en cours |
| démarrer un flash | cible (écran ou capteurs), image | infusion ou purge en cours, flash déjà en cours |
| oublier le réseau Wi-Fi | — | — |

La commande brute est un outil de banc et de calibration, pas un chemin
d'usage : elle porte un bail comme n'importe quel `SET`, n'est jamais
renouvelée automatiquement, et retombe donc d'elle-même.

### Configuration

Lecture de l'intégralité du réglable, écriture **partielle acceptée et validée
tout ou rien** (une seule valeur hors bornes rejette l'objet entier, en nommant
la clé fautive), valeurs par défaut, bornes exposées. Schéma, bornes et pas :
`firmware.md` pour l'objet `/config`, `ui.md` pour la table des réglages. Les
identifiants Wi-Fi n'en font pas partie, ni en lecture ni en écriture.

C'est la **même porte** pour `POST /config` et pour un appui sur `+` dans les
réglages de l'écran. Il n'existe pas de chemin d'écriture qui contourne la
validation.

---

## Ordre retenu, et pourquoi

L'ordre des six points de la phase 6 dans `firmware-implementation.md`
(réseau → HTTP → WebSocket → couche capteurs → BLE → LVGL) est un ordre de
**description**, pas de construction. Trois inversions, chacune pour une raison
précise :

1. **Un écran de service arrive tout de suite** (lot 2), bien avant l'UI. Trois
   bénéfices, et le troisième est le plus concret : la dalle cesse d'être un
   rectangle noir qui ne sert à rien pendant les trois quarts de la phase ; le
   débogage gagne un afficheur permanent (« réseau ok », « requête reçue »,
   « bus perdu ») qui ne dépend ni de l'USB ni du Wi-Fi ; et surtout le
   **conflit CH422G entre le transceiver CAN et la dalle** (voir les pièges
   plus bas) se manifeste au lot 2, sur cinquante lignes de code, au lieu du
   lot 10 au milieu de LVGL.
2. **La couche capteurs passe devant le réseau** (point 4 → lot 3). C'est elle
   qui émet le trafic `REQSTATUS` périodique, donc c'est elle qui lève la
   béquille de la phase 5 (la boucle de `PING` manuelle sans laquelle aucun
   actionneur ne tient plus de ~3 s, voir `firmware-implementation.md`, section
   SSR). Tant qu'elle n'existe pas, tout essai d'actionneur est artificiel.
   Elle est aussi la face « sorties » du cœur, dont tout le reste dépend, et
   elle se teste **avec `coffeetool` sur l'USB seul**.
3. **Le BLE passe avant LVGL** (point 5 → lot 8). Le lot 3 de `ui.md` a besoin
   de `scale_present` pour la bascule poids/temps, le lot 5 du poids réel. Or
   le BLE se valide entièrement sans écran, à travers la télémétrie HTTP
   construite au lot 5 : arriver devant LVGL avec la balance déjà décodée évite
   de déboguer deux inconnues à la fois.

Deux lots n'existent pas dans le découpage d'origine :

- **Lot 0 — geler les images factory.** L'image factory de l'écran *est* le
  pont série↔CAN actuel. Dès que la phase 6 écrit dans `screen/main/`, cette
  image n'existe plus en l'état si elle n'a pas été archivée. C'est la seule
  chose de tout ce plan dont l'oubli ne se rattrape pas.
- **Lot 9 — la face « actions » du cœur, sans écran.** Voir plus haut : c'est
  la conséquence directe de la frontière posée en tête de document.

Ordre final :

| Lot | Contenu | Point d'origine |
| --- | --- | --- |
| 0 | Images factory gelées et archivées | — |
| 1 | Restructuration de `firmware/screen/` | — |
| 2 | Écran de service : dalle, tactile, LVGL minimal | prépare 6 |
| 3 | Cœur, face sorties : couche capteurs et télémétrie | 4 |
| 4 | Wi-Fi, provisioning, cœur face configuration | 1 |
| 5 | Serveur HTTP | 2 |
| 6 | WebSocket miroir | 3 |
| 7 | OTA par le réseau (écran et capteurs) | 2 (suite) |
| 8 | Client BLE Acaia et politique radio | 5 |
| 9 | Cœur, face actions : infusion et purge, sans écran | — |
| 10 | LVGL, en cinq sous-lots | 6 |
| 11 | Gel des partitions, sortie de phase | — |

---

## Décisions prises dans ce plan

Prises ici pour ne pas l'être en cours d'implémentation. Chacune est
révisable, mais par une décision explicite, pas par dérive.

- **L'application de phase 6 garde le pont série↔CAN.** Elle ne le remplace
  pas, elle l'englobe. Coût nul (le code existe et tourne), bénéfice permanent :
  `coffeetool` sur l'USB reste le diagnostic de dernier recours, y compris
  quand le Wi-Fi est coupé — c'est-à-dire **pendant un shot**, exactement le
  moment où le WebSocket n'est pas disponible du fait de la politique radio. Il
  n'y a donc pas de « mode debug Wi-Fi pendant l'infusion » à prévoir.
- **Réinitialisation du Wi-Fi : par l'interface, pas par le bus.** Un bouton
  *réinitialiser le réseau* dans les réglages efface l'espace de noms Wi-Fi de
  la NVS et remet l'écran en point d'accès, en attente d'une connexion. C'est
  une action du cœur comme une autre, donc disponible aussi en HTTP dès le
  lot 5 — soit bien avant que l'écran de réglages existe. **Aucun nouveau
  message CAN n'est ajouté** : l'image factory reste ce qu'elle est, et le lot 0
  devient un simple archivage.
- **Un SSID erroné ne peut pas isoler la carte** : le repli en point d'accès se
  déclenche seul sur échec d'association répété (trois tentatives, ou aucune
  association dans les 60 s après le boot). C'est ce mécanisme, et non un
  effacement manuel, qui est le vrai filet ; le bouton n'est que le chemin
  volontaire.
- **Répartition sur les cœurs**, conforme à `firmware.md` : LVGL, boucle
  d'infusion, tâches CAN et pont série sur le **cœur 1** ; Wi-Fi, httpd et pile
  BLE sur le **cœur 0**, là où Espressif les épingle déjà.
- **Secret HTTP et mot de passe du point d'accès** vivent dans un même en-tête
  non commité, avec un fichier d'exemple versionné et une entrée `.gitignore`.
  La compilation échoue avec un message explicite si l'en-tête manque, plutôt
  que de compiler avec une valeur par défaut.
- **Les endpoints de provisioning ne demandent pas le secret** : en mode point
  d'accès, le mot de passe WPA2 de l'AP est la barrière. Tout le reste de l'API
  exige `Authorization`.
- **Le format d'enregistrement de `coffeetool recorder` (JSON Lines, phase 1)
  est le format de fixture** des tests hôte de l'écran. Aucun autre format
  n'est inventé.
- **Heure obtenue par SNTP à l'association Wi-Fi, et jamais utilisée pour
  mesurer.** L'ESP32-S3 n'a pas de pile de sauvegarde : son RTC ne compte que
  sous tension, et la machine est coupée à l'interrupteur entre deux sessions.
  L'heure n'est donc pas *décalée* au démarrage, elle est **absente** — d'où
  une synchronisation unique au moment où l'association réussit, et pas de
  resynchronisation périodique (sur une session de trente minutes, la dérive de
  l'oscillateur RC du domaine RTC est de l'ordre de la dizaine de secondes, ce
  qui ne gêne rien de ce qui est fait ici).
- **Le consommateur de cette heure est identifié : `coffeetracker`**, le
  magasin de shots existant (dépôt séparé, hors de ce projet — en demander
  l'accès plutôt que de le chercher). Il
  horodate aujourd'hui **côté serveur**, à la réception, précisément parce que
  le M5Core2 qui l'alimente n'a pas d'horloge (`main.py`, `create_brew`). Ça
  marche tant que l'envoi est immédiat ; ça tombe dès que la machine met un
  shot de côté parce que le serveur ou le Wi-Fi sont indisponibles — le shot
  serait daté de sa réception, pas de son extraction. **C'est le tampon
  différé, pas l'envoi direct, qui exige une horloge à bord.** Décisions qui en
  découlent, à prendre maintenant parce qu'elles sont chères plus tard : voir
  `firmware.md`, « Envoi des shots vers `coffeetracker` ».
- **Règle qui va avec, et qui est la vraie raison de traiter le sujet
  maintenant : tout ce qui *mesure* une durée utilise l'horloge monotone**
  (`esp_timer`, cadencée par le quartz principal), **et l'heure murale ne sert
  qu'à *étiqueter*.** Chronomètre d'infusion, phases, bail, péremption des
  valeurs, temporisateur d'invalidation OTA, seuils de veille : tous monotones.
  Sans cette règle, un pas SNTP survenant en plein shot — cas normal, puisque
  l'association peut aboutir à n'importe quel moment — décale ou fait reculer
  un chronomètre en cours. C'est beaucoup moins cher de poser la règle avant
  d'écrire la boucle d'infusion qu'après.
- **Pas de réglage de luminosité, et le rétroéclairage n'est jamais modulé ni
  éteint** (décidé le 2026-09-09, `ui.md` mis à jour, section « Veille »). La
  machine est allumée par session d'une trentaine de minutes : la seule
  intensité utile est le maximum. À la place, une veille automatique à trois
  temps — pleine intensité, calque noir à ~50 % après 4 min, bloc de veille
  mobile après 30 min. Conséquence pour l'implémentation : l'absence de PWM sur
  EXIO2 du CH422G cesse d'être un problème, un seul calque LVGL à opacité fixe
  suffit, et EXIO2 est allumé une fois au boot puis oublié.

---

## Décisions qui restent à prendre (à trancher avec David)

- **Quelle taille pour la partition de données ?** Les polices générées par
  `lv_font_conv` sont des tableaux C liés dans l'image applicative, pas des
  fichiers : `assets` ne sert donc à rien pour LVGL. Mais l'envoi différé des
  shots vers `coffeetracker` (voir la décision ci-dessous) a besoin d'un
  tampon persistant, et c'est **le seul candidat**. Ordre de grandeur : un shot
  de 30 s échantillonné à 10 Hz fait quelques ko de JSON, donc quelques
  centaines de ko couvrent largement une panne de plusieurs semaines du
  serveur. La partition reste, probablement bien plus petite que 4000 ko — se
  tranche au lot 11, une fois le poids réel de l'application connu, et **une
  table de partitions ne se change plus une fois la carte en boîte**.

---

## Architecture cible de `firmware/screen/`

`main.cpp` fait aujourd'hui 615 lignes et porte le pont, le protocole et l'OTA
local. La phase 6 y ajoute le réseau, le BLE, le cœur et l'UI : il faut découper
avant d'ajouter, pas après.

```
firmware/screen/main/
  main.cpp            app_main : init matériel, création des tâches, rien d'autre
  board.h/.cpp        CH422G, GPIO, brochage, rétroéclairage — voir le piège ci-dessous
  can_link.h/.cpp     TWAI, émission/réception, dispatch protocolaire, présence
  serial_bridge.h/.cpp pont série↔CAN (code existant, déplacé tel quel)
  ota_local.h/.cpp    réception FLASH_CTRL/FLASH_DATA pour soi (code existant, déplacé)
  ota_proxy.h/.cpp    flash des capteurs à travers le CAN
  core/
    core.h            LA façade : sorties, actions, configuration, événements
    snapshot.h/.cpp   le modèle étendu + instantané sous verrou + péremption
    telemetry.cpp     REQSTATUS périodiques, réception des STATUS_*
    calibration.cpp   brut -> grandeurs physiques (logique pure, testable Mac)
    machine.cpp       machine à états infusion/purge, émission des SET (logique pure)
    config.cpp        magasin NVS, bornes, validation tout ou rien
    events.cpp        flux d'événements et abonnés
  net_wifi.h/.cpp     station, point d'accès de secours, politique radio
  net_http.h/.cpp     httpd, routes, authentification — client du cœur
  net_ws.h/.cpp       WebSocket miroir du trafic CAN
  ble_scale.h/.cpp    client GATT Acaia
  service_screen.h/.cpp écran de service du lot 2, absorbé par ui/ au lot 10
  ui/                 exactement le découpage de ui.md — client du cœur
```

Règles de dépendance, à tenir :

- **`core/core.h` est la seule chose que `net_http` et `ui/` incluent** du côté
  machine. Si un fichier d'UI inclut `can_link.h`, la frontière est franchie.
- `calibration.cpp` et `machine.cpp` **ne connaissent ni ESP-IDF ni LVGL** :
  logique pure, entrées et sorties par structures. C'est ce qui les rend
  testables sur le Mac, condition posée par `firmware.md` (« l'algorithme se
  teste sur le Mac, sans machine »).
- **Seul le cœur émet des `SET`.** L'UI et le HTTP appellent des actions.

### Pièges déjà identifiés sur cette carte

- **Le registre de sortie du CH422G est partagé.** `CAN_SEL` est EXIO5,
  `LCD_BL` EXIO2, `LCD_RST` EXIO3, `TP_RST` EXIO1, `SD_CS` EXIO4. Le pont
  actuel n'écrit que le bit CAN_SEL, ce qui maintient de fait le LCD et le
  tactile en reset et le rétroéclairage éteint — sans conséquence tant qu'il
  n'y a pas d'écran. **Toute écriture doit passer par un état maintenu en RAM**
  dans `board.cpp`, dès le lot 1 : une écriture partielle éteindra le
  transceiver CAN au moment d'allumer la dalle, et « le bus tombe quand l'écran
  s'allume » est un symptôme qui envoie chercher très loin de la cause. C'est
  la raison principale pour laquelle l'écran de service est au lot 2.
- **EXIO5 est aussi `USB_SEL`.** Le multiplexeur analogique bascule GPIO19/20
  entre USB natif et CAN. Ne pas le rebasculer.
- **`CONFIG_ESP_CONSOLE_NONE` est actif** sur ce projet : il n'y a ni `printf`
  ni `ESP_LOGx`, l'UART0/GPIO43-44 étant pris par le pont. Le débogage passe
  par les trames `LOG` du protocole, décodées par `coffeetool` — et, à partir
  du lot 2, par l'écran de service. Ne pas réactiver la console pour déboguer :
  ça casse le pont, donc l'outil de diagnostic lui-même.
- **Ce projet est en ESP-IDF v6.1**, alors que les exemples officiels Waveshare
  (`tmp/ESP32-S3-Touch-LCD-4.3/`) sont en v5.x. La phase 3 a déjà coûté une
  session entière sur une régression `uart_set_pin()` entre ces deux versions.
  Attendre le même genre d'écart sur `esp_lcd` RGB.

---

## Discipline commune à tous les lots

- **Incrémenter `common::kFirmwareVersion*` à chaque image flashée**, sans
  exception : sur une carte en boîte, le `PONG` est la seule façon de savoir ce
  qui tourne.
- **Tout nouveau code `LOG` va dans `firmware/common/codegen/log_codes.yaml`**,
  jamais en dur des deux côtés. Les codes attendus par cette phase : connexion
  et perte Wi-Fi, point d'accès démarré, réseau oublié, requête HTTP refusée
  pour authentification, configuration rejetée (clé fautive en `arg16`),
  balance connectée et perdue, infusion et purge démarrées et arrêtées avec
  leur cause.
- **À partir du lot 2, l'écran se flashe par OTA**, comme les capteurs depuis la
  phase 5 — c'est la répétition générale de la vie en boîte. L'USB reste
  disponible en secours, mais l'utiliser par confort fait perdre le bénéfice de
  l'exercice.
- **Un lot n'est pas fini tant que son critère de sortie n'a pas été observé
  sur le vrai matériel**, et le résultat est consigné dans
  `firmware-implementation.md` comme pour les phases précédentes.
- Rappels de mise en route (ports série instables, `PYTHONUNBUFFERED`,
  `coffeetool` invoqué depuis `firmware/tools/`, un seul process par port) :
  checklist en fin de `firmware-implementation.md`. Ne pas les redécouvrir.

---

## Lot 0 — Geler et archiver les images factory

**Objectif :** rendre les deux cartes récupérables avant de toucher à quoi que
ce soit. Aucun code à écrire — c'est justement pour ça qu'il est facile de
l'oublier, et son oubli est le seul irréversible de ce plan.

Contenu : construire les images factory des deux cartes dans leur état actuel
(écran = pont série↔CAN prouvé en barrière C, capteurs = application phase 5),
les flasher en USB **dans la partition `factory`**, vérifier qu'elles démarrent
et se pinguent, puis les archiver dans le dépôt avec leur numéro de version, la
date, et une note disant comment les reflasher.

**Critère de sortie :** les deux `.bin` sont dans le dépôt avec leur version ;
un `PING`/`PONG` passe entre les deux cartes démarrées sur leur partition
`factory` ; la procédure de reflash est écrite et a été suivie une fois.

---

## Lot 1 — Restructurer `firmware/screen/`

**Objectif :** découper `main.cpp` **sans changer un seul comportement**, pour
que les lots suivants ajoutent au lieu d'entasser.

Contenu : déplacer le code existant dans les fichiers de l'arborescence cible
(`board`, `can_link`, `serial_bridge`, `ota_local`), `main.cpp` ne gardant
qu'`app_main`. Introduire l'état CH422G maintenu en RAM dans `board`. Épingler
explicitement les tâches sur le cœur 1. Poser `core/core.h` avec ses trois
faces, même si elles sont encore vides — c'est le contrat que les lots suivants
remplissent.

**Critère de sortie :** aucune régression, prouvée en rejouant les essais de la
barrière C déjà passés — `PING`/`PONG` à travers le pont, un flash OTA de
`sensors` par le CAN, un flash OTA de `screen` par lui-même. Si l'un des trois
échoue, c'est la restructuration qui est en cause, pas le lot suivant.

---

## Lot 2 — Écran de service

**Objectif :** que la dalle serve à quelque chose dès maintenant, et que le
conflit CH422G se révèle sur cinquante lignes plutôt qu'au milieu de LVGL.

Contenu :

1. `esp_lcd` RGB 800 × 480 + GT911 + `esp_lvgl_port` sous IDF v6.1, **bounce
   buffer activé** (`ui.md` : sans lui le framebuffer en PSRAM déchire dès
   qu'une tâche prend le bus).
2. Allumage de la dalle **par l'état CH422G maintenu du lot 1**, jamais par une
   écriture directe du registre.
3. Un affichage de service, volontairement laid et utile : version de l'image,
   état du bus CAN, état du réseau, adresse IP, et **les cinq derniers
   événements** du flux du cœur, les plus récents en haut. Police `montserrat`
   intégrée à LVGL, aucun jeton de style, aucune cote de `ui.md` — ce n'est pas
   un brouillon de l'interface, c'est une console.
4. Abonnement au flux d'événements du cœur (encore quasi vide à ce stade : boot,
   présence CAN perdue et retrouvée). Chaque lot suivant l'enrichit sans rien
   changer ici.
5. Un appui n'importe où affiche les coordonnées touchées pendant une seconde —
   c'est la vérification du tactile, et elle reste utile ensuite.

**Piège :** si les API `esp_lcd` RGB ont trop bougé entre v5.x et v6.1 et que ce
lot s'enlise, l'isoler dans un projet jetable `firmware/screen-lcd-test/`, sur
le précédent de `firmware/dimmer-test/` en phase 5, plutôt que de laisser
`screen/` dans un état non fonctionnel. Revenir dans `screen/` une fois la
séquence de bring-up connue.

**Critère de sortie :** l'écran affiche sa version au boot et reste stable
plusieurs minutes ; **le bus CAN continue de fonctionner dalle allumée**
(`PING`/`PONG` et flash OTA revérifiés dans cet état) — c'est le vrai objet du
lot ; débrancher le câble CAN fait apparaître la ligne d'événement
correspondante à l'écran en moins de 3 s ; un appui affiche des coordonnées
cohérentes avec le point touché.

---

## Lot 3 — Cœur, face sorties : couche capteurs et télémétrie

**Objectif :** l'écran demande, reçoit et interprète la télémétrie tout seul.
C'est le lot qui lève la béquille de la phase 5.

Contenu :

1. Émission des `REQSTATUS` aux périodes de la table de `ui.md` (repos 500 /
   1000 / 1000 ms ; infusion et purge 100 / 100 / 200 ms ; plein écran OTA :
   tout à 0), réémises périodiquement pour survivre à un redémarrage des
   capteurs.
2. Réception des trois `STATUS_*`, remplissage de l'instantané, horodatage de
   chaque grandeur.
3. **Péremption** telle que `ui.md` la définit, dans le cœur et non dans l'UI.
4. `pressure_valid` à faux (bit0 de `STATUS_PRESSURE`) invalide **pression et
   température ensemble** : même capteur.
5. Calibration : facteur K et impulsions cumulées → volume et débit, pleine
   échelle → bars, brut → degrés. Valeurs par défaut de `firmware.md`,
   remplacées par la NVS au lot 4. Le débit se dérive du compteur cumulé et de
   l'horodatage du dernier front, **jamais d'un delta de trames** (une trame
   perdue ne doit rien coûter).
6. `sensors_alive`, versions et uptimes des deux nœuds, compteurs d'erreur TWAI
   remontés depuis les trames `LOG`.
7. L'écran de service affiche désormais quelques grandeurs vivantes : c'est le
   premier retour visuel direct de ce lot.

**Pièges :**

- La résolution du débitmètre impose une fenêtre de moyennage (≈ 4 s à
  1 ml/s, `firmware.md`). Un débit instantané calculé sur deux impulsions
  sauterait d'un facteur deux : c'est un totaliseur, pas un débitmètre temps
  réel.
- Le trafic périodique entretient la présence côté capteurs mais **ne descend
  jamais sous 1 Hz**, y compris écran en veille (`ui.md`), sinon le cycle
  `PRESENCE_LOST` revient.

**Critère de sortie :** `coffeetool monitor` sur l'USB montre le trafic
`REQSTATUS`/`STATUS_*` périodique émis par l'écran seul ; un `SET ssr=1
ttl_ms=30000` envoyé à la main tient **les 30 s complètes sans boucle de `PING`
manuelle** ; débrancher le XDB401 fait passer pression et température à
invalides en moins de 3 s, à l'écran de service comme dans l'instantané, et les
fait revenir au rebranchement.

---

## Lot 4 — Wi-Fi, provisioning, cœur face configuration

**Objectif :** l'écran est sur le réseau, et sa configuration est persistante et
validée.

Contenu :

1. Station Wi-Fi, identifiants en NVS (espace de noms dédié, exclu de
   `/config`), reconnexion automatique avec temporisation croissante.
2. Point d'accès de secours au déclencheur décidé plus haut, avec une page
   d'accueil servie en HTTP proposant les réseaux vus et un formulaire. Après
   enregistrement : bascule en station, et retour à l'AP si l'association
   échoue de nouveau.
3. **Action « oublier le réseau »** du cœur : effacement de l'espace de noms
   Wi-Fi, retour immédiat en point d'accès. C'est ce que le bouton
   *réinitialiser le réseau* des réglages appellera au lot 10, et ce que le
   HTTP expose dès le lot 5.
4. Magasin de configuration : clés et bornes de la table de `ui.md` (espace de
   noms `ui`) et calibrations de `firmware.md` (espace de noms `cal`), valeurs
   par défaut à la première ouverture, validation tout ou rien.
5. En-tête de secrets non commité, fichier d'exemple versionné, entrée
   `.gitignore`, échec de compilation explicite s'il manque.
6. **SNTP à l'association**, fuseau `CET-1CEST,M3.5.0,M10.5.0/3` (Suisse, avec
   les règles d'heure d'été) figé dans le firmware — une machine, un lieu, et un
   fuseau réglable serait une chaîne arbitraire à valider pour rien. Un
   indicateur « heure connue » dans l'instantané, à faux tant qu'aucune
   synchronisation n'a abouti : **rien n'affiche ni n'horodate avec une heure
   fausse**, l'absence se voit plutôt qu'elle ne se devine.
7. L'écran de service affiche l'état réseau, l'adresse IP et l'heure une fois
   connue — ce qui est précisément le moment où il commence à payer.

**Piège :** le mot de passe Wi-Fi ne doit apparaître ni dans un `LOG`, ni dans
une réponse HTTP, ni dans `/config`, ni à l'écran de service. Une sauvegarde de
configuration ne doit jamais contenir un mot de passe en clair (`firmware.md`).

**Critère de sortie :** carte dont le Wi-Fi n'a jamais été configuré → l'AP
monte tout seul, le formulaire enregistre le réseau, l'écran s'y associe après
redémarrage et le reste après une coupure d'alimentation ; un SSID
volontairement faux ramène l'AP au bout du délai prévu ; l'action « oublier le
réseau » ramène l'AP immédiatement ; les réglages écrits survivent à une
coupure.

---

## Lot 5 — Serveur HTTP

**Objectif :** le cœur devient pilotable et lisible depuis le LAN. Le httpd
n'est qu'un traducteur : **il ne contient aucune règle métier.**

Contenu, avec authentification `Authorization` sur tout sauf le provisioning :

| Route | Face du cœur |
| --- | --- |
| `GET /telemetry` | sorties : instantané complet, grandeurs calibrées, valeurs brutes, validités, présences, versions |
| `GET /config` | configuration : l'intégralité du réglable, `version` en tête |
| `POST /config` | configuration : remplacement partiel, validation tout ou rien, `400` avec la clé fautive, `409` pendant une infusion ou une purge |
| `POST /action` | actions : la table du cœur, la réponse portant le motif en cas de refus |

Le schéma de `/config` est celui de `firmware.md`, `profiles` restant un tableau
vide. Une version de schéma inconnue est refusée, jamais interprétée.

Au lot 5, la seule action réellement disponible est la commande d'actionneur
brute (plus « oublier le réseau ») ; les autres apparaissent au lot 9 **sans
changer la route** — c'est l'intérêt d'exposer la face, pas une liste
d'endpoints ad hoc.

**Critère de sortie :** un aller-retour `curl` complet — sauvegarde de la
configuration dans un fichier, modification d'une valeur, restauration
intégrale, valeur hors bornes refusée avec le nom de la clé, télémétrie
cohérente avec ce que `coffeetool monitor` montre au même instant sur l'USB, et
une action refusée renvoyant un motif lisible.

---

## Lot 6 — WebSocket miroir

**Objectif :** le sniffer CAN existe sans câble USB, avec **le même décodeur**
côté Mac.

Contenu : une route WebSocket diffusant le PDU nu de chaque trame CAN observée,
dans les deux sens, au format déjà défini (`common/framing.hpp` : PDU nu en
WebSocket, COBS uniquement sur le fil série). Plusieurs clients simultanés ; un
client lent est déconnecté plutôt que de bloquer la tâche protocole.

**Piège :** `WebSocketTransport` de `coffeetool` (phase 1) n'a **jamais parlé à
un vrai serveur**. Attendre des surprises côté client autant que côté firmware,
et se servir des vecteurs de test croisés déjà en place
(`test_framing.py::test_golden_vectors_from_cpp`) pour trancher qui a tort.

**Critère de sortie :** `coffeetool monitor` sur le WebSocket produit la même
trace que le même `coffeetool monitor` sur l'USB, lancés en parallèle sur la
même période.

---

## Lot 7 — OTA par le réseau

**Objectif :** le flash passe du câble série au réseau, pour les deux cartes.
C'est la répétition de ce que la phase 7 refera en boîte.

Contenu :

1. `POST /firmware?target=screen` : écriture de l'emplacement OTA inactif,
   réutilisant la logique de `ota_local` déjà éprouvée (effacement avant
   acquittement, `PENDING_VERIFY`, temporisateur d'invalidation 30 s,
   validation conditionnée à un `PING`/`PONG` réel).
2. `POST /firmware?target=sensors` : l'écran devient l'émetteur de la séquence
   `BEGIN` / blocs de 2 ko acquittés / `END` sur le CAN — portage côté firmware
   de ce que fait `flash_client.py`, avec les deux corrections apprises en
   phase 4 : **attendre le bon `FLASH_CTRL` en ignorant les `LOG` qui le
   précèdent**, et **espacer les trames `FLASH_DATA`** au lieu de les envoyer
   en rafale.
3. Progression dans le flux d'événements du cœur, donc visible d'un coup sur le
   WebSocket **et** sur l'écran de service : c'est le premier lot où l'écran
   sert vraiment pendant une opération longue.
4. Le streaming `REQSTATUS` est arrêté et les actionneurs coupés pendant un
   flash (`firmware.md`).
5. Ajouter la cible réseau à `coffeetool flash`, à côté du transport série.

**Piège connu, non résolu :** un bloc rejoué après échec CRC16 n'est pas
distinguable du bloc suivant côté récepteur (limite documentée en phase 4 dans
`sensors/main/main.cpp`). Le CRC32 global du `END` empêche de flasher une image
fausse, mais le rattrapage bloc par bloc annoncé par le protocole n'est pas
réel. Ce lot est le bon moment pour le corriger si le rejeu se produit sur le
vrai bus ; sinon, laisser en l'état et ne pas le redécouvrir plus tard.

**Critère de sortie :** les quatre essais de la phase 4 rejoués **par le
réseau** — image saine sur l'écran, image saine sur les capteurs, image
sciemment cassée avec rollback, coupure en plein transfert sans mouvement
d'`otadata`.

---

## Lot 8 — Client BLE Acaia et politique radio

**Objectif :** le poids arrive, et les deux radios cohabitent selon la règle.

Contenu :

1. Client GATT natif ESP-IDF vers la Lunar. Le protocole applicatif est décrit
   en détail dans `docs/reference/acaia-ble/README.md` : cadrage `0xEF 0xDD`,
   sous-messages du `cmd 12`, décodage du poids avec son diviseur et son bit de
   signe, et surtout le **battement toutes les 2,5 s sans lequel la balance
   cesse d'émettre**. Le code de référence est Arduino-ESP32 : c'est la logique
   qui se porte, pas le code BLE bas niveau.
2. Deux points explicitement notés comme non vérifiés dans cette référence, à
   confirmer sur la vraie balance avant de s'en servir : les UUID exacts du
   service et de la caractéristique, et le bit de signe du poids (à valider par
   un poids négatif après tare).
3. `scale_present` selon `ui.md` : connecté **et** pesée reçue depuis moins de
   2 s. L'action « tarer » du cœur devient disponible.
4. Cadence adaptative : ~1 Hz au repos, pleine cadence pendant une infusion ou
   une purge.
5. Politique radio de `firmware.md` : Wi-Fi arrêté au départ d'une infusion,
   relancé au retour au repos. Le pont USB reste actif pendant ce temps.

**Piège :** la coupure du Wi-Fi ferme les connexions WebSocket et HTTP en
cours. C'est délibéré, ce n'est pas une panne, et l'UI l'affichera comme telle
(icône atténuée, ni rouge ni message — `ui.md`). Un client réseau se reconnecte
après le shot.

**Critère de sortie :** `GET /telemetry` montre le poids suivant une charge
posée sur le plateau ; `scale_present` retombe en moins de 2 s quand la balance
est éteinte et revient au rallumage ; une session de plusieurs minutes sans
perte de notifications (le battement fait son travail).

---

## Lot 9 — Cœur, face actions : infusion et purge, sans écran

**Objectif :** la machine sait faire un shot et une purge, pilotée par `curl`,
avant qu'une interface existe. C'est le lot qui rend le lot 10 facile.

Contenu :

1. Machine à états de `ui.md` (repos, infusion, purge, terminé, feuille, plein
   écran) en logique pure, sans dépendance IDF, pilotée par les actions du cœur
   et par l'instantané.
2. Émission des `SET` avec bail, renouvelés à 10 Hz pendant une infusion ou une
   purge. C'est le seul composant qui émet des `SET`.
3. Phases : pré-infusion (temps fixe ou attente de pression), extraction,
   ramp-down (aucune, temps, poids, ou chute de pression) — les quatre
   stratégies de la table de réglages de `ui.md`. Leur **réglage fin** est une
   affaire de calibration, pas de ce lot.
4. Arrêt : cible de poids atteinte moins l'anticipation, ou cible de temps, ou
   arrêt manuel.
5. **Tous les cas limites de la table de `ui.md`**, qui sont pour la plupart des
   cas de cette machine et non de l'interface : perte de balance en cours
   d'infusion au poids → arrêt immédiat ; recul de poids de plus de 5 g →
   traité comme une perte ; tare non confirmée après 1,5 s → départ quand même
   avec un delta depuis la valeur lue ; `goal` gelé au démarrage du shot ;
   `dimmer_ready` qui retombe en cours → on continue.
6. Purge en homme-mort : coule tant que l'action « purge appuyée » est
   renouvelée, plafonnée à la durée maximale configurée.
7. Entrée en plein écran (verrou, bus perdu, flash) → **coupure des actionneurs
   avant tout affichage** (`ui.md`).
8. Résumé de fin de shot publié dans le flux d'événements : poids final, durée,
   débit moyen, et **la date du shot si l'heure est connue**, prise au *départ*
   du shot et non à la publication (`firmware.md`, « Envoi des shots vers
   `coffeetracker` »). L'envoi lui-même reste hors phase 6 ; l'horodatage coûte
   un champ maintenant et évite d'avoir une collection de shots inexploitables
   le jour où on les ramassera.
9. **Toutes les durées de cette machine à états sont monotones**, sans
   exception : un pas SNTP ne doit jamais pouvoir décaler un chronomètre
   d'infusion en cours.
10. Les actions correspondantes deviennent disponibles sur `POST /action`, sans
    nouvelle route.

**Tests hôte**, dans `firmware/screen/test/`, sur le modèle de
`common/test/run_tests.sh` : rejeu d'enregistrements JSON Lines de
`coffeetool recorder` dans la machine, vérification des transitions et des `SET`
produits. C'est là que les cas limites se testent — les provoquer à la main sur
la machine est lent et parfois impossible (perte de balance en plein shot).

**Critère de sortie :** un shot au temps complet exécuté depuis un `curl`, sur
charge de test, avec la trace `coffeetool` montrant les phases attendues et
l'arrêt à l'échéance, et le déroulé visible sur l'écran de service ; une purge
qui s'arrête au relâchement **et** au plafond ; les cas limites couverts par les
tests hôte, au vert.

---

## Lot 10 — LVGL

**Objectif :** l'interface de `ui.md`, sur une machine qui fonctionne déjà et
qu'on a déjà pilotée autrement.

Le découpage en cinq sous-lots et leurs critères de sortie sont **déjà écrits**
dans `ui.md`, section « Ordre d'écriture, et critère de sortie de chaque lot ».
Les suivre tels quels : dalle et jetons ; bandeau L0 et repos statique ; modèle
vivant et page de diagnostic ; interaction, NVS et purge ; infusion.

Ce que ce plan ajoute :

- Le sous-lot 1 est **déjà fait pour sa moitié matérielle** par le lot 2 : il ne
  reste que `ui_theme.h`, les polices générées et les styles partagés.
- Le sous-lot 3 (modèle vivant) est **déjà fait** par le lot 3 : il ne reste que
  le rendu. Idem le sous-lot 4 pour la NVS (lot 4) et la purge (lot 9), et le
  sous-lot 5 pour toute la logique d'infusion (lot 9). C'est le bénéfice direct
  de la frontière posée en tête de document : les sous-lots de `ui.md` se
  réduisent à de l'affichage et à des appels d'actions.
- **Si du calcul apparaît dans un `ui_*.c`, c'est qu'il est au mauvais
  endroit.** Y compris une règle du type « griser le bouton si le dimmer n'est
  pas prêt » : c'est le motif de refus renvoyé par l'action, pas une condition
  réécrite dans l'UI.
- Ajouter le bouton *réinitialiser le réseau* dans les réglages, qui appelle
  l'action posée au lot 4. Prévoir une confirmation : c'est la seule action de
  l'interface qui puisse rendre l'écran injoignable en Wi-Fi.
- La veille (`ui.md`, section « Veille ») appartient au sous-lot 4 : calque
  d'atténuation, bloc de veille mobile, réveil sans action. Elle ne dépend
  d'aucun réglage tactile.
- L'écran de service du lot 2 est **retiré ou relégué** ici, une fois l'UI en
  place. La page de diagnostic de `ui.md` en est le successeur légitime ; le
  garder en doublon accessible par un geste caché est une option, pas une
  obligation.

---

## Lot 11 — Gel des partitions et sortie de phase

**Objectif :** rendre la mise en boîte possible sans regret.

Contenu :

1. Mesurer la taille réelle de l'image applicative complète (LVGL, polices,
   BLE, Wi-Fi, httpd) et la comparer aux 3500 ko d'un emplacement OTA.
2. Trancher le sort de la partition `assets` (voir plus haut).
3. Ajuster la table de partitions **si nécessaire, maintenant** — après, la
   carte est en façade.
4. Reflasher les deux images factory dans leur partition et revérifier qu'elles
   démarrent et se pinguent : c'est le filet de la phase 7, il doit être éprouvé
   avant d'en avoir besoin.
5. Consigner l'état final dans `firmware-implementation.md`.

**Critère de sortie :** la sortie de phase 6 telle que
`firmware-implementation.md` la formule — l'écran se flashe et flashe les
capteurs par le réseau, et l'outil Mac voit tout par WebSocket — plus une marge
connue sur les emplacements OTA.

---

## Risques, par ordre décroissant

| Risque | Où il se manifeste | Ce qui le désamorce |
| --- | --- | --- |
| Registre CH422G partagé : le bus CAN tombe en allumant la dalle | lot 2 | état maintenu en RAM dès le lot 1, et l'écran de service placé tôt exprès |
| Écarts d'API `esp_lcd` entre IDF v5.x et v6.1 | lot 2 | repli sur un projet jetable si ça s'enlise, sans laisser `screen/` cassé |
| Cohabitation Wi-Fi / BLE dégradant la pesée | lot 8 | la politique radio de `firmware.md`, appliquée telle quelle |
| Bande passante du panneau RGB : déchirement | lots 2 et 10 | bounce buffer, animations locales, rafraîchissement plafonné à 10 Hz |
| UUID et bit de signe Acaia non vérifiés | lot 8 | les confirmer sur la vraie balance avant d'écrire l'algorithme au poids |
| Frontière du cœur qui fuit (règles recopiées dans l'UI ou le httpd) | lots 5 et 10 | `core.h` seule inclusion autorisée ; toute règle d'éligibilité est un motif de refus |
| Emplacements OTA trop justes | lot 11 | mesurer avant de fermer les boîtiers, pas après |
| `WebSocketTransport` jamais exercé | lot 6 | vecteurs de test croisés déjà en place depuis la phase 1 |

---

## Ce qui reste explicitement hors phase 6

Pour mémoire, et pour résister à la tentation de les avancer : calibration sur
la machine (facteur K, seuil OPV, point de décrochage, carte dimmer → pression),
profils, flow control, pré-infusion pilotée au débitmètre, graphe temps réel,
**envoi des shots vers `coffeetracker`** (dont seules les décisions de schéma
et d'horodatage sont prises ici — voir `firmware.md`).
Tout ça arrive **après le débranchement de l'USB**, par OTA, sur une machine
qui marche — c'est le principe qui ordonne tout le projet.

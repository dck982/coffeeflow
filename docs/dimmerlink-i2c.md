# DimmerLink — registres I2C

Référence des registres I2C du contrôleur RBDimmer/DimmerLink (module intégré,
pas la variante deux cartes avec broches Z-C/DIM exposées — voir
`docs/cablage.md`, "Câble du Digmesa (R2)" et le tableau des ports en tête de
`docs/firmware.md`). Source : doc officielle du fabricant, clonée dans
`tmp/DimmerLink/` (non versionné dans ce dépôt) —
<https://github.com/robotdyn-dimmer/DimmerLink/blob/main/04_I2C_COMMUNICATION.md>
si `tmp/` a été nettoyé depuis. Confirmée et corrigée contre le vrai matériel
en bring-up le 2026-09-09 (voir `docs/firmware-implementation.md`, phase 5,
section dimmer, pour le récit complet du diagnostic).

Le firmware (`firmware/sensors/main/main.cpp`) n'utilise aujourd'hui que
`DIM0_LEVEL` (écriture) et `STATUS`/`ERROR` (lecture de santé). Ce document
couvre aussi ce qui n'est **pas encore** câblé, pour ne pas repartir de zéro
si on veut l'ajouter plus tard.

## Adresse et paramètres bus

| Paramètre | Valeur |
| --- | --- |
| Adresse I2C (7 bits) | `0x50` |
| Vitesse | 100 kHz (Standard Mode) |
| Pull-up | 4,7 kΩ sur SDA et SCL — déjà fournies par le XDB401 sur ce bus partagé, voir `docs/firmware.md` §"I2C partagé" |

Transaction de lecture : **combinée**, pas deux transactions séparées avec
STOP entre les deux (write pointeur de registre + read en repeated-start,
comme `i2c_master_transmit_receive()` côté ESP-IDF ou `readfrom_mem()` côté
MicroPython). Deux transactions séparées ont donné une lecture incohérente
sur le vrai module (valeur figée à `0x32` en continu, y compris au repos) —
voir le récit du bring-up dans `docs/firmware-implementation.md`.

## Carte des registres

| Adresse | Nom | R/W | Description |
| --- | --- | --- | --- |
| `0x00` | STATUS | R | État du device — voir détail ci-dessous |
| `0x01` | COMMAND | W | Commandes de contrôle |
| `0x02` | ERROR | R | Dernier code d'erreur |
| `0x03` | VERSION | R | Version firmware |
| `0x10` | DIM0_LEVEL | R/W | Luminosité dimmer 0 (0-100 %) — **la seule écriture utilisée par ce firmware** |
| `0x11` | DIM0_CURVE | R/W | Courbe de dimming — **pas encore utilisé, voir plus bas** |
| `0x20` | AC_FREQ | R | Fréquence secteur (50/60 Hz) — **peu fiable sur notre module, voir mise en garde plus bas** |
| `0x21` | AC_PERIOD_L | R | Période secteur, octet bas |
| `0x22` | AC_PERIOD_H | R | Période secteur, octet haut |
| `0x23` | CALIBRATION | R | État de calibration — **valeur observée non conforme à la doc, voir plus bas** |
| `0x30` | I2C_ADDRESS | R/W | Adresse I2C courante (`0x08`-`0x77`) — **ne jamais écrire depuis ce firmware**, changerait l'adresse du module en direct |

### STATUS (`0x00`)

| Bit | Nom | Description |
| --- | --- | --- |
| 0 | READY | 1 = device prêt |
| 1 | ERROR | 1 = dernière opération en échec |
| 2-7 | — | Réservé |

C'est le registre que le firmware utilise comme **source de vérité** pour
`g_dimmer_ready`/`g_dimmer_error_active` (voir `read_dimmer_health()`) — pas
`ERROR` seul, dont les valeurs observées en pratique ne correspondent pas
toujours à la table documentée (voir plus bas).

### COMMAND (`0x01`, écriture seule)

| Valeur | Commande | Description |
| --- | --- | --- |
| `0x00` | NOP | Rien |
| `0x01` | RESET | Reset logiciel |
| `0x02` | RECALIBRATE | Relance la calibration de fréquence |
| `0x03` | SWITCH_UART | Bascule l'interface en UART — **écrit en EEPROM, ne jamais appeler depuis ce firmware** (voir `docs/firmware.md`) |

**`RECALIBRATE` n'est volontairement pas appelé automatiquement au boot du
firmware `sensors`** : le module DimmerLink lance déjà sa propre calibration
à sa mise sous tension secteur, indépendamment de l'ESP32. Appeler
`RECALIBRATE` en même temps risquerait de relancer une calibration déjà en
cours plutôt que de la laisser aboutir (race condition identifiée le
2026-09-09, avant qu'un test de power-cycle complet confirme si la
convergence naturelle suffit sans intervention). Voir
`tick_dimmer_health()` : le firmware attend une marge généreuse et ne tente
`RECALIBRATE` qu'une fois, en repli, si le module reste `READY=0` trop
longtemps après notre propre démarrage.

### ERROR (`0x02`)

Table documentée par le fabricant :

| Code | Nom | Description |
| --- | --- | --- |
| `0x00` | OK | Pas d'erreur |
| `0xF9` | ERR_SYNTAX | Adresse de registre invalide |
| `0xFC` | ERR_NOT_READY | Erreur d'écriture EEPROM |
| `0xFD` | ERR_INDEX | Index de dimmer invalide |
| `0xFE` | ERR_PARAM | Valeur de paramètre invalide |

**Observé en pratique sur notre module : `0x01`, une valeur absente de cette
table.** Pas d'explication trouvée dans la doc fabricant. C'est la raison
pour laquelle le firmware ne s'appuie plus sur ce registre comme critère de
"prêt" — voir STATUS ci-dessus.

### DIM0_LEVEL (`0x10`)

Lecture : niveau courant (0-100). Écriture : fixe le niveau (0-100).
Confirmé fiable sur le vrai matériel, à tous les paliers testés
(0/15/30/50/75/100 %) — c'est l'écriture réellement utilisée par
`apply_dimmer()`.

**Palier bas non allumé, pas un bug** : à 15 %, la lampe de test (dimmable)
restait éteinte visuellement bien que l'écriture ait réussi — simplement en
dessous du seuil de conduction de cette charge. Confirmé en testant 30 %,
qui allume normalement. À garder en tête pour la calibration de la pompe
(seuil de calage, déjà documenté dans `docs/firmware.md` comme mesure sur la
machine, pas une constante).

### DIM0_CURVE (`0x11`) — pas encore câblé

| Valeur | Courbe | Application documentée |
| --- | --- | --- |
| `0` | LINEAR (défaut) | Universel — puissance proportionnelle au niveau |
| `1` | RMS | Lampes incandescentes/halogènes — compense la non-linéarité, 50 % niveau ≈ 50 % luminosité perçue |
| `2` | LOG | LED — épouse la perception logarithmique de l'œil |

Recommandations du fabricant par type de charge :

| Charge | Courbe recommandée |
| --- | --- |
| LED dimmable | LOG (2) |
| Lampe incandescente | RMS (1) |
| Résistance chauffante | LINEAR (0) |
| Moteur (ventilateur) | LINEAR (0) |

**Mise en garde du fabricant sur les moteurs** : moteurs universels à
balais (aspirateurs, mixeurs, perceuses) OK ; moteurs à induction/asynchrones
(la plupart des ventilateurs et pompes) **à proscrire** — ronflent et
chauffent. Notre pompe, l'**OLAB Silent Green 35 W**, n'est **ni l'un ni
l'autre** : c'est une pompe vibratoire (électroaimant + piston à ressort,
synchronisée sur le secteur), pas un moteur rotatif — la mise en garde ne
s'applique donc pas telle quelle, mais elle n'est pas non plus dans le
tableau des charges compatibles du fabricant.

**Recommandation pour la pompe vibratoire, à valider empiriquement** :
`LINEAR` (0) comme point de départ. Le rôle de `LOG` est de compenser la
perception logarithmique de l'œil humain — sans objet pour une pompe, dont
ce qui compte est la puissance électrique délivrée à la bobine (et donc,
indirectement, le débit), pas une sensation visuelle. `LINEAR` (puissance
proportionnelle au niveau) donne la relation la plus directe et prévisible
entre le niveau demandé et la puissance réellement appliquée — un point de
départ raisonnable pour la calibration "carte dimmer → pression" déjà
prévue (`docs/firmware-implementation.md`, section "Après", point 1).
`RMS` reste une option à essayer si `LINEAR` donne une réponse trop non
linéaire en pratique (comportement électromagnétique d'une pompe vibratoire
pas forcément identique à une résistance pure). Ni l'un ni l'autre n'a été
testé sous charge réelle (pompe + circuit hydraulique) à ce stade — seulement
sur une lampe dimmable, comme prévu par la checklist phase 5 avant de
brancher le circuit hydraulique.

Pas encore exposé dans le protocole CAN (`SetPayload` n'a pas de champ
courbe) : à ajouter si on veut piloter la courbe depuis l'écran plutôt que
la figer une fois pour toutes côté firmware `sensors`.

### AC_FREQ (`0x20`), AC_PERIOD_L/H (`0x21`/`0x22`), CALIBRATION (`0x23`) — peu fiables sur ce module

Doc fabricant : `AC_FREQ` doit lire 50 ou 60 (Hz) une fois le secteur
détecté ; `CALIBRATION` doit valoir `0` (en cours) ou `1` (terminée).

**Observé en bring-up (2026-09-09), sur le vrai module, secteur présent et
dimmer fonctionnel (lampe confirmée variant de 0 à 100 %)** :
- `AC_FREQ` reste à `0` en continu, y compris avec `STATUS.READY=1` et
  `STATUS.ERROR=0`.
- `CALIBRATION` a été lu à `49` (`0x31`) — ni `0` ni `1`, hors de
  l'énumération documentée.

Pas d'explication trouvée dans la doc fabricant pour cet écart. Hypothèses
non tranchées : révision firmware du module différente de celle documentée
(le registre `VERSION`, `0x03`, lisait `0x01`), ou ces registres
informatifs spécifiques ne sont simplement pas fiables sur ce lot. Le
firmware `sensors` journalise `AC_FREQ` en `LOG DIMMER_MAINS_FREQ` (sévérité
debug) à titre indicatif seulement — **aucun flag `STATUS_ACTUATORS` n'est
piloté par ces registres**, seul `STATUS` (`0x00`) sert de source de vérité.

### VERSION (`0x03`)

Lu à `0x01` sur notre module. Pas de table de correspondance version→
comportement trouvée dans la doc fabricant.

### I2C_ADDRESS (`0x30`)

Permet de changer l'adresse I2C du module (`0x08`-`0x77`), écrite en EEPROM,
effective **immédiatement** (le module cesse de répondre sur l'ancienne
adresse dans la même transaction). Aucun intérêt à l'utiliser tant qu'un
seul dimmer est sur le bus — **ne jamais écrire ce registre** depuis ce
firmware, une erreur ici rendrait le module injoignable sans reprogrammer
son adresse en connaissant la nouvelle valeur.

## Voir aussi

- `docs/firmware.md`, section "Dimmer — pompe" — vue d'ensemble protocole/
  sécurité, pas le détail registre par registre.
- `docs/firmware-implementation.md`, phase 5 — récit complet du bring-up
  (câblage phase/neutre inversé, diagnostic RECALIBRATE, firmware de test
  dédié `firmware/dimmer-test/`).
- `docs/cablage.md`, "Câble du Digmesa (R2)" — piège de câblage similaire
  (connecteur qui inverse des broches), pour le débitmètre plutôt que le
  dimmer, mais même classe de piège.

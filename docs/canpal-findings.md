# CAN Pal (clone AliExpress) — pourquoi ça ne marchait pas

Journal de bring-up phase 2 (voir `firmware-implementation.md`). Le module capteurs
(XIAO ESP32-S3) ne parvenait pas à parler CAN à travers son transceiver TJA1051T/3 —
ni au bus réel, ni même en auto-test isolé. Deux exemplaires du même module donnaient
le même symptôme, ce qui a fini par pointer vers une cause systémique (sous-tension)
plutôt que deux puces mortes par hasard.

**Conclusion courte, si tu reprends juste ça :** le module doit être alimenté en
**5 V**, pas 3,3 V (voir « Conclusion », en bas). À 3,3 V, la puce est en sous-tension
et se coupe du bus (comportement documenté, pas un défaut). À 5 V, la puce fonctionne
mais RXD grimpe à 5 V (dangereux direct sur un GPIO 3,3 V) et TXD attend un seuil haut
que 3,3 V n'atteint pas forcément — il faut adapter les niveaux logiques sur les deux
lignes avant de rebrancher au XIAO.

---

## Configuration de base

- **Carte** : XIAO ESP32-S3, sur son Grove Shield, port **R3**.
- **Module** : transceiver CAN "CAN Pal" acheté sur AliExpress, décrit par le vendeur
  comme identique à l'Adafruit CAN Pal (produit Adafruit 5708, discontinué chez
  Adafruit) — puce **TJA1051T/3** annoncée (avec broche VIO séparée). Deux
  exemplaires achetés, soudés indépendamment.
- **Câblage tel que posé** :
  - Bornier à vis 4 points (VCC, GND, TX, RX) côté gauche → câble Grove → port R3.
  - Connecteur JST-XH 2 points (CANH, CANL) côté droit.
  - Jumper de terminaison 120 Ω activé sur le module.
  - Broche `SLNT`/`S` (mode control, broche 8 du TJA1051) **non câblée** — physiquement
    obstruée par le bornier et le JST déjà soudés, impossible d'y souder un fil sans
    tout dessouder.
  - Alimentation : **VCC = 3,3 V**, tiré du rail 3,3 V du XIAO (comme documenté à
    l'origine pour ce port).

### Pinout GPIO — bug de documentation trouvé et corrigé au passage

Le silkscreen du Grove Shield XIAO numérote ses ports en **D-number Seeed** (D0…D10),
qui **ne correspond pas** au GPIO natif de l'ESP32-S3 au-delà de D5 : D6/D7 partent sur
GPIO43/44 (réservés à l'UART0), ce qui décale tout ce qui suit.

| D-number (silkscreen) | GPIO natif |
| --- | --- |
| D8 | GPIO 7 |
| D9 | GPIO 8 |
| D10 | GPIO 9 |

`docs/firmware.md`, `docs/cablage.md` et `firmware/sensors/main/main.cpp` donnaient à
l'origine les GPIO en lisant directement les chiffres du silkscreen (ex. "R3 : TX GPIO
8, RX GPIO 9") — **déjà corrigé** dans ces fichiers (TX = GPIO 7, RX = GPIO 8 pour le
CAN Pal ; SSR = GPIO 9 ; débitmètre = GPIO 44 ; I2C = GPIO 5/6). Confirmé par
continuité directe pastille-à-pastille sur le module XIAO (D8 → pad TX du CAN Pal, D9 →
pad RX). Ce bug est indépendant du problème de sous-tension ci-dessous, mais l'a
longtemps brouillé pendant le diagnostic.

Le même type d'erreur existait sur l'autre carte du projet (M5Stack Atom S3 + Unit
CAN, GPIO 26/36 documentés à tort — les vrais pins de cet exemplaire sont TX=GPIO2,
RX=GPIO1, trouvés par tâtonnement puis validés par auto-test).

---

## Outils de test utilisés

- `firmware/can-selftest/` — projet ESP-IDF autonome, écrit pour ce diagnostic.
  Deux modes, choisis par `#define TEST_MODE_TWAI` en tête de `main.cpp` :
  - **Mode TWAI** (`TEST_MODE_TWAI 1`) : installe le driver TWAI en
    `TWAI_MODE_NO_ACK` (transmission sans attendre d'accusé de réception — utile en
    isolation, seul sur le bus), transmet une trame de test en boucle, tente de la
    relire (avec ou sans le flag `TWAI_MSG_FLAG_SELF` pour forcer la mise en file RX
    même sans ACK externe), affiche `twai_status_info_t` (compteurs d'erreur, état du
    contrôleur) à chaque itération.
  - **Mode GPIO brut** (`TEST_MODE_TWAI 0`) : bypass complet du contrôleur TWAI. Pilote
    TXD à la main comme une sortie GPIO classique (toggle haut/bas), lit RXD comme une
    entrée GPIO classique, affiche si RXD suit TXD. **Piège découvert en cours de
    route** : le TJA1051 a un timer de sécurité « TXD dominant time-out »
    (0,3–12 ms, typ. 1 ms — datasheet §6.2.1) qui coupe le transmetteur si TXD reste
    bas trop longtemps. Le premier test bit-bang tenait TXD bas pendant ~1 seconde,
    donc **un transceiver parfaitement sain aurait donné le même résultat "RXD bloqué
    haut"** que celui observé — ce test-là n'a rien prouvé. Un bit-bang valide doit
    utiliser une impulsion TXD basse **< 300 µs**, lire RXD pendant l'impulsion, puis
    relâcher TXD.
  - Sélection de la carte testée par `#define BOARD_XIAO_SENSORS` /
    `#define BOARD_ATOM_CANMON` en tête de fichier.
- `firmware/can-monitor/` — moniteur CAN passif sur l'Atom S3 (voir
  `firmware-implementation.md`, phase 2), utilisé comme sonde côté bus pour les essais
  de bout en bout.

---

## Tests effectués et résultats

1. **Bring-up initial, câble CAN branché entre XIAO et Atom** — silence total : le
   XIAO tente d'émettre (`PING` périodique) et time-out systématiquement, l'Atom ne
   reçoit rien. Premier signe que quelque chose ne va pas au niveau transceiver/câblage.

2. **Auto-test TWAI isolé (câble débranché), NO_ACK, sur l'Atom** — après correction
   des GPIO réels (TX=2, RX=1, trouvés par tâtonnement) : **PASS propre et répété**,
   compteurs d'erreur à zéro, état `RUNNING`. Confirme que la méthodologie de test
   (TWAI NO_ACK + flag SELF, isolé) sait distinguer un transceiver qui marche d'un qui
   ne marche pas — sert de témoin positif pour la suite.

3. **Auto-test TWAI isolé sur le XIAO**, avec les GPIO confirmés corrects par
   continuité (TX=7, RX=8) — échec : `bus_error_count` qui grimpe, `tx_error_counter`
   à 128, bascule en `BUS_OFF` en quelques trames. Swap TX/RX testé : même résultat.
   Avec le flag `TWAI_MSG_FLAG_SELF` en plus : même résultat (les erreurs de bit
   détectées pendant la transmission elle-même ne sont pas contournées par ce flag).

4. **Test GPIO brut (bit-bang 1 Hz, invalide comme expliqué plus haut) sur le premier
   module CAN Pal** — RXD bloqué en permanence à HIGH, ne suit jamais TXD quand il
   passe bas. Swap TX/RX : même résultat.

5. **Jumper direct GPIO7↔GPIO8, CAN Pal totalement retiré du circuit** — RXD suit
   TXD parfaitement, à chaque transition. **Innocente définitivement les GPIO du XIAO
   et le firmware de test.**

6. **Deuxième module CAN Pal, soudé indépendamment, même montage** — exactement le
   même symptôme que le premier (RXD bloqué haut, bit-bang invalide comme au point 4).
   Deux modules indépendants donnant un résultat identique a orienté le diagnostic
   vers une cause systémique plutôt que deux unités défectueuses par coïncidence.

7. **Vérifications d'écartement** (toutes passées, aucune n'explique le symptôme) :
   - Tension VCC/GND au bornier : 3,4 V, cohérent avec le rail 3,3 V du XIAO.
   - Continuité GND module ↔ GND XIAO (alimentation coupée) : bonne.
   - Continuité TX/RX jusqu'aux bons GPIO natifs du XIAO : bonne (voir pinout ci-dessus).
   - Résistance CANH↔CANL (alimentation coupée, rien d'autre branché) : 120 Ω, cohérent
     avec le jumper de terminaison activé, pas de court-circuit.
   - Court-circuit via la vis de fixation dans `boitier_dc` : écarté (vis enlevée,
     symptôme identique).
   - `SLNT` flottant comme cause de Silent mode : **écarté**. Le TJA1051 a un
     **pull-up interne au silicium sur S vers GND** (datasheet §6.2.2 : "pin S has an
     internal pull-down to GND [...] ensures a safe, defined state in case [...] left
     floating"), donc flottant = Normal mode par construction, pas Silent. Un contact
     bref (sonde, pas soudé) entre SLNT et GND pendant un test n'avait de toute façon
     rien changé — cohérent avec cette explication. **Inutile de dessouder le bornier
     pour câbler SLNT.**

8. **Mesures à VCC = 5 V** (alimentation depuis le pad VBUS/5V du XIAO, masse commune,
   avant tout rebranchement au GPIO) :
   - VCC↔GND : 5,16 V.
   - RX (RXD, sortie de la puce) ↔ GND, au repos : **5,16 V** — quasi = VCC.
   - TX (TXD, entrée de la puce) ↔ GND, au repos (GPIO du XIAO non branché dessus à ce
     moment) : **4,55 V** — cohérent avec le pull-up interne de TXD "vers VIO"
     (datasheet §6.2.2), et confirme que **VIO n'est pas séparément câblé en 3,3 V**
     sur ce clone : il est tiré en interne vers un niveau proche de VCC.

---

## Recherche externe (agent, sources datasheet + schéma Adafruit officiel)

Un second avis a été demandé (recherche web, datasheet NXP, schéma EagleCAD officiel
Adafruit) pour trancher entre "deux modules morts" et "défaut systémique". Résumé :

- **Datasheet NXP TJA1051, §6.2.3** (Undervoltage detection on pins VCC and VIO) :
  > Should VCC or VIO drop below their respective undervoltage detection levels, the
  > transceiver will **switch off and disengage from the bus (zero load)** until VCC
  > and VIO have recovered.

  Seuils : **Vuvd(VCC) : 3,5 V (min) – 4,5 V (max)**. VCC opérationnel nominal :
  **4,5–5,5 V**. Nos 3,4 V mesurés au bornier sont **sous le minimum** de cette
  fenêtre — l'arrêt n'est donc pas probable, il est **garanti par la spec**.
  Conséquence documentée : transmetteur désactivé, récepteur désactivé, RXD relâché →
  exactement le symptôme observé, et reproductible à l'identique sur n'importe quel
  exemplaire sain.

- **Schéma EagleCAD officiel de l'Adafruit CAN Pal**
  (`github.com/adafruit/Adafruit-CAN-Pal-PCB`) : la carte Adafruit embarque une
  **pompe de charge (IC3 = AP3602)** qui prend le `VCC` utilisateur (3,3 V typique) en
  entrée et sort un rail `5.0V` dédié, branché sur la **broche 3 (VCC) de la puce**.
  La broche **5 (VIO)** de la puce reste, elle, sur le `VCC` utilisateur (3,3 V). C'est
  la configuration prescrite par NXP (VCC=5V + VIO=3,3V pour interfacer un MCU 3,3V).
  L'Adafruit ne "tolère" pas 3,3 V en entrée : elle le **convertit** en interne.

- Le clone AliExpress a un design différent de l'Adafruit officiel (bornier 4 points +
  JST + jumper, contre bornier 3 points + header 7 broches + interrupteur à glissière
  chez Adafruit) — rien n'indique qu'il embarque la même pompe de charge. Aucun
  teardown public trouvé pour confirmer dans un sens ou l'autre ; le mode de panne
  « module TJA1051 générique alimenté en 3,3V, communication morte ou unidirectionnelle »
  revient comme un classique sur ce genre de clone.

- **Piège méthodologique confirmé** : datasheet §6.2.1, timer "TXD dominant time-out"
  (0,3–12 ms, typ. 1 ms). Le premier test bit-bang (TXD bas pendant 1 s) coupait le
  transmetteur bien avant la lecture de RXD — un transceiver sain aurait donné le même
  résultat. Ce test ne discriminait donc rien tant qu'il n'a pas été refait avec une
  impulsion courte.

---

## Conclusion

**Le module doit être alimenté en 5 V, pas 3,3 V.** À 3,3 V, la puce est en
sous-tension garantie (spec NXP), se déconnecte du bus, et produit exactement le
symptôme observé — sur n'importe quel exemplaire, ce qui explique pourquoi les deux
modules testés se comportaient identiquement sans être forcément défectueux.

À 5 V, la puce devrait fonctionner, mais deux adaptations de niveau logique sont
nécessaires avant de rebrancher les GPIO du XIAO (3,3 V) :

- **RX (RXD sortie puce → GPIO8 XIAO)** : grimpe à ~5 V, dangereux direct sur un GPIO
  3,3 V. Un **diviseur résistif simple** (ex. 1 kΩ / 2 kΩ) suffit — c'est une entrée
  numérique passive côté XIAO, pas besoin de composant actif.
- **TX (GPIO7 XIAO → TXD entrée puce)** : le seuil de reconnaissance HIGH est calé sur
  VIO (~5 V ici), donc ~3,5 V — un 3,3 V venant du XIAO en sortie push-pull classique
  risque de ne jamais être reconnu comme récessif. **Un diviseur ou une LDO ne
  résolvent pas ce sens** (on ne peut pas faire monter une tension avec un composant
  passif). Deux options :
  - Reconfigurer GPIO7 en **sortie open-drain** (`GPIO_MODE_OUTPUT_OD`) + une petite
    résistance série (220–470 Ω, valeur non calculée précisément faute de connaître la
    résistance exacte du pull-up interne du TJA1051, mais large marge à cette gamme) :
    GPIO7 tire activement à la masse pour le dominant, et c'est le **pull-up interne
    de la puce** qui remonte tout seul vers VIO pour le récessif — plus besoin de
    sortir plus que 3,3 V, la puce fait le reste.
  - Ou un **level shifter bidirectionnel actif** (module générique à base de BSS138,
    très courant et pas cher) sur TX et RX — solution plus universelle, gère les deux
    sens sans dépendre du pull-up interne exact.
- **AMS1117 / N7803-1CW (Mean Well)** : ce sont des régulateurs/convertisseurs
  d'alimentation, pas des level shifters — leur boucle de rétroaction est bien trop
  lente pour laisser passer un signal numérique qui bascule. Utiles pour générer un
  rail DC propre, pas pour translater des niveaux logiques sur TX/RX.

**Prochaine étape suggérée** (non faite à la date de cette note) : implémenter le
diviseur sur RX + l'open-drain sur TX (option la moins chère en composants), rebrancher
en 5 V, refaire l'auto-test TWAI NO_ACK + flag SELF (celui qui a déjà validé le
transceiver de l'Atom) comme oracle de validation.

---

## Suite — abandon du CAN Pal clone, passage au M5Stack Unit CAN (2026-09-08)

Plutôt que de bricoler les adaptations de niveau logique ci-dessus, remplacement du
clone AliExpress par un **M5Stack Unit CAN** (référence U085, transceiver
**CA-IS3050G isolé galvaniquement**, voir <https://docs.m5stack.com/en/unit/can>) côté
XIAO — le même module que celui déjà utilisé côté Atom pour `can-monitor`.
Alimentation confirmée au multimètre au connecteur Grove : **5 V** au repos, donc pas
de rejeu du problème de sous-tension ci-dessus (module différent, pas de VIO tiré en
interne comme sur le clone).

**Nouveau symptôme, même signature qu'avant** : premier essai avec le brochage porté
par analogie depuis le CAN Pal (TX=GPIO7/D8, RX=GPIO8/D9) — échec identique au run
initial du CAN Pal, `BUS_OFF` quasi immédiat, `tx_err=128`, `bus_err=16`, y compris en
self-test isolé (câble débranché). Mais cette fois la cause n'est **pas** une
sous-tension : c'est un **TX/RX inversé**, propre à ce module.

**Cause** : la doc officielle M5Stack de l'Unit CAN (HY2.0-4P) donne **jaune = CAN_TX,
blanc = CAN_RX** — ce sont les noms des broches du *transceiver lui-même*
(`CAN_TX`/TXD = entrée du transceiver, à driver depuis le contrôleur ; `CAN_RX`/RXD =
sortie du transceiver, à lire par le contrôleur), pas une convention "câble croisé"
comme supposé par erreur au départ. Le fil blanc (`CAN_RX`, une **sortie** du module)
était câblé sur D8/GPIO7, configuré comme **sortie** TX du contrôleur — deux sorties
en collision sur le même fil, ce qui donne exactement le symptôme observé.

**Correction** : inverser TX/RX par rapport à l'ancien brochage CAN Pal — **GPIO7 =
RX contrôleur, GPIO8 = TX contrôleur** (`firmware/sensors/main.cpp`,
`firmware/can-selftest/main.cpp`). Validé par le même auto-test TWAI NO_ACK + flag
SELF que celui qui avait servi d'oracle sur l'Atom : 18/18 PASS, `state=RUNNING`,
compteurs d'erreur à zéro. Puis validé sur le vrai bus entre XIAO et Atom
(`can-monitor`) : trafic réel reçu (`PING` périodique du XIAO, `LOG`
`TWAI_ERROR_COUNTERS` avec `rx_error=0, tx_error=0, bus_error_count=0`), aucun warning
de transmission côté XIAO.

**À retenir pour la suite** : même si l'Atom et le XIAO utilisent le même module M5Stack
(même chip, même convention de brochage), rien ne garantit qu'un brochage validé par
tâtonnement sur une carte (numéros de GPIO trouvés empiriquement, voir plus haut pour
l'Atom) se transpose tel quel sur une autre carte sans revalider — la vraie référence
qui a tranché ici, c'est la doc officielle du module (noms de broches `CAN_TX`/`CAN_RX`
au sens du transceiver), pas une analogie de câblage entre deux bring-up différents.

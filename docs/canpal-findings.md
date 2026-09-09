# CAN Pal (clone AliExpress) — pourquoi ça ne marchait pas

Journal de bring-up phase 2 (voir `firmware-implementation.md`). Le module capteurs
(XIAO ESP32-S3) ne parvient pas à parler CAN à travers son transceiver TJA1051T/3 —
ni au bus réel, ni même en auto-test isolé. Deux exemplaires du même module donnent
le même symptôme : **`SLNT` tenu à ~2,7 V** (Silent mode), pas deux puces mortes
au hasard.

**Conclusion courte, si tu reprends juste ça :** le clone **a** la pompe de charge
Adafruit (AP3602, marquage `AG7T` / `G7T`). Alimenté en **3,3 V** sur la pastille
`Vcc`, on mesure **~5 V sur la broche VCC du TJA1051** et **~3,3 V sur RXD** (VIO
suit le VCC utilisateur). C'est le régime prévu, pas une sous-tension. Le diagnostic
« il faut alimenter en 5 V / VIO collé à VCC » était une **erreur de mesure** (on
lisait le bornier, pas la puce ; puis on a alimenté le bornier en 5 V, donc VIO à
5 V). Le symptôme venait de **`SLNT` à 2,7 V** : l'émetteur est coupé (Silent).
À coller au GND (rework prévu, voir conclusion).

---

## Configuration de base

- **Carte** : XIAO ESP32-S3, sur son Grove Shield, port **R3**.
- **Module** : transceiver CAN "CAN Pal" acheté sur AliExpress, décrit par le vendeur
  comme identique à l'Adafruit CAN Pal (produit Adafruit 5708, discontinué chez
  Adafruit). Visuellement c'est le layout Adafruit (PCB noir, bornier 3 points
  L/GND/H, header 7 broches Vcc/GND/RX/TX/SLNT/CANH/CANL, switch Termination,
  TJA1051T/3 `A1051/3` au centre, SOT-23-6 `AG7T` à gauche = pompe AP3602).
  Les connecteurs Grove (bornier 4 vis + JST) sont soudés par-dessus ces pastilles.
  Deux exemplaires, même symptôme.
- **Câblage tel que posé** :
  - Bornier à vis 4 points (VCC, GND, TX, RX) côté gauche → câble Grove → port R3.
  - Connecteur JST-XH 2 points (CANH, CANL) côté droit.
  - Jumper de terminaison 120 Ω activé sur le module.
  - Broche `SLNT`/`S` (mode control, broche 8 du TJA1051) **non câblée au GND** —
    obstruée par le bornier 4 vis et le JST-XH. Mesurée à **2,7 V** (Silent).
    Rework : dessouder le XH, fil `SLNT` → vis GND du bornier CAN (plot central),
    remplacer le XH par deux fils sertis en JST SM.
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
pad RX). Confirmé au bit-bang lent : **fil blanc = D8/GPIO7** bascule 0/3,3 V ;
**fil jaune** (D9/GPIO8) reste à 3,4 V (RXD récessif). Le GPIO TX arrive bien au
pad. La boucle analogique échoue **après** TXD.

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
  - Sélection de la carte par `#define BOARD_XIAO_SENSORS` /
    `#define BOARD_ATOM_CANMON`. Sur le XIAO, `#define PINOUT_CANPAL` choisit
    TX=GPIO7/RX=GPIO8 (câblage Pal documenté) ou l'inverse (Unit CAN actuel).
  - Bit-bang **lent** (~1 Hz, 400 ms bas / 400 ms haut) ajouté pour lecture au
    multimètre (invalide pour le transceiver : time-out dominant). Sert uniquement
    à voir si le GPIO atteint le pad.
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
   - Tension VCC/GND **au bornier** : 3,4 V, cohérent avec le rail 3,3 V du XIAO.
     C'est l'**entrée** de la pompe, pas l'alim de la puce (voir point 9).
   - Continuité GND module ↔ GND XIAO (alimentation coupée) : bonne.
   - Continuité TX/RX jusqu'aux bons GPIO natifs du XIAO : bonne (voir pinout ci-dessus).
   - Résistance CANH↔CANL (alimentation coupée, rien d'autre branché) : 120 Ω, cohérent
     avec le jumper de terminaison activé, pas de court-circuit.
   - Court-circuit via la vis de fixation dans `boitier_dc` : écarté (vis enlevée,
     symptôme identique).
   - `SLNT` flottant : le TJA1051 a un **pull-down interne vers GND** (datasheet
     §6.2.2), donc flottant **devrait** être Normal. Un contact bref SLNT–GND n'avait
     rien changé — probablement parce que `SLNT` n'était pas à 0 V (voir point 12).

8. **Mesures à VCC bornier = 5 V** (pad VBUS/5V du XIAO, masse commune, GPIO
   débranchés) — **mal interprétées à l'époque** :
   - VCC↔GND : 5,16 V.
   - RX (RXD) ↔ GND, au repos : **5,16 V**.
   - TX (TXD) ↔ GND, au repos : **4,55 V** (pull-up interne de TXD vers VIO).
   On en avait conclu que VIO était collé à VCC et qu'il fallait des level shifters.
   En réalité c'est le schéma Adafruit : **VIO suit le VCC utilisateur**. Alimenter
   le bornier en 5 V met VIO (donc RXD) à 5 V. Ça ne dit rien sur le régime 3,3 V.

9. **Mesures à VCC bornier = 3,3 V, sur les pattes du TJA1051** (2026-09-08, un
   exemplaire, orientation bornier CAN en haut) — **régime Adafruit confirmé** :
   - Pastille `Vcc` : 3,3 V.
   - Patte bas-gauche du `A1051` (RXD, broche 4) : **3,3 V**.
   - Patte au-dessus (VCC puce, broche 3) : **5 V**.
   - Une patte de la pompe `AG7T` : **5 V**.
   La pompe booste. VIO = 3,3 V. Pas de sous-tension, pas besoin de level shifter
   tant qu'on reste en 3,3 V sur le bornier.

10. **Bit-bang lent TX=GPIO7 / RX=GPIO8** (2026-09-08, 3,3 V, terminaison ON, pas de
    câble CAN) : le **blanc (D8/GPIO7)** oscille 0 ↔ 3,3 V au pad — le XIAO drive
    TXD. Le **jaune** reste à **3,4 V** (un flou du multimètre en bougeant la pointe
    n'était pas du signal). GPIO8 lu par le firmware : RXD = 1 en permanence.

11. **CANH et CANL** (même run) : **0 V** vs GND. Pas de polarisation récessive
    (~2,5 V) ni de dominant. Bus débrayé, cohérent avec émetteur coupé.

12. **`SLNT` vs GND** (même run) : **2,7 V**. VIH du pin S ≈ 0,7×VIO ≈ 2,3 V —
    c'est un **HIGH**. Silent mode, transmetteur désactivé (datasheet : "pin S
    pulled high → Silent"). Le pull-down interne est **perdu** face à un tirage
    vers 3,3 V (pull-up clone, ou fuite/pont depuis `Vcc`/`TX` : `SLNT` est la
    pastille suivante du header, coincée sous le bornier 4 vis + le XH). 2,7 V
    plutôt que 3,3 V = pont faible / diviseur pull-up vs pull-down puce.

---

## Recherche externe (agent, sources datasheet + schéma Adafruit officiel)

Un second avis a été demandé (recherche web, datasheet NXP, schéma EagleCAD officiel
Adafruit) pour trancher entre "deux modules morts" et "défaut systémique". Résumé :

- **Datasheet NXP TJA1051, §6.2.3** (Undervoltage detection on pins VCC and VIO) :
  > Should VCC or VIO drop below their respective undervoltage detection levels, the
  > transceiver will **switch off and disengage from the bus (zero load)** until VCC
  > and VIO have recovered.

  Seuils : **Vuvd(VCC) : 3,5 V (min) – 4,5 V (max)**. VCC opérationnel nominal :
  **4,5–5,5 V**. Cette spec s'applique à la **broche 3 de la puce**, pas à la
  pastille `Vcc` du module. On a d'abord lu 3,4 V au bornier et conclu à une UVLO
  garantie — faux dès que la pompe sort 5 V sur la broche 3 (mesuré, point 9).

- **Schéma EagleCAD officiel de l'Adafruit CAN Pal**
  (`github.com/adafruit/Adafruit-CAN-Pal-PCB`) : la carte Adafruit embarque une
  **pompe de charge (IC3 = AP3602)** qui prend le `VCC` utilisateur (3,3 V typique) en
  entrée et sort un rail `5.0V` dédié, branché sur la **broche 3 (VCC) de la puce**.
  La broche **5 (VIO)** de la puce reste, elle, sur le `VCC` utilisateur (3,3 V). C'est
  la configuration prescrite par NXP (VCC=5V + VIO=3,3V pour interfacer un MCU 3,3V).
  L'Adafruit ne "tolère" pas 3,3 V en entrée : elle le **convertit** en interne.

- Le PCB du clone **est** le layout Adafruit (voir photo / configuration de base),
  pas un TJA1051 nu 4 vis + JST. IC3 est bien là (SOT-23-6 `AG7T` = AP3602AKTR-G1,
  marquage datasheet `G7T`), avec les condo d'entrée/sortie/volant. Les 4 vis + JST
  sont des connecteurs rapportés sur les pastilles Adafruit.

- Guide Adafruit / `canio` : 3,3 V sur `Vcc`, MCU TX→pad TX, MCU RX→pad RX.
  Sur un Pal en breakout ils laissent SLNT flottant (pull-down = Normal). Leur
  exemple force `CAN_STANDBY` à **0** dès que le MCU a ce pin — exactement ce
  qu'il faut faire ici, parce que sur ce clone SLNT n'est **pas** à 0 V.

- **Piège méthodologique confirmé** : datasheet §6.2.1, timer "TXD dominant time-out"
  (0,3–12 ms, typ. 1 ms). Le premier test bit-bang (TXD bas pendant 1 s) coupait le
  transmetteur bien avant la lecture de RXD — un transceiver sain aurait donné le même
  résultat. Ce test ne discriminait donc rien tant qu'il n'a pas été refait avec une
  impulsion courte.

---

## Conclusion (corrigée 2026-09-08, soir)

**Alimenter en 3,3 V.** La pompe sort 5 V sur VCC puce, VIO/RXD à 3,3 V. Pas
d'UVLO, pas de level shifter.

**Cause :** `SLNT` à **2,7 V** → Silent → émetteur coupé. D'où RXD coincé haut,
CANH/CANL à 0 V, `BUS_OFF` isolé, **les deux** exemplaires (même layout, même
pastille `SLNT` non ramenée au GND, obstruée par les connecteurs).

Le GPIO n'y est pour rien : blanc/GPIO7 bascule, jaune/RXD ne suit pas parce que
rien n'est posé sur le bus.

**Rework (prévu, pas encore fait) :**

1. Dessouder le JST-XH (CANH/CANL) pour accéder à la pastille `SLNT`.
2. Souder un fil `SLNT` → vis **GND du bornier CAN** (plot central L/GND/H).
3. À la place du XH : deux fils CANH/CANL, sertis dans un **JST SM**.
4. Revérifier `SLNT` ≈ 0 V, puis `can-selftest` (bit-bang 80 µs puis TWAI
   `NO_ACK`, `PINOUT_CANPAL 1` : TX=GPIO7, RX=GPIO8).

Adafruit : `standby.switch_to_output(False)`. Ici un strap au GND suffit, pas
besoin d'un GPIO.

Les level shifters / open-drain du diagnostic 5 V **ne s'appliquent pas**.
`AMS1117` / `N7803-1CW` hors sujet.

---

## Suite — contournement : M5Stack Unit CAN (2026-09-08)

Le firmware capteurs tourne sur un **M5Stack Unit CAN** (U085, **CA-IS3050G isolé**,
<https://docs.m5stack.com/en/unit/can>) — même module que l'Atom / `can-monitor`.
Grove au repos : **5 V**. Le Pal n'est plus « mort » : c'est Silent ; le Unit CAN
reste le chemin qui marche tant que le rework `SLNT` n'est pas fait.

**Nouveau symptôme, même signature qu'avant** : premier essai avec le brochage porté
par analogie depuis le CAN Pal (TX=GPIO7/D8, RX=GPIO8/D9) — échec identique au run
initial du CAN Pal, `BUS_OFF` quasi immédiat, `tx_err=128`, `bus_err=16`, y compris en
self-test isolé (câble débranché). Cette fois la cause est un **TX/RX inversé**,
propre à ce module — même signature `BUS_OFF` que le Pal, autre origine.

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

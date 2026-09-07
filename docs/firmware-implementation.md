# Firmware — séquence d'implémentation

Conception, protocole et décisions : `firmware.md`. Ce fichier ne dit pas *comment* coder, il dit **dans quel ordre**, et ce qu'il ne faut pas oublier avant de passer à la suite.

L'objectif de bout en bout est un point précis : **débrancher l'USB**. Tout ce qui vient avant existe pour rendre ce moment sans risque ; tout ce qui vient après arrive par OTA.

## Le principe qui ordonne tout

On ne met **rien** en boîte, et on ne branche **rien** sur le 230 V, tant que le chemin qui permet de se rattraper n'est pas prouvé. Les phases sont donc construites à l'envers de l'envie : le protocole et le flash d'abord, les capteurs et l'UI ensuite.

Trois barrières, dans cet ordre :

| Barrière | Ce qu'elle autorise |
| --- | --- |
| **A** — les deux cartes se pinguent sur le bus | brancher les périphériques basse tension |
| **B** — bail, présence et verrou 60 s vérifiés | brancher le 230 V sur la pompe et la vanne |
| **C** — OTA prouvé dans les deux sens, rollback compris | fermer les boîtiers et débrancher l'USB |

---

## Phase 0 — Socle

Rien de visible, mais c'est ce qui évite de tout refaire en phase 4.

- Deux projets ESP-IDF, `sensors/` et `screen/`, et un composant `common/` partagé par les deux.
- Figer la version d'ESP-IDF et la noter dans le dépôt. Une montée de version en cours de route se paie sur la carte qu'on ne peut plus atteindre.
- Dans `common/` : définition des types de messages, des charges utiles, du CRC — **et la table de codes `LOG` générée depuis une source unique**, consommée par le C++ et par l'outil Python. C'est le point à ne pas rater : une table dupliquée dérive, et le premier message qu'on ne comprendra plus sera celui d'un crash.
- Un numéro de version par image, remonté dans le `PONG`. Discipline d'incrémentation dès maintenant : sur une carte en boîte, la seule façon de savoir ce qui tourne, c'est de le lui demander.
- Tests hôte sur `common/` : encoder / décoder chaque message, aller-retour. Ça tourne sur le Mac, sans matériel.

**Sortie :** `common/` compile pour les deux cibles et passe ses tests sur le Mac.

---

## Phase 1 — L'outil Mac

Avant les cartes, parce que tout le reste se débogue à travers lui.

- Décodeur de trames : lit le flux, imprime du texte lisible, horodate.
- Deux transports, même décodeur : **USB série** (phase 2 et 3) et **WebSocket** (phase 6). Écrire l'abstraction tout de suite, même si seul l'USB existe.
- Émission aussi, pas seulement lecture : pouvoir envoyer un `SET`, un `PING`, un `REQSTATUS` à la main est ce qui rend les phases 2 à 5 tenables.
- Enregistrement dans un fichier, et relecture. **Ce format devient la fixture des tests d'algorithme** en phase 6 : on enregistre un vrai shot, on le rejoue sur le Mac.
- Le client de flash (`BEGIN`, blocs, `END`) vit ici aussi.

**Sortie :** l'outil décode et rejoue un fichier de trames fabriqué à la main, sans matériel.

---

## Phase 2 — Capteurs factory, sur la table

Le XIAO seul, alimenté en USB, hors de la machine, sans aucun périphérique branché sauf le CAN Pal.

- TWAI à 500 kbit/s, réception en accept-all, aiguillage sur le type.
- `PING` / `PONG` avec identité et version.
- `LOG` au boot.
- `RESET`.
- **Exposer les compteurs d'erreur TWAI** dans un `LOG` périodique. C'est comme ça qu'on valide le câblage et la terminaison sans oscilloscope, et ça servira à chaque phase suivante.
- GPIO 10 tenu bas dès le démarrage, avant l'initialisation du CAN.
- Machine à états de sécurité **complète**, même sans actionneur branché : bail, présence, verrou 60 s en mémoire RTC. On la teste ici, à vide, où elle ne peut rien casser.
- Rien d'autre : ni I2C, ni débitmètre, ni logique d'infusion.

**Piège :** le verrou 60 s doit survivre à un `RESET` logiciel et ne se lever que sur un démarrage à froid. À vérifier explicitement, pas à supposer.

**Sortie :** l'outil Mac, branché sur un adaptateur CAN ou sur la seconde carte en phase 3, voit les pongs. Le verrou se déclenche à la commande et ne se lève qu'à la coupure d'alimentation.

---

## Phase 3 — Écran factory, le pont USB ↔ CAN

Le Waveshare seul, en USB, hors de la machine.

- `CAN_SEL` (EXIO5 du CH422G) tenu haut. **C'est la première chose à faire marcher** : sans ça le transceiver n'est pas sélectionné et il ne se passe rien, sans message d'erreur. Le bring-up CH422G de `tests/screen/hello_waveshare/` sert de référence.
- TWAI, même pile que les capteurs, depuis `common/`.
- CDC USB : pont bidirectionnel entre le port série et le bus.
- `PING` / `PONG` / `LOG` / `RESET`.
- Ni Wi-Fi, ni HTTP, ni LVGL, ni BLE. L'écran peut rester noir.
- Sauvegarder les deux images factory dans le dépôt ou à côté, avec leur version. Ce sont les seules qu'on reflashera un jour en USB.

**Puis :** relier les deux cartes par la paire CAN, terminaison activée aux deux bouts.

**Sortie — barrière A.** Les deux cartes se pinguent, l'outil Mac voit le trafic à travers le pont, les compteurs d'erreur TWAI restent à zéro sur plusieurs minutes.

---

## Phase 4 — Le flash, dans les deux sens

C'est le jalon. Tant qu'il n'est pas franchi, la suite est théorique.

- `FLASH_CTRL` / `FLASH_DATA` dans `common/`, donc identique des deux côtés.
- Table de partitions posée sur les deux puces : `factory`, `ota_0`, `ota_1`, `nvs`, `otadata`. Tailles provisoires, à réajuster en phase 6 quand on connaîtra le poids réel de l'application écran — mais **réajuster une table de partitions sur une carte en boîte n'est pas possible**, donc prévoir large dès maintenant.
- Effacement complet de la partition **avant** l'acquittement du `BEGIN`.
- Blocs de 2 ko acquittés avec CRC16, CRC32 global sur le `END`.
- Progression en `LOG`.
- Validation de l'image après démarrage, conditionnée au ping/pong CAN.
- **Temporisateur d'invalidation propre** : IDF ne redémarre pas tout seul une image en attente de validation. Écrire ce mécanisme, ne pas compter sur un rollback automatique.

Les quatre essais qui valident la phase, tous à faire pour de vrai :

1. Flasher une image saine dans les capteurs depuis le Mac, à travers l'écran. Elle démarre, elle pongue avec sa nouvelle version.
2. Flasher une image sciemment cassée dans les capteurs. **Le rollback ramène la précédente sans intervention.**
3. Idem sur l'écran, avec son propre OTA local.
4. Débrancher la paire CAN en plein transfert. L'écriture s'annule, `otadata` ne bouge pas, la carte redémarre sur l'image précédente.

**Sortie — barrière C (partielle).** Une carte se reflashe et se récupère sans USB. À ce stade seulement, la mise en boîte devient une option raisonnable.

---

## Phase 5 — Application capteurs, livrée par OTA

À partir d'ici, on ne flashe plus le XIAO en USB. Chaque itération passe par le bus : c'est la répétition générale de la vie en boîte, tant qu'on peut encore ouvrir.

Dans l'ordre, un périphérique à la fois, en vérifiant à chaque fois sur l'outil Mac :

1. **XDB401** — bus I2C, mutex, conversion déclenchée puis relâchement du mutex pendant les 50 ms d'attente. `STATUS_PRESSURE` avec les 5 octets bruts. `REQSTATUS` avec période.
2. **Débitmètre** — ISR sur front descendant, compteur 32 bits, horodatage du dernier front, `STATUS_FLOW`. Vérification à la main : souffler dans le capteur ou le faire tourner, compter les impulsions.
3. **Dimmer** — écriture du registre de niveau, lecture du statut et de l'erreur, remontés dans les flags. **Le dimmer exige le secteur pour sortir de `Calibrating...`** : c'est la première fois que 230 V et USB coexistent sur le plan de travail. **Laptop sur batterie, débranché du secteur** — sinon la masse du Mac rejoint celle de l'alim RECOM.
4. **SSR** — sortie GPIO, puis `SET` complet avec bail et `STATUS_ACTUATORS` en retour.

Puis la vérification de sécurité, **avant** de relier la pompe et la vanne :

- Bail : couper l'émission de `SET`, les actionneurs retombent en ~500 ms.
- Présence : débrancher la paire CAN, tout retombe, le streaming s'arrête.
- Verrou : commander 60 s d'affilée, vérifier la coupure, le code `LOG`, le refus des commandes suivantes, et que seul un cycle d'alimentation le lève.
- Rearmement : deux activations de 30 s séparées de plus de 2 s ne déclenchent rien.

**Sortie — barrière B.** Les actionneurs 230 V peuvent être reliés. Le module capteurs est complet et se met à jour par le bus.

---

## Phase 6 — Application écran, livrée par OTA

Le plus gros morceau, mais le moins risqué : l'écran reste atteignable en USB.

1. **Réseau** — provisioning Wi-Fi (point d'accès + page d'accueil suffit ; la saisie tactile peut attendre LVGL), identifiants en NVS, secret HTTP dans un en-tête non commité. Prévoir **un moyen d'effacer la NVS depuis l'image factory** : un SSID erroné enregistré rend l'écran injoignable en Wi-Fi, et c'est l'USB qui doit pouvoir rattraper ça.
2. **HTTP** — `GET` télémétrie, `POST` commandes, `POST` firmware avec cible. C'est le moment où le flash passe du câble série au réseau.
3. **WebSocket** — miroir du trafic CAN, **même format qu'en USB**. L'outil Mac ne change pas, il change de transport.
4. **Couche capteurs** — l'interface unique `{ horodatage, valeur brute, validité }` et les calibrations en NVS par-dessus. Les sources CAN d'abord.
5. **BLE** — client GATT vers l'Acaia Lunar, comme une source de plus.
6. **LVGL** — écran, tactile, et l'UI minimale : purge, départ d'infusion, arrêt.

**Sortie :** l'écran se flashe et flashe les capteurs par le réseau, et l'outil Mac voit tout par WebSocket.

---

## Phase 7 — Mise en boîte et débranchement

- Vérifier une dernière fois les deux images factory sauvegardées, avec leur version.
- Refaire les quatre essais de la phase 4, **par le réseau cette fois**, cartes hors boîte mais câblées comme en service.
- Monter le XIAO dans `boitier_dc`, l'écran dans `screen_wedge` / `screen_base`.
- Vérifier l'accès USB-C de l'écran une fois la façade montée, panneau latéral retiré. C'est le filet, il doit être praticable.
- Un flash complet des deux cartes, en boîte, par le réseau.
- **Débrancher l'USB.**

---

## Après

Tout ce qui suit arrive par OTA, dans la machine.

1. **Calibration** (`firmware.md`, section dédiée) — facteur K sous OPV, puis le sort de l'OPV au-dessus de son seuil, puis le point de décrochage, puis la carte dimmer → pression. C'est ce qui débloque les features suivantes, et c'est la première chose à faire une fois l'USB débranché.
2. **Purge / flush**, la plus simple, et celle qui exerce le chemin complet écran → CAN → actionneurs.
3. **Infusion au temps**, puis **au poids**.
4. **Pré-infusion**, une fois qu'on sait si le débitmètre sert à quelque chose en dessous de 1 ml/s.
5. **Flow control**, en dernier. L'OPV renvoyant à l'entrée de la pompe et restant fermée à 9 bar, le débitmètre lit bien le débit d'infusion ; la contrainte est sa résolution (~4 s de moyennage à 1 ml/s), donc pression et poids restent les signaux rapides.

---

## Ce qu'on oublie habituellement

- **La table de codes `LOG` en double.** Générée depuis une source unique, sinon elle dérive.
- **La table de partitions figée trop tard.** On ne la change plus une fois la carte inaccessible.
- **Le rollback jamais testé pour de vrai.** Une image sciemment cassée, ou ça ne compte pas.
- **Le verrou 60 s effacé par un reset logiciel.** Il doit être en mémoire RTC, levé seulement au démarrage à froid.
- **Les masses jointes.** Laptop sur batterie dès qu'un USB et le 230 V se croisent.
- **Le dimmer muet sans secteur.** Il reste en `Calibrating...` et refuse tout : ce n'est pas un bug du firmware.
- **Les pull-ups I2C qui vivent dans le XDB401.** Débrancher la sonde de pression rend le dimmer muet, en phase 5 comme en service.
- **Le mutex I2C tenu pendant les 50 ms de conversion.** Il ne doit pas l'être.
- **Les compteurs d'erreur TWAI jamais regardés.** C'est le seul diagnostic de câblage disponible sans oscilloscope.
- **La version qui ne bouge pas.** Sur une carte en boîte, le `PONG` est la seule façon de savoir ce qui tourne.
- **Les images factory non archivées.** Ce sont les seules qu'on reflashera en USB, il faut les retrouver.

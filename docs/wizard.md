# Infusion guidée

L'infusion guidée fait suivre à l'écran la routine de David avant chaque
infusion. Elle lance elle-même les deux rinçages et l'infusion, et
enregistre le déroulé dans la capture HF de l'infusion. L'infusion directe
reste telle quelle.

Objectif de mesure : le journal des infusions
([journal-infusions.md](journal-infusions.md)) montre des écarts de
température en tasse de plus de 2 °C à réglages égaux, liés à ce qui
précède l'infusion. Exemple : l'attente porte-filtre en place, le
13 octobre. La capture HF commence à l'appui sur « infuser », et chaque
purge l'efface : rien de ce qui précède l'infusion n'est conservé
aujourd'hui.

## Principe

- On passe d'un écran au suivant par un bouton : **suivant** ou
  **démarrer**. **Quitter** est toujours présent.
- La balance et la température ne font qu'**activer** ce bouton ; elles ne
  font jamais avancer le guide seules.
- La tare est envoyée par l'écran en BLE (commande Acaia `0x04`, déjà
  utilisée par l'action `tare`). La tare automatique de la Lunar peut donc
  être désactivée.

## Entrée

Sur l'accueil, « infuser » s'active dès que la machine accepterait une
purge (capteurs vivants, pas de verrou, pas de cycle, dimmer prêt), même
pendant la chauffe. Il ouvre une fenêtre à trois boutons : **directe**,
**guidée** et **annuler**.

- **Directe** est grisé tant que les conditions actuelles d'« infuser » ne
  sont pas réunies. La raison s'affiche dessous, comme sur l'accueil
  aujourd'hui.
- **Guidée** est toujours actif, avec ou sans balance.

## Conditions

| Nom | Définition |
| --- | --- |
| Balance | balance connectée et poids reçu depuis moins de 2 s (`scale_present` actuel) |
| Tasse posée | poids > **30 g** |
| Tasse retirée | poids < **−5 g**. Le guide attend que le poids soit stable (variation de 0,5 g au plus sur 1 s), puis tare : la balance vide revient à 0 |
| Rinçage possible | NTC valide et au moins à **consigne − 5 °C** |
| Consigne atteinte | condition actuelle d'« infuser » : NTC à **± 1 °C** de la consigne depuis 3 s, plus les autres conditions d'infusion (dimmer, chauffage, fraîcheur des mesures) |

Le seuil de −5 g plutôt que 0 g évite qu'une oscillation de la balance
autour de zéro passe pour un retrait.

Une tare met un instant à revenir de la balance. Après chaque tare, le
guide ignore le poids jusqu'à lire une valeur entre −2 et +2 g, ou pendant
2 s au plus, et n'envoie pas de nouvelle tare dans cet intervalle. Une tare
refusée par le cœur (balance non inscriptible) se traite comme une balance
absente : « reconnectez la balance ».

## Écrans

Chaque écran a le bandeau d'état habituel (heure, poids, pression,
température), un titre « infusion guidée », une consigne en une ou deux
lignes, et en bas son bouton d'action à droite et **quitter** à gauche.
Une seule consigne est affichée à la fois, la première de la liste de
l'écran dont la condition est vraie.

### Écran 1 : premier rinçage

| Condition | Consigne |
| --- | --- |
| pas de balance | « connectez la balance » |
| NTC sous consigne − 5 °C | « attendez 87 °C » (consigne − 5 °C), avec la température en grand |
| tasse non posée | « placez une tasse, sans porte-filtre, pour le premier rinçage » |
| sinon | « appuyez sur suivant pour lancer le rinçage » |

**Suivant** est actif quand la balance est là, la tasse posée et le
rinçage possible. L'appui tare la balance (tasse sur le plateau) puis
lance une purge de **5 s**. Pendant la purge, l'écran montre le compteur et
**arrêter** remplace **quitter**. À la fin de la purge, on passe à
l'écran 2.

### Écran 2 : rinçage du porte-filtre

| Condition | Consigne |
| --- | --- |
| pas de balance | « reconnectez la balance » |
| tasse retirée depuis l'écran 1 | « replacez la tasse » (la tare a été faite à vide) |
| NTC sous consigne − 5 °C | « attendez 87 °C » |
| sinon | « installez le porte-filtre, puis appuyez sur suivant pour le rinçage du porte-filtre » |

**Suivant** est actif quand la balance est là, que la tasse n'a pas été
retirée (ou a été replacée : poids > 30 g après la tare à vide), et que le
rinçage est possible. L'appui lance une purge de 5 s **sans tare**, pour
que la tasse garde l'eau du premier rinçage. À la fin, on passe à
l'écran 3.

### Écran 3 : attente et préparation

L'écran a deux temps, séparés par la consigne atteinte.

Avant la consigne :

| Condition | Consigne |
| --- | --- |
| pas de balance | « reconnectez la balance » |
| tasse retirée | « avez-vous enlevé la tasse ? replacez-la pour continuer » |
| sinon | « laissez la tasse et le porte-filtre en place, attendez la consigne », avec la température et le temps depuis le rinçage |

Une fois la consigne atteinte, le guide garde ce temps même si la NTC
ressort ensuite de la plage :

| Condition | Consigne |
| --- | --- |
| pas de balance | « reconnectez la balance » |
| tasse pas encore retirée depuis la consigne | « videz et rincez la tasse, séchez le porte-filtre, préparez la galette, replacez le porte-filtre et la tasse vide » |
| tasse retirée, pas encore replacée | même consigne |
| chaudière hors consigne | « attendez la consigne », avec la température |
| sinon | « appuyez sur démarrer » |

**Démarrer** est actif quand la balance est là, que la tasse a été
**retirée puis replacée depuis la consigne atteinte** (poids > 30 g après
la tare à vide), et que la consigne est atteinte au moment de l'appui.
L'appui tare la balance ; l'infusion part une fois la tare absorbée
(lecture entre −2 et +2 g, ou 2 s au plus), car la machine retient le poids
au départ comme référence. Le bouton est inactif entre-temps. L'écran
d'infusion habituel prend alors le relais.

Le retrait est exigé après la consigne : sans lui, la tasse encore pleine
des rinçages (environ 50 g) passerait pour la tasse vide.

## Cas tranchés

- **Purge de 5 s** : le cœur envoie `purge_press`, puis `purge_release`
  à 5,0 s, à la puissance `purge.pump_pct`. La durée est une constante du
  guide, pas un réglage. Le garde-fou `purge.max_s` reste actif.
- **Pendant une purge du guide**, un retrait de tasse ou la perte de la
  balance n'arrêtent pas la pompe : 5 s d'eau finissent au pire dans le
  bac. **Arrêter** coupe la purge et ramène à l'accueil.
- **Rinçage refusé par le cœur** (cycle, verrou, dimmer) : le bouton
  reste en place et la raison s'affiche en rouge, comme sur l'accueil.
- **Démarrer refusé par le cœur** : idem. Le guide reste à l'écran 3.
- **Plusieurs retraits avant la consigne** : chacun tare à vide et
  demande de replacer la tasse ; ils sont comptés dans le déroulé.
- **Quitter** abandonne le guide sans confirmation et revient à l'accueil.
  Le déroulé est jeté.
- **Inactivité** : le guide abandonne après **30 min** sans appui.
  Pendant le guide, la veille assombrit et masque l'écran comme ailleurs ;
  un appui le réveille sur l'écran en cours.
- **Défaut** (verrou, module interne injoignable, mise à jour) : les plein
  écrans de sécurité gardent leur priorité et le guide est abandonné.

## Déroulé enregistré

Le cœur tient en RAM une structure `session` distincte du tampon HF. Elle
naît à l'ouverture du guide, et les purges ne l'effacent pas. Elle est
attachée à la capture de l'infusion lancée par **démarrer**, puis gardée
avec elle jusqu'à la capture suivante. Elle n'enregistre pas
d'échantillons, seulement des instants et des valeurs résumées.

Les instants sont en millisecondes **relatives au début de la capture HF**
de l'infusion, donc négatifs. Ils viennent de l'horloge monotone et
restent justes sans heure SNTP.

### Export

`GET /hf-capture` d'une infusion porte une clé `session` en tête, avant
`samples`. Elle vaut `null` pour une infusion directe et n'existe pas pour
une purge. Le schéma reste `coffeeflow.hf_capture.v2`, car les outils
ignorent les champs qu'ils ne connaissent pas (précédent :
`scace_temperature_c`).

```json
"session": {
  "opened_ms": -412300,
  "opened_at_unix_s": 1791890277,
  "setpoint_c": 92.0,
  "ntc_open_c": 88.1,
  "purges": [
    {"start_ms": -301200, "duration_ms": 5000, "volume_ml": 24.9,
     "weight_g": 23.8, "ntc_start_c": 91.2, "ntc_min_c": 84.0,
     "ntc_end_c": 84.6, "pressure_max_bar": 0.4},
    {"start_ms": -279400, "duration_ms": 5000, "volume_ml": 24.6,
     "weight_g": 23.1, "ntc_start_c": 85.9, "ntc_min_c": 74.2,
     "ntc_end_c": 74.8, "pressure_max_bar": 0.5}
  ],
  "wait": {"ntc_min_c": 74.0, "setpoint_reached_ms": -233800,
           "heater_on_ms": 31200, "early_removals": 0},
  "cup_removed_ms": -118900,
  "cup_returned_ms": -21400,
  "start_pressed_ms": 0,
  "scale_lost_ms": 0
}
```

| Champ | Définition |
| --- | --- |
| `opened_ms`, `opened_at_unix_s` | ouverture du guide ; l'heure murale vaut 0 si l'heure n'est pas connue, comme `started_at_unix_s` |
| `setpoint_c`, `ntc_open_c` | consigne d'infusion et NTC à l'ouverture |
| `purges[]` | les deux rinçages : départ, durée réelle jusqu'à l'arrêt de la pompe (moins de 5 000 si arrêtée ou refusée en cours), volume du débitmètre, poids gagné 2 s après l'arrêt (`null` sans balance), NTC au départ, minimum et NTC à l'arrêt, pression maximale |
| `wait.ntc_min_c` | minimum de la NTC entre la fin du second rinçage et **démarrer** |
| `wait.setpoint_reached_ms` | consigne atteinte à l'écran 3 |
| `wait.heater_on_ms` | temps de SSR actif entre la fin du second rinçage et **démarrer** ; ×1 200 W donne l'énergie fournie |
| `wait.early_removals` | retraits de la tasse à l'écran 3 avant la consigne |
| `cup_removed_ms`, `cup_returned_ms` | premier retrait après la consigne, et retour qui active **démarrer** |
| `start_pressed_ms` | départ réel de l'infusion, une fois la tare absorbée ; vaut 0 ou presque, car la capture commence à ce départ |
| `scale_lost_ms` | durée cumulée sans balance, de l'ouverture à **démarrer** |

Durées lues dans le journal :

- **retour à la consigne** : de la fin du second rinçage à
  `wait.setpoint_reached_ms` ;
- **porte-filtre en place après la consigne** : de
  `wait.setpoint_reached_ms` à `cup_removed_ms`. Le porte-filtre quitte le
  groupe juste avant ou juste après le retrait de la tasse, à quelques
  secondes près ;
- **préparation** : de `cup_removed_ms` à `start_pressed_ms`.

`describe_hf_capture.py` affiche le déroulé en tête du rapport HTML, avec
ces trois durées. Ce sont les sources des entrées du journal.

## Répartition du code

| Partie | Rôle |
| --- | --- |
| `screen-core`, nouveau module `guided` | écrans, conditions d'activation, détection du retrait et tare, purge de 5 s, structure `session`. Testé sur l'hôte avec des traces synthétiques de poids et de température, une par cas tranché |
| `screen-core::core` | actions `guided_open`, `guided_next` et `guided_quit`, réservées à l'UI et refusées par HTTP comme `start_brew` ; attache la session au départ de l'infusion ; export de `session` |
| `screen-core::snapshot` | écran en cours, consigne affichée, bouton actif, raison d'un refus |
| `screen` (Slint et présentateur) | fenêtre de choix et page du guide. Les gestes deviennent des actions, sans logique d'étape |
| `screen-sim` | scénarios d'instantanés : fenêtre, chaque consigne de chaque écran, purge en cours |
| `describe_hf_capture.py` | lecture de `session` |

Le guide ne change aucun réglage ni aucune loi de chauffe ou de pompe. Ses
rinçages sont des purges ordinaires, chacune avec sa propre capture HF, que
l'infusion efface ensuite comme aujourd'hui.

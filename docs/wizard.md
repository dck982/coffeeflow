# Infusion guidée

L'infusion guidée fait suivre à l'écran la routine de David avant chaque
infusion. Elle lance elle-même le rinçage du porte-filtre et l'infusion, et
enregistre le déroulé dans la capture HF de l'infusion. L'infusion directe
reste telle quelle.

La purge sans porte-filtre qui ouvre la routine se fait avant le guide, au
bouton purge de l'accueil. Sans porte-filtre, l'eau gicle autour d'une
tasse à espresso et mouille la balance ; le guide ne la demande donc pas et
ne vérifie pas qu'elle a eu lieu.

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
- Le guide pèse la tasse vide au premier écran et la reconnaît quand elle
  revient avant l'infusion. Le poids est gardé dans le déroulé, pour savoir
  plus tard quelle tasse a servi.

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
| Tasse retirée | poids < **−5 g**, sur les deux écrans. Le guide attend que le poids soit stable (variation de 0,5 g au plus sur 1 s), puis tare : la balance vide revient à 0 |
| Tasse reconnue | à l'écran 2, après une tare à vide depuis la consigne atteinte : poids à **± 1,5 g** de `cup_g`, quelle que soit la tasse ; la marge couvre les gouttes d'une tasse rincée |
| Rinçage possible | NTC valide et au moins à **consigne − 5 °C** |
| Consigne atteinte | condition actuelle d'« infuser » : NTC à **± 1 °C** de la consigne depuis 3 s, plus les autres conditions d'infusion (dimmer, chauffage, fraîcheur des mesures) |

Le seuil de −5 g plutôt que 0 g évite qu'une oscillation de la balance
autour de zéro passe pour un retrait.

Une tare met un instant à revenir de la balance. Après chaque tare, le
guide ignore le poids jusqu'à lire une valeur entre −2 et +2 g, ou pendant
2 s au plus, et n'envoie pas de nouvelle tare dans cet intervalle. Une tare
refusée par le cœur (balance non inscriptible) se traite comme une balance
absente : « reconnectez la balance ».

**Tare d'ouverture.** Le guide tare la balance à son ouverture, ou à la
première apparition de la balance s'il s'ouvre sans elle. À l'écran 1,
chaque retour de la balance après une perte tare de nouveau : on ne sait
pas ce qui a été posé entre-temps. À l'écran 2, un retour de la balance
ne tare pas, pour garder la référence de la tasse.

## Écrans

Chaque écran a le bandeau d'état habituel (heure, poids, pression,
température), un titre « infusion guidée », une consigne en une ou deux
lignes, et en bas son bouton d'action à droite et **quitter** à gauche.
Une seule consigne est affichée à la fois, la première de la liste de
l'écran dont la condition est vraie.

### Écran 1 : rinçage du porte-filtre

| Condition | Consigne |
| --- | --- |
| pas de balance | « connectez la balance » |
| NTC sous consigne − 5 °C | « attendez 87 °C » (consigne − 5 °C), avec la température en grand |
| tasse non posée | « installez le porte-filtre et placez une tasse pour le rinçage » |
| sinon | « appuyez sur suivant pour le rinçage du porte-filtre » |

**Suivant** est actif quand la balance est là, la tasse posée et le
rinçage possible. L'appui relève le poids affiché, celui de la tasse vide
depuis la tare d'ouverture, dans `cup_g`. Il tare ensuite la balance
(tasse sur le plateau) puis lance une purge de **5 s**. Le poids relevé
est la dernière valeur reçue avant l'appui, sans attente de stabilité :
la tasse est posée avant que le doigt ne touche l'écran. Pendant la purge, l'écran montre le compteur et
**arrêter** remplace **quitter**. À la fin de la purge, on passe à
l'écran 2.

### Écran 2 : attente et préparation

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
du rinçage (environ 25 g) passerait pour la tasse vide.

Quand la tasse est reconnue, l'illustration dessine la tasse en vert ;
sinon elle reste dans sa couleur habituelle. C'est une indication : une
autre tasse, hors tolérance, active **démarrer** comme avant.

## Cas tranchés

- **Purge de 5 s** : le cœur lance une purge dont la durée, 5,0 s, est
  fixée au départ ; le cycle s'arrête seul, à la puissance
  `purge.pump_pct`, comme `coffeetool purge`. La durée est une constante
  du guide, pas un réglage, et remplace `purge.max_s` pour ce rinçage. Le
  relâchement physique de l'écran ne vaut que pour le bouton purge : le
  doigt qui se lève après **suivant** ne coupe pas le rinçage.
- **Pendant une purge du guide**, un retrait de tasse ou la perte de la
  balance n'arrêtent pas la pompe : 5 s d'eau finissent au pire dans le
  bac. **Arrêter** coupe la purge et ramène à l'accueil.
- **Rinçage refusé par le cœur** (cycle, verrou, dimmer) : le bouton
  reste en place et la raison s'affiche en rouge, comme sur l'accueil.
- **Démarrer refusé par le cœur** : idem. Le guide reste à l'écran 2.
- **Tasse déjà sur la balance à l'ouverture** : la tare d'ouverture la
  compte comme plateau vide, et le poids reste près de 0 g ; la consigne
  demande de placer une tasse. La soulever fait passer le poids sous
  −5 g : le guide tare à vide, et la reposer la pèse correctement.
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
  "cup_g": 41.3,
  "rinse": {"start_ms": -279400, "duration_ms": 5000, "volume_ml": 24.6,
            "weight_g": 23.1, "ntc_start_c": 85.9, "ntc_min_c": 74.2,
            "ntc_end_c": 74.8, "pressure_max_bar": 0.5},
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
| `cup_g` | poids de la tasse vide relevé à l'appui sur **suivant** de l'écran 1, au dixième de gramme |
| `rinse` | le rinçage du porte-filtre : départ, durée réelle jusqu'à l'arrêt de la pompe (moins de 5 000 si arrêtée ou refusée en cours), volume du débitmètre, poids gagné 2 s après l'arrêt (`null` sans balance), NTC au départ, minimum et NTC à l'arrêt, pression maximale |
| `wait.ntc_min_c` | minimum de la NTC entre la fin du rinçage et **démarrer** |
| `wait.setpoint_reached_ms` | consigne atteinte à l'écran 2 |
| `wait.heater_on_ms` | temps de SSR actif entre la fin du rinçage et **démarrer** ; ×1 200 W donne l'énergie fournie |
| `wait.early_removals` | retraits de la tasse à l'écran 2 avant la consigne |
| `cup_removed_ms`, `cup_returned_ms` | premier retrait après la consigne, et retour qui active **démarrer** |
| `start_pressed_ms` | départ réel de l'infusion, une fois la tare absorbée ; vaut 0 ou presque, car la capture commence à ce départ |
| `scale_lost_ms` | durée cumulée sans balance, de l'ouverture à **démarrer** |

Les captures guidées antérieures au 15 octobre 2026 n'ont ni `cup_g` ni
`rinse`, mais un tableau `purges` de deux entrées : la purge sans
porte-filtre, puis le rinçage du porte-filtre. `describe_hf_capture.py`
lit `rinse`, ou à défaut la dernière entrée de `purges`, et n'affiche pas
la tasse quand `cup_g` manque.

Durées lues dans le journal :

- **retour à la consigne** : de la fin du rinçage du porte-filtre à
  `wait.setpoint_reached_ms` ;
- **porte-filtre en place après la consigne** : de
  `wait.setpoint_reached_ms` à `cup_removed_ms`. Le porte-filtre quitte le
  groupe juste avant ou juste après le retrait de la tasse, à quelques
  secondes près ;
- **préparation** : de `cup_removed_ms` à `start_pressed_ms`.

`describe_hf_capture.py` affiche le déroulé en tête du rapport HTML, avec
ces trois durées. Ce sont les sources des entrées du journal.

## Piste : temps minimal sur le groupe

Envisagée le 14 octobre 2026, pas codée. Le guide garderait « laissez la
tasse et le porte-filtre en place » jusqu'à ce que la consigne soit
atteinte **et** qu'une durée minimale soit écoulée depuis la fin du
rinçage du porte-filtre, même consigne atteinte. Le chrono part de la fin
de ce rinçage : c'est depuis cet instant que le porte-filtre chauffe sur
le groupe, et l'écran 2 affiche déjà ce temps.

Origine : à galette et chauffe égales, les deux premières routines
guidées diffèrent de 0,76 °C en NTC tasse, dont 0,34 °C seulement viennent
du modèle de chaudière. Le reste suit l'état du porte-filtre :

| Routine | Sur le groupe après le rinçage du porte-filtre | Préparation | Écart du modèle (énergie SSR) | NTC tasse |
| --- | ---: | ---: | ---: | ---: |
| 14/10 9 h 42 | 148 s | 143 s | +0,06 °C | 90,89 °C |
| 14/10 14 h 48 | 642 s | 70 s | −0,50 °C | 91,65 °C |

Un modèle sous la mesure veut dire une eau moins refroidie que ne le
prévoit la chaudière seule : porte-filtre et groupe plus chauds. Les deux
durées ont varié ensemble, en sens inverse ; rien ne les sépare encore.

**Quantification par les captures.** Chaque entrée du journal relève, pour
une routine guidée, la durée sur le groupe (retour à la consigne plus
porte-filtre en place après la consigne), la préparation, la résistance au
plateau, l'écart du modèle avec l'énergie SSR et la NTC tasse. Une capture
compte si les réglages sont ceux en cours et la galette entre 7,0 et
8,5 bar·s/g. On ne force pas les durées : on note celles de la routine du
jour.

- **Lecture** : l'écart du modèle en fonction de la durée sur le groupe,
  à préparation de 60 à 150 s. Le temps minimal est la plus courte durée
  au-delà de laquelle l'écart ne baisse plus de plus de 0,2 °C.
- **Effet confirmé** : au moins trois captures de 4 min ou moins sur le
  groupe et trois de 6 min ou plus, dont les écarts moyens diffèrent d'au
  moins 0,4 °C sans que les deux groupes se chevauchent.
- **Effet écarté** : mêmes effectifs, écarts moyens à moins de 0,2 °C. Pas
  de temps minimal.
- **La préparation à la place** : si l'écart suit la préparation et non la
  durée sur le groupe, le levier est la préparation ; un temps minimal ne
  suffirait pas.

**Bascule sur la sonde SCACE** si, au **28 octobre**, l'un des deux groupes
compte moins de trois captures, ou si les groupes se chevauchent sans
écart net. La procédure ci-dessous peut aussi être menée avant.

*Avec eau.* L'air entre la douchette et le puck ne pèse presque rien
(≈ 13 cm³, 0,015 J/K, contre ≈ 42 J/K pour les 10 premiers ml d'eau et
≈ 190 J/K pour le porte-filtre), et une perle dans l'air suit mal le métal
autour. L'eau du rinçage, retenue au-dessus du puck par le panier aveugle,
couvre la perle et la relie à la douchette et au panier : c'est l'état
réel de l'attente, et celui où le critère de 0,5 K/min a été retenu le
10 octobre ([sonde-scace.md](sonde-scace.md)).

*Hors du guide.* Avec le panier SCACE, la pression monte dès que la pompe
tourne (5,8 bar en 10 s le 10 octobre ; 2 à 3 bar attendus en 5 s) et le
joint fuit : la balance reste à l'abri, et le guide, qui l'exige, n'est pas
utilisé. Chaque rinçage est un `coffeetool purge 5`, la purge du guide.

*Préparation.* Machine à la consigne de 92 °C depuis 30 min au moins,
porte-filtre habituel en place. Core2 chargé, pont sous tension (touche
A). Sur le Mac, `coffeetool telemetry --record --scace`, arrêté par
Ctrl-C à la fin : un seul fichier, à 5 Hz, porte la NTC chaudière, la
résistance, la pression et la SCACE, attentes comprises. Balance retirée
ou protégée, une tasse sous le porte-filtre. Noter l'heure de chaque
geste, surtout les déverrouillages et reverrouillages, que rien
n'enregistre. Après chaque purge, `coffeetool download-hf` (fin des 5 s
attendue, `409` avant) donne en plus la capture à 10 Hz ; facultatif.

| Étape | Geste | Mesure |
| --- | --- | --- |
| A. Temps sur le groupe | purge de 5 s sans porte-filtre au bouton purge ; porte-filtre SCACE verrouillé, `coffeetool purge 5` ; **10 min** sans toucher | remontée de l'eau au-dessus du puck depuis la fin du rinçage |
| B. Référence | sans déverrouiller, `coffeetool purge 5` | première eau, porte-filtre à l'équilibre |
| C. Préparation de 70 s | 10 min après B, qui sert de rinçage : déverrouiller, vider, essuyer, tenir **70 s** comme pendant une préparation, reverrouiller, `coffeetool purge 5` aussitôt | ce qu'une préparation courte reprend |
| D. Préparation de 150 s | 10 min après C : même geste, **150 s** hors du groupe | ce qu'une préparation longue reprend |
| E. Croisement | 10 min après D : déverrouiller, vider, reverrouiller, `coffeetool purge 5`, qui sert de rinçage ; **140 s** sur le groupe ; préparation de **70 s** ; reverrouiller, `coffeetool purge 5` | durée sur le groupe courte, préparation courte |

Environ 55 min, six purges de 5 s.

*Pression sur le panier SCACE.* Le joint de la sonde fuit sous pression
([sonde-scace.md](sonde-scace.md#limites-du-montage)). Aucune étape
n'infuse : chaque purge de 5 s monte à 2 à 3 bar, puis la pression
retombe par la vanne 3 voies à l'arrêt de la pompe. Pour exposer la
sonde le moins possible, faire **A seule** : une purge, qui répond à
« 140 s sur le groupe suffisent-elles ? ». B à E ajoutent cinq purges et
répondent à la préparation ; ne les faire que si A ne montre pas plus de
≈ 1 K entre 140 et 274 s (voir la lecture). Arrêter l'essai si la fuite mouille la balance ou les
fils : la mesure de la purge en cours reste lisible, pas les suivantes.

*Lecture.*

- **Temps minimal** : dans A, le temps depuis la fin du rinçage pour que la
  pente de la SCACE sur 60 s passe sous **0,5 K/min**. Au repos, elle ne
  descend pas sous 0,35 à 0,45 K/min, les cycles de chauffe réchauffant le
  groupe. Relever aussi la SCACE à 140, 148, 274 et 339 s, durées des
  routines du 17 octobre à 13 h 57 (clairement acide), du 14 octobre à
  9 h 42 (légèrement acide), du 15 octobre à 9 h 08 (le plus équilibré) et
  du 17 octobre à 10 h 36 (satisfaisant).
- **140 s, trop court ?** SCACE à 140 s plus de ≈ 1 K sous celle à
  274 s : la durée sur le groupe suffit à expliquer l'acidité des deux
  routines les plus courtes, et le temps minimal se lit sur A. Moins de
  0,5 K : elle ne l'explique pas ; la préparation (B à E) ou le
  remplissage long du 17 octobre à 13 h 57 (28,4 ml) restent les
  candidats. Entre les deux, elle n'en explique qu'une partie. Dans ces
  deux cas, faire B à E.
- **Préparation** : SCACE moyenne pendant les 5 s de purge de B, C et D.
  C et D à plus de ≈ 1 K sous B, D plus bas que C : la préparation est le
  levier, et le guide en bornerait la durée plutôt que d'imposer un temps
  sur le groupe. Les trois à moins de 0,5 K : la préparation ne compte pas,
  seul le temps minimal de A reste. Entre les deux : refaire C et D.
- **Croisement** : E à moins de 0,5 K de C (même préparation de 70 s) :
  140 s sur le groupe ne refroidissent pas l'eau au-delà de ce que fait
  la préparation, le levier est la préparation. E plus de ≈ 1 K sous C :
  la durée sur le groupe compte même avec une préparation courte. Les deux
  routines acides cumulaient 140 à 148 s sur le groupe et 139 à 143 s de
  préparation ; seule E sépare les deux causes.
- Les seuils sont des ordres de grandeur : la SCACE lit l'eau au puck, pas
  en tasse, et les écarts en tasse à expliquer sont de 0,5 à 0,8 °C.

## Répartition du code

| Partie | Rôle |
| --- | --- |
| `screen-core`, nouveau module `guided` | écrans, conditions d'activation, tare d'ouverture, détection du retrait et tare, pesée et reconnaissance de la tasse, purge de 5 s, structure `session`. Testé sur l'hôte avec des traces synthétiques de poids et de température, une par cas tranché |
| `screen-core::core` | actions `guided_open`, `guided_next` et `guided_quit`, réservées à l'UI et refusées par HTTP comme `start_brew` ; attache la session au départ de l'infusion ; export de `session` |
| `screen-core::snapshot` | écran en cours, consigne affichée, bouton actif, raison d'un refus |
| `screen` (Slint et présentateur) | fenêtre de choix et page du guide, tasse de l'illustration en vert quand elle est reconnue. Les gestes deviennent des actions, sans logique d'étape |
| `screen-sim` | scénarios d'instantanés : fenêtre, chaque consigne des deux écrans, purge en cours |
| `describe_hf_capture.py` | lecture de `session`, `cup_g` et `rinse`, avec repli sur `purges` des anciennes captures |

Le guide ne change aucun réglage ni aucune loi de chauffe ou de pompe. Son
rinçage est une purge ordinaire, avec sa propre capture HF, que l'infusion
efface ensuite comme aujourd'hui.

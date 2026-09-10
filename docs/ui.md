# UI de l'écran — style, structure, décisions

Document de travail pour le point 6 de la phase 6 (`firmware-implementation.md`).
Il fixe **le style et la structure**, pas le code. Maquette à l'échelle :
`docs/ui-mockup.html` (ouvrir dans le navigateur, les cadres font 800 × 480 px
réels).

Conception d'ensemble et protocole : `firmware.md`. Ordre de réalisation :
`firmware-implementation.md`.

---

## Contraintes physiques, qui décident presque tout

- **800 × 480 sur 4,3"** = ~217 dpi, soit **8,5 px/mm**. Ce n'est pas un écran
  de PC : 100 px de hauteur de chiffre = 12 mm réels, lisible depuis le
  comptoir à 1,5 m. C'est ce qui rend le mode « statut plein écran pendant
  l'infusion » utile plutôt que gadget.
- **Doigt = 10 mm**, doigt mouillé ou gras sur un capacitif GT911 = touches
  ratées. **Cible tactile minimum 80 px**, jamais deux cibles adjacentes sans
  16 px de garde. Conséquence : peu de cibles, grandes. Aucun écran n'en porte
  plus de 6.
- **RGB565 (65 K couleurs)** : 32 niveaux de rouge et de bleu, 64 de vert. Deux
  gris qui diffèrent de moins de ~8/255 deviennent **le même gris** après
  quantification, et un dégradé plein écran bande visiblement. D'où : pas de
  dégradés, et les filets à au moins ~12 % de blanc pour survivre à l'arrondi
  (un filet à 6 % disparaît sur la moitié des dalles).
- **Panneau RGB parallèle + framebuffer en PSRAM** : la bande passante est le
  facteur limitant, pas le CPU. Une animation qui repeint tout l'écran
  déchire. D'où : animations locales et courtes, jamais de transition
  plein écran.
- **Pas de bouton physique.** L'ancien bouton brew est supprimé (son trou sert
  de passe-câble). Le seul repli matériel est l'interrupteur principal de la
  machine — qui est aussi, par construction, le réarmement du verrou 60 s.

---

## Parti pris de style

**Sombre et chaud, typographique, sans cadres.** L'écran est dans un boîtier
noir avec un cadre de ~1 cm : un fond quasi noir fait disparaître la dalle et
il ne reste que les chiffres qui flottent dans la façade. Un fond clair aurait
fait un rectangle lumineux permanent dans une cuisine.

Les sept leviers qui évitent le look « UI industrielle » (le risque réel d'un
tableau de bord de machine) :

1. **Aucun cadre autour d'une valeur.** Les séparations sont des **filets de
   1 px**, horizontaux ou verticaux, jamais des boîtes. Un chiffre n'est pas
   dans un conteneur, il est posé sur le fond.
2. **Beaucoup de vide.** Marge extérieure de 32 px, et l'écran de repos n'a
   que trois zones. Le vide est ce qui distingue une interface d'un synoptique.
3. **Un seul accent.** L'ambre `#D98324` (couleur crema, et surtout pas le
   rouge de la machine, qui entrerait en concurrence avec elle) et rien
   d'autre. Le rouge `#B9412F` est **réservé aux fautes** — s'il apparaît
   ailleurs il ne veut plus rien dire.
4. **Pas de vert/orange/rouge en feux tricolores.** Un état normal est écrit en
   blanc cassé. La couleur signale l'action en cours, pas la santé.
5. **Une typographie humaniste**, pas un faux afficheur sept segments ni un
   DIN condensé. Chiffres à **chasse fixe** (`tabular figures`) pour qu'un
   `1` ne fasse pas sauter la ligne dix fois par seconde.
6. **Minuscules partout**, sauf les micro-étiquettes (18 px, interlettrage
   ouvert). Pas de `ALL CAPS` sur les valeurs.
7. **Pas de jauges, pas de cadrans, pas de barres épaisses.** Un seul élément
   graphique dans tout le système : le filet de progression (voir ci-dessous).

### Le filet de progression, élément signature

Sous le chiffre héros, un **filet de 2 px** qui se remplit en ambre vers la
cible. C'est **la même séparation discrète** que partout ailleurs dans l'UI,
qui se trouve porter l'information de progression. Ça évite d'ajouter un
widget « barre de progression » (qui ramènerait le look tableau de bord) tout
en donnant l'information la plus utile pendant un shot : *combien il reste*,
lisible sans lire un chiffre.

Il sert pour le poids (vers la cible), pour le temps (vers la cible), et pour
l'OTA. Un seul vocabulaire graphique pour trois choses.

---

## Jetons de style

À figer tel quel dans `ui_theme.h` le jour où le code démarre — ces valeurs
sont déjà choisies en tenant compte de la quantification RGB565.

| Jeton | Valeur | Emploi |
| --- | --- | --- |
| `bg` | `#141110` | fond, partout |
| `bg_raised` | `#1E1A18` | pavé numérique, feuille de profils |
| `hairline` | `#332C28` | filets de séparation (≈ 12 % de blanc chaud) |
| `text` | `#F2EBE3` | valeurs, titres |
| `text_dim` | `#A2968C` | unités, valeurs secondaires |
| `text_faint` | `#6B615A` | étiquettes, valeurs absentes (`—`) |
| `accent` | `#D98324` | action en cours, progression, bouton primaire |
| `accent_wash` | `#D98324` à 14 % | fond d'un bouton pressé |
| `fault` | `#B9412F` | faute uniquement (verrou, perte de bus, erreur dimmer) |

**Pas de couleur « succès ».** Ce qui va bien s'écrit en `text`.

### La rampe d'engagement

Une seule échelle de couleur, partagée par **la pompe et le débit**, qui dit
**à quel point la machine pousse** — pas si c'est bien ou mal. Quatre niveaux,
tous dérivés de la même teinte ambre : ça reste un seul accent (règle 3 du
parti pris), avec de la luminosité en plus, et pas un feu tricolore.

| Niveau | Jeton | Valeur | Sens |
| --- | --- | --- | --- |
| 0 | `ramp_off` | `#6B615A` (= `text_faint`) | éteint, ou indisponible |
| 1 | `ramp_low` | `#8C5A22` | actif, en dessous du régime visé |
| 2 | `ramp_on` | `#D98324` (= `accent`) | actif **au régime visé** |
| 3 | `ramp_full` | `#F2B25C` | plein régime / saturation |

Application :

| Grandeur | 0 | 1 | 2 | 3 |
| --- | --- | --- | --- | --- |
| **Pompe** | dimmer à 0, ou `dimmer_ready = false` | 1–99 % hors du niveau du profil | au niveau demandé par le profil (± 5 %) | 100 % |
| **Débit** (flow control actif) | pas de débit, ou pas de cible | écart > 10 % de la cible | **dans les ± 10 %** | débit qui plafonne alors que la pompe est à 100 % |

Sans flow control, le débit n'a pas de cible : il reste en niveau 1 dès qu'il
coule, en 0 sinon. Le niveau 3 du débit est la signature d'un **décrochage** —
la pompe est à fond et le débit ne suit plus — et c'est exactement
l'information qu'on veut voir d'un coup d'œil pendant une extraction qui part
mal.

Les quatre valeurs restent distinctes après quantification RGB565 (les écarts
sont d'au moins 40/255 par canal). Elles ne sont utilisées **que** sur ces deux
grandeurs, jamais sur du texte courant.

| Rôle | Taille | Notes |
| --- | --- | --- |
| Héros infusion | 104 px | chiffres seuls |
| Héros repos | 76 px | la cible |
| Unité accolée au héros | 32 px | `text_dim`, alignée sur la ligne de base |
| Valeur secondaire | 40 px | temps pendant une infusion, résumé |
| Bandeau de statut | 28 px | toutes les mesures au repos |
| Bouton | 26 px | |
| Étiquette | 18 px | interlettrage +8 %, `text_faint` |

Rayon des coins : **8 px** partout (assez pour ne pas être brut, assez peu pour
ne pas faire « application mobile »). Épaisseur de contour : **1,5 px**.

### Police

**Inter** (SIL OFL, chiffres tabulaires natifs, dessinée pour les basses
résolutions), embarquée via `lv_font_conv`. `montserrat` intégré à LVGL sert
de repli pendant le bring-up, mais son `1` n'est pas tabulaire — à ne pas
garder pour l'affichage d'une valeur qui change.

Coût flash, à surveiller pour les partitions (`firmware.md` laisse la taille
exacte à trancher) : sortir les gros corps en **jeu de glyphes restreint**.
Le 104 px n'a besoin que de `0-9`, `.`, `g`, `s` — une douzaine de glyphes en
4 bpp ≈ 55 ko, contre ~700 ko pour un latin complet à ce corps. Les corps
texte (18/26/40) prennent le latin-1 complet.

Icônes : **quatre**, converties d'un jeu SVG libre (Lucide, ISC) en une police
d'icônes de 24 px par le même outil. Pas de PNG : ça ne se recolore pas (or la
rampe d'engagement ci-dessous en vit) et ça mange la partition de ressources.

| Icône | Ce qu'elle dit | Couleur |
| --- | --- | --- |
| balance | la balance est appairée et répond | `text` présente / `text_faint` absente |
| Wi-Fi | **connecté au réseau** | `text` connecté / `text_faint` coupé ou absent |
| pompe | le régime de la pompe | rampe d'engagement, niveaux 0-3 |
| goutte | la vanne est ouverte | `accent` ouverte / `text_faint` fermée |

**Pas d'icône Bluetooth** : elle serait redondante avec l'icône balance, qui
est la seule chose que le BLE sert à faire. Une icône qui ne peut jamais
contredire sa voisine ne porte pas d'information.

L'icône Wi-Fi est **atténuée pendant une infusion** parce que la radio est
délibérément rendue au BLE (`firmware.md`, « Politique radio »). Ce n'est pas
une faute et ça ne s'affiche pas comme telle : pas de rouge, pas de croix, pas
de message.

---

## Les boutons

**Contour, pas remplissage.** Un rectangle de 8 px de rayon, contour 1,5 px en
`hairline`, fond transparent, libellé en `text`. Le bouton **primaire** de
l'écran (celui qui lance) a son contour et son libellé en `accent` — c'est la
seule différence, pas de pavé plein.

**Retour au toucher**, en 90 ms, sur trois propriétés à la fois pour que ce
soit perceptible même du coin de l'œil :

- le fond passe à `accent_wash`,
- le contour passe à `accent`,
- le libellé passe à `accent`.

Au relâchement, retour en 150 ms. Pas d'ondulation façon Material (elle repeint
une grande surface, voir la contrainte de bande passante), pas de déplacement
de 1 px (illisible), pas de son (il n'y a pas de haut-parleur dans la façade).

Un bouton fait **88 px de haut**, jamais moins. Largeur selon le texte, minimum
200 px.

---

## Structure : cinq niveaux de statut

Le statut n'est pas un écran, c'est **un niveau d'intensité** qui change selon
ce que la machine fait. C'est ce qui permet d'avoir le poids central pendant un
brew by weight et discret le reste du temps, sans dupliquer les écrans.

| Niveau | Quand | Forme |
| --- | --- | --- |
| **L0 — bandeau** | repos | une ligne de 28 px en haut, toutes les mesures, séparées par des points médians ; sous un filet pleine largeur |
| **L1 — héros** | infusion, purge | une valeur à 104 px au centre + son filet de progression + deux valeurs secondaires à 40 px ; le bandeau L0 reste, atténué |
| **L2 — plein écran** | boot, OTA, faute, verrou | tout le reste disparaît ; un titre, une phrase, éventuellement un filet de progression |
| **L3 — feuille** | profils, pavé numérique | recouvre le bas de l'écran sur `bg_raised`, le bandeau L0 reste visible |
| **L4 — veille** | 30 min sans touche ni infusion | recouvre tout ; un petit bloc qui se déplace lentement. Voir « Veille » |

**Quelle valeur est héros en L1** : c'est la cible qui décide, pas un réglage.

- balance présente → **le poids** est héros, le temps est secondaire ;
- balance absente → **le temps** est héros, le volume estimé est secondaire ;
- purge → **le temps** est héros.

C'est le même mécanisme qui pilote le libellé du bouton (voir ci-dessous) :
une seule notion d'« objectif courant », lue à deux endroits.

### Composition du bandeau L0, état par état

Le bandeau n'affiche pas toujours les mêmes grandeurs : il montre **ce qui est
pertinent maintenant**, et rien d'autre. Une mesure sans objet n'est pas
affichée en `—`, elle est **absente** — la règle du `—` vaut pour une valeur
attendue qui manque, pas pour une valeur qu'on n'a aucune raison d'attendre.

| État | À gauche | À droite, dans l'ordre |
| --- | --- | --- |
| Repos, balance présente | profil | température · poids · ⚖ · ᯤ |
| Repos, **balance absente** | profil | température · ⚖ · ᯤ — **aucun champ de poids** |
| Infusion | profil | température · pression · débit (rampe) · pompe (rampe) · ⚖ · ᯤ atténuée |
| Purge | `purge` | **pression** · pompe (rampe) · goutte · ᯤ atténuée |
| Fin d'infusion | profil | température · ⚖ · ᯤ |
| Feuille L3 | titre de la feuille | température · ⚖ · ᯤ |

Sans balance, il n'y a **rien** à la place du poids : afficher `— g` en
permanence sur une machine qu'on utilise sans balance serait un reproche
affiché en continu. L'icône balance atténuée suffit à dire que la fonction
existe et dort.

**La purge montre la pression** : c'est le seul moment où elle est
l'information utile (backflush, contrôle de l'OPV), et le débitmètre n'y veut
rien dire — la pompe recircule (`firmware.md`, section débitmètre).

---

## Les écrans

### Repos

Trois zones, rien d'autre.

1. **Bandeau L0** — à gauche le nom du profil, touchable (ouvre la feuille de
   profils) ; à droite les mesures et les présences, composées selon la table
   ci-dessus. Les icônes de présence sont en `text` quand présentes, en
   `text_faint` quand absentes — jamais en rouge, une balance éteinte n'est pas
   une faute.
2. **Cible au centre**, 76 px, encadrée de deux zones `−` et `+` de 88 px, en
   contour comme les boutons. **Toujours visibles** : pas de mode « édition »,
   pas de découverte à faire. Un appui = un pas (0,5 g ou 1 s), **pas de
   répétition sur appui long** — c'est un réglage qu'on bouge de quelques
   crans, pas une molette de volume. Pour un grand écart, on tape le chiffre :
   **le pavé numérique** (L3) s'ouvre.
3. **Trois boutons** en bas : *infuser* (primaire), *purge*, *réglages*.

Sous la cible, une ligne d'étiquette rappelle les paramètres du profil courant
(pré-infusion, rampe) sans les rendre touchables — on les modifie dans
*réglages*, pas ici.

### Contexte : ce que fait le bouton primaire

Le bouton ne s'appelle jamais « démarrer ». Il dit ce qui va se passer :

| Condition | Bouton | Cible affichée |
| --- | --- | --- |
| Balance Acaia connectée | **infuser · 36 g** | poids, en `g`, pas 0,5 g |
| Pas de balance | **infuser · 28 s** | temps, en `s`, pas 1 s |
| Verrou 60 s actif | *(bouton absent)* | L2 plein écran, voir plus bas |
| Dimmer pas prêt (`READY=0`) | **infuser** grisé, non touchable | étiquette « dimmer en calibration » |

Le basculement poids ↔ temps est **automatique et silencieux** : brancher la
balance change la cible affichée et le libellé du bouton. Aucun réglage
« mode » à faire quelque part. C'est le point d'interface le plus important du
projet, et il tient en une règle.

### Infusion

Bandeau L0 atténué (les mesures continuent de vivre, elles ne captent plus le
regard), héros L1 au centre, et **un bouton d'arrêt de 320 × 88** centré en
bas. Pas plus large : le cas normal est **l'arrêt automatique** au poids ou au
temps ; ce bouton est un secours, il doit être impossible à rater mais il n'a
pas à occuper la surface qui sert à lire le shot en cours. Le reste de l'écran
n'est pas touchable pendant l'infusion (un chiffon qui passe ne coupe rien).

Au-dessus du héros, **la phase courante** en étiquette ambre :
`pré-infusion` → `infusion` → `rampe`. C'est la seule information de l'écran
qui ne soit pas une mesure, et c'est celle qui explique pourquoi la pression
fait ce qu'elle fait.

**Pas de graphe temps réel** (les implémentations Gaggiuino en mettent un ;
il n'apporte rien pendant qu'on regarde couler, et il est illisible à 1,5 m).
Mais le corps de cet écran est **une zone interchangeable** : y substituer un
`lv_chart` pression/débit ne touche ni le bandeau, ni la barre d'arrêt, ni la
machine à états. Si le besoin vient, c'est un basculement local, pas une
refonte — et un `lv_chart` de 600 × 260 en repaint partiel tient dans le
budget de bande passante, contrairement à un plein écran.

### Fin d'infusion

Le héros devient le résultat (`36,4 g`), les secondaires donnent le temps total
et le débit moyen. Reste **15 s** puis retour au repos tout seul. Un bouton
*fermer* discret pour ceux qui n'attendent pas. Aucun jugement affiché (pas de
« bon shot » / « trop rapide ») : la machine mesure, elle ne note pas.

### Profils (feuille L3)

Une **sélection du profil courant**, pas un bouton par profil : des lignes de
88 px, nom à gauche, résumé à droite (`36 g · pré-inf. 6 s`), la ligne active
marquée par un filet ambre à sa gauche — pas une case cochée. Quatre lignes
visibles, défilement vertical si plus.

Tant que les profils n'existent pas, cette feuille n'est pas construite : le
nom du profil dans le bandeau reste affiché mais n'est pas touchable. Rien
d'autre dans l'UI n'a à changer le jour où ils arrivent.

### Réglages

Même grammaire que le repos : une liste de lignes de 88 px, valeur à droite,
`−`/`+` au tap sur la ligne. Contenu : cible temps, cible poids, stratégie de
pré-infusion (temps fixe / attente de pression, avec le seuil), stratégie de
ramp-down (temps avant fin / poids / chute de pression), Wi-Fi (dont
*réinitialiser le réseau*, avec confirmation), calibrations, version du
firmware. **Pas de luminosité** : voir « Veille ».

Les stratégies sont des **choix parmi 2-3**, présentés en segments côte à côte
(contour, celui qui est actif en ambre), pas en menu déroulant — un déroulant
demande deux touches précises et une liste flottante.

### Diagnostic (feuille L3)

**Un appui sur le groupe d'icônes du bandeau ouvre la page de diagnostic.**
C'est le seul raccourci caché de l'interface, et il est justifié : ces icônes
sont exactement ce qu'on regarde quand on se demande « est-ce qu'il voit
vraiment mes capteurs ? ». La cible tactile fait 88 px de haut, centrée sur le
bandeau et débordant sous le filet — la seule cible de l'UI qui déborde d'une
zone.

Une liste, en lecture seule, de lignes de 72 px : à gauche le capteur, au
centre sa valeur instantanée, à droite son état brut. Pas de graphique, pas de
bouton, pas de possibilité d'agir sur quoi que ce soit.

| Ligne | Valeur affichée | À droite |
| --- | --- | --- |
| Pression | `9,1 bar` | brut `pressure_raw`, `valide` / `absent` (bit0) |
| Température | `92,4°` | brut `temperature_raw` |
| Débit | `2,1 ml/s` | impulsions cumulées, âge de la dernière |
| Pompe | `100 %` | `prêt` / `calibration` / `erreur` (bits 1-2) |
| Vanne | `ouverte` / `fermée` | bail restant, marche continue |
| Balance | `36,2 g` | `connectée` / `absente`, âge de la dernière pesée |
| Bus CAN | `ok` / `perdu` | compteurs d'erreur TWAI, dernier code `LOG` |
| Réseau | adresse IP | `connecté` / `coupé (infusion)` / `absent` |
| Versions | `écran 0,2,3` | `capteurs 0,1,7`, uptime des deux |

**Les valeurs brutes sont affichées telles quelles**, sans calibration : c'est
la seule page de l'interface qui montre ce que le bus transporte réellement.
C'est ce qui permet de diagnostiquer une sonde débranchée depuis la façade,
sans sortir le Mac ni ouvrir la machine — et c'est le pendant à l'écran de ce
que `coffeetool monitor` donne sur le Mac.

Tant qu'elle est ouverte, les périodes `REQSTATUS` passent à celles de
l'infusion (100 ms) pour que les valeurs vivent. **Inaccessible pendant une
infusion ou une purge** : le bandeau n'est pas une cible à ce moment-là.

### Plein écran (L2)

Un seul gabarit pour tous ces cas : titre 48 px, phrase 24 px en `text_dim`,
filet de progression optionnel, un bouton optionnel.

| Cas | Titre | Ce que dit la phrase |
| --- | --- | --- |
| Boot | `coffeeflow` | version, puis disparaît |
| Mise à jour | `mise à jour` | cible (écran/capteurs) + filet de progression. **Aucun bouton** : on ne coupe pas un OTA par mégarde |
| Verrou 60 s | `verrou de sécurité` (`fault`) | « couper la machine à l'interrupteur principal pour réarmer » — la seule sortie réelle, autant l'écrire |
| Bus CAN perdu | `module interne injoignable` (`fault`) | « les commandes sont coupées » |
| Dimmer en calibration | *(pas de L2)* | reste au repos, bouton primaire grisé |

Le verrou et la perte de bus **prennent tout l'écran** parce qu'ils rendent la
machine inutilisable : afficher un petit badge alors qu'aucun bouton ne
fonctionne serait pire que d'afficher la panne.

---

## Notes de mise en œuvre LVGL

- **LVGL v9.x** (9.3 courante), via le composant managé
  `espressif/esp_lvgl_port` + `esp_lcd` RGB. v8 est en fin de vie ; l'exemple
  officiel Waveshare (`waveshareteam/ESP32-S3-Touch-LCD-4.3`, cloné dans
  `tmp/`) est la référence de bring-up de la dalle et du GT911.
- **Bounce buffer obligatoire** sur ce panneau RGB (`CONFIG_LCD_RGB_BOUNCE_BUFFER`) :
  sans lui, le framebuffer en PSRAM déchire dès qu'une tâche prend le bus.
  C'est un réglage de `sdkconfig`, pas un problème d'UI, mais il conditionne
  tout ce qui a été dit sur les animations.
- **Pas d'`lv_style` par objet** : une table de styles partagés
  (`style_hero`, `style_value`, `style_label`, `style_button`,
  `style_button_pressed`, `style_hairline`) initialisée une fois. LVGL v9 les
  applique par référence ; en dupliquer un par widget mange la RAM interne.
- **Un écran = une fonction `create_*`**, pas de `lv_scr_load` à répétition :
  les niveaux L0/L1 vivent sur le même écran et changent d'attributs
  (`lv_obj_add_flag(..., LV_OBJ_FLAG_HIDDEN)`, changement de style), les
  niveaux L2/L3 sont des `lv_obj` superposés. Ça évite de recréer l'arbre à
  chaque transition d'état d'infusion.
- **Rafraîchissement** : la télémétrie arrive par CAN à la période demandée par
  `REQSTATUS`. Rafraîchir les libellés à **10 Hz maximum**, même si les trames
  arrivent plus vite — au-delà, c'est illisible et ça repeint pour rien. Le
  filet de progression s'anime, lui, en continu.
- **Le tactile n'a aucun geste** : que des appuis. Pas de balayage, pas d'appui
  long (sauf éventuellement un accès de service caché dans *réglages*), pas de
  double appui. Un capacitif derrière une vitre, avec un doigt humide, ne
  reconnaît un geste qu'une fois sur deux.
- **Veille** : voir la section dédiée ci-dessous. Le rétroéclairage n'est
  **jamais** éteint et n'est jamais modulé — il est sur EXIO2 du CH422G, une
  sortie tout ou rien sans PWM ; l'atténuation est un calque LVGL. La dalle
  n'est jamais ré-initialisée : ça prend du temps et fait un flash blanc.

---

## Veille

**La machine est allumée par session d'une trentaine de minutes** — chauffe,
un ou deux shots, puis on coupe à l'interrupteur. C'est cette durée qui décide
de toute la veille, et elle explique pourquoi **il n'y a pas de réglage de
luminosité** : sur une session aussi courte, personne n'ira chercher un
curseur, et la seule luminosité utile est le maximum. Ce qu'il faut à la place
est une courbe automatique, à trois temps.

| Temps sans touche ni infusion | État | Ce qui se passe |
| --- | --- | --- |
| 0 | pleine intensité | l'écran normal, tel que décrit plus haut |
| **4 min** | atténué | un calque noir à ~50 % d'opacité au-dessus de l'arbre. Rien ne bouge, rien ne disparaît : l'écran reste lisible de près, il cesse juste d'éclairer la cuisine |
| **30 min** | veille (L4) | tout est recouvert par un bloc unique qui se déplace |

Trois choses que cette veille **ne fait pas** :

- **Elle ne module pas le rétroéclairage.** Il est sur EXIO2 du CH422G, une
  sortie tout ou rien, sans PWM. L'atténuation est un calque LVGL noir dont on
  change l'opacité — un seul palier, donc un seul objet, et un repeint par
  transition (pas une animation, la contrainte de bande passante ne s'y
  applique pas).
- **Elle n'éteint jamais la dalle.** Un écran noir dans une façade noire se lit
  comme une machine éteinte, alors qu'elle est chaude et sous tension. C'est
  précisément l'information qu'on ne veut pas effacer.
- **Elle ne se déclenche jamais pendant une infusion ou une purge**, ni
  pendant une mise à jour. Le compteur repart de zéro à chaque touche et à
  chaque changement d'état.

### Le bloc de veille

Un seul bloc, centré sur lui-même, qui **change de position toutes les 60 s**
en tirant au sort un emplacement dans une zone en retrait de 80 px des bords.
Il contient, en `text_dim` sur `bg` :

- la **température du groupe**, en gros — c'est la seule chose qu'on veut
  savoir de loin sur une machine qui chauffe depuis vingt minutes, et elle est
  déjà là (`STATUS_PRESSURE` porte la température) ;
- **l'heure**, en dessous, **uniquement si elle est connue** — c'est-à-dire si
  le Wi-Fi s'est associé et que le SNTP a abouti (`firmware.md`, « L'heure
  vient du réseau »). Sans réseau, la ligne est simplement absente : pas de
  `--:--`, pas d'heure fausse. C'est la seule horloge de toute l'interface, et
  elle n'existe qu'ici, là où l'écran n'a rien de mieux à montrer ;
- le nom du profil courant, en étiquette.

Le déplacement n'est pas de la prévention de marquage : cette dalle est un LCD,
une image fixe ne lui fait rien. C'est une preuve de vie — un écran qui bouge
lentement dit « allumée, au repos » sans avoir à l'écrire.

**Réveil** : n'importe quelle touche ramène l'écran normal à pleine intensité,
et **ne déclenche aucune action** (règle déjà posée dans les cas limites).
Une infusion, une purge, une mise à jour ou une faute réveillent aussi l'écran,
et les plein écran L2 préemptent la veille comme ils préemptent tout le reste.

Les deux seuils (4 min, 30 min) sont en NVS mais **ne sont pas dans l'écran de
réglages** : ce sont des valeurs qu'on ajuste une fois, depuis `/config`, si
jamais on les ajuste. Une ligne de plus dans les réglages tactiles coûte plus
cher qu'elle ne rapporte.

---

## Cotes exactes

Tout est posé sur une grille verticale de 8 px, marges **32 px** à gauche et à
droite, **24 px** en haut et en bas. Ces coordonnées sont normatives : elles
évitent d'avoir à « placer à l'œil » en phase 6.

| Élément | x | y | l × h |
| --- | --- | --- | --- |
| Bandeau L0 (texte) | 32 → 768 | 24 | 736 × 44 |
| Filet sous le bandeau | 32 → 768 | 82 | 736 × 1 |
| Bloc héros L1 (haut du bloc) | centré | 120 | — |
| — étiquette de phase | centré | 120 | h 22 |
| — chiffre héros 104 px | centré | 156 | h 104 |
| — filet de progression | centré | 282 | 420 × 2 |
| — valeurs secondaires | centré, écart 52 | 304 | h 46 |
| Bloc héros L0 (repos, haut) | centré | 150 | — |
| — `−`, chiffre 76 px, `+` | centré, écart 56 | 150 | boutons 88 × 88 |
| — étiquette de paramètres | centré | 268 | h 22 |
| Rangée de boutons | 32 → 768 | 368 | 736 × 88, gouttière 24 |
| Bouton d'arrêt (seul) | centré | 368 | 320 × 88 |
| Feuille L3 | 0 → 800 | 90 | 800 × 390 |
| Ligne de liste (L3, réglages) | 32 → 768 | — | 736 × 88 |

Toute zone tactile fait **au minimum 88 × 88**, y compris quand son dessin est
plus petit : la cible est agrandie par du remplissage transparent, pas par du
contour visible.

---

## Modèle de données de l'UI

**Un seul état, une seule structure.** Aucun widget ne détient de valeur : la
tâche protocole remplit `ui_model_t`, l'UI le lit à 10 Hz et met à jour les
libellés. C'est ce qui permet de rejouer un shot enregistré
(`coffeetool recorder`, phase 1) dans l'UI sans matériel.

```c
typedef enum { UI_IDLE, UI_BREW, UI_PURGE, UI_DONE, UI_SHEET, UI_FULLSCREEN } ui_state_t;
typedef enum { BREW_BY_WEIGHT, BREW_BY_TIME } ui_goal_t;
typedef enum { PHASE_PREINFUSION, PHASE_EXTRACTION, PHASE_RAMP } ui_phase_t;

typedef struct {
  ui_state_t state;
  ui_goal_t  goal;            // dérivé de scale_present, jamais réglé à la main
  ui_phase_t phase;

  float   temperature_c;      // depuis STATUS_PRESSURE, calibration écran
  float   pressure_bar;       // idem
  float   flow_ml_s;          // dérivé de STATUS_FLOW (facteur K, écran)
  float   flow_target_ml_s;   // 0 = pas de flow control → rampe niveau 1 max
  float   weight_g;           // BLE Acaia
  uint8_t dimmer_pct;         // écho de STATUS_ACTUATORS
  uint8_t pump_target_pct;    // niveau demandé par le profil, pour la rampe
  bool    valve_open;         // écho du bit ssr

  bool    scale_present;      // BLE connecté ET pesée reçue < 2 s
  bool    wifi_connected;     // associé ET adresse IP ; faux pendant une infusion
  bool    sensors_alive;      // trafic CAN reçu < 3 s
  bool    pressure_valid;     // STATUS_PRESSURE flags bit0
  bool    dimmer_ready;       // STATUS_ACTUATORS flags bit1
  bool    lockout;            // STATUS_ACTUATORS flags bit0
  bool    flash_active;       // opération OTA acceptée par le cœur, préempte en L2
  uint8_t flash_target;       // écran ou capteurs
  uint32_t flash_bytes_done;
  uint32_t flash_bytes_total;
  uint32_t last_status_ms;    // pour la péremption d'affichage

  float   target_weight_g, target_time_s;
  float   elapsed_s;
  char    profile_name[16];
} ui_model_t;
```

`goal` n'est **jamais** un réglage utilisateur : `scale_present ?
BREW_BY_WEIGHT : BREW_BY_TIME`, réévalué à chaque rafraîchissement au repos,
**gelé au démarrage d'une infusion** (voir les cas limites).

### Périodes de télémétrie (`REQSTATUS`)

| État | `STATUS_PRESSURE` | `STATUS_FLOW` | `STATUS_ACTUATORS` |
| --- | --- | --- | --- |
| Repos | 500 ms | 1000 ms | 1000 ms |
| Infusion / purge | **100 ms** (le plancher) | 100 ms | 200 ms |
| Plein écran (OTA) | 0 (arrêt) | 0 | 0 |

Ce trafic périodique est aussi ce qui **entretient la présence** côté capteurs
(`tick_presence()`, voir `firmware-implementation.md` phase 5) : au repos,
1 trame/s au minimum suffit à ne jamais retomber dans le cycle
`PRESENCE_LOST` → `PING` observé pendant tout le bring-up. À ne pas descendre
en dessous de 1 Hz, même en veille écran.

L'UI **ne rafraîchit jamais un libellé plus de 10 fois par seconde**, même si
les trames arrivent à 100 ms : au-delà c'est illisible, et ça repeint pour
rien sur un panneau RGB.

### Formatage des valeurs

**Virgule décimale**, une seule décimale partout sauf le débit moyen du
résumé (deux). Unité toujours détachée du nombre par une espace fine, en
`text_dim`, jamais dans le même corps.

| Grandeur | Format | Exemple |
| --- | --- | --- |
| Température | `%.1f°` | `92,4°` |
| Pression | `%.1f bar` | `9,1 bar` |
| Débit | `%.1f ml/s` | `2,1 ml/s` |
| Poids | `%.1f g` | `36,2 g` |
| Temps (en cours) | `%.1f s` | `21,6 s` |
| Temps (cible) | `%d s` | `28 s` |
| Débit moyen (résumé) | `%.2f g/s` | `1,26 g/s` |

**Valeur absente ou invalide : `—` en `text_faint`, jamais un `0,0`.** Un zéro
affiché à la place d'une mesure manquante est le seul mensonge que cette
interface peut faire ; il est interdit. Règle de péremption : une valeur dont
la dernière trame date de plus de **2 × sa période demandée** passe en
`text_dim`, de plus de **3 s** passe à `—`.

`pressure_valid = false` (bit0 de `StatusPressurePayload::flags`) →
pression **et** température à `—` : les deux viennent du même XDB401.

---

## Machine à états de l'UI

```
        ┌──────────────── (15 s, ou « fermer ») ───────────────┐
        ↓                                                      │
     UI_IDLE ──[ infuser ]──→ UI_BREW ──[ cible atteinte ]──→ UI_DONE
        │  ↑                     │  ↑         [ arrêter ]      │
        │  │                     └──[ arrêter ]────────────────┘
        │  └──[ relâché / 20 s ]── UI_PURGE
        ├──[ appui maintenu purge ]──↑
        └──[ profil / réglage ]──→ UI_SHEET ──[ valider / annuler ]──→ UI_IDLE

  UI_FULLSCREEN préempte tout état, à tout moment, et le rend au retour.
```

Transitions, **qui les déclenche** :

| Transition | Déclencheur | Effet côté CAN |
| --- | --- | --- |
| `IDLE → BREW` | appui *infuser* | tare balance, `REQSTATUS` en période courte, la boucle d'infusion prend la main sur les `SET` |
| `BREW → DONE` | cible atteinte (algorithme), appui *arrêter*, ou **perte de la balance** en brew by weight | `SET ssr=0 dimmer=0`, `REQSTATUS` en période longue |
| `IDLE → PURGE` | **appui maintenu** sur *purge* | `SET ssr=1 dimmer=<niveau purge>` renouvelé à 10 Hz |
| `PURGE → IDLE` | doigt relâché, ou 20 s écoulées | arrêt des `SET`, le bail retombe seul |
| `IDLE → FULLSCREEN` | OTA accepté par le cœur, seulement si les actionneurs sont confirmés à l'arrêt | voir la priorité ci-dessous |
| `* → FULLSCREEN` | verrou / bus perdu / boot | voir la priorité ci-dessous |

**La purge est un homme-mort** : elle coule tant que le doigt est posé,
plafonnée à 20 s. C'est le seul geste maintenu de l'interface, et il est
justifié — une purge lancée puis oubliée arrose le plan de travail, et le bail
du protocole ne protège que d'un écran mort, pas d'un humain distrait.

### Priorité des plein écran (L2)

Un seul L2 à la fois, dans cet ordre décroissant. Le plus prioritaire présent
gagne, sans exception :

1. **Verrou de sécurité** (`flags` bit0 de `STATUS_ACTUATORS`)
2. **Module interne injoignable** (aucune trame CAN reçue depuis 3 s)
3. **Mise à jour en cours** (OTA local ou distant)
4. **Boot** (2 s maximum, puis repos)

Une mise à jour interrompue par une perte de bus affiche donc l'écran « module
injoignable », avec la phrase de reprise (« mise à jour interrompue, l'image
précédente est intacte ») — pas un écran d'OTA figé à 46 %.

Entrer en L2 **coupe les actionneurs** (`SET ssr=0 dimmer=0`) si une infusion
ou une purge était en cours, avant même de dessiner. L'affichage n'est jamais
la première chose qu'on fait.

Une mise à jour, elle, n'est jamais le moyen de provoquer cet arrêt : le cœur
ne l'accepte qu'en `UI_IDLE`, sans infusion ni purge, et si le dernier état
actionneur frais confirme SSR fermé et pompe à zéro. Sinon elle est refusée et
l'interface courante reste affichée. Une fois acceptée, `flash_active` et ses
compteurs figent l'UI en L2 « mise à jour » jusqu'au résultat ; aucun appui ne
peut interrompre l'opération.

---

## Cas limites, tous tranchés

Ce sont les questions qui se posent en écrivant le code. Elles sont décidées
ici pour ne pas l'être à ce moment-là.

| Situation | Décision |
| --- | --- |
| **La balance se déconnecte pendant un brew by weight** | **arrêt immédiat** de l'infusion, passage en `UI_DONE` avec l'étiquette `balance perdue` en `fault` et le dernier poids connu. Pas de repli sur un temps cible : quand on infuse au poids, c'est la balance qui décide, et un temps cible n'est pas forcément à jour ni même défini. Le cas est rare (jamais rencontré en quatre ans d'usage) et un shot coupé se rattrape ; un shot qui continue à l'aveugle déborde. Critère de perte : plus aucune pesée reçue depuis 2 s (c'est-à-dire `scale_present` qui retombe). |
| **La balance se connecte pendant un brew by time** | rien ne change pour ce shot (`goal` est gelé), le poids apparaît en valeur secondaire. Le shot suivant sera au poids. |
| **La tare ne répond pas au départ** | on part quand même après 1,5 s, étiquette `tare non confirmée`, et l'arrêt au poids utilise le delta depuis la valeur lue au départ. |
| **Poids qui recule** (tasse bougée) | pas d'arrêt anticipé sur un maximum ; on suit la valeur courante. Un recul de plus de 5 g est traité comme une perte de balance : **arrêt**, même raisonnement que ci-dessus (la mesure n'est plus fiable, continuer déborderait). |
| **Cible atteinte alors que le dimmer n'a jamais démarré** | l'infusion se termine normalement ; le résumé affiche les mesures telles quelles, sans commentaire. |
| **`dimmer_ready == false` au repos** | bouton primaire grisé et non touchable, étiquette `dimmer en calibration — vérifier le secteur`. Ce n'est **pas** un plein écran : la machine est simplement pas prête, comme un boiler froid. |
| **`dimmer_ready` retombe pendant une infusion** | l'infusion continue (le module reste joignable en I2C et garde son niveau, vérifié en phase 5), étiquette `secteur instable`. Pas d'arrêt : couper un shot à cause d'un flicker serait pire. |
| **Appui sur `+` au-delà du maximum** | la valeur bute, le bouton passe en `text_faint`. Pas de bip, pas de secousse, pas de bouclage à la valeur minimale. |
| **Trame `STATUS_*` reçue pendant un L2** | le modèle est mis à jour, l'affichage ne change pas. Le retour du L2 montre des valeurs fraîches, jamais gelées. |
| **Appui pendant une infusion, ailleurs que sur *arrêter*** | ignoré, aucun retour visuel. Le reste de l'écran n'est pas une cible. |
| **Réveil depuis l'atténuation ou la veille** | la touche qui réveille **ne déclenche aucune action**. Réveiller et agir sont deux gestes. |
| **Wi-Fi coupé au départ d'une infusion** | l'icône s'atténue, rien d'autre. Pas de message, pas de `fault` : c'est la politique radio (`firmware.md`), pas une panne. Elle se rallume seule au retour au repos. |
| **Wi-Fi jamais configuré** | icône atténuée en permanence. Aucun rappel, aucune invitation à configurer : la machine fait café sans réseau. |
| **Diagnostic ouvert quand le bus tombe** | la feuille se ferme et laisse la place au L2 « module injoignable » — la priorité des plein écran s'applique aussi aux feuilles. |
| **Verrou levé (retour de `flags` bit0 à 0)** | retour direct au repos, sans écran intermédiaire. Le verrou ne se lève que sur démarrage à froid, donc en pratique c'est un boot. |

---

## Textes

Toute la chaîne d'affichage est ici. **Une seule table**, en minuscules, sans
point final, sans jargon protocolaire visible (jamais « CAN », « TWAI »,
« bail », « SSR » à l'écran).

| Clé | Texte |
| --- | --- |
| `btn.brew.weight` | `infuser · %.0f g` |
| `btn.brew.time` | `infuser · %d s` |
| `btn.purge` | `purge` |
| `btn.settings` | `réglages` |
| `btn.stop` | `arrêter` |
| `btn.close` | `fermer` |
| `btn.cancel` / `btn.ok` | `annuler` / `valider` |
| `lbl.target` | `cible` |
| `lbl.phase.pre` | `pré-infusion` |
| `lbl.phase.pre.pressure` | `pré-infusion · attente %.0f bar` |
| `lbl.phase.brew` | `infusion` |
| `lbl.phase.ramp` | `rampe` |
| `lbl.done` | `terminé` |
| `lbl.scale_lost` | `balance perdue — infusion arrêtée` |
| `lbl.tare_unconfirmed` | `tare non confirmée` |
| `lbl.dimmer_cal` | `dimmer en calibration — vérifier le secteur` |
| `lbl.mains_unstable` | `secteur instable` |
| `diag.title` | `diagnostic` |
| `diag.rows` | `pression` · `température` · `débit` · `pompe` · `vanne` · `balance` · `bus can` · `réseau` · `versions` |
| `diag.valid` / `diag.absent` | `valide` / `absent` |
| `diag.valve` | `ouverte` / `fermée` |
| `diag.scale` | `connectée` / `absente` |
| `diag.net` | `connecté` / `coupé (infusion)` / `absent` |
| `full.boot.title` | `coffeeflow` |
| `full.ota.title` | `mise à jour` |
| `full.ota.body` | `%s · bloc %d / %d` puis `ne pas couper la machine` |
| `full.ota.aborted` | `mise à jour interrompue — l'image précédente est intacte` |
| `full.lock.title` | `verrou de sécurité` |
| `full.lock.body` | `la pompe est restée en marche plus de 60 s. couper la machine à l'interrupteur principal, puis la rallumer` |
| `full.bus.title` | `module interne injoignable` |
| `full.bus.body` | `le module interne ne répond plus. les commandes de pompe et de vanne sont coupées` |

Seule exception à la règle « pas de jargon protocolaire » : la page de
diagnostic, dont c'est précisément le sujet — on y va pour savoir ce que le bus
raconte.

Français uniquement, pas d'infrastructure d'internationalisation : une machine,
un utilisateur, et chaque chaîne de plus est du flash en moins.

---

## Réglages : valeurs, plages, pas, stockage

Tout est en NVS côté écran (`firmware.md` : les calibrations vivent ici, un
remplacement de XIAO ne fait rien perdre). Espace de noms `ui`.

| Réglage | Clé NVS | Défaut | Min | Max | Pas |
| --- | --- | --- | --- | --- | --- |
| Cible poids | `tgt_w` | 36,0 g | 10 g | 100 g | **0,5 g** |
| Cible temps | `tgt_t` | 28 s | 5 s | 60 s | **1 s** |
| Stratégie pré-infusion | `pi_mode` | `temps` | — | — | `temps` / `pression` |
| Pré-infusion, durée | `pi_t` | 6 s | 0 s | 20 s | 1 s |
| Pré-infusion, seuil | `pi_bar` | 4,0 bar | 1 bar | 9 bar | 0,5 bar |
| Pré-infusion, niveau pompe | `pi_pct` | 30 % | 0 % | 100 % | 5 % |
| Stratégie ramp-down | `rd_mode` | `aucune` | — | — | `aucune` / `temps` / `poids` / `chute de pression` |
| Ramp-down, avance | `rd_t` | 3 s | 0 s | 15 s | 0,5 s |
| Ramp-down, avance poids | `rd_g` | 4,0 g | 0 g | 20 g | 0,5 g |
| Ramp-down, chute | `rd_bar` | 1,0 bar | 0,5 bar | 4 bar | 0,5 bar |
| Niveau pompe infusion | `br_pct` | 100 % | 20 % | 100 % | 5 % |
| Niveau pompe purge | `pg_pct` | 100 % | 20 % | 100 % | 5 % |
| Purge, durée maximale | `pg_max` | **20 s** | 5 s | 60 s | 5 s |
| Atténuation après | `dim_s` | **240 s** | 60 s | 1800 s | 60 s |
| Veille après | `sby_s` | **1800 s** | 300 s | 3600 s | 300 s |

Les deux dernières lignes n'apparaissent **pas** dans l'écran de réglages
tactile (voir « Veille ») : elles ne vivent qu'en NVS et dans `/config`. Il n'y
a **pas de réglage de luminosité** — la session dure une trentaine de minutes,
la seule intensité utile est le maximum, et l'atténuation est automatique.

Ces défauts sont des points de départ raisonnables, pas des vérités : ils
seront ajustés à la calibration (`firmware.md`, section dédiée). Ce qui compte
est que **rien ne manque au moment d'écrire l'écran de réglages**.

Les réglages qui n'ont de sens qu'avec une stratégie donnée (`pi_bar` sans le
mode pression) restent **visibles mais atténués**, pas cachés : une ligne qui
disparaît fait douter de l'avoir rêvée.

---

## Découpage des fichiers

```
firmware/screen/main/ui/
  ui_theme.h       jetons (couleurs, corps, cotes), styles partagés — aucune logique
  ui_model.h       ui_model_t + accesseurs ; rempli par la tâche protocole
  ui_root.c        création de l'écran unique, bandeau L0, aiguillage d'état
  ui_home.c        repos : cible ±, rangée de boutons
  ui_brew.c        L1 : héros, filet de progression, secondaires, arrêt
  ui_sheet.c       L3 : pavé numérique, profils, réglages
  ui_full.c        L2 : le gabarit unique + ses quatre cas
  ui_fonts/        polices générées par lv_font_conv (ne pas éditer)
```

Règles qui tiennent l'ensemble :

- **`ui_theme.h` n'inclut rien d'autre que LVGL.** Aucun fichier d'UI ne
  connaît le protocole : il lit `ui_model_t`, point.
- **Aucun `ui_*.c` n'envoie de trame.** Un appui produit un événement
  (`ui_event_brew_start`, `ui_event_stop`, …) consommé par la boucle
  d'infusion, qui est seule à émettre des `SET`. C'est ce qui permet de tester
  l'UI sur un modèle rejoué et l'algorithme sans écran.
- **Une seule tâche touche LVGL** (celle d'`esp_lvgl_port`) ; le modèle est
  publié depuis la tâche CAN sous verrou, jamais un widget mis à jour depuis
  une autre tâche.

---

## Ordre d'écriture, et critère de sortie de chaque lot

Le point 6 de la phase 6 se découpe en cinq lots. Chacun est utilisable seul,
et chacun a de quoi savoir qu'il est fini.

1. **Dalle et jetons** — `esp_lcd` RGB + GT911 + `esp_lvgl_port`, bounce
   buffer, `ui_theme.h`, polices générées. *Fini quand* : un écran de test
   affiche les huit couleurs et les sept corps de texte, et qu'un appui
   quelque part change une couleur (le tactile répond).
2. **Bandeau L0 + repos statique** — sur des valeurs figées en dur. *Fini
   quand* : la maquette du cadre 1 est reproduite au pixel près.
3. **Modèle vivant** — `ui_model_t` alimenté par le CAN réel, péremption,
   `—` sur valeur absente, bascule poids/temps sur présence de la balance.
   Page de diagnostic (elle n'est que le modèle rendu ligne à ligne : c'est le
   moment le moins cher pour l'écrire).
   *Fini quand* : débrancher le XDB401 met la pression à `—` en moins de 3 s
   **et le montre en `absent` dans le diagnostic**, et débrancher le câble CAN
   fait apparaître le L2 « module injoignable ».
4. **Interaction** — `−`/`+`, pavé numérique, NVS, réglages, purge homme-mort.
   *Fini quand* : une cible modifiée survit à une coupure d'alimentation, et
   la purge s'arrête au relâchement **et** à 20 s.
5. **Infusion** — L1, phases, filet de progression, arrêt automatique et
   manuel, résumé, cas limites de la table ci-dessus. *Fini quand* : un shot
   enregistré (`coffeetool recorder`) rejoué dans le modèle produit le même
   écran qu'en direct.

---

## Ce que cette interface ne fera jamais

À relire quand l'envie d'ajouter quelque chose se présentera.

- Pas de widgets déplaçables, pas de disposition configurable.
- Pas de thème clair, pas de choix de couleurs.
- Pas de graphe par défaut (l'accroche existe, elle reste vide).
- Pas de notation d'un shot, pas d'historique, pas de statistiques.
- Pas de saisie de texte au tactile (le provisioning Wi-Fi passe par la page
  web, décidé en phase 6 point 1).
- Pas de menu à plus d'un niveau de profondeur.
- Pas de terme technique du protocole visible par l'utilisateur.

---

## Ce qui reste vraiment ouvert

Réduit au minimum : tout le reste est tranché ci-dessus, y compris par défaut.

- **Le graphe temps réel** — hors périmètre, avec le point d'accroche décrit
  plus haut. À reconsidérer après quelques semaines d'usage réel.
- **Les profils** — la feuille est spécifiée et dessinée, rien n'est construit
  tant que la notion de profil n'existe pas côté algorithme. Quand elle
  existera, les clés NVS ci-dessus deviennent un enregistrement indexé par
  profil, et **rien d'autre dans l'UI ne bouge**.
- **Les défauts de réglage** — chiffres de départ, à ajuster à la calibration.
  Ce n'est pas une décision d'interface.

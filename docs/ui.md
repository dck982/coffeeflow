# UI de l'écran — style, structure, décisions

Ce document fixe **le style et la structure** de l'interface tactile, pas le
code. `docs/ui-mockup.html` reste une maquette historique à l'échelle ; en cas
d'écart, les cotes et critères de ce document sont normatifs. L'écran
d'infusion a son propre document, `ecran-infusion.md`.

Conception d'ensemble, protocole et `/config` : `firmware.md`.

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

**Sombre et chaud, typographique, sans cadres autour des données.** L'écran est dans un boîtier
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
3. **Une palette courte, mais pas monochrome.** L'ambre `#D98324` (couleur
   crema) porte l'action et l'engagement de la machine. Un bleu-vert
   `#4A9BB5` est réservé à la température. Les bruns des surfaces
   rendent les zones tactiles visibles sans les transformer en cadres. Le
   rouge `#B9412F` est **réservé aux fautes** — s'il apparaît ailleurs il ne
   veut plus rien dire.
4. **Pas de vert/orange/rouge en feux tricolores.** Un état normal est écrit en
   blanc cassé. La couleur signale l'action en cours, pas la santé.
5. **Une typographie humaniste**, pas un faux afficheur sept segments ni un
   DIN condensé. Chiffres à **chasse fixe** (`tabular figures`) pour qu'un
   `1` ne fasse pas sauter la ligne dix fois par seconde.
6. **Minuscules partout**, sauf les micro-étiquettes (18 px, interlettrage
   ouvert). Pas de `ALL CAPS` sur les valeurs.
7. **Pas de jauges, pas de cadrans, pas de barres épaisses.** Un seul élément
   graphique dans tout le système : le filet de progression (voir ci-dessous).

« Sans cadres » ne veut pas dire « sans surfaces » : une valeur reste posée sur
le fond, tandis qu'un contrôle touchable est une surface pleine, légèrement
plus claire. Ce contraste rend l'affordance tactile évidente et donne plus de
profondeur que les contours actuels. Les pastilles du diagnostic sont une
exception de service : ambre pour une donnée présente, gris pour une absence
et rouge pour une faute réelle.

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
| `bg` | `#16110C` | fond, partout |
| `bg_raised` | `#241B14` | barre haute, feuille et pavé numérique |
| `surface` | `#33271E` | bouton ou tuile secondaire au repos |
| `surface_high` | `#453426` | bouton pressé, choix sélectionné |
| `surface_accent` | `#4A2E16` | fond du bouton primaire |
| `hairline` | `#332C28` | filets de séparation (≈ 12 % de blanc chaud) |
| `text` | `#F2EBE3` | valeurs, titres |
| `text_dim` | `#A2968C` | unités, valeurs secondaires |
| `text_faint` | `#6B615A` | étiquettes, valeurs absentes (`-`) |
| `accent` | `#D98324` | action en cours, progression, bouton primaire |
| `accent_wash` | `#D98324` à 14 % | surimpression locale au toucher |
| `thermal` | `#4A9BB5` | température valide |
| `ambient` | `#A485D0` | lecture sonde brute, machine froide |
| `fault` | `#B9412F` | faute uniquement (verrou, perte de bus, erreur dimmer) |

Ces teintes doivent toutes figurer sur l'écran de test après conversion
RGB565. `surface`, `surface_high` et `surface_accent` doivent rester
distinguables sur la dalle réelle ; si deux se confondent, éclaircir
`surface_high`, pas le fond. **Pas de couleur « succès ».** Ce qui va bien
s'écrit en `text`.

`thermal` ne signifie ni erreur ni succès : la température l'emploie dès que
sa mesure est valide. Une mesure périmée emploie la même teinte à opacité
réduite ; une mesure absente passe en `text_faint`. Cette règle ne requiert
donc aucune nouvelle consigne dans le modèle. L'ambre demeure la couleur de
l'action hydraulique et de la cible.

**Machine froide, la température du bandeau est la lecture sonde.** Tant que
la température utilisateur reste sous 50 °C, le bandeau affiche `sensor_c`,
sans l'offset de −10,5 °C, en `ambient`. Allumée chauffe coupée, la machine
montre ainsi la température ambiante, contrôle visuel de la sonde. Entre
50 °C sonde et 50 °C utilisateur (60,5 °C sonde), l'affichage reste à 50,0
pour ne jamais reculer ; au-delà, la température utilisateur et ses couleurs
habituelles reprennent. Le violet est hors des teintes de chauffe : il ne se
confond pas avec une consigne ou un état de régulation.

### La rampe d'engagement

Une seule échelle de couleur, partagée par **la pompe et le débit**, qui dit
**à quel point la machine pousse** — pas si c'est bien ou mal. Quatre niveaux,
tous dérivés de la même famille ambre : ils décrivent une intensité, avec de
la luminosité en plus, et pas un feu tricolore.

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
sont d'au moins 40/255 par canal). Elles sont prévues pour la pompe et le débit.
Pendant un cycle, la pression peut aussi reprendre cette luminosité ambre pour
montrer son approche du seuil de pré-infusion ou des 9 bar, sans signaler un
état de santé.

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
ne pas faire « application mobile »). Les contrôles pleins n'ont pas de
contour ; les séparations d'information restent des filets de **1 px**.

### Police

**Inter** (SIL OFL, chiffres tabulaires natifs, dessinée pour les basses
résolutions), embarquée via `lv_font_conv`. `montserrat` intégré à LVGL sert
de repli pendant le bring-up, mais son `1` n'est pas tabulaire — à ne pas
garder pour l'affichage d'une valeur qui change.

Coût flash, à surveiller pour les partitions : sortir les gros corps en
**jeu de glyphes restreint**.
Le 104 px n'a besoin que de `0-9`, `.`, `g`, `s` — une douzaine de glyphes en
4 bpp ≈ 55 ko, contre ~700 ko pour un latin complet à ce corps. Les corps
texte (18/26/40) prennent le latin-1 complet.

Les icônes sont converties depuis **un seul jeu SVG libre** en une police
d'icônes par le même outil. Pas de PNG : il ne se recolore pas et mange la
partition de ressources. Le choix exact des glyphes peut être fait pendant
l'implémentation ; la spécification fixe leur sens, leur taille et leur
placement, pas leur dessin.

Les chevrons retour/précédent/suivant et le retour arrière du pavé proviennent
de cette même police ; ne pas employer des caractères Unicode dont la présence
dans Inter dépendrait du sous-ensemble de glyphes embarqué.

Les icônes de statut prévues font 24 px :

| Icône | Ce qu'elle dit | Couleur |
| --- | --- | --- |
| Wi-Fi | mode Wi-Fi actif et associé au réseau | `text` connecté / `text_faint` mode machine ou non associé |
| pompe | le régime de la pompe | rampe d'engagement, niveaux 0-3 |
| goutte | la vanne est ouverte | `accent` ouverte / `text_faint` fermée |

L'accueil ajoute trois icônes d'action de **32 px**, toutes issues de la même
famille et avec la même épaisseur de trait :

| Destination | Sens recherché | Couleur au repos |
| --- | --- | --- |
| infuser | tasse, porte-filtre ou extraction | `accent` |
| purge | eau, gouttes ou rinçage ; distinct de l'icône vanne | `ramp_full` |
| réglages | curseurs ou engrenage | `text_dim` |

Elles aident à reconnaître les trois grandes destinations, mais ne remplacent
jamais leurs libellés. Ne pas ajouter d'icône aux réglages ligne par ligne, au
pavé numérique ou aux confirmations : là, le texte est plus précis.

**Pas d'icône Bluetooth ni de balance dans le bandeau** : le poids visible
indique déjà qu'une balance fournit des mesures.

En mode machine, le Wi-Fi est entièrement déchargé et l'icône reste atténuée :
ce n'est ni une panne ni une connexion en attente. En mode Wi-Fi, elle devient
claire seulement après l'association. Le mode courant est toujours explicité
par l'écran modal ci-dessous ; l'icône n'en est qu'un rappel discret.

### Destination Wi-Fi

Le Wi-Fi n'est pas un état de fond de l'écran de repos. L'utilisateur entre
dans **mode Wi-Fi** par une destination dédiée de l'accueil si la place le
permet, sinon depuis les réglages, uniquement quand la machine est au repos.
L'entrée demande confirmation, puis le cœur vérifie que pompe et vanne sont
arrêtées, arrête NimBLE et charge Wi-Fi/HTTP.

L'UI passe alors en **L2 `WIFI MODE`**, une page de destination : fond normal,
titre `wifi mode` en ambre, état réseau (AP de configuration, association ou
adresse IP) et un bouton local `retour`. L'adresse IP est affichée en grand
pour simplifier l'accès depuis un navigateur. Pendant un flash réseau, cette
page affiche sa progression (barre et compteur). Elle peut aussi montrer la
dernière infusion conservée — par exemple `9:16 · 27 s · 35 g` — et offrir
plus tard le bouton `envoyer l'infusion`; après un envoi accepté, elle indique
`pas d'infusion à envoyer`. Les profils, *infuser*, et les réglages d'infusion
disparaissent. Cela rend visible qu'on a quitté le mode machine et évite toute
ambiguïté sur la disponibilité de la balance.

Les clients distants peuvent faire le diagnostic, lancer une purge de banc et
flasher, ou transmettre la dernière infusion au backend. Ils ne peuvent jamais
lancer une infusion : cette interdiction est appliquée par le cœur, pas par la
seule UI. Le bouton `retour` coupe réellement Wi-Fi/HTTP et son netif, relance
NimBLE, puis retourne au repos. La politique et la raison mémoire sont dans
`firmware.md`, « Politique radio ».

---

## Les boutons

**Surface pleine, pas cadre.** Un rectangle de 8 px de rayon, sans contour,
fond `surface`, libellé en `text`. Un bouton doit se reconnaître à sa surface,
pas à une boîte dessinée autour de vide. Le bouton **primaire** de l'écran
emploie `surface_accent`, avec icône et libellé en `accent`. Les boutons
secondaires emploient `surface`; un choix actif ou un bouton pressé emploie
`surface_high`.

Le bouton destructif ne reçoit pas un fond rouge permanent : surface normale,
libellé `fault`, puis confirmation explicite. Un contrôle désactivé garde sa
surface, avec fond `bg_raised` et contenu `text_faint`, pour que la disposition
ne saute pas.

Une confirmation est elle-même une surface `bg_raised` centrée sur un voile
noir local, sans bordure ambre. Son titre ou une arête verticale de 4 px porte
la couleur sémantique (`accent` ou `fault`). Ses deux actions sont des boutons
pleins : annuler en `surface`, valider en `surface_accent` — ou contenu `fault`
pour confirmer une destruction.

**Retour au toucher**, en 90 ms, sur trois propriétés à la fois pour que ce
soit perceptible même du coin de l'œil :

- le fond passe à `surface_high`,
- une surimpression `accent_wash` peut être appliquée au primaire,
- le contenu gagne en luminosité (`text_dim` → `text`, ou `accent` inchangé).

Au relâchement, retour en 150 ms. Pas d'ondulation façon Material (elle repeint
une grande surface, voir la contrainte de bande passante), pas de déplacement
de 1 px (illisible), pas de son (il n'y a pas de haut-parleur dans la façade).

Un bouton fait **80 px de haut au minimum**, avec une cible tactile d'au moins
80 × 80 px. Les actions de bas d'écran font 88 ou 96 px. Un bouton texte isolé
fait au moins 160 px de large ; les touches `−`, `+` et les chevrons peuvent
être carrés. Deux cibles voisines gardent 16 px entre leurs surfaces.

---

## Hiérarchie des surfaces

| Élément | Fond | Contenu | Trait |
| --- | --- | --- | --- |
| Écran | `bg` | `text` / couleurs sémantiques | aucun |
| Barre haute L3 | `bg_raised` | retour et titre `text` | aucun |
| Bouton secondaire | `surface` | `text` ou `text_dim` | aucun |
| Bouton primaire | `surface_accent` | `accent` | aucun |
| Bouton pressé / choix actif | `surface_high` | `text` ou `accent` | aucun |
| Valeur | transparent sur le fond parent | `text`, `accent` ou `thermal` | aucun |
| Faute / destructif | `surface` ou fond de l'écran | `fault` | jamais un aplat rouge permanent |

Les tabs sont hors périmètre : il n'existe pas deux vues sœurs qu'il faille
garder simultanément visibles. La pagination des réglages change seulement un
groupe de paramètres et reste matérialisée par `‹`, `1/4`, `›`.

### Critères visuels vérifiables dans le simulateur

- `home.png` ne contient plus aucun bouton à fond transparent : les cinq
  contrôles (`−`, `+`, *infuser*, *purge*, *réglages*) sont des surfaces.
- La cible est au-dessus de `−` et `+`; sa largeur ou le passage de `28 s` à
  `100,0 g` ne déplace jamais ces boutons.
- Les trois destinations de l'accueil ont une icône et un libellé. L'action
  *infuser* est la seule tuile primaire.
- `settings.png`, `settings1.png`, `settings2.png` et `settings3.png` partagent exactement la
  même barre haute. Elles n'affichent ni `suite`, ni `fermer`, ni boutons
  `page 1/2/3/4`, mais le retour, le titre, l'index et deux chevrons.
- Le pavé numérique est capturé dans au moins deux variantes : poids avec
  virgule active, temps avec virgule désactivée.
- Au moins quatre familles chromatiques sont visibles dans les snapshots
  pertinents : fond brun, surfaces brunes relevées, ambre d'action, bleu-vert
  thermique ; le rouge n'apparaît que dans un snapshot de faute ou une action
  destructive.
- Aucun changement d'état ne déclenche de transition ou de dégradé plein
  écran. L'état pressé et le filet de progression restent les seules
  animations locales.

---

## Structure : cinq niveaux de statut

Le statut n'est pas un écran, c'est **un niveau d'intensité** qui change selon
ce que la machine fait. C'est ce qui permet d'avoir le poids central pendant un
brew by weight et discret le reste du temps, sans dupliquer les écrans.

| Niveau | Quand | Forme |
| --- | --- | --- |
| **L0 — bandeau** | repos | une ligne de 28 px en haut, toutes les mesures, séparées par des points médians ; sous un filet pleine largeur |
| **L1 — héros** | purge | une valeur à 104 px au centre + son filet de progression + deux valeurs secondaires à 40 px ; le bandeau L0 reste, atténué |
| **L2 — plein écran** | boot, OTA, faute, verrou, mode Wi-Fi | tout le reste disparaît ; un titre, une phrase, éventuellement un filet de progression |
| **L3 — destination** | profils, réglages, diagnostic, pavé numérique | page secondaire sur `bg`, avec barre haute `bg_raised`; le bandeau L0 est remplacé par la navigation |
| **L4 — veille** | 30 min sans touche ni infusion | recouvre tout ; un petit bloc qui se déplace lentement. Voir « Veille » |

L'infusion n'emploie pas ces niveaux : elle a son propre écran, avec frise des
phases et résumé du shot (`ecran-infusion.md`).

**L'objectif courant** — poids si la balance fournit des mesures, temps
sinon — est décidé par la présence de la balance, pas par un réglage. Il
pilote la cible de l'accueil et le libellé du bouton (voir ci-dessous). En
purge, le temps est héros.

### Composition du bandeau L0, état par état

Le bandeau aligne à droite trois sous-zones dans l'ordre **poids · pression ·
température**. La température est à droite et la pression juste avant elle.
La sous-zone du poids absorbe l'espace restant ; son contenu disparaît quand la
balance ne fournit pas de mesure, sans déplacer les deux autres valeurs.
Une mesure de pression ou de température manquante affiche `-` dans sa propre
sous-zone.

| État | À gauche | À droite, dans l'ordre |
| --- | --- | --- |
| Balance présente | diagnostic · heure · version | poids · pression · température |
| Balance absente | diagnostic · heure · version | pression · température |

Sans balance, le contenu du poids disparaît. Sa sous-zone flexible conserve
l'espace libre, et la pression ainsi que la température restent en place.

**La purge montre la pression** : c'est le seul moment où elle est
l'information utile (backflush, contrôle de l'OPV), et le débitmètre n'y veut
rien dire — la pompe recircule (`firmware.md`, section débitmètre).

---

## Les écrans

### Repos

Trois zones, rien d'autre.

1. **Bandeau L0** — à gauche le diagnostic, l'heure et la version ; à droite
   le poids quand la balance fournit des mesures, puis la pression et la
   température, dont la position reste fixe.
2. **Cible au centre**, 76 px, posée au-dessus de deux surfaces `−` et `+`.
   La valeur n'est dans aucune boîte : sa position supérieure laisse aux deux
   boutons une largeur indépendante du nombre de chiffres, comme sur une
   commande de consigne. Les boutons restent **toujours visibles**. Un appui =
   un pas (0,5 g ou 1 s), sans répétition sur appui long. Un appui sur la valeur
   ouvre le pavé numérique pour les grands écarts.
3. **Trois tuiles pleines et iconées** en bas : *infuser* (primaire), *purge*,
   *réglages*. Les icônes accélèrent la reconnaissance, les libellés restent
   obligatoires. Le mode Wi-Fi s'ouvre depuis les réglages, jamais depuis cet
   écran.

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

L'écran d'infusion est décrit dans `ecran-infusion.md` : frise des phases,
six tuiles de résumé, tuile de fonctionnement. Trois règles de ce document
s'y appliquent :

- **Un seul bouton pendant l'écoulement, *arrêter*.** Le cas normal est
  l'arrêt automatique au poids ou au temps ; ce bouton est un secours. Le
  reste de l'écran n'est pas touchable (un chiffon qui passe ne coupe rien).
  Après l'arrêt de la pompe, il devient *fermer*, actif à la fin de la
  capture ; il n'y a pas de retour automatique au repos.
- **Pas de graphe temps réel** : il n'apporte rien pendant qu'on regarde
  couler, et il est illisible à 1,5 m. La frise montre les phases, pas une
  courbe.
- **Aucun jugement affiché** (pas de « bon shot » / « trop rapide ») : la
  machine mesure, elle ne note pas.

### Profils (destination L3)

Une **sélection du profil courant**, pas un bouton isolé par profil : quatre
lignes-surface de 80 px, nom à gauche, résumé à droite
(`36 g · pré-inf. 6 s`). La ligne active emploie `surface_high` et un filet
ambre de 4 px à gauche — pas une case cochée. Quatre lignes sont visibles ;
si davantage de profils existent un jour, ils sont paginés par les mêmes
chevrons, sans geste de défilement.

Tant que les profils n'existent pas, cette destination n'est pas construite : le
nom du profil dans le bandeau reste affiché mais n'est pas touchable. Rien
d'autre dans l'UI n'a à changer le jour où ils arrivent.

### Navigation des destinations L3

Toutes les pages secondaires emploient la même barre haute de 88 px sur
`bg_raised` : chevron retour à gauche dans une cible 80 × 80, titre à sa droite,
puis éventuelles actions à droite. Le chevron revient à la page précédente ;
depuis une destination de premier niveau, il revient à l'accueil. Aucun bouton
`fermer` n'est ajouté dans le contenu et le bandeau de télémétrie L0 n'est pas
visible : les deux ne doivent pas se battre pour le haut de l'écran.

Le retour est un **chevron gauche**, convention des interfaces mobiles, sans
libellé. Son glyphe peut ne faire que 24–32 px, mais sa cible reste 80 × 80.
Il n'y a jamais plus d'un niveau sous une destination : réglages → éditeur →
réglages est la profondeur maximale.

### Réglages

Les réglages occupent des pages horizontales de six tuiles pleines, deux
colonnes par trois lignes. Chaque tuile montre une étiquette 18 px en
`text_dim` puis la valeur courante 26 px en `text`; elle ne concatène pas les
deux sur une seule ligne. Un appui ouvre l'éditeur adapté. On ne change plus
silencieusement une valeur en touchant plusieurs fois une tuile.

La barre haute contient, à droite, l'indicateur `1/4`, un chevron gauche et un
chevron droit dans deux cibles séparées. Les chevrons remplacent les boutons
textuels de pagination. Au début, le chevron précédent
est désactivé ; à la fin, le suivant est désactivé : les pages ne bouclent pas.
Un changement de page est instantané, sans glissement plein écran.

Contenu des quatre pages : cibles et puissances de pompe ; remplissage,
pré-infusion et consigne chaudière ; ramp-down et purge ; système (réseau,
LCD, précharge de chauffe). Le détail est dans la table « Réglages » plus bas.
**Pas de luminosité** : voir « Veille ».

Les stratégies sont des **choix parmi 2-4**, présentés dans un éditeur en
segments pleins côte à côte (`surface`, choix actif `surface_high` avec texte
`accent`), jamais en menu déroulant. Une valeur numérique ouvre le pavé décrit
ci-dessous. Les bornes et pas restent ceux de la table « Réglages » ; la
validation applique la même vérification que `put_config()`.

### Pavé numérique

Le pavé est une destination L3 complète, pas une feuille superposée. La barre
haute porte retour, le nom court du réglage (`cible poids`, `durée pré-inf.`),
et un bouton `valider` à droite. La valeur en cours est affichée hors surface,
en 76 px à gauche avec son unité en 32 px. À droite, douze touches pleines sur
quatre rangées : `1 2 3`, `4 5 6`, `7 8 9`, `⌫ 0 ,`. Pour un réglage entier,
la virgule est visible mais désactivée afin que la grille ne change pas.

Retour annule la saisie et restaure la valeur précédente. `valider` n'est actif
que si la saisie est parseable et dans les bornes ; sinon une explication
courte en `fault` apparaît sous la valeur, sans fermer le pavé. Le pavé est
également celui qu'ouvre un appui sur la cible de l'accueil.

### Diagnostic (destination L3)

**Un appui sur le groupe d'icônes du bandeau ouvre la page de diagnostic.**
C'est le seul raccourci caché de l'interface, et il est justifié : ces icônes
sont exactement ce qu'on regarde quand on se demande « est-ce qu'il voit
vraiment mes capteurs ? ». La cible tactile fait 88 px de haut, centrée sur le
bandeau et débordant sous le filet — la seule cible de l'UI qui déborde d'une
zone.

Une grille diagnostic de 3 × 3 cellules : étiquette et pastille en haut,
valeur instantanée au centre, état brut en dessous. La grille exploite le ratio
large sans prétendre que neuf lignes de 72 px pourraient tenir sous la barre
haute. Les cellules restent sur `bg`, séparées par les seuls filets de 1 px.
Les cases pompe, vanne et chauffage ouvrent chacune leur écran de service ;
les six autres ne sont pas interactives. Aucun graphique n'est affiché.

| Case | Valeur affichée | Détail |
| --- | --- | --- |
| Pression | `9,1 bar` | `valide` / `périmé` / `absent` |
| Chaudière | `92,4°` | `valide` / `absent` |
| Débit | `2,1 ml/s` | `valide` / `périmé` / `absent` |
| Pompe | `100 %` | `valide` / `calibration` / `erreur` / `absent` |
| Vanne | `ouverte` / `fermée` | `valide` / `maintenance` / `périmé` / `absent` |
| Chauffage | `62,5 %` ou `OFF` | `valide` / `désactivé` / `absent` |
| Bus CAN | `OK` / `ERREUR` | total compact des erreurs TWAI (`K`, `M`) |
| Balance | `36,2 g` | `présente` / `absente` |
| Versions | versions écran et capteurs | noms `screen` et `sensors` sous leur version |

Chaque case standard expose une pastille d'état, un titre, une valeur et un
détail. La pastille est grise si la fonction est désactivée, verte si elle est
valide, ambre pour un avertissement et rouge pour une erreur. La case versions
est la seule exception : elle partage son contenu en deux couples valeur/détail.

Un appui sur chauffage ouvre un écran avec les actions `activer` et
`désactiver`. Elles modifient `heating.enabled` par `core::put_config()` ;
l'action correspondant à l'état courant est désactivée. Un appui sur pompe
conserve l'écran de service DimmerLink (reset et calibration).

Un appui sur vanne ouvre l'écran **vanne · maintenance**, prévu pour vider la
chaudière : vanne seule, pompe arrêtée, via `MAINTENANCE_VALVE` (voir
`firmware.md`). Il porte `ouvrir 30 s` (devient `relancer 30 s` vanne ouverte),
`fermer la vanne` et `retour`. Chaque appui sur ouvrir accorde 30 s, comptées
par les capteurs, sans renouvellement automatique : pour une vidange plus
longue, appuyer de nouveau. La ligne d'état suit les capteurs à chaque
rafraîchissement : `vanne fermée · pompe arrêtée`, `vanne ouverte · fermeture
dans N s`, ou le motif d'indisponibilité. Une ligne fixe rappelle d'ouvrir la
buse vapeur pour laisser entrer l'air.

Ouvrir exige le **chauffage désactivé** (case chauffage) : sinon le bouton est
grisé et l'état indique `désactiver le chauffage avant d'ouvrir`. Il est aussi
grisé si le module capteurs est injoignable, verrouillé ou trop ancien pour
annoncer la commande (`module capteurs à mettre à jour`). Un refus du cœur
(cycle en cours, etc.) remplace la ligne d'état jusqu'au prochain appui.
Quitter l'écran, par `retour` ou toute navigation, referme la vanne : elle ne
reste ouverte que sous les yeux de l'utilisateur. Tant qu'elle est ouverte en
maintenance, infusion, purge et commande brute sont refusées.

Les valeurs sont celles du dernier instantané cohérent fourni par le cœur. La
pastille et le détail distinguent une valeur courante, périmée ou absente afin
de diagnostiquer une sonde débranchée directement depuis la façade.

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
| Mode Wi-Fi | `wifi mode` (`accent`) | état AP/association/adresse IP + bouton `quitter le mode wifi`. Ni infusion ni réglages locaux |
| Verrou 60 s | `verrou de sécurité` (`fault`) | « couper la machine à l'interrupteur principal pour réarmer » — la seule sortie réelle, autant l'écrire |
| Bus CAN perdu | `module interne injoignable` (`fault`) | « les commandes sont coupées » |
| Dimmer en calibration | *(pas de L2)* | reste au repos, bouton primaire grisé |

Le verrou et la perte de bus **prennent tout l'écran** parce qu'ils rendent la
machine inutilisable : afficher un petit badge alors qu'aucun bouton ne
fonctionne serait pire que d'afficher la panne.

---

## Notes de mise en œuvre LVGL

- **LVGL v9.x** (9.5 courante), via le composant managé
  `espressif/esp_lvgl_port` + `esp_lcd` RGB. v8 est en fin de vie ; l'exemple
  officiel Waveshare (`waveshareteam/ESP32-S3-Touch-LCD-4.3`, cloné dans
  `tmp/`) est la référence de bring-up de la dalle et du GT911.
- **Bounce buffer obligatoire** sur ce panneau RGB (`CONFIG_LCD_RGB_BOUNCE_BUFFER`) :
  sans lui, le framebuffer en PSRAM déchire dès qu'une tâche prend le bus.
  C'est un réglage de `sdkconfig`, pas un problème d'UI, mais il conditionne
  tout ce qui a été dit sur les animations.
- **Répartition mémoire stricte** : le framebuffer reste en PSRAM et les deux
  bounce buffers RGB restent en SRAM interne DMA. L'allocateur LVGL dédié
  (`ui/lvgl_psram_allocator.cpp`) place en PSRAM les widgets, descripteurs
  d'événements et textes non statiques ; il ne faut jamais les ramener au
  `malloc()` ordinaire, dont les petites allocations préfèrent la SRAM.
  Les tampons de texte persistants de l'UI sont eux aussi réservés par
  `heap_caps_calloc(..., MALLOC_CAP_SPIRAM)`. `EXT_RAM_BSS_ATTR` ne constitue
  pas une garantie suffisante tant que `CONFIG_SPIRAM_ALLOW_BSS_SEG_EXTERNAL_MEMORY`
  n'est pas activé.
- **Diagnostic de démarrage LCD** : juste avant `esp_lcd_new_rgb_panel()`, les
  logs CAN `LCD_INIT_STEP` publient `arg16=20` (SRAM interne libre), `21`
  (plus grand bloc interne) et `22` (PSRAM libre), dans `arg32`. Cela permet
  d'établir une pression SRAM même quand l'UART applicative transporte le flux
  binaire CAN et qu'aucune console texte n'est disponible.
- **Fréquence CPU** : l'écran cible 240 MHz. Cela réduit le coût CPU de LVGL,
  mais ne corrige pas une contention de bus PSRAM ou une allocation DMA
  impossible ; les invalidations inutiles restent interdites.
- **Pas d'`lv_style` par objet** : une table de styles partagés
  (`style_hero`, `style_value`, `style_label`, `style_surface_button`,
  `style_primary_button`, `style_button_pressed`, `style_navbar`,
  `style_hairline`) initialisée une fois. LVGL v9 les applique par référence ;
  en dupliquer un par widget mange la RAM interne.
- **Un écran = une fonction `create_*`**, pas de `lv_scr_load` à répétition :
  les niveaux L0/L1 vivent sur le même écran et changent d'attributs
  (`lv_obj_add_flag(..., LV_OBJ_FLAG_HIDDEN)`, changement de style), les
  niveaux L2/L3 sont des `lv_obj` superposés. Ça évite de recréer l'arbre à
  chaque transition d'état d'infusion.
- **Rafraîchissement** : la télémétrie arrive par CAN à la période demandée par
  `REQSTATUS`. Rafraîchir les libellés à **10 Hz maximum**, même si les trames
  arrivent plus vite — au-delà, c'est illisible et ça repeint pour rien. Le
  filet de progression s'anime, lui, en continu. Un libellé dynamique n'est
  réécrit que si son texte change ; les icônes et fonds statiques ne le sont
  jamais.
- **Destinations L3** : toutes créées une fois, puis masquées ou affichées.
  La navigation ne reconstruit pas l'arbre LVGL. Un éditeur travaille sur une
  copie candidate écrite par `core::put_config()` à *valider* ; le chevron
  retour l'abandonne.
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
évitent d'avoir à « placer à l'œil ».

| Élément | x | y | l × h |
| --- | --- | --- | --- |
| Bandeau L0 (texte) | 32 → 768 | 24 | 736 × 44 |
| Filet sous le bandeau | 32 → 768 | 82 | 736 × 1 |
| Bloc héros L1, purge (haut du bloc) | centré | 120 | — |
| — étiquette de phase | centré | 120 | h 22 |
| — chiffre héros 104 px | centré | 156 | h 104 |
| — filet de progression | centré | 282 | 420 × 2 |
| — valeurs secondaires | centré, écart 52 | 304 | h 46 |
| Cible L0 (valeur + cible tactile) | 256 | 112 | 288 × 96 |
| `−` cible L0 | 256 | 216 | 136 × 80 |
| `+` cible L0 | 408 | 216 | 136 × 80 |
| Étiquette de paramètres | centré | 312 | h 22 |
| Tuile *infuser* | 32 | 368 | 288 × 88 |
| Tuile *purge* | 336 | 368 | 200 × 88 |
| Tuile *réglages* | 552 | 368 | 216 × 88 |
| Barre haute L3 | 0 | 0 | 800 × 88 |
| Retour L3 (cible) | 16 | 4 | 80 × 80 |
| Titre L3 | 112 | centré dans la barre | — |
| Réglages, colonne gauche | 32 | 104 / 208 / 312 | 360 × 88 |
| Réglages, colonne droite | 408 | 104 / 208 / 312 | 360 × 88 |
| Pagination : index | 504 | centré dans la barre | 72 × 80 |
| Pagination : précédent / suivant | 592 / 688 | 4 | 80 × 80 chacun |
| Profils : quatre lignes | 32 | 104 / 200 / 296 / 392 | 736 × 80 |
| Diagnostic : colonnes | 32 / 280 / 528 | 104 / 216 / 328 | 240 × 96 |
| Pavé : bloc valeur | 32 | 144 | 192 × 160 |
| Pavé : touches, colonnes | 256 / 432 / 608 | 104 / 200 / 296 / 392 | 160 × 80 |
| Pavé : valider | 608 | 4 | 160 × 80 |
| Confirmation | 136 | 116 | 528 × 248 |

Toute zone tactile fait **au minimum 80 × 80**, y compris quand son dessin est
plus petit. L'exception de largeur est sans risque pour `−`, `+` et les
chevrons, dont les cibles dépassent déjà 80 px. L'agrandissement éventuel de la
cible se fait par du remplissage transparent, jamais par un contour visible.

Schémas de composition (les cotes de la table priment) :

```text
ACCUEIL
┌ profil ─────────── température · pression · poids · présences ┐
│                           36,0 g                              │
│                    [    −    ] [    +    ]                   │
│              cible · pré-infusion 6 s · rampe                │
│ [ icône  infuser · 36 g ] [ icône  purge ] [ icône réglages ]│
└───────────────────────────────────────────────────────────────┘

RÉGLAGES
┌ [‹]  réglages                         1/4   [‹] [›] ┐
│ [ étiquette          ] [ étiquette                 ]│
│ [ valeur             ] [ valeur                    ]│
│ [ étiquette / valeur ] [ étiquette / valeur        ]│
│ [ étiquette / valeur ] [ étiquette / valeur        ]│
└──────────────────────────────────────────────────────┘

ÉDITEUR NUMÉRIQUE
┌ [‹]  cible poids                              [valider] ┐
│   36,0 g       [ 1 ] [ 2 ] [ 3 ]                         │
│                [ 4 ] [ 5 ] [ 6 ]                         │
│                [ 7 ] [ 8 ] [ 9 ]                         │
│                [⌫ ] [ 0 ] [ , ]                          │
└───────────────────────────────────────────────────────────┘
```

---

## Modèle de données de l'UI

**Un seul état, une seule structure.** Aucun widget ne détient de valeur :
l'UI lit le `core::Snapshot` cohérent publié par le cœur, au plus à 10 Hz, et
la configuration par `core::get_config()`. Elle n'inclut jamais `can_link.h`
et n'émet aucune trame : un appui produit une action du cœur, qui l'accepte ou
la refuse. C'est ce qui permet de rendre l'UI dans `firmware/screen/ui_sim`
sur un instantané rejoué, sans matériel.

L'objectif poids/temps n'est **jamais** un réglage utilisateur : présence de
la balance → poids, sinon temps, réévalué à chaque rafraîchissement au repos
et **gelé au démarrage d'une infusion** (voir les cas limites).

### Périodes de télémétrie (`REQSTATUS`)

| État | `STATUS_PRESSURE` | `STATUS_FLOW` | `STATUS_ACTUATORS` |
| --- | --- | --- | --- |
| Repos | 500 ms | 1000 ms | 1000 ms |
| Infusion / purge | **100 ms** (le plancher) | 100 ms | 200 ms |
| Plein écran (OTA) | 0 (arrêt) | 0 | 0 |

Ce trafic périodique est aussi ce qui **entretient la présence** côté capteurs
(`tick_presence()`) : au repos,
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

**Valeur absente ou invalide : `-` en `text_faint`, jamais un `0,0`.** Les
polices embarquées n'ont pas de tiret long : le tiret ASCII le remplace. Un zéro
affiché à la place d'une mesure manquante est le seul mensonge que cette
interface peut faire ; il est interdit. Règle de péremption : une valeur dont
la dernière trame date de plus de **2 × sa période demandée** passe en
`text_dim`, de plus de **3 s** passe à `-`.

`pressure_valid = false` (bit0 de `StatusPressurePayload::flags`) →
pression **et** température XDB401 à `-` : les deux viennent du même capteur.

---

## Machine à états de l'UI

```
        ┌──────────────────── (« fermer ») ────────────────────┐
        ↓                                                      │
     UI_IDLE ──[ infuser ]──→ UI_BREW ──[ cible atteinte ]──→ UI_DONE
        │  ↑                     │  ↑         [ arrêter ]      │
        │  │                     └──[ arrêter ]────────────────┘
        │  └──[ relâché / 20 s ]── UI_PURGE
        ├──[ appui maintenu purge ]──↑
        └──[ profil / réglage ]──→ UI_SHEET(destination) ──[ retour ]──→ UI_IDLE
                                      │  ↑
                         [valeur]     │  │ [ valider / retour ]
                                      ↓  │
                                  UI_SHEET(editor)

  UI_FULLSCREEN préempte tout état, à tout moment, et le rend au retour.
```

Transitions, **qui les déclenche** :

| Transition | Déclencheur | Effet côté CAN |
| --- | --- | --- |
| `IDLE → BREW` | appui *infuser* | tare balance, `REQSTATUS` en période courte, la boucle d'infusion prend la main sur les `SET` |
| `BREW → DONE` | cible atteinte (algorithme), appui *arrêter*, ou **perte de la balance** en brew by weight | `SET dimmer=0`, `REQSTATUS` en période longue |
| `IDLE → PURGE` | **appui maintenu** sur *purge* | `SET dimmer=<niveau purge>` renouvelé à 10 Hz |
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

Entrer en L2 **coupe les actionneurs** (`SET dimmer=0`) si une infusion
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
| **Entrée en mode Wi-Fi** | acceptée seulement au repos, après arrêt confirmé des actionneurs. NimBLE est arrêté avant le chargement de Wi-Fi/HTTP ; l'écran passe à L2 `wifi mode`. |
| **Appui local pendant le mode Wi-Fi** | seul `quitter le mode wifi` est une action locale. Les contrôles d'infusion ne sont pas dessinés. |
| **Demande d'infusion distante pendant le mode Wi-Fi** | refusée par le cœur. Diagnostic, flash, envoi du dernier shot et purge de banc restent les seules opérations distantes prévues. |
| **Wi-Fi jamais configuré** | en mode machine, icône atténuée sans rappel. Entrer en mode Wi-Fi lance l'AP de configuration ; la machine fait café sans réseau. |
| **Diagnostic ouvert quand le bus tombe** | la destination se ferme et laisse la place au L2 « module injoignable » — la priorité des plein écran s'applique aussi aux pages L3. |
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
| `btn.wifi.enter` / `btn.wifi.exit` | `entrer en mode wifi` / `quitter le mode wifi` |
| `btn.stop` | `arrêter` |
| `btn.close` | `fermer` |
| `btn.cancel` / `btn.ok` | `annuler` / `valider` |
| `nav.back` / `nav.prev` / `nav.next` | icônes retour / précédent / suivant, sans texte visible |
| `keypad.backspace` | icône effacer, sans texte visible |
| `lbl.target` | `cible` |
| `lbl.phase.preheat` | `précharge thermique` |
| `lbl.phase.fill` | `remplissage` |
| `lbl.phase.pre` | `pré-infusion` |
| `lbl.phase.brew` | `infusion` |
| `lbl.phase.ramp` | `rampe` |
| `lbl.done` | `terminé` |
| `lbl.scale_lost` | `balance perdue — infusion arrêtée` |
| `lbl.tare_unconfirmed` | `tare non confirmée` |
| `lbl.dimmer_cal` | `dimmer en calibration — vérifier le secteur` |
| `lbl.mains_unstable` | `secteur instable` |
| `diag.title` | `diagnostic` |
| `diag.rows` | `pression` · `chaudière` · `débit` · `pompe` · `vanne` · `chauffage` · `bus can` · `balance` · `versions` |
| `diag.valid` / `diag.warn` / `diag.absent` | `valide` / `périmé` / `absent` |
| `diag.valve` | `ouverte` / `fermée` |
| `valve.title` | `vanne · maintenance` |
| `valve.hint` | `pompe arrêtée · ouvrir la buse vapeur pour laisser entrer l'air` |
| `valve.open` / `valve.reopen` / `valve.close` / `valve.back` | `ouvrir 30 s` / `relancer 30 s` / `fermer la vanne` / `retour` |
| `valve.state` | `vanne fermée · pompe arrêtée` / `vanne ouverte · fermeture dans %d s` / `désactiver le chauffage avant d'ouvrir` / `module capteurs à mettre à jour` |
| `diag.scale` | `présente` / `absente` |
| `diag.can` | `OK` / `ERREUR`, nombre compact d'erreurs |
| `wifi.title` | `wifi mode` |
| `wifi.ap` / `wifi.offline` | `configuration wifi` / `réseau non associé` |
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

Toute la configuration est un seul enregistrement NVS côté écran (espace de
noms `ui`, deux emplacements transactionnels), lu et écrit par `/config`
(`firmware.md`) et par l'écran de réglages. Les bornes ci-dessous sont celles
de la validation du cœur (`core/config.cpp`) ; l'UI n'en a pas d'autres.

| Réglage | Clé `/config` | Défaut | Min | Max | Pas | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Cible poids | `brew.target_weight_g` | 36,0 g | 10 g | 100 g | **0,5 g** | 1 |
| Cible temps | `brew.target_time_s` | 28 s | 5 s | 60 s | **1 s** | 1 |
| Cible pression | `brew.target_pressure_bar` | 9,0 bar | 6 bar | 12 bar | 0,1 bar | 1 |
| Pompe infusion | `brew.pump_pct` | 100 % | 50 % | 100 % | 5 % | 1 |
| Pompe remplissage | `filling.pump_pct` | 100 % | 20 % | 100 % | 5 % | 1 |
| Pompe pré-infusion | `preinfusion.pump_pct` | 35 % | 0 % | 100 % | 5 % | 1 |
| Pression de fin de remplissage | `filling.pressure_bar` | 1,0 bar | 0,3 bar | 2,0 bar | 0,1 bar | 2 |
| Durée maximale de remplissage | `filling.time_s` | 10 s | 1 s | 20 s | 1 s | 2 |
| Critères pré-infusion | `preinfusion.time` / `.weight` | temps | — | — | `temps` et/ou `poids` | 2 |
| Échéance pré-infusion | `preinfusion.time_s` | 4 s | 0 s | 20 s | 1 s | 2 |
| Consigne chaudière | `heating.brew_temperature_c` | 90 °C | 50 °C | 100 °C | 0,5 °C | 2 |
| Stratégie ramp-down | `rampdown.mode` | `aucune` | — | — | `aucune` / `temps` / `poids` / `chute pression` | 3 |
| Ramp-down, avance temps | `rampdown.lead_time_s` | 3,0 s | 0 s | 15 s | 0,5 s | 3 |
| Ramp-down, avance poids | `rampdown.lead_weight_g` | 4,0 g | 0 g | 20 g | 0,5 g | 3 |
| Ramp-down, chute | `rampdown.pressure_drop_bar` | 1,0 bar | 0,5 bar | 4 bar | 0,5 bar | 3 |
| Pompe purge | `purge.pump_pct` | 100 % | 20 % | 100 % | 5 % | 3 |
| Purge, durée maximale | `purge.max_s` | **20 s** | 5 s | 60 s | 5 s | 3 |
| Précharge de chauffe | `heating.brew_preheat_time_s` | 2,5 s | 0 s | 15 s | 0,5 s | 4 |
| Chauffe active | `heating.enabled` | oui | — | — | — | diagnostic |
| Atténuation après | `ui.dim_after_s` | **240 s** | 60 s | 1800 s | 60 s | — |
| Veille après | `ui.standby_after_s` | **1800 s** | 300 s | 3600 s | 300 s | — |

Les deux dernières lignes n'apparaissent **pas** dans l'écran de réglages
tactile (voir « Veille ») : elles ne vivent qu'en NVS et dans `/config`. Il n'y
a **pas de réglage de luminosité** — la session dure une trentaine de minutes,
la seule intensité utile est le maximum, et l'atténuation est automatique.

Les réglages qui n'ont de sens qu'avec une stratégie donnée (l'avance poids
sans le ramp-down au poids) restent **visibles mais atténués**, pas cachés :
une ligne qui disparaît fait douter de l'avoir rêvée.

---

## Découpage des fichiers

```
firmware/screen/main/ui/
  ui_theme.h                jetons (couleurs, corps, cotes) — aucune logique
  ui_root.cpp               création de l'écran LVGL unique
  ui_home.cpp               accueil, purge, écran d'infusion, réglages, éditeurs,
                            diagnostic, plein écran, veille
  ui_test_screen.cpp        mire de bring-up de la dalle et des jetons
  lvgl_psram_allocator.cpp  allocateur LVGL en PSRAM
  ui_fonts/                 polices générées par lv_font_conv (ne pas éditer)
firmware/screen/main/core/shot_summary.h   résumé de l'écran d'infusion, testé sur l'hôte
firmware/screen/ui_sim/                    rendu hôte et snapshots de régression
```

Règles qui tiennent l'ensemble :

- **`ui_theme.h` n'inclut rien d'autre que LVGL.** Aucun fichier d'UI ne
  connaît le protocole : il lit le cœur, point.
- **Aucun fichier d'UI n'envoie de trame.** Un appui produit une action du
  cœur, qui est seul à émettre des `SET`. C'est ce qui permet de tester l'UI
  sur un instantané rejoué et l'algorithme sans écran.
- **Une seule tâche touche LVGL** (celle d'`esp_lvgl_port`) ; jamais un widget
  mis à jour depuis une autre tâche.

---

## Ce que cette interface ne fera jamais

À relire quand l'envie d'ajouter quelque chose se présentera.

- Pas de widgets déplaçables, pas de disposition configurable.
- Pas de thème clair, pas de choix de couleurs.
- Pas de graphe par défaut (l'accroche existe, elle reste vide).
- Pas de notation d'un shot, pas d'historique, pas de statistiques.
- Pas de saisie de texte au tactile (le provisioning Wi-Fi passe par la page
  web).
- Pas de menu à plus d'un niveau de profondeur.
- Pas de terme technique du protocole visible par l'utilisateur.

---

## Ce qui reste vraiment ouvert

Réduit au minimum : tout le reste est tranché ci-dessus, y compris par défaut.

- **Le graphe temps réel** — hors périmètre, avec le point d'accroche décrit
  plus haut. À reconsidérer après quelques semaines d'usage réel.
- **Les profils** — la destination est spécifiée et dessinée, rien n'est construit
  tant que la notion de profil n'existe pas côté algorithme. Quand elle
  existera, les clés NVS ci-dessus deviennent un enregistrement indexé par
  profil, et **rien d'autre dans l'UI ne bouge**.
- **Les défauts de réglage** — chiffres de départ, à ajuster à la calibration.
  Ce n'est pas une décision d'interface.

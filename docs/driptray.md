# Pesée du drip tray — géométrie et cotes

Référence pour la cellule de pesage sous le drip tray de la Profitec Go. Ce fichier
tient les cotes du bac, du tray et de la barre de charge, plus les contraintes qui
cadrent le dessin, pour ne pas les reprendre à chaque discussion. Les pièces PETG
vivent dans `print/parts/` ; les cotes qui deviennent fit-critiques sont recopiées dans
`print/measurements.toml` au moment de modéliser.

Sauf mention contraire, tout est en mm et relevé par l'utilisateur au pied à coulisse.

## Repère

Origine au **centre du fond du bac**, vu de face :

- **X** vers la droite, ±96 (largeur 192)
- **Y** vers l'avant, ±70 (profondeur 140) — `Y_fond` = distance depuis la paroi arrière, donc `y = Y_fond − 70`
- **Z** vers le haut depuis le fond du bac

## Le bac métallique

| Cote | Valeur | Note |
| --- | --- | --- |
| Largeur intérieure X | **192** | vis retirées, tôle à tôle |
| Profondeur intérieure Y | **140** | idem |
| Hauteur au-dessus du plan de travail | **12** | le bac ne touche pas le plan |
| Cercle de perforations | **Ø80** | centré en X, centre à `Y_fond` = 55 → `y = +15` |
| Emprise du cercle | x ∈ [−40, +40], y ∈ [−25, +55] | évacuation si le tray est oublié |

Le bac est un **membre structurel** de la machine : quatre vis le tiennent au châssis et
**deux des quatre pieds de la machine sont sous son avant**. La masse de la Profitec Go
(10–15 kg, boiler + groupe en colonne, donc centre de gravité très en avant) transite en
partie par lui.

> **Refaire le bac en PETG est écarté.** Deux calculs le disent. Pieds laissés en place :
> ~45 N en porte-à-faux de 140 mm sur des flancs de 3 × 30 → 7 MPa et 1,5 mm de flèche
> immédiate, qui deviennent 5 à 7 mm après un an de fluage à 45 °C. Pieds déplacés vers
> l'arrière : structurellement sain (1,6 MPa, 0,15 mm), mais les appuis avant reculent de
> ~80 mm alors que toute la charge est devant — la machine tangue au verrouillage du
> porte-filtre. Décision de l'utilisateur, 2026-08-25.

### Les quatre vis de fixation

Têtes cylindriques **Ø7, dépassant de 4 mm** dans le bac. Ce sont les seuls points
d'ancrage rigides disponibles : elles vont directement au châssis et court-circuitent la
tôle perforée, qui est souple.

| Paire | X | Y | Hauteur |
| --- | --- | --- | --- |
| **Basses** (fond du bac) | ±49 (47 depuis la paroi latérale) | `Y_fond` = 4 → `y = −66` | sur le fond, dépassement 4 |
| **Hautes** | ±88,5 à confirmer (bord de tête à 4 de la paroi) | `Y_fond` = 29 → `y = −41` | **inconnue**, à relever |

Atteindre les vis hautes demande des faces montantes, **plafonnées à 8 mm de haut** pour
ne pas empiéter sur la cavité OPV et empêcher le tray d'y glisser. L'épaisseur n'y est pas
limitée en dehors de l'aplomb de la vis : cette zone du tray est creuse.

## Le drip tray

| Cote | Valeur |
| --- | --- |
| Masse | **~900 g** |
| Fond | **plein**, il retient les liquides (rien ne peut le traverser) |
| Jeu latéral au bac | **13** de chaque côté → largeur extérieure **166** |
| Appui d'origine | sur les faces latérales du bac (les « rails ») |

Le fond plein a une conséquence utile : **l'intérieur du bac est normalement sec.** Rien
n'y coule tant que le tray est en place et ne déborde pas. L'électronique peut donc vivre
dans le bac, dans les ~17 mm disponibles sous le tray, partout où le socle ne passe pas.

La tasse se pose vers **l'arrière** du tray, sous le bec du porte-filtre, c'est-à-dire
vers `y ≈ −25`. La barre est placée là plutôt qu'au centre géométrique : c'est aussi le
côté des ancrages rigides.

Mise en place **en biais** : on engage la tête du tray dans la cavité de reflux OPV, puis
on rabat l'avant. Il reste un léger mouvement horizontal à l'insertion.

### Le budget vertical, mesuré

Hauteur d'un objet rigide posé sur le fond du bac, sous le tray :

| Hauteur | Comportement |
| --- | --- |
| **17** | le tray quitte les rails |
| **18** | libre et de niveau — vérifié, rien ne porte à l'arrière |
| **19** | la cavité OPV le force à pencher vers l'avant |

**Cote de dessin : 18,0.** Il reste alors **1,0 mm de chute** avant que le tray ne
retombe sur les rails. Le tray ne peut donc monter que de 1 à 2 mm au-dessus de sa
position d'origine, quelle que soit l'architecture — une épaule latérale se heurte au
même plafond qu'un plateau central.

## La barre de charge

| Cote | Valeur |
| --- | --- |
| Capacité | **5 kg** |
| Longueur | **75,5** |
| Section | **12,7 × 12,7** (carrée) |
| Trous | **M4 filetés traversants** |
| Position des trous | centres à 5 et 15 du bout → **x = ±32,75 et ±22,75** |
| Pâte silicone | ~1 au-dessus et ~1 au-dessous, **au centre seulement** |
| Bout **fixe** | **gauche**, celui du câble |
| Bout **chargé** | **droite**, celui du sticker fléché « 5 kg » |
| Flèche à pleine échelle | ~0,2 (typique de ce format, à confirmer sur la fiche) |

Le câble sort toujours au bout fixe : sinon il se plie à chaque cycle et sa raideur vient
en parallèle de la mesure. Si l'orientation est inversée, le signe s'inverse — ça se voit
immédiatement, rien ne casse.

## Bilan de charge

| Poste | Masse |
| --- | --- |
| Drip tray | 900 g |
| Tasse | ~350 g |
| Café | 20 à 100 g |
| **Total** | **~1 350 g = 27 % de la pleine échelle** |

Marge confortable. Une cellule 2 kg donnerait 2,5× mieux en résolution au même format
mécanique, si la 5 kg déçoit.

## Contraintes qui cadrent le dessin

1. **Plateau A1 Mini de 180 mm.** Aucune pièce pleine largeur (192) en une seule fois.
   Les deux vis hautes sont écartées de ~177 mm bord à bord : un socle qui les relierait
   ne passe pas. Les vis basses (98 mm d'entraxe) passent largement.
2. **Le fond est perforé.** Le socle doit enjamber le Ø80 sans rien poser dessus.
3. **Butée de surcharge obligatoire.** 1,0 mm de chute jusqu'aux rails correspond à
   `1,0 / 0,2 × 5 kg = 25 kg` sur la cellule, détruite vers 7,5 kg. Les rails ne
   protègent rien ; il faut une butée réglable (vis M3, un demi-tour = 0,25 mm, arrêt
   vers 6 kg).
4. **Raideur du socle : exigence modérée, pas héroïque.** Une souplesse *en série* avec
   la cellule ne crée **aucune erreur de mesure** — la force qui traverse un chemin de
   charge en série est la même partout, et la cellule mesure sa propre déformation. Elle
   ne coûte que deux choses : elle abaisse la résonance (traitée par le filtre firmware,
   point 5) et elle importe le fluage du PETG dans la boucle (absorbé par la tare avant
   chaque extraction). Passer par les vis d'ancrage plutôt que par la tôle perforée reste
   préférable, mais ce n'est pas une contrainte dure.
5. **Résonance et pompe vibrante.** `f = √(k/m)/2π` avec 1,2 kg suspendu donne 25 à 72 Hz
   selon la raideur du plateau. La pompe de la Go tape à **50 Hz**, en plein dedans.
   Se règle en firmware : passe-bas 3–5 Hz (moyenne glissante ~0,25 s), sans coût, un
   débit ne demandant pas plus de bande passante.
6. **Le jeu de cage est un piège à café.** Un jeu de 0,5 mm sèche en pont rigide et tue
   la mesure. Viser 1,0–1,5 mm, sans lèvre horizontale qui retienne du liquide, et un
   plateau démontable à la main pour le rinçage.
7. **Deux têtes de vis M4 ne rentrent pas dans les 5,3 mm** restants (18 − 12,7). Voir
   la pile ci-dessous.
8. **Zéro perçage, zéro collage.** Tout se reprend sur les vis existantes, complété par
   des aimants (puits Ø8, 3 de haut, fond de 0,6 — voir `canal.py`).
   **Vérifié 2026-08-25 : un aimant tient sur le fond du bac.**
9. **Tenue en température du plateau.** De l'eau à 93 °C peut couler dans le tray, mais
   par périodes de moins d'une minute et jamais stagnante (utilisateur, 2026-08-25). La
   chaleur atteint le plateau par conduction à travers la tôle du tray, aux seules
   pastilles d'appui. Le PETG (Tg ≈ 80 °C) tient : la contrainte de service calculée est
   de ~1,8 MPa, soit 3 % de la limite, et l'exposition est brève. Étaler les pastilles
   plutôt que les concentrer laisse de la marge.

## Résolution attendue

HX711 + cellule 5 kg, gain 128 :

| | Valeur |
| --- | --- |
| Bruit sur une lecture brute | **~0,2 g** |
| Après moyennage à ~4 Hz | **~0,1 g** |
| Sur une dose de 20 g | 0,5 % |

Le bruit mécanique (résonance, pompe) dominera probablement le bruit électrique.

**Câblage :** les 4 fils analogiques de la cellule doivent rester **courts** ;
l'amplificateur va près de la barre, et c'est le numérique (I²C ou l'horloge/données du
HX711) qui parcourt la distance.

## Séquence d'usage

1. le drip tray est vidé et posé sur le bac
2. une tasse est posée
3. tare à 0
4. extraction, 20 à 100 g

La tare juste avant chaque extraction absorbe le fluage, la dérive thermique du zéro
(~1 g/°C sur une 5 kg) et l'erreur d'excentration constante. Ce qui n'est pas absorbé :
les gouttes du porte-filtre pendant les ~30 s d'extraction, soit quelques dixièmes de
gramme, le fond du tray étant étanche.

## Architecture retenue

Décision utilisateur, 2026-08-25. **Cellule unique, centrée sous le tray.** Deux pièces
PETG, `base_pesage` et `plateau_pesage`.

Le plateau n'a **pas** à couvrir toute la largeur du tray : ~110 mm en X suffisent, ce qui
laisse 2,1× de marge au basculement (9 N × 55 contre une tasse de 3 N à 80 mm) et passe
sans effort sur le plateau de 180 mm.

L'excentration est assumée : un tray de 192×140 sur une barre de 75,5 est hors de la plage
de compensation de la cellule. La tare avant chaque extraction absorbe la part constante ;
la part variable reste sous le gramme tant que la tasse revient à peu près au même endroit,
et l'étalonnage se fait à cet endroit-là.

### L'alternative écartée — deux cellules latérales (Gaggiuino)

Meilleure en principe : deux cellules sommées sont statiquement justes quel que soit le
point d'application, ce qui supprime l'excentration ; deux petites pièces contournent le
plateau de 180 mm ; les cellules sortent de la zone humide.

Bloquée par une cote : l'espace latéral fait **13,0** et la section d'une barre **12,7**,
soit 0,15 par côté — il ne reste rien pour un logement imprimé, dont le mur minimum est
2 mm. Il faudrait 17 mm de fente. Une TAL221, plus courte, a la même section.

Trois réserves secondaires : 750 g par cellule serait à 90 % de la pleine échelle (il
faudrait des 2 kg) ; deux appuis sont instables en tangage et la langue arrière ne peut
pas servir de troisième appui sans court-circuiter la mesure ; et l'architecture n'échappe
pas au budget de 1 à 2 mm, qui se déplace simplement sous les patins.

**À rouvrir si l'espace latéral se révèle être 18 mm plutôt que 13.**

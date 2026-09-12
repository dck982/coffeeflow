# Support du débitmètre Digmesa

## Description du module

Le module à maintenir est le débitmètre **Digmesa FHKSC 932-9525-B**, équipé d’une buse de 1,00 mm et retenu pour mesurer le débit d’eau dans la machine à café. Il est installé sur le circuit basse pression, entre le réservoir et la pompe, après le petit filtre boule : le chemin nominal devient donc **réservoir → petit filtre boule → débitmètre → pompe**, au lieu de **réservoir → petit filtre boule → pompe**. Le débitmètre possède un connecteur sur sa face supérieure, avec trois fils qui partent vers le **boitier_dc** pour rejoindre le XIAO sensors ; le support devra prévoir au-dessus de ce connecteur un « toit » protecteur afin de limiter le risque qu’une fuite d’eau atteigne la connexion. Deux pins présents sous le boîtier pourront être utilisés pour contribuer à son maintien. Le module doit rester placé à l’horizontale, orientation nécessaire pour respecter ses spécifications de mesure. Les cotes et la géométrie de référence sont données dans [le schéma de mesures extrait de la datasheet](datasheets/digmesa-measurements.png), issu de la page 2 de [la datasheet Digmesa](datasheets/flowmeter-digmesa.pdf) ; le principe d’installation de la page 4 est repris dans [le schéma d’installation](datasheets/digmesa-install.png), après rotation de 90°.
## Emplacement dans la machine

Les photos ont été prises à l’arrière de la machine, ouverte et sans la plaque de séparation du réservoir, sauf indication contraire. Elles servent à identifier les éléments et l’espace disponible ; elles ne doivent pas être utilisées seules pour déduire des cotes.

- [Photo 1](../tmp/digmesa/photo1.png) : vue générale de l’arrière ouvert, avec la soupape/adaptateur gris sous le réservoir, le filtre boule, la pompe verte et les tuyaux environnants.
- [Photo 2](../tmp/digmesa/photo2.png) : vue rapprochée de la soupape/adaptateur gris, du filtre boule et de la zone située devant la pompe.
- [Photo 3](../tmp/digmesa/photo3.png) : vue rapprochée de la pompe **OLBA Silent Green 35 W**, reconnaissable à son module vert ; elle montre notamment l’espace et les tuyaux autour de son entrée.
- [Photo 4](../tmp/digmesa/photo4.png) : même zone avec la plaque de séparation du réservoir installée, pour visualiser le volume réellement disponible lorsque la machine est remontée.
- [Photo 5](../tmp/digmesa/photo5.png) : indication de l’emplacement envisagé pour le débitmètre, dans l’angle inférieur gauche de la machine.
- [Photo 6](../tmp/digmesa/photo6.png) : débitmètre sorti de la machine, avec son connecteur et ses trois fils, ainsi que la visserie disponible pour réaliser le support.
- [Photo 7](../tmp/digmesa/photo7.png) : vue rapprochée du connecteur électrique sur le dessus du débitmètre et du raccord hydraulique latéral ; les fils peuvent être pliés pour passer sous le toit protecteur.
- [Photo 8](../tmp/digmesa/photo8.png) : vue du dessous du débitmètre, montrant la géométrie du fond et le pin de maintien à prendre en compte dans le support.

### Éléments visibles et chemin de l’eau

Un tube en silicone arrive par le haut : il s’agit du retour de la vanne OPV. Un autre tube en silicone part de la zone tenue par la main sur les photos ; il relie l’adaptateur sous le réservoir au petit filtre boule, puis le filtre à l’entrée de la pompe. Le connecteur gris sur lequel sont raccordés les deux tubes est la soupape/adaptateur qui se monte sous le réservoir. Sur les photos, cette pièce est positionnée approximativement à la hauteur qu’elle aura une fois fixée à la plaque.

Le circuit actuel est constitué d’un tube silicone monté en press-fit dans l’adaptateur du réservoir, d’environ **60 mm**, qui se raccorde en press-fit au filtre boule, d’environ **20 mm**. Un second tube repart ensuite du filtre, décrit un virage vers l’avant de la machine et entre dans la pompe. Le circuit visé avec le débitmètre est : **adaptateur du réservoir → filtre boule → raccord hydraulique inférieur du débitmètre → raccord hydraulique supérieur du débitmètre → pompe**. Le tube entre le filtre et l’entrée inférieure du débitmètre pourra être confectionné plus court, avec un virage de 180° aussi court que l’espace le permet. Depuis la sortie supérieure, le tube partirait droit avant de tourner vers la gauche pour rejoindre la pompe. La boucle de tube recommandée par la datasheet pourra être ajoutée si l’encombrement le permet.

La hauteur du débitmètre reste à choisir entre deux configurations :

1. **Configuration haute** : placer le débitmètre de manière que sa sortie hydraulique soit à **Z = 50 mm**, au même niveau que l’entrée de la pompe ; le tube de sortie peut alors rester à plat jusqu’à la pompe.
2. **Configuration basse** : placer le débitmètre aussi bas que possible afin de conserver une boucle de silicone sous la plaque du réservoir, située à **Z = 102 mm**, puis faire remonter cette boucle jusqu’à l’entrée de la pompe à **Z = 50 mm**. Cette configuration nécessite une bague ou un guide qui laisse la boucle coulisser pour absorber les mouvements et vibrations, tout en la maintenant en place et en empêchant son déplacement latéral ou son décrochage.

La configuration basse est à étudier en priorité si elle permet de réaliser la boucle avec un rayon de courbure acceptable et sans contact avec le fond, les parois ou les autres composants. La configuration haute reste la solution de référence si la boucle prend trop de place ou impose une contrainte excessive au tube.

### Repère et mesures disponibles

Les coordonnées X/Y des trous sont décrites en considérant que le trou situé tout en bas à gauche de la photo est le trou **(1,1)**. Repère confirmé le 12 septembre 2026 : **X vers la droite sur les photos prises de l'arrière, Y vers l'avant de la machine, Z vers le haut**. Les deux trous de la rangée gauche, espacés de 35 mm en Y, sont retenus pour la fixation.

| Élément | Mesure ou contrainte | Référence / remarque |
| --- | ---: | --- |
| Sol → plaque de fond | **10 mm** | Un boulon ou une vis peut être installé depuis dessous. |
| Diamètre des trous de fixation | **6 mm** | Compatible avec les vis/boulons et écrous M6 disponibles. |
| Entraxe des trous, direction X | **40 mm** | Entre les centres. |
| Entraxe des trous, direction Y | **35 mm** | Entre les centres. |
| Premier trou à gauche → face gauche | **23 mm** | Mesure dans le repère décrit ci-dessus. |
| Premier trou en bas → rebord arrière | **23 mm** | Mesure dans le repère décrit ci-dessus. |
| Plaque de fond → plaque portant le réservoir | **102 mm** | Distance verticale Z. |
| Cylindre de la soupape/adaptateur | **Z = 38 mm**, diamètre **25 mm** | Hauteur mesurée depuis la plaque de fond. |
| Entrée de la pompe | **Z = 50 mm** | Hauteur depuis la plaque de fond. |
| Pieds de la plaque de séparation | débordement **15 mm en X**, longueur **40 mm vers Y− depuis la vis de maintien** | Clarification utilisateur du 12 septembre : les 40 mm ne sont pas une distance depuis le rebord arrière. |

### Précisions recueillies pendant la modélisation — 12 septembre 2026

- Digmesa connecté : **55 mm depuis la base du corps jusqu'aux fils pliés**, pins inférieurs exclus.
- Tube silicone : **Ø extérieur 8 mm**, plus petite boucle **Ø50 mm**. Le diamètre de boucle étant donné sans préciser extérieur ou axe, utiliser 50 mm à l'axe pour une première enveloppe conservatrice de 58 mm hors tout.
- Visserie : essai réel utilisateur confirmant le serrage d'une semelle imprimée de **1,6 à 2 mm** ; conception prévue à 2 mm.
- `print/parts/berceau_digmesa.py` : appui du corps Ø32 et logements des pins. Il est maintenant emboîté dans `support_digmesa`, avec la retenue portée par `chapeau_digmesa` ; assemblage `ensemble_digmesa`.
- Pied gauche précisé : 35 mm libres entre son extrémité arrière et l'arrière de la machine ; son bord côté X+ est à 1 mm vers X− du bord gauche du trou M6 (1,2). Avec les axes des trous à X=23 et leur diamètre de 6 mm, cela place ce bord à X=19 mm. Sa hauteur n'est pas encore cotée.
- Second pin du Digmesa : Ø2,85 mm, centre (0,-12) par rapport au centre du débitmètre lorsque les sorties sont orientées vers +X. Le berceau porte deux alésages et un rebord continu ; l'ouverture latérale du premier coupon était une erreur de lecture et a été supprimée.

Ces valeurs sont pour l’instant des mesures de contexte destinées à guider la conception. Celles qui seront retenues comme contraintes géométriques du modèle devront ensuite être reportées dans `measurements.toml`, avec leur méthode et leur référence.

### Implantation nurb révisée — 12 septembre 2026

Centre capteur **X=52,Y=29**, décalage de 4 mm vers l'avant appliqué. Les têtes
M3 gardent 2 mm avec la paroi arrière. Les deux ancrages M6 restent fixes.
Structure à **5 mm du fond**, ancrages serrés sur **2 mm**, troisième appui libre
**6×6 mm en (69,29)** sous le côté libre du berceau pour réduire le porte-à-faux.
Les drains voisins restent dégagés. Base du corps Z=19,1, sommet du toit Z=80,1.

David a choisi le **guide indépendant sur le fond**. Fixation proposée dans le
trou existant (103,23), avec base rainurée. Passage **ovale 21×10,5 mm**, deux
segments Ø8 côte à côte, jeu diamétral **2,5 mm**. Axe (113,29,39,1). Tube de
référence ouvert, tour de 360°, sommet Z=93,1 : 8,9 mm sous la plaque réservoir.
L'implantation et les raccordements côté pompe restent à vérifier physiquement.

Quatre pièces à imprimer séparément : support sur sa tranche arrière, berceau
fond à plat, chapeau sur sa face arrière, guide couché sur sa face droite.
Les arêtes exposées ont des chanfreins 1 mm ; ajustements et appuis laissés nets.
Fiches de montage : `print/parts/ensemble_digmesa.md`. État actuel et limites :
[support-digmesa-reprise.md](support-digmesa-reprise.md).

## Concept de support envisagé

L’implantation préférée est l’angle inférieur gauche visible sur la [photo 5](../tmp/digmesa/photo5.png). Le support devrait exploiter deux trous M6 seulement si cela suffit à empêcher la rotation et les vibrations ; les autres trous doivent rester libres autant que possible afin de conserver des chemins d’évacuation en cas de fuite. Le débitmètre doit rester proche des faces environnantes pour limiter l’encombrement, mais avec un jeu suffisant pour qu’il ne les touche pas lorsqu’il vibre.

Le maintien du débitmètre pourra se faire soit par son pin inférieur, soit par un cylindre ou logement dans lequel ce pin vient se positionner. Une fixation principale par vis depuis le dessous de la machine est souhaitable : elle évite de devoir accéder à la tête de vis avec un tournevis depuis le dessus. Les vis et écrous M6 disponibles pourront être utilisés, sous réserve de vérifier l’accès, le jeu autour des têtes et l’évacuation de l’eau.

Une variante à étudier est un support formant un rail dans la direction Y : le débitmètre pourrait y coulisser et être positionné avec un jeu fonctionnel, tandis qu’une extension vers les tuyaux porterait deux petites plateformes pour des brides plastiques. Ces brides maintiendraient les tubes et pourraient absorber une partie des vibrations, sans imposer au débitmètre une fixation rigide sur toute sa longueur.

Enfin, le support devra probablement être complété par une seconde pièce indépendante formant un chapeau au-dessus du connecteur électrique et des câbles. Ce chapeau doit réduire le risque qu’une fuite venant du haut atteigne le connecteur, tout en laissant passer les fils vers le boitier_dc et en restant démontable pour l’entretien. Les fils peuvent être pliés sous le toit : il n’est donc pas nécessaire de prévoir un grand dégagement vertical pour les maintenir droits. La [photo 6](../tmp/digmesa/photo6.png) documente aussi la visserie disponible ; la forme exacte des vis, écrous et rondelles devra être mesurée ou vérifiée avant de dessiner leurs logements.

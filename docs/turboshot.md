# Turbo shot

Ce document rassemble ce que l'on sait du turbo shot : l'étude dont il vient,
ce que les baristas en ont fait depuis, et ce que cela veut dire sur
CoffeeFlow. Le suivi des infusions reste dans
[journal-infusions.md](journal-infusions.md).

## Définition courte

Un turbo shot est un espresso **moins dosé**, moulu **beaucoup plus gros**,
extrait à **basse pression** (6 bar au plus) en **moins de 15 à 20 s**, à un
ratio d'environ **1:2,5 à 1:3**. Il vise une extraction plus homogène et plus
reproductible qu'un espresso classique, au prix d'une concentration plus
faible (≈ 8 à 9,5 % TDS au lieu de 10 à 12 %) et d'une texture plus légère.
L'écoulement est rapide et « moche » (jets, giclures) : c'est normal.

## L'étude d'origine

Cameron, Morisco, Hofstetter, Uman, Wilkinson, Kennedy, Fontenot, Lee,
Hendon et Foster, « Systematically Improving Espresso: Insights from
Mathematical Modeling and Experiment », *Matter* 2(3), 631–648, mars 2020,
[doi:10.1016/j.matt.2019.12.019](https://doi.org/10.1016/j.matt.2019.12.019)
(PDF librement accessible sur la page des publications de C. Hendon,
Université de l'Oregon).

### Montage expérimental

| Élément | Valeur |
| --- | --- |
| Machine | San Remo Opera 3 groupes (pression, température et temps contrôlés) |
| Paniers | 20 g, sans nervure |
| Moulin | Mahlkönig EK43, meules « coffee » ; réglage espresso usuel 1,3 à 2,3 |
| Tassage | Puqpress, 98 N ; la force de tassage ne changeait ni le temps ni l'EY de façon notable (le goût, si, non quantifié) |
| Recette de référence | 20,0 g → 40,0 g, ± 1 g toléré, **92 °C**, **6 bar statiques**, profil plat |
| Mesure | réfractomètre, TDS → EY ; n = 20 pour l'étalonnage, puis séries de 5 |

Pourquoi 6 bar : à 9 bar, la galette se colmatait aux moutures fines et
l'expérience ne couvrait pas toute la plage de mouture. À 6 bar, le temps de
shot est linéaire avec le réglage (R² = 0,995).

### Résultats

- **Effet volcan.** Le modèle, qui suppose un écoulement homogène, prédit
  une EY qui monte toujours quand on moud plus fin. Mesurée, l'EY passe par
  un **maximum vers le réglage 1,7** puis **baisse** aux réglages plus fins :
  des zones de la galette sont colmatées et peu extraites, d'autres
  surextraites. La moyenne mesurée cache ce mélange, qu'on goûte comme
  amer et acide à la fois.
- **Deux EY de 22 % ne se valent pas.** Le même 20 g → 40 g donne 22 %
  d'EY au réglage 1,3 (lent, colmaté) et 2,0 (rapide, homogène), avec une
  chimie forcément différente : il faut goûter, pas seulement mesurer.
- **Le temps n'est pas une variable indépendante.** Une mouture plus grosse
  peut extraire autant ou plus en moins de temps. Les 20–30 s de la
  définition SCA poussent à moudre dans la zone colmatée.
- **Moins de café extrait mieux.** À ratio constant, baisser la dose monte
  l'EY maximale ; la pression plus basse aussi (modèle, figure 3).

### Protocole proposé (figure 6)

1. Trouver un « point savoureux » avec la recette habituelle.
2. Soit (A) moudre plus gros jusqu'à l'EY maximale, puis **réduire le
   poids en tasse** pour retrouver l'EY du point savoureux : boisson plus
   courte, plus concentrée, plus reproductible ;
3. soit (B) **réduire la dose** (20 → 15 g, en gardant 40 g en tasse) et
   moudre **beaucoup plus gros** : même EY, 25 % de café en moins,
   **shots très rapides (< 15 s)**, concentration plus basse, profil
   aromatique différent.

C'est la voie B, **15 g → 40 g à 6 bar en 7 à 15 s** (ratio 1:2,7), que
l'on appelle aujourd'hui turbo shot. Le papier ne lui donne pas ce nom.

### Ce que l'étude dit elle-même de ses limites

- Les shots rapides ont moins de « complexité » qu'un 22 % obtenu en zone
  colmatée ; les auteurs proposent, sans y croire pour un usage courant, de
  mélanger un shot fort dosé peu extrait et un shot faible dosé très
  extrait.
- Un seul café testé ; les auteurs extrapolent à tous les arabicas parce
  qu'ils moulent de façon semblable.
- Mise en œuvre réelle : un café d'Eugene (Oregon) a baissé ses doses de
  25 % à partir de 2017 ; ses shots tombaient **couramment à 14 s**.

## Lectures postérieures

### James Hoffmann (2020, à la sortie du papier)

Réplication à Square Mile avec un EK43, 20 g → 40 g à 6 bar plat :

- le maximum d'EY est bien là, au réglage 1,8 à 1,85, avec un shot
  d'environ 15 s ; ce shot n'était pas bon tel quel, il a fallu ajuster la
  recette ensuite, comme le papier le demande ;
- les meilleurs shots : propres, sucrés, clairs, **≈ 9,5 % TDS**, texture
  correcte sans plus ;
- critiques : un seul café ; sur une Decent, l'écoulement passe souvent
  d'homogène à hétérogène en cours de shot (hausse brutale du débit à
  pression fixe), et même à 6 bar la galette finit par se défaire ;
- ce n'est **pas une recette d'automatisation** : il faut un barista qui
  goûte. Il faut aussi un moulin assez homogène.

(Il situe le café d'essai à Portland ; le papier dit Eugene.)

### Lance Hedrick, « Turbo shots » (2021)

Vidéo relue par Cameron et Hendon. Ce qui précise le papier :

- recette courante de Cameron : **15 g, ratio ≈ 1:3, 12 à 18 s**, plutôt
  sous 20 s ; au-delà de 20 s, des composés désagréables masquent le fruit
  et le chocolat ;
- **la pression est un leurre** : 6 bar est un **plafond**, pas une cible.
  La galette s'érode pendant le shot et ne tient pas la pression ; ce qui
  compte est le **débit, ≈ 4,5 à 5 g/s** ;
- utiliser un **panier adapté à la dose** (il fait 17 g → 51 g en 19 s à
  6 bar, sans pré-infusion, faute de panier de 15 g) ;
- le shot est laid, gicle, et c'est correct ; nettoyage pénible sans bec
  ou papier sous la galette ;
- variante pour torréfactions très claires (« blooming turbo », Decent) :
  ≈ 3 s à plein débit pour mouiller, fermeture jusqu'à ≈ 2 bar, puis
  ≈ 5 ml/s, toujours sous 20 s à 1:3.

### Lance Hedrick, tendances actuelles (vidéo récente)

Trois cafés réputés qui poussent la logique plus loin, vers des extractions
**basses** (10 à 16 % d'EY), des températures **basses** et des ratios
**longs**. La pression n'y joue plus aucun rôle.

| Programme | Dose → tasse | Ratio | Temps | Température | Particularités |
| --- | --- | --- | --- | --- | --- |
| SEY (Brooklyn) | 18 → 42 g | 1:2,3 | 9 à 12 s | 90 °C | 6 bar, papier sous la galette, débit d'origine de la machine ; ≈ 14 % d'EY mesurés |
| Big Sur « soup » (Shanghai) | 17 → 80 g | 1:4,7 | 15 à 18 s | 83 °C (88 °C à la maison) | mouture plus grosse que du filtre, sans tassage, papiers dessus et dessous ; ≈ 2 % TDS, 10 % d'EY |
| Motors (Paris) | 16 → 96–100 g | 1:6 | ≈ 16 s | < 90 °C (88 °C) | tasse changée à mi-poids, plein débit ≈ 9–10 g/s, bec recommandé |

Règle pratique qu'il donne pour régler le temps selon sa machine : mesurer
le débit libre du groupe sur 10 s. **Sous 7 g/s, viser 11 à 14 s** ; au-
dessus, 9 à 12 s. En mouture très grosse, le temps ne descend plus : il est
borné par la pompe, et on dispose d'une grande plage de mouture au même
temps. Prévoir une grille ou un papier sur la galette contre les cratères
de la douche, surtout avec beaucoup d'espace libre.

## Repères de recette

| Grandeur | Papier (2020) | Turbo courant | Tendance « soupe » |
| --- | --- | --- | --- |
| Dose | 15 g en panier de 20 g | 15 à 17 g, panier adapté | 16 à 18 g, panier plus grand |
| Ratio | 1:2,7 | 1:2,5 à 1:3 | 1:4,5 à 1:6 |
| Temps | 7 à 15 s | 12 à 18 s, < 20 s | 9 à 18 s, borné par la pompe |
| Pression | 6 bar statiques | 6 bar au plus | sans importance |
| Débit en tasse | — | 4,5 à 5 g/s | 5 à 10 g/s |
| Température | 92 °C | 90 à 93 °C | 83 à 90 °C |
| TDS / EY | ≈ 22 à 24 % d'EY | 8 à 9,5 % TDS | 2 à 5 % TDS, 10 à 16 % d'EY |

## Sur CoffeeFlow

### Ce que la machine impose

- **Débit libre de 5,68 g/s à 100 %** ([purge_sans_resistance.html](purge_sans_resistance.html)),
  sous les 7 g/s de la règle de Hedrick : viser **11 à 14 s** d'écoulement.
  Le débitmètre sous-compte d'environ 1,4 à ce débit : 3,7 ml/s comptés
  valent ≈ 5,2 g/s.
- **La pression n'est pas réglable à débit élevé.** La pompe vibrante
  plafonne : avec la mouture du single espresso et 11 g dans le panier de
  58 mm, la galette est si perméable (0,38 à 0,43 bar·s/g) que 100 % ne
  donne que ≈ 2 bar. `brew.target_pressure_bar` n'agit qu'en
  plafond ; il ne devient actif que si la mouture est assez fine pour que
  la galette monte jusqu'à la cible. La pompe à 100 % tient ≈ 4,9 g/s
  sous 2 bar et **≈ 4,4 g/s sous 4,85 bar** (15 g, 26/10 à 15 h 35) ;
  le débit à 6 bar n'est pas mesuré.
- **Arrêt au poids.** Le firmware coupe la pompe à
  `target_weight_g − rampdown.lead_weight_g` (en mode `none`). Le café
  continue d'arriver pendant ≈ 0,9 s après la coupure, quel que soit le
  débit : à 1,1 g/s cela fait ≈ 1 g, à 5 g/s ≈ 4,5 g. Le décalage à
  prévoir est proportionnel au débit.
- **`brew.target_time_s` ne sert pas** quand la balance est présente : il
  ne s'applique qu'en infusion sans objectif de poids.
- **Remplissage.** Avec 11 g dans le panier de 58 mm 15 g, le remplissage
  à 70 % admet 25 à 29 ml : fin sur la pression à 25,2 ml le 26/10 à
  13 h 32, échéance de 10 s sans atteindre 0,8 bar à 9 h 34. Première
  goutte entre 26 et 32 ml (≈ 27 ml avec 8 g en LM1 41 mm). Ces 9 à 12 s
  de mouillage à basse pression avant la première goutte n'existent pas
  dans les recettes publiées, qui partent sans pré-infusion.

## Essais

Le détail de chaque essai est dans [journal-infusions.md](journal-infusions.md) ;
cette section ne garde que la comparaison avec les repères de recette.

### 26 octobre 2026, 9 h 34 — `261026-093443`

Profil « turbo shot » de `configs/261026-093452.json` : 11,0 g en panier
58 mm 15 g (73 % de la capacité, comme 15 g en 20 g dans le papier), cible
45 g et 6 bar, pompe à 100 %, 91 °C, remplissage 70 % / 0,8 bar / 10 s,
pré-infusion 50 % / 4 s ou goutte, `lead_weight_g` de 1,0 g. Café « Cozy
Chocolate » (Brésil, nature), **même mouture que le single espresso**,
café tassé (« mis à niveau » sur le moment, déjà un tassage maximal
selon David à 13 h 32). Écran et sensors 0.4.24.
Goût : **très intéressant, zéro amertume, notes florales**.

| Grandeur | Mesure | Repère turbo |
| --- | ---: | --- |
| Remplissage | 10,05 s (échéance), 28,7 ml, 0,77 bar | — |
| Pré-infusion | 1,92 s, 2,6 ml, finie sur la goutte, 0,80 bar | aucune |
| Première goutte → arrêt | **10,9 s** | 11 à 14 s sur cette machine |
| Pompe en marche | 22,9 s | < 20 s |
| Pression d'infusion | **2,1 bar** (2,17 max), pompe à 100 % dès 2,9 s | 6 bar au plus |
| Débit en tasse | **≈ 4,9 g/s** (10 → 40 g en 5,9 s) | 4,5 à 5 g/s |
| Résistance de la galette | 0,43 bar·s/g (7,8 pour l'espresso du 25/10) | — |
| Poids | 44,4 g à la coupure, **48,5 g final** | 45 g visés |
| Ratio | **1:4,4** | 1:2,5 à 1:3 |
| NTC tasse | 89,37 °C, −1,63 °C sous la consigne | 90 à 93 °C |

Lecture : l'hydraulique est celle d'un turbo moderne borné par la pompe
(débit, temps d'écoulement, dose relative au panier), mais à **2 bar** et
non 6, et le **ratio** est celui d'une « soupe » légère, pas d'un turbo.
Pour un turbo au sens du papier avec 11 g : 30 à 33 g en tasse, et une
mouture assez fine pour que la pression monte vers 6 bar en restant sous
15 s d'écoulement.

### 26 octobre 2026, 13 h 32 — `261026-133226`

Même profil, **cible 40 g** et **88 °C** (`configs/261026-142235.json`),
même café, même mouture, même tassage, sans puck screen. Goût : **bon,
mais très léger**, pas d'amertume, légère acidité, **pas de crema**.

| Grandeur | Mesure | Repère turbo |
| --- | ---: | --- |
| Remplissage | 7,90 s, 25,2 ml, fini sur la pression (0,89 bar) | — |
| Pré-infusion | 0,79 s, 1,2 ml, finie sur la goutte, 0,90 bar | aucune |
| Première goutte → arrêt | **9,6 s** | 11 à 14 s sur cette machine |
| Pompe en marche | 18,4 s | < 20 s |
| Pression d'infusion | **1,9 bar** (1,98 max), pompe à 100 % dès 2,7 s | 6 bar au plus |
| Débit en tasse | **≈ 4,9 g/s** (10 → 39,6 g en 5,8 s) | 4,5 à 5 g/s |
| Résistance de la galette | 0,38 bar·s/g | — |
| Poids | 39,6 g à la coupure, **44,2 g final** | 40 g visés |
| Ratio | **1:4,0** | 1:2,5 à 1:3 |
| NTC tasse | 88,74 °C, **+0,74 °C** au-dessus de la consigne | 90 à 93 °C |

Lecture : le tassage ne change pas la galette ; la pression reste à 2 bar.
La consigne baisse de 3 °C mais la tasse de 0,63 °C seulement. L'arrêt
laisse encore ≈ 1 s de débit, ≈ 5 g, arriver après le seuil.

### 26 octobre 2026, 15 h 35 — `261026-153548`

**15 g** dans le même panier, cible 45 g, `lead_weight_g` de 5,0 g, 88 °C
(`configs/261026-153711.json`) ; même mouture, même tassage. Goût :
**espresso long, crema onctueuse, goût caractéristique de l'espresso,
légère amertume** ; loin de la « soupe » fruitée des deux essais à 11 g.

| Grandeur | Mesure | Repère turbo |
| --- | ---: | --- |
| Remplissage | 6,28 s, 19,7 ml, fini sur la pression (0,97 bar) | — |
| Pré-infusion | 4,08 s, 5,5 ml, finie à l'échéance, 1,31 bar | aucune |
| Pompe avant la goutte | **12,0 s** | 0 dans le papier |
| Première goutte → arrêt | **9,9 s** | 7 à 15 s (papier) ; 11 à 14 s sur cette machine |
| Pompe en marche | 21,9 s | < 20 s |
| Pression d'infusion | **4,85 bar** (4,91 max), pompe à 97–100 % | 6 bar au plus |
| Débit en tasse | **4,40 g/s** | 4,5 à 5 g/s |
| Résistance de la galette | **1,10 bar·s/g** | ≈ 1 à 2 (estimé depuis le papier) |
| Poids | 40,8 g à la coupure, **43,9 g final** | 45 g visés |
| Ratio | **1:2,9** | 1:2,5 à 1:3 |
| NTC tasse | 89,37 °C, +1,37 °C au-dessus de la consigne | 90 à 93 °C |

Lecture : la galette, le débit, le ratio et le temps depuis la goutte
sont ceux du turbo du papier, à 4,9 bar au lieu de 6. Ce qui l'en
éloigne est le **mouillage de 12 s** avant la goutte, absent du papier, et
la mouture espresso, plus fine. Passer de 11 à 15 g a triplé la
résistance (0,38 → 1,10 bar·s/g).

### 26 octobre 2026, 16 h 32 — `261026-163228`

Même recette qu'à 15 h 35 (15 g, 45 g, 88 °C), **mouture nettement plus
grosse**. David voit du **channelling** ; goût **plat**, sans fruité,
amertume attribuée au channelling.

| Grandeur | Mesure | Repère turbo |
| --- | ---: | --- |
| Pompe avant la goutte | 9,0 s (pré-infusion finie sur la goutte) | 0 dans le papier |
| Première goutte → arrêt | **10,6 s** | 7 à 15 s (papier) ; 11 à 14 s sur cette machine |
| Pression d'infusion | 2,52 → **2,33 bar**, pompe à 100 % | 6 bar au plus |
| Débit en tasse | 4,30 → **4,86 g/s** | 4,5 à 5 g/s |
| Résistance de la galette | **0,58 → 0,49 bar·s/g**, −16 % | ≈ 1 à 2 (estimé depuis le papier) |
| Poids | **44,0 g final** | 45 g visés |
| Ratio | **1:2,9** | 1:2,5 à 1:3 |
| NTC tasse | 88,77 °C ; la NTC chute de 6,9 °C pendant l'écoulement | 90 à 93 °C |

Lecture : grossir la mouture divise la résistance par 2 sans raccourcir
l'écoulement, borné par la pompe. La galette s'érode pendant le shot, ce
qu'elle ne faisait pas à mouture espresso et à pression plus haute. La
**résistance par tranche de 10 g** est la mesure de l'érosion : constante
à 15 h 35, en baisse ici.

## Pistes

Contrainte de David : **garder la mouture du single espresso**, pour faire
les deux cafés sans toucher au moulin. La résistance de la galette ne se
règle donc qu'au puck screen, à la dose et au panier ; le tassage était
déjà ferme au premier essai. La dose a été le levier décisif : 15 g
donnent 1,10 bar·s/g, dans la plage du papier.

| Piste | Depuis | Lecture |
| --- | --- | --- |
| Température plus basse | 88 °C le 26/10 à 13 h 32 (91 °C à 9 h 34) | va dans le sens de toutes les recettes récentes (83 à 90 °C). La tasse est sortie à −1,63 °C puis +0,74 °C de la consigne : 3 °C de consigne en moins n'en ont retiré que 0,63 en tasse |
| Tassage normal, comme pour l'espresso | essayé le 26/10 à 13 h 32 | **sans effet** : le tassage de 9 h 34 était déjà maximal ; 0,38 bar·s/g contre 0,43. La géométrie du panier de 58 mm avec 11 g donnerait ≈ 2,2 à 2,7 bar·s/g ; l'écart reste inexpliqué |
| Puck screen | aucun le 26/10 | protège la galette de la douche (cratères, plus marqués avec beaucoup d'espace libre) et réduit cet espace de quelques ml. Avec 15 g, la douchette marque déjà la galette : il demande probablement 14 g |
| Puissance de pompe plus basse | 100 % le 26/10 | à faible résistance, `brew.pump_pct` fixe presque le débit : ≈ 3,3 g/s à 60 %, 45 g en ≈ 13 s depuis la goutte. Piste de David contre l'érosion de la mouture grosse (16 h 32) |
| Ratio 1:3 | 45 g visés pour 15 g, 43,9 g final (1:2,9) le 26/10 à 15 h 35 | ≈ 0,9 s de débit arrive après le seuil : `lead_weight_g` ≈ 0,9 s × débit, soit ≈ 4 g à 4,4 g/s |
| Dose de 15 g | essayée le 26/10 à 15 h 35 | 1,10 bar·s/g, 4,85 bar, 4,4 g/s : galette du papier ; goût d'espresso long avec crema |

Le puck screen et la dose changent tous deux la résistance : changés
ensemble, rien dans la capture ne sépare leurs effets. La température se
lit à part dans la NTC, le ratio dans le poids.

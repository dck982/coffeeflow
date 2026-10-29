# Débitmètres — fiches et comparaison

Le capteur retenu est le **Digmesa 932-9525-B (buse 1,00 mm)**, en collecteur ouvert NPN.
Depuis le 23 octobre, il est lu par la carte sensors (`pcb/sensors/`) : pull-up de
1 kΩ et 10 nF sur FLOW, GPIO 44 (D7) du XIAO. L'ancien câblage est dans `cablage.md`.
Fiche famille FHKSC 932-952x-B : `datasheets/flowmeter-digmesa.pdf`
(courbe 1,00 mm 0° = #932-9525-B).

Ce fichier garde la comparaison des buses et du OOTDTY : le choix est fait, mais le
paysage sert encore à décider où placer le capteur et à interpréter ce qu'il mesure.

## Les capteurs

| | Digmesa FHKSC 932-9521-A | Digmesa FHKSC 932-9525-B | OOTDTY POM 1,2 mm |
| --- | --- | --- | --- |
| État | déposé (ancien) | **retenu, en service** | reçu (banc ; POM non tracé food-grade) |
| Buse / diamètre interne | 1,20 mm | **1,00 mm** | 1,20 mm |
| Plage linéaire (fiche) | 0,075 – 0,569 L/min | **0,033 – 0,40 L/min** | 0,05 – 1 L/min ±3 % |
| Loi d'impulsions | 1925 imp/L | **2382 imp/L** (montage 0°) | `Hz = 86 × Q` ±2 %, Q en L/min |
| **Impulsions / litre** | **1925** | **2382** | **5160** |
| Masse par impulsion | 0,519 g | **0,42 g** | 0,194 g |
| Pression max | **3 bar** à 20 °C | **3 bar** à 20 °C | **0,8 MPa = 8 bar** |
| Température | PVDF, NSF | PVDF, NSF | 0 – 80 °C, corps POM |
| Perte de charge | ~0,42 bar à 0,6 L/min | ~0,48 bar (à ~0,40 L/min) | non spécifiée |
| Alimentation | 5 V (OC, 3,8–20 V) | idem | DC 3,5 – 24 V, 15 mA à 5 V |
| **Sortie** | **collecteur ouvert NPN** | **collecteur ouvert NPN** | **push-pull** (>4,5 V haut à 5 V) |
| Sens de montage | 0° | 0° | horizontal (étalonnage) |

Courbe Digmesa 1,00 mm 0° : 2382 imp/L, 0,42 g/imp, linéaire dès **0,033 L/min** (0,55 g/s). L'emballage / la famille cite souvent 0,03 – 0,41 L/min ; 0,03 L/min = **0,5 g/s**.

Le OOTDTY existe aussi en 2,5 mm (`Hz = 43 × Q`, 0,3 – 4 L/min) — trop haut pour cet usage.

## Ce que ça donne sur une extraction

Base de calcul : 36 g en 28 s, soit 1,29 g/s ≈ **0,077 L/min**. Pré-infusion visée ~**0,5 g/s** = 0,03 L/min.

| | Digmesa 1,2 mm | Digmesa 1,0 mm | OOTDTY |
| --- | --- | --- | --- |
| Extraction vs min. linéaire | 1,03 × (plancher) | **2,3 ×** | 1,54 × |
| Pré-infusion 0,5 g/s | **sous** le linéaire (0,075) | **au bas** du linéaire (0,033) | sous 0,05, turbine peut-être |
| Fréquence à 0,077 L/min | 2,5 Hz | **3,1 Hz** | 6,6 Hz |
| Impulsions par tasse 36 g | 69 | **86** | 186 |
| Résolution | 0,52 g (1,4 %) | **0,42 g (1,2 %)** | 0,19 g (0,5 %) |

Le 1,0 mm Digmesa est le bon compromis **alimentaire + pré-infusion** : même interface NPN que le 1,2 mm, PVDF/NSF, et 0,5 g/s n'est plus hors plage. Plafond **0,40 L/min** — une chasse pompe ouverte côté aspiration peut saturer ; une extraction non.

Le OOTDTY reste plus fin en impulsions, mais sortie push-pull 5 V et POM sans certificat.

## Le piège du OOTDTY : sortie push-pull

Le Digmesa est en collecteur ouvert — il tire à la masse et ne monte jamais la ligne,
donc c'est le tirage qui fixe le niveau haut, et il est à 3,3 V. Le OOTDTY **pilote
activement les deux états**. Deux conséquences :

- **Aucun tirage nécessaire.** Ni résistance externe, ni `INPUT_PULLUP` : le GPIO passe en
  `INPUT` simple. Toute la discussion sur la pull-up du débitmètre tombe.
- **Alimenté en 5 V, il envoie >4,5 V sur le GPIO.** L'ESP32-S3 ne tolère pas le 5 V.
  Branché comme le Digmesa, il abîme la broche.

Deux sorties possibles, à trancher **avant de le brancher** :

1. **L'alimenter en 3,3 V.** La sortie suit l'alimentation, donc branchement direct et
   zéro composant. Mais la fiche annonce un minimum de 3,5 V : 3,3 V est hors spec de
   0,2 V. La plupart des circuits Hall descendent bien plus bas — à tester sur
   l'établi avant de conclure.
2. **Pont diviseur 10 kΩ / 20 kΩ** si le point 1 échoue : 5 V → 3,33 V, et à 6,6 Hz
   aucune constante de temps ne gêne. Il se loge dans les Wago comme le reste :

   | Nœud | Conducteurs | Borne |
   | --- | --- | --- |
   | SIGNAL brut | fil du capteur + patte R1 | 221-2411 |
   | SIGNAL divisé | patte R1 + patte R2 + fil vers le GPIO | 221-413 |
   | Masse | les trois existants + patte R2 | 221-**414** |

## En dessous de la plage : que se passe-t-il vraiment ?

C'est la question de la pré-infusion, autour de **0,03 L/min** (~0,5 g/s). Sur le
1,2 mm Digmesa et le OOTDTY, ce point est **sous** le linéaire (0,075 et 0,05). Sur le
**932-9525-B (1,0 mm), 0,033 L/min est le début de plage Digmesa** — 0,5 g/s n'est plus
hors spec. La turbine et l'inertie restent des sujets ; le seuil publié n'est toujours
pas le point d'arrêt.

1. **Au-dessus du min. linéaire** — plage spécifiée, loi linéaire (±2 % Digmesa, ±3 % OOTDTY).
2. **Entre le décrochage et ce min.** — la turbine tourne, mais le frottement des paliers
   pèse lourd face au couple moteur : le capteur **sous-compte**, de façon systématique
   et donc corrigeable, mais d'un facteur non spécifié.
3. **Sous le décrochage** — le couple hydraulique ne suffit plus à vaincre l'adhérence
   au repos. La turbine reste immobile, l'eau passe, et **rien n'est compté**.

Le point important : **le min. publié est le bas de la plage *précise*, pas le
point d'arrêt.** Le décrochage réel est plus bas, non publié — et sur une pièce à 3 CHF
il varie d'un exemplaire à l'autre.

Donc : **un volume est plus facile à obtenir qu'un débit** dans cette zone, parce qu'une
erreur d'échelle systématique se corrige par un coefficient, alors qu'un débit instantané
à 1 Hz n'a aucune finesse. Mais ça ne suffit pas, pour deux raisons :

- **On ne sait pas si la turbine tourne.** Si le décrochage est au-dessus de ton débit de
  pré-infusion, le coefficient de correction ne sert à rien : il n'y a rien à corriger.
- **La pré-infusion est un transitoire, et c'est le pire cas pour une turbine.** Le rotor
  a de l'inertie : il perd des impulsions à la mise en route et il **continue de tourner
  après l'arrêt du débit**, en ajoutant des impulsions fantômes. Sur un événement de
  5 à 10 s, ces deux effets ne sont pas négligeables et ils ne se compensent pas de
  façon fiable.

Côté résolution : une pré-infusion de 8 g fait ~19 impulsions sur le Digmesa 1,0 mm
(15 sur le 1,2 mm, ~41 sur le OOTDTY). Il y a de quoi mesurer — si le rotor tourne.

### Le protocole qui répond, à faire à réception

Dix minutes, et ça donne les deux chiffres manquants :

1. Capteur monté **horizontalement**, sortie dans une tasse sur une balance de référence, compteur
   d'impulsions sur le XIAO.
2. Faire couler à débit décroissant : ~0,15 puis 0,10, 0,077, 0,05, 0,04, 0,03,
   0,02 L/min. Chaque palier assez long pour accumuler au moins 100 impulsions.
3. À chaque palier, noter **impulsions comptées** et **grammes réels lus par la balance**.
4. Tracer imp/g en fonction du débit.

On en sort :
- le **point de décrochage** — le débit sous lequel le compte tombe à zéro ;
- la **courbe de sous-comptage** entre ce point et 0,05, donc le coefficient de
  correction applicable à la pré-infusion ;
- une **vérification du facteur K** annoncé (5160 imp/L) dans la plage nominale.

Tant que ces mesures ne sont pas faites, considérer le volume de pré-infusion comme
**non mesuré**, pas comme mesuré avec incertitude.

## Contraintes communes aux deux capteurs

- **Le capteur va en amont de la pompe**, côté basse pression : 3 bar pour le Digmesa,
  8 bar pour le OOTDTY, contre 9 bar d'infusion et davantage avant l'OPV.
- **Un filtre en entrée est obligatoire.** Avec une buse de 1,2 mm, une particule bloque
  la turbine — et une turbine bloquée se lit comme un débit nul, pas comme une panne.
- **Où débouche l'OPV — réglé.** Elle **ne renvoie pas au réservoir**. Le bac a une
  soupape de fond qui presse contre un adaptateur à deux raccords : l'un part à la pompe
  en passant par le filtre, l'autre vient de l'OPV. Le retour se fait donc **à l'entrée
  de la pompe**, en amont du débitmètre.

  Le capteur mesure par conséquent le **débit de la pompe**, pas le débit net tiré du bac.
  OPV fermée, les deux sont identiques et la lecture est celle de l'infusion. OPV ouverte,
  le capteur compte en plus l'eau recirculée et sur-lit. Avec l'OPV à 11 bar pour un
  fonctionnement à 9 bar, elle ne s'ouvre pas pendant une extraction : le seul régime
  concerné est la purge de backflush. L'emplacement retenu tient. Le tarage n'est pas
  mesuré (~10 bar d'après le plateau en panier aveugle, `cablage.md`), ni une fuite
  sous ce tarage : voir [En service](#en-service--un-k-par-régime).

  Le corollaire utile : le débit décroche de la balance exactement quand l'OPV s'ouvre,
  ce qui en fait une **mesure du seuil d'ouverture** (`firmware.md`, calibration).
- **La balance reste le meilleur débitmètre** dès que l'eau tombe dans la tasse : une
  balance de précision sort du 10–20 Hz au centigramme. L'intérêt propre de la turbine est de voir l'eau qui
  entre dans le groupe, y compris celle qu'absorbe la galette — soit précisément la
  phase que ni l'une ni l'autre ne mesure bien aujourd'hui.

## En service : un K par régime

Le firmware compte **3450 imp/L** (`screen-core/src/calibration.rs`), mesuré le
17 septembre avec le porte-filtre de simulation à ~8,5 bar, contre 2382 imp/L sur la
fiche. Les captures d'infusion comparent ce compte à la balance une fois l'eau en
tasse : grammes en tasse par ml compté, de 3 s après la première goutte jusqu'à
l'arrêt de la pompe, carte sensors (depuis le 23 octobre).

| Régime | Débit compté | Pression | Pompe | g/ml |
| --- | ---: | ---: | ---: | ---: |
| Espresso, plateau (6 infusions, 23 au 28/10) | 1,36 à 1,65 ml/s | 8,8 à 8,9 bar | 64 à 73 % | **0,79 à 0,86** |
| Espresso, 28/10 13 h 47 et 29/10 9 h 19 | 1,32 à 1,53 ml/s | 8,8 à 8,9 bar | 64 à 72 % | 1,04 et 1,11 |
| Espresso lent, 23/10 14 h 38 et 14 h 57 | 0,72 à 0,91 ml/s | 9,0 bar | 64 à 68 % | 1,00 et 0,93 |
| **Turbo shot** (7 infusions, 26 et 27/10) | **2,87 à 3,83 ml/s** | 1,9 à 5,3 bar | 70 à 100 % | **1,27 à 1,35** |

Ce qui est mesuré :

- **Dans une infusion, le compte suit la balance.** Sur le plateau, le rapport par
  fenêtre de 3 s varie de ±5 à 8 % autour de sa moyenne, soit ±1 impulsion sur
  ≈ 13. Pas de dérive ni de saut au sein d'un plateau.
- **D'un régime à l'autre, le K change d'un facteur ≈ 1,6.** Les turbo shots,
  serrés de 1,27 à 1,35 g/ml, y compris à 4,9 et 5,3 bar, comptent ≈ 1,6 fois moins
  d'eau par gramme que le plateau d'espresso. Avec le K de la fiche, ils tomberaient à
  ≈ 0,90 g/ml (eau chaude, galette) : la fiche vaut au débit élevé, et l'espresso
  compterait ≈ 1,6 fois l'eau réelle. Le K de 3450, pris à ~8,5 bar, absorbe ce
  sur-comptage au plateau et fait sous-compter les débits élevés.
- **Le turbo shot du 27/10 à 7 h 44** a la commande de pompe d'un plateau (70 %),
  mais 1,85 bar et 2,87 ml/s : 1,28 g/ml. La commande seule n'est pas la variable ;
  le débit et la pression changent ensemble dans ces données.
- **Quatre espressos sortent du groupe de 0,79–0,86**, avec un rapport tenu pendant
  tout leur plateau. Le 28/10 à 13 h 47, 1,04 / 0,83 ≈ 1,24 rejoint les volumes × 1,3
  que demandait le modèle thermique (`journal-infusions.md`).
- **Le remplissage** (3,1 à 3,3 ml/s à 70 %, sous 1 bar) est dans le régime du turbo
  shot du 27/10 à 7 h 44 : l'eau admise y serait ≈ 1,5 fois le volume compté. La loi de
  chauffe et le simulateur sont réglés sur ces comptes : l'erreur fixe se compense,
  sa variation d'une infusion à l'autre non.
- Pendant la précharge, SSR en commutation et pompe arrêtée, aucune impulsion dans
  aucune capture.

Causes possibles, aucune testée :

1. **Pulsation de la pompe vibrante.** Le capteur est sur l'aspiration. Si la colonne
   d'eau oscille dans la turbine à chaque coup de piston, le capteur à effet Hall
   compte les deux sens. L'oscillation pèse d'autant plus que le débit net est faible
   et la contre-pression forte : sur-comptage au plateau d'espresso. Le guide de boucle
   (`print/parts/guide_boucle_digmesa`) tient la boucle par le bas : il peut transmettre
   la vibration de la pompe au segment qui va au Digmesa. Proposition de David : tenir
   la boucle par le haut, les deux segments du bas libres. Cela traite la voie
   mécanique, pas la pulsation de la masse d'eau dans le tuyau, à laquelle la boucle
   répond déjà.
2. **Fuite de l'OPV sous son tarage.** Le retour de l'OPV rejoint l'aspiration en amont
   du capteur : une OPV qui fuit vers 9 bar ferait recompter l'eau recirculée au plateau
   d'espresso seulement. Les turbo shots, sous 5,3 bar, n'en auraient pas. Une fuite qui
   varie d'une infusion à l'autre irait avec les quatre espressos hors groupe.
3. **Bruit électrique**, peu probable sur le plateau : des impulsions parasites
   disperseraient le rapport fenêtre par fenêtre, alors que la pompe y est en coupure de
   phase à 64–70 %.

Essais qui les séparent :

- **Purges sur la balance**, sans porte-filtre, à 40, 60, 80 et 100 %, 10 s, deux fois
  chacune. Toutes sous 1 bar. Un rapport voisin de 1,3 g/ml à tous les débits désigne
  la pression (OPV) ; un rapport qui descend vers 0,8 aux faibles débits désigne le
  débit (pulsation). La dispersion des grammes à commande égale dit si la pompe varie.
- **Panier aveugle tenu à 9 bar** par la régulation d'infusion : aucune eau ne sort,
  toute impulsion comptée est de l'eau recirculée par l'OPV ou une oscillation dans la
  turbine.
- **Signal du Digmesa débranché** côté carte, pompe en marche : le pull-up tient la
  ligne haute, toute impulsion comptée est un parasite.

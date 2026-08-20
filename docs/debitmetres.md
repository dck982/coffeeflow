# Débitmètres — fiches et comparaison

Notes de référence pour le capteur de débit de l'**Atom Sensor**. Le câblage en vigueur
est celui du Digmesa : voir `atom_sensor.html`. Ce fichier existe pour ne pas perdre les
chiffres du second capteur, commandé comme point de comparaison.

## Les deux capteurs

| | Digmesa FHKSC 932-9521-A | OOTDTY POM 1,2 mm |
| --- | --- | --- |
| État | **en service**, reçu | commandé, ~3 CHF, non reçu |
| Buse / diamètre interne | 1,20 mm | 1,20 mm |
| Plage annoncée | 0,075 – 0,569 L/min (linéaire) | 0,05 – 1 L/min ±3 % |
| Loi d'impulsions | 1925 imp/L | `Hz = 86 × Q` ±2 %, Q en L/min |
| **Impulsions / litre** | **1925** | **5160** |
| Masse par impulsion | 0,519 g | 0,194 g |
| Pression max | **3 bar** à 20 °C | **0,8 MPa = 8 bar** |
| Température | PVDF | 0 – 80 °C, corps POM |
| Perte de charge | ~0,42 bar à 0,6 L/min | non spécifiée |
| Alimentation | 5 V | DC 3,5 – 24 V, 15 mA à 5 V |
| **Sortie** | **collecteur ouvert NPN** | **push-pull** (>4,5 V haut, <0,5 V bas à 5 V) |
| Sens de montage | 0° | horizontal (l'étalonnage y est donné) |
| Précision | — | ±3 % sur la plage, ±2 % sur la loi |

Le OOTDTY existe aussi en 2,5 mm (`Hz = 43 × Q`, 0,3 – 4 L/min) — trop haut pour cet usage.

## Ce que ça donne sur une extraction

Base de calcul : 36 g en 28 s, soit 1,29 g/s ≈ **0,077 L/min**.

| | Digmesa | OOTDTY |
| --- | --- | --- |
| Position dans la plage | 1,03 × le minimum | **1,54 ×** |
| Fréquence | 2,5 Hz | **6,6 Hz** |
| Impulsions par tasse | 69 | **186** |
| Résolution | 0,52 g (1,4 %) | **0,19 g (0,5 %)** |
| Impulsions par fenêtre de 1 s | 2 – 3 | 6 – 7 |

Le OOTDTY fait passer le débit instantané d'inutilisable à exploitable. Le Digmesa reste
correct en cumul.

## Le piège du OOTDTY : sortie push-pull

Le Digmesa est en collecteur ouvert — il tire à la masse et ne monte jamais la ligne,
donc c'est le tirage qui fixe le niveau haut, et il est à 3,3 V. Le OOTDTY **pilote
activement les deux états**. Deux conséquences :

- **Aucun tirage nécessaire.** Ni résistance externe, ni `INPUT_PULLUP` : `G38` passe en
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
   | SIGNAL divisé | patte R1 + patte R2 + fil vers `G38` | 221-413 |
   | Masse | les trois existants + patte R2 | 221-**414** |

## En dessous de la plage : que se passe-t-il vraiment ?

C'est la question de la pré-infusion, autour de **0,03 L/min** (~0,5 g/s), sous le seuil
de 0,05 des deux capteurs. La réponse n'est ni « il voit tout » ni « il ne voit rien » :
il y a **trois régimes**, et le seuil publié n'est pas celui qu'on croit.

1. **Au-dessus de 0,05 L/min** — plage spécifiée, loi linéaire, ±3 %.
2. **Entre le décrochage et 0,05** — la turbine tourne, mais le frottement des paliers
   pèse lourd face au couple moteur : le capteur **sous-compte**, de façon systématique
   et donc corrigeable, mais d'un facteur non spécifié.
3. **Sous le décrochage** — le couple hydraulique ne suffit plus à vaincre l'adhérence
   au repos. La turbine reste immobile, l'eau passe, et **rien n'est compté**.

Le point important : **le 0,05 L/min annoncé est le bas de la plage *précise*, pas le
point d'arrêt.** Le décrochage réel est plus bas, mais aucun des deux fabricants ne le
publie — et sur une pièce à 3 CHF il varie d'un exemplaire à l'autre.

Donc, pour ta question précise : **oui, un volume est plus facile à obtenir qu'un débit**
dans cette zone, parce qu'une erreur d'échelle systématique se corrige par un
coefficient, alors qu'un débit instantané à 1 Hz n'a aucune finesse. Mais ça ne suffit
pas, pour deux raisons :

- **On ne sait pas si la turbine tourne.** Si le décrochage est au-dessus de ton débit de
  pré-infusion, le coefficient de correction ne sert à rien : il n'y a rien à corriger.
- **La pré-infusion est un transitoire, et c'est le pire cas pour une turbine.** Le rotor
  a de l'inertie : il perd des impulsions à la mise en route et il **continue de tourner
  après l'arrêt du débit**, en ajoutant des impulsions fantômes. Sur un événement de
  5 à 10 s, ces deux effets ne sont pas négligeables et ils ne se compensent pas de
  façon fiable.

Côté résolution, en revanche, rien n'empêche : une pré-infusion de 8 g fait ~41
impulsions sur le OOTDTY (contre 15 sur le Digmesa). Il y a largement de quoi mesurer —
si le rotor tourne.

### Le protocole qui répond, à faire à réception

Dix minutes, et ça donne les deux chiffres manquants :

1. Capteur monté **horizontalement**, sortie dans une tasse sur une balance de référence, compteur
   d'impulsions sur l'Atom.
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
- **Où débouche l'OPV ?** À vérifier sur la machine. Si elle renvoie au réservoir, un
  débitmètre en amont de la pompe mesure le débit de la **pompe**, pas celui de
  l'infusion : l'excédent part en dérivation sans passer par le café. En profilage sous
  la pression d'ouverture de l'OPV les deux coïncident, mais pas sur une extraction à
  pleine puissance. Ça peut décider de l'emplacement plus sûrement que la plage de
  mesure.
- **La balance reste le meilleur débitmètre** dès que l'eau tombe dans la tasse : une
  balance de précision sort du 10–20 Hz au centigramme. L'intérêt propre de la turbine est de voir l'eau qui
  entre dans le groupe, y compris celle qu'absorbe la galette — soit précisément la
  phase que ni l'une ni l'autre ne mesure bien aujourd'hui.

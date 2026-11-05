# Capteur de présence d'eau XKC-Y26A-NPN

Capteur capacitif sans contact qui se clipse sur un tube non métallique et
signale la présence de liquide à sa hauteur. Fiche :
`datasheets/xkc-y26a-npn.pdf`. Un exemplaire reçu, essayé au banc le 5/11 ;
**il n'est pas monté dans la machine** (voir « Faut-il le monter »).

## Caractéristiques

| | XKC-Y26A-NPN |
| --- | --- |
| Alimentation | 5 – 12 V (variante 24 V vendue à part) ; l'exemplaire reçu fonctionne à 5,16 V |
| Consommation | ≤ 5 mA |
| Sortie | NPN collecteur ouvert, ≤ 100 mA, active à l'état bas |
| Temps de réponse | 500 ms |
| Tube | diamètre extérieur 5 – 10 mm, 11 mm au plus, non métallique |
| Température | −20 à 105 °C |
| Étanchéité | IP67 |
| Réglage | vis de sensibilité multitour : sens antihoraire = plus sensible |

Fils :

| Fil | Rôle |
| --- | --- |
| brun | + alimentation |
| bleu | GND |
| jaune | sortie NPN |
| noir | mode : **en l'air** = normalement ouvert (le transistor conduit en présence d'eau) ; **à GND** = normalement fermé (logique inversée) |

Fil noir en l'air : **eau → sortie à 0 V, air → sortie libre**, tirée au
niveau haut par la résistance de tirage. La LED du capteur s'allume en
présence d'eau.

## Essai au banc (5/11)

AtomS3, script `tests/test_xkc_y26a.py` (lecture numérique de G8 toutes les
250 ms) :

- brun sur 5 V (5,16 V mesurés), bleu sur GND ;
- jaune sur G8, tiré à 3,3 V par 4,7 kΩ ;
- noir en l'air ;
- tube silicone 5 × 7 mm rempli d'eau.

Résultats :

- **Réglage d'usine trop peu sensible** : rien n'est détecté tant que la vis
  n'a pas fait **20 à 30 tours** dans le sens antihoraire. Avant ce réglage, le
  capteur ne détectait l'eau que tenu en main.
- Après réglage : G8 = 0 avec de l'eau, G8 = 1 avec de l'air. La LED s'allume
  dès que l'eau atteint le capteur, ou en est à environ 1 cm de part et
  d'autre. Elle s'éteint quand il n'y a que de l'air autour.
- **Une colonne d'eau de 1,5 cm entre deux bulles d'air n'est pas détectée.**
  Approcher une main à moins de 1 cm la fait détecter : ce réglage est déjà au
  seuil où l'environnement compte.
- Ne pas ouvrir l'ADC sur la broche lue en numérique : sur l'ESP32-S3, l'ADC
  bascule la broche en mode analogique et `Pin.value()` lit alors 0 même à
  3,1 V.

## Câblage dans la machine

Emplacement prévu : sur le tube entre le bac et le Digmesa, en amont de la
pompe.

- **Alimentation 5 V** de la RECOM, comme le Digmesa : c'est la variante
  5 – 12 V. Le 24 V ne convient pas à cet exemplaire.
- **GND** : celui de la carte combined, donc celui de l'alimentation 24 V et
  de la pompe à engrenage.
- **Sortie** sur un GPIO, tirée à **3,3 V par 4,7 kΩ** sur la carte. Comme
  pour le Digmesa, le collecteur ouvert ne monte jamais la ligne : le GPIO ne
  voit jamais plus de 3,3 V même si le capteur est en 5 V. 0,7 mA quand le
  transistor conduit.
- **Connecteur** : le câble du capteur arrive déjà serti en **JST XH
  4 pôles**. On le garde tel quel, avec un JST XH 4 pôles vertical sur la
  carte. La broche du fil noir n'est reliée à rien sur la carte, ce qui le
  laisse en l'air (normalement ouvert).
- **Pas de condensateur sur la ligne.** Le capteur filtre déjà sur 500 ms et
  sa sortie ne rebondit pas comme un contact mécanique. Le filtrage se fait en
  logiciel (ci-dessous) : une pointe induite par les câbles de la pompe ne
  tombe que sur un échantillon et ne passe pas le filtre. Un condensateur
  ajouterait une pièce sur une carte serrée sans rien apporter de plus.

## Lecture dans le firmware

- **Scrutation, pas d'interruption** : lire le GPIO toutes les **50 ms**.
- Un changement d'état n'est accepté qu'après **500 ms d'état constant**
  (10 échantillons identiques).
- **Bac vide** : état « air » accepté pendant **1 s** pendant que la pompe
  tourne → consigne pompe à 0 et erreur « bac vide ».

Ce délai laisse passer les bulles isolées. Le tube de 5 mm de diamètre
intérieur a une section de 19,6 mm² : 1 ml/s y fait 5,1 cm/s.

| Régime | Débit | Vitesse dans le tube | Passage d'une bulle de 2 cm |
| --- | --- | --- | --- |
| Espresso lent | 0,72 ml/s | 3,7 cm/s | 0,54 s |
| Espresso, plateau | 1,36 – 1,65 ml/s | 7 – 8 cm/s | 0,25 – 0,29 s |
| Remplissage | 3,6 ml/s | 18 cm/s | 0,11 s |

Le capteur fait la moyenne sur 2 à 3 cm de tube avec un temps de réponse de
500 ms. Une bulle qui ne remplit pas toute la section ne change pas sa
lecture. Une bulle qui la remplit passe plus vite que les 500 ms. Seul un bac
vide envoie de l'air en continu.

## Sensibilité et fausses détections

Les deux erreurs ne coûtent pas la même chose :

- **Faux « air »** (eau présente, capteur muet) : arrêt sans raison. Gênant,
  sans danger.
- **Faux « eau »** (air présent, capteur allumé) : la pompe tourne à sec.
  C'est l'erreur à éviter.

Le réglage visé est donc **juste au-dessus du seuil de détection**, pas une
sensibilité assez haute pour voir une colonne de 1,5 cm.

- **Régler en place, pas au banc.** Le capteur mesure par rapport à son GND.
  Au banc, ce GND suit l'USB du portable. Tenu en main, le corps sert de
  grand plan de masse et la mesure gagne beaucoup de signal. Dans la machine,
  le GND de la combined est relié à l'alimentation 24 V et à la pompe, avec
  du métal autour : le capteur s'y comportera plutôt comme tenu en main. Le
  réglage du banc y sera probablement trop sensible.
- **Méthode** : tube monté et fixé, pompe à l'arrêt puis en marche. Partir
  peu sensible et tourner dans le sens antihoraire jusqu'à ce que la LED
  s'allume tube plein. Ajouter 1 à 2 tours, puis vérifier que la LED
  s'éteint tube vidé, pompe en marche.
- **Pompe à engrenage** : à 500 ms, le capteur est peu sensible au bruit
  électrique du moteur BLDC, et la fiche annonce un filtrage des parasites.
  Une masse métallique proche décale la référence, mais ce décalage est fixe
  et le réglage en place le compense. Garder le capteur à **au moins 3 cm de
  la pompe** et de toute pièce qui bouge ou vibre.
- **Tube** : le tenir de part et d'autre du capteur. S'il bouge dans la pince,
  le couplage change.
- **Présence humaine** : au banc, une main ne compte qu'à moins de 1 cm.
  Derrière un panneau, à 20 – 30 cm, c'est négligeable. Le cas réaliste est
  une main près du tube pendant le remplissage du bac. La pompe ne tourne pas
  à ce moment-là.

## Faut-il le monter

**Par défaut, non.** Le capteur est placé sur le tube, après le bac : il voit
l'air **en même temps que la pompe**, pas avant. Il n'avertit pas d'un bac
presque vide, il détecte un bac déjà vide. C'est déjà le rôle de la règle de
marche à sec de [pompe-engrenage.md](pompe-engrenage.md) : pompe au-dessus de
1 000 tr/min, aucune impulsion Digmesa pendant 2 s, pression qui ne monte pas
→ arrêt.

| | Capteur sur le tube | Règle Digmesa + pression |
| --- | --- | --- |
| Délai de détection d'un bac vide | ~1,5 s (500 ms capteur + 1 s de confirmation) | ~2 s |
| Pièce en plus | capteur, JST XH 4 pôles vertical, 4,7 kΩ | aucune |
| GPIO | le dernier libre | aucun |
| Réglage | sensibilité en place, à refaire après toute intervention | seuils logiciels |
| Avertit avant que le bac soit vide | non | non |

La fenêtre de 2 s de la règle Digmesa ne doit pas descendre à 1 s : le
Digmesa compte 2,38 impulsions/ml. À 0,5 ml/s (pré-infusion), cela fait une
impulsion toutes les 0,84 s, et la turbine décroche près de ce débit.

**Ce qui ferait changer d'avis** : l'essai « bac débranché » prévu pour
régler la règle de marche à sec. Si la turbine du Digmesa compte des
impulsions quand la pompe aspire de l'air, la règle ne détecte plus rien
pendant un remplissage, où la pression ne monte pas de toute façon. Le
capteur sur le tube redevient alors la seule détection fiable pendant le
remplissage.

## Le dernier GPIO : réservé au niveau bas du bac

Détecter un bac vide arrive trop tard. Si la pompe s'est désamorcée, il faut
ouvrir la machine pour la réamorcer à la main. Ce qui sert vraiment, c'est un
avertissement « bac bas », à quelques centimètres du fond, avant que l'air
n'atteigne le tube. **Si le dernier GPIO libre de la combined sert un jour,
ce sera pour ce capteur-là**, pas pour le Y26A.

Le Y26A ne convient pas : il ne prend que des tubes de 5 à 11 mm de
diamètre. Il faut un capteur capacitif pour paroi de réservoir.

Contrainte : le bac est amovible et plus petit que son logement, avec
beaucoup de jeu autour. Le capteur ne peut pas être fixé sur le bac. Il doit
être monté sur la machine et se retrouver plaqué contre la paroi du bac une
fois celui-ci en place. Une tôle métallique entre le capteur et le bac
masquerait l'eau : le capteur doit toucher le bac directement, par une
découpe dans la tôle ou sur un support qui dépasse de la tôle.

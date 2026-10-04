# Écran d'infusion

Depuis l'écran **0.3.38**, une infusion s'affiche sur un écran unique : du
démarrage à la fermeture, il présente la frise des phases et le résumé
calculé pendant la capture HF. La purge garde l'écran de cycle précédent.

Code : `firmware/screen/main/core/shot_summary.h` (résumé, testé sur l'hôte
par `test/test_shot_summary.cpp`) et `render_brew()` dans
`firmware/screen/main/ui/ui_home.cpp`. `describe_hf_capture.py` calcule le
même résumé (`shot_summary()`) en tête du rapport HTML ; sur la capture
`261004-100128`, les deux donnent les mêmes valeurs.

## Disposition

| Zone (y écran) | Pendant l'écoulement | Après l'arrêt de la pompe |
| --- | --- | --- |
| 84 | phase en cours ; date et heure du départ à droite | « dernières gouttes », « écoulement », puis « terminé » |
| 122–148 | frise, segment en cours cerclé de blanc ; réserve des gouttes en contour | frise complète |
| 154 | durée de chaque segment, gain des gouttes à droite | idem |
| 192–356 | six tuiles | idem, valeurs finales |
| 376–464 | tuile de fonctionnement (chauffe, pompe, débit balance, débit pompe) et « arrêter » | « fermer », grisé avec le compte à rebours de la capture, actif à sa fin |

Les six tuiles, en colonnes totaux, infusion, température :

| | Totaux | Infusion | Température |
| --- | --- | --- | --- |
| Ligne 1 | poids | temps infusion | moyenne en tasse · cible |
| Ligne 2 | temps total · pompe | débit infusion (g/s) · débit débitmètre (ml/s) | baisse thermique · min–max |

Une valeur encore inconnue s'affiche « - ». Les polices embarquées (Inter,
ASCII et Latin-1) n'ont ni tiret long ni signe moins typographique : les
signes et les plages utilisent « - ».

## Définitions

Toutes les durées partent du début de la capture HF, c'est-à-dire de l'appui
sur « infuser ».

| Valeur | Définition |
| --- | --- |
| Segments | précharge, remplissage, pré-infusion selon l'état de la machine ; l'infusion (rampe comprise) est coupée en **montée** puis **infusion** au premier échantillon à consigne − 1 bar ou plus ; **gouttes** de l'arrêt de la pompe à leur fin |
| Pression jamais atteinte | la montée devient l'infusion à l'arrêt de la pompe |
| Temps total | jusqu'à l'arrêt de la pompe ; « pompe » depuis son démarrage |
| Temps infusion | de consigne − 1 bar à l'arrêt de la pompe ; sans pression atteinte, toute la phase d'infusion, connue seulement à l'arrêt |
| Débit infusion | poids et volume amont gagnés sur la même fenêtre, divisés par sa durée ; inconnu sous 1 s |
| Poids | relatif au premier échantillon, suivi jusqu'à la fin des gouttes puis figé |
| Gouttes | après l'arrêt de la pompe, jusqu'au premier débit en tasse nul ou inconnu (fenêtre de 2 s) ; une tasse retirée (poids négatif) les termine |
| Débit balance (tuile du bas) | poids gagné sur les 2 dernières secondes ; inconnu si une mesure manque ou est négative dans la fenêtre |
| Moyenne en tasse | NTC pondérée par chaque gramme arrivé, poids en maximum courant, jusqu'à l'arrêt de la pompe (critère de [chauffe-infusion.md](chauffe-infusion.md#objectif)) |
| Baisse thermique | température au début de la capture moins le minimum pompe en marche ; min et max sur la même période |

La fin des gouttes utilise une fenêtre de 2 s qui se termine à l'échantillon
courant, comme l'estimation de fin de la chauffe. Le graphe du rapport HTML
lisse le débit sur une fenêtre centrée : sa courbe de gouttes retombe à 0
environ 1 s plus tôt que la frise.

## Échelle de la frise

| Moment | Échelle |
| --- | --- |
| Avant le démarrage de la pompe | précharge réglée + 30 s, plus 4 s de réserve |
| Pompe en marche | démarrage de la pompe + 30 s, plus 4 s ; quand l'écoulé atteint 90 % de l'arrêt prévu, celui-ci grandit de 25 % ; il ne diminue jamais |
| Après l'arrêt de la pompe | arrêt réel + 4 s, ou l'écoulé si les gouttes durent plus |
| Gouttes terminées | fin des gouttes : la frise remplit la largeur |

Les 30 s correspondent au temps de pompe visé. Le cycle précédent n'est pas
retenu : la machine est souvent éteinte entre deux cafés. Sur la capture
`261004-100128` (pompe de 41,3 s), l'échelle passe de 39,7 à 48,6 s à
32,2 s, puis se cale sur l'arrêt (51,0 s).

Un segment affiche le plus long de ses trois libellés qui tient dans sa
largeur (« PRÉ-CHAUFFE », « CHAUFFE », « C » ; « REMPLISSAGE », « REMPL. »,
« R » ; « PRÉ-INFUSION », « PRÉ-INF. », « PI » ; « MONTÉE », « MONT. », « M » ;
« INFUSION », « INF. », « I » ; « GOUTTES », « GTTE », « G »), en 18 px.
Sa durée s'affiche dessous si elle tient dans le segment.

Couleurs : pré-chauffe `kFault`, remplissage `kThermal`, pré-infusion
`kSurfaceHigh`, montée `kRampLow`, infusion `kAccent`, gouttes `kTextFaint`.
Libellés en #EEEEEE, sombres sur l'infusion.

## Vérification

```sh
sh firmware/screen/test/run_tests.sh
cmake --build firmware/screen/ui_sim/build -j 4
firmware/screen/ui_sim/build/ui_snapshot out.ppm brew-30
```

Les scénarios `brew-<t>` de `ui_sim` rejouent une infusion synthétique à
`t` secondes (pompe de 5,5 à 40 s, capture terminée à 60 s) ;
`replay=<csv>@<t>` rejoue une capture convertie, une ligne par échantillon :
`t mode pompe pression_valide pression balance poids volume ntc_valide ntc`,
avec `mode` 0 précharge, 1 remplissage, 2 pré-infusion, 3 infusion,
4 récupération.

# Chauffe au repos

Ce document décrit la régulation de la chaudière au repos, hors infusion,
purge et récupération (version écran **0.3.29**). Le code est dans
`firmware/screen/main/core/thermal_control.h`, classe `Controller`.

## Périmètre

| Situation | Loi |
| --- | --- |
| **Repos** (ce document) | prédicteur de retard |
| Précharge et écoulement d'une infusion | loi d'infusion, voir [chauffe-infusion.md](chauffe-infusion.md) |
| Purge, puis récupération (30 s après un écoulement) | prédiction par la pente et la chaleur en transit, voir la [régulation de la chaudière](chauffe-chaudiere.md) |

La récupération dure 30 s, plus que le retard de 23,5 s du prédicteur :
quand le repos reprend, l'historique de commande qu'il consulte ne
contient plus aucun écoulement.

## Constats

Deux captures de surveillance du 29 septembre, consigne 90 °C, version
0.3.27 :

- `monitor-heating-20260929-080056-320888.json` : 5 min de montée depuis
  78,5 °C ;
- `monitor-heating-20260929-081752-779702.json` : 3 min autour de la
  consigne.

L'ancien régulateur (jusqu'à 0.3.28) entretenait un cycle :

| Grandeur | Mesure |
| --- | --- |
| Période | ≈ 90 s |
| NTC | 89,6–89,8 °C au creux, 91,2–91,4 °C au sommet |
| Moyenne sur un cycle | **90,6 °C**, au-dessus de la consigne |
| Puissance moyenne sur un cycle | **2,1 à 2,7 %** (26 à 32 W) |
| Commande au plus fort | 8 à 10 % |

Déroulé d'un cycle (capture de 8 h 17) : sommet à 91,34 °C (8 s),
commande nulle jusqu'à 90,86 °C en descente (36 s), montée de la commande
jusqu'à 8,3 % pendant la descente, creux à 89,85 °C (68 s), coupure à
90,0 °C **en montée** (77 s), puis nouveau sommet à 91,19 °C (98 s). Après
chaque coupure, la NTC monte encore pendant **21 à 22 s**.

Ajusté sur ces deux captures, un modèle « retard pur + capacité + pertes »
donne :

| Paramètre | Valeur |
| --- | --- |
| Retard commande → NTC | **23,5 s** |
| Capacité effective | 1,28 kJ/K |
| Pertes | 0,45 W/K au-dessus de 24,5 °C, soit **30 W (2,5 %)** à 90,6 °C |
| Écart RMS | 0,23 et 0,25 °C |

Au repos, le retard est bien plus long que l'amorce de 8 s mesurée en
chauffe forte : une faible puissance met plus de temps à atteindre la
sonde.

## Diagnostic de l'ancien régulateur

1. **Le retard n'était pas modélisé comme tel.** L'ancien régulateur
   extrapolait la pente de la NTC sur 20 s (après un filtre de 8 s) et
   n'ajoutait qu'une chaleur en transit partielle : la commande au-delà de
   3,5 %, sur 22 s, bornée à 1,5 °C. La pente reflète des commandes
   anciennes ; son extrapolation coupe trop tôt en montée et relance trop
   fort en descente. L'intégrale, nourrie par cette erreur extrapolée,
   gonflait pendant la descente alors que la chaleur était déjà en route.
2. **Le gain était trop élevé pour ce retard.** Au repos, la chaudière est
   presque un intégrateur : +0,0094 °C/s par point de commande. Avec
   8 % par °C et 23,5 s de retard, la boucle est instable : le cycle en est
   la conséquence.
3. **Le maintien de 3,5 % dépassait les pertes réelles (2,5 %)**, et
   l'intégrale ne pouvait pas descendre sous 0, ni survivre au-delà de
   consigne + 0,5 °C. L'équilibre ne pouvait s'établir qu'au-dessus de la
   consigne.

## Loi

À chaque pas (250 ms) :

```
pertes      = 0,45 W/K × (NTC − 24,5 °C) / 12 J par %·s      (2,45 % à 90 °C)
en_attente  = Σ sur les 23,5 dernières s de (commande − pertes) × dt × 12 / 1 280
erreur      = consigne − (NTC + en_attente)
commande    = pertes + 6 × erreur + intégrale, bornée à 0–100 %
```

- **Prédicteur de retard** : `en_attente` est l'élévation de température
  que produira la chaleur déjà commandée mais pas encore visible à la NTC.
  La pente de la NTC n'est plus utilisée.
- **Intégrale** : gain 0,05 % par °C·s sur l'erreur prédite, bornée à
  −10…+35 %. Elle corrige l'erreur du modèle de pertes, peut donc être
  négative, et n'est plus remise à zéro au-dessus de la consigne. Elle
  n'intègre que si la NTC est à moins de 8 °C de la consigne, et pas dans
  le sens d'une sortie saturée (0 % ou 100 %). Elle est conservée pendant
  les infusions et purges ; un changement de consigne, une mesure
  invalide ou la chauffe désactivée la remettent à zéro.
- **Pas de filtre de sortie** au repos.

## Réglages

| Constante (`Controller::`) | Valeur | Origine |
| --- | --- | --- |
| `kIdleDelayS` | 23,5 s | ajustement sur les captures de repos |
| `kIdleHeatGainCPerPctSecond` | 12 / 1 280 °C par %·s | capacité de 1,28 kJ/K |
| `kIdleLossPctPerK`, `kAmbientC` | 0,45 / 12 % par K, 24,5 °C | pertes ajustées |
| `kIdleProportionalPctPerC` | 6 % par °C | simulation (ci-dessous) |
| `kIdleIntegralPctPerCSecond` | 0,05 % par °C·s | simulation |
| `kIdleIntegralMinPct`, `kIdleIntegralMaxPct` | −10 %, +35 % | — |
| `kIdleIntegralBandC` | 8 °C | ancien régulateur |

## Simulation

L'ancien régulateur et le prédicteur ont été portés en Python et simulés
en boucle fermée, sur deux modèles de chaudière : le modèle de repos
ci-dessus, et celui ajusté sur les infusions (retard réparti, délai moyen de
17,6 s, voir [chauffe-infusion.md](chauffe-infusion.md#simulation)). Bruit de
la NTC : 0,01 °C.

| Régulateur | Modèle | Régime établi | Depuis 88,4 °C : ±0,2 °C atteint | Depuis 25 °C : ±0,2 °C atteint |
| --- | --- | --- | ---: | ---: |
| Ancien | repos | cycle 89,85–90,71 °C, 75 s, moyenne 90,25 °C | jamais | jamais |
| Ancien | infusions | cycle 89,92–90,27 °C, 49 s | jamais | jamais |
| Prédicteur, Kp 4, Ki 0,05 | repos | 90,00 °C | 187 s, max 90,34 °C | 309 s, max 90,55 °C |
| **Prédicteur, Kp 6, Ki 0,05** | repos | **90,00 °C** | **59 s**, max 90,20 °C | **212 s**, max 90,21 °C |
| **Prédicteur, Kp 6, Ki 0,05** | infusions | **90,00 °C** | **75 s**, max 90,11 °C | **396 s**, max 90,35 °C |
| Prédicteur, Kp 8, Ki 0,1 | repos | 90,00 °C | 100 s, max 90,21 °C | 131 s, max 90,15 °C |

Sur le modèle de repos, un retard supposé de 18 s ou de 30 s au lieu de
23,5 s reste stable (±0,2 °C atteint en 177 à 186 s depuis 88,4 °C, avec
Kp 4). Des pertes supposées de 3,5 % au lieu de 2,5 % laissent un écart que
l'intégrale résorbe lentement.

Kp 6 est retenu : stable sur les deux modèles, avec une marge sur Kp 8. La
simulation ignore les impulsions de 100 ms du SSR sous 10 % ; avec
l'ancien régulateur, elle donne un cycle deux fois moins ample que le cycle
réel. Le modèle de repos est donc optimiste : la prochaine capture dira si
le prédicteur tient sur la machine.

## Première capture (29 septembre, 8 h 45)

`monitor-heating-20260929-084528-643973.json` : 0.3.29, 5 min de montée
depuis 72,5 °C, régulateur démarré 30 s avant la capture (après flash).

| | 0.3.27 (8 h 17, 0–180 s) | 0.3.29 (8 h 45, 200–300 s) |
| --- | ---: | ---: |
| NTC moyenne | 90,61 °C | **90,14 °C** |
| Écart-type | 0,50 °C | **0,10 °C** |
| Plage | 89,81–91,34 °C | **89,96–90,32 °C** |
| Commande | 0 à 8,3 %, par à-coups | **1,7 à 3,1 %**, continue |
| Commande moyenne | 2,15 % | 2,53 % |

Le cycle de 90 s a disparu. Restent :

- **un dépassement en montée** : 91,01 °C vers 100 s (+1,0 °C), creux à
  89,62 °C vers 164 s, 90,35 °C vers 195 s, puis stabilisation. La
  simulation prévoyait +0,2 à +0,35 °C. Hypothèse : en chauffe forte,
  environ 20 % de la chaleur atteint la sonde après 23,5 s (modèle des
  infusions), et le retard pur l'ignore. Cela ne concerne que la montée
  depuis loin de la consigne ; la reprise après une purge ou une infusion
  passe par la récupération ;
- **une moyenne à +0,14 °C et une ondulation de ±0,18 °C** (période
  ≈ 90 s) sur les 100 dernières secondes : trop court pour savoir si
  l'intégrale ramène la moyenne à la consigne.

Décision : ne rien changer avant une capture plus longue au repos.
Pistes si le dépassement ou l'ondulation gênent : un retard réparti au lieu
du retard pur, ou Kp 4 (plus lent : 187 s au lieu de 59 s depuis 88,4 °C en
simulation).

## À vérifier sur la prochaine capture

Faire **10 min** de surveillance (`record_heating.py --mode monitor`) en
partant d'une machine déjà stabilisée à la consigne :

| Point | Attendu |
| --- | --- |
| Moyenne | ramenée vers la consigne par l'intégrale (+0,14 °C à 8 h 45) |
| Ondulation | ±0,18 °C à 8 h 45 : amortie, ou au moins pas plus ample |
| Commande | ≈ 2,5 %, continue, sans à-coups |

Si un cycle subsiste, sa période indique le retard réel : une période
d'environ 4 fois le retard suggère de recaler `kIdleDelayS`. Une moyenne
décalée de la consigne indique une erreur de pertes ; l'intégrale devrait
la résorber en quelques minutes.

## Limites

- Le retard dépend de la sonde et de la puissance. Il faudra le recaler
  avec la PT1000.
- Les pertes dépendent de la pièce ; l'intégrale absorbe l'écart.
- Une chauffe forte (montée depuis l'ambiante) atteint la sonde plus vite
  que 23,5 s : le prédicteur est alors prudent, la montée finale plus
  lente mais sans dépassement.

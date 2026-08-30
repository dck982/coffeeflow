# Connexion du capteur de débit Digmesa FHKSC au M5Stack Atom Echo S3R

## Référence du capteur

**Digmesa FHKSC 932-9521-B90**
- Buse : Ø 1.20 mm
- Sortie : collecteur ouvert NPN
- Sortie signal : onde carrée, duty cycle ~50%
- Alimentation : +3.8 à +20 VDC
- Consommation : < 8 mA
- Charge signal max (courant de sink) : 20 mA
- Niveau bas (saturation) : < 0.7 V
- Fuite en état haut : max 10 µA
- Connexions : GND, +V (5V), Signal

## Schéma électrique

```
                    3.3V (Atom)
                       │
                       ├──────────────┐
                       │              │
                    ┌──┴──┐           │
                    │ 1kΩ │           │
                    └──┬──┘           │
                       │              │
   Capteur ──SIGNAL────┼──────────────┼──── GPIO (Atom)
   (collecteur          │              │
    ouvert NPN)         │           ┌──┴──┐
                         │           │100nF│
                         │           └──┬──┘
                         │              │
   Capteur ──GND─────────┴──────────────┴──── GND (Atom)

   Capteur ──5V──────────────────────────────── 5V (Atom)
```

### Composants
- **R1 = 1 kΩ** entre SIGNAL et 3.3V (pull-up)
- **C1 = 100 nF** (TRU COMPONENTS TC-K100NF5, céramique THT, 50V, 20%) entre SIGNAL et GND (filtrage anti-bruit)

### Points importants
- Le pull-up est câblé vers **3.3V**, pas 5V, car les GPIO de l'ESP32-S3 ne sont pas tolérants au 5V. Le capteur peut être alimenté en 5V sans problème car sa sortie est en collecteur ouvert : le niveau haut du signal est fixé par la tension du pull-up, pas par l'alimentation du capteur.
- Le pull-up interne du GPIO (~45 kΩ, activé par défaut par MicroPython) n'est **pas utilisé** ici : une résistance externe plus forte (1 kΩ) associée au condensateur de filtrage offre une bien meilleure immunité au bruit électromagnétique, ce qui est important à proximité d'une pompe/moteur.

## Débit attendu et fréquence des impulsions

**Plage de débit visée : 0.06 à 0.18 l/min**

Facteur du capteur (buse Ø 1.20 mm) : **1925 pulses/litre**

| Débit (l/min) | Fréquence (Hz) | Période (ms) |
|---|---|---|
| 0.06 | 1.93 | ~520 |
| 0.18 | 5.78 | ~173 |

Formule : `f (Hz) = débit (l/min) × 1925 / 60`

### Remarques
- Signal très basse fréquence (quelques Hz) : aucune contrainte de timing serrée pour le comptage d'impulsions par interruption GPIO côté MicroPython.
- Le filtre RC (1 kΩ + 100 nF, coupure ≈ 1.6 kHz) est très largement au-dessus de la fréquence utile — aucune déformation du signal, filtrage efficace du bruit haute fréquence.
- Le fabricant garantit la linéarité de calibration à partir de **0.07 l/min** (début de la plage linéaire pour cette buse). En dessous (notre cas à 0.06 l/min), s'attendre à une précision de mesure dégradée par rapport à la valeur nominale de 1925 pulses/litre — une calibration spécifique au point bas est recommandée si la précision y est critique.

## Sources
- Datasheet Digmesa FHKSC 932-952x-Bxxx : `docs/datasheets/flowmeter-digmesa.pdf`
- Datasheet Digmesa FHKSC 974-950X/XXX (schémas d'interfaçage collecteur ouvert, génériques à la famille FHKSC)

# print/

Pièces FDM de coffeeflow. Vue d'ensemble, câblage et brochage : `../README.md` et `../docs/cablage.md`.

Quatre boîtiers autour de la machine :

- **`boitier_ps`** — alimentation RECOM et Wago 230 V, intérieur, face ouest le long du réservoir
- **`boitier_dc`** — XIAO ESP32-S3 + Grove Shield, Adafruit CAN Pal, Wago 5 V ; intérieur, zone froide entre le module PID et le cadran manomètre
- **`boitier_ac`** — dimmer 4 A DimmerLink et M5Stack Unit SSR ; accolé au DC
- **UI** — `screen_wedge` + `screen_base`, façade, écran Waveshare 4,3"

`boitier_dc` et `boitier_ac` sont **côte à côte et reliés par un couvercle commun** (`couvercle_acdc`) ; l'assemblage des deux bacs est `ensemble_boitiers`.

nurb sert **ce** dossier (un projet nurb est un répertoire qui contient `parts/`) :

```
cd print
nurb dev          # http://127.0.0.1:7373
nurb export canal
```

**Python :** sur cette machine, préférer **`uv`** (`uv run …`, `uv run --with nurb …`). Le binaire système `python` / `python3` n’est pas fiable ici ; `nurb` lui-même est installé via `uv tool install nurb` et reste sur le PATH.

**Export :** format préféré **3MF** (`printer.toml` → `[export] formats = ["3mf"]`). Bambu Studio peut avertir sur la version 1.41 du 3MF nurb ; l’objet s’importe quand même. STL uniquement si besoin (`nurb export … --formats stl`).

## Les objets

| Fichier | Rôle |
| --- | --- |
| `parts/boitier_ps.py` | Bac de l'alimentation (RECOM, Wago 230 V, Wago 5 V) |
| `parts/couvercle_ps.py` | Couvercle du bac alimentation |
| `parts/boitier_dc.py` | Bac intérieur ouest : XIAO + Shield, CAN Pal, Wago 5 V |
| `parts/boitier_ac.py` | Bac intérieur est : dimmer et SSR |
| `parts/couvercle_acdc.py` | Couvercle unique des deux bacs |
| `parts/ensemble_boitiers.py` | Assemblage DC + AC |
| `parts/passe_cable.py` | Passe-câble fileté pour le trou Ø16 de l'ex-bouton brew (câble CAN) |
| `parts/ecrou_passe_cable.py` | Écrou SW22 du passe-câble |
| `parts/ensemble_passe_cable.py` | Assemblage passe-câble + écrou + tôle Ø16 |
| `parts/screen_wedge.py` | Cadre de l'écran Waveshare (vis M2.5 à l'arrière) |
| `parts/screen_base.py` | Berceau du wedge |
| `parts/screen_assembly.py` | Assemblage wedge + base |
| `parts/canal.py` | Goulotte de guidage des fils |
| `parts/base_pesage.py`, `plateau_pesage.py`, `ensemble_pesage.py`, `butee_goupille.py`, `goujon_indexage.py` | Pesée drip tray — **en pause** (le poids vient d'une Acaia Lunar en BLE). Cotes : `../docs/driptray.md` |
| `measurements.toml` | Cotes (Wago, aimants, Helutherm, M3, modules) |
| `printer.toml` | A1 Mini |
| `system.py` | Filet imprimable, puits d'aimant couchés, ouvertures des modules |

Traversée intérieur → façade : **câble CAN** (paire torsadée orange/gris) par le trou **Ø 16 mm** de l'ancien bouton brew, avec `passe_cable`. Le 5 V de l'écran vient de `boitier_ps` par un bornier adaptateur USB-C. Le 230 V ne quitte pas le compartiment technique.

Encombrements machine : `../docs/profitec_go.html`.

## Matière

**PLA HT recuit** pour les boîtiers : après recuit, la tenue en température monte à **140 °C**, très au-dessus des 40–50 °C du compartiment technique, et sans le fluage du PLA standard dès ~45 °C. La machine tourne 10–15 min, deux fois par jour, et reste à l'ambiante le reste du temps.

Le recuit **retire les pièces** : vérifier les cotes fit-critiques (puits d'aimant, logements Wago, inserts, filet du passe-câble) **après** recuit, pas sur la pièce sortie du plateau.

`screen_wedge` est la pièce visible : l'*ironing* de sa face supérieure est la raison de son réglage à part.

`printer.toml` déclare `material = "pla"` ; toutes les cartes de pièces sont alignées dessus. Il n'y a plus de PETG dans le projet.

## Wago et visserie

Côté machine : FASTON 6,3 × 0,8 mm isolées nylon. Côté mod : Wago 221 (412 / 415 / 423). Pas de piggyback.

- Vis **M3×10** classique (trou pilote 2,6 mm) ; **M2.5×8** pour l'écran dans le wedge
- Inserts laiton **M2.5×4** et **M3** (stock atelier) pour le montage des cartes
- Aimants **8 × 3 mm**, fond de puits 0,6 mm
- Helutherm 145 : **0,75 mm²** en 230 V (paires L/N sous gaine thermo), **0,25 mm²** en 5 V / signaux / CAN

## Atelier

Bambu Lab A1 Mini. Sécher le filament avant une pièce visible (le stringing se voit).

Contraintes d'une pièce : sa carte `parts/<nom>.md` (`## Don't`), pas un `docs/` à part.

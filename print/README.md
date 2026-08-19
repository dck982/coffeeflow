# print/

Pièces FDM de coffeeflow. Vue d'ensemble et schéma électrique : `../README.md`. Firmware à la racine du dépôt.

nurb sert **ce** dossier (un projet nurb est un répertoire qui contient `parts/`) :

```
cd print
nurb dev          # http://127.0.0.1:7373
nurb export boitier_4w
nurb export boitier_2w
```

**Python :** sur cette machine, préférer **`uv`** (`uv run …`, `uv run --with nurb …`). Le binaire système `python` / `python3` n’est pas fiable ici ; `nurb` lui-même est installé via `uv tool install nurb` et reste sur le PATH.

**Export :** format préféré **3MF** (`printer.toml` → `[export] formats = ["3mf"]`). Bambu Studio peut avertir sur la version 1.41 du 3MF nurb ; l’objet s’importe quand même. STL uniquement si besoin (`nurb export … --formats stl`).

## Les six objets

| # | Pièce | Où | Matière | Statut |
| --- | --- | --- | --- | --- |
| 1 | Dérivation 230 V (`boitier_4w` / `couvercle_4w`, `boitier_2w` / `couvercle_2w`) | intérieur, ~10 cm du boiler | PETG | actuel |
| 2 | `canal` × 2 (230 V et 5 V, un de chaque côté du réservoir) | intérieur, ~20 cm du boiler | PETG | actuel |
| 3 | Boîtier **Atom Sensor** (Atom Echo S3R, KISOMeter, XDB401, débitmètre) | intérieur | PETG | à faire |
| 4 | `screen_wedge` (cadre écran Waveshare, sert de couvercle à la base) | dessus | **PLA** | en validation |
| 5 | `screen_base` (Atom Control, PbHub, ACSSR, relais NC, RobotDyn, alim 5 V, fuse box) | dessus, sur le plateau | PETG | à dimensionner |
| 6 | Grille d'entretoise 5 mm sous `screen_base` | dessus, contact plateau | PETG | à faire |

Traversée intérieur → extérieur : 4 fils 230 V dans un canal, le câble USB-C Sensor → Control dans l'autre.

`screen_base` n'est pas encore dimensionnée : son volume découle du placement réel des modules du BOM (voir `../README.md`).

## Matière par pièce

Châssis 40–50 °C, plateau supérieur un peu plus. Cycle : 10–15 min de marche, 2× par jour, ambiante le reste du temps.

**PETG par défaut** (`printer.toml`). Le PLA flue lentement sous charge dès ~45 °C, et cinq des six pièces sont soit dans l'air enfermé de la machine, soit en contact avec le plateau. La grille (#6) est le pire cas : la plus chaude *et* la seule en compression permanente.

**`screen_wedge` en PLA**, seule exception, pour l'*ironing* de la face supérieure — le PETG ne s'ironise pas. C'est la pièce la plus loin de la source et le cycle est trop court pour qu'elle atteigne l'équilibre thermique. À surveiller : l'alim vit dans `screen_base` juste dessous, et la chaleur monte. Si le cadre gondole, retour au PETG.

Détail et chiffres : section « Matières d'impression » de `../README.md`.

## Quatre Wago 230 V

1. Phase machine allumée (relais boiler du PID)
2. Neutre (pompe)
3. Phase brew (sortie vanne)
4. Phase pompe (entrée pompe : relais NC ou dimmer)

Côté machine : FASTON 6,3 × 0,8 mm isolées nylon. Côté mod : Wago. Pas de piggyback.

## Contenu actuel

| Fichier | Rôle |
| --- | --- |
| `parts/boitier_4w.py` | Boîtier 4 Wago verticaux |
| `parts/couvercle_4w.py` | Couvercle M3 (2 vis) |
| `parts/ensemble_4w.py` | Assemblage 4 Wago + châssis |
| `parts/boitier_2w.py` | Boîtier 2 Wago (vis centrée, 1 aimant) |
| `parts/couvercle_2w.py` | Couvercle M3 (1 vis) |
| `parts/ensemble_2w.py` | Assemblage 2 Wago + châssis |
| `parts/canal.py` | Canal de guidage des fils |
| `parts/canal_test.py` | Coupon de fit du canal |
| `parts/screen_wedge.py` | Cadre de l'écran Waveshare (PLA) |
| `parts/screen_base.py` | Berceau du wedge, contiendra l'électronique |
| `parts/screen_assembly.py` | Assemblage wedge + base |
| `measurements.toml` | Cotes (Wago, aimants, Helutherm, M3) |
| `printer.toml` | A1 Mini, PETG HF 33102 |
| `system.py` | Layout partagé 2w / 4w |

Contraintes d'une pièce : sa carte `parts/<nom>.md` (`## Don't`), pas un `docs/` à part.

## Atelier

- Bambu Lab A1 Mini, PETG HF Black 33102 — plus du PLA pour `screen_wedge` uniquement
- Vis **M3×10** classique (trou pilote 2,6 mm) ; **M2.5×8** pour l'écran dans le wedge
- Helutherm 145 0,75 mm², Ø 2,2 mm pour le 230 V ; 0,25 mm² pour le 5 V
- Aimants 8 × 3 mm, châssis ~40–50 °C, Wago 221 sur rails 2 mm

Sur PETG visible : sécher le filament (le PETG boit l'humidité, ça se voit en stringing) et ne pas compter sur l'ironing.

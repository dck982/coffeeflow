# print/

Pièces FDM de coffeeflow. Vue d'ensemble et schéma électrique : `../README.md`. Firmware à la racine du dépôt.

Trois boîtiers à imprimer autour de la machine :

- **Boîtier AC** — 230 V (dimmer, SSR, alim RECOM), compartiment technique, face ouest
- **Boîtier DC** — XIAO ESP32-S3 + capteurs + CAN, intérieur, zone froide
- **Boîtier UI** — `screen_wedge` + `screen_base`, façade, écran Waveshare 4,3"

nurb sert **ce** dossier (un projet nurb est un répertoire qui contient `parts/`) :

```
cd print
nurb dev          # http://127.0.0.1:7373
nurb export canal
```

**Python :** sur cette machine, préférer **`uv`** (`uv run …`, `uv run --with nurb …`). Le binaire système `python` / `python3` n’est pas fiable ici ; `nurb` lui-même est installé via `uv tool install nurb` et reste sur le PATH.

**Export :** format préféré **3MF** (`printer.toml` → `[export] formats = ["3mf"]`). Bambu Studio peut avertir sur la version 1.41 du 3MF nurb ; l’objet s’importe quand même. STL uniquement si besoin (`nurb export … --formats stl`).

## Les objets

| Pièce | Où | Matière | Statut |
| --- | --- | --- | --- |
| `canal` | intérieur | PETG | actuel |
| **Boîtier AC** | intérieur, baie 70 mm, face ouest | PETG | **à faire** — `parts/ac_box` **deprecated** |
| **Boîtier DC** | intérieur, zone froide (entre PID et cadran, face purge vanne) | PETG | à faire |
| `screen_wedge` (cadre Waveshare, vis M2.5 à l'arrière) | façade | **PLA** | en validation |
| `screen_base` (accueille le wedge) | façade | PETG | existe en berceau de bureau ; **fixation façade à reprendre** |

Traversée intérieur → façade : câble DC (5 V ± CAN) par le trou **Ø 16 mm** de l'ancien bouton brew. Le 230 V reste dans le compartiment technique.

`parts/ac_box` et `docs/ac_box.html` sont **deprecated** (reliquat Atom Control, autre placement). Encombrements machine : `docs/profitec_go.html`. Le boîtier AC reste à dessiner dans la baie de 70 mm, face ouest.

La grille d'entretoise 5 mm sous le plateau chauffant et le logement 230 V dans `screen_base` n'ont plus lieu : l'UI n'est plus posée sur la machine, et plus aucun module secteur n'y vit.

## Matière par pièce

Châssis 40–50 °C dans le compartiment technique. Cycle : 10–15 min de marche, 2× par jour, ambiante le reste du temps. La façade (UI) est hors de cette enceinte.

**PETG par défaut** (`printer.toml`). Le PLA flue lentement sous charge dès ~45 °C. Boîtiers AC / DC et dérivations 230 V : air enfermé, et le AC tient des bornes secteur.

**`screen_wedge` en PLA**, seule exception, pour l'*ironing* de la face supérieure — le PETG ne s'ironise pas. L'écran est en façade et l'alim 5 V est dans le boîtier AC, plus sous le cadre. Si le cadre gondole, retour au PETG.

Détail et chiffres : section « Matières d'impression » de `../README.md`.

## Wago 230 V

Trois paires, tout dans le compartiment technique :

1. L+N machine allumée (relais boiler du PID) → boîtier AC
2. L+N boîtier AC → pompe
3. L+N boîtier AC → vanne solénoïde

Côté machine : FASTON 6,3 × 0,8 mm isolées nylon. Côté mod : Wago. Pas de piggyback.

## Contenu actuel

| Fichier | Rôle |
| --- | --- |
| `parts/canal.py` | Canal de guidage des fils |
| `parts/passe_cable.py` | Passe-câble fileté, deux câbles Ø4.8 |
| `parts/ecrou_passe_cable.py` | Écrou SW22 du passe-câble |
| `parts/ensemble_passe_cable.py` | Assemblage passe-câble + écrou + tôle Ø16 |
| `parts/screen_wedge.py` | Cadre de l'écran Waveshare (PLA) |
| `parts/screen_base.py` | Berceau du wedge (fixation, plus l'électronique 230 V) |
| `parts/screen_assembly.py` | Assemblage wedge + base |
| `parts/base_pesage.py` / `plateau_pesage.py` | Pesée drip tray (HX711, pas encore décidé) |
| `measurements.toml` | Cotes (Wago, aimants, Helutherm, M3) |
| `printer.toml` | A1 Mini, PETG HF 33102 |
| `system.py` | Filet imprimable, puits d'aimant couchés |

Contraintes d'une pièce : sa carte `parts/<nom>.md` (`## Don't`), pas un `docs/` à part.

## Atelier

- Bambu Lab A1 Mini, PETG HF Black 33102 — plus du PLA pour `screen_wedge` uniquement
- Vis **M3×10** classique (trou pilote 2,6 mm) ; **M2.5×8** pour l'écran dans le wedge
- Silicone 0,75 mm² en paires L/N pour le 230 V ; 0,25 mm² pour le 5 V / CAN
- Aimants 8 × 3 mm, châssis ~40–50 °C, Wago 221 sur rails 2 mm

Sur PETG visible : sécher le filament (le PETG boit l'humidité, ça se voit en stringing) et ne pas compter sur l'ironing.

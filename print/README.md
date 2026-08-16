# print/

Pièces FDM de coffeeflow. Vue d'ensemble et schéma électrique : `../README.md`. Firmware à la racine du dépôt.

nurb sert **ce** dossier (un projet nurb est un répertoire qui contient `parts/`) :

```
cd print
nurb dev          # http://127.0.0.1:7373
nurb export boitier_4w --formats stl
nurb export boitier_2w --formats stl
```

Bambu Studio : ouvrir le **STL** dans `build/`, pas le 3MF nurb (version 1.41 non supportée, l'objet s'importe quand même).

## Roadmap des boîtiers

| Pièce | Statut |
| --- | --- |
| Dérivation 230 V, 4 Wago (`boitier_4w` / `couvercle_4w`) | actuel |
| Dérivation 230 V, 2 Wago (`boitier_2w` / `couvercle_2w`) | actuel |
| Dérivation DC (KISOMeter + hub I2C) | à faire |
| Boîtier Atom (ACSSR, Unit Relay, RobotDyn) | à faire |
| Boîtier écran triangulaire 60° | à faire |
| Goulottes 230 V + Grove | si besoin |

Les deux dérivations restent **dans** la machine. Atom + écran **dessus**, dehors. Traversée : 4 fils 230 V + 1 Grove.

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
| `measurements.toml` | Cotes (Wago, aimants, Helutherm, M3) |
| `printer.toml` | A1 Mini, PETG HF 33102 |
| `system.py` | Layout partagé 2w / 4w |

Contraintes d'une pièce : sa carte `parts/<nom>.md` (`## Don't`), pas un `docs/` à part.

## Atelier

- Bambu Lab A1 Mini, PETG HF Black 33102
- Vis **M3×10** classique (trou pilote 2,6 mm)
- Helutherm 145 0,75 mm², Ø 2,2 mm
- Aimants 8 × 3 mm, châssis ~40–50 °C, Wago sur rails 2 mm

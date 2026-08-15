# print/

Pièces FDM de coffeeflow. Vue d'ensemble et schéma électrique : `../README.md`. Firmware à la racine du dépôt.

nurb sert **ce** dossier (un projet nurb est un répertoire qui contient `parts/`) :

```
cd print
nurb dev          # http://127.0.0.1:7373
nurb export boitier --formats stl
```

Bambu Studio : ouvrir le **STL** dans `build/`, pas le 3MF nurb (version 1.41 non supportée, l'objet s'importe quand même).

## Roadmap des boîtiers

| Pièce | Statut |
| --- | --- |
| Dérivation 230 V, **4 Wago 221-423 verticaux** | prochaine pièce |
| Dérivation 230 V, 3 Wago à plat (`boitier` / `couvercle`) | premier essai imprimé, à remplacer |
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
| `parts/boitier.py` | Essai 3 Wago à plat |
| `parts/couvercle.py` | Couvercle M3 de cet essai |
| `parts/ensemble.py` | Assemblage de l'essai |
| `measurements.toml` | Cotes (Wago, aimants, Helutherm, M3) |
| `printer.toml` | A1 Mini, PETG HF 33102 |
| `system.py` | Layout de l'essai 3 Wago |

Contraintes d'une pièce : sa carte `parts/<nom>.md` (`## Don't`), pas un `docs/` à part.

## Atelier

- Bambu Lab A1 Mini, PETG HF Black 33102
- Vis **M3** (trou 2,5 mm validé)
- Helutherm 145 0,75 mm², Ø 2,2 mm
- Aimants 8 × 3 mm, châssis ~40–50 °C, Wago sans contact fond/parois

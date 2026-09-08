@AGENTS.md

## Machine (coffeeflow / print)

- Prefer **`uv`** for any Python on this machine: `uv run …` or `uv run --with nurb …` for one-off probes that import nurb / `system.py`.
- Do not rely on bare `python` / `python3` — they are often missing or wrong here.
- The `nurb` CLI is already on PATH (`uv tool install nurb`). Use `nurb …` for build/check/dev; use `uv run` only when you need a short Python snippet outside the CLI.

## Style des pièces (parts/*.py, system.py)

Une pièce nurb reste avant tout du code Python : les bonnes pratiques Python s'appliquent, pas seulement la doctrine nurb (`nurb rules`). En particulier :

- Pas de copier-coller entre fonctions d'un même fichier ou entre fichiers : une géométrie répétée devient un helper (local au fichier, ou dans `system.py` si plusieurs pièces la partagent — voir `nurb extract`).
- Noms de variables et de fonctions clairs et cohérents dans tout le fichier ; pas d'abréviations qui ne servent qu'à gagner deux caractères.
- Une fonction fait une chose ; découper une fonction de pièce qui grossit plutôt que l'allonger indéfiniment.
- Pas de code mort, de branches commentées « au cas où », ni de paramètres qui ne servent plus (voir le principe general de ne pas garder de code inutilisé).

Objectif : un fichier de pièce doit se lire comme du code qu'on serait content de relire dans six mois, pas comme une suite d'opérations OCCT collées les unes aux autres.

## Orientation (vue utilisateur)

Sauf si l'utilisateur mentionne autrement, il regarde les pièces depuis le dessus, avec X en horizontal et Y en vertical. Il utilise les termes Nord pour les coordonnées hautes en Y, Sud pour les coordonnées basses en Y, Est pour les coordonnées hautes en X et Ouest pour les coordonnées basses en X.

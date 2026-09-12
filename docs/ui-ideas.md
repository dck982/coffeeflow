# Idées UI — rendre l'écran plus vivant

Portée : uniquement la couche LVGL (`firmware/screen/main/ui/`). Aucun
changement de coeur, de CAN ni de layout métrique.

## Constat

`docs/ui.md` décrit neuf jetons de couleur et une échelle de rampe à quatre
niveaux. L'implémentation n'en emploie réellement que quatre : `kBg`,
`kBgRaised`, et les trois gris de texte. Résultat : un écran fait de gris sur
brun très sombre.

Points vérifiables :

- `kRampLow` (`#8C5A22`) et `kRampFull` (`#F2B25C`) sont déclarés dans
  `main/ui/ui_theme.h` et **jamais référencés** ailleurs que dans l'écran de
  test. L'échelle de rampe de la spec (§ « une seule échelle ») n'existe pas.
- Le **filet de progression**, présenté par la spec comme *l'unique* élément
  graphique et la signature visuelle du produit, n'est pas implémenté du tout :
  `hairline()` ne sert qu'à la séparation statique du bandeau (ui_home.cpp:117).
- La valeur héros est peinte en `kTextFaint` au repos (ui_home.cpp:341), soit le
  gris le plus éteint de la palette pour l'élément le plus gros de l'écran.
- Les six boutons sont identiques : fond transparent, bordure `kHairline`. Seul
  « valider » de la confirmation passe `primary = true`.
- L'accent n'apparaît qu'en bordure et sur trois titres de calques modaux, tous
  cachés en usage normal. **L'écran d'accueil ne contient littéralement aucun
  pixel coloré.**

Autrement dit, la spec n'est pas trop monochrome : elle est sous-appliquée.
Les propositions ci-dessous ajoutent de la vie *à spec constante* (groupe A),
puis proposent une extension mesurée de la palette (groupe B).

## Groupe A — appliquer la palette déjà spécifiée

### A1. Réveiller l'échelle de rampe sur les mesures du bandeau

Aujourd'hui pression, température, débit et poids sont colorés par *fraîcheur*
(`kText` / `kTextDim` / `kTextFaint`, ui_home.cpp:507-518). Ajouter une
coloration par *régime*, ce que la spec destine explicitement à la pompe et au
débit :

```cpp
// ui_theme.h
inline lv_color_t ramp_color(float value, float target) {
  if (target <= 0.f || value <= 0.f) return kTextFaint;   // ramp_off
  const float r = value / target;
  if (r < 0.85f) return kRampLow;                          // #8C5A22
  if (r < 1.10f) return kAccent;                           // #D98324
  return kRampFull;                                        // #F2B25C
}
```

Appliqué à la pression (cible = seuil de pré-infusion ou 9 bar), le bandeau
passe du gris au brun ambré puis à l'ambre clair pendant un cycle. C'est de la
couleur *porteuse d'information*, donc conforme à la règle « un seul accent ».
La fraîcheur reste rendue par l'opacité du label plutôt que par sa teinte.

### A2. Implémenter le filet de progression

L'élément signature manquant, et le plus rentable visuellement. Un
`lv_obj_create` de 420 × 2 px à y = 282 (cote donnée par ui.md:535), largeur
animée par `lv_obj_set_width()`, couleur donnée par la phase :

| phase | couleur du filet |
| --- | --- |
| repos | `kHairline`, largeur pleine |
| pré-infusion | `kRampLow`, largeur = avancement du palier |
| infusion | `kAccent`, largeur = poids/temps rapporté à la cible |
| rampe | `kRampFull` |
| purge | `kAccent` en pulsation lente (opacité 60 ↔ 100 %) |

Coût framebuffer : une seule bande de 2 px de haut invalidée, soit très loin
des contraintes de `docs/screen-issue.md`. C'est l'ajout le plus visible pour
le moins de bande passante.

### A3. Colorer la valeur héros selon l'état

`kTextFaint` au repos est un choix par défaut, pas une intention. Proposition :
`kText` au repos (la cible est une information nette), `kAccent` dès qu'un
cycle est actif, `kRampFull` sur la valeur finale à l'état `kFinished`. Le
chiffre de 76/104 px devient alors le centre de gravité chromatique de l'écran.

### A4. Différencier les boutons

Trois niveaux au lieu d'un :

- **primaire** (« infuser ») : bordure `kAccent`, fond `accent_wash` (`kAccent`
  à 14 %, la valeur `accent_wash` déjà spécifiée mais seulement utilisée en
  état pressé), caption `kAccent` ;
- **secondaire** (« purge », « réglages ») : l'actuel, bordure `kHairline` ;
- **destructif** (« réinitialiser réseau ») : bordure et caption `kFault`.

Un seul bouton coloré par écran suffit à casser la platitude, et il guide vers
l'action principale.

### A5. Feuilles modales : une arête d'accent

Les calques diagnostic / réglages / pavé sont des rectangles `kBgRaised` sans
marque. Ajouter un filet `kAccent` de 3 px sur le bord supérieur les rattache
visuellement au reste et signale « surface temporaire » sans texte.

### A6. Diagnostic : pastilles d'état

Les neuf lignes sont uniformément `kTextDim`. Préfixer chacune d'une pastille
de 10 px, `kAccent` si valide, `kTextFaint` si absente, `kFault` si en faute.
Coût : neuf petits objets statiques, recolorés seulement quand la feuille est
visible (la garde `lv_obj_has_flag(..., HIDDEN)` de ui_home.cpp:538 reste).

### A7. Veille moins morte

Le bloc de veille est `kTextDim` sur noir. Le passer en `kRampLow` et faire
respirer son opacité sur le cycle de 120 s déjà présent (ui_home.cpp:296) donne
une machine « chaude et sous tension », exactement ce que ui.md:484 réclame.

## Groupe B — extension assumée de la palette

À ne retenir que si l'on accepte d'amender ui.md. La règle « un seul accent »
existe pour éviter le look tableau de bord ; l'assouplir un peu reste possible
sans y tomber.

### B1. Un second accent froid, réservé à la thermique

`#4A9BB5` (bleu-vert) pour la température **sous consigne** uniquement, avec
transition vers `kAccent` à l'approche de la cible. Un espresso se lit
naturellement en chaud/froid, et le contraste froid/ambre anime le bandeau au
démarrage de la machine — le moment où l'écran est aujourd'hui le plus terne.

### B2. Un fond légèrement moins neutre

`kBg` passe de `#141110` à `#16110C` et `kBgRaised` de `#1E1A18` à `#241B14`.
Décalage minime en RGB565 mais qui réchauffe l'ensemble et fait mieux ressortir
l'ambre. Aucun coût, aucun risque de contraste.

### B3. Dégradé vertical très sombre sur le fond

`lv_obj_set_style_bg_grad_dir(screen, LV_GRAD_DIR_VER, 0)` de `#16110C` vers
`#0E0B09`. Donne de la profondeur. **À mesurer** : un dégradé plein écran est
recalculé à chaque invalidation, ce qui va à l'encontre de
`docs/screen-issue.md`. À n'envisager qu'après mesure du temps de flush, ou à
restreindre au seul bandeau supérieur.

## Ordre d'implémentation suggéré

1. A2 (filet de progression) — le plus gros gain visuel, coût bande passante
   négligeable.
2. A3 + A4 — quelques lignes, l'accueil cesse d'être entièrement gris.
3. A1 + A6 — donne du sens à l'échelle de rampe déjà spécifiée.
4. A5 + A7 — finition.
5. B1 + B2 — après validation sur écran réel, car le rendu RGB565 sur ce
   panneau ne se juge pas depuis `ui_sim`.

B3 en dernier, sous réserve de mesure.

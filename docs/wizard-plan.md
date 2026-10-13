# Plan d'implémentation de l'infusion guidée

Plan de travail, à supprimer une fois le guide livré. Le comportement est
défini dans [wizard.md](wizard.md) : ce plan dit où le coder, dans quel
ordre et comment le vérifier. En cas de doute, `wizard.md` fait foi.

## Règles pour les agents

- Ne rien flasher et ne rien lancer sur la machine (ni `coffeetool`, ni
  HTTP vers l'écran, ni moniteur série). La validation sur la machine est
  faite par David.
- Pas de `cargo fmt` sur `firmware/rust/screen` : mettre en forme à la main
  et vérifier `git diff --stat` avant de conclure.
- Commentaires autonomes : ils expliquent sans renvoyer à `wizard.md` ni à
  ce plan.
- Ne modifier ni les lois de chauffe ou de pompe, ni la configuration, ni
  les conditions actuelles d'infusion.
- Textes de l'écran : en français, en minuscules comme le reste de l'UI. Ni
  « … », ni signe moins typographique, ni guillemets « » dans les chaînes
  affichées. Chaque texte doit s'afficher sans glyphe manquant dans les
  instantanés du simulateur.
- Tailles de police : seulement celles de `SLINT_FONT_SIZES` dans
  `firmware/rust/screen/build.rs` (18, 26, 28, 32, 40, 76, 104). Une autre
  taille s'afficherait avec les mauvais glyphes sur la carte.
- Chaque tâche se termine par ses commandes de vérification au vert et un
  commit. Le relecteur (Opus) relit entre deux tâches.

## Tâche 1 : cœur (`screen-core`)

### Module `guided` (nouveau, `src/guided.rs`)

Machine d'états pure, sans I/O. Elle reçoit l'heure monotone, l'instantané
et la consigne, et rend des demandes d'action au cœur.

- `Screen` : `Rinse1`, `Rinse2`, `Wait`.
- `Hint` : une valeur par consigne des tableaux de `wizard.md`
  (`ConnectScale`, `WaitRinseTemp`, `PlaceCup`, `PressNextRinse1`,
  `ReplaceCup`, `InstallPortafilter`, `CupRemovedEarly`, `WaitSetpoint`,
  `Prepare`, `WaitBrewTemp`, `PressStart`). Le texte est choisi par le
  présentateur, pas par le cœur.
- `View` (`Copy`, dans l'instantané) : `active`, `screen`, `hint`,
  `button_enabled`, `purging`, `purge_elapsed_ms`, `rinse_min_c`
  (consigne − 5), `since_rinse_s`, `refusal: Option<ActionStatus>`.
- Suivi de la tasse : `Present` → `Removed` (poids < −5 g) → tare une fois
  le poids stable → `TaredEmpty` → `Present` (poids > 30 g). « Stable » :
  variation de 0,5 g au plus sur 1 s. Garder une fenêtre courte des
  derniers poids reçus, horodatés. Appliquer la règle d'attente après une
  tare (entre −2 et +2 g, ou 2 s).
- Purge du guide : sur **suivant**, demander `PurgePress`. Si le cœur
  accepte, noter le départ ; à 5,0 s, demander `PurgeRelease`. La fin est
  le passage de `cycle_state` hors de `Purge` (relâchement, `purge.max_s`
  ou refus des capteurs) ; alors seulement, passer à l'écran suivant. Le
  poids de la purge est lu 2 s après cette fin, même si l'écran a déjà
  changé.
- Écran 3 : `setpoint_reached` mémorisé la première fois que la condition
  d'infusion est vraie. Le retrait de la tasse ne compte pour **démarrer**
  qu'après ce moment. Avant, il incrémente `early_removals` et affiche
  `CupRemovedEarly`.
- Abandon : sur `guided_quit`, verrou, flash, provisioning, capteurs
  perdus, ou 30 min sans appui (`note_touch_press` du cœur). Un abandon
  pendant une purge demande d'abord `PurgeRelease`.
- `Session` : tous les champs de l'export de `wizard.md`, instants en µs
  monotones. Purges dans un tableau fixe de 2 (pas d'allocation).

### Condition d'infusion partagée

La condition de `set_brew_enabled` (`screen/src/ui/presenter.rs`, fonction
`content`) devient une fonction de `screen-core`, par exemple
`snapshot::brew_ready(&Snapshot, &Config) -> bool`, à l'identique. Elle
est utilisée par le bouton **directe**, par **démarrer** et par la
consigne « consigne atteinte » du guide. Ne pas toucher aux contrôles de
`perform_action(StartBrew)`.

### Intégration dans `core.rs`

- `Action::GuidedOpen`, `Action::GuidedNext`, `Action::GuidedQuit` :
  refusés si la source est HTTP (comme `StartBrew`). Ne pas les ajouter au
  tableau des noms d'action de `http.rs`.
- `GuidedNext` sur les écrans de rinçage passe par le même chemin que
  `PurgePress`. Sur l'écran 3, il envoie `Tare` puis le chemin de
  `StartBrew`. Le refus éventuel est rendu dans `View::refusal`.
- `tick` fait avancer le guide (purge de 5 s, tare, abandon).
  `update_scale_weight` lui transmet chaque poids horodaté.
- Au départ d'une infusion guidée, la session est figée et attachée à la
  capture (`begin_hf_capture(Origin::Brew, …)`) ; le guide se termine.
  Toute autre capture qui commence efface la session attachée.
- L'instantané porte `guided: guided::View`.

### Export `/hf-capture`

`hf_capture::export_header` reçoit la session. Pour l'origine `brew`, il
écrit `"session":{…}` ou `"session":null` juste avant `"samples":[`. Pour
les autres origines, il n'écrit pas la clé. Instants exportés :
`(instant − started_at_us) / 1000`, arrondis à l'entier. Les valeurs
inconnues s'écrivent `null`. Mettre à jour les documents dorés de
`tests/golden_json.rs` si l'en-tête y est vérifié.

### Tests (`tests/guided.rs`, sur le banc `core_logic.rs`)

Un test par ligne des tableaux « Écrans » et par cas tranché de
`wizard.md`, au minimum :

- **suivant** inactif sans balance, avec une tasse à 25 g, sous
  consigne − 5 °C ; actif à 31 g et à consigne − 5 °C ;
- écran 1 : la tare part avant la purge ; `purge_release` part à 5,0 s ;
  l'écran 2 n'arrive qu'à la fin réelle de la purge ;
- écran 2 : retrait (−120 g), tare après stabilité, pas de seconde tare
  pendant l'attente, replacement à 31 g, **suivant** réactivé ; pas de
  tare avant la purge 2 ;
- écran 3 : retrait avant la consigne (`early_removals` = 1, **démarrer**
  inactif après replacement) ; consigne atteinte puis NTC qui sort de la
  plage : `setpoint_reached_ms` gardé, **démarrer** inactif jusqu'au
  retour ; retrait puis replacement après la consigne : **démarrer**
  actif ;
- **démarrer** : tare puis infusion ; session attachée ; instants négatifs
  cohérents ; `start_pressed_ms` vaut 0 ou presque ;
- abandons : quitter pendant une purge (relâchement envoyé), verrou,
  30 min sans appui ;
- export : `session` présent pour une infusion guidée, `null` pour une
  directe, absent pour une purge ; une purge après l'infusion efface la
  session ;
- HTTP : les trois actions refusées.

Vérification :

```sh
cd firmware/rust
cargo +stable test -p screen-core
cargo +stable clippy -p screen-core --all-targets -- -D warnings
```

## Tâche 2 : écran (Slint, présentateur, simulateur)

### Présentateur (`screen/src/ui/presenter.rs`)

- Bouton **infuser** de l'accueil : actif selon les conditions d'une purge
  (capteurs vivants, pas de verrou, dimmer prêt et valide, pas de cycle,
  pas de `capture_cooldown`). Il ouvre la fenêtre de choix au lieu
  d'envoyer `StartBrew`.
- Fenêtre de choix : superposition, sur le modèle de `confirm_visible`.
  **directe** envoie `StartBrew` ; il est grisé si `brew_ready` est faux,
  avec la raison actuelle dessous. **guidée** envoie `GuidedOpen`.
  **annuler** ferme la fenêtre.
- Nouvelle page 8 « guidée ». Dans `update`, `s.guided.active` force la
  page 8 **avant** la règle `cycle_state → page` : une purge du guide ne
  doit pas afficher la page de purge. Une fois l'infusion lancée, le guide
  est inactif et la page d'infusion habituelle s'affiche.
- Nouveaux identifiants de commande à partir de 21 (choix, suivant,
  quitter). **Quitter**/**arrêter** du guide doit passer le filtre
  « cycle actif » de `command`, comme 9 et 20.
- Textes : une fonction `hint_text(Hint, &Snapshot, &Config) -> String`.
  La température cible s'affiche avec la consigne réelle (« attendez
  87 °C »).
- Rafraîchissement : la page 8 fait partie du groupe « contenu ».

### Page Slint (`screen/ui/app.slint`)

Disposition 800 × 480 :

- bandeau d'état habituel en haut, avec le même code que l'accueil ;
- à gauche, une illustration de 200 × 200 px, et sous elle les trois
  étapes (1, 2, 3) en pastilles : faite en vert, en cours en orange, à
  venir en gris ;
- à droite, le titre « infusion guidée » en 26 px `dim`, la consigne en
  32 px `text` sur deux lignes au plus, puis la valeur utile en 76 px
  (température pendant une attente, compteur de purge, poids sinon) ;
- en bas, **quitter** à gauche (devient **arrêter** pendant une purge) et
  le bouton d'action à droite (**suivant**, **démarrer**), 88 px de haut,
  le bouton d'action en `primary`. Une raison de refus s'affiche en
  `fault` au-dessus.

### Illustrations (`screen/ui/icons/guided-*.svg`)

Dessins au trait dans le style des icônes existantes : `viewBox` de 200,
`stroke="white"`, `fill="none"`, trait de 4, coloriés par `colorize` (en
`dim`, la pièce concernée par la consigne en `accent` quand c'est
possible, sinon tout en `dim`). Quatre dessins :

| Fichier | Écran | Sujet |
| --- | --- | --- |
| `guided-rinse.svg` | 1 | tête de groupe, filet d'eau, tasse sur la balance, sans porte-filtre |
| `guided-portafilter.svg` | 2 | même vue, porte-filtre en place au-dessus de la tasse |
| `guided-wait.svg` | 3, avant la consigne | même vue avec un thermomètre |
| `guided-prepare.svg` | 3, après la consigne | porte-filtre posé avec une galette, tasse vide à côté |

Elles sont embarquées pré-rendues (`EmbedForSoftwareRenderer`) ; noter
l'écart de taille du binaire en tâche 4.

### Simulateur (`screen-sim/src/main.rs`)

Ajouter des scénarios à `SCENARIOS` : `choice`, `choice-heating`
(**directe** grisé), `guided-connect`, `guided-rinse1-temp`,
`guided-rinse1-ready`, `guided-purge`, `guided-rinse2-replace`,
`guided-wait`, `guided-early-removal`, `guided-prepare`,
`guided-start`. Ils se construisent en remplissant `guided::View` dans
l'instantané de démonstration, comme les scénarios existants.

Vérification :

```sh
cargo +stable test --manifest-path firmware/rust/screen-sim/Cargo.toml
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml -- --snapshots /tmp/coffeeflow-ui
cd firmware/rust/screen && cargo clippy --release --lib --bin screen --offline -- -D warnings
```

Le relecteur regarde les PNG de `/tmp/coffeeflow-ui/{choice,guided-*}.png`.

## Tâche 3 : analyse (`firmware/analysis/describe_hf_capture.py`)

- Lire `session` si elle existe. Sous les cartes de résumé du rapport HTML,
  ajouter un bloc « infusion guidée » avec les deux rinçages (durée,
  volume, poids, NTC départ → minimum → arrêt), puis les trois durées de
  `wizard.md` en secondes : retour à la consigne, porte-filtre en place
  après la consigne, préparation. Rien si `session` est absent ou `null`.
- Test dans `firmware/analysis/test/` avec une capture synthétique portant
  une session, plus une sans session.

```sh
uv run --with pytest --with numpy --with scipy python -m pytest firmware/analysis/test -q
```

## Tâche 4 : documentation, version, image

- `docs/ui.md` : fenêtre de choix et page guidée (sections « Accueil et
  repos » et nouvelle section « Infusion guidée » qui renvoie à
  `wizard.md` pour le déroulé).
- `docs/firmware.md` : clé `session` de `GET /hf-capture` (paragraphe
  `/hf-capture`), avec renvoi à `wizard.md`.
- Version : passer à 0.4.9 selon la procédure habituelle
  (`FIRMWARE_VERSION` dans `protocol/src/version.rs`, `Cargo.toml` et
  `Cargo.lock` de screen, screen-net, screen-sim, sensors, sensors-logic).
- Construire et produire `/tmp/screen-0.4.9.bin` et `/tmp/sensors-0.4.9.bin`
  (et `.elf`) comme dans `docs/firmware-build.md`. Noter la taille de
  l'image écran avant et après les illustrations. **Ne pas flasher.**
- Supprimer ce plan.

## Validation sur la machine (David)

1. Accueil pendant la chauffe : **infuser** actif, **directe** grisé avec
   la raison, **guidée** actif.
2. Écran 1 sans tasse, puis avec : **suivant** s'active au-dessus de 30 g
   et à consigne − 5 °C ; tare, puis rinçage de 5 s.
3. Écran 2 : retirer la tasse, la balance revient à 0, « replacez la
   tasse » ; replacer, rinçage du porte-filtre.
4. Écran 3 : attente jusqu'à la consigne ; vider, rincer, préparer ;
   **démarrer** actif au retour de la tasse vide ; infusion normale.
5. `coffeetool download-hf` : la capture porte `session`, et le rapport
   HTML affiche le bloc « infusion guidée ».

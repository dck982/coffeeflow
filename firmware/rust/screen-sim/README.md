# Simulateur écran Rust

Depuis la racine du dépôt :

```sh
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml -- --snapshots /tmp/coffeeflow-ui
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml -- --snapshot provisioning /tmp/coffeeflow-ui
cargo +stable test --manifest-path firmware/rust/screen-sim/Cargo.toml
cargo +stable clippy --manifest-path firmware/rust/screen-sim/Cargo.toml --all-targets -- -D warnings
```

Sans argument, une fenêtre de 800 × 480 utilise le vrai `screen-core`. Un faux
module interne publie les STATUS, répond aux commandes CAN et fournit les
mesures ADS et balance. Un appui est traité par la même interface `Input` que
sur la carte ; les refus de sécurité restent ceux du cœur. Ce banc illustre
l'interface et les transitions, pas la physique thermique/hydraulique réelle.

Les captures n'ouvrent pas de fenêtre : elles utilisent le renderer logiciel,
le même fichier Slint, les polices embarquées et des pixels RGB565, reconvertis
en PNG. `--list` énumère les 24 scénarios : accueil avec et sans balance,
infusion, résumé, purge, quatre pages de réglages, diagnostic, deux pavés,
choix de pré-infusion/rampe, maintenance et plein écran.

La navigation et le présentateur sont partagés dans
`screen/src/ui/presenter.rs`. `Presenter::command` retourne un `UiRequest` :
l'assemblage transmet `Core(Input)` au cœur et renvoie le `Reply` à `feedback` ;
`RestartLcd` reste une demande locale au pilote d'affichage. `update` reçoit un
instantané cohérent, la configuration, le SSID de provisioning et l'heure
locale formatée par le service réseau. Aucune commande CAN n'est produite par
l'UI. Les quatre groupes de valeurs sont espacés de 62,5 ms ; un appel ne
rafraîchit qu'un groupe, sans rafale de rattrapage. Les priorités de sécurité
et le changement de destination sont mis à jour avant chaque rendu.

Le code archivé prime sur `docs/ui.md` : ses quatre petits onglets numérotés,
les tuiles d'infusion, la frise et ses teintes (dont rouge pré-chauffe et vert
prêt/choix de pré-infusion) sont conservés. La page « calibrations » reste une
indication « depuis /config », comme en C++, et non un nouvel assistant tactile.
Le changement radio du plan remplace la destination modale Wi-Fi par une
bascule de session et un écran de provisioning sans action d'infusion.

Les snapshots vérifient le rendu et ses dimensions, mais ne constituent pas
une comparaison pixel par pixel avec LVGL : les métriques et l'anticrénelage
Slint diffèrent. Les tests sur la dalle et le contrôle tactile réel restent
une validation matérielle séparée.

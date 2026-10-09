# Interface de l'écran

L'écran tactile Waveshare 800 × 480 est en façade de la machine. L'interface
est écrite en Rust avec Slint : les écrans sont décrits dans
`firmware/rust/screen/ui/app.slint`, les couleurs dans `theme.slint`, et la
navigation, les textes et les rafraîchissements dans
`firmware/rust/screen/src/ui/presenter.rs`. L'UI LVGL d'origine est archivée
dans `archive/firmware/screen` ; l'UI Slint en reprend les écrans, les textes
et les couleurs.

## Contraintes d'usage

- L'écran est lu à distance et parfois les doigts mouillés : gros caractères,
  grosses cibles tactiles (boutons d'action de 80 à 88 px de haut), peu de
  texte par écran.
- Fond brun très sombre, texte crème ; l'orange (`accent`) désigne l'action ou
  la phase en cours.
- L'UI ne commande jamais le matériel directement. Un geste devient une
  demande (`UiRequest`) que l'assemblage transmet au cœur ; le cœur reste seul
  juge des refus, des actionneurs et de la persistance. L'UI n'émet aucune
  commande CAN.
- Pendant un cycle (infusion ou purge), seuls « arrêter » et le relâchement de
  la purge sont acceptés. Les écrans de sécurité, de mise à jour et de
  provisioning ignorent les autres gestes.

## Couleurs de rôle

| Rôle | Usage |
|---|---|
| `bg`, `raised`, `surface`, `high` | fond, bandeaux et tuiles, boutons, bouton appuyé |
| `text`, `dim`, `faint` | texte principal, secondaire, discret |
| `accent` (orange) | action principale, phase en cours, titres de menus |
| `primary` (brun chaud) | fond du bouton principal |
| `fault` (rouge) | avertissements, défauts, action destructive |
| `success` (vert) | prêt, état valide, choix actif |
| `thermal` (bleu) | chauffe en cours, remplissage |
| `near` (vert-gris) | chaudière proche de la consigne |
| `ambient` (violet) | chaudière froide (moins de 50 °C) |
| `low`, `full` | brun ocre (montée, nom « coffeeflow » en veille), jaune des valeurs clés |

## Bandeau d'état

Présent sur l'accueil, l'infusion et la purge. De gauche à droite : icône
« diagnostic » (zone tactile qui ouvre l'écran de diagnostic), heure locale
(vide tant que le SNTP n'a pas synchronisé ; CET/CEST calculé dans l'UI), version
de l'écran (`v0.4.02`), poids de la balance (vide si elle est absente),
pression (`-` si invalide) et température de chaudière. Sous la température, un
filet de 4 px indique la puissance de chauffe.

La couleur de la température suit l'état de la chaudière :

- gris : mesure invalide ; semi-transparent si elle est périmée ;
- violet : chaudière froide (sous 50 °C), c'est alors la sonde qui est affichée ;
- rouge : plus d'un degré au-dessus de la consigne d'infusion ;
- vert : prête à infuser ;
- vert-gris : à moins de 5 °C sous la consigne ;
- bleu : encore en chauffe.

## Accueil et repos

L'accueil est la page 0. Au centre, la cible en grand : le poids (`36,0 g`) si
la balance est présente, sinon le temps (`28 s`). Appuyer sur la cible ouvre le
pavé numérique ; « − » et « + » l'ajustent directement (0,5 g entre 10 et
100 g, ou 1 s entre 5 et 60 s) et enregistrent la configuration. Une ligne
discrète rappelle la cible et la pré-infusion, ou « balance absente ». Une
ligne rouge dit pourquoi l'infusion est refusée : erreur dimmer, dimmer en
calibration, chauffage désactivé, chaudière en chauffe.

Trois boutons en bas :

- **infuser · 36 g** (ou `· 28 s`), principal, grisé tant que les conditions
  ne sont pas réunies (capteurs vivants, pas de verrou, dimmer prêt, chauffage
  activé, températures et puissance de chauffe fraîches, chaudière à moins
  d'un degré de la consigne et prête, pas de délai de capture) ;
- **purge**, qui pompe tant que le doigt reste appuyé ;
- **réglages**.

La veille a deux niveaux, réglés par `dim_after_s` et `standby_after_s` de la
configuration : l'écran s'assombrit, puis passe sur un fond uni portant
« coffeeflow / au repos », décalé lentement pour ne pas marquer la dalle. Elle
ne s'applique ni pendant un cycle, ni pendant une mise à jour ou un provisioning.
Tout appui, même hors bouton, réveille l'écran.

## Infusion et résumé

La page d'infusion s'affiche dès qu'un cycle est actif, et reste après la fin
tant que le résumé n'est pas fermé. Elle comporte, de haut en bas :

1. la phase (`pré-chauffe`, `remplissage`, `pré-infusion`, `rampe`,
   `infusion`, `dernières gouttes`, `écoulement`, `terminé`) et, à droite, la
   date et l'heure de départ ;
2. la **frise** (voir ci-dessous) ;
3. six tuiles : poids en tasse, temps d'infusion, moyenne en tasse (avec la
   cible de température), temps total (avec le temps de pompe), débit
   d'infusion (g/s et ml/s) et baisse thermique (avec le min-max) ;
4. pendant le cycle, quatre mesures en direct (chauffe, pompe, débit balance,
   débit pompe) et le bouton **arrêter** ; une fois terminé, un seul bouton
   **fermer**, qui affiche un compte à rebours (`fermer · 3 s`) et reste grisé
   tant que le délai de capture court.

### Frise

La frise est une bande horizontale de 736 px dont l'axe est le temps du tir.
Chaque phase réellement parcourue y occupe un segment proportionnel à sa durée,
avec son nom en majuscules s'il y a la place et sa durée en secondes dessous :

| Segment | Teinte |
|---|---|
| pré-chauffe | rouge |
| remplissage | bleu |
| pré-infusion | brun foncé |
| montée | brun ocre |
| infusion | orange, nom en sombre |
| gouttes | gris ; la durée est remplacée par le gain d'égouttage (`+1,2 g`) |

Le segment dont la fin correspond à l'instant courant est cerclé de clair : c'est
la phase en cours. Tant que la pompe tourne, un cadre gris à droite réserve la
place de l'égouttage à venir. Les phases non traversées n'ont pas de segment.

## Purge

Page distincte : la phase (`purge`) en orange, un grand compteur de secondes,
un filet de progression qui avance jusqu'à la durée maximale de purge, puis le
débit (ml/s) et la pression. Le bouton « arrêter » ferme la purge. Lancée au
doigt depuis l'accueil, elle s'arrête au relâchement.

Cette page sert aussi à un cycle d'infusion sans suivi de tir (`shot.active`
faux). Si le cycle vise un poids, le compteur se scinde alors en deux colonnes,
poids écoulé et temps écoulé ; une fois terminé, il affiche le dernier poids.

## Réglages

La page « réglages » (page 3) a un en-tête avec retour, une bascule Wi-Fi
(session : elle active ou coupe la radio, sans changer d'écran) et quatre
onglets numérotés. Chaque page affiche six tuiles en deux colonnes ; une tuile
numérique ouvre le pavé, une tuile de choix ouvre l'écran de choix.

1. **Cibles et pompes** : cible temps, pompe remplissage, cible poids, pompe
   pré-inf., cible pression, pompe infusion.
2. **Remplissage et pré-infusion** : pression remplissage, durée remplissage,
   critères pré-inf. (choix), échéance pré-inf., cible chaudière.
3. **Rampe et purge** : stratégie rampe (choix), avance temps, avance poids,
   chute pression, pompe purge, purge max.
4. **Réseau et maintenance** : réinitialiser réseau (demande confirmation),
   calibrations (indication « depuis /config », non modifiable à l'écran),
   réinitialiser LCD (redémarre l'affichage), veille (« automatique », non
   modifiable), précharge chauffe.

Les tuiles inactives (calibrations, veille) sont grisées. « Réinitialiser réseau »
apparaît en rouge et ouvre une fenêtre « effacer les identifiants wifi
enregistrés ? » avec annuler et valider.

### Pavé numérique et choix

Le pavé occupe la moitié droite : chiffres, effacement et virgule (grisée pour
les valeurs entières). La valeur courante, en grand à gauche, est remplacée dès
la première touche. « valider » envoie la configuration au cœur ; une valeur
invalide ou hors limites, ou un refus du cœur (par exemple pendant un cycle),
s'affiche en rouge sous la valeur.

Les deux écrans de choix sont des grands boutons : critères de pré-infusion
(temps, poids, cumulables ; actifs en vert) et stratégie de rampe (aucune,
temps, poids, chute pression ; un seul choix, en orange).

## Diagnostic

On y accède par l'icône du bandeau. Neuf tuiles en grille 3 × 3, chacune avec
une pastille d'état (vert valide, orange périmé ou en calibration, rouge
absent ou en erreur, gris désactivé), un nom, une valeur et un détail :

| Tuile | Valeur | Détail |
|---|---|---|
| pression | bar | valide, périmé, absent |
| chaudière | température | valide, périmé, absent |
| débit | ml/s | valide, périmé, absent |
| pompe | puissance en % | valide, calibration, erreur, absent |
| vanne | ouverte / fermée | valide, périmé, absent, maintenance |
| chauffage | puissance acceptée, ou OFF | valide, périmé, absent, désactivé |
| bus can | OK / ERREUR | nombre d'erreurs RX, TX et bus cumulées (`1,2 K erreurs`) |
| balance | poids | présente / absente |
| versions | version de l'écran | `screen`, et à droite la version `sensors` |

Une ligne au bas de l'écran donne la SRAM libre et le plus gros bloc libre.

Les tuiles pompe, vanne et chauffage sont tactiles et ouvrent un petit menu à
deux boutons plus « fermer » :

- **dimmerlink** : reset (en rouge) ou calibrer ; la pompe est arrêtée avant ;
- **vanne · maintenance** : ouvrir 30 s (ou relancer) ou fermer la vanne ;
  l'ouverture est refusée tant que le chauffage est activé ou si le module
  capteurs est trop ancien, et le texte indique le temps restant ;
- **chauffage** : activer ou désactiver.

## Plein écrans

Ils recouvrent tout, par ordre de priorité :

1. **verrou de sécurité** (rouge) : pompe restée en marche plus de 60 s, couper
   la machine à l'interrupteur ;
2. **mise à jour** : cible (écran, capteurs, firmware), octets reçus sur total,
   consigne de ne pas couper la machine, filet de progression ;
3. **module interne injoignable** (rouge) : capteurs muets, pompe et vanne
   coupées ;
4. **coffeeflow** : écran de démarrage pendant les deux premières secondes ;
5. **configuration wifi** : le provisioning demande de se connecter au réseau
   dont le nom est affiché, puis de saisir ses identifiants.

Les plein écrans de sécurité ramènent la navigation à l'accueil, sauf si un
cycle est actif.

## Rafraîchissement

Les priorités (page, plein écran, veille) sont mises à jour avant chaque
rendu. Les valeurs sont réparties en quatre groupes : bandeau d'état, contenu
des écrans (accueil, infusion, purge, frise), diagnostic, réglages. Chaque
groupe revient à 4 Hz, espacé de 62,5 ms des autres ; un appel ne rafraîchit
qu'un groupe, sans rafale de rattrapage si le rendu est en retard. Un geste
force un nouveau tour.

## Simulateur

Le simulateur `firmware/rust/screen-sim` fait tourner le même Slint et le même
présentateur sur le poste, avec le vrai cœur de l'écran et un faux module
interne. Depuis la racine du dépôt :

```sh
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml -- --snapshots /tmp/coffeeflow-ui
cargo +stable run --manifest-path firmware/rust/screen-sim/Cargo.toml -- --snapshot provisioning /tmp/coffeeflow-ui
cargo +stable test --manifest-path firmware/rust/screen-sim/Cargo.toml
```

Sans argument, une fenêtre de 800 × 480 s'ouvre. `--snapshots` écrit en PNG les
24 scénarios (accueil, infusion, résumé, purge, réglages, diagnostic, pavés,
choix, maintenance, plein écrans) avec le renderer logiciel, sans fenêtre.
`--list` les énumère. Le banc illustre l'interface et ses transitions, pas la
physique thermique ou hydraulique.

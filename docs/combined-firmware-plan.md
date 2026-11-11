# Firmware : passage de sensors à combined

Plan de travail pour faire tourner le firmware Rust `sensors` sur la carte
`combined` avec la pompe à engrenage MPB003, et adapter l'écran. À supprimer
une fois le passage fait.

Références : `pcb/combined/combined_design.md` (brochage, interface pompe,
table de consigne mesurée), `docs/pompe-engrenage.md` (pompe, tachymètre,
défauts, débitmètre), `docs/flowcontrol.md` (régulation en débit, chantier
suivant, hors de ce plan).

## Ce qui ne change pas

Vérifié au banc le 11 novembre 2026 sur la carte combined, XIAO en
MicroPython :

| Fonction | GPIO / adresse | Combined |
| --- | --- | --- |
| Vanne (SSR) | GPIO 9, actif HIGH | D1 et ampoule 230 V suivent la commande |
| Chaudière (HEATER_CMD) | GPIO 4, actif HIGH | J8 : 0 V / 5,16 V |
| LED d'état | GPIO 2 | D4 |
| CAN | TX GPIO 7, RX GPIO 8 | bouclage par le TJA1051, 120 Ω (122 Ω mesurés) |
| I2C | SDA GPIO 5, SCL GPIO 6, pull-ups 4,7 kΩ de la carte | TMP102 0x48, ADS1115 0x49 |
| Pression | ADS1115 AIN1, J10 | 0 V et 3,285 V lus aux deux extrêmes |
| Débitmètre | GPIO 44, front descendant, pull-up 1 kΩ et 10 nF de la carte | impulsions comptées |

Le code de ces fonctions reste tel quel dans `firmware/rust/sensors` et
`sensors-logic`.

## 1. Retirer le DimmerLink de sensors

- `sensors-logic/src/machine.rs` : supprimer les registres, la relecture de
  santé (`DIMMER_HEALTH_PERIOD_US`), le recalibrage automatique et
  `apply_dimmer`. `zero_guard.rs` disparaît : une sortie PWM locale ne peut
  pas perdre une écriture, la sortie est remise à 0 directement.
- Trait `Hw` : `dimmer_write` / `dimmer_read` sont remplacés par
  `set_pump_duty(u16)` et `pump_tach() -> TachCounter` (section 2).
- `sensors/src/main.rs`, `boot_fault` : ne plus interroger 0x50. La variante
  `led::BootFault::Dimmer` (clignotement à 1 Hz) disparaît ; la LED garde le
  clignotement à 4 Hz si le TMP102 ou l'ADS1115 ne répondent pas.
- `DIMMER_COMMAND` (0x03) reçu : ignoré, un LOG Warn au premier reçu. Le
  type reste réservé dans `protocol` ; l'écran cesse de l'émettre (section 4).
- **Drapeaux de `STATUS_ACTUATORS` : mêmes bits, nouveau sens.** Le bit 1
  devient « pompe prête » (pas de défaut en cours) et le bit 2 « pilote de
  pompe présent », toujours à 1 : une entrée GPIO ne détecte pas une pompe à
  l'arrêt. Le bit 3 devient « défaut pompe ». `brew_ready` de l'écran reste
  vrai sans modification, donc les deux firmwares peuvent être flashés dans
  n'importe quel ordre. Renommer les constantes (`FLAG_PUMP_READY`,
  `FLAG_PUMP_PRESENT`, `FLAG_PUMP_FAULT`) et les commentaires du protocole.
- Tests de `machine_tests.rs` : les tests du dimmer sont remplacés par ceux
  de la pompe ; le faux `Hw` enregistre le rapport cyclique au lieu des
  écritures I2C.

## 2. Commande et tachymètre de la pompe

### Consigne, GPIO 1

- **Au démarrage**, GPIO 1 est posé bas avec la vanne et la chauffe, avant
  le CAN (R34 le tient déjà bas pendant le reset). Il passe ensuite au LEDC.
- **LEDC à 1 kHz, 14 bits.** Le LEDC de l'ESP32-S3 s'arrête à 14 bits ; les
  « 16 bits » de `combined_design.md` et `pompe-engrenage.md` sont à
  corriger. Un pas vaut 0,006 % ; 1 V de consigne (≈ 1 %) reste à ≈ 160 pas.
- **`pump_pct` garde son sens de SET : un pourcentage de la vitesse
  maximale**, 100 % = 4 500 tr/min, linéaire en vitesse. Le protocole et les
  machines d'état de l'écran ne changent pas.
- **Chaîne de conversion** (dans `sensors-logic`, testée sur le Mac) :
  `pump_pct` → vitesse visée (× 45 tr/min) → consigne en volts
  (1 000 tr/min par volt, mesuré sur la pompe) → rapport cyclique par
  interpolation linéaire dans la table mesurée sur la carte, exprimée en
  fraction de Vmax. Vmax à la pompe vaut ≈ 4,83 V (5,09 V au banc,
  × 4,98 / 5,16 pour le jaune, × 0,983 pour les 176 kΩ de la pompe) :
  constante à recaler à la première mise en route (section 6).
- 0 % donne un rapport cyclique de 0 exact : la consigne tombe à 0 V par R35.
- Toutes les coupures existantes (bail SET, verrou de 60 s, perte de
  présence, STOP, RESET) passent par le même chemin et posent 0.

### Tachymètre, GPIO 3

- **Interruption GPIO datée**, comme le débitmètre (`flow.rs`) : au plus
  167 Hz, la charge est négligeable, et l'horodatage donne la période. Le
  gestionnaire d'interruption GPIO est unique (`Io::set_interrupt_handler`) :
  il traite GPIO 44 et GPIO 3.
- Front descendant de GPIO 3 (signal inversé par U9), pas de pull-up interne
  (R39 de 4,7 kΩ sur la carte).
- **Filtre** : un front moins de 3 ms après le précédent est ignoré
  (5 000 tr/min donnent 6 ms entre impulsions).
- **Vitesse** = 30 000 000 / période en µs, sur les deux dernières périodes
  (un tour) pour moyenner l'écart entre les deux impulsions du tour. Vitesse
  nulle si aucun front depuis 200 ms (sous 150 tr/min).
- `TachCounter` dans `sensors-logic` (comme `FlowCounter`) : compteur
  d'impulsions cumulé, dernier front, période ; la logique est testée sur le
  Mac.

### Défauts de la pompe

Règle de la notice de la famille (`pompe-engrenage.md`, Protections
internes) :

- **Détection** : consigne au-dessus de 0,3 V (≈ 7 %) depuis plus de
  500 ms, et aucun front de tachymètre depuis 300 ms.
- **Réaction** : rapport cyclique à 0 pendant 50 ms au moins (la pompe se
  réarme à 0 V pendant 10 ms), LOG `PumpFault` (nouveau code), bit « défaut
  pompe » levé. Pas de redémarrage automatique pendant le cycle : le défaut
  se lève au prochain SET qui part de 0.
- Démarrage à froid : la pompe ignore une consigne dans les 65 ms qui
  suivent sa mise sous tension. Le XIAO démarre bien après (buck à 18 V, puis
  démarrage) : aucune action.

### Marche à sec

Règle par défaut de `pompe-engrenage.md` (Marche à sec) : vitesse au-dessus
de 1 000 tr/min, aucune impulsion Digmesa pendant 2 s et pression qui ne
monte pas → consigne à 0, LOG `PumpDry` (nouveau code), même bit de défaut.
La règle est toujours active ; si les seuils déclenchent à tort, ils se
corrigent par un nouveau firmware.

## 3. Télémétrie du tachymètre

Même partage que le débitmètre et la sonde de pression : sensors envoie
la mesure brute, l'écran la convertit avec ses constantes de
`screen-core/src/calibration.rs`, et `/telemetry` expose la valeur brute et
la valeur convertie. Celui qui lit la télémétrie n'a besoin d'aucune
constante.

- Sensors calcule seulement la vitesse, parce qu'elle demande l'horodatage
  des fronts, que l'écran ne voit pas ; il connaît les 2 impulsions par tour
  pour ses propres règles (consigne en tr/min, défauts, marche à sec).
- L'écran ajoute `PUMP_TACH_PULSES_PER_TURN = 2` et `PUMP_ML_PER_TURN = 0.1`
  à `calibration.rs`, à côté de `FLOW_PULSES_PER_LITER`.
- « Refoulé » veut dire tours × cylindrée : le glissement n'est pas retiré,
  ce n'est pas le débit net. L'écart avec le Digmesa donne le glissement.

### Nouveau message CAN `STATUS_PUMP` (0x25), capteurs → écran

Même forme que `STATUS_FLOW` :

| Octets | Champ | Unité |
| --- | --- | --- |
| 0–3 | `tach_pulses` | impulsions cumulées depuis le démarrage |
| 4–5 | `rpm` | tr/min, sur le dernier tour ; 0 sans front depuis 200 ms |
| 6 | `flags` | bit 0 tachymètre vu depuis le démarrage, bit 1 défaut, bit 2 marche à sec |

DLC 7. Publication périodique par `REQ_STATUS` (cible 0x25), comme
`STATUS_FLOW` : plancher de 50 ms. Un écran ancien ignore le type inconnu.
Le rapport cyclique appliqué n'est pas publié : il se déduit de `pump_pct`
par la table de sensors, et part dans le LOG de démarrage de chaque cycle.

- `protocol` : `MessageType::StatusPump`, `StatusPumpPayload` (pack /
  unpack), vecteurs de `protocol/tests/golden.rs`, nouveaux `LogCode`.
- `sensors-logic` : `pump_period_ms`, envoi dans `tick_safety` comme les
  autres statuts ; `on_reqstatus` accepte 0x25.
- `coffeetool monitor` décode le nouveau message.

### Écran

- `screen-core` : demande `STATUS_PUMP` à 100 ms en profil actif, 1 000 ms
  au repos (mêmes profils que `STATUS_FLOW`). Le snapshot gagne
  `pump_tach_pulses`, `pump_rpm`, `pump_fault`, `pump_dry`, et les valeurs
  converties `pump_displaced_ml` (impulsions / 2 × 0,1 ml) et
  `pump_displaced_ml_s` (tr/min × 0,1 / 60).
- **`/telemetry`** : objet `pump`, avec `pct`, `rpm`, `displaced_ml_s`,
  `displaced_ml`, `tach_pulses` (brut, comme `flow.pulse_count`), `ready`,
  `fault`, `dry`. Mettre à jour `tests/golden_json.rs` et les vecteurs
  cJSON.
- **`/hf-capture`** : `pump_rpm` et `pump_displaced_ml` dans chaque
  échantillon, et `firmware/analysis/describe_hf_capture.py` les affiche
  (vitesse et débit refoulé au plateau, écart avec le Digmesa, qui donne le
  glissement). Recommandé
  pour les premiers essais ; c'est la seule façon de relire la vitesse
  pendant une infusion.

## 4. Écran : le reste

- Retirer les actions `ResetDimmer` / `RecalibrateDimmer` (UI et HTTP) et
  l'émission de `DIMMER_COMMAND`.
- Renommer les champs `dimmer_*` du snapshot en `pump_*`, **clés JSON
  comprises** : `dimmer_pct`, `dimmer_ready`, `dimmer_valid` et
  `dimmer_error_active` disparaissent au profit de l'objet `pump`
  (`pct`, `ready`, `fault`). Le statut `dimmer_not_ready` devient
  `pump_not_ready`. Lecteurs à mettre à jour en même temps : `screen/src/ui/presenter.rs`,
  `screen-sim`, les tests de `screen-core` (`golden_json.rs`,
  `core_logic.rs`, `guided.rs`, `tests/common`) et `docs/firmware.md`.
  coffeetool et `firmware/analysis` ne lisent pas ces clés.
- **Parité avec le C++ archivé** : `tests/vectors/cjson_vectors.txt` compare
  `/telemetry` au cJSON du firmware C++ (`gen_cjson_vectors.sh`). Le
  nouveau schéma s'en écarte : les vecteurs de télémétrie sortent de cette
  comparaison, ceux de `/config` restent. `golden_json.rs` devient la
  référence de `/telemetry`.
- Aucun changement des machines d'état : remplissage, pré-infusion,
  infusion et purge envoient toujours un pourcentage. **Leurs réglages
  changent de sens** (section 6).

## 5. Versions et flash

- **Version 0.5.0** pour l'écran et sensors, pour marquer le passage à
  combined et à la pompe à engrenage : `FIRMWARE_VERSION` **et**
  `protocol/version.rs`.
- **Flash recommandé : par coffeetool, dans la machine, juste avant le
  démontage.** Écran d'abord, puis sensors par le proxy CAN. L'image
  sensors est confirmée par l'écran (`CONFIRM_SENSORS_OTA`) sur l'ancienne
  carte : GPIO 1 et GPIO 3 y sont NC, le nouveau firmware n'y fait rien de
  dangereux, et la pompe vibrante ne tournera plus (plus de dimmer), ce qui
  ne gêne pas puisqu'elle part.
- **Flash par USB au banc** : une image écrite dans `ota_0` puis basculée
  par `otatool` démarre en attente de confirmation. Sans écran sur le bus,
  elle revient en arrière au bout de 30 s. À vérifier avant de choisir
  cette voie ; sinon ne pas démarrer le XIAO avant de l'avoir posé dans la
  machine.
- Le XIAO MicroPython du banc garde son `main.py` de test (cycle de la
  vanne) : il ne va pas dans la machine.

## 6. Première mise en route dans la machine

Essais sobres, avec `/telemetry` ouvert :

1. **Repos** : `pump.rpm` à 0, aucun défaut, `pump_ready` vrai.
2. **Purges à 20, 40, 60 %** sans porte-filtre : relever `rpm` contre la
   vitesse visée (900, 1 800, 2 700 tr/min) et le débit Digmesa. Recaler
   Vmax si l'écart est le même partout ; si l'écart varie, remplacer la
   table de tensions par une table rapport cyclique → tr/min mesurée sur la
   pompe (balayage automatisé, plus tard).
3. **Défaut** : débrancher J9 pendant une purge courte. Le défaut doit
   tomber en moins d'1 s, la pompe s'arrêter et le LOG partir.
4. **Infusion** : régler les pourcentages, puis vérifier le PI de pression
   de l'écran, réglé pour la pompe vibrante. La pompe à engrenage est plus
   raide : le PI peut osciller. L'OPV (11 bar) borne la pression.

**Réglages de départ suggérés** (à 45 tr/min par %, 0,1 ml par tour), à
fixer par David via `/config` :

| Phase | Pompe vibrante | Besoin | Pompe à engrenage |
| --- | --- | --- | --- |
| Remplissage | 70 % (2,9 à 3,3 ml/s) | 3 ml/s jusqu'à ≈ 1 bar | ≈ 45–50 % |
| Pré-infusion | 50 % | ≈ 1–1,5 ml/s à basse pression | ≈ 20–25 % |
| Infusion | jusqu'à 100 %, cible 9 bar | 1,4–1,8 ml/s à 9 bar | plafond 100 %, cible 9 bar |
| Purge | 80 % (≈ 4,5–5 g/s) | 4,5–5 ml/s à 0 bar | ≈ 60–65 % |

## Hors de ce plan

- Régulation en débit (`docs/flowcontrol.md`) et extension de SET à une
  vitesse en tr/min.
- Boucle de vitesse fermée sur le tachymètre dans sensors : à décider après
  l'étape 2 de la section 6, selon l'écart mesuré.
- Filtrage de la pression allégé sans la pulsation de la pompe vibrante :
  après une capture de comparaison.
- Calibration du glissement (`pompe-engrenage.md`).

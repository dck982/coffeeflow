# Passage du XDB401 I2C au XDB401 analogique

Document de travail pour la transition de sonde de pression. Il complète la
section « Évolution prévue : XDB401 analogique sur l'ADS1115 » de
[cablage.md](cablage.md) et tient l'état d'avancement. À supprimer une fois la
transition terminée et ses résultats reportés dans `cablage.md`.

## Contraintes

- **Montage unique.** La sonde est peu accessible et le joint se change à chaque
  démontage. Une fois l'analogique monté, on ne revient pas à l'I2C : toutes les
  mesures de la sonde I2C se font **avant** le démontage.
- **Tout à froid.** Après une chauffe à 90 °C, il faut ~8 h pour revenir à
  l'ambiante. Les deux séries (I2C avant, analogique après) se font machine
  froide, chauffage désactivé (`heating.enabled=false`).

## La sonde reçue

- Facture : « SUP: 3.3V, OUT 0.4-2.4V, 0-1.2 MPa, G1/8 ».
- Gravure d'usine sur le cylindre : « SUP: 5V, OUT: 0.4-2.4V, RANGE: 0-1.2Mpa ».
  Marquage générique de la famille ; la pièce facturée est la version 3,3 V.
- Conversion d'usine : 0,4 V à 0 bar, 2,4 V à 12 bar, soit **6 bar/V**. Sur
  l'ADS1115 (±4,096 V, 125 µV par code) : 0,00075 bar par code.
- Joint torique ajusté fourni.

### Sortie non ratiométrique (étape 1 de `cablage.md`, faite)

Mesure au M5Stack Unit Voltmeter, sonde à l'air : **397,7 mV sous 5 V comme
sous 3,3 V**. La sortie ne suit pas l'alimentation (référence interne) : A0
n'intervient pas dans la conversion, et la ligne « Sortie ratiométrique » du
tableau d'erreurs de `cablage.md` disparaît. Écart au zéro nominal :
−2,3 mV ≈ −0,014 bar, absorbé par le zéro.

### Alimentation 3,3 V

Raisons de rester en 3,3 V :

- l'ADS1115 est alimenté en 3,3 V ; une entrée ne doit pas dépasser
  VDD + 0,3 V ≈ 3,6 V. Alimentée en 5 V, une sonde défaillante pourrait
  dépasser cette limite sur A2. En 3,3 V, c'est impossible ;
- c'est la tension de la pièce facturée.

Seul risque restant : une marge d'alimentation insuffisante en haut de plage,
qui aplatirait la sortie au plateau (≈ 2,0–2,07 V à 9,6–10 bar) sans toucher le
zéro. Le contrôle à 0,3 bar ci-dessous le détecte ; en cas d'écart, refaire la
série sous 5 V (voir « Analyse »).

## Joint et serrage

Joint torique, **pas de serrage à fond** (contrairement au joint plat de la
sonde I2C) : l'étanchéité vient de la compression fixée par la gorge ; serrer
plus pince ou extrude le joint.

1. Graisse silicone alimentaire, joint posé au fond de sa gorge contre le
   six-pans, pas sur le filet.
2. Visser à la main jusqu'au contact métal contre métal du six-pans.
3. Environ 1/8 de tour à la clé, pas plus.

La fiche XDB401 ne donne pas de couple. Mesurer le joint (diamètre intérieur ×
section) avant montage, pour pouvoir en racheter un.

**Loxeal 58-11** ([fiche](datasheets/5811e.pdf)) en complément sur le filet,
non décidé :

| Point | Fiche | Conséquence |
| --- | --- | --- |
| Eau potable | WRAS, eau chaude jusqu'à 85 °C | l'eau à la sonde peut dépasser 85 °C (produit stable jusqu'à 150 °C) |
| Polymérisation avant eau potable | 24 h laiton, 7 jours inox (21 °C) | pas de mesure le jour du montage |
| Élastomères | le produit liquide peut en attaquer certains | risque pour le joint torique |
| Démontage | desserrage 18–24 N·m (M10 acier) | démontable, mais effort sur le corps de la sonde |

Options : joint seul, Loxeal seulement en cas de fuite (il faut alors un joint
de rechange) ; ou joint + Loxeal dès le montage.

## Câblage

- Sonde analogique : **noir → GND**, **rouge → 3V3**, pris sur le port de
  l'ADS1115 dans le boîtier screen ; **jaune → A2**. Sortie et masse ensemble,
  torsadées si possible, à l'écart du 230 V.
- Côté XIAO : le module Grove de pull-ups (2 × 4,7 kΩ vers 3,3 V) prend la
  place du XDB401 sur **R1**. Sans lui, le dimmer ne répond plus et la pompe ne
  démarre pas, sans autre symptôme.

## Firmware et outils (0.3.42, non committé)

- `screen` lit A2 dans la boucle ADS1115 à 10 Hz, après A0/A1, même gain. Un
  échec sur A2 n'invalide pas la température chaudière ; il est signalé par
  les logs `BOILER_ADC_*` (étapes 8–10 = A2 dans le décodeur).
- `/telemetry` : `pressure.a2_raw` (`null` si la dernière lecture a échoué).
- Capture HF : `pressure_a2_raw` dans les vues `raw` et `both`, bit5 des
  `flags` = A2 lu.
- `record_probe.py` : enregistre à 5 Hz `a2_raw` et la pression I2C
  (`pressure_bar`, `pressure_valid`) en plus des codes NTC.
- `purge.py` : la durée est confiée au firmware. `purge.max_s` est réglé sur la
  durée arrondie au multiple de 5 supérieur, puis rétabli. Pour un multiple de
  5, l'arrêt ne dépend d'aucune requête Wi-Fi ; sinon `purge_release` arrête à
  la durée demandée, renvoyé jusqu'à l'arrêt firmware. Une réponse perdue à
  `purge_press` est vérifiée sur la télémétrie, jamais renvoyée.
- `purge.max_s` est actuellement réglé à **15 s** (sauvegarde
  `configs/261006-174056.json`) : `purge.py 15` ne modifie pas la
  configuration.

Plus tard, dans cet ordre : conversion A2 → `pressure_bar` / `pressure_valid`
dans `screen`, puis `sensors` sans XDB401 (lecture I2C et `STATUS_PRESSURE` à
retirer ou à marquer invalide, à trancher). Entre le démontage et la
conversion : **pas d'espresso** (remplissage, pré-infusion et contrôle de
pression lisent `pressure_valid`) ; les purges, à pompe fixe, fonctionnent.
`sensors` émet alors un `I2C_ERROR` à chaque lecture de pression ; le bus reste
libre (NACK immédiat).

## Méthode de comparaison

But : **continuité, pas exactitude.** La sonde analogique doit rendre la même
valeur que l'I2C au plateau de l'OPV (jamais retouchée), en « plateau − zéro ».

### Enregistrement d'une série

1. Terminal 1, depuis la racine du dépôt :
   `uv run firmware/tools/record_probe.py`, lancé avant la première purge et
   laissé jusqu'à la fin de la série. Il fournit les **zéros au repos**, absents
   des captures HF (qui démarrent avec la pompe).
2. Terminal 2, pour chaque purge : `uv run firmware/tools/purge.py 15`, puis,
   5 s après « Purge arrêtée » (cooldown d'une capture de purge),
   `uv run firmware/tools/download_hf_capture.py`. La capture HF donne le
   plateau à 10 Hz.
3. Entre deux purges, attendre que la pression soit stable.

### Pression piégée : il n'y a pas de zéro au repos

La sonde est entre la pompe et l'électrovanne. Au repos, la vanne ferme ce
tronçon : la sonde lit la pression piégée à la dernière fermeture, pas
l'atmosphère. Après une purge en panier aveugle, ~9,2 bar restent piégés
(captures du 25.09). La purge de test du 6.10 (`261006-174531`, sans
porte-filtre) est partie de **0,455 bar** piégés et a laissé **0,16 bar** (la
pression d'écoulement par la douchette à ~4 ml/s, figée à la fermeture), qui
décroît lentement (0,163 → 0,150 bar en 20 s). La lecture de −0,57 bar relevée
plus tôt n'a pas d'état de vanne connu.

D'où l'alternance : une purge sans porte-filtre ramène le tronçon à un état
bas reproductible, puis la purge en panier aveugle donne le plateau. Les deux
sondes voient ces deux mêmes états physiques ; la comparaison porte sur
« plateau − état bas ».

### Série I2C (avant démontage)

Deux cycles, chacun : purge sans porte-filtre (état bas), purge en panier
aveugle (plateau). Un cycle de plus resserre la dispersion.

### Série analogique (après montage)

1. Contrôler le dimmer par une purge courte.
2. Mêmes purges, mêmes conditions (froid, chauffage coupé, 100 %, 15 s).

### Analyse

- Par cycle : plateau moyen sur la partie stable − état bas piégé lu juste
  avant la purge en panier aveugle. Moyenne et dispersion par série.
- Pente d'usine (6 bar/V) : le plateau analogique doit tomber à **0,3 bar + la
  dispersion** de celui de l'I2C. Sinon, ne pas aligner :
  1. refaire la série analogique en alimentant la sonde **en 5 V** (A2 reste
     sous 2,1 V au plateau). Plateau identique : l'alimentation n'est pas en
     cause ;
  2. chercher ensuite un défaut de zéro ou de masse.
- Si l'écart est dans la tolérance : pente = (plateau − bas)<sub>I2C</sub> /
  (V<sub>plateau</sub> − V<sub>bas</sub>).
- Offset, **à trancher** : continuité (l'analogique reproduit les valeurs I2C,
  offset compris, via le point bas) ou exactitude (zéro à l'atmosphère, en
  décalant les consignes de l'offset I2C). Dans les deux cas, une lecture de la
  sonde I2C **à l'atmosphère** (vanne ouverte, pompe arrêtée : vanne de
  maintenance) chiffre son offset. Retenu : pas de vanne ; la sonde I2C
  démontée mais encore branchée, machine allumée, est à l'atmosphère, et
  `pressure.bar` se lit dans `/telemetry`. Même chose pour l'analogique sur A2
  avant de la visser.
- Report des valeurs (zéros, plateaux, tensions, pente) dans `cablage.md`.

### Valeurs de référence

| Grandeur | Valeur |
| --- | --- |
| I2C au repos, état de vanne inconnu | −0,57 bar |
| I2C piégé avant la purge de test du 6.10 | 0,455 bar |
| I2C pendant une purge sans porte-filtre, puis piégé après | 0,15–0,17 bar |
| Plateau OPV I2C, captures existantes (pas en panier aveugle) | 9,55 bar (19.09, infusion, 52 °C) ; 9,71–9,73 bar (25.09, panier limité, ~105 °C) |
| A2 relié au GND | 0 |
| A2 relié au 3V3 | 26 305 (= `ntc_a0_raw`, 3,288 V) ; 26 302 ± 0,9 sur 55 s le 6.10 |
| A2 en l'air | ~2 376, sans signification |
| A2 attendu, sonde à l'air | ~3 182 (397,7 mV) |
| A2 attendu au plateau | ~16 000–16 600 (2,0–2,07 V) |

Les captures existantes ne remplacent pas la série I2C : pas en panier aveugle
(l'OPV ne voit pas tout le débit), pas de zéro au repos, et 0,17 bar d'écart
entre elles.

## Résultats de la série I2C (6 octobre 2026, à froid)

Relevé `probe-20261006-175014-248141`, captures HF `261006-175111` à
`261006-175328`, purges de 15 s à 100 %, chauffage coupé, corps de sonde
27,3–27,5 °C. Alternance panier aveugle (`175111`, `175221`, `175328`) et sans
porte-filtre (`175149`, `175253`).

| Purge aveugle | État bas avant (moyenne record_probe, 3 s) | Plateau (moyenne dès 9,5 bar + 0,5 s) | Plateau − bas |
| --- | --- | --- | --- |
| `175111` | 0,151 bar | 9,868 ± 0,141 bar (creux à 9,53 vers 14 s) | 9,717 bar |
| `175221` | 0,111 bar | 9,900 ± 0,025 bar | 9,789 bar |
| `175328` | 0,147 bar | 9,869 ± 0,033 bar | 9,722 bar |
| **Série** | | | **9,743 ± 0,040 bar** (étendue 0,072) |

- Seuil pour l'analogique, pente d'usine : plateau − bas à **9,74 ± 0,34 bar**
  (0,3 bar + dispersion).
- Pression piégée après un panier aveugle : 6,14–6,25 bar (et non ~9,2 bar
  comme le 25.09). Après une purge sans porte-filtre : 0,11–0,15 bar.
- Pendant une purge sans porte-filtre : 0,04–0,15 bar.
- Lectures I2C aberrantes isolées : **12,00 et 12,50 bar** (`175221` en plein
  plateau, `175111` en cooldown), et une température à 56 °C ; 18 lectures
  `pressure.valid=false` dans le relevé. Ce sont les erreurs du bus I2C
  (« Interférences » dans `cablage.md`) ; elles sont écartées du plateau (écart
  > 0,5 bar à la médiane).

### Sonde I2C à l'air libre

Relevé `probe-20261006-183409-140018` : sonde démontée, encore branchée,
**+0,0270 ± 0,0007 bar** sur 118 lectures valides (3 invalides), pression
ambiante 961,3 hPa (Netatmo). La sonde est donc relative (une sonde absolue
lirait ~0,96 bar). Offset de +27 mbar, bien sous sa tolérance de ±0,16 bar.
Sans seconde mesure à une autre pression ambiante, on ne sait pas si sa
référence est ventilée ou scellée ; l'effet météo resterait de quelques
centièmes de bar.

Conséquences :

- l'état bas après une purge sans porte-filtre (0,11–0,15 bar) est une vraie
  surpression ; les −0,57 bar relevés plus tôt étaient une dépression piégée ;
- continuité et exactitude ne diffèrent que de **0,03 bar** : le zéro de
  l'analogique peut se prendre à l'air sans décaler les consignes.

### Sonde analogique à l'air libre

Relevé `probe-20261006-190857-984186` (3,5 min, les deux sondes à l'air) :

| Grandeur | Valeur |
| --- | --- |
| A2 | **3 167,3 codes** (395,9 mV), écart-type 1,47 code, min–max 3 163–3 171 |
| Bruit | 0,18 mV soit 0,0011 bar (écart-type), 0,006 bar crête à crête ; pas de dérive |
| Lecture d'usine (6 bar/V) | −0,025 bar (I2C au même moment : +0,027 bar) |
| Voltmètre M5Stack, plus tôt | 397,7 mV, soit +1,8 mV (0,011 bar) d'écart entre instruments |
| A0 | 26 301,7, inchangé : la sonde ne charge pas le 3V3 |
| Lecture A2 en échec | 1 sur 1 057 |

Zéro retenu : **3 167,3 codes = 0 bar** (atmosphère). Avec la pente d'usine :
bar = (code − 3 167,3) × 0,00075.

Attendus après montage, pente d'usine :

| Point | I2C rapporté à l'atmosphère | A2 attendu |
| --- | --- | --- |
| État bas après purge sans porte-filtre | 0,08–0,12 bar | ~3 280–3 330 |
| Plateau OPV | ~9,84 bar | ~16 290 (2,04 V) |
| Plateau − bas | 9,74 ± 0,34 bar | 12 990 ± 450 codes |

## État au 6 octobre 2026

- [x] Sortie non ratiométrique vérifiée au voltmètre.
- [x] Firmware 0.3.42 construit et flashé ; A2 validé (GND → 0, 3V3 → 26 305).
- [x] `purge.max_s` = 15 s, chauffage désactivé.
- [x] `purge.py` fiabilisé, `record_probe.py` étendu (tests hôte : 68 passent).
- [x] Purge de test avec enregistrement : `probe-20261006-174440-209287`,
      `261006-174531` (A2 sur 3V3). `purge.py` arrête par le firmware (pompe
      14,9 s), `pressure_a2_raw` présent et bit5 sur les 201 échantillons,
      relevé à 5 Hz sans trou notable.
- [x] Série I2C, voir « Résultats de la série I2C ».
- [ ] Purge sans porte-filtre avant démontage : la série finit sur un panier
      aveugle, 6,18 bar restent piégés.
- [x] Sonde I2C démontée, à l'air : +0,027 bar.
- [x] Sonde analogique à l'air sur A2 : 3 167,3 codes.
- [x] Démontage I2C, module de pull-ups sur R1, montage de l'analogique,
      raccord en T (laiton) refait au Loxeal 58-11 le 6.10 vers 19 h 45.
      Dimmer joignable en I2C (resté en calibration, comportement connu).
      A2 monté, circuit à pression nulle (`probe-20261006-194756-060623`,
      100 s) : 3 177,6 codes (+0,008 bar), mais écart-type **7,9 codes**
      (0,006 bar) contre 1,5 à l'air, surtout d'un échantillon au suivant,
      plus une dérive lente de ±10 codes. Probablement électrique (corps de
      sonde au châssis, trajet du câble) ; sans effet à l'échelle de la
      régulation. Filtre RC ou moyenne à envisager dans le firmware.
- [ ] +12 h : purge sans porte-filtre, contrôle visuel du raccord.
- [ ] +24 h (cure eau potable laiton) : purge en panier aveugle, essuie-tout
      sous le raccord en T et autour de la sonde. La pression piégée baisse
      même d'origine (fuite de l'électrovanne, vers le groupe) : sa pente ne
      distingue pas une fuite du raccord, seul le contrôle visuel tranche.
      Enregistrer quand même la décroissance (`record_probe.py`, 5–10 min)
      comme référence.
- [ ] Série analogique (mêmes purges que la série I2C) et analyse.
- [ ] Bruit de A2 sous découpe de phase : le câble longe désormais le 230 V
      (sortie avant, bouton marche). À 100 % le dimmer ne découpe pas ;
      ajouter une purge sans porte-filtre à `purge.pump_pct` = 50 et
      comparer l'écart-type de A2 au repos (7,9 codes), à 100 % et à 50 %.
- [ ] Firmware `screen` : conversion A2 en bar.
- [ ] Firmware `sensors` sans XDB401, mise à jour de `README.md` et `cablage.md`.
- [ ] Commit des changements firmware et outils.

# Régulation de la chaudière

L'écran lit la NTC chaudière et calcule la puissance demandée. Le module
capteurs transforme cette puissance en temps de marche du SSR sur une période
fixe de 5 s. L'écran renouvelle un bail de 1500 ms toutes les 500 ms : la
chauffe initiale peut durer plusieurs minutes, mais une perte de l'écran ou
du CAN coupe le chauffage à l'expiration du bail.
La consigne a une résolution de 0,1 %. Une période isolée ne peut produire
moins de 100 ms de chauffe avec le tick actuel ; un accumulateur répartit les
faibles consignes sur plusieurs périodes (1 % = 100 ms toutes les 10 s,
0,6 % ≈ 100 ms toutes les 16,7 s en moyenne).

La configuration NVS v6 contient `heating.brew_temperature_c` (90 °C par
défaut, 50 à 100 °C par pas de 0,5 °C) et `heating.enabled` (`true` par
défaut). La cible est modifiable sur la deuxième page des réglages ; le
commutateur n'est disponible que par `POST /config`. La purge reste possible
quand il vaut `false`. L'infusion exige une mesure fraîche dans la bande
consigne ±0,5 °C pendant trois secondes, et un module capteurs compatible.
La borne basse à 50 °C permet les essais de calibration à température modérée.
Tant que la courbe NTC reste provisoire, cette consigne est celle calculée par
le firmware et ne garantit pas 50 °C réels dans la chaudière.
`firmware/tools/set_heating.py true` ou `false` modifie ce commutateur via
`GET /config` puis `POST /config`, avec `COFFEEFLOW_HTTP_TOKEN` et
`COFFEEFLOW_IP` dans l'environnement.

Toute migration depuis une configuration v1–v5 persiste `heating.enabled=false`
avant le démarrage des tâches ; une installation neuve conserve `true`.

La loi actuelle est un réglage initial : puissance plafonnée à 100 % au repos,
anticipation de 20 s sur la pente filtrée dans les deux sens, maintien nominal
de 3,5 % à la consigne, et estimation bornée de la chaleur encore en transit à
partir de la puissance au-dessus de 3,5 % demandée durant les 22 dernières
secondes. La pente est filtrée sur 8 s et les variations de puissance non nulles
sont lissées sur 5 s ; les coupures restent immédiates. L'intégrale utilise l'erreur prédite, pour ne pas
accumuler une erreur déjà expliquée par cette chaleur. Pendant l'infusion et la
purge, le mode écoulement applique au moins 18 % lorsque la température reste
proche ou sous la cible, et limite la puissance à 35 %. Il ne prolonge pas la
pente négative de la NTC sur les 20 s de prédiction. Durant les 30 s suivant
l'arrêt de l'écoulement, la reprise reste plafonnée à 35 %, la pente négative
n'est pas extrapolée et l'intégrale est suspendue ; la chaleur déjà commandée
reste prise en compte. La prédiction de reprise utilise 10 s et retient le
plus grand effet entre la pente montante et la chaleur en transit, car ces
deux estimations se recouvrent partiellement. En reprise, la chauffe peut être
coupée dès que la prédiction passe au-dessus de la cible ; pendant
l'écoulement, l'appoint est
coupé quand la mesure dépasse la cible de 0,5 °C. Au-dessus de 105 °C, sur mesure
invalide ou si la chauffe est désactivée, la consigne tombe à zéro. Ces
coefficients doivent être ajustés avec des mesures sur la machine réelle.

Une purge commencée à 5 °C ou plus de la consigne, au-dessus ou en dessous,
est traitée comme une purge de réglage thermique : aucune chauffe n'est
commandée jusqu'à son arrêt, même si la NTC traverse la consigne. La reprise
du contrôleur après la purge reste active si la température descend sous la
consigne.

Sur la purge de 8 s du 24 septembre 2026 à consigne 90 °C, la télémétrie
ancienne loi montre une chute de la NTC jusqu'à 80,9 °C environ 15 s après
le début, suivie d'une commande maximale de 93,9 % vers 23 s et d'un pic à
96,6 °C vers 50 s. La température ponctuelle dans le panier était 86,7 °C.
Le nouveau plafonnement vise à réduire ce rebond ; son effet sur la machine
est visible dans l'essai suivant : pour une purge de 8 s à consigne 90 °C,
la puissance ne dépasse pas 35 %, la NTC atteint 79,4 °C au minimum puis
88,2 °C au maximum sur les 90 s enregistrées, et la température mesurée
immédiatement dans le panier est de 88,4 °C. Le départ était à 90,6 °C,
contre 89,7 °C lors de l'essai précédent, et le volume a aussi changé
(environ 33 ml contre 31 ml) : la comparaison n'isole pas l'effet du logiciel.
Le raccourcissement de la prédiction de reprise a été ajouté après cette
capture et reste à vérifier sur la machine.

Depuis 0.3.1, l'écran diffuse un `LOG` CAN à la première erreur ADS1115, puis
au plus toutes les 5 s si la même erreur persiste. `BOILER_ADC_NOT_FOUND`,
`BOILER_ADC_I2C_ERROR`, `BOILER_ADC_CONVERSION_TIMEOUT` et
`BOILER_NTC_INVALID_READING` distinguent les causes ; `BOILER_ADC_RECOVERED`
indique le retour d'une mesure valide et la durée de l'interruption. Le
décodeur `coffeetool` affiche l'adresse, l'étape I²C, le code ESP ou les valeurs
ADC brutes selon le cas.

Depuis 0.3.5, une erreur de lecture I²C entraîne un nouvel essai après 200 ms,
puis 400 ms, 800 ms et 1 s si elle persiste. Une lecture réussie remet ce délai
à 200 ms pour la prochaine erreur. Le PGA ADS1115 reste à ±4,096 V pour A0
et A1 ; le calcul de résistance utilise le rapport des deux codes bruts.

La calibration 0.3.5 a essayé un offset de -18 °C ; de la vapeur est sortie
pendant la purge. Depuis 0.3.6, l'offset est revenu à 0 °C et la coupure de
chauffe à 105 °C de la température publiée est rétablie. Les constantes NTC
restent provisoires en attendant une mesure à froid de la machine.

## État des essais du 24 septembre 2026 — reprise de la calibration

La sonde chaudière est la [Profitec P6036](https://links.imagerelay.com/cdn/2615/ql/423575fef2a046eb97c8b5af763d79b5/Pro-600-Parts-Diagram.pdf),
référencée comme NTC 1/8″ dans la nomenclature du fabricant. Sa courbe
résistance/température n'est pas indiquée dans cette nomenclature. Les
marquages lus sur le métal de la sonde installée sont `1408504` et `16/18` ;
leur signification n'a pas été confirmée et ne donne pas de valeur de `R25`
ou de `Beta`. La sonde n'a pas été démontée. La résistance fixe du pont, entre
A1 et GND, a été mesurée à **2,193 kΩ** ; la résistance de la NTC débranchée,
mesurée directement sur ses deux fils, était **42,1 kΩ** lors des essais à
froid. Les constantes actuellement compilées sont `R25 = 27 290 Ω`,
`Beta = 3 728 K`, sans offset. Avec elles, 42,1 kΩ donnent environ 15 °C.
Deux hypothèses nominales à comparer après l'essai à froid sont
`R25 = 47 kΩ, Beta = 4 700 K` et `R25 = 50 kΩ, Beta = 4 800 K` ; seule la
première correspond à une combinaison trouvée dans un
[catalogue Panasonic](https://industrial.panasonic.com/cdbs/www-data/pdf/AUA0000/AUA0000C10.pdf),
et ce composant n'identifie pas la sonde montée dans la chaudière. Une mesure
de résistance après une nuit d'équilibre près de 25 °C donnera directement
un meilleur ancrage pour `R25`. Comparer les deux courbes dans
[l'explorateur NTC](ntc_curve_explorer.html).
Les anciens couples résistance/température de
[`ntc_ads1115_calibration.md`](ntc_ads1115_calibration.md) utilisent la
température entière indiquée par le PID d'origine, lue avant coupure puis
débranchement de la sonde : ce n'est pas une mesure indépendante de la
température réelle de la NTC.

Après trois heures d'arrêt, la pièce était à **24,8 °C** près de la machine,
la plaque supérieure de la chaudière et l'écrou du raccord à **26,7 °C** au
thermomètre IR, tandis que le nouveau firmware affichait environ **13–15 °C**.
Les températures d'eau mesurées en tasse après purges, de 28,50 à 26,69 °C,
ne sont pas des mesures au contact de la NTC mais confortent l'existence d'un
écart à froid. Captures :
[`143100`](../captures/260924-143100.json),
[`143456`](../captures/260924-143456.json),
[`143921`](../captures/260924-143921.json),
[`145454`](../captures/260924-145454.json).

À température café, les mesures dans le panier après purge donnent **81,6 °C**
pour une consigne de 80 °C ([capture](../captures/260924-162535.json)) et
**86,7 puis 88,4 °C** pour une consigne de 90 °C
([avant mode écoulement](../captures/260924-164051.json),
[après](../captures/260924-165337.json)). Il semble donc possible de faire
un café autour de 90 °C avec le firmware actuel, à quelques degrés près dans
ces conditions ; la température du panier n'est pas identique à celle de la
NTC. À consigne 65 °C, deux purges ont donné **72,6 puis 69,6 °C** dans le
panier malgré une NTC proche de 65 °C avant chaque purge
([première](../captures/260924-170809.json),
[seconde](../captures/260924-171142.json)). Une purge intermédiaire a eu lieu
et le panier a été vidé deux fois entre ces mesures : sa chaleur résiduelle
n'explique pas seule la baisse. Le groupe et le circuit d'eau ont pu se
refroidir ; ces points ne permettent pas encore de modifier la courbe NTC.

Les relevés longs des purges à 90 °C sont
[`163749`](../captures/monitor-heating-20260924-163749-565936.json) et
[`165225`](../captures/monitor-heating-20260924-165225-542907.json) ; les
relevés à 65 °C sont
[`170713`](../captures/monitor-heating-20260924-170713-740281.json) et
[`171044`](../captures/monitor-heating-20260924-171044-990745.json).
Ces fichiers `captures/` sont locaux et ignorés par Git.

### Prochain essai à froid

Laisser `heating.enabled=false` et la machine au repos toute la nuit. Au
redémarrage, vérifier que la chauffe est toujours désactivée. Avant toute
purge, relever la température ambiante près de la chaudière, celle de l'eau
du réservoir avec le même thermomètre digital utilisé dans le panier, et
sauvegarder `GET /telemetry`, notamment `boiler_temperature_c`,
`boiler_ntc_a0_raw`, `boiler_ntc_a1_raw`, `xdb401_temperature_c`,
`heating_enabled` et l'état de la chauffe. L'eau du réservoir peut différer
de l'air ambiant. La mesure IR sur métal nu est sensible à l'émissivité ; la
température de la pièce et l'équilibre nocturne sont les meilleurs repères
pour la NTC installée.

Faire ensuite **deux purges identiques de 8 s**, avec le même panier et le
thermomètre digital immédiatement après chaque purge, en vidant le panier
entre les deux. Noter la température de l'eau du réservoir, la température
dans le panier, la durée, le volume et toute différence de procédure.
`firmware/tools/purge.py 8` lit la télémétrie avant et après mais n'enregistre
que les valeurs affichées dans le terminal ; sauvegarder séparément la
télémétrie brute avant la première purge et les captures haute fréquence :

```sh
curl -fsS -H "Authorization: Bearer ${COFFEEFLOW_HTTP_TOKEN}" \
  "http://${COFFEEFLOW_IP}/telemetry" > captures/cold-before.json
uv run firmware/tools/purge.py 8
uv run firmware/tools/download_hf_capture.py
```

Répéter les deux dernières commandes pour la seconde purge et renommer les
captures si nécessaire. `record_heating.py --mode monitor` requiert
`heating.enabled=true` et ne convient donc pas à cet essai. Le calcul de
résistance à partir de la télémétrie est
`R_NTC = 2193 × (A0/A1 − 1)` en ohms ; comparer ce résultat à la mesure
directe antérieure de 42,1 kΩ. Si la NTC affiche encore environ 15 °C alors
que l'ensemble machine/eau/pièce est proche de 25 °C, on aura un point froid
solide. On pourra ensuite ajuster `Beta` et `R25` en conservant un ancrage
logiciel à 90 °C, puis contrôler la nouvelle courbe à 50–100 °C et la coupure
de chauffe. [Explorateur interactif des courbes](ntc_curve_explorer.html) :
il montre la sensibilité de chaque paramètre sans modifier le firmware.

Pour mesurer une montée depuis l'ambiante :

Le script `firmware/tools/record_heating.py` automatise les étapes 3 et 4,
coupe ensuite la chauffe et écrit un JSON dans `captures/`. Il lit
`COFFEEFLOW_HTTP_TOKEN` et `COFFEEFLOW_IP` ; `--output` permet de choisir
le fichier.

1. Envoyer `POST /config` avec
   `{"version":6,"heating":{"enabled":false}}`, puis laisser refroidir.
2. Allumer la machine et activer le Wi-Fi. La valeur `false` reste persistée.
3. Envoyer `POST /config` avec
   `{"version":6,"heating":{"enabled":true}}`.
4. Interroger `GET /telemetry` toutes les 500 ms. Enregistrer `uptime_ms`,
   `boiler_temperature_c`, sa validité et son âge, `heating_power_pct`,
   `heating_power_accepted_pct`, `heater_on` et
   `brew_temperature_target_c`. Le client doit horodater lui aussi chaque
   réponse. L'écho de puissance et `heater_on` viennent du module capteurs ;
   une absence d'écho frais doit apparaître comme telle dans l'analyse.

Un essai de chauffe et un essai d'infusion sont nécessaires avant de considérer
les gains calibrés. Le mode vapeur reste à ajouter : il devra sélectionner une
cible propre, bloquer l'infusion et garder la purge disponible.

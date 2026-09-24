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
défaut, 60 à 100 °C par pas de 0,5 °C) et `heating.enabled` (`true` par
défaut). La cible est modifiable sur la quatrième page des réglages ; le
commutateur n'est disponible que par `POST /config`. La purge reste possible
quand il vaut `false`. L'infusion exige une mesure fraîche dans la bande
consigne ±0,5 °C pendant trois secondes, et un module capteurs compatible.
`firmware/tools/set_heating.py true` ou `false` modifie ce commutateur via
`GET /config` puis `POST /config`, avec `COFFEEFLOW_HTTP_TOKEN` et
`COFFEEFLOW_IP` dans l'environnement.

Toute migration depuis une configuration v1–v5 persiste `heating.enabled=false`
avant le démarrage des tâches ; une installation neuve conserve `true`.

La loi actuelle est un réglage initial : puissance plafonnée à 100 %,
anticipation de 20 s sur la pente filtrée dans les deux sens, maintien nominal
de 3,5 % à la consigne, et estimation bornée de la chaleur encore en transit à
partir de la puissance au-dessus de 3,5 % demandée durant les 22 dernières
secondes. La pente est filtrée sur 8 s et les variations de puissance non nulles
sont lissées sur 5 s ; les coupures restent immédiates. L'intégrale utilise l'erreur prédite, pour ne pas
accumuler une erreur déjà expliquée par cette chaleur. Une compensation de
15 % s'ajoute pendant l'infusion. La puissance peut toujours retomber à zéro si la
température prévue dépasse la cible. Au-dessus de 105 °C, sur mesure
invalide ou si la chauffe est désactivée, la consigne tombe à zéro. Ces
coefficients doivent être ajustés avec des mesures sur la machine réelle.

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

# Plan temporaire — température de chaudière dans `screen`

Ce document suit l'ajout de la mesure NTC/ADS1115 au firmware de l'écran. Le supprimer une fois la fonction déployée et ses contrats stables intégrés à `firmware.md`. Le montage et les paramètres provisoires sont décrits dans [ntc_ads1115_calibration.md](ntc_ads1115_calibration.md) ; la [fiche ADS1115](datasheets/ads1115.pdf) détaille la programmation I2C en §7.5 et les registres en §8.1. Le plan de l'actionneur est dans [heating-firmware-plan.md](heating-firmware-plan.md).

## Résultat attendu

Le Waveshare lit la **température de chaudière** par son bus I2C et l'affiche à la place de la température issue du XDB401. `GET /telemetry` et les captures HF exposent la mesure chaudière, ses valeurs ADC brutes et son état de validité. La température XDB401 reste identifiable comme telle dans les données de diagnostic ; elle ne doit pas être confondue avec celle de la chaudière. Une absence ou panne de l'ADS1115 ne doit pas empêcher l'écran, le CAN, HTTP ou l'OTA de démarrer.

## 1. Vérifier le bus et le montage

- L'écran utilise déjà `board::i2c_bus()` sur **GPIO 8 SDA / GPIO 9 SCL**, partagé notamment avec le **CH422G et le contrôleur tactile GT911**. Ajouter l'ADS1115 comme périphérique de ce bus ; ne pas créer un second contrôleur sur ces mêmes broches. Selon ADDR, l'adresse 7 bits est **0x48 (GND), 0x49 (VDD), 0x4A (SDA) ou 0x4B (SCL)** ; relever celle du breakout et vérifier qu'elle ne chevauche aucun périphérique présent. Le bus existant tourne à 400 kHz pour le CH422G, vitesse acceptée par l'ADS1115 (§7.5.1.3). Vérifier que les conversions et leurs délais n'altèrent ni le tactile, ni les accès au CH422G, ni la sélection CAN.
- Relever la tension de **VDD de l'ADS1115**, les résistances de tirage SDA/SCL du breakout et les tensions du port I2C Waveshare. Le bus de l'ESP32 doit rester à **3,3 V**. Le 3,3 V de l'AMS1117 alimente le pont NTC ; il est aussi mesuré sur **A0**. La NTC va de ce 3,3 V à **A1**, et **2,193 kΩ** relient A1 à GND. Toutes les masses sont communes.
- Vérifier au repos A0 proche de la tension réelle du LDO, A1 entre 0 et A0 et le sens de variation d'A1 quand la chaudière chauffe. Conserver l'I2C disponible pour le CH422G même lorsque l'ADS1115 est débranché.

## 2. Ajouter la lecture dans `firmware/screen`

- Créer un petit pilote ADS1115 qui utilise le handle I2C partagé de `board`. Le registre **Config est à l'adresse pointeur 0x01**, le résultat à **0x00** (§8.1). Pour chaque canal, écrire le pointeur puis les deux octets Config **MSB d'abord** ; lancer une conversion *single-shot* (bit OS), attendre OS=1 dans Config ou le délai de conversion, repositionner le pointeur sur Conversion et lire ses deux octets MSB d'abord (§7.5.2–7.5.3). Lire **A0 puis A1 en mode simple entrée vers GND** : MUX `100b` puis `101b`. La configuration par défaut est **différentielle A0−A1**, donc elle ne convient pas au pont. Employer le même PGA `001b` pour les deux (plage **±4,096 V**) et choisir explicitement le débit ; `128 SPS` est le défaut du composant (§8.1.3). La donnée de conversion est un entier 16 bits signé en complément à deux (§7.5.4). Attendre les conversions sans bloquer les tâches UI ou CAN.
- Ne pas envoyer de *general call reset* sur le bus partagé : l'ADS1115 répond à l'adresse générale 0x00 (§7.5.1.2), et une telle commande pourrait aussi atteindre d'autres circuits.
- Lire les deux canaux à cadence définie, par exemple quelques mesures par seconde au repos et au moins une paire fraîche par échantillon HF à 10 Hz si la conversion le permet. Horodater une **paire** A0/A1 ; ne pas calculer un rapport avec deux lectures éloignées dans le temps. Utiliser une tâche dédiée de basse priorité et des transactions I2C courtes.
- Calculer `R_NTC = 2193 × (A0/A1 − 1)` puis la température avec les constantes provisoires du document de calibration (`R0 = 27290 Ω`, `B = 3728 K`, `T0 = 298,15 K`). Les deux canaux doivent employer le même PGA. Garder les paramètres de calibration nommés et remplaçables ; ne pas figer le 3,316 V mesuré comme référence à la place d'A0.
- Marquer la mesure invalide si I2C échoue, si un code ADC sature, si A0 est incohérent, si A1 est nul/hors intervalle, si la résistance/température calculée est hors plage plausible ou si la paire est périmée. Publier la dernière valeur avec son âge et sa validité ; ne jamais présenter un zéro ou une ancienne valeur comme température actuelle.

## 3. Séparer les deux températures dans le cœur et l'API

- Dans `core::Snapshot`, créer des champs dédiés `boiler_temperature_c`, `boiler_ntc_a0_raw`, `boiler_ntc_a1_raw`, `boiler_temperature_valid`, `boiler_temperature_freshness` et `boiler_temperature_age_ms` (noms exacts à fixer avant codage). Ajouter un point d'entrée du cœur pour publier une lecture atomique A0/A1 depuis la tâche ADS1115.
- Dans `on_status_pressure()`, conserver la température XDB401 sous un nom explicite, par exemple `xdb401_temperature_c` et `xdb401_temperature_raw`. Sa validité reste liée au statut XDB401 ; elle ne doit plus piloter la température de chaudière. Distinguer aussi les deux températures dans les logs et l'écran de diagnostic.
- Dans `GET /telemetry`, ajouter les champs explicites `boiler_temperature_c`, `boiler_temperature_valid`, `boiler_temperature_age_ms`, `boiler_ntc_a0_raw`, `boiler_ntc_a1_raw` et `xdb401_temperature_c`. Garder provisoirement `temperature_c`/`temperature_raw` avec leur **sens actuel XDB401**, documenté, pour les clients existants ; ne pas les réinterpréter silencieusement. Si ces alias doivent disparaître, versionner la réponse.
- Dans `ui_home.cpp`, utiliser **uniquement la validité et la fraîcheur chaudière** pour le nombre de température principal. Afficher « — » si l'ADS1115 est absent/invalide ; ne pas se rabattre silencieusement sur XDB401. L'écran de diagnostic peut afficher côte à côte les deux températures avec leurs sources.

## 4. Étendre les captures HF

- Dans `core::HFSample`, ajouter les codes **A0/A1 bruts**, la température chaudière calibrée et un bit de validité chaudière ; conserver la température XDB401 sous un nom explicite. Relever la consommation de RAM de la nouvelle structure × `kHFCaptureCapacity` avant de figer la taille du buffer.
- Échantillonner dans `tick_hf_capture()` la **dernière paire ADC horodatée**, sans lancer de conversion I2C dans la boucle HF. Enregistrer sa fraîcheur ou son âge afin qu'une lecture répétée à 10 Hz ne soit pas interprétée comme dix mesures indépendantes.
- Dans `net_http.cpp`, étendre les vues `raw`, `calibrated` et `both`. Préférer un schéma **`coffeeflow.hf_capture.v2`** avec des noms explicites (`boiler_ntc_a0_raw`, `boiler_ntc_a1_raw`, `boiler_temperature_c`, `xdb401_temperature_c`) et les indicateurs de validité. Adapter la taille du buffer JSON par échantillon et vérifier qu'aucune troncature n'interrompt l'export.
- Adapter `firmware/tools/plot_hf_capture.py` pour lire v1 et v2 : v1 trace l'ancienne `temperature_c` XDB401 ; v2 trace la chaudière en premier et peut montrer XDB401 à part. Les captures déjà enregistrées restent lisibles sans changer leur sens.

## 5. Déploiement et lien avec `heating`

La lecture ADS1115 n'exige pas de changement CAN côté `sensors` : elle peut être intégrée dans une image `screen` compatible avec le firmware actuel. Déployer avec la stratégie de validation OTA décrite dans [heating-firmware-plan.md](heating-firmware-plan.md) : une image qui répond au CAN mais perd HTTP ne doit pas être validée définitivement. Vérifier après redémarrage l'accès à `GET /telemetry` et à la route de flash, y compris avec l'ADS1115 débranché.

Le premier essai de LED du SSR utilise une impulsion courte et n'exige pas encore la NTC. Pour toute chauffe normale et durable, exiger une mesure chaudière fraîche/valide, une limite de température et un repli chauffage coupé si la mesure disparaît. La régulation PID/thermostat et les consignes café/vapeur constituent une étape distincte.

## Fin du suivi

Supprimer ce plan après validation sur la machine de la mesure I2C, de l'affichage chaudière, de la télémétrie, des captures HF v2, de la lecture des anciens exports et de l'OTA `screen` récupérable sans USB.

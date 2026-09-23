# Plan temporaire — actionneur `heating`

Ce document suit l'introduction de la commande de chaudière. Le supprimer une fois le firmware déployé et documenté dans `firmware.md` et `cablage.md`.

## Résultat attendu

Un `POST /action` authentifié active brièvement le chauffage **sans démarrer la pompe ni ouvrir la vanne**. La réponse HTTP et `GET /telemetry` montrent la consigne et l'état rapporté par `sensors`, et la LED du SSR chaudière s'allume puis s'éteint. Ce premier jalon vérifie la chaîne de commande ; il ne constitue pas encore une régulation de température. La mesure NTC/ADS1115 dans `screen` et la régulation viendront ensuite.

Matériel : **L2 / D2 / GPIO 3** du XIAO → **IN4 du HW-399** dans `boitier_pid` → **OUT4 à 5 V** → entrée DC `+`/`−` du SSR chaudière. HIGH demande la chauffe. Le SSR de **vanne** reste le M5Stack Unit SSR sur **R4 / D10 / GPIO 9**. Le dimmer de **pompe** reste sur I2C. Voir `cablage.md`.

## 1. Figer les interfaces et les noms

- Renommer les noms **métier** : `ssr` → `valve_open` / `valve`, `dimmer` → `pump_pct` / `pump`. Garder `dimmer` dans les noms du pilote DimmerLink, de ses registres I2C et de ses diagnostics matériels.
- Ajouter `heater_on` pour l'état appliqué et `heating` ou `heater_on` pour la commande HTTP. Employer un seul nom dans l'API publique ; proposition : `set_heating` avec `on: true|false`, et `heater_on` dans la télémétrie.
- Conserver au premier déploiement les **octets et identifiants CAN existants** : dans `SET` (0x01), l'octet 0 reste la puissance de pompe ; dans `STATUS_ACTUATORS` (0x22), les octets 0 et 1 restent vanne et pompe. Renommer les champs C++ sans changer leur disposition. Garder provisoirement les champs HTTP `dimmer` et `dimmer_pct` comme alias pour les clients existants ; introduire `pump_pct` sans modifier le sens des anciens champs.
- Ajouter des messages CAN distincts pour le chauffage, par exemple `SET_HEATING` (0x04, commande + bail) et `STATUS_HEATING` (0x23, état appliqué + bail/validité). Une ancienne image `sensors` ignore le nouveau type ; une nouvelle image `screen` doit accepter l'absence de `STATUS_HEATING` comme « chauffage indisponible » et refuser le POST. Ne jamais interpréter l'ancien bit `ssr` comme le chauffage.
- Annoncer la capacité chauffage dans une version/capacité de `sensors` observable par `screen`. Un simple PONG CAN prouve la liaison, pas la compatibilité fonctionnelle.

## 2. Rendre le chauffage indépendant

- Dans `sensors`, configurer **GPIO 3 en sortie LOW dès le début du boot**, avant CAN et avant de traiter des commandes. Mettre LOW aussi sur `STOP`, perte de présence, début d'OTA, erreur fatale et expiration du bail chauffage.
- **GPIO 3 est une broche de strapping de l'ESP32-S3** ([documentation Espressif](https://docs.espressif.com/projects/esp-idf/en/v5.0/esp32s3/api-reference/peripherals/gpio.html)). Avant le premier flash, mesurer son niveau au démarrage avec le HW-399 raccordé et vérifier que le montage n'active pas le SSR avant l'initialisation logicielle et ne modifie pas le chemin JTAG/USB de récupération. Un LOW défini uniquement par le firmware arrive trop tard pour protéger la phase de reset.
- Garder le séquencement actuel pompe → vanne pour les cycles d'infusion ; le futur code peut le nommer explicitement. La commande chauffage possède **son propre état et son propre bail**. Elle ne dépend ni de `pump_pct > 0` ni de l'état de la vanne. Un `SET` infusion ne doit ni activer ni éteindre le chauffage ; `SET_HEATING` ne doit pas toucher à la pompe ou à la vanne.
- Le verrou actuel de **60 s** concerne pompe/vanne. Le chauffage devra fonctionner au repos et durant une infusion ; il ne doit pas hériter de ce plafond. Prévoir une limite adaptée à la chaudière et à sa future mesure NTC. Pour le premier essai LED, limiter la commande à une impulsion courte avec extinction automatique et sans restauration après reboot. Conserver les protections thermiques matérielles de la machine.
- Dans `screen`, conserver une consigne chauffage indépendante de la machine d'infusion et renouveler son bail tant qu'elle est voulue. La perte de CAN, l'arrêt du service ou un flash doivent laisser le bail expirer et ramener GPIO 3 à LOW. Ne pas réactiver automatiquement la chauffe après un redémarrage. Le POST `on:false` doit couper immédiatement, même si aucune infusion n'est active.
- Distinguer dans le code un **essai diagnostic limité dans le temps**, qui s'arrête sans renouvellement, et la future demande de chauffe régulée, qui pourra renouveler son bail. La première livraison n'autorise que l'essai diagnostic.
- Faire remonter `heater_on`, la fraîcheur de son écho et la capacité de `sensors` dans le snapshot et `GET /telemetry`. Une réponse `200` au POST signifie seulement « commande acceptée » ; vérifier ensuite l'écho et la LED. Si l'écho manque ou vieillit, exposer un état inconnu et couper la consigne.

## 3. Rendre l'OTA déployable sans USB

Constats dans le code actuel :

| Point | Conséquence pour ce changement |
| --- | --- |
| `screen` accepte `POST /firmware?target=screen` et `target=sensors` ; le transfert de `sensors` passe par `ota_proxy.cpp` et CAN | Mettre d'abord à jour `screen` avec un protocole compatible avec l'ancien `sensors`, puis `sensors` |
| Les deux images OTA se valident après un PONG CAN | Une image qui parle CAN mais perd HTTP ou le proxy de flash peut se valider et devenir difficile à récupérer à distance |
| `ota_proxy.cpp` retente l'envoi d'un bloc si l'ACK manque ; `sensors/main.cpp` indique que le récepteur ne déduplique pas les blocs rejoués | Un ACK perdu peut faire échouer le flash au CRC global ; rendre le protocole de retransmission idempotent avant le déploiement |
| Le proxy utilise `ota_staging`, puis se replie sur `assets` si la partition manque | Vérifier la table de partitions réellement installée et supprimer le repli destructeur vers `assets` avant le flash ; une mise à jour OTA ne change pas la table de partitions |
| `begin_flash()` coupe aujourd'hui la pompe/vanne et ne vérifie que leur état | Ajouter `heater_on` aux conditions de refus et exiger un écho **frais de tous les actionneurs à l'arrêt** avant d'effacer ou de transférer ; envoyer `STOP` et confirmer son écho |
| Le proxy retourne HTTP `202 queued` dès la mise en file et finit le flash en recevant `END` | Attendre ensuite la nouvelle version, la capacité chauffage et l'état de repos ; `202` seul ne prouve ni le redémarrage ni la disponibilité du nouveau firmware |

Avant les images fonctionnelles, livrer une **mise à jour de récupération** du `screen` qui garde les anciens messages CAN, fiabilise le proxy et permet un contrôle distant de la santé OTA. Pour `screen`, ne valider une image `PENDING_VERIFY` qu'après retour du CAN **et** preuve que Wi-Fi/HTTP et la route de flash sont accessibles ; une confirmation distante explicite avec délai de rollback est préférable à un simple PONG. Pour `sensors`, attendre depuis `screen` sa nouvelle version et ses statuts avant une confirmation explicite de l'image ; l'absence de cette confirmation doit provoquer le rollback. Vérifier d'abord les partitions et l'image de repli effectivement présentes sur chaque carte : la partition `factory` ne garantit pas à elle seule un chemin de récupération distant.

Ordre de déploiement :

1. Relever via HTTP/CAN les versions, partitions et états de démarrage actuels ; conserver les deux binaires connus fonctionnels et un accès réseau stable. Vérifier que `screen` peut encore flasher `sensors` avant de toucher à ce dernier.
2. Déployer **`screen` compatible ancien/nouveau protocole**, avec le proxy et l'OTA durcis. Confirmer HTTP, CAN, télémétrie et possibilité de mise à jour après redémarrage ; sinon laisser revenir l'image précédente.
3. Déployer **`sensors`** avec GPIO 3 LOW au boot et le nouveau message chauffage. Confirmer version, capacité, état `heater_on=false`, télémétrie pompe/vanne et capacité de recevoir un autre flash. En cas d'échec, laisser le rollback se faire sans valider l'image.
4. Autoriser le POST de chauffage seulement après confirmation de la capacité et de l'écho `STATUS_HEATING`. Faire une impulsion courte, constater LED allumée, puis `on:false` et LED éteinte. Vérifier que pompe et vanne restent inactives ; vérifier aussi l'extinction à l'expiration du bail et à la perte du bus.

## 4. Portée du premier POST

Exemple de contrat proposé :

```http
POST /action
Authorization: Bearer <token>
Content-Type: application/json

{"action":"set_heating","on":true,"ttl_ms":1000}
```

`ttl_ms` borne la **durée maximale d'une commande de diagnostic** pour le premier jalon, sans renouvellement automatique. Le serveur refuse `on:true` si le flash est en cours, si `sensors` est absent, si sa capacité chauffage manque, si son état est inconnu ou si le système est verrouillé. La réponse devrait inclure l'état demandé et l'état appliqué une fois confirmé, ou indiquer clairement que l'écho n'est pas encore arrivé. Un POST `on:false` reste possible dans tous les états où le serveur répond.

Pour le fonctionnement normal de la chaudière, ajouter ensuite la lecture ADS1115/NTC, des limites de température et une régulation qui maintient une consigne ; ne pas transformer le POST de diagnostic en chauffe indéfinie sans mesure de chaudière.

## Fin du suivi

Le plan peut être supprimé lorsque le POST, l'écho CAN, l'indépendance chauffage/infusion, les replis de sécurité et le déploiement OTA sans USB sont validés sur la machine, et que leurs contrats stables sont intégrés à `firmware.md`.

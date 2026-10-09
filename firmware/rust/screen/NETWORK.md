# Réseau, stockage et OTA

Les services vivent dans la bibliothèque `screen` et s'exécutent sur le cœur 1.
L'assemblage initialise la flash/table de partitions avant la remapping XIP,
puis appelle `nvs::start(spawner, flash, layout).await` et
`network::start(spawner, wifi).await`. Il peut alors annoncer la radio prête
et créer `Core::new(service::CoreStore::new(), clock)` sur le cœur 0.

À chaque tour de rendu, appeler `service::process(&mut core, clock)` puis
les entrées/tick du cœur. Router ses sorties `Radio` vers
`network::configure`, `ForgetNetwork` vers `network::forget_network`, et
les trames CAN reçues/émises vers `network::mirror` (en plus de TWAI/USB).
`network::wall_time()` fournit l'heure Unix facultative ; aucune correction
SNTP ne modifie le temps monotone.

`CoreStore` échange des opérations NVS sur deux canaux inter-cœurs. Il attend
l'acquittement durable avec interruptions actives, sans verrou ni section
critique. Aucun timeout ne peut annoncer un échec suivi d'une écriture tardive
réussie : le watchdog de l'application détecte un cœur 1 bloqué. Les accès
flash sont sérialisés par un mutex async ; aucune section critique ne couvre
le travail du pilote. NVS est compatible IDF (`ui/a`, `ui/b`, blob
`wifi/credentials` de 97 octets), les writes sont relus avant succès. `Nvs::new` peut réparer des pages dès
l'ouverture (et donc écrire aussi lors d'une lecture) ; la production autorise
ces réparations avec le même pilote que l'essai NVS.

STA avec identifiants ; AP `CoffeeFlow-XXXX`, WPA2, 192.168.4.1/24 et DHCP
sans identifiants. Une panne STA ne lance jamais l'AP. Le toggle session
arrête/redémarre uniquement le Wi-Fi (pas BLE), via les symboles IDF car le
contrôleur actuel n'expose pas d'arrêt public. L'AP sert la page et `/scan`
(texte, inséré par textContent) ; les identifiants enregistrés provoquent un
redémarrage. Pour scanner, le pilote impose temporairement AP+station non
connectée, puis revient en AP : le BLE reste coupé par le cœur.

Quatre workers HTTP, trois sessions WebSocket au plus. `/telemetry`,
`/config`, `/action` et `/hf-capture` passent par les handlers de screen-core.
Le cadrage borné rejette les longueurs ambiguës, les headers dupliqués et le
chunked. Les documents de capture sont construits sur le cœur 0 en PSRAM ;
le réseau les écrit en flux TCP sans créer une seconde copie. Le WebSocket
émet les PDU CAN du protocole historique, répond aux ping et ignore les
messages clients bornés : aucune injection CAN. Une session trop lente est
fermée sans retenir le bus.

OTA écran : effacement par blocs de 64 Ko au fil du flux vers le slot inactif, puis relecture structure ESP32-S3,
checksum XOR et SHA256 appendu. Otadata bascule après cette vérification,
puis réponse HTTP et reboot. Une image pending dispose de cinq minutes,
comme le C++, pour une confirmation explicite avec STA connectée, capteurs
vivants, PONG et actionneurs frais ; sinon invalidation/reboot/rollback IDF.
La confirmation d'une image déjà valide reste idempotente.
OTA sensors : staging, réponse queued, transfert par les blocs CAN du proxy.
OTA USB locale : FLASH_CTRL/DATA, blocs 2048 octets et CRC16/CRC32, même
vérification finale, ACK seulement sur USB, expiration et abandon des octets
anciens après 5 s.

Copier `secrets.example.rs` en `secrets.rs` et remplacer les deux constantes.
Sans ce fichier, le build utilise l'exemple mais **désactive l'authentification
HTTP et le provisioning** : `CHANGE_ME` n'autorise aucune requête.
Le firmware peut toujours rejoindre une STA existante sans exposer HTTP.
Aucun identifiant n'est journalisé.

Validation : tests et Clippy hôte du crate `screen-net`, compilation et Clippy
Xtensa de la bibliothèque et du binaire `screen`. L'assemblage final est une
tâche séparée ; DHCP, reconnexion, WS, écritures flash, OTA et rollback restent
à vérifier sur carte sur décision de David. Aucun flash matériel dans ce port.

## Assemblage et contrôles hors matériel

Le binaire initialise maintenant le propriétaire flash et le réseau sur le
cœur 1 avant le LCD ; `runtime.rs` possède le cœur et traite les requêtes
réseau sur le cœur 0. Le pilote CAN distribue lui-même ses miroirs WS, sans
écho USB des trames issues du pont. L'exclusion OTA reste active jusqu'à la
libération du mutex flash, y compris durant l'attente de l'écho STOP sensors.

Les quatre workers gardent leur fenêtre RX de 8192 octets. RX/TX et en-têtes
HTTP sont en PSRAM, pour un maximum de 56 Ko de tampons, afin de préserver la
SRAM radio. Le tas interne est de 120 Ko et la pile du cœur 1 de 16 Ko.
`stack-budget.x` impose au moins 16 Ko de pile au cœur 0 ; le linker produit
une carte mémoire `screen.map` dans le répertoire de sortie du build.

Depuis `firmware/rust/screen`, sans accès à la carte :

```sh
source ~/export-esp.sh
cargo build --release --offline
cargo clippy --release --lib --bin screen --offline -- -D warnings
espflash save-image --chip esp32s3 --flash-size 16mb --skip-update-check \
  target/xtensa-esp32s3-none-elf/release/screen target/screen-0.4.0.bin
```

Ne pas utiliser `--all-targets` pour les contrôles Xtensa : le harnais Rust
`test` n'existe pas sur cette cible. Les tests du protocole, du cœur et des
formats réseau s'exécutent dans le workspace hôte. La compilation sans
`secrets.rs` ne permet pas de tester HTTP/provisioning sur carte ; renseigner
ce fichier local avant les essais réseau. Les premiers essais USB restent
sur le bureau, sans CAN, avec le slot C++ et les précautions de retour conservés.

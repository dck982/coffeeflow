# Tâche 7 — CAN, pont USB et OTA sensors

Les services sont exportés par la bibliothèque `screen`. L'assemblage lance
`can::task` avec TWAI à 500 kbit/s, RX GPIO19/TX GPIO20, après avoir placé
CAN_SEL haut ; il lance `serial_bridge::receive`/`transmit` avec UART0 à
115200 bauds, RX GPIO44/TX GPIO43. Le cœur consomme `can::RX` et produit
`can::TX` ; la présence, les PING/PONG et les décisions de confirmation
restent dans `screen-core`. `can::WS` est le miroir brut destiné au réseau.

Les trames USB transitent sans réinterprétation vers CAN, sans écho USB.
FLASH_CTRL/FLASH_DATA destinés à screen passent dans `LOCAL_OTA` au lieu du
bus. Les trames CAN étendues et RTR ne sont pas du protocole et sont ignorées.
Les compteurs d'erreur sont accessibles ; le service lance une récupération
bus-off. Les files des miroirs sont bornées et perdent la trame récente si
l'observateur est trop lent, sans retarder les commandes de la machine.

Le propriétaire flash du cœur 1 utilise `flash::Flash`, les zones `Region`
et la table `Layout`. `esp-storage` n'active pas sa section critique : les
interruptions radio doivent continuer pendant les opérations ROM, comme
dans l'essai flash avec affichage. Le même propriétaire prête la flash à
NVS, OTA locale et au proxy ; aucun alias mutable ne traverse les cœurs.
`otadata` reprend l'implémentation qui respecte INVALID/ABORTED après rollback.

Le réseau reçoit l'image via `proxy_service::Upload::begin/write/commit`.
La taille est bornée par staging et par 1 Mo. Les fragments arbitraires sont
écrits en NOR par mots de quatre octets en préservant les octets voisins,
sans réeffacer le secteur. L'en-tête staging C++ de seize octets est écrit
en dernier, après calcul CRC32 ; les données commencent à l'octet 4096.

`send_staged` annonce BEGIN, puis BLOCK_START et les données par blocs de
2048 octets et trames de huit octets, espacées de 2 ms. Les ACK doivent venir
de sensors, avec numéro et CRC du bloc. BEGIN ignore le CRC de l'ACK comme
le C++. END exige l'ACK END. L'échéance fixe est de cinq secondes : un ACK
obsolète ne la prolonge pas. Un abandon distant fait échouer immédiatement
l'envoi. Trois essais par bloc sont autorisés seulement quand `replay_safe`
est vrai (capacité protocole chauffage de sensors), sinon un seul. L'échec
émet ABORT ; la progression et le résultat sont transmis vers l'UI.

Vérification sans matériel : cinq tests hôte ajoutés (format staging,
filtrage/routage, ACK/CRC, échéances et fragments NOR impairs) ; tests complets
et Clippy du crate `protocol` verts. `source ~/export-esp.sh`, puis
`cargo clippy --release --offline --lib -- -D warnings` et
`cargo build --release --offline` depuis `firmware/rust/screen` vérifient
les services sur Xtensa. Le linker conserve son avertissement RWX existant
pour le binaire écran ; aucun flash ni moniteur n'a été lancé.

L'assemblage et les essais physiques restent nécessaires : démarrage des
services, interconnexion HTTP/OTA locale/NVS, débit série, bus-off réel,
retransmission sur CAN et rollback. Les journaux texte et le cadrage série
partagent UART0 comme le firmware de référence. `serial_bridge::init_logger`
remplace le logger initial ; après lancement du transmetteur, les journaux
passent dans une file sans allocation en tas. Un seul propriétaire écrit
chaque journal/PDU en entier, et le zéro terminal des journaux recadre COBS.
Une file de journaux pleine abandonne le message plutôt que bloquer la radio.
Un test hôte vérifie que même un journal tronqué préserve la trame suivante.

# Cœur de l'écran

Port de la tâche 4, sans matériel, `no_std + alloc`. `protocol` fournit tous
les messages CAN. L'heure monotone en microsecondes est passée à chaque appel ;
l'heure murale reste `Option<i64>`.

Un propriétaire tient `Core<S: ConfigStore>`, injecte les `Input`, appelle
`tick` toutes les 50 ms et exécute les `Output` dans leur ordre. L'UI et le
réseau lisent `snapshot`, les handlers HTTP décident les réponses sans I/O.
Les services BLE réassemblent les notifications avec `acaia::Session` et
`scace::parse_notification`, puis transmettent les échantillons au cœur.
La rustdoc de `core` contient un exemple complet minimal.

`ConfigStore` attend la validation durable des deux blobs alternés avant de
répondre ; un adaptateur peut transmettre les opérations NVS au cœur 1 et
attendre son acquittement sans section critique. Aucune configuration n'est
publiée en RAM si la sauvegarde échoue. Les writes ne sont pas des sorties
optimistes : HTTP doit pouvoir répondre `500 storage` sans modifier l'état.

La lecture JSON utilise **serde + serde_json**, `default-features = false`,
features `alloc` et `float_roundtrip` (ce dernier garantit la relecture des
nombres cJSON). Un visiteur conserve l'ordre des clés et leurs doublons :
première occurrence pour une recherche, dernière lors d'un patch. Le writer
compact dédié conserve les décimales de `/config` et la règle `%1.15g` puis
`%1.17g` de cJSON ; les documents dorés vérifient les octets exacts.
La première valeur suivie de données et le BOM restent acceptés, comme cJSON.
Écart volontaire de validation : serde_json rejette les caractères de contrôle
non échappés, les nombres JSON mal formés et plus de 128 niveaux d'imbrication
(cJSON limitait à 1000) ; aucun document normal du service n'en dépend.

Les changements radio demandés sont appliqués : infusion avec Wi-Fi, aucun
start HTTP, toggle Wi-Fi de session non persisté, AP sans identifiants et BLE
coupé pendant le provisioning, scan 60/30 ms. Les trois nouveaux états radio
sont dans le snapshot ; ils n'ajoutent aucune clé à `/telemetry`.

Les cinq fichiers de tests C++ archivés sont portés bloc par bloc : machine,
thermique/précharge, résumé de tir, échéancier et scan BLE. S'ajoutent les
migrations et blobs NVS v1–v10 issus du C++, les tests du cœur/CAN/SCACE,
les handlers HTTP (validation atomique, auth, doublons, refus pendant cycles),
le protocole Acaia et les documents JSON produits par le vrai cJSON.

Validation hôte : `cargo +stable test -p screen-core`,
`cargo +stable clippy -p screen-core --all-targets -- -D warnings`.
Pas de flash, d'accès à la carte ni de validation du binaire Xtensa dans cette
tâche. Les tâches applicatives doivent encore intégrer ce cœur aux services.

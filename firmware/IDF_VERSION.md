# Version ESP-IDF figée

**ESP-IDF v6.1** (version stable courante au moment du bootstrap, septembre 2026).

Le choix initial s'était porté sur v5.1 comme LTS, mais cette LTS-là a
expiré (mai 2026) : elle n'aurait pas eu de sens à figer pour un projet qui
commence maintenant. On fige donc la stable courante, pas une LTS en fin de
vie. Le principe reste le même : une carte en boîte ne se met plus à jour
de toolchain, seulement d'application — on ne migre pas de version d'IDF en
cours de route. Voir `docs/firmware-implementation.md`, phase 0.

Installée via [`eim`](https://github.com/espressif/idf-im-cli) (ESP-IDF
Installation Manager), pas via un clone manuel :

```sh
eim install -t esp32s3 -i v6.1
eim select v6.1
```

Les deux projets (`sensors/`, `screen/`) ciblent `esp32s3`
(`idf.py set-target esp32s3`).

Référence API pour cette version, cette cible : <https://docs.espressif.com/projects/esp-idf/en/v6.1/esp32s3/api-reference/index.html>

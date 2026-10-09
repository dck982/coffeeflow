#!/bin/sh
# Sauvegarde, efface et restaure la partition `nvs` de l'écran, qui contient
# les identifiants Wi-Fi. La sauvegarde s'écrit hors du dépôt, en 600, et
# l'outil n'en affiche que la taille et le SHA-256 : le contenu ne passe
# jamais par la console. Voir docs/rust-migration.md, étape 8.
#
#   nvs-vault.sh save <fichier>      lit nvs dans <fichier> (qui ne doit pas exister)
#   nvs-vault.sh erase <fichier>     efface nvs, si <fichier> en est la copie exacte
#   nvs-vault.sh restore <fichier>   réécrit nvs depuis <fichier>, puis vérifie
#
# PORT choisit le port série (défaut : celui de l'écran).

set -eu

PORT=${PORT:-/dev/cu.usbmodem5B790235091}
REPO=$(cd "$(dirname "$0")/../.." && pwd -P)

die() {
  echo "nvs-vault: $*" >&2
  exit 1
}

usage() {
  sed -n '7,9s/^# *//p' "$0" >&2
  exit 2
}

# parttool.py vient d'ESP-IDF 6.1. Son script d'activation ne le met pas
# dans le PATH (c'est une fonction du shell) et refuse d'être sourcé depuis
# un script ; avec -e, il affiche l'environnement, une variable par ligne.
env_lines=$(sh ~/.espressif/tools/activate_idf_v6.1.sh -e 2>/dev/null) || die "ESP-IDF 6.1 introuvable"
idf_var() {
  printf '%s\n' "$env_lines" | sed -n "s/^$1=//p"
}
IDF_PATH=$(idf_var IDF_PATH)
IDF_PYTHON=$(idf_var IDF_PYTHON_ENV_PATH)/bin/python
[ -x "$IDF_PYTHON" ] && [ -f "$IDF_PATH/components/partition_table/parttool.py" ] ||
  die "parttool.py ou le Python d'ESP-IDF introuvable"
export IDF_PATH

parttool() {
  "$IDF_PYTHON" "$IDF_PATH/components/partition_table/parttool.py" -q -p "$PORT" "$@"
}

# Chemin absolu, dossier résolu (le fichier peut ne pas exister encore).
absolute() {
  dir=$(cd "$(dirname "$1")" 2>/dev/null && pwd -P) || die "dossier absent : $(dirname "$1")"
  echo "$dir/$(basename "$1")"
}

outside_repo() {
  case "$1" in
    "$REPO"/*) die "refusé : $1 est dans le dépôt" ;;
  esac
}

sha() {
  shasum -a 256 "$1" | cut -d' ' -f1
}

nvs_size() {
  parttool get_partition_info --partition-name nvs --info size 2>/dev/null |
    tail -n 1 || die "table de partitions illisible sur $PORT"
}

# Lit nvs dans le fichier donné, sans rien afficher de son contenu.
read_nvs() {
  parttool read_partition --partition-name nvs --output "$1" >/dev/null 2>&1 ||
    die "lecture de nvs impossible sur $PORT"
}

# Tout fichier créé ici est en 600. La relecture de nvs, pour comparaison,
# va dans un dossier temporaire effacé en sortie.
umask 077
TMPDIR_VAULT=$(mktemp -d)
trap 'rm -rf "$TMPDIR_VAULT"' EXIT INT TERM
current="$TMPDIR_VAULT/nvs.bin"

check_backup() {
  [ -f "$1" ] || die "sauvegarde absente : $1"
  expected=$(nvs_size)
  actual=$(wc -c <"$1" | tr -d ' ')
  [ "$((expected))" -eq "$actual" ] || die "taille $actual, nvs fait $((expected)) octets"
}

[ $# -eq 2 ] || usage
file=$(absolute "$2")
outside_repo "$file"

case "$1" in
  save)
    [ -e "$file" ] && die "refusé : $file existe déjà"
    read_nvs "$file"
    chmod 600 "$file"
    echo "nvs sauvegardé : $file"
    echo "taille $(wc -c <"$file" | tr -d ' ') octets, sha256 $(sha "$file")"
    ;;
  erase)
    check_backup "$file"
    read_nvs "$current"
    [ "$(sha "$current")" = "$(sha "$file")" ] ||
      die "refusé : $file n'est pas la copie du nvs actuel"
    parttool erase_partition --partition-name nvs >/dev/null 2>&1 ||
      die "effacement de nvs impossible"
    echo "nvs effacé ; sauvegarde vérifiée : sha256 $(sha "$file")"
    ;;
  restore)
    check_backup "$file"
    parttool write_partition --partition-name nvs --input "$file" >/dev/null 2>&1 ||
      die "écriture de nvs impossible"
    read_nvs "$current"
    [ "$(sha "$current")" = "$(sha "$file")" ] || die "relecture différente de $file"
    echo "nvs restauré et relu : sha256 $(sha "$file")"
    ;;
  *)
    usage
    ;;
esac

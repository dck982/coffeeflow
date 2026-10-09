"""Décode une zone NVS avec le parseur d'IDF et la compare aux données
factices de l'essai.

    python nvs/verifier.py generer <dossier>         CSV et image de départ
    python nvs/verifier.py avant|apres <zone.bin>    contrôle d'une zone relue

Le décodage passe par `nvs_parser` d'IDF (le module de `nvs_tool.py`),
puis `nvs_tool.py -i` contrôle l'intégrité. Pour `apres`, c'est la preuve
que les écritures de la Rust se relisent avec les outils d'IDF.
À lancer avec l'environnement d'IDF v6.1 activé.
"""

import os
import subprocess
import sys

import donnees

OUTIL = os.path.join(
    os.environ["IDF_PATH"], "components", "nvs_flash", "nvs_partition_tool"
)
sys.path.insert(0, OUTIL)

import nvs_parser  # noqa: E402

TAILLE = 0x6000
NUMERIQUES = {
    "uint8_t": "u8", "int8_t": "i8", "uint16_t": "u16", "int16_t": "i16",
    "uint32_t": "u32", "int32_t": "i32", "uint64_t": "u64", "int64_t": "i64",
}


def contenu(entree):
    """Données d'une entrée à longueur variable, sans le bourrage."""
    brut = b"".join(bytes(e.raw) for e in entree.children)
    return brut[: entree.data["size"]]


def decoder(zone):
    """{(espace, clé): valeur}, blobs recomposés selon leur index."""
    partition = nvs_parser.NVS_Partition("essai", bytearray(zone))
    ecrites = [
        e for p in partition.pages for e in p.entries if e.state == "Written"
    ]
    espaces = {
        e.data["value"]: e.key for e in ecrites if e.metadata["namespace"] == 0
    }
    valeurs, index, morceaux = {}, {}, {}
    for e in ecrites:
        ns = e.metadata["namespace"]
        if ns == 0:
            continue
        cle = (espaces[ns], e.key)
        genre = e.metadata["type"]
        if genre in NUMERIQUES:
            valeurs[cle] = (NUMERIQUES[genre], e.data["value"])
        elif genre == "string":
            valeurs[cle] = contenu(e).rstrip(b"\0").decode()
        elif genre == "blob":
            valeurs[cle] = contenu(e)
        elif genre == "blob_index":
            index[cle] = (e.data["chunk_start"], e.data["chunk_count"], e.data["size"])
        elif genre == "blob_data":
            morceaux.setdefault(cle, {})[e.metadata["chunk_index"]] = contenu(e)
    for cle, (debut, nombre, taille) in index.items():
        parts = morceaux.get(cle, {})
        manquants = [i for i in range(debut, debut + nombre) if i not in parts]
        if manquants:
            raise SystemExit(f"{cle}: morceaux {manquants} absents")
        blob = b"".join(parts[i] for i in range(debut, debut + nombre))
        if len(blob) != taille:
            raise SystemExit(f"{cle}: {len(blob)} octets, l'index en annonce {taille}")
        valeurs[cle] = blob
    return valeurs


def comparer(valeurs, attendu):
    ecarts = 0
    for cle in sorted(set(valeurs) | set(attendu)):
        lu, voulu = valeurs.get(cle), attendu.get(cle)
        if lu == voulu:
            print(f"ok     {cle[0]}/{cle[1]}")
            continue
        ecarts += 1
        print(f"ECART  {cle[0]}/{cle[1]} : lu {apercu(lu)}, attendu {apercu(voulu)}")
    return ecarts


def apercu(valeur):
    if isinstance(valeur, bytes):
        return f"{len(valeur)} octets {valeur[:8].hex()}…"
    return repr(valeur)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    action, chemin = sys.argv[1:]
    if action == "generer":
        donnees.csv(chemin)
        subprocess.run(
            [sys.executable, "-m", "esp_idf_nvs_partition_gen", "generate",
             f"{chemin}/essai.csv", f"{chemin}/essai.bin", hex(TAILLE)],
            check=True,
        )
        chemin = f"{chemin}/essai.bin"
        action = "avant"
    attendu = {"avant": donnees.AVANT, "apres": donnees.APRES}[action]
    with open(chemin, "rb") as f:
        zone = f.read()
    if len(zone) != TAILLE:
        raise SystemExit(f"{len(zone)} octets, {TAILLE} attendus")
    integrite = subprocess.run(
        [sys.executable, os.path.join(OUTIL, "nvs_tool.py"), "-i", "-d", "none", chemin],
        capture_output=True, text=True,
    )
    print(integrite.stdout.strip())
    ecarts = comparer(decoder(zone), attendu)
    if integrite.returncode != 0 or ecarts:
        raise SystemExit(f"échec : {ecarts} écart(s), intégrité {integrite.returncode}")
    print(f"zone conforme à « {action} »")


if __name__ == "__main__":
    main()

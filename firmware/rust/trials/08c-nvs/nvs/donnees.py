"""Données factices de l'essai 08c, les mêmes que `src/attendu.rs`.

Aucune ne vient du vrai NVS : les identifiants Wi-Fi sont faux, les blobs
de configuration ne sont que des motifs. Ce module écrit le CSV pour
`nvs_partition_gen` et décrit ce que la zone doit contenir avant et après
le passage de la Rust.
"""

import os


def motif(n, a, b):
    return bytes((i * a + b) & 0xFF for i in range(n))


def identifiants(ssid, mot_de_passe):
    """`Credentials` de net_wifi.cpp : ssid[33] puis password[64]."""
    return ssid.encode().ljust(33, b"\0") + mot_de_passe.encode().ljust(64, b"\0")


AVANT = {
    ("ui", "a"): motif(120, 7, 1),
    ("ui", "b"): motif(120, 13, 5),
    ("wifi", "credentials"): identifiants("essai-ssid", "faux-mot-de-passe"),
    ("essai", "compteur"): ("u32", 123456),
    ("essai", "signe"): ("i32", -42),
    ("essai", "octet"): ("u8", 7),
    ("essai", "nom"): "coffeeflow-essai",
    ("essai", "gros"): motif(6000, 31, 3),
}

# Ce que la Rust change : une valeur modifiée, un blob réécrit, une clé
# effacée, et des types que coffeeflow n'utilise pas encore.
APRES = dict(AVANT)
APRES[("essai", "compteur")] = ("u32", 654321)
APRES[("ui", "b")] = motif(120, 11, 2)
del APRES[("essai", "signe")]
APRES[("essai", "nouveau")] = "ecrit par rust"
APRES[("essai", "rust_blob")] = motif(300, 3, 1)
APRES[("essai", "grand")] = ("u64", 0x0123456789ABCDEF)
APRES[("essai", "petit")] = ("i8", -5)


def csv(dossier):
    """Écrit `essai.csv` et le fichier du gros blob dans `dossier`."""
    os.makedirs(dossier, exist_ok=True)
    lignes = ["key,type,encoding,value"]
    courant = None
    for (ns, cle), valeur in AVANT.items():
        if ns != courant:
            lignes.append(f"{ns},namespace,,")
            courant = ns
        if isinstance(valeur, bytes):
            if len(valeur) > 1000:
                chemin = f"{dossier}/{cle}.bin"
                with open(chemin, "wb") as f:
                    f.write(valeur)
                lignes.append(f"{cle},file,binary,{chemin}")
            else:
                lignes.append(f"{cle},data,hex2bin,{valeur.hex()}")
        elif isinstance(valeur, str):
            lignes.append(f"{cle},data,string,{valeur}")
        else:
            type_, nombre = valeur
            lignes.append(f"{cle},data,{type_},{nombre}")
    with open(f"{dossier}/essai.csv", "w") as f:
        f.write("\n".join(lignes) + "\n")

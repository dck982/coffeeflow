# Outils du dépôt

## Browser-use

`browser-use` est installé à la demande avec `uv`, pas exposé directement dans
le `PATH`. Lancer la CLI ainsi depuis la racine du dépôt :

```sh
uv run --with browser-use browser-use --doctor
printf '%s\n' 'new_tab("file:///chemin/absolu/rapport.html")' 'wait_for_load()' 'print(page_info())' | uv run --with browser-use browser-use
```

Cette version (`browser-harness`) reçoit ses commandes Python sur l'entrée
standard ; ne pas supposer que les sous-commandes `open` et `state` existent.

## Secrets

Ne jamais lire directement `secrets.h`, `secrets.rs` ni un autre fichier de
secrets, ni afficher leurs valeurs dans les sorties d'outils. Une conversion
locale explicitement demandée peut passer par un script qui lit et écrit ces
fichiers sans en exposer le contenu. Les fichiers de secrets restent ignorés
par Git ; seuls les fichiers d'exemple peuvent être consultés et versionnés.

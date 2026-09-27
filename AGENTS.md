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

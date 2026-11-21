# Arborescence des commandes

Le plugin enregistre la commande racine `/infected`, avec `/inf` comme alias.

```text
/infected (alias : /inf)
├── join
└── launch
```

## Commandes

| Commande | Permission | Fonction |
| --- | --- | --- |
| `/infected` ou `/inf` | `infected::join` (permission donnée à l'enregistrement de la commande) | Affiche l'aide d'utilisation. |
| `/infected join` ou `/inf join` | `infected::join` | Téléporte le joueur dans le monde courant choisi depuis `config.yml`, aux coordonnées `spawn` configurées. Ne fait rien si le joueur s'y trouve déjà. |
| `/infected launch` ou `/inf launch` | `infected::admin` vérifiée par le gestionnaire de la sous-commande | Passe la session à l'état `Active`, affiche le décompte aux joueurs du monde courant, puis annonce le lancement. |

Les sous-commandes doivent être exécutées par un joueur. Le plugin crée `config.yml` dans son dossier de données privé au premier chargement. Les mondes configurés y sont chargés ou créés, et le monde courant est sélectionné aléatoirement au démarrage.

## Organisation du code

- `src/lib.rs` construit la commande racine, ses alias et ses sous-commandes, puis les enregistre.
- `src/commands/infected.rs` gère l'aide de la commande racine.
- `src/commands/join.rs` gère l'entrée dans le monde du mini-jeu.
- `src/commands/launch.rs` démarre la session et diffuse l'annonce de lancement.

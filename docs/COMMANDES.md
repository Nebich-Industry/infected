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
| `/infected join` ou `/inf join` | `infected::join` | Téléporte le joueur dans le monde `infected_world`, à son point d'apparition. Ne fait rien si le joueur s'y trouve déjà. |
| `/infected launch` ou `/inf launch` | `infected::admin` vérifiée par le gestionnaire de la sous-commande | Passe la session de jeu à l'état `Active` et annonce le lancement à tous les joueurs. |

Les sous-commandes doivent être exécutées par un joueur. La commande `join` requiert que le monde `infected_world` existe.

## Organisation du code

- `src/lib.rs` construit la commande racine, ses alias et ses sous-commandes, puis les enregistre.
- `src/commands/infected.rs` gère l'aide de la commande racine.
- `src/commands/join.rs` gère l'entrée dans le monde du mini-jeu.
- `src/commands/launch.rs` démarre la session et diffuse l'annonce de lancement.

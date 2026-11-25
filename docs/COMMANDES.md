# Arborescence des commandes

Le plugin enregistre la commande racine `/infected`, avec `/inf` comme alias.

```text
/infected (alias : /inf)
├── join
├── class
│   ├── archer
│   ├── doctor
│   └── ninja
├── admin
│   ├── launch
│   └── transform
└── launch (compatibilité)
```

## Commandes

| Commande | Permission | Fonction |
| --- | --- | --- |
| `/infected` ou `/inf` | `Infected::join` (permission donnée à l'enregistrement de la commande) | Affiche l'aide d'utilisation. |
| `/infected join` ou `/inf join` | `Infected::join` | Téléporte le joueur dans le monde courant choisi depuis `config.yml`, aux coordonnées `spawn` configurées, et l'enregistre comme survivant ou zombie selon l'état de la partie. S'il s'y trouve déjà, vérifie seulement son enregistrement. |
| `/infected class <archer\|doctor\|ninja>` | `Infected::join` | Choisit la classe du survivant. |
| `/infected admin launch` ou `/inf admin launch` | `Infected::admin` | Affiche un décompte de 5 secondes, puis démarre la partie. `/infected launch` reste disponible comme alias historique. |
| `/infected admin transform` ou `/inf admin transform` | `Infected::admin` | Transforme l'administrateur en zombie pendant une partie en cours. |

Les commandes de jeu doivent être exécutées par un joueur. Les joueurs rejoignent en mode aventure. Une partie démarre automatiquement après deux minutes si le monde courant contient au moins trois joueurs ; sinon, l'attente est prolongée de 30 secondes. Les survivants reçoivent une épée, un arc et cinq flèches au lancement, puis une flèche toutes les 20 secondes. Une partie s'achève après cinq minutes ou lorsqu'il ne reste plus de survivants.

Pendant une partie, un bonus pour survivants et un bonus d'invisibilité réservé aux zombies apparaissent toutes les 20 secondes dans les quatre premières minutes. Les bonus sont ramassés en interagissant avec leur affichage : soin (4 PV et vitesse pendant 5 secondes), instakill (10 secondes), fumigène (à lancer comme une boule de neige) et invisibilité zombie (5 secondes). Le fumigène aveugle les joueurs à proximité, sauf le lanceur et les ninjas survivants, qui gagnent vitesse et invisibilité.

Le sidebar « Infected » affiche le nombre de survivants et de zombies, mis à jour lors des inscriptions, transformations et départs.

Le plugin crée `config.yml` dans son dossier de données privé au premier chargement. Les mondes configurés y sont chargés ou créés, et le monde courant est sélectionné aléatoirement au démarrage.

## Organisation du code

- `src/lib.rs` construit la commande racine, ses alias et ses sous-commandes, puis les enregistre.
- `src/commands/infected.rs` gère l'aide de la commande racine.
- `src/commands/join.rs` gère l'entrée dans le monde du mini-jeu.
- `src/commands/class.rs` permet au survivant de choisir sa classe.
- `src/commands/launch.rs` gère le démarrage manuel et automatique de la session.
- `src/commands/transform.rs` gère la transformation administrative en zombie.
- `src/management/player.rs` suit les rôles et les transformations des joueurs et traite les événements de mort et de dégâts Pumpkin.
- `src/management/bonus.rs` fait apparaître les bonus, gère leur collecte et applique leurs effets.
- `src/management/scoreboard.rs` affiche le nombre de survivants et de zombies dans le tableau latéral.

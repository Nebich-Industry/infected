# Plugin Infected

## Installation

### Gestion des mondes

Au premier chargement, le plugin crée `config.yml` dans son dossier de données privé. Il charge chaque monde déclaré sous `worlds` ou le crée en dimension Overworld s'il n'existe pas, configure sa bordure, puis choisit un monde courant aléatoire.

```yaml
worlds:
  world_infected_city:
    spawn:
      x: 2.0
      y: -59.0
      z: 13.0
    border:
      center:
        x: 2.0
        y: -59.0
        z: 13.0
      radius: 200.0
```

Les coordonnées `spawn` déterminent le point d'arrivée de `/infected join`; `border.center` et `border.radius` configurent le centre et la taille (diamètre) de la bordure.

## Développement

Pour lancer des tests unitaires en local il faut executer la commande :

```bash
cargo test --target x86_64-pc-windows-msvc
```
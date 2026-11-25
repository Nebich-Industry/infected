# Plugin Infected

## Licence

Ce projet est distribué sous licence MIT. Voir le fichier [LICENSE](LICENSE).

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

### Lancer Pumpkin avec Docker

Les données du serveur sont conservées dans `dev-server/`, dans le projet. Pour construire le plugin et le charger au démarrage du serveur, exécutez ces commandes depuis la racine du dépôt avec PowerShell :

```powershell
cargo build --release
New-Item -ItemType Directory -Force dev-server\plugins | Out-Null
Copy-Item target\wasm32-wasip2\release\infected.wasm dev-server\plugins\infected.wasm -Force
docker compose up -d
docker compose logs -f pumpkin
```

Pumpkin charge les plugins `.wasm` placés dans `dev-server/plugins/`. Après une modification du plugin, reconstruisez-le, recopiez le fichier et redémarrez le serveur avec `docker compose restart pumpkin`.

Pour lancer des tests unitaires en local il faut executer la commande :

```bash
cargo test --target x86_64-pc-windows-msvc
```

### Hook Git pre-push

Le hook `.githooks/pre-push` vérifie le formatage, construit la version release et lance Clippy avant chaque push. Pour l'activer dans ce clone :

```bash
git config core.hooksPath .githooks
```
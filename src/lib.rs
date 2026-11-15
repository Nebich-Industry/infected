mod commandes;
use crate::commandes::join::JoinExecutor;
use pumpkin_plugin_api::{Plugin, PluginMetadata, command::Command, permission::{Permission, PermissionDefault}, Context};
use tracing::info;

struct InfectedPlugin;
impl Plugin for InfectedPlugin {
    fn new() -> Self {
        InfectedPlugin
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Infected".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Nebich_".into()],
            description: "Un plugin développé avec amour pour jouer à un mini jeu de survie à une invasion de zombies.".into(),
            dependencies: vec![],
            permissions: vec![],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        // 1. Enregistrement des permissions
        context.register_permission(&Permission {
            node: "infected::join".to_string(),
            description: "Permet au joueur de rejoindre le mini jeu Infected.".to_string(),
            default: PermissionDefault::Allow,
            children: Vec::new(),
        })?;

        // 2. Construction des commandes avec alias et executeur
        let join_names = vec!["join".to_string(), "j".to_string()];
        let join_command = Command::new(&join_names, "Permet au joueur de rejoindre le mini jeu Infected.")
            .execute(JoinExecutor);

        // 3. Enregistrement des commandes
        context.register_command(join_command, "infected::join");

        info!("Chargement du plugin Infected réussi !");
        Ok(())
    }

    fn on_unload(&self, _context: pumpkin_plugin_api::Context) -> pumpkin_plugin_api::Result<()> {
        info!("Déchargement du plugin Infected.");
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(InfectedPlugin);
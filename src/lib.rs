mod commands;
mod management;
use crate::commands::{infected::InfectedExecutor, join::JoinExecutor, launch::LaunchExecutor};
use pumpkin_plugin_api::{
    Context, Plugin, PluginMetadata,
    command::{Command, CommandNode},
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::FS_WRITE_DATA,
};
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
            permissions: vec![FS_WRITE_DATA.to_string()],
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

        context.register_permission(&Permission {
            node: "infected::admin".to_string(),
            description:
                "Permet au joueur d'accéder aux commandes administratives du mini jeu Infected."
                    .to_string(),
            default: PermissionDefault::Op(PermissionLevel::Three),
            children: Vec::new(),
        })?;

        crate::management::world::load_worlds(&context)?;

        // 2. Construction de la commande infected avec sous-commandes
        let infected_aliases = vec!["infected".to_string(), "inf".to_string()];
        let infected_command = Command::new(&infected_aliases, "Commandes du plugin Infected.")
            .execute(InfectedExecutor)
            .then(CommandNode::literal("join").execute(JoinExecutor))
            .then(CommandNode::literal("launch").execute(LaunchExecutor));

        // 3. Enregistrement de la commande
        context.register_command(infected_command, "infected::join");

        info!("Chargement du plugin Infected réussi !");
        Ok(())
    }

    fn on_unload(&self, _context: pumpkin_plugin_api::Context) -> pumpkin_plugin_api::Result<()> {
        info!("Déchargement du plugin Infected.");
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(InfectedPlugin);

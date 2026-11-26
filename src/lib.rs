mod commands;
mod management;
use crate::commands::{
    admin::AdminExecutor, class::SelectClassExecutor, infected::InfectedExecutor,
    join::JoinExecutor, launch::LaunchExecutor, transform::TransformExecutor,
};
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
            node: "Infected::join".to_string(),
            description: "Permet au joueur de rejoindre le mini jeu Infected.".to_string(),
            default: PermissionDefault::Allow,
            children: Vec::new(),
        })?;

        context.register_permission(&Permission {
            node: "Infected::admin".to_string(),
            description:
                "Permet au joueur d'accéder aux commandes administratives du mini jeu Infected."
                    .to_string(),
            default: PermissionDefault::Op(PermissionLevel::Three),
            children: Vec::new(),
        })?;

        crate::management::world::load_worlds(&context)?;
        crate::management::player::register(&context).map_err(|error| {
            format!("Chargement du gestionnaire de joueurs impossible : {error}")
        })?;
        crate::management::scoreboard::initialize(&context.get_server()).map_err(|error| {
            format!("Initialisation du tableau d'affichage impossible : {error}")
        })?;
        crate::management::bonus::register(&context)
            .map_err(|error| format!("Chargement du gestionnaire de bonus impossible : {error}"))?;
        crate::commands::launch::start_waiting_task();

        // 2. Construction de la commande infected avec sous-commandes
        let infected_aliases = vec!["infected".to_string(), "inf".to_string()];
        let infected_command = Command::new(&infected_aliases, "Commandes du plugin Infected.")
            .execute(InfectedExecutor)
            .then(CommandNode::literal("join").execute(JoinExecutor))
            .then(
                CommandNode::literal("class")
                    .then(CommandNode::literal("archer").execute(SelectClassExecutor(
                        crate::management::player::SurvivorRole::Archer,
                    )))
                    .then(CommandNode::literal("doctor").execute(SelectClassExecutor(
                        crate::management::player::SurvivorRole::Doctor,
                    )))
                    .then(CommandNode::literal("ninja").execute(SelectClassExecutor(
                        crate::management::player::SurvivorRole::Ninja,
                    ))),
            )
            .then(
                CommandNode::literal("admin")
                    .execute(AdminExecutor)
                    .then(CommandNode::literal("launch").execute(LaunchExecutor))
                    .then(CommandNode::literal("transform").execute(TransformExecutor)),
            );

        // 3. Enregistrement de la commande
        context.register_command(infected_command, "Infected::join");

        info!("Chargement du plugin Infected réussi !");
        Ok(())
    }

    fn on_unload(&self, _context: pumpkin_plugin_api::Context) -> pumpkin_plugin_api::Result<()> {
        info!("Déchargement du plugin Infected.");
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(InfectedPlugin);

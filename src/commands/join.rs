use pumpkin_plugin_api::{
    Server,
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    text::TextComponent,
};

use crate::management::world::current_world;

fn is_already_in_world(current_world: &str, target_world: &str) -> bool {
    current_world == target_world
}

pub struct JoinExecutor;
impl CommandHandler for JoinExecutor {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = sender
            .as_player()
            .ok_or_else(|| CommandError::InvalidRequirement)?;

        if !player.has_permission("infected::join") {
            sender.send_message(TextComponent::text(
                "Vous n'avez pas la permission de rejoindre le mini jeu Infected.",
            ));
            return Ok(0);
        }

        let (world, config) = match current_world(&server) {
            Ok(world) => world,
            Err(error) => {
                sender.send_message(TextComponent::text(&error));
                return Ok(0);
            }
        };

        // Vérifie si le joueur est déjà dans le monde
        if is_already_in_world(&player.get_world().get_name(), &world.get_name()) {
            sender.send_message(TextComponent::text("Vous êtes déjà dans le monde !"));
            return Ok(0);
        }

        // Ajoute le joueur au monde
        player.teleport_world(
            world,
            (config.spawn.x, config.spawn.y, config.spawn.z),
            Some(player.get_yaw()),
            Some(player.get_pitch()),
        );
        sender.send_message(TextComponent::text(
            "Vous avez rejoint le monde avec succès !",
        ));
        Ok(1)
    }
}

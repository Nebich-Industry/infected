use pumpkin_plugin_api::{Server, command::{CommandError, CommandSender, ConsumedArgs}, commands::CommandHandler, text::TextComponent};

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
        let player = sender.as_player().ok_or_else(|| CommandError::InvalidRequirement)?;
        let world = server.get_all_worlds()
            .into_iter()
            .find(|w| w.get_name() == "infected_world")
            .ok_or_else(|| CommandError::InvalidRequirement)?;

        // Vérifie si le joueur est déjà dans le monde
        if is_already_in_world(&player.get_world().get_name(), &world.get_name()) {
            sender.send_message(TextComponent::text("Vous êtes déjà dans le monde !"));
            return Ok(0);
        }

        // Ajoute le joueur au monde
        let world_spawn = world.get_spawn_location().pos;
        player.teleport_world(world, (world_spawn.x.into(), world_spawn.y.into(), world_spawn.z.into()), Some(player.get_yaw()), Some(player.get_pitch()));
        sender.send_message(TextComponent::text("Vous avez rejoint le monde avec succès !"));
        Ok(1)
    }
}
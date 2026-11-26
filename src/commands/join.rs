use pumpkin_plugin_api::{
    Server,
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    text::TextComponent,
    wit::pumpkin::plugin::common::GameMode,
};

use crate::management::world::{current_world, safe_spawn_position};

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

        if !player.has_permission("Infected::join") {
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

        if !player.set_gamemode(GameMode::Adventure) {
            sender.send_message(TextComponent::text(
                "Impossible de passer en mode aventure.",
            ));
        }

        // Vérifie si le joueur est déjà dans le monde
        if is_already_in_world(&player.get_world().get_name(), &world.get_name()) {
            if let Err(error) = crate::management::player::add_player(&player) {
                sender.send_message(TextComponent::text(&error));
                return Ok(0);
            }
            sender.send_message(TextComponent::text("Vous êtes déjà dans le monde !"));
            return Ok(0);
        }

        let spawn = match safe_spawn_position(&world, &config.spawn) {
            Ok(spawn) => spawn,
            Err(error) => {
                sender.send_message(TextComponent::text(&error));
                return Ok(0);
            }
        };

        // Ajoute le joueur au monde
        player.teleport_world(
            world,
            spawn,
            Some(player.get_yaw()),
            Some(player.get_pitch()),
        );
        if let Err(error) = crate::management::player::add_player(&player) {
            sender.send_message(TextComponent::text(&error));
            return Ok(0);
        }
        sender.send_message(TextComponent::text(
            "Vous avez rejoint le monde avec succès ! Choisissez votre classe avec /infected class.",
        ));
        Ok(1)
    }
}

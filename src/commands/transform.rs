use pumpkin_plugin_api::{
    Server,
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    text::TextComponent,
};

use crate::management::{
    player::get_player,
    session::{GameState, game_session},
};

pub struct TransformExecutor;

impl CommandHandler for TransformExecutor {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = sender
            .as_player()
            .ok_or_else(|| CommandError::InvalidRequirement)?;

        if !player.has_permission("Infected::admin") {
            sender.send_message(TextComponent::text(
                "Vous n'avez pas la permission d'exécuter cette commande.",
            ));
            return Ok(0);
        }

        let is_active = game_session()
            .lock()
            .map_err(|error| {
                CommandError::CommandFailed(TextComponent::text(&format!(
                    "Impossible de verrouiller la session de jeu : {error}"
                )))
            })?
            .state()
            == &GameState::Active;
        if !is_active {
            sender.send_message(TextComponent::text(
                "Vous ne pouvez pas vous transformer en zombie car la partie n'a pas encore commencé.",
            ));
            return Ok(0);
        }

        match get_player(&player) {
            Ok(Some(infected_player)) if infected_player.is_survivor() => {
                if let Err(error) = crate::management::player::transform_in_zombie(&player) {
                    sender.send_message(TextComponent::text(&error));
                    return Ok(0);
                }
                sender.send_message(TextComponent::text(
                    "Vous venez de vous transformer en zombie.",
                ));
                Ok(1)
            }
            Ok(Some(_)) => {
                sender.send_message(TextComponent::text("Vous êtes déjà un zombie."));
                Ok(0)
            }
            Ok(None) => {
                sender.send_message(TextComponent::text(
                    "Vous devez rejoindre le mini jeu avant de vous transformer.",
                ));
                Ok(0)
            }
            Err(error) => {
                sender.send_message(TextComponent::text(&error));
                Ok(0)
            }
        }
    }
}

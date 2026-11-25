use pumpkin_plugin_api::{
    Server,
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    text::TextComponent,
};

use crate::management::player::SurvivorRole;

pub struct SelectClassExecutor(pub SurvivorRole);

impl CommandHandler for SelectClassExecutor {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = sender
            .as_player()
            .ok_or_else(|| CommandError::InvalidRequirement)?;
        let role_name = match self.0 {
            SurvivorRole::Archer => "Archer",
            SurvivorRole::Doctor => "Docteur",
            SurvivorRole::Ninja => "Ninja",
        };

        match crate::management::player::set_survivor_role(&player, self.0) {
            Ok(()) => {
                sender.send_message(TextComponent::text(&format!(
                    "Vous avez choisi la classe {role_name}."
                )));
                Ok(1)
            }
            Err(error) => {
                sender.send_message(TextComponent::text(&error));
                Ok(0)
            }
        }
    }
}

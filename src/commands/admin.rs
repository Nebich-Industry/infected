use pumpkin_plugin_api::{
    Server,
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    text::TextComponent,
};

pub struct AdminExecutor;

impl CommandHandler for AdminExecutor {
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
                "Vous n'avez pas la permission d'accéder aux commandes administratives.",
            ));
            return Ok(0);
        }

        sender.send_message(TextComponent::text(
            "Utilisation: /infected admin <launch|transform>",
        ));
        Ok(0)
    }
}

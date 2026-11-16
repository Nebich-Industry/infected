use pumpkin_plugin_api::{Server, command::{CommandError, CommandSender, ConsumedArgs}, commands::CommandHandler, text::TextComponent};

pub struct InfectedExecutor;
impl CommandHandler for InfectedExecutor {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        sender.send_message(TextComponent::text("Utilisation: /infected <command>"));
        sender.send_message(TextComponent::text("Commandes disponibles: join"));
        return Ok(0);
    }
}
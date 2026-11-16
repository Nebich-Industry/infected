use pumpkin_plugin_api::{Server, command::{CommandError, CommandSender, ConsumedArgs}, commands::CommandHandler, text::TextComponent};

use crate::management::session::{GameState, game_session};

pub struct LaunchExecutor;
impl CommandHandler for LaunchExecutor {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = sender.as_player().ok_or_else(|| CommandError::InvalidRequirement)?;

        if !player.has_permission("infected::admin") {
            sender.send_message(TextComponent::text("Vous n'avez pas la permission d'exécuter cette commande."));
            return Ok(0);
        }

        // Permet de changer l'état du jeu à "Active" pour indiquer que le mini-jeu a commencé
        // C'est dans un bloc pour s'assurer que le verrou concernant la session est libéré après utilisation
        // pour éviter les problèmes de concurrence et de blocage.
        {
            let mut session = game_session().lock().expect("Echec du verrouillage de la session de jeu");
            session.set_state(GameState::Active);
        }

        server.broadcast("Le mini jeu Infected a été lancé ! Bonne chance à tous les joueurs !");
        Ok(1)
    }
}
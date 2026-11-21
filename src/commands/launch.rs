use pumpkin_plugin_api::{Server, command::{CommandError, CommandSender, ConsumedArgs}, commands::CommandHandler, player::{Sound, SoundCategory}, text::TextComponent};

use crate::management::{session::{GameState, game_session}, world::world_name};

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

        // Affiche les nombres dans l'ordre 5 à 0 aux joueurs dans le monde "infected" et joue un son pour chaque nombre
        let players = server.get_players_in_world(
            server.get_world_by_name(world_name().lock().expect("Echec du verrouillage du nom du monde").as_str()).ok_or_else(|| CommandError::InvalidRequirement)?
        );
        for seconds in (0..=5).rev() {
            for player in players.iter().clone() {
                player.show_title(TextComponent::text(&seconds.to_string()));
                player.play_sound(Sound::BlockNoteBlockHarp, SoundCategory::Master, 100.0, 1.0);
            }
        }

        for player in players {
            player.show_title(TextComponent::text("Lancement de la partie"));
            player.show_subtitle(TextComponent::text("Bonne chance à tous !"));
        }

        server.broadcast("Le mini jeu Infected a été lancé ! Bonne chance à tous les joueurs !");
        Ok(1)
    }
}

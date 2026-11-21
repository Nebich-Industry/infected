use pumpkin_plugin_api::{
    Server,
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    player::{Sound, SoundCategory},
    scheduler::schedule_delayed_task,
    text::TextComponent,
};

use crate::management::{
    session::{GameState, game_session},
    world::current_world,
};

fn finish_countdown(seconds: u32, broadcast: impl FnOnce()) -> bool {
    if seconds != 0 {
        return false;
    }

    broadcast();
    true
}

// Execution du compte à rebours avant le lancement du mini-jeu Infected
// Tant que les secondes ne sont pas écoulées, on affiche le nombre de secondes restantes
// à tous les joueurs du monde sur lequel le mini-jeu est en cours.
fn schedule_countdown(world_name: String, seconds: u32, delay_ticks: u64) {
    schedule_delayed_task(delay_ticks, move |server| {
        let Some(world) = server.get_world_by_name(&world_name) else {
            tracing::error!("Le monde configuré {world_name} n'est plus chargé.");
            return;
        };
        let players = server.get_players_in_world(world);

        for player in &players {
            player.show_title(TextComponent::text(&seconds.to_string()));
            player.play_sound(Sound::BlockNoteBlockHarp, SoundCategory::Master, 100.0, 1.0);
        }

        if !finish_countdown(seconds, || {
            server
                .broadcast("Le mini jeu Infected a été lancé ! Bonne chance à tous les joueurs !");
        }) {
            schedule_countdown(world_name.clone(), seconds - 1, 20);
        }
    });
}

pub struct LaunchExecutor;
impl CommandHandler for LaunchExecutor {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = sender
            .as_player()
            .ok_or_else(|| CommandError::InvalidRequirement)?;

        if !player.has_permission("infected::admin") {
            sender.send_message(TextComponent::text(
                "Vous n'avez pas la permission d'exécuter cette commande.",
            ));
            return Ok(0);
        }

        // Récupérationn du monde configuré dans config.yml
        let (world, _) = match current_world(&server) {
            Ok(world) => world,
            Err(error) => {
                sender.send_message(TextComponent::text(&error));
                return Ok(0);
            }
        };

        // Permet de changer l'état du jeu à "Active" pour indiquer que le mini-jeu a commencé
        // C'est dans un bloc pour s'assurer que le verrou concernant la session est libéré après utilisation
        // pour éviter les problèmes de concurrence et de blocage.
        {
            let mut session = game_session()
                .lock()
                .expect("Echec du verrouillage de la session de jeu");
            session.set_state(GameState::Active);
        }

        schedule_countdown(world.get_name(), 5, 0);
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::finish_countdown;

    #[test]
    fn broadcasts_when_countdown_reaches_zero() {
        let mut broadcast_called = false;

        let finished = finish_countdown(0, || broadcast_called = true);

        assert!(finished);
        assert!(broadcast_called);
    }
}

use pumpkin_plugin_api::{
    Server,
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    player::SoundCategory,
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
fn cancel_pending_launch() {
    match game_session().lock() {
        Ok(mut session) => session.cancel_launch_countdown(),
        Err(error) => tracing::error!("Verrouillage de la session de jeu impossible : {error}"),
    }
}

fn postpone_waiting_launch() {
    match game_session().lock() {
        Ok(mut session) => session.reset_waiting_timer(30),
        Err(error) => tracing::error!("Verrouillage de la session de jeu impossible : {error}"),
    }
}

fn restore_zombies_after_game(server: &Server) {
    crate::management::bonus::clear(server);
    if let Err(error) = crate::management::player::reset_zombies_to_survivors(server) {
        tracing::error!("Impossible de rétablir les zombies en survivants : {error}");
    }
}

fn schedule_countdown(world_name: String, seconds: u32, delay_ticks: u64) {
    schedule_delayed_task(delay_ticks, move |server| {
        let Some(world) = server.get_world_by_name(&world_name) else {
            tracing::error!("Le monde configuré {world_name} n'est plus chargé.");
            cancel_pending_launch();
            return;
        };
        let players = server.get_players_in_world(world);

        for player in &players {
            player.show_title(TextComponent::text(&seconds.to_string()));
            player.play_custom_sound(
                "minecraft:block.note_block.harp",
                SoundCategory::Master,
                100.0,
                1.0,
            );
        }

        if !finish_countdown(seconds, || match start_game(&server) {
            Ok(()) => server
                .broadcast("Le mini jeu Infected a été lancé ! Bonne chance à tous les joueurs !"),
            Err(error) => {
                tracing::error!("Impossible de lancer le mini jeu Infected : {error}");
                cancel_pending_launch();
            }
        }) {
            schedule_countdown(world_name.clone(), seconds - 1, 20);
        }
    });
}

fn start_game(server: &Server) -> Result<(), String> {
    if game_session()
        .lock()
        .map_err(|error| format!("Verrouillage de la session de jeu impossible : {error}"))?
        .state()
        == &GameState::Active
    {
        return Err("Une partie Infected est déjà en cours.".to_string());
    }

    crate::management::player::register_current_world_players(server)?;
    crate::management::player::choose_first_zombies(server)?;

    crate::management::bonus::clear(server);
    crate::management::player::give_starter_items(server)?;
    {
        let mut session = game_session()
            .lock()
            .map_err(|error| format!("Verrouillage de la session de jeu impossible : {error}"))?;
        session.launch();
    }

    schedule_game_timer_tick();
    schedule_survivor_arrow_tick();
    crate::management::bonus::start_spawn_task();
    crate::management::scoreboard::refresh_for_server(server);
    Ok(())
}

pub fn start_waiting_task() {
    schedule_waiting_tick();
}

fn schedule_waiting_tick() {
    schedule_delayed_task(20, |server| {
        let should_launch = match game_session().lock() {
            Ok(mut session)
                if session.state() != &GameState::Active && !session.launch_pending() =>
            {
                if session.state() == &GameState::Ended {
                    session.end();
                }
                if session.waiting_timer() == 0 {
                    true
                } else {
                    session.tick_waiting_timer()
                }
            }
            Ok(_) => false,
            Err(error) => {
                tracing::error!("Verrouillage de la session de jeu impossible : {error}");
                schedule_waiting_tick();
                return;
            }
        };

        if should_launch {
            let enough_players = match current_world(&server) {
                Ok((world, _)) => server.get_players_in_world(world).len() > 2,
                Err(error) => {
                    tracing::error!(
                        "Impossible de vérifier les joueurs avant le lancement : {error}"
                    );
                    false
                }
            };

            if enough_players {
                if let Err(error) = start_game(&server) {
                    tracing::error!("Impossible de lancer automatiquement la partie : {error}");
                    postpone_waiting_launch();
                } else {
                    server.broadcast("Le mini jeu Infected a été lancé automatiquement !");
                }
            } else {
                postpone_waiting_launch();
            }
        }

        schedule_waiting_tick();
    });
}

fn schedule_game_timer_tick() {
    schedule_delayed_task(20, |server| {
        let game_finished = match game_session().lock() {
            Ok(mut session) if session.state() == &GameState::Active => {
                session.decrement_game_timer();
                if session.game_timer() == 0 {
                    session.set_state(GameState::Ended);
                    true
                } else {
                    false
                }
            }
            Ok(_) => return,
            Err(error) => {
                tracing::error!("Verrouillage de la session de jeu impossible : {error}");
                return;
            }
        };

        if game_finished {
            restore_zombies_after_game(&server);
            server.broadcast("La partie Infected est terminée !");
            return;
        }

        match crate::management::player::has_survivors(&server) {
            Ok(false) => {
                match game_session().lock() {
                    Ok(mut session) => session.set_state(GameState::Ended),
                    Err(error) => {
                        tracing::error!("Verrouillage de la session de jeu impossible : {error}")
                    }
                }
                restore_zombies_after_game(&server);
                server.broadcast("Tous les survivants ont été infectés. La partie est terminée !");
            }
            Ok(true) => schedule_game_timer_tick(),
            Err(error) => {
                tracing::error!("Impossible de vérifier les survivants : {error}");
                schedule_game_timer_tick();
            }
        }
    });
}

fn schedule_survivor_arrow_tick() {
    schedule_delayed_task(20 * 20, |server| {
        let is_active = match game_session().lock() {
            Ok(session) => session.state() == &GameState::Active,
            Err(error) => {
                tracing::error!("Verrouillage de la session de jeu impossible : {error}");
                return;
            }
        };
        if !is_active {
            return;
        }

        if let Err(error) = crate::management::player::give_survivor_arrows(&server) {
            tracing::error!("Impossible de distribuer les flèches aux survivants : {error}");
        }
        schedule_survivor_arrow_tick();
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

        if !player.has_permission("Infected::admin") {
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

        {
            let mut session = match game_session().lock() {
                Ok(session) => session,
                Err(error) => {
                    sender.send_message(TextComponent::text(&format!(
                        "Impossible de verrouiller la session de jeu : {error}"
                    )));
                    return Ok(0);
                }
            };
            if !session.begin_launch_countdown() {
                sender.send_message(TextComponent::text(
                    "Une partie est déjà en cours ou son lancement est déjà programmé.",
                ));
                return Ok(0);
            }
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

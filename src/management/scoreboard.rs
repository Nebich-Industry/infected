use pumpkin_plugin_api::{
    Server, World,
    text::TextComponent,
    wit::pumpkin::plugin::scoreboard::{DisplaySlot, NumberFormat, RenderType},
};

use crate::management::{player, world::current_world};

const OBJECTIVE: &str = "infectedBoard";
pub fn initialize(server: &Server) -> Result<(), String> {
    let (world, _) = current_world(server)?;
    let scoreboard = world.get_scoreboard();

    scoreboard.remove_objective(OBJECTIVE);
    scoreboard.add_objective(
        OBJECTIVE,
        TextComponent::text("Infected"),
        RenderType::Integer,
        None,
    );
    scoreboard.set_display_slot(DisplaySlot::Sidebar, OBJECTIVE);
    scoreboard.update_score("  ", OBJECTIVE, 4, Some(NumberFormat::Blank));
    scoreboard.update_score("Survivants : ", OBJECTIVE, 0, None);
    scoreboard.update_score(" ", OBJECTIVE, 2, Some(NumberFormat::Blank));
    scoreboard.update_score("Zombies : ", OBJECTIVE, 0, None);

    refresh(&world);
    Ok(())
}

pub fn refresh_for_server(server: &Server) {
    match current_world(server) {
        Ok((world, _)) => refresh(&world),
        Err(error) => tracing::error!("Impossible de rafraîchir le tableau d'affichage : {error}"),
    }
}

pub fn refresh(world: &World) {
    let counts = match player::scoreboard_counts() {
        Ok(counts) => counts,
        Err(error) => {
            tracing::error!(
                "Impossible de compter les joueurs pour le tableau d'affichage : {error}"
            );
            return;
        }
    };
    let scoreboard = world.get_scoreboard();
    scoreboard.update_score("Survivants : ", OBJECTIVE, counts.0, None);
    scoreboard.update_score("Zombies : ", OBJECTIVE, counts.1, None);
}

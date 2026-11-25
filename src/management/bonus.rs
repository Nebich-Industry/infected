use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use pumpkin_plugin_api::{
    Context, EntityDisplayExt, EntityType, ItemDisplayMode, ItemStack, Player, Server,
    events::{
        EventData, EventHandler, EventPriority, PlayerInteractAtEntityEvent, ProjectileHitEvent,
        ProjectileLaunchEvent,
    },
    scheduler::schedule_delayed_task,
    text::TextComponent,
    wit::pumpkin::plugin::{
        common::Position,
        status_effect::{StatusEffectInstance, StatusEffectType},
    },
};

use crate::management::{
    player::{self, SurvivorRole, get_player},
    session::{GameState, game_session},
    world::current_world,
};

type PlayerId = (u64, u64);

#[derive(Clone, Copy)]
enum BonusKind {
    Smoke,
    Heal,
    Instakill,
    ZombieInvisibility,
}

impl BonusKind {
    fn is_available_to(self, is_zombie: bool) -> bool {
        matches!(self, Self::ZombieInvisibility) == is_zombie
    }

    fn item(self) -> (&'static str, &'static str) {
        match self {
            Self::Smoke => ("minecraft:snowball", "Bonus Fumigène"),
            Self::Heal => ("minecraft:potion", "Bonus Heal"),
            Self::Instakill => ("minecraft:diamond_sword", "Bonus Instakill"),
            Self::ZombieInvisibility => ("minecraft:potion", "Bonus Invisibilité"),
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Smoke => "Fumigène",
            Self::Heal => "Soin",
            Self::Instakill => "Instakill",
            Self::ZombieInvisibility => "Invisibilité zombie",
        }
    }
}

#[derive(Clone)]
struct BonusPickup {
    kind: BonusKind,
    display_id: u32,
    world_name: String,
}

static BONUS_PICKUPS: OnceLock<Mutex<HashMap<u32, BonusPickup>>> = OnceLock::new();
static SMOKE_GRENADES: OnceLock<Mutex<HashMap<PlayerId, u32>>> = OnceLock::new();
static SMOKE_PROJECTILES: OnceLock<Mutex<HashMap<u32, (PlayerId, String)>>> = OnceLock::new();

fn bonus_pickups() -> &'static Mutex<HashMap<u32, BonusPickup>> {
    BONUS_PICKUPS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn smoke_grenades() -> &'static Mutex<HashMap<PlayerId, u32>> {
    SMOKE_GRENADES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn smoke_projectiles() -> &'static Mutex<HashMap<u32, (PlayerId, String)>> {
    SMOKE_PROJECTILES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn player_id(player: &Player) -> PlayerId {
    let id = player.get_id();
    (id.high, id.low)
}

pub fn register(context: &Context) -> Result<(), String> {
    context
        .register_event_handler(BonusPickupHandler, EventPriority::Normal, true)
        .map_err(|error| {
            format!("Impossible d'enregistrer l'événement de collecte des bonus : {error}")
        })?;
    context
        .register_event_handler(SmokeGrenadeLaunchHandler, EventPriority::Normal, false)
        .map_err(|error| {
            format!("Impossible d'enregistrer le lancement des fumigènes : {error}")
        })?;
    context
        .register_event_handler(SmokeGrenadeHitHandler, EventPriority::Normal, false)
        .map_err(|error| format!("Impossible d'enregistrer l'impact des fumigènes : {error}"))?;
    Ok(())
}

pub fn start_spawn_task() {
    schedule_spawn_tick();
}

pub fn clear(server: &Server) {
    let pickups = match bonus_pickups().lock() {
        Ok(mut pickups) => std::mem::take(&mut *pickups),
        Err(error) => {
            tracing::error!("Verrouillage des bonus impossible : {error}");
            return;
        }
    };

    for (interaction_id, pickup) in pickups {
        remove_entity(server, &pickup.world_name, interaction_id);
        remove_entity(server, &pickup.world_name, pickup.display_id);
    }

    if let Err(error) = smoke_grenades().lock().map(|mut grenades| grenades.clear()) {
        tracing::error!("Verrouillage des fumigènes impossible : {error}");
    }
    if let Err(error) = smoke_projectiles()
        .lock()
        .map(|mut projectiles| projectiles.clear())
    {
        tracing::error!("Verrouillage des projectiles fumigènes impossible : {error}");
    }
}

fn schedule_spawn_tick() {
    schedule_delayed_task(20 * 20, |server| {
        let timer = match game_session().lock() {
            Ok(session) if session.state() == &GameState::Active => session.game_timer(),
            Ok(_) => {
                clear(&server);
                return;
            }
            Err(error) => {
                tracing::error!("Verrouillage de la session de jeu impossible : {error}");
                return;
            }
        };

        if timer <= 4 * 60 && timer > 1 && timer % 20 == 0 {
            let random = random_number(3);
            let survivor_bonus = match random {
                0 => BonusKind::Smoke,
                1 => BonusKind::Heal,
                _ => BonusKind::Instakill,
            };
            for kind in [survivor_bonus, BonusKind::ZombieInvisibility] {
                if let Err(error) = spawn_bonus(&server, kind) {
                    tracing::error!("Impossible de faire apparaître le bonus : {error}");
                }
            }
        }

        schedule_spawn_tick();
    });
}

fn random_number(upper_bound: u64) -> u64 {
    let random_id = pumpkin_plugin_api::uuid::generate();
    (random_id.high ^ random_id.low) % upper_bound
}

fn spawn_bonus(server: &Server, kind: BonusKind) -> Result<(), String> {
    let (world, config) = current_world(server)?;
    let border = world.get_world_border();
    let center = border.get_center();
    let half_size = (border.get_size() / 2.0 - 1.0).max(0.0);
    let position = (
        center.0 + random_offset(half_size),
        config.spawn.y + 2.0,
        center.2 + random_offset(half_size),
    );

    let (item_id, name) = kind.item();
    let item = ItemStack::new(item_id, 1);
    item.set_custom_name(Some(TextComponent::text(name)));

    let display = world.spawn_entity(EntityType::ItemDisplay, position);
    let Some(item_display) = display.as_item_display() else {
        display.remove();
        return Err("L'entité d'affichage du bonus est invalide.".to_string());
    };
    item_display.set_item(Some(item));
    item_display.set_item_display_mode(ItemDisplayMode::Ground);
    item_display
        .get_display()
        .set_billboard(pumpkin_plugin_api::BillboardMode::Vertical);

    let interaction = world.spawn_entity(EntityType::Interaction, position);
    let Some(interaction_entity) = interaction.as_interaction() else {
        interaction.remove();
        display.remove();
        return Err("La zone de collecte du bonus est invalide.".to_string());
    };
    interaction_entity.set_width(1.0);
    interaction_entity.set_height(1.0);
    interaction_entity.set_response(true);
    interaction.set_custom_name(TextComponent::text(name));
    interaction.set_custom_name_visible(false);

    let pickup = BonusPickup {
        kind,
        display_id: display.get_id(),
        world_name: world.get_name(),
    };
    bonus_pickups()
        .lock()
        .map_err(|error| format!("Verrouillage des bonus impossible : {error}"))?
        .insert(interaction.get_id(), pickup);

    Ok(())
}

fn random_offset(half_size: f64) -> f64 {
    let random_id = pumpkin_plugin_api::uuid::generate();
    let unit = (random_id.high ^ random_id.low) as f64 / u64::MAX as f64;
    (unit * 2.0 - 1.0) * half_size
}

fn remove_entity(server: &Server, world_name: &str, entity_id: u32) {
    let Some(world) = server.get_world_by_name(world_name) else {
        tracing::warn!(
            "Impossible de supprimer un bonus : le monde {world_name} n'est plus chargé."
        );
        return;
    };

    if let Some(entity) = world
        .get_entities()
        .into_iter()
        .find(|entity| entity.get_id() == entity_id)
    {
        entity.remove();
    }
}

struct BonusPickupHandler;

impl EventHandler<PlayerInteractAtEntityEvent> for BonusPickupHandler {
    fn handle(
        &self,
        server: Server,
        mut event: EventData<PlayerInteractAtEntityEvent>,
    ) -> EventData<PlayerInteractAtEntityEvent> {
        let Ok(interaction_id) = u32::try_from(event.entity_id) else {
            return event;
        };
        let pickup = match bonus_pickups().lock() {
            Ok(pickups) => pickups.get(&interaction_id).cloned(),
            Err(error) => {
                tracing::error!("Verrouillage des bonus impossible : {error}");
                return event;
            }
        };
        let Some(pickup) = pickup else {
            return event;
        };

        event.cancelled = true;
        let infected_player = match get_player(&event.player) {
            Ok(Some(infected_player)) => infected_player,
            Ok(None) => {
                event.player.send_system_message(
                    TextComponent::text("Vous devez rejoindre la partie pour prendre un bonus."),
                    false,
                );
                return event;
            }
            Err(error) => {
                tracing::error!("Impossible de lire le joueur qui collecte un bonus : {error}");
                return event;
            }
        };

        if !pickup.kind.is_available_to(infected_player.is_zombie()) {
            let message = if infected_player.is_zombie() {
                "Seuls les survivants peuvent prendre ce bonus."
            } else {
                "Seuls les zombies peuvent prendre ce bonus."
            };
            event
                .player
                .send_system_message(TextComponent::text(message), false);
            return event;
        }

        if matches!(pickup.kind, BonusKind::Smoke) && empty_inventory_slot(&event.player).is_none()
        {
            event.player.send_system_message(
                TextComponent::text(
                    "Votre inventaire est plein : le fumigène n'a pas été ramassé.",
                ),
                false,
            );
            return event;
        }

        let pickup = match bonus_pickups().lock() {
            Ok(mut pickups) => pickups.remove(&interaction_id),
            Err(error) => {
                tracing::error!("Verrouillage des bonus impossible : {error}");
                return event;
            }
        };
        let Some(pickup) = pickup else {
            return event;
        };

        remove_entity(&server, &pickup.world_name, interaction_id);
        remove_entity(&server, &pickup.world_name, pickup.display_id);

        match pickup.kind {
            BonusKind::Smoke => give_smoke_grenade(&event.player),
            BonusKind::Heal => {
                event
                    .player
                    .set_health((event.player.get_health() + 4.0).min(20.0));
                event.player.add_effect(StatusEffectInstance {
                    effect_type: StatusEffectType::Speed,
                    duration: 5 * 20,
                    amplifier: 0,
                    ambient: false,
                    show_particles: true,
                    show_icon: true,
                });
            }
            BonusKind::Instakill => {
                if let Err(error) = player::grant_instakill_bonus(&event.player) {
                    tracing::error!("Impossible d'appliquer le bonus instakill : {error}");
                    event.player.send_system_message(
                        TextComponent::text("Impossible d'appliquer ce bonus."),
                        false,
                    );
                }
            }
            BonusKind::ZombieInvisibility => event.player.add_effect(StatusEffectInstance {
                effect_type: StatusEffectType::Invisibility,
                duration: 5 * 20,
                amplifier: 0,
                ambient: false,
                show_particles: true,
                show_icon: true,
            }),
        }

        event.player.send_system_message(
            TextComponent::text(&format!("Bonus ramassé : {}.", pickup.kind.description())),
            false,
        );
        event
    }
}

fn empty_inventory_slot(player: &Player) -> Option<u32> {
    player
        .get_inventory()
        .as_inventory()
        .get_all_items()
        .iter()
        .position(Option::is_none)
        .map(|slot| slot as u32)
}

fn give_smoke_grenade(player: &Player) {
    let id = player_id(player);
    match smoke_grenades().lock() {
        Ok(mut grenades) => {
            *grenades.entry(id).or_default() += 1;
        }
        Err(error) => {
            tracing::error!("Verrouillage des fumigènes impossible : {error}");
            return;
        }
    }

    let smoke_grenade = ItemStack::new("minecraft:snowball", 1);
    smoke_grenade.set_custom_name(Some(TextComponent::text("Fumigène Infected")));
    if let Some(slot) = empty_inventory_slot(player) {
        player
            .get_inventory()
            .as_inventory()
            .set_item(slot, Some(smoke_grenade));
    }
}

struct SmokeGrenadeLaunchHandler;

impl EventHandler<ProjectileLaunchEvent> for SmokeGrenadeLaunchHandler {
    fn handle(
        &self,
        server: Server,
        event: EventData<ProjectileLaunchEvent>,
    ) -> EventData<ProjectileLaunchEvent> {
        let Some(shooter_id) = event.shooter_id.and_then(|id| u32::try_from(id).ok()) else {
            return event;
        };
        let Some(shooter) = server
            .get_all_players()
            .into_iter()
            .find(|player| player.as_entity().get_id() == shooter_id)
        else {
            return event;
        };
        let projectile_id = match u32::try_from(event.entity_id) {
            Ok(id) => id,
            Err(_) => return event,
        };
        let projectile_is_snowball = shooter
            .get_world()
            .get_entities()
            .into_iter()
            .any(|entity| {
                entity.get_id() == projectile_id && entity.get_type() == EntityType::Snowball
            });
        if !projectile_is_snowball {
            return event;
        }

        let id = player_id(&shooter);
        let mut grenades = match smoke_grenades().lock() {
            Ok(grenades) => grenades,
            Err(error) => {
                tracing::error!("Verrouillage des fumigènes impossible : {error}");
                return event;
            }
        };
        let Some(count) = grenades.get_mut(&id) else {
            return event;
        };
        *count -= 1;
        if *count == 0 {
            grenades.remove(&id);
        }
        drop(grenades);

        match smoke_projectiles().lock() {
            Ok(mut projectiles) => {
                projectiles.insert(projectile_id, (id, shooter.get_world().get_name()));
            }
            Err(error) => {
                tracing::error!("Verrouillage des projectiles fumigènes impossible : {error}")
            }
        }
        event
    }
}

struct SmokeGrenadeHitHandler;

impl EventHandler<ProjectileHitEvent> for SmokeGrenadeHitHandler {
    fn handle(
        &self,
        server: Server,
        event: EventData<ProjectileHitEvent>,
    ) -> EventData<ProjectileHitEvent> {
        let Ok(projectile_id) = u32::try_from(event.entity_id) else {
            return event;
        };
        let smoke = match smoke_projectiles().lock() {
            Ok(mut projectiles) => projectiles.remove(&projectile_id),
            Err(error) => {
                tracing::error!("Verrouillage des projectiles fumigènes impossible : {error}");
                return event;
            }
        };
        if let Some((shooter_id, world_name)) = smoke {
            run_smoke_tick(server, world_name, shooter_id, event.hit_position, 5);
        }
        event
    }
}

fn run_smoke_tick(
    server: Server,
    world_name: String,
    shooter_id: PlayerId,
    center: Position,
    remaining_seconds: u8,
) {
    if remaining_seconds == 0 {
        return;
    }

    let shooter = server
        .get_all_players()
        .into_iter()
        .find(|player| player_id(player) == shooter_id);
    for player in server.get_all_players() {
        if player.get_world().get_name() != world_name {
            continue;
        }
        let position = player.as_entity().get_position();
        let distance_squared = (position.0 - center.0).powi(2)
            + (position.1 - center.1).powi(2)
            + (position.2 - center.2).powi(2);
        if distance_squared > 25.0 {
            continue;
        }

        let infected_player = match get_player(&player) {
            Ok(Some(infected_player)) => infected_player,
            Ok(None) => continue,
            Err(error) => {
                tracing::error!("Impossible de lire un joueur dans le fumigène : {error}");
                continue;
            }
        };
        if infected_player.is_survivor() && infected_player.role() == Some(SurvivorRole::Ninja) {
            add_short_effect(&player, StatusEffectType::Speed);
            add_short_effect(&player, StatusEffectType::Invisibility);
        } else if shooter
            .as_ref()
            .is_none_or(|shooter| player_id(&player) != player_id(shooter))
        {
            add_short_effect(&player, StatusEffectType::Blindness);
        }
    }

    let next_world_name = world_name.clone();
    schedule_delayed_task(20, move |server| {
        run_smoke_tick(
            server,
            next_world_name.clone(),
            shooter_id,
            center,
            remaining_seconds - 1,
        );
    });
}

fn add_short_effect(player: &Player, effect_type: StatusEffectType) {
    player.add_effect(StatusEffectInstance {
        effect_type,
        duration: 2 * 20,
        amplifier: 0,
        ambient: false,
        show_particles: true,
        show_icon: true,
    });
}

#[cfg(test)]
mod tests {
    use super::BonusKind;

    #[test]
    fn bonus_collection_is_restricted_to_the_intended_team() {
        assert!(BonusKind::ZombieInvisibility.is_available_to(true));
        assert!(!BonusKind::ZombieInvisibility.is_available_to(false));
        assert!(BonusKind::Instakill.is_available_to(false));
        assert!(!BonusKind::Instakill.is_available_to(true));
    }
}

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use pumpkin_plugin_api::{
    Context, ItemStack, Player, Server, TeamSettingsBuilder,
    events::{
        EntityDamageByEntityEvent, EventData, EventHandler, EventPriority, PlayerChangedWorldEvent,
        PlayerDeathEvent, PlayerLeaveEvent,
    },
    scheduler::schedule_delayed_task,
    text::TextComponent,
    wit::pumpkin::plugin::status_effect::{StatusEffectInstance, StatusEffectType},
};

use crate::management::{
    session::{GameState, game_session},
    world::{current_world, is_managed_world, safe_spawn_position},
};

const SURVIVOR_TEAM: &str = "TEAM_SURVIVORS";
const ZOMBIE_TEAM: &str = "TEAM_ZOMBIES";
const KILL_CREDIT_WINDOW: Duration = Duration::from_secs(10);

type PlayerId = (u64, u64);

static PLAYERS: OnceLock<Mutex<HashMap<PlayerId, InfectedPlayer>>> = OnceLock::new();
static LAST_ATTACKERS: OnceLock<Mutex<HashMap<PlayerId, (PlayerId, Instant)>>> = OnceLock::new();

fn players() -> &'static Mutex<HashMap<PlayerId, InfectedPlayer>> {
    PLAYERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn last_attackers() -> &'static Mutex<HashMap<PlayerId, (PlayerId, Instant)>> {
    LAST_ATTACKERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn player_id(player: &Player) -> PlayerId {
    let id = player.get_id();
    (id.high, id.low)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurvivorRole {
    Archer,
    Doctor,
    Ninja,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZombieRole {
    Pigman,
    Skeleton,
    Husk,
}

#[derive(Debug, Clone)]
pub struct InfectedPlayer {
    id: PlayerId,
    name: String,
    role: Option<SurvivorRole>,
    zombie_role: Option<ZombieRole>,
    is_zombie: bool,
    is_survivor: bool,
    is_selected_at_start: bool,
    has_instakill_bonus: bool,
    killed_players_as_zombie: Vec<PlayerId>,
    zombie_upgrade_resolved: bool,
}

impl InfectedPlayer {
    fn new(player: &Player, is_zombie: bool) -> Self {
        Self {
            id: player_id(player),
            name: player.get_name(),
            role: (!is_zombie).then(random_survivor_role),
            zombie_role: None,
            is_zombie,
            is_survivor: !is_zombie,
            is_selected_at_start: false,
            has_instakill_bonus: false,
            killed_players_as_zombie: Vec::new(),
            zombie_upgrade_resolved: false,
        }
    }

    pub fn is_zombie(&self) -> bool {
        self.is_zombie
    }

    pub fn id(&self) -> (u64, u64) {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_is_zombie(&mut self, is_zombie: bool) {
        self.is_zombie = is_zombie;
        self.is_survivor = !is_zombie;
    }

    pub fn is_survivor(&self) -> bool {
        self.is_survivor
    }

    pub fn set_is_survivor(&mut self, is_survivor: bool) {
        self.is_survivor = is_survivor;
        self.is_zombie = !is_survivor;
    }

    pub fn role(&self) -> Option<SurvivorRole> {
        self.role
    }

    pub fn set_role(&mut self, role: Option<SurvivorRole>) {
        self.role = role;
    }

    pub fn zombie_role(&self) -> Option<ZombieRole> {
        self.zombie_role
    }

    pub fn set_zombie_role(&mut self, role: Option<ZombieRole>) {
        self.zombie_role = role;
    }

    pub fn is_selected_at_start(&self) -> bool {
        self.is_selected_at_start
    }

    pub fn set_selected_at_start(&mut self, selected: bool) {
        self.is_selected_at_start = selected;
    }

    pub fn has_instakill_bonus(&self) -> bool {
        self.has_instakill_bonus
    }

    pub fn set_has_instakill_bonus(&mut self, has_bonus: bool) {
        self.has_instakill_bonus = has_bonus;
    }

    pub fn add_zombie_kill(&mut self, player_id: PlayerId) {
        self.killed_players_as_zombie.push(player_id);
    }

    pub fn killed_players_as_zombie(&self) -> &[PlayerId] {
        &self.killed_players_as_zombie
    }
}

fn random_survivor_role() -> SurvivorRole {
    match random_number(3) {
        0 => SurvivorRole::Archer,
        1 => SurvivorRole::Doctor,
        _ => SurvivorRole::Ninja,
    }
}

fn random_zombie_role() -> ZombieRole {
    match random_number(100) {
        0..=33 => ZombieRole::Pigman,
        34..=66 => ZombieRole::Skeleton,
        _ => ZombieRole::Husk,
    }
}

fn random_number(upper_bound: u64) -> u64 {
    let random_id = pumpkin_plugin_api::uuid::generate();
    (random_id.high ^ random_id.low) % upper_bound
}

pub fn add_player(player: &Player) -> Result<(), String> {
    let is_zombie = {
        let session = game_session()
            .lock()
            .map_err(|error| format!("Verrouillage de la session de jeu impossible : {error}"))?;
        session.state() == &GameState::Active
    };
    let id = player_id(player);

    let (is_zombie, is_new) = {
        let mut infected_players = players()
            .lock()
            .map_err(|error| format!("Verrouillage des joueurs infectés impossible : {error}"))?;
        if let Some(infected_player) = infected_players.get_mut(&id) {
            infected_player.name = player.get_name();
            (infected_player.is_zombie(), false)
        } else {
            infected_players.insert(id, InfectedPlayer::new(player, is_zombie));
            (is_zombie, true)
        }
    };

    set_scoreboard_team(player, is_zombie);
    if is_zombie && is_new {
        schedule_zombie_upgrade(id);
    }
    if is_new && let Some(infected_player) = get_player(player)? {
        tracing::debug!(
            "Joueur infecté enregistré : {} ({:?})",
            infected_player.name(),
            infected_player.role()
        );
    }
    Ok(())
}

pub fn register_current_world_players(server: &Server) -> Result<(), String> {
    let (world, _) = current_world(server)?;
    for player in server.get_players_in_world(world) {
        add_player(&player)?;
    }
    Ok(())
}

pub fn get_player(player: &Player) -> Result<Option<InfectedPlayer>, String> {
    players()
        .lock()
        .map_err(|error| format!("Verrouillage des joueurs infectés impossible : {error}"))
        .map(|infected_players| {
            infected_players
                .get(&player_id(player))
                .filter(|infected_player| infected_player.id() == player_id(player))
                .cloned()
        })
}

pub(crate) fn scoreboard_counts() -> Result<(i32, i32), String> {
    let infected_players = players()
        .lock()
        .map_err(|error| format!("Verrouillage des joueurs infectés impossible : {error}"))?;
    let survivors = infected_players
        .values()
        .filter(|player| player.is_survivor())
        .count();
    let zombies = infected_players
        .values()
        .filter(|player| player.is_zombie())
        .count();

    Ok((
        i32::try_from(survivors)
            .map_err(|error| format!("Nombre de survivants hors limites : {error}"))?,
        i32::try_from(zombies)
            .map_err(|error| format!("Nombre de zombies hors limites : {error}"))?,
    ))
}

pub fn set_survivor_role(player: &Player, role: SurvivorRole) -> Result<(), String> {
    let mut infected_players = players()
        .lock()
        .map_err(|error| format!("Verrouillage des joueurs infectés impossible : {error}"))?;
    let infected_player = infected_players
        .get_mut(&player_id(player))
        .ok_or_else(|| format!("Le joueur {} n'est pas enregistré.", player.get_name()))?;
    if !infected_player.is_survivor() {
        return Err("Seuls les survivants peuvent choisir une classe.".to_string());
    }

    infected_player.set_role(Some(role));
    Ok(())
}

pub fn has_survivors(server: &Server) -> Result<bool, String> {
    let (world, _) = current_world(server)?;
    for player in server.get_players_in_world(world) {
        if get_player(&player)?.is_some_and(|infected_player| infected_player.is_survivor()) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn give_starter_items(server: &Server) -> Result<(), String> {
    let (world, _) = current_world(server)?;
    for player in server.get_players_in_world(world) {
        let Some(infected_player) = get_player(&player)? else {
            continue;
        };
        let inventory = player.get_inventory().as_inventory();
        inventory.clear();
        if infected_player.is_survivor() {
            inventory.set_item(0, Some(ItemStack::new("minecraft:stone_sword", 1)));
            inventory.set_item(1, Some(ItemStack::new("minecraft:bow", 1)));
            inventory.set_item(2, Some(ItemStack::new("minecraft:arrow", 5)));
        }
    }
    Ok(())
}

pub fn give_survivor_arrows(server: &Server) -> Result<(), String> {
    let (world, _) = current_world(server)?;
    for player in server.get_players_in_world(world) {
        if get_player(&player)?.is_some_and(|infected_player| infected_player.is_survivor()) {
            give_item_if_slot_available(&player, ItemStack::new("minecraft:arrow", 1));
        }
    }
    Ok(())
}

pub fn grant_instakill_bonus(player: &Player) -> Result<(), String> {
    let id = player_id(player);
    {
        let mut infected_players = players()
            .lock()
            .map_err(|error| format!("Verrouillage des joueurs infectés impossible : {error}"))?;
        let infected_player = infected_players
            .get_mut(&id)
            .ok_or_else(|| format!("Le joueur {} n'est pas enregistré.", player.get_name()))?;
        infected_player.set_has_instakill_bonus(true);
    }

    schedule_delayed_task(10 * 20, move |_| match players().lock() {
        Ok(mut infected_players) => {
            if let Some(infected_player) = infected_players.get_mut(&id) {
                infected_player.set_has_instakill_bonus(false);
            }
        }
        Err(error) => tracing::error!("Verrouillage des joueurs infectés impossible : {error}"),
    });
    Ok(())
}

pub fn transform_in_zombie(player: &Player) -> Result<(), String> {
    let id = player_id(player);
    {
        let mut infected_players = players()
            .lock()
            .map_err(|error| format!("Verrouillage des joueurs infectés impossible : {error}"))?;
        let infected_player = infected_players
            .get_mut(&id)
            .ok_or_else(|| format!("Le joueur {} n'est pas enregistré.", player.get_name()))?;
        infected_player.set_is_zombie(true);
        infected_player.set_role(None);
        infected_player.zombie_upgrade_resolved = false;
    }

    set_scoreboard_team(player, true);
    schedule_zombie_upgrade(id);
    Ok(())
}

pub fn transform_in_survivor(player: &Player) -> Result<(), String> {
    let id = player_id(player);
    {
        let mut infected_players = players()
            .lock()
            .map_err(|error| format!("Verrouillage des joueurs infectés impossible : {error}"))?;
        let infected_player = infected_players
            .get_mut(&id)
            .ok_or_else(|| format!("Le joueur {} n'est pas enregistré.", player.get_name()))?;
        infected_player.set_is_survivor(true);
        infected_player.set_zombie_role(None);
        infected_player.set_role(Some(random_survivor_role()));
        infected_player.set_has_instakill_bonus(false);
    }

    set_scoreboard_team(player, false);
    Ok(())
}

pub fn reset_zombies_to_survivors(server: &Server) -> Result<(), String> {
    let (world, _) = current_world(server)?;
    for player in server.get_players_in_world(world) {
        if get_player(&player)?.is_some_and(|infected_player| infected_player.is_zombie()) {
            transform_in_survivor(&player)?;
        }
    }
    Ok(())
}

fn set_scoreboard_team(player: &Player, is_zombie: bool) {
    if let Some(java_player) = player.as_java() {
        let scoreboard = java_player.get_scoreboard();
        for team_name in [SURVIVOR_TEAM, ZOMBIE_TEAM] {
            if scoreboard.get_team(team_name).is_none() {
                scoreboard.create_team(
                    team_name,
                    TeamSettingsBuilder::new().friendly_fire(false).build(),
                );
            }
            scoreboard.remove_player_from_team(team_name, &player.get_name());
        }
        scoreboard.add_player_to_team(
            if is_zombie {
                ZOMBIE_TEAM
            } else {
                SURVIVOR_TEAM
            },
            &player.get_name(),
        );
    }
    crate::management::scoreboard::refresh(&player.get_world());
}

fn remove_player(player: &Player) {
    let id = player_id(player);
    match players().lock() {
        Ok(mut infected_players) => {
            infected_players.remove(&id);
        }
        Err(error) => tracing::error!("Verrouillage des joueurs infectés impossible : {error}"),
    }
    match last_attackers().lock() {
        Ok(mut attackers) => {
            attackers.remove(&id);
        }
        Err(error) => tracing::error!("Verrouillage des attaquants impossible : {error}"),
    }

    if let Some(java_player) = player.as_java() {
        let scoreboard = java_player.get_scoreboard();
        scoreboard.remove_player_from_team(SURVIVOR_TEAM, &player.get_name());
        scoreboard.remove_player_from_team(ZOMBIE_TEAM, &player.get_name());
    }
    crate::management::scoreboard::refresh(&player.get_world());
}

fn schedule_zombie_upgrade(id: PlayerId) {
    schedule_delayed_task(20, move |server| {
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

        let decision = match players().lock() {
            Ok(mut infected_players) => {
                let Some(infected_player) = infected_players.get_mut(&id) else {
                    return;
                };
                if !infected_player.is_zombie || infected_player.zombie_upgrade_resolved {
                    return;
                }
                if infected_player.killed_players_as_zombie().len() < 3 {
                    None
                } else {
                    infected_player.zombie_upgrade_resolved = true;
                    if random_number(100) <= 50 {
                        Some(Some(random_zombie_role()))
                    } else {
                        Some(None)
                    }
                }
            }
            Err(error) => {
                tracing::error!("Verrouillage des joueurs infectés impossible : {error}");
                return;
            }
        };

        let Some(decision) = decision else {
            schedule_zombie_upgrade(id);
            return;
        };
        let Some(player) = find_player(&server, id) else {
            return;
        };

        match decision {
            Some(role) => {
                if let Err(error) = set_zombie_role(&player, role) {
                    tracing::error!("Impossible d'appliquer le rôle zombie : {error}");
                }
            }
            None => {
                if let Err(error) = transform_in_survivor(&player) {
                    tracing::error!("Impossible de transformer le joueur en survivant : {error}");
                }
            }
        }
    });
}

fn set_zombie_role(player: &Player, role: ZombieRole) -> Result<(), String> {
    let mut infected_players = players()
        .lock()
        .map_err(|error| format!("Verrouillage des joueurs infectés impossible : {error}"))?;
    let infected_player = infected_players
        .get_mut(&player_id(player))
        .ok_or_else(|| format!("Le joueur {} n'est pas enregistré.", player.get_name()))?;
    infected_player.set_zombie_role(Some(role));
    let role = infected_player
        .zombie_role()
        .ok_or_else(|| "Le rôle zombie n'a pas été enregistré.".to_string())?;
    drop(infected_players);

    match role {
        ZombieRole::Pigman => {
            player.set_max_health(18.0);
            player.add_effect(StatusEffectInstance {
                effect_type: StatusEffectType::Speed,
                duration: u32::MAX,
                amplifier: 0,
                ambient: false,
                show_particles: true,
                show_icon: true,
            });
        }
        ZombieRole::Skeleton => {
            give_item_if_slot_available(player, ItemStack::new("minecraft:bow", 1));
            give_item_if_slot_available(player, ItemStack::new("minecraft:arrow", 5));
        }
        ZombieRole::Husk => {}
    }
    Ok(())
}

fn give_item_if_slot_available(player: &Player, item: ItemStack) {
    let inventory = player.get_inventory().as_inventory();
    let slots = inventory.get_all_items();
    if let Some(slot) = slots.iter().position(Option::is_none) {
        inventory.set_item(slot as u32, Some(item));
    } else {
        player.send_system_message(
            TextComponent::text(
                "Votre inventaire est plein : un objet de rôle n'a pas pu être ajouté.",
            ),
            false,
        );
    }
}

fn find_player(server: &Server, id: PlayerId) -> Option<Player> {
    server
        .get_all_players()
        .into_iter()
        .find(|player| player_id(player) == id)
}

fn damage_for_survivor(timer: u32, has_instakill_bonus: bool) -> f32 {
    let mut damage = 4.0;
    if timer <= 210 {
        damage = 8.0;
    }
    if timer <= 90 {
        damage = 16.0;
    }
    if timer <= 150 {
        damage = 12.0;
    }
    if timer <= 30 || has_instakill_bonus {
        damage = 20.0;
    }
    damage
}

struct PlayerDeathHandler;

impl EventHandler<PlayerDeathEvent> for PlayerDeathHandler {
    fn handle(
        &self,
        server: Server,
        event: EventData<PlayerDeathEvent>,
    ) -> EventData<PlayerDeathEvent> {
        let dead_player = &event.player;
        let dead_id = player_id(dead_player);
        let dead_was_zombie = match get_player(dead_player) {
            Ok(Some(infected_player)) => {
                tracing::debug!("Mort du joueur infecté {}", infected_player.name());
                let dead_was_zombie = infected_player.is_zombie();
                if infected_player.is_survivor()
                    && let Err(error) = transform_in_zombie(dead_player)
                {
                    tracing::error!("Impossible de transformer le joueur en zombie : {error}");
                }

                match current_world(&server) {
                    Ok((world, config)) => match safe_spawn_position(&world, &config.spawn) {
                        Ok(spawn) => dead_player.teleport_world(
                            world,
                            spawn,
                            Some(dead_player.get_yaw()),
                            Some(dead_player.get_pitch()),
                        ),
                        Err(error) => {
                            tracing::error!(
                                "Impossible de trouver un emplacement de réapparition sûr : {error}"
                            )
                        }
                    },
                    Err(error) => {
                        tracing::error!("Impossible de téléporter le joueur mort : {error}")
                    }
                }
                dead_was_zombie
            }
            Ok(None) => false,
            Err(error) => {
                tracing::error!("Impossible de lire le joueur mort : {error}");
                false
            }
        };

        let last_attacker = match last_attackers().lock() {
            Ok(mut attackers) => attackers.remove(&dead_id),
            Err(error) => {
                tracing::error!("Verrouillage des attaquants impossible : {error}");
                None
            }
        };
        if let Some((attacker_id, timestamp)) = last_attacker
            && timestamp.elapsed() <= KILL_CREDIT_WINDOW
        {
            let attacker = find_player(&server, attacker_id);
            if let Some(attacker) = attacker {
                match get_player(&attacker) {
                    Ok(Some(infected_player)) if infected_player.is_zombie() => {
                        match players().lock() {
                            Ok(mut infected_players) => {
                                if let Some(infected_player) =
                                    infected_players.get_mut(&attacker_id)
                                {
                                    infected_player.add_zombie_kill(dead_id);
                                }
                            }
                            Err(error) => tracing::error!(
                                "Verrouillage des joueurs infectés impossible : {error}"
                            ),
                        }
                    }
                    Ok(Some(infected_player))
                        if infected_player.role() == Some(SurvivorRole::Doctor)
                            && dead_was_zombie =>
                    {
                        if random_number(100) <= 20 {
                            attacker.set_health((attacker.get_health() + 4.0).min(20.0));
                        }
                    }
                    Ok(_) => {}
                    Err(error) => tracing::error!("Impossible de lire le tueur : {error}"),
                }
            }
        }
        event
    }
}

struct EntityDamageByEntityHandler;

impl EventHandler<EntityDamageByEntityEvent> for EntityDamageByEntityHandler {
    fn handle(
        &self,
        server: Server,
        mut event: EventData<EntityDamageByEntityEvent>,
    ) -> EventData<EntityDamageByEntityEvent> {
        let has_infected_players = match players().lock() {
            Ok(infected_players) => !infected_players.is_empty(),
            Err(error) => {
                tracing::error!("Verrouillage des joueurs infectés impossible : {error}");
                return event;
            }
        };
        if !has_infected_players {
            return event;
        }

        event.cancelled = true;
        let online_players = server.get_all_players();
        let (Ok(victim_id), Ok(attacker_id)) = (
            u32::try_from(event.entity_id),
            u32::try_from(event.damager_id),
        ) else {
            return event;
        };
        let victim = online_players
            .iter()
            .find(|player| player.as_entity().get_id() == victim_id);
        let attacker = online_players
            .iter()
            .find(|player| player.as_entity().get_id() == attacker_id);
        let (Some(victim), Some(attacker)) = (victim, attacker) else {
            return event;
        };

        let victim_id = player_id(victim);
        let attacker_id = player_id(attacker);
        let victim_state = match get_player(victim) {
            Ok(state) => state,
            Err(error) => {
                tracing::error!("Impossible de lire la victime : {error}");
                return event;
            }
        };
        let attacker_state = match get_player(attacker) {
            Ok(state) => state,
            Err(error) => {
                tracing::error!("Impossible de lire l'attaquant : {error}");
                return event;
            }
        };

        let Some(victim_state) = victim_state else {
            return event;
        };
        let Some(attacker_state) = attacker_state else {
            return event;
        };

        if victim_state.is_survivor()
            && attacker_state.is_zombie()
            && attacker_state.zombie_role() == Some(ZombieRole::Husk)
            && random_number(100) <= 30
        {
            victim.add_effect(StatusEffectInstance {
                effect_type: StatusEffectType::Blindness,
                duration: 5 * 20,
                amplifier: 0,
                ambient: false,
                show_particles: true,
                show_icon: true,
            });
        }

        if !victim_state.is_zombie() || !attacker_state.is_survivor() {
            return event;
        }
        let timer = match game_session().lock() {
            Ok(session) => session.game_timer(),
            Err(error) => {
                tracing::error!("Verrouillage de la session de jeu impossible : {error}");
                return event;
            }
        };
        let damage = damage_for_survivor(timer, victim_state.has_instakill_bonus());

        match last_attackers().lock() {
            Ok(mut attackers) => {
                attackers.insert(victim_id, (attacker_id, Instant::now()));
            }
            Err(error) => tracing::error!("Verrouillage des attaquants impossible : {error}"),
        }
        victim.set_health((victim.get_health() - damage).max(0.0));
        event
    }
}

struct PlayerChangedWorldHandler;

impl EventHandler<PlayerChangedWorldEvent> for PlayerChangedWorldHandler {
    fn handle(
        &self,
        server: Server,
        event: EventData<PlayerChangedWorldEvent>,
    ) -> EventData<PlayerChangedWorldEvent> {
        let from_world = event.from_world.get_name();
        let to_world = event.to_world.get_name();
        match is_managed_world(&from_world) {
            Ok(true) => remove_player(&event.player),
            Ok(false) => {}
            Err(error) => tracing::error!("Impossible de vérifier le monde quitté : {error}"),
        }

        match current_world(&server) {
            Ok((world, _)) if world.get_name() == to_world => {
                if let Err(error) = add_player(&event.player) {
                    tracing::error!("Impossible d'enregistrer le joueur : {error}");
                }
            }
            Ok(_) => {}
            Err(error) => {
                tracing::error!("Impossible de déterminer le monde du mini-jeu : {error}")
            }
        }
        crate::management::scoreboard::refresh_for_server(&server);
        event
    }
}

struct PlayerLeaveHandler;

impl EventHandler<PlayerLeaveEvent> for PlayerLeaveHandler {
    fn handle(
        &self,
        server: Server,
        event: EventData<PlayerLeaveEvent>,
    ) -> EventData<PlayerLeaveEvent> {
        remove_player(&event.player);
        crate::management::scoreboard::refresh_for_server(&server);
        event
    }
}

pub fn register(context: &Context) -> Result<(), String> {
    context
        .register_event_handler(PlayerDeathHandler, EventPriority::Normal, false)
        .map_err(|error| format!("Impossible d'enregistrer l'événement de mort : {error}"))?;
    context
        .register_event_handler(EntityDamageByEntityHandler, EventPriority::Normal, true)
        .map_err(|error| format!("Impossible d'enregistrer l'événement de dégâts : {error}"))?;
    context
        .register_event_handler(PlayerChangedWorldHandler, EventPriority::Normal, false)
        .map_err(|error| {
            format!("Impossible d'enregistrer l'événement de changement de monde : {error}")
        })?;
    context
        .register_event_handler(PlayerLeaveHandler, EventPriority::Normal, false)
        .map_err(|error| format!("Impossible d'enregistrer l'événement de départ : {error}"))?;

    let server = context.get_server();
    let (world, _) = current_world(&server)?;
    for player in server.get_players_in_world(world) {
        add_player(&player)?;
    }
    Ok(())
}

pub fn choose_first_zombies(server: &Server) -> Result<(), String> {
    let (world, _) = current_world(server)?;
    let mut candidates = Vec::new();
    for player in server.get_players_in_world(world) {
        let Some(infected_player) = get_player(&player)? else {
            continue;
        };
        if infected_player.is_survivor() && !infected_player.is_selected_at_start() {
            candidates.push(player);
        }
    }

    if candidates.is_empty() {
        return Ok(());
    }

    let zombie_count = if candidates.len() <= 10 {
        1
    } else {
        ((candidates.len() as f32 / 10.0).round() as usize).max(5)
    }
    .min(candidates.len());

    for _ in 0..zombie_count {
        let index = random_number(candidates.len() as u64) as usize;
        let player = candidates.swap_remove(index);
        transform_in_zombie(&player)?;
        {
            let mut infected_players = players().lock().map_err(|error| {
                format!("Verrouillage des joueurs infectés impossible : {error}")
            })?;
            if let Some(infected_player) = infected_players.get_mut(&player_id(&player)) {
                infected_player.set_selected_at_start(true);
            }
        }
        player.send_system_message(
            TextComponent::text("Vous venez de vous transformer en zombie."),
            false,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{InfectedPlayer, PlayerId, SurvivorRole, ZombieRole, damage_for_survivor};

    #[test]
    fn survivor_damage_uses_game_timer_and_bonus_thresholds() {
        assert_eq!(damage_for_survivor(211, false), 4.0);
        assert_eq!(damage_for_survivor(210, false), 8.0);
        assert_eq!(damage_for_survivor(150, false), 12.0);
        assert_eq!(damage_for_survivor(30, false), 20.0);
        assert_eq!(damage_for_survivor(240, true), 20.0);
    }

    #[test]
    fn role_changes_keep_zombie_and_survivor_flags_complementary() {
        let mut player = InfectedPlayer {
            id: PlayerId::default(),
            name: "test".to_string(),
            role: Some(SurvivorRole::Archer),
            zombie_role: Some(ZombieRole::Husk),
            is_zombie: false,
            is_survivor: true,
            is_selected_at_start: false,
            has_instakill_bonus: false,
            killed_players_as_zombie: Vec::new(),
            zombie_upgrade_resolved: false,
        };

        player.set_is_zombie(true);
        player.set_has_instakill_bonus(true);

        assert!(player.is_zombie());
        assert!(!player.is_survivor());
        assert!(player.has_instakill_bonus());
    }
}

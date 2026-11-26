use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Mutex, OnceLock},
};

use pumpkin_plugin_api::{Context, Server, common::BlockPos, server::Dimension, world::World};
use serde::Deserialize;

const DEFAULT_CONFIG: &str = include_str!("default_config.yml");

static WORLD_MANAGER: OnceLock<Mutex<Option<WorldManager>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize)]
struct WorldConfigurationFile {
    worlds: BTreeMap<String, WorldConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorldConfig {
    pub spawn: Coordinates,
    border: BorderConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Coordinates {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct BorderConfig {
    center: Coordinates,
    radius: f64,
}

struct WorldManager {
    worlds: BTreeMap<String, WorldConfig>,
    current_world: String,
}

fn world_manager() -> &'static Mutex<Option<WorldManager>> {
    WORLD_MANAGER.get_or_init(|| Mutex::new(None))
}

pub fn load_worlds(context: &Context) -> Result<(), String> {
    let data_folder = context.get_data_folder();
    let config_path = Path::new(&data_folder).join("config.yml");
    let config_contents = match fs::read_to_string(&config_path) {
        Ok(contents) => contents,
        // Si le fichier de configuration n'existe pas, on crée le dossier et le fichier
        // avec la configuration par défaut
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = config_path
                .parent()
                .ok_or_else(|| "Le dossier de configuration du plugin est invalide.".to_string())?;
            fs::create_dir_all(parent).map_err(|error| {
                format!("Impossible de créer le dossier de configuration : {error}")
            })?;
            fs::write(&config_path, DEFAULT_CONFIG)
                .map_err(|error| format!("Impossible de créer le fichier config.yml : {error}"))?;
            DEFAULT_CONFIG.to_string()
        }
        Err(error) => {
            return Err(format!("Impossible de lire config.yml : {error}"));
        }
    };

    let worlds = parse_config(&config_contents)?;
    let server = context.get_server();

    for (name, config) in &worlds {
        let world = server
            .get_world_by_name(name)
            .unwrap_or_else(|| server.create_world(name, Dimension::Overworld));
        configure_world(&world, config);
    }

    let current_world = worlds
        .keys()
        .next()
        .cloned()
        .ok_or_else(|| "Aucun monde n'est configuré dans config.yml.".to_string())?;

    {
        let mut manager = world_manager()
            .lock()
            .map_err(|_| "Verrouillage du gestionnaire de mondes impossible.".to_string())?;
        *manager = Some(WorldManager {
            worlds,
            current_world,
        });
    }

    select_random_world(&server)
}

pub fn select_random_world(server: &Server) -> Result<(), String> {
    let (name, config) = {
        let manager = world_manager()
            .lock()
            .map_err(|_| "Verrouillage du gestionnaire de mondes impossible.".to_string())?;
        let manager = manager
            .as_ref()
            .ok_or_else(|| "Le gestionnaire de mondes n'est pas initialisé.".to_string())?;
        let names: Vec<_> = manager.worlds.keys().cloned().collect();
        if names.is_empty() {
            return Err("Aucun monde n'est configuré dans config.yml.".to_string());
        }
        let random_uuid = pumpkin_plugin_api::uuid::generate();
        let index = ((random_uuid.high ^ random_uuid.low) % names.len() as u64) as usize;
        let name = names[index].clone();
        let config = manager
            .worlds
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("La configuration du monde {name} est introuvable."))?;
        (name, config)
    };

    let world = server
        .get_world_by_name(&name)
        .ok_or_else(|| format!("Le monde configuré {name} n'est pas chargé."))?;
    configure_world(&world, &config);

    let mut manager = world_manager()
        .lock()
        .map_err(|_| "Verrouillage du gestionnaire de mondes impossible.".to_string())?;
    let manager = manager
        .as_mut()
        .ok_or_else(|| "Le gestionnaire de mondes n'est pas initialisé.".to_string())?;
    manager.current_world = name;
    Ok(())
}

pub fn current_world(server: &Server) -> Result<(World, WorldConfig), String> {
    let (name, config) = {
        let manager = world_manager()
            .lock()
            .map_err(|_| "Verrouillage du gestionnaire de mondes impossible.".to_string())?;
        let manager = manager
            .as_ref()
            .ok_or_else(|| "Le gestionnaire de mondes n'est pas initialisé.".to_string())?;
        let config = manager
            .worlds
            .get(&manager.current_world)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "La configuration du monde {} est introuvable.",
                    manager.current_world
                )
            })?;
        (manager.current_world.clone(), config)
    };

    let world = server
        .get_world_by_name(&name)
        .ok_or_else(|| format!("Le monde configuré {name} n'est pas chargé."))?;
    Ok((world, config))
}

pub fn safe_spawn_position(world: &World, spawn: &Coordinates) -> Result<(f64, f64, f64), String> {
    let block_x = block_coordinate(spawn.x)
        .ok_or_else(|| "La coordonnée X du spawn est invalide.".to_string())?;
    let block_z = block_coordinate(spawn.z)
        .ok_or_else(|| "La coordonnée Z du spawn est invalide.".to_string())?;
    let min_y = world.get_min_y();
    let start_y = block_coordinate(spawn.y.ceil())
        .ok_or_else(|| "La coordonnée Y du spawn est invalide.".to_string())?
        .max(min_y);
    let max_feet_y = world
        .get_top_block_y(block_x, block_z)
        .saturating_add(1)
        .max(start_y)
        .min(i32::MAX - 1);

    for feet_y in start_y..=max_feet_y {
        let head_y = feet_y + 1;
        if world
            .get_block_state(BlockPos {
                x: block_x,
                y: feet_y,
                z: block_z,
            })
            .is_air
            && world
                .get_block_state(BlockPos {
                    x: block_x,
                    y: head_y,
                    z: block_z,
                })
                .is_air
        {
            return Ok((spawn.x, f64::from(feet_y), spawn.z));
        }
    }

    Err("Aucun emplacement libre n'a été trouvé au-dessus du spawn.".to_string())
}

fn block_coordinate(coordinate: f64) -> Option<i32> {
    let coordinate = coordinate.floor();
    if coordinate < f64::from(i32::MIN) || coordinate >= 2_147_483_648.0 {
        return None;
    }
    Some(coordinate as i32)
}

pub fn is_managed_world(name: &str) -> Result<bool, String> {
    let manager = world_manager()
        .lock()
        .map_err(|_| "Verrouillage du gestionnaire de mondes impossible.".to_string())?;
    let manager = manager
        .as_ref()
        .ok_or_else(|| "Le gestionnaire de mondes n'est pas initialisé.".to_string())?;
    Ok(manager.worlds.contains_key(name))
}

fn configure_world(world: &World, config: &WorldConfig) {
    let border = world.get_world_border();
    border.set_center(config.border.center.x, config.border.center.z);
    border.set_size(config.border.radius);
}

fn parse_config(contents: &str) -> Result<BTreeMap<String, WorldConfig>, String> {
    let config: WorldConfigurationFile = serde_yaml::from_str(contents)
        .map_err(|error| format!("Le fichier config.yml est invalide : {error}"))?;

    if config.worlds.is_empty() {
        return Err("Aucun monde n'est configuré dans config.yml.".to_string());
    }

    for (name, world) in &config.worlds {
        if name.trim().is_empty()
            || !world.spawn.x.is_finite()
            || !world.spawn.y.is_finite()
            || !world.spawn.z.is_finite()
            || !world.border.center.x.is_finite()
            || !world.border.center.y.is_finite()
            || !world.border.center.z.is_finite()
            || !world.border.radius.is_finite()
            || world.border.radius <= 0.0
        {
            return Err(format!(
                "La configuration du monde {name:?} contient des coordonnées ou une bordure invalides."
            ));
        }
    }

    Ok(config.worlds)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_CONFIG, parse_config};

    #[test]
    fn parses_default_world_configuration() {
        let worlds = parse_config(DEFAULT_CONFIG).expect("la configuration par défaut est valide");
        let world = worlds
            .get("world_infected_city")
            .expect("le monde par défaut est configuré");

        assert_eq!(world.spawn.x, 2.0);
        assert_eq!(world.spawn.y, -59.0);
        assert_eq!(world.spawn.z, 13.0);
        assert_eq!(world.border.radius, 200.0);
    }

    #[test]
    fn rejects_empty_world_configuration() {
        let error = parse_config("worlds: {}").expect_err("une liste vide doit être rejetée");

        assert!(error.contains("Aucun monde"));
    }

    #[test]
    fn rejects_non_positive_world_border_size() {
        let config = r#"
worlds:
  arena:
    spawn: { x: 0.0, y: 64.0, z: 0.0 }
    border:
      center: { x: 0.0, y: 64.0, z: 0.0 }
      radius: 0.0
"#;
        let error = parse_config(config).expect_err("une bordure nulle doit être rejetée");

        assert!(error.contains("bordure invalides"));
    }
}

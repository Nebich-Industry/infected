use std::sync::{Mutex, OnceLock};

static WORLD_NAME: OnceLock<Mutex<String>> = OnceLock::new();

pub fn world_name() -> &'static Mutex<String> {
    WORLD_NAME.get_or_init(|| Mutex::new(String::from("infected_world")))
}
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    Waiting,
    Active,
    Ended,
}

pub struct GameSession {
    state: GameState,
}

impl GameSession {
    pub fn new() -> Self {
        GameSession {
            state: GameState::Waiting,
        }
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }
}

static GAME_SESSION: OnceLock<Mutex<GameSession>> = OnceLock::new();

pub fn game_session() -> &'static Mutex<GameSession> {
    GAME_SESSION.get_or_init(|| Mutex::new(GameSession::new()))
}
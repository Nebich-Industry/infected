use std::sync::{Mutex, OnceLock};

const GAME_DURATION_SECONDS: u32 = 5 * 60;
const WAITING_DURATION_SECONDS: u32 = 2 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    Waiting,
    Active,
    Ended,
}

pub struct GameSession {
    state: GameState,
    game_timer: u32,
    waiting_timer: u32,
    launch_pending: bool,
}

impl GameSession {
    pub fn new() -> Self {
        GameSession {
            state: GameState::Waiting,
            game_timer: GAME_DURATION_SECONDS,
            waiting_timer: WAITING_DURATION_SECONDS,
            launch_pending: false,
        }
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn launch(&mut self) {
        self.state = GameState::Active;
        self.game_timer = GAME_DURATION_SECONDS;
        self.waiting_timer = WAITING_DURATION_SECONDS;
        self.launch_pending = false;
    }

    pub fn end(&mut self) {
        self.state = GameState::Waiting;
    }

    pub fn game_timer(&self) -> u32 {
        self.game_timer
    }

    pub fn decrement_game_timer(&mut self) {
        self.game_timer = self.game_timer.saturating_sub(1);
    }

    pub fn waiting_timer(&self) -> u32 {
        self.waiting_timer
    }

    pub fn tick_waiting_timer(&mut self) -> bool {
        self.waiting_timer = self.waiting_timer.saturating_sub(1);
        self.waiting_timer == 0
    }

    pub fn reset_waiting_timer(&mut self, seconds: u32) {
        self.waiting_timer = seconds;
    }

    pub fn begin_launch_countdown(&mut self) -> bool {
        if self.state == GameState::Active || self.launch_pending {
            return false;
        }

        self.launch_pending = true;
        true
    }

    pub fn cancel_launch_countdown(&mut self) {
        self.launch_pending = false;
    }

    pub fn launch_pending(&self) -> bool {
        self.launch_pending
    }
}

static GAME_SESSION: OnceLock<Mutex<GameSession>> = OnceLock::new();

pub fn game_session() -> &'static Mutex<GameSession> {
    GAME_SESSION.get_or_init(|| Mutex::new(GameSession::new()))
}

#[cfg(test)]
mod tests {
    use super::{GAME_DURATION_SECONDS, GameSession, GameState, WAITING_DURATION_SECONDS};

    #[test]
    fn launch_resets_state_and_game_timer() {
        let mut session = GameSession::new();
        session.decrement_game_timer();

        session.launch();

        assert_eq!(session.state(), &GameState::Active);
        assert_eq!(session.game_timer(), GAME_DURATION_SECONDS);
    }

    #[test]
    fn ending_returns_session_to_waiting() {
        let mut session = GameSession::new();
        session.launch();

        session.end();

        assert_eq!(session.state(), &GameState::Waiting);
    }

    #[test]
    fn timers_use_the_expected_durations_and_do_not_underflow() {
        let mut session = GameSession::new();

        assert_eq!(session.waiting_timer(), WAITING_DURATION_SECONDS);
        assert!(!session.tick_waiting_timer());
        session.reset_waiting_timer(1);
        assert!(session.tick_waiting_timer());
        assert!(session.tick_waiting_timer());
        for _ in 0..=GAME_DURATION_SECONDS {
            session.decrement_game_timer();
        }
        assert_eq!(session.game_timer(), 0);
    }

    #[test]
    fn launch_countdown_cannot_be_started_twice_or_during_a_game() {
        let mut session = GameSession::new();

        assert!(session.begin_launch_countdown());
        assert!(!session.begin_launch_countdown());
        session.launch();
        assert!(!session.begin_launch_countdown());
        assert!(!session.launch_pending());
    }
}

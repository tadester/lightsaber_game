#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    None,
    AttackLeft,
    AttackRight,
    ForcePush,
    GuardStart,
    GuardEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionSource {
    Keyboard,
    Camera,
    Network,
}

#[derive(Debug, Clone, Copy)]
pub struct ActionCommand {
    pub player_id: u8,
    pub action: PlayerAction,
    pub confidence: f32,
    pub timestamp_seconds: f64,
    pub source: ActionSource,
}

impl ActionCommand {
    pub fn new(
        player_id: u8,
        action: PlayerAction,
        confidence: f32,
        timestamp_seconds: f64,
        source: ActionSource,
    ) -> Self {
        Self {
            player_id,
            action,
            confidence,
            timestamp_seconds,
            source,
        }
    }
}

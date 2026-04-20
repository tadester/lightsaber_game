use crate::command::PlayerAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroneArchetype {
    Basic,
    Shield,
    Heavy,
}

#[derive(Debug, Clone)]
pub struct DroneState {
    pub id: u32,
    pub archetype: DroneArchetype,
    pub health: i32,
    pub lane: i8,
    pub progress: f32,
    pub shielded: bool,
    pub staggered_seconds: f32,
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub id: u8,
    pub health: i32,
    pub guarding: bool,
    pub score: i32,
    pub combo: u32,
    pub combo_expiry_seconds: f32,
    pub attack_cooldown_seconds: f32,
    pub force_cooldown_seconds: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CombatConfig {
    pub attack_cooldown_seconds: f32,
    pub force_cooldown_seconds: f32,
    pub combo_window_seconds: f32,
    pub player_max_health: i32,
}

impl Default for CombatConfig {
    fn default() -> Self {
        Self {
            attack_cooldown_seconds: 0.35,
            force_cooldown_seconds: 1.5,
            combo_window_seconds: 2.25,
            player_max_health: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CombatEvent {
    PlayerActionAccepted { player_id: u8, action: PlayerAction },
    PlayerActionRejected { player_id: u8, action: PlayerAction },
    DroneDestroyed { drone_id: u32, player_id: u8 },
    PlayerDamaged { player_id: u8, damage: i32 },
    ComboAdvanced { player_id: u8, combo: u32 },
    FitnessActionCounted { player_id: u8, action: PlayerAction },
}

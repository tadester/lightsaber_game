pub mod combat;
pub mod command;
pub mod fitness;
pub mod game_mode;
pub mod multiplayer;
pub mod simulation;

pub use combat::{CombatConfig, CombatEvent, DroneArchetype, DroneState, PlayerState};
pub use command::{ActionCommand, ActionSource, PlayerAction};
pub use fitness::{FitnessMetrics, FitnessPhase};
pub use game_mode::GameMode;
pub use multiplayer::{DuelScoreboard, MatchState};
pub use simulation::{GameState, SimulationConfig, SimulationTickResult};

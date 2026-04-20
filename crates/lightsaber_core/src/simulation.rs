use crate::combat::{CombatConfig, CombatEvent, DroneArchetype, DroneState, PlayerState};
use crate::command::{ActionCommand, PlayerAction};
use crate::fitness::{FitnessMetrics, FitnessPhase};
use crate::game_mode::GameMode;
use crate::multiplayer::MatchState;

#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub combat: CombatConfig,
    pub starting_drones: usize,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            combat: CombatConfig::default(),
            starting_drones: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub mode: GameMode,
    pub players: Vec<PlayerState>,
    pub drones: Vec<DroneState>,
    pub wave_index: u32,
    pub elapsed_seconds: f32,
    pub fitness: Option<FitnessMetrics>,
    pub match_state: Option<MatchState>,
}

#[derive(Debug, Default)]
pub struct SimulationTickResult {
    pub events: Vec<CombatEvent>,
}

impl GameState {
    pub fn new(mode: GameMode, config: &SimulationConfig, player_count: usize) -> Self {
        let players = (0..player_count)
            .map(|idx| PlayerState {
                id: idx as u8,
                health: config.combat.player_max_health,
                guarding: false,
                score: 0,
                combo: 0,
                combo_expiry_seconds: 0.0,
                attack_cooldown_seconds: 0.0,
                force_cooldown_seconds: 0.0,
            })
            .collect();

        let drones = (0..config.starting_drones)
            .map(|idx| DroneState {
                id: idx as u32,
                archetype: match idx % 3 {
                    0 => DroneArchetype::Basic,
                    1 => DroneArchetype::Shield,
                    _ => DroneArchetype::Heavy,
                },
                health: match idx % 3 {
                    0 => 1,
                    1 => 2,
                    _ => 3,
                },
                lane: (idx % 3) as i8 - 1,
                progress: 0.0,
                shielded: idx % 3 == 1,
                staggered_seconds: 0.0,
            })
            .collect();

        Self {
            mode,
            players,
            drones,
            wave_index: 1,
            elapsed_seconds: 0.0,
            fitness: (mode == GameMode::FitnessMode).then(FitnessMetrics::default),
            match_state: matches!(mode, GameMode::MultiplayerDuel).then(MatchState::default),
        }
    }

    pub fn update(&mut self, delta_seconds: f32) {
        self.elapsed_seconds += delta_seconds;

        for player in &mut self.players {
            player.attack_cooldown_seconds = (player.attack_cooldown_seconds - delta_seconds).max(0.0);
            player.force_cooldown_seconds = (player.force_cooldown_seconds - delta_seconds).max(0.0);
            player.combo_expiry_seconds = (player.combo_expiry_seconds - delta_seconds).max(0.0);

            if player.combo_expiry_seconds <= 0.0 {
                player.combo = 0;
            }
        }

        for drone in &mut self.drones {
            drone.progress += delta_seconds * 0.15;
            drone.staggered_seconds = (drone.staggered_seconds - delta_seconds).max(0.0);
        }

        let mut breach_damage = 0;
        self.drones.retain(|drone| {
            let breached = drone.progress >= 1.0;
            if breached {
                breach_damage += match drone.archetype {
                    DroneArchetype::Basic => 6,
                    DroneArchetype::Shield => 10,
                    DroneArchetype::Heavy => 16,
                };
            }
            !breached
        });

        if breach_damage > 0 {
            if let Some(player) = self.players.first_mut() {
                let applied = if player.guarding {
                    (breach_damage / 2).max(1)
                } else {
                    breach_damage
                };
                player.health = (player.health - applied).max(0);
            }
        }

        if !matches!(self.mode, GameMode::MultiplayerDuel) && self.drones.is_empty() {
            self.wave_index += 1;
            self.spawn_wave();
        }

        if let Some(fitness) = &mut self.fitness {
            fitness.active_time_seconds += delta_seconds;
            fitness.phase = if fitness.active_time_seconds < 45.0 {
                FitnessPhase::WarmUp
            } else if fitness.active_time_seconds < 150.0 {
                FitnessPhase::Peak
            } else {
                FitnessPhase::Recovery
            };
        }

        if let Some(match_state) = &mut self.match_state {
            match_state.round_time_remaining = (match_state.round_time_remaining - delta_seconds).max(0.0);
        }
    }

    pub fn apply_command(&mut self, config: &SimulationConfig, command: ActionCommand) -> SimulationTickResult {
        let mut result = SimulationTickResult::default();
        let Some(player_index) = self
            .players
            .iter()
            .position(|player| player.id == command.player_id) else {
            return result;
        };

        match command.action {
            PlayerAction::AttackLeft | PlayerAction::AttackRight => {
                if self.players[player_index].attack_cooldown_seconds > 0.0 {
                    result.events.push(CombatEvent::PlayerActionRejected {
                        player_id: self.players[player_index].id,
                        action: command.action,
                    });
                    return result;
                }

                let player_id = self.players[player_index].id;
                self.players[player_index].guarding = false;
                self.players[player_index].attack_cooldown_seconds = config.combat.attack_cooldown_seconds;
                result.events.push(CombatEvent::PlayerActionAccepted {
                    player_id,
                    action: command.action,
                });
                if self.mode == GameMode::MultiplayerDuel {
                    self.resolve_duel_attack(player_id, command.action, &mut result);
                } else {
                    self.resolve_slash(player_id, command.action, &mut result);
                }
            }
            PlayerAction::ForcePush => {
                if self.players[player_index].force_cooldown_seconds > 0.0 {
                    result.events.push(CombatEvent::PlayerActionRejected {
                        player_id: self.players[player_index].id,
                        action: command.action,
                    });
                    return result;
                }

                let player_id = self.players[player_index].id;
                self.players[player_index].guarding = false;
                self.players[player_index].force_cooldown_seconds = config.combat.force_cooldown_seconds;
                result.events.push(CombatEvent::PlayerActionAccepted {
                    player_id,
                    action: command.action,
                });
                if self.mode == GameMode::MultiplayerDuel {
                    self.resolve_duel_force_push(player_id, &mut result);
                } else {
                    self.resolve_force_push(player_id, &mut result);
                }
            }
            PlayerAction::GuardStart => {
                self.players[player_index].guarding = true;
                result.events.push(CombatEvent::PlayerActionAccepted {
                    player_id: self.players[player_index].id,
                    action: command.action,
                });
            }
            PlayerAction::GuardEnd => {
                self.players[player_index].guarding = false;
                result.events.push(CombatEvent::PlayerActionAccepted {
                    player_id: self.players[player_index].id,
                    action: command.action,
                });
            }
            PlayerAction::None => {}
        }

        result
    }

    fn resolve_slash(&mut self, player_id: u8, action: PlayerAction, result: &mut SimulationTickResult) {
        let target_lane = match action {
            PlayerAction::AttackLeft => -1,
            PlayerAction::AttackRight => 1,
            _ => 0,
        };

        let mut destroyed_id = None;

        for drone in &mut self.drones {
            let attack_matches = drone.lane == target_lane || drone.archetype == DroneArchetype::Basic;
            let shield_blocks = drone.archetype == DroneArchetype::Shield && action == PlayerAction::AttackLeft;

            if attack_matches && !shield_blocks {
                drone.health -= 1;
                drone.shielded = false;
                if drone.health <= 0 {
                    destroyed_id = Some(drone.id);
                    break;
                }
            }
        }

        if let Some(drone_id) = destroyed_id {
            self.drones.retain(|drone| drone.id != drone_id);
            self.reward_success(player_id, action, result, 10);
            result.events.push(CombatEvent::DroneDestroyed { drone_id, player_id });
        } else {
            self.reset_combo(player_id);
            result.events.push(CombatEvent::PlayerActionRejected { player_id, action });
        }
    }

    fn resolve_force_push(&mut self, player_id: u8, result: &mut SimulationTickResult) {
        let mut destroyed = Vec::new();

        for drone in &mut self.drones {
            if drone.progress > 0.15 {
                drone.health -= if drone.archetype == DroneArchetype::Heavy { 1 } else { 2 };
                drone.staggered_seconds = 0.75;
                if drone.health <= 0 {
                    destroyed.push(drone.id);
                }
            }
        }

        if destroyed.is_empty() {
            self.reset_combo(player_id);
            result.events.push(CombatEvent::PlayerActionRejected {
                player_id,
                action: PlayerAction::ForcePush,
            });
        } else {
            self.drones.retain(|drone| !destroyed.contains(&drone.id));
            self.reward_success(player_id, PlayerAction::ForcePush, result, 14);
            for drone_id in destroyed {
                result.events.push(CombatEvent::DroneDestroyed { drone_id, player_id });
            }
        }
    }

    fn reward_success(
        &mut self,
        player_id: u8,
        action: PlayerAction,
        result: &mut SimulationTickResult,
        base_score: i32,
    ) {
        let Some(player) = self.players.iter_mut().find(|player| player.id == player_id) else {
            return;
        };

        player.combo += 1;
        player.combo_expiry_seconds = 2.25;
        player.score += base_score + (player.combo as i32 - 1) * 2;
        result
            .events
            .push(CombatEvent::ComboAdvanced { player_id, combo: player.combo });

        if let Some(fitness) = &mut self.fitness {
            match action {
                PlayerAction::AttackLeft | PlayerAction::AttackRight => fitness.swings_performed += 1,
                PlayerAction::ForcePush => fitness.force_pushes_performed += 1,
                _ => {}
            }
            fitness.successful_actions += 1;
            fitness.best_combo = fitness.best_combo.max(player.combo);
            fitness.estimated_effort_score += match action {
                PlayerAction::ForcePush => 2.2,
                PlayerAction::AttackLeft | PlayerAction::AttackRight => 1.0,
                _ => 0.2,
            };
            result
                .events
                .push(CombatEvent::FitnessActionCounted { player_id, action });
        }
    }

    fn reset_combo(&mut self, player_id: u8) {
        if let Some(player) = self.players.iter_mut().find(|player| player.id == player_id) {
            player.combo = 0;
            player.combo_expiry_seconds = 0.0;
        }
    }

    fn resolve_duel_attack(&mut self, player_id: u8, action: PlayerAction, result: &mut SimulationTickResult) {
        let Some(target_index) = self.players.iter().position(|player| player.id != player_id) else {
            self.reset_combo(player_id);
            return;
        };

        let target_id = self.players[target_index].id;
        let damage = if self.players[target_index].guarding { 4 } else { 10 };
        self.players[target_index].health = (self.players[target_index].health - damage).max(0);
        result
            .events
            .push(CombatEvent::PlayerDamaged { player_id: target_id, damage });
        self.reward_success(player_id, action, result, 12);

        if self.players[target_index].health <= 0 {
            if let Some(match_state) = &mut self.match_state {
                if player_id == 0 {
                    match_state.duel_scoreboard.player_one_rounds += 1;
                } else {
                    match_state.duel_scoreboard.player_two_rounds += 1;
                }
                match_state.round_time_remaining = 90.0;
            }

            for player in &mut self.players {
                player.health = 100;
                player.combo = 0;
                player.combo_expiry_seconds = 0.0;
                player.guarding = false;
            }
        }
    }

    fn resolve_duel_force_push(&mut self, player_id: u8, result: &mut SimulationTickResult) {
        let Some(target_index) = self.players.iter().position(|player| player.id != player_id) else {
            self.reset_combo(player_id);
            return;
        };

        let target_id = self.players[target_index].id;
        let damage = if self.players[target_index].guarding { 2 } else { 8 };
        self.players[target_index].health = (self.players[target_index].health - damage).max(0);
        result
            .events
            .push(CombatEvent::PlayerDamaged { player_id: target_id, damage });
        self.reward_success(player_id, PlayerAction::ForcePush, result, 14);

        if self.players[target_index].health <= 0 {
            if let Some(match_state) = &mut self.match_state {
                if player_id == 0 {
                    match_state.duel_scoreboard.player_one_rounds += 1;
                } else {
                    match_state.duel_scoreboard.player_two_rounds += 1;
                }
                match_state.round_time_remaining = 90.0;
            }

            for player in &mut self.players {
                player.health = 100;
                player.combo = 0;
                player.combo_expiry_seconds = 0.0;
                player.guarding = false;
            }
        }
    }

    fn spawn_wave(&mut self) {
        let count = 2 + self.wave_index as usize;
        self.drones = (0..count)
            .map(|idx| DroneState {
                id: (self.wave_index * 100 + idx as u32),
                archetype: match idx % 3 {
                    0 => DroneArchetype::Basic,
                    1 => DroneArchetype::Shield,
                    _ => DroneArchetype::Heavy,
                },
                health: match idx % 3 {
                    0 => 1,
                    1 => 2,
                    _ => 3,
                },
                lane: (idx % 3) as i8 - 1,
                progress: 0.0,
                shielded: idx % 3 == 1,
                staggered_seconds: 0.0,
            })
            .collect();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitnessPhase {
    WarmUp,
    Peak,
    Recovery,
}

#[derive(Debug, Clone)]
pub struct FitnessMetrics {
    pub swings_performed: u32,
    pub force_pushes_performed: u32,
    pub active_time_seconds: f32,
    pub successful_actions: u32,
    pub best_combo: u32,
    pub estimated_effort_score: f32,
    pub phase: FitnessPhase,
}

impl Default for FitnessMetrics {
    fn default() -> Self {
        Self {
            swings_performed: 0,
            force_pushes_performed: 0,
            active_time_seconds: 0.0,
            successful_actions: 0,
            best_combo: 0,
            estimated_effort_score: 0.0,
            phase: FitnessPhase::WarmUp,
        }
    }
}

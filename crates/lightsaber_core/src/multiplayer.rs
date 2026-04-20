#[derive(Debug, Clone, Copy, Default)]
pub struct DuelScoreboard {
    pub player_one_rounds: u8,
    pub player_two_rounds: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct MatchState {
    pub round_time_remaining: f32,
    pub rounds_to_win: u8,
    pub duel_scoreboard: DuelScoreboard,
}

impl Default for MatchState {
    fn default() -> Self {
        Self {
            round_time_remaining: 90.0,
            rounds_to_win: 2,
            duel_scoreboard: DuelScoreboard::default(),
        }
    }
}

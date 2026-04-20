use lightsaber_core::{ActionCommand, ActionSource, PlayerAction};

pub fn parse_keyboard_action(input: &str, player_id: u8, now_seconds: f64) -> Option<ActionCommand> {
    let action = match input.trim().to_ascii_lowercase().as_str() {
        "a" | "left" | "slash_left" => PlayerAction::AttackLeft,
        "d" | "right" | "slash_right" => PlayerAction::AttackRight,
        "w" | "push" | "force_push" => PlayerAction::ForcePush,
        "s" | "guard_start" | "guard" => PlayerAction::GuardStart,
        "guard_end" => PlayerAction::GuardEnd,
        _ => PlayerAction::None,
    };

    (action != PlayerAction::None).then(|| ActionCommand::new(player_id, action, 1.0, now_seconds, ActionSource::Keyboard))
}

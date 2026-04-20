use lightsaber_core::{ActionCommand, ActionSource, PlayerAction};

pub fn parse_action_command_json(payload: &str) -> Option<ActionCommand> {
    let gesture = extract_string(payload, "gesture")?;
    let confidence = extract_number(payload, "confidence").unwrap_or(1.0) as f32;
    let timestamp = extract_number(payload, "timestamp").unwrap_or(0.0);
    let player_id = extract_number(payload, "playerId").unwrap_or(0.0) as u8;

    let action = match gesture.as_str() {
        "slash_left" => PlayerAction::AttackLeft,
        "slash_right" => PlayerAction::AttackRight,
        "force_push" => PlayerAction::ForcePush,
        "guard_start" => PlayerAction::GuardStart,
        "guard_end" => PlayerAction::GuardEnd,
        _ => PlayerAction::None,
    };

    (action != PlayerAction::None).then(|| ActionCommand::new(player_id, action, confidence, timestamp, ActionSource::Camera))
}

fn extract_string(payload: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\"");
    let start = payload.find(&pattern)?;
    let after_key = &payload[start + pattern.len()..];
    let first_quote = after_key.find('"')?;
    let after_first_quote = &after_key[first_quote + 1..];
    let second_quote = after_first_quote.find('"')?;
    Some(after_first_quote[..second_quote].to_string())
}

fn extract_number(payload: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{key}\"");
    let start = payload.find(&pattern)?;
    let after_key = &payload[start + pattern.len()..];
    let colon = after_key.find(':')?;
    let value = after_key[colon + 1..]
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.' || *ch == '-')
        .collect::<String>();
    value.parse::<f64>().ok()
}

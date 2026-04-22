use bevy::prelude::*;
use bevy::window::WindowResolution;
use lightsaber_core::{
    ActionCommand, ActionSource, GameMode, GameState, PlayerAction, SimulationConfig,
};
use lightsaber_runtime::UdpGestureReceiver;

const ARENA_WIDTH: f32 = 1280.0;
const ARENA_HEIGHT: f32 = 720.0;
const PLAYER_X: f32 = -430.0;
const DUEL_PLAYER_TWO_X: f32 = 430.0;
const DRONE_START_X: f32 = 520.0;
const LANE_Y: [f32; 3] = [-170.0, 0.0, 170.0];

fn main() {
    let mode = parse_mode_from_args();

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.04, 0.09)))
        .insert_resource(AppMode(mode))
        .insert_resource(SimResource::new(mode))
        .insert_resource(UdpResource::new("127.0.0.1:7777"))
        .insert_resource(StatusMessage::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Lightsaber Game".into(),
                resolution: WindowResolution::new(ARENA_WIDTH as u32, ARENA_HEIGHT as u32),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                keyboard_input_system,
                udp_input_system,
                simulation_step_system,
                sync_world_system,
                sync_hud_system,
                animate_effects_system,
            ),
        )
        .run();
}

#[derive(Resource)]
struct AppMode(GameMode);

#[derive(Resource)]
struct SimResource {
    config: SimulationConfig,
    state: GameState,
}

impl SimResource {
    fn new(mode: GameMode) -> Self {
        let config = SimulationConfig::default();
        let player_count = if matches!(mode, GameMode::MultiplayerDuel | GameMode::MultiplayerCoop) {
            2
        } else {
            1
        };
        let state = GameState::new(mode, &config, player_count);
        Self { config, state }
    }
}

#[derive(Resource)]
struct UdpResource {
    receiver: Option<UdpGestureReceiver>,
}

impl UdpResource {
    fn new(addr: &str) -> Self {
        let receiver = UdpGestureReceiver::bind(addr).ok();
        Self { receiver }
    }
}

#[derive(Resource, Default)]
struct StatusMessage {
    label: String,
    ttl: f32,
}

#[derive(Component)]
struct PlayerVisual {
    id: u8,
}

#[derive(Component)]
struct DroneVisual {
    drone_id: u32,
}

#[derive(Component)]
struct ParallaxLayer {
    speed: f32,
    wrap: f32,
}

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct HudScore;

#[derive(Component)]
struct HudMode;

#[derive(Component)]
struct HudHealth;

#[derive(Component)]
struct HudCombo;

#[derive(Component)]
struct HudWave;

#[derive(Component)]
struct HudFitness;

#[derive(Component)]
struct HudStatus;

#[derive(Component)]
struct PulseFx {
    ttl: f32,
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>, app_mode: Res<AppMode>) {
    commands.spawn(Camera2d);

    spawn_background(&mut commands);
    spawn_players(&mut commands, app_mode.0);
    spawn_hud(&mut commands, &asset_server);
}

fn spawn_background(commands: &mut Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.04, 0.08, 0.15), Vec2::new(ARENA_WIDTH, ARENA_HEIGHT)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));

    for idx in 0..2 {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.1, 0.28, 0.45, 0.33), Vec2::new(800.0, 260.0)),
            Transform::from_xyz(-240.0 + idx as f32 * 760.0, 220.0, -10.0),
            ParallaxLayer {
                speed: 18.0,
                wrap: 1520.0,
            },
        ));
    }

    for idx in 0..3 {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.17, 0.5, 0.78, 0.22), Vec2::new(330.0, 110.0)),
            Transform::from_xyz(-420.0 + idx as f32 * 420.0, -240.0, -5.0),
            ParallaxLayer {
                speed: 32.0,
                wrap: 1260.0,
            },
        ));
    }

    commands.spawn((
        Sprite::from_color(Color::srgb(0.05, 0.13, 0.2), Vec2::new(ARENA_WIDTH, 170.0)),
        Transform::from_xyz(0.0, -275.0, -2.0),
    ));
}

fn spawn_players(commands: &mut Commands, mode: GameMode) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.4, 0.85, 1.0), Vec2::new(46.0, 120.0)),
        Transform::from_xyz(PLAYER_X, 0.0, 5.0),
        PlayerVisual { id: 0 },
    ));

    if matches!(mode, GameMode::MultiplayerDuel | GameMode::MultiplayerCoop) {
        commands.spawn((
            Sprite::from_color(Color::srgb(1.0, 0.55, 0.3), Vec2::new(46.0, 120.0)),
            Transform::from_xyz(DUEL_PLAYER_TWO_X, 0.0, 5.0),
            PlayerVisual { id: 1 },
        ));
    }
}

fn spawn_hud(commands: &mut Commands, asset_server: &AssetServer) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(px(18.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            HudRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|left| {
                    left.spawn((
                        Text::new("MODE"),
                        TextFont {
                            font: font.clone(),
                            font_size: 34.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        HudMode,
                    ));
                    left.spawn((
                        Text::new("HP"),
                        TextFont {
                            font: font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        HudHealth,
                    ));
                    left.spawn((
                        Text::new("COMBO"),
                        TextFont {
                            font: font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        HudCombo,
                    ));
                    left.spawn((
                        Text::new("WAVE"),
                        TextFont {
                            font: font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        HudWave,
                    ));
                });

            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::End,
                        row_gap: px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|right| {
                    right.spawn((
                        Text::new("SCORE"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        HudScore,
                    ));
                    right.spawn((
                        Text::new("FITNESS"),
                        TextFont {
                            font: font.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 1.0, 0.8)),
                        HudFitness,
                    ));
                    right.spawn((
                        Text::new("STATUS"),
                        TextFont {
                            font: font.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.85, 0.5)),
                        HudStatus,
                    ));
                });
        });
}

fn keyboard_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    app_mode: Res<AppMode>,
    mut sim: ResMut<SimResource>,
) {
    let now = time.elapsed_secs_f64();
    let config = sim.config.clone();

    let mut commands = Vec::new();

    if keys.just_pressed(KeyCode::KeyA) {
        commands.push(ActionCommand::new(0, PlayerAction::AttackLeft, 1.0, now, ActionSource::Keyboard));
    }
    if keys.just_pressed(KeyCode::KeyD) {
        commands.push(ActionCommand::new(0, PlayerAction::AttackRight, 1.0, now, ActionSource::Keyboard));
    }
    if keys.just_pressed(KeyCode::KeyW) {
        commands.push(ActionCommand::new(0, PlayerAction::ForcePush, 1.0, now, ActionSource::Keyboard));
    }
    if keys.just_pressed(KeyCode::KeyS) {
        commands.push(ActionCommand::new(0, PlayerAction::GuardStart, 1.0, now, ActionSource::Keyboard));
    }
    if keys.just_released(KeyCode::KeyS) {
        commands.push(ActionCommand::new(0, PlayerAction::GuardEnd, 1.0, now, ActionSource::Keyboard));
    }

    if matches!(app_mode.0, GameMode::MultiplayerDuel | GameMode::MultiplayerCoop) {
        if keys.just_pressed(KeyCode::KeyJ) {
            commands.push(ActionCommand::new(1, PlayerAction::AttackLeft, 1.0, now, ActionSource::Keyboard));
        }
        if keys.just_pressed(KeyCode::KeyL) {
            commands.push(ActionCommand::new(1, PlayerAction::AttackRight, 1.0, now, ActionSource::Keyboard));
        }
        if keys.just_pressed(KeyCode::KeyI) {
            commands.push(ActionCommand::new(1, PlayerAction::ForcePush, 1.0, now, ActionSource::Keyboard));
        }
        if keys.just_pressed(KeyCode::KeyK) {
            commands.push(ActionCommand::new(1, PlayerAction::GuardStart, 1.0, now, ActionSource::Keyboard));
        }
        if keys.just_released(KeyCode::KeyK) {
            commands.push(ActionCommand::new(1, PlayerAction::GuardEnd, 1.0, now, ActionSource::Keyboard));
        }
    }

    for command in commands {
        let _ = sim.state.apply_command(&config, command);
    }
}

fn udp_input_system(mut sim: ResMut<SimResource>, udp: Res<UdpResource>) {
    let Some(receiver) = &udp.receiver else {
        return;
    };
    let config = sim.config.clone();

    for command in receiver.drain() {
        let _ = sim.state.apply_command(&config, command);
    }
}

fn simulation_step_system(
    time: Res<Time>,
    mut commands: Commands,
    mut sim: ResMut<SimResource>,
    mut status: ResMut<StatusMessage>,
) {
    let pre_wave = sim.state.wave_index;
    let pre_drones = sim.state.drones.len();
    let pre_scores = sim.state.players.iter().map(|player| player.score).collect::<Vec<_>>();
    let pre_health = sim.state.players.iter().map(|player| player.health).collect::<Vec<_>>();

    sim.state.update(time.delta_secs());

    if sim.state.wave_index > pre_wave {
        status.label = format!("Wave {} engaged", sim.state.wave_index);
        status.ttl = 1.4;
        spawn_pulse(&mut commands, Color::srgba(0.4, 0.85, 1.0, 0.18), 0.85);
    }

    if sim.state.drones.len() < pre_drones {
        status.label = "Drone destroyed".into();
        status.ttl = 0.7;
        spawn_pulse(&mut commands, Color::srgba(1.0, 0.74, 0.28, 0.18), 0.35);
    }

    if sim
        .state
        .players
        .iter()
        .map(|player| player.score)
        .zip(pre_scores)
        .any(|(after, before)| after > before)
    {
        status.label = "Clean hit".into();
        status.ttl = 0.45;
    }

    if sim
        .state
        .players
        .iter()
        .map(|player| player.health)
        .zip(pre_health)
        .any(|(after, before)| after < before)
    {
        status.label = "Impact taken".into();
        status.ttl = 0.65;
        spawn_pulse(&mut commands, Color::srgba(1.0, 0.2, 0.35, 0.14), 0.4);
    }

    status.ttl = (status.ttl - time.delta_secs()).max(0.0);
    if status.ttl <= 0.0 && status.label.is_empty() {
        status.label = "Camera UDP ready on 127.0.0.1:7777".into();
    }
}

fn spawn_pulse(commands: &mut Commands, color: Color, ttl: f32) {
    commands.spawn((
        Sprite::from_color(color, Vec2::new(ARENA_WIDTH, ARENA_HEIGHT)),
        Transform::from_xyz(0.0, 0.0, 30.0),
        PulseFx { ttl },
    ));
}

fn sync_world_system(
    mut commands: Commands,
    mut player_query: Query<(&PlayerVisual, &mut Transform, &mut Sprite)>,
    drone_query: Query<(Entity, &DroneVisual)>,
    sim: Res<SimResource>,
    app_mode: Res<AppMode>,
) {
    for (player_visual, mut transform, mut sprite) in &mut player_query {
        if let Some(player) = sim.state.players.iter().find(|player| player.id == player_visual.id) {
            transform.translation.x = if player_visual.id == 0 {
                PLAYER_X
            } else {
                DUEL_PLAYER_TWO_X
            };

            transform.translation.y = if player.guarding { -20.0 } else { 0.0 };
            sprite.color = if player.guarding {
                if player_visual.id == 0 {
                    Color::srgb(0.6, 0.95, 1.0)
                } else {
                    Color::srgb(1.0, 0.74, 0.52)
                }
            } else if player_visual.id == 0 {
                Color::srgb(0.4, 0.85, 1.0)
            } else {
                Color::srgb(1.0, 0.55, 0.3)
            };
        }
    }

    for (entity, drone_visual) in &drone_query {
        if !sim.state.drones.iter().any(|drone| drone.id == drone_visual.drone_id) {
            commands.entity(entity).despawn();
        }
    }

    for drone in &sim.state.drones {
        if drone_query.iter().any(|(_, visual)| visual.drone_id == drone.id) {
            continue;
        }

        let color = match drone.archetype {
            lightsaber_core::DroneArchetype::Basic => Color::srgb(0.51, 0.91, 1.0),
            lightsaber_core::DroneArchetype::Shield => Color::srgb(0.98, 0.8, 0.36),
            lightsaber_core::DroneArchetype::Heavy => Color::srgb(1.0, 0.45, 0.35),
        };

        commands.spawn((
            Sprite::from_color(color, Vec2::new(64.0, 64.0)),
            Transform::from_xyz(DRONE_START_X, lane_to_y(drone.lane), 4.0),
            DroneVisual { drone_id: drone.id },
        ));
    }

    let _ = app_mode;
}

fn sync_hud_system(
    app_mode: Res<AppMode>,
    sim: Res<SimResource>,
    status: Res<StatusMessage>,
    mut mode_query: Query<&mut Text, With<HudMode>>,
    mut score_query: Query<&mut Text, (With<HudScore>, Without<HudMode>)>,
    mut health_query: Query<&mut Text, (With<HudHealth>, Without<HudMode>)>,
    mut combo_query: Query<&mut Text, (With<HudCombo>, Without<HudMode>)>,
    mut wave_query: Query<&mut Text, (With<HudWave>, Without<HudMode>)>,
    mut fitness_query: Query<&mut Text, (With<HudFitness>, Without<HudMode>)>,
    mut status_query: Query<&mut Text, (With<HudStatus>, Without<HudMode>)>,
) {
    if let Ok(mut text) = mode_query.single_mut() {
        *text = Text::new(match app_mode.0 {
            GameMode::SoloCombat => "SOLO COMBAT",
            GameMode::FitnessMode => "FITNESS MODE",
            GameMode::MultiplayerDuel => "DUEL MODE",
            GameMode::MultiplayerCoop => "CO-OP MODE",
        });
    }

    if let Some(player) = sim.state.players.first() {
        if let Ok(mut text) = score_query.single_mut() {
            *text = Text::new(format!("Score {}", player.score));
        }
        if let Ok(mut text) = health_query.single_mut() {
            let duel_suffix = sim
                .state
                .players
                .get(1)
                .map(|p2| format!(" | P2 {}", p2.health))
                .unwrap_or_default();
            *text = Text::new(format!("HP {}{}", player.health, duel_suffix));
        }
        if let Ok(mut text) = combo_query.single_mut() {
            let duel_combo = sim
                .state
                .players
                .get(1)
                .map(|p2| format!(" | P2 x{}", p2.combo))
                .unwrap_or_default();
            *text = Text::new(format!("Combo x{}{}", player.combo, duel_combo));
        }
    }

    if let Ok(mut text) = wave_query.single_mut() {
        let duel_text = sim.state.match_state.map(|match_state| {
            format!(
                "Round {:.0}s | {}-{}",
                match_state.round_time_remaining,
                match_state.duel_scoreboard.player_one_rounds,
                match_state.duel_scoreboard.player_two_rounds
            )
        });
        *text = Text::new(
            duel_text.unwrap_or_else(|| format!("Wave {} | Drones {}", sim.state.wave_index, sim.state.drones.len())),
        );
    }

    if let Ok(mut text) = fitness_query.single_mut() {
        if let Some(fitness) = &sim.state.fitness {
            *text = Text::new(format!(
                "Moves {} | Pushes {} | Effort {:.1} | {:?}",
                fitness.swings_performed,
                fitness.force_pushes_performed,
                fitness.estimated_effort_score,
                fitness.phase
            ));
        } else {
            *text = Text::new("A/D/W/S or camera bridge for player 1");
        }
    }

    if let Ok(mut text) = status_query.single_mut() {
        *text = Text::new(if status.label.is_empty() {
            "Camera UDP ready on 127.0.0.1:7777".to_string()
        } else {
            status.label.clone()
        });
    }
}

fn animate_effects_system(
    time: Res<Time>,
    mut layer_query: Query<(&mut Transform, &ParallaxLayer), Without<DroneVisual>>,
    mut drone_query: Query<(&DroneVisual, &mut Transform, &mut Sprite)>,
    mut pulse_query: Query<(Entity, &mut Sprite, &mut PulseFx)>,
    mut commands: Commands,
    sim: Res<SimResource>,
) {
    for (mut transform, layer) in &mut layer_query {
        transform.translation.x -= layer.speed * time.delta_secs();
        if transform.translation.x < -layer.wrap * 0.5 {
            transform.translation.x += layer.wrap;
        }
    }

    for (visual, mut transform, mut sprite) in &mut drone_query {
        if let Some(drone) = sim.state.drones.iter().find(|drone| drone.id == visual.drone_id) {
            let x = DRONE_START_X - drone.progress * 900.0;
            transform.translation.x = x;
            transform.translation.y = lane_to_y(drone.lane);
            transform.scale = Vec3::splat(1.0 + (drone.staggered_seconds * 0.2));
            sprite.color.set_alpha(1.0 - (drone.progress * 0.15).min(0.3));
        }
    }

    for (entity, mut sprite, mut pulse) in &mut pulse_query {
        pulse.ttl -= time.delta_secs();
        let alpha = (pulse.ttl / 0.85).clamp(0.0, 1.0) * 0.25;
        sprite.color.set_alpha(alpha);
        if pulse.ttl <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn parse_mode_from_args() -> GameMode {
    match std::env::args().nth(1).as_deref() {
        Some("fitness") => GameMode::FitnessMode,
        Some("duel") => GameMode::MultiplayerDuel,
        Some("coop") => GameMode::MultiplayerCoop,
        _ => GameMode::SoloCombat,
    }
}

fn lane_to_y(lane: i8) -> f32 {
    match lane {
        -1 => LANE_Y[0],
        0 => LANE_Y[1],
        1 => LANE_Y[2],
        _ => 0.0,
    }
}

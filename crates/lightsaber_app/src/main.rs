use bevy::prelude::*;
use bevy::window::WindowResolution;
use lightsaber_core::{
    ActionCommand, ActionSource, CombatEvent, DroneArchetype, GameMode, GameState, PlayerAction,
    SimulationConfig,
};
use lightsaber_runtime::UdpGestureReceiver;
use std::collections::HashMap;
use std::env;
use std::process::{Child, Command, Stdio};

const ARENA_WIDTH: f32 = 1280.0;
const ARENA_HEIGHT: f32 = 720.0;
const PLAYER_X: f32 = -430.0;
const DUEL_PLAYER_TWO_X: f32 = 430.0;
const DRONE_START_X: f32 = 520.0;
const FLOOR_Y: f32 = -245.0;
const LANE_Y: [f32; 3] = [-170.0, 0.0, 170.0];
const PLAYER_MAX_HEALTH: f32 = 100.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.04, 0.09)))
        .insert_resource(RuntimeSettings::default())
        .insert_resource(SimResource::new(GameMode::SoloCombat))
        .insert_resource(UdpResource::new("127.0.0.1:7777"))
        .insert_resource(PythonBridgeProcess::default())
        .insert_resource(ObstacleSpawner::default())
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
        .init_state::<AppScreen>()
        .add_systems(Startup, setup_base_scene)
        .add_systems(OnEnter(AppScreen::MainMenu), spawn_menu)
        .add_systems(OnExit(AppScreen::MainMenu), cleanup_menu)
        .add_systems(OnEnter(AppScreen::Playing), setup_gameplay)
        .add_systems(OnExit(AppScreen::Playing), cleanup_gameplay)
        .add_systems(
            Update,
            (
                menu_button_system.run_if(in_state(AppScreen::MainMenu)),
                sync_menu_labels_system.run_if(in_state(AppScreen::MainMenu)),
                python_bridge_monitor_system,
                keyboard_input_system.run_if(in_state(AppScreen::Playing)),
                udp_input_system.run_if(in_state(AppScreen::Playing)),
                back_to_menu_system.run_if(in_state(AppScreen::Playing)),
                simulation_step_system.run_if(in_state(AppScreen::Playing)),
                sync_world_system.run_if(in_state(AppScreen::Playing)),
                sync_hud_system.run_if(in_state(AppScreen::Playing)),
                obstacle_system.run_if(in_state(AppScreen::Playing)),
                animate_player_parts_system.run_if(in_state(AppScreen::Playing)),
                animate_background_system,
                animate_effects_system.run_if(in_state(AppScreen::Playing)),
            ),
        )
        .run();
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AppScreen {
    #[default]
    MainMenu,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Keyboard,
    Python,
    Hybrid,
}

impl InputMode {
    fn label(self) -> &'static str {
        match self {
            InputMode::Keyboard => "Keyboard Only",
            InputMode::Python => "Python Camera",
            InputMode::Hybrid => "Keyboard + Python",
        }
    }

    fn uses_keyboard(self) -> bool {
        matches!(self, InputMode::Keyboard | InputMode::Hybrid)
    }

    fn uses_python(self) -> bool {
        matches!(self, InputMode::Python | InputMode::Hybrid)
    }
}

#[derive(Resource)]
struct RuntimeSettings {
    mode: GameMode,
    input_mode: InputMode,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            mode: GameMode::SoloCombat,
            input_mode: InputMode::Keyboard,
        }
    }
}

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

#[derive(Resource, Default)]
struct PythonBridgeProcess {
    child: Option<Child>,
    last_error: Option<String>,
}

impl PythonBridgeProcess {
    fn ensure_running(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(None) => return true,
                Ok(Some(status)) => {
                    self.last_error = Some(format!("Python bridge exited: {status}"));
                    self.child = None;
                }
                Err(error) => {
                    self.last_error = Some(format!("Python bridge check failed: {error}"));
                    self.child = None;
                }
            }
        }

        let script = env::current_dir()
            .ok()
            .map(|cwd| cwd.join("Tools/gesture_bridge/mediapipe_bridge.py"));
        let Some(script) = script.filter(|path| path.exists()) else {
            self.last_error = Some("Python bridge script not found".into());
            return false;
        };

        match Command::new("python3")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                self.child = Some(child);
                self.last_error = None;
                true
            }
            Err(error) => {
                self.last_error = Some(format!("Could not launch Python bridge: {error}"));
                false
            }
        }
    }
}

#[derive(Resource)]
struct ObstacleSpawner {
    timer: f32,
    next_id: u32,
}

impl Default for ObstacleSpawner {
    fn default() -> Self {
        Self {
            timer: 2.2,
            next_id: 1,
        }
    }
}

#[derive(Component)]
struct GameplayEntity;

#[derive(Component)]
struct MenuEntity;

#[derive(Component)]
struct PlayerVisual {
    id: u8,
    action_timer: f32,
    jump_timer: f32,
    hurt_timer: f32,
}

#[derive(Component)]
struct DroneVisual {
    drone_id: u32,
}

#[derive(Component)]
struct PlayerLeg {
    player_id: u8,
    side: f32,
}

#[derive(Component)]
struct SaberFx {
    ttl: f32,
    total: f32,
    action: PlayerAction,
}

#[derive(Component)]
struct ForceWaveFx {
    ttl: f32,
    total: f32,
}

#[derive(Component)]
struct ExplosionFx {
    ttl: f32,
    total: f32,
    velocity: Vec2,
}

#[derive(Component)]
struct ObstacleVisual {
    hit: bool,
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
struct HealthBarFill {
    player_id: u8,
    max_width: f32,
}

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

#[derive(Component)]
struct MenuSelectionText;

#[derive(Component, Clone, Copy)]
enum MenuButtonAction {
    SelectInput(InputMode),
    SelectMode(GameMode),
    StartGame,
}

#[derive(Bundle)]
struct MenuButtonBundle {
    button: Button,
    node: Node,
    background_color: BackgroundColor,
    action: MenuButtonAction,
}

fn setup_base_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
    spawn_background(&mut commands);
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

    for idx in 0..12 {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.16, 0.9, 1.0, 0.18), Vec2::new(86.0, 3.0)),
            Transform::from_xyz(-610.0 + idx as f32 * 112.0, FLOOR_Y - 42.0, -1.0),
            ParallaxLayer {
                speed: 70.0,
                wrap: 1344.0,
            },
        ));
    }
}

fn spawn_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.02, 0.05, 0.72)),
            MenuEntity,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(1120.0),
                        height: px(640.0),
                        padding: UiRect::all(px(22.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.08, 0.14, 0.95)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("Lightsaber Game"),
                        ui_font(40.0),
                        TextColor(Color::WHITE),
                    ));
                    card.spawn((
                        Text::new("Choose your input method and gameplay mode, then launch the same combat engine with those rules."),
                        ui_font(17.0),
                        TextColor(Color::srgb(0.78, 0.86, 0.94)),
                    ));
                    card.spawn((
                        Text::new(""),
                        ui_font(19.0),
                        TextColor(Color::srgb(1.0, 0.86, 0.52)),
                        MenuSelectionText,
                    ));

                    card.spawn((
                        Node {
                            width: percent(100.0),
                            flex_grow: 1.0,
                            column_gap: px(18.0),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ))
                    .with_children(|content| {
                        content
                            .spawn((
                                Node {
                                    width: percent(48.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ))
                            .with_children(|left| {
                                spawn_menu_section(
                                    left,
                                    "Input Source",
                                    &[
                                        ("Keyboard Only", MenuButtonAction::SelectInput(InputMode::Keyboard)),
                                        ("Python Camera", MenuButtonAction::SelectInput(InputMode::Python)),
                                        ("Keyboard + Python", MenuButtonAction::SelectInput(InputMode::Hybrid)),
                                    ],
                                );

                                spawn_menu_section(
                                    left,
                                    "Game Mode",
                                    &[
                                        ("Solo Combat", MenuButtonAction::SelectMode(GameMode::SoloCombat)),
                                        ("Fitness Mode", MenuButtonAction::SelectMode(GameMode::FitnessMode)),
                                        ("Duel Mode", MenuButtonAction::SelectMode(GameMode::MultiplayerDuel)),
                                    ],
                                );
                            });

                        spawn_how_to_play(content);
                    });

                    card.spawn(MenuButtonBundle {
                        button: Button,
                        node: Node {
                            width: px(220.0),
                            height: px(48.0),
                            margin: UiRect::top(px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgb(0.27, 0.72, 0.95)),
                        action: MenuButtonAction::StartGame,
                    })
                    .with_children(|button| {
                        button.spawn((
                            Text::new("Start Mission"),
                            ui_font(20.0),
                            TextColor(Color::srgb(0.03, 0.07, 0.12)),
                        ));
                    });
                });
        });
}

fn spawn_how_to_play(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: percent(52.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(7.0),
                padding: UiRect::all(px(14.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.05, 0.09, 0.86)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Controls / How To Play"),
                ui_font(22.0),
                TextColor(Color::srgb(0.85, 0.94, 1.0)),
            ));
            panel.spawn((
                Text::new("Keyboard: A left slash, D right slash, W force push, S guard, Space jump."),
                ui_font(15.0),
                TextColor(Color::srgb(0.8, 0.9, 1.0)),
            ));
            panel.spawn((
                Text::new("Duel Player 2: J left slash, L right slash, I force push, hold K guard."),
                ui_font(15.0),
                TextColor(Color::srgb(0.8, 0.9, 1.0)),
            ));
            panel.spawn((
                Text::new("Camera: swing left/right, thrust palm forward, raise hand quickly to jump, palm close to guard."),
                ui_font(15.0),
                TextColor(Color::srgb(0.78, 1.0, 0.82)),
            ));
            panel.spawn((
                Text::new("Jump floor obstacles, cut flying drones, keep moving for bigger fitness scores."),
                ui_font(15.0),
                TextColor(Color::srgb(0.95, 0.87, 0.68)),
            ));
            panel.spawn((
                Text::new("During gameplay: press Esc to return to this menu."),
                ui_font(15.0),
                TextColor(Color::srgb(1.0, 0.72, 0.72)),
            ));
        });
}

fn spawn_menu_section(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    buttons: &[(&str, MenuButtonAction)],
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(8.0),
                margin: UiRect::top(px(2.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|section| {
            section.spawn((
                Text::new(title),
                ui_font(22.0),
                TextColor(Color::srgb(0.85, 0.94, 1.0)),
            ));

            section
                .spawn((
                    Node {
                        column_gap: px(14.0),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|row| {
                    for (label, action) in buttons {
                        row.spawn(MenuButtonBundle {
                            button: Button,
                            node: Node {
                                min_width: px(180.0),
                                height: px(44.0),
                                padding: UiRect::axes(px(14.0), px(8.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::bottom(px(4.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgb(0.11, 0.18, 0.28)),
                            action: *action,
                        })
                        .with_children(|button| {
                            button.spawn((
                                Text::new(*label),
                                ui_font(17.0),
                                TextColor(Color::WHITE),
                            ));
                        });
                    }
                });
        });
}

fn cleanup_menu(mut commands: Commands, menu_query: Query<Entity, With<MenuEntity>>) {
    for entity in &menu_query {
        commands.entity(entity).despawn();
    }
}

fn setup_gameplay(mut commands: Commands, settings: Res<RuntimeSettings>, mut sim: ResMut<SimResource>) {
    *sim = SimResource::new(settings.mode);

    spawn_players(&mut commands, settings.mode);
    spawn_hud(&mut commands, settings.mode);
}

fn cleanup_gameplay(
    mut commands: Commands,
    mut queries: ParamSet<(
        Query<Entity, With<GameplayEntity>>,
        Query<Entity, With<MenuEntity>>,
    )>,
) {
    let gameplay_entities = queries.p0().iter().collect::<Vec<_>>();
    for entity in gameplay_entities {
        commands.entity(entity).despawn();
    }

    let menu_entities = queries.p1().iter().collect::<Vec<_>>();
    for entity in menu_entities {
        commands.entity(entity).despawn();
    }
}

fn spawn_players(commands: &mut Commands, mode: GameMode) {
    spawn_player(commands, 0, PLAYER_X, Color::srgb(0.4, 0.85, 1.0));

    if matches!(mode, GameMode::MultiplayerDuel | GameMode::MultiplayerCoop) {
        spawn_player(commands, 1, DUEL_PLAYER_TWO_X, Color::srgb(1.0, 0.55, 0.3));
    }
}

fn spawn_player(commands: &mut Commands, id: u8, x: f32, color: Color) {
    commands
        .spawn((
            Sprite::from_color(color, Vec2::new(44.0, 108.0)),
            Transform::from_xyz(x, FLOOR_Y + 78.0, 5.0),
            PlayerVisual {
                id,
                action_timer: 0.0,
                jump_timer: 0.0,
                hurt_timer: 0.0,
            },
            GameplayEntity,
        ))
        .with_children(|player| {
            player.spawn((
                Sprite::from_color(Color::srgb(0.88, 0.96, 1.0), Vec2::new(34.0, 34.0)),
                Transform::from_xyz(0.0, 70.0, 0.2),
                GameplayEntity,
            ));
            for side in [-1.0, 1.0] {
                player.spawn((
                    Sprite::from_color(Color::srgb(0.12, 0.2, 0.32), Vec2::new(12.0, 48.0)),
                    Transform::from_xyz(side * 12.0, -68.0, 0.1),
                    PlayerLeg { player_id: id, side },
                    GameplayEntity,
                ));
            }
        });
}

fn spawn_hud(commands: &mut Commands, mode: GameMode) {
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
            GameplayEntity,
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
                        ui_font(34.0),
                        TextColor(Color::WHITE),
                        HudMode,
                    ));
                    left.spawn((
                        Text::new("HP"),
                        ui_font(24.0),
                        TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        HudHealth,
                    ));
                    spawn_health_bar(left, 0, Color::srgb(0.22, 0.9, 1.0));
                    left.spawn((
                        Text::new("COMBO"),
                        ui_font(24.0),
                        TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        HudCombo,
                    ));
                    left.spawn((
                        Text::new("WAVE"),
                        ui_font(24.0),
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
                        ui_font(32.0),
                        TextColor(Color::WHITE),
                        HudScore,
                    ));
                    right.spawn((
                        Text::new("FITNESS"),
                        ui_font(22.0),
                        TextColor(Color::srgb(0.75, 1.0, 0.8)),
                        HudFitness,
                    ));
                    right.spawn((
                        Text::new("STATUS"),
                        ui_font(22.0),
                        TextColor(Color::srgb(1.0, 0.85, 0.5)),
                        HudStatus,
                    ));
                    if matches!(mode, GameMode::MultiplayerDuel | GameMode::MultiplayerCoop) {
                        right.spawn((
                            Text::new("P2 HEALTH"),
                            ui_font(22.0),
                            TextColor(Color::srgb(1.0, 0.78, 0.58)),
                        ));
                        spawn_health_bar(right, 1, Color::srgb(1.0, 0.5, 0.25));
                    }
                });
        });
}

fn spawn_health_bar(parent: &mut ChildSpawnerCommands, player_id: u8, color: Color) {
    let max_width = 260.0;
    parent
        .spawn((
            Node {
                width: px(max_width),
                height: px(18.0),
                padding: UiRect::all(px(3.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.06, 0.1, 0.92)),
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    width: px(max_width - 6.0),
                    height: percent(100.0),
                    ..default()
                },
                BackgroundColor(color),
                HealthBarFill {
                    player_id,
                    max_width: max_width - 6.0,
                },
            ));
        });
}

fn menu_button_system(
    mut interaction_query: Query<
        (&Interaction, &MenuButtonAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: ResMut<RuntimeSettings>,
    mut next_screen: ResMut<NextState<AppScreen>>,
    mut bridge: ResMut<PythonBridgeProcess>,
    mut status: ResMut<StatusMessage>,
) {
    for (interaction, action, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                match action {
                    MenuButtonAction::SelectInput(input_mode) => {
                        settings.input_mode = *input_mode;
                        if input_mode.uses_python() {
                            if bridge.ensure_running() {
                                status.label = "Camera bridge launching".into();
                            } else {
                                status.label = bridge
                                    .last_error
                                    .clone()
                                    .unwrap_or_else(|| "Camera bridge could not start".into());
                            }
                            status.ttl = 2.0;
                        }
                    }
                    MenuButtonAction::SelectMode(mode) => settings.mode = *mode,
                    MenuButtonAction::StartGame => {
                        if settings.input_mode.uses_python() {
                            let _ = bridge.ensure_running();
                        }
                        next_screen.set(AppScreen::Playing);
                    }
                }
                *color = BackgroundColor(Color::srgb(0.27, 0.72, 0.95));
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.2, 0.35, 0.5));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.11, 0.18, 0.28));
            }
        }
    }
}

fn sync_menu_labels_system(
    settings: Res<RuntimeSettings>,
    status: Res<StatusMessage>,
    mut queries: ParamSet<(
        Query<&mut Text, With<MenuSelectionText>>,
        Query<(&MenuButtonAction, &mut BackgroundColor), With<Button>>,
    )>,
) {
    if let Ok(mut text) = queries.p0().single_mut() {
        let bridge_note = if status.ttl > 0.0 && !status.label.is_empty() {
            format!(" | {}", status.label)
        } else {
            String::new()
        };
        *text = Text::new(format!(
            "Selected: {} | {}{}",
            mode_label(settings.mode),
            settings.input_mode.label(),
            bridge_note
        ));
    }

    for (action, mut color) in &mut queries.p1() {
        let selected = match action {
            MenuButtonAction::SelectInput(input_mode) => *input_mode == settings.input_mode,
            MenuButtonAction::SelectMode(mode) => *mode == settings.mode,
            MenuButtonAction::StartGame => false,
        };

        if selected {
            *color = BackgroundColor(Color::srgb(0.27, 0.72, 0.95));
        }
    }
}

fn keyboard_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    settings: Res<RuntimeSettings>,
    mut sim: ResMut<SimResource>,
    mut commands_out: Commands,
    mut player_query: Query<&mut PlayerVisual>,
) {
    if !settings.input_mode.uses_keyboard() {
        return;
    }

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
    if keys.just_pressed(KeyCode::Space) {
        commands.push(ActionCommand::new(0, PlayerAction::Jump, 1.0, now, ActionSource::Keyboard));
    }
    if keys.just_pressed(KeyCode::KeyS) {
        commands.push(ActionCommand::new(0, PlayerAction::GuardStart, 1.0, now, ActionSource::Keyboard));
    }
    if keys.just_released(KeyCode::KeyS) {
        commands.push(ActionCommand::new(0, PlayerAction::GuardEnd, 1.0, now, ActionSource::Keyboard));
    }

    if matches!(settings.mode, GameMode::MultiplayerDuel | GameMode::MultiplayerCoop) {
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
        if command.action == PlayerAction::Jump {
            trigger_jump(&mut player_query, command.player_id);
        }
        apply_command_with_fx(&mut commands_out, &mut sim, &config, command);
    }
}

fn udp_input_system(
    settings: Res<RuntimeSettings>,
    mut sim: ResMut<SimResource>,
    udp: Res<UdpResource>,
    mut commands_out: Commands,
    mut player_query: Query<&mut PlayerVisual>,
) {
    if !settings.input_mode.uses_python() {
        return;
    }

    let Some(receiver) = &udp.receiver else {
        return;
    };

    let config = sim.config.clone();
    for command in receiver.drain() {
        if command.action == PlayerAction::Jump {
            trigger_jump(&mut player_query, command.player_id);
        }
        apply_command_with_fx(&mut commands_out, &mut sim, &config, command);
    }
}

fn trigger_jump(player_query: &mut Query<&mut PlayerVisual>, player_id: u8) {
    if let Some(mut player) = player_query.iter_mut().find(|player| player.id == player_id) {
        player.jump_timer = 0.72;
    }
}

fn apply_command_with_fx(
    commands: &mut Commands,
    sim: &mut SimResource,
    config: &SimulationConfig,
    command: ActionCommand,
) {
    let drone_positions = sim
        .state
        .drones
        .iter()
        .map(|drone| {
            (
                drone.id,
                (
                    DRONE_START_X - drone.progress * 900.0,
                    lane_to_y(drone.lane),
                    drone.archetype,
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    let result = sim.state.apply_command(config, command);
    for event in result.events {
        match event {
            CombatEvent::PlayerActionAccepted { player_id, action } => {
                spawn_action_fx(commands, player_id, action);
            }
            CombatEvent::PlayerActionRejected { player_id, action } => {
                if matches!(
                    action,
                    PlayerAction::AttackLeft | PlayerAction::AttackRight | PlayerAction::ForcePush | PlayerAction::Jump
                ) {
                    spawn_action_fx(commands, player_id, action);
                }
            }
            CombatEvent::DroneDestroyed { drone_id, .. } => {
                if let Some((x, y, archetype)) = drone_positions.get(&drone_id).copied() {
                    spawn_explosion(commands, Vec2::new(x, y), archetype);
                }
            }
            CombatEvent::PlayerDamaged { player_id, .. } => {
                let x = if player_id == 0 { PLAYER_X } else { DUEL_PLAYER_TWO_X };
                spawn_explosion(commands, Vec2::new(x, FLOOR_Y + 94.0), DroneArchetype::Basic);
            }
            CombatEvent::ComboAdvanced { .. } | CombatEvent::FitnessActionCounted { .. } => {}
        }
    }
}

fn back_to_menu_system(keys: Res<ButtonInput<KeyCode>>, mut next_screen: ResMut<NextState<AppScreen>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next_screen.set(AppScreen::MainMenu);
    }
}

fn python_bridge_monitor_system(
    settings: Res<RuntimeSettings>,
    mut bridge: ResMut<PythonBridgeProcess>,
    mut status: ResMut<StatusMessage>,
) {
    if !settings.input_mode.uses_python() {
        return;
    }

    if let Some(child) = &mut bridge.child {
        match child.try_wait() {
            Ok(None) => {}
            Ok(Some(exit_status)) => {
                bridge.child = None;
                bridge.last_error = Some(format!("Camera bridge stopped: {exit_status}"));
                status.label = "Camera bridge stopped, keyboard fallback still works".into();
                status.ttl = 2.5;
            }
            Err(error) => {
                bridge.child = None;
                bridge.last_error = Some(format!("Camera bridge monitor failed: {error}"));
                status.label = "Camera bridge monitor failed".into();
                status.ttl = 2.5;
            }
        }
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
}

fn spawn_pulse(commands: &mut Commands, color: Color, ttl: f32) {
    commands.spawn((
        Sprite::from_color(color, Vec2::new(ARENA_WIDTH, ARENA_HEIGHT)),
        Transform::from_xyz(0.0, 0.0, 30.0),
        PulseFx { ttl },
        GameplayEntity,
    ));
}

fn spawn_action_fx(commands: &mut Commands, player_id: u8, action: PlayerAction) {
    let x = if player_id == 0 { PLAYER_X } else { DUEL_PLAYER_TWO_X };
    let facing = if player_id == 0 { 1.0 } else { -1.0 };
    let origin = Vec2::new(x + facing * 48.0, FLOOR_Y + 112.0);

    match action {
        PlayerAction::AttackLeft | PlayerAction::AttackRight => {
            let swing_dir = if matches!(action, PlayerAction::AttackLeft) { -1.0 } else { 1.0 };
            commands.spawn((
                Sprite::from_color(Color::srgba(0.45, 0.95, 1.0, 0.92), Vec2::new(26.0, 185.0)),
                Transform {
                    translation: Vec3::new(origin.x + facing * 36.0, origin.y, 18.0),
                    rotation: Quat::from_rotation_z((28.0 * swing_dir * facing).to_radians()),
                    ..default()
                },
                SaberFx {
                    ttl: 0.2,
                    total: 0.2,
                    action,
                },
                GameplayEntity,
            ));
            commands.spawn((
                Sprite::from_color(Color::srgba(0.08, 0.75, 1.0, 0.26), Vec2::new(92.0, 220.0)),
                Transform {
                    translation: Vec3::new(origin.x + facing * 48.0, origin.y, 17.0),
                    rotation: Quat::from_rotation_z((54.0 * swing_dir * facing).to_radians()),
                    ..default()
                },
                SaberFx {
                    ttl: 0.28,
                    total: 0.28,
                    action,
                },
                GameplayEntity,
            ));
        }
        PlayerAction::ForcePush => {
            commands.spawn((
                Sprite::from_color(Color::srgba(0.54, 0.9, 1.0, 0.34), Vec2::new(42.0, 210.0)),
                Transform::from_xyz(origin.x + facing * 38.0, origin.y, 16.0),
                ForceWaveFx { ttl: 0.45, total: 0.45 },
                GameplayEntity,
            ));
        }
        PlayerAction::Jump => {
            commands.spawn((
                Sprite::from_color(Color::srgba(0.3, 0.85, 1.0, 0.25), Vec2::new(100.0, 24.0)),
                Transform::from_xyz(x, FLOOR_Y - 4.0, 12.0),
                ForceWaveFx { ttl: 0.28, total: 0.28 },
                GameplayEntity,
            ));
        }
        PlayerAction::GuardStart => {
            commands.spawn((
                Sprite::from_color(Color::srgba(0.35, 0.9, 1.0, 0.25), Vec2::new(95.0, 165.0)),
                Transform::from_xyz(x + facing * 30.0, FLOOR_Y + 92.0, 14.0),
                ForceWaveFx { ttl: 0.32, total: 0.32 },
                GameplayEntity,
            ));
        }
        PlayerAction::GuardEnd | PlayerAction::None => {}
    }
}

fn spawn_explosion(commands: &mut Commands, position: Vec2, archetype: DroneArchetype) {
    let color = match archetype {
        DroneArchetype::Basic => Color::srgb(0.45, 0.92, 1.0),
        DroneArchetype::Shield => Color::srgb(1.0, 0.82, 0.24),
        DroneArchetype::Heavy => Color::srgb(1.0, 0.35, 0.24),
    };

    for idx in 0..18 {
        let angle = idx as f32 / 18.0 * std::f32::consts::TAU;
        let speed = 135.0 + (idx % 5) as f32 * 38.0;
        commands.spawn((
            Sprite::from_color(color, Vec2::new(10.0 + (idx % 3) as f32 * 3.0, 10.0)),
            Transform::from_xyz(position.x, position.y, 22.0),
            ExplosionFx {
                ttl: 0.55,
                total: 0.55,
                velocity: Vec2::new(angle.cos() * speed, angle.sin() * speed),
            },
            GameplayEntity,
        ));
    }

    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 0.95, 0.6, 0.32), Vec2::new(120.0, 120.0)),
        Transform::from_xyz(position.x, position.y, 19.0),
        ForceWaveFx { ttl: 0.28, total: 0.28 },
        GameplayEntity,
    ));
}

fn sync_world_system(
    mut commands: Commands,
    time: Res<Time>,
    mut player_query: Query<(&mut PlayerVisual, &mut Transform, &mut Sprite)>,
    drone_query: Query<(Entity, &DroneVisual)>,
    sim: Res<SimResource>,
) {
    for (mut player_visual, mut transform, mut sprite) in &mut player_query {
        if let Some(player) = sim.state.players.iter().find(|player| player.id == player_visual.id) {
            player_visual.action_timer = (player_visual.action_timer - time.delta_secs()).max(0.0);
            player_visual.jump_timer = (player_visual.jump_timer - time.delta_secs()).max(0.0);
            player_visual.hurt_timer = (player_visual.hurt_timer - time.delta_secs()).max(0.0);

            transform.translation.x = if player_visual.id == 0 {
                PLAYER_X
            } else {
                DUEL_PLAYER_TWO_X
            };
            let run_bob = (sim.state.elapsed_seconds * 11.0 + player_visual.id as f32).sin() * 5.0;
            let jump_arc = if player_visual.jump_timer > 0.0 {
                let t = player_visual.jump_timer / 0.72;
                (1.0 - (t * 2.0 - 1.0).abs()) * 118.0
            } else {
                0.0
            };
            transform.translation.y = FLOOR_Y + 78.0 + run_bob + jump_arc + if player.guarding { -10.0 } else { 0.0 };
            transform.rotation = Quat::from_rotation_z(if player.guarding { 0.08 } else { run_bob * 0.004 });
            sprite.color = if player_visual.hurt_timer > 0.0 {
                Color::srgb(1.0, 0.34, 0.38)
            } else if player.guarding {
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
            GameplayEntity,
        ))
        .with_children(|drone_body| {
            drone_body.spawn((
                Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.3), Vec2::new(86.0, 12.0)),
                Transform::from_xyz(0.0, 0.0, -0.1),
                GameplayEntity,
            ));
            if drone.shielded {
                drone_body.spawn((
                    Sprite::from_color(Color::srgba(1.0, 0.88, 0.2, 0.24), Vec2::new(92.0, 92.0)),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                    GameplayEntity,
                ));
            }
        });
    }
}

fn sync_hud_system(
    settings: Res<RuntimeSettings>,
    sim: Res<SimResource>,
    status: Res<StatusMessage>,
    mut health_bar_query: Query<(&HealthBarFill, &mut Node)>,
    mut hud_query: Query<(
        &mut Text,
        Option<&HudMode>,
        Option<&HudScore>,
        Option<&HudHealth>,
        Option<&HudCombo>,
        Option<&HudWave>,
        Option<&HudFitness>,
        Option<&HudStatus>,
    )>,
) {
    let player = sim.state.players.first();
    let score_text = player
        .map(|player| format!("Score {}", player.score))
        .unwrap_or_else(|| "Score 0".to_string());
    let health_text = player
        .map(|player| {
            let duel_suffix = sim
                .state
                .players
                .get(1)
                .map(|p2| format!(" | P2 {}", p2.health))
                .unwrap_or_default();
            format!("HP {}{}", player.health, duel_suffix)
        })
        .unwrap_or_else(|| "HP 0".to_string());
    let combo_text = player
        .map(|player| {
            let duel_combo = sim
                .state
                .players
                .get(1)
                .map(|p2| format!(" | P2 x{}", p2.combo))
                .unwrap_or_default();
            format!("Combo x{}{}", player.combo, duel_combo)
        })
        .unwrap_or_else(|| "Combo x0".to_string());

    let wave_text = {
        let duel_text = sim.state.match_state.map(|match_state| {
            format!(
                "Round {:.0}s | {}-{}",
                match_state.round_time_remaining,
                match_state.duel_scoreboard.player_one_rounds,
                match_state.duel_scoreboard.player_two_rounds
            )
        });
        duel_text.unwrap_or_else(|| format!("Wave {} | Drones {}", sim.state.wave_index, sim.state.drones.len()))
    };

    let fitness_text = if let Some(fitness) = &sim.state.fitness {
        format!(
            "Moves {} | Pushes {} | Effort {:.1} | {:?}",
            fitness.swings_performed,
            fitness.force_pushes_performed,
            fitness.estimated_effort_score,
            fitness.phase
        )
    } else if settings.input_mode.uses_python() {
        "Python UDP enabled on 127.0.0.1:7777".to_string()
    } else {
        "Keyboard only mode".to_string()
    };

    let status_text = if status.ttl > 0.0 && !status.label.is_empty() {
        status.label.clone()
    } else if settings.input_mode.uses_python() {
        "Press Esc for menu | Python bridge ready".to_string()
    } else {
        "Press Esc for menu".to_string()
    };

    for (
        mut text,
        hud_mode,
        hud_score,
        hud_health,
        hud_combo,
        hud_wave,
        hud_fitness,
        hud_status,
    ) in &mut hud_query
    {
        if hud_mode.is_some() {
            *text = Text::new(format!("{} | {}", mode_label(settings.mode), settings.input_mode.label()));
        } else if hud_score.is_some() {
            *text = Text::new(score_text.clone());
        } else if hud_health.is_some() {
            *text = Text::new(health_text.clone());
        } else if hud_combo.is_some() {
            *text = Text::new(combo_text.clone());
        } else if hud_wave.is_some() {
            *text = Text::new(wave_text.clone());
        } else if hud_fitness.is_some() {
            *text = Text::new(fitness_text.clone());
        } else if hud_status.is_some() {
            *text = Text::new(status_text.clone());
        }
    }

    for (bar, mut node) in &mut health_bar_query {
        let health = sim
            .state
            .players
            .iter()
            .find(|player| player.id == bar.player_id)
            .map(|player| player.health as f32)
            .unwrap_or(0.0);
        node.width = px(bar.max_width * (health / PLAYER_MAX_HEALTH).clamp(0.0, 1.0));
    }
}

fn obstacle_system(
    time: Res<Time>,
    settings: Res<RuntimeSettings>,
    mut spawner: ResMut<ObstacleSpawner>,
    mut commands: Commands,
    mut sim: ResMut<SimResource>,
    mut status: ResMut<StatusMessage>,
    mut obstacle_query: Query<(Entity, &mut Transform, &mut ObstacleVisual)>,
    mut player_query: Query<&mut PlayerVisual>,
) {
    if matches!(settings.mode, GameMode::MultiplayerDuel) {
        return;
    }

    spawner.timer -= time.delta_secs();
    if spawner.timer <= 0.0 {
        spawner.timer = 2.3 + (spawner.next_id % 3) as f32 * 0.45;
        spawner.next_id += 1;
        commands.spawn((
            Sprite::from_color(Color::srgb(1.0, 0.52, 0.22), Vec2::new(46.0, 62.0)),
            Transform::from_xyz(650.0, FLOOR_Y + 10.0, 6.0),
            ObstacleVisual { hit: false },
            GameplayEntity,
        ))
        .with_children(|obstacle| {
            obstacle.spawn((
                Sprite::from_color(Color::srgba(1.0, 0.9, 0.35, 0.38), Vec2::new(70.0, 8.0)),
                Transform::from_xyz(0.0, 36.0, 0.1),
                GameplayEntity,
            ));
        });
    }

    let player_jump = player_query
        .iter()
        .find(|player| player.id == 0)
        .map(|player| player.jump_timer)
        .unwrap_or(0.0);

    for (entity, mut transform, mut obstacle) in &mut obstacle_query {
        transform.translation.x -= 250.0 * time.delta_secs();
        transform.rotation = Quat::from_rotation_z((time.elapsed_secs() * 5.0).sin() * 0.1);

        if !obstacle.hit && (transform.translation.x - PLAYER_X).abs() < 44.0 {
            obstacle.hit = true;
            if player_jump <= 0.0 {
                if let Some(player) = sim.state.players.first_mut() {
                    player.health = (player.health - 8).max(0);
                }
                if let Some(mut visual) = player_query.iter_mut().find(|player| player.id == 0) {
                    visual.hurt_timer = 0.35;
                }
                status.label = "Jump obstacles with Space".into();
                status.ttl = 1.0;
                spawn_explosion(&mut commands, Vec2::new(PLAYER_X, FLOOR_Y + 40.0), DroneArchetype::Heavy);
            } else {
                status.label = "Clean jump".into();
                status.ttl = 0.6;
            }
        }

        if transform.translation.x < -700.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn ui_font(font_size: f32) -> TextFont {
    TextFont {
        font_size,
        ..default()
    }
}

fn animate_background_system(time: Res<Time>, mut layer_query: Query<(&mut Transform, &ParallaxLayer)>) {
    for (mut transform, layer) in &mut layer_query {
        transform.translation.x -= layer.speed * time.delta_secs();
        if transform.translation.x < -layer.wrap * 0.5 {
            transform.translation.x += layer.wrap;
        }
    }
}

fn animate_player_parts_system(
    time: Res<Time>,
    player_query: Query<&PlayerVisual>,
    mut leg_query: Query<(&PlayerLeg, &mut Transform)>,
) {
    for (leg, mut transform) in &mut leg_query {
        let Some(player) = player_query.iter().find(|player| player.id == leg.player_id) else {
            continue;
        };
        let stride = (time.elapsed_secs() * 15.0 + leg.side * std::f32::consts::FRAC_PI_2).sin();
        transform.rotation = Quat::from_rotation_z(stride * 0.32);
        transform.translation.y = -68.0 + stride.abs() * 5.0;
        if player.jump_timer > 0.0 {
            transform.rotation = Quat::from_rotation_z(leg.side * 0.5);
        }
    }
}

fn animate_effects_system(
    time: Res<Time>,
    mut queries: ParamSet<(
        Query<(&DroneVisual, &mut Transform, &mut Sprite)>,
        Query<(Entity, &mut Sprite, &mut PulseFx)>,
        Query<(Entity, &mut Transform, &mut Sprite, &mut SaberFx)>,
        Query<(Entity, &mut Transform, &mut Sprite, &mut ForceWaveFx)>,
        Query<(Entity, &mut Transform, &mut Sprite, &mut ExplosionFx)>,
    )>,
    mut commands: Commands,
    sim: Res<SimResource>,
) {
    for (visual, mut transform, mut sprite) in &mut queries.p0() {
        if let Some(drone) = sim.state.drones.iter().find(|drone| drone.id == visual.drone_id) {
            transform.translation.x = DRONE_START_X - drone.progress * 900.0;
            transform.translation.y = lane_to_y(drone.lane);
            transform.scale = Vec3::splat(1.0 + (drone.staggered_seconds * 0.2));
            sprite.color.set_alpha(1.0 - (drone.progress * 0.15).min(0.3));
        }
    }

    for (entity, mut sprite, mut pulse) in &mut queries.p1() {
        pulse.ttl -= time.delta_secs();
        let alpha = (pulse.ttl / 0.85).clamp(0.0, 1.0) * 0.25;
        sprite.color.set_alpha(alpha);
        if pulse.ttl <= 0.0 {
            commands.entity(entity).despawn();
        }
    }

    for (entity, mut transform, mut sprite, mut fx) in &mut queries.p2() {
        fx.ttl -= time.delta_secs();
        let t = (1.0 - fx.ttl / fx.total).clamp(0.0, 1.0);
        let swing = if matches!(fx.action, PlayerAction::AttackLeft) { -1.0 } else { 1.0 };
        transform.rotation *= Quat::from_rotation_z((swing * 10.0 * time.delta_secs()).to_radians());
        transform.scale = Vec3::new(1.0 + t * 0.35, 1.0 - t * 0.25, 1.0);
        sprite.color.set_alpha((1.0 - t).max(0.0) * 0.9);
        if fx.ttl <= 0.0 {
            commands.entity(entity).despawn();
        }
    }

    for (entity, mut transform, mut sprite, mut fx) in &mut queries.p3() {
        fx.ttl -= time.delta_secs();
        let t = (1.0 - fx.ttl / fx.total).clamp(0.0, 1.0);
        transform.scale = Vec3::new(1.0 + t * 6.0, 1.0 + t * 1.25, 1.0);
        sprite.color.set_alpha((1.0 - t) * 0.35);
        if fx.ttl <= 0.0 {
            commands.entity(entity).despawn();
        }
    }

    for (entity, mut transform, mut sprite, mut fx) in &mut queries.p4() {
        fx.ttl -= time.delta_secs();
        fx.velocity.y -= 520.0 * time.delta_secs();
        transform.translation.x += fx.velocity.x * time.delta_secs();
        transform.translation.y += fx.velocity.y * time.delta_secs();
        if transform.translation.y < FLOOR_Y - 20.0 {
            transform.translation.y = FLOOR_Y - 20.0;
            fx.velocity.y = fx.velocity.y.abs() * 0.42;
        }
        let t = (fx.ttl / fx.total).clamp(0.0, 1.0);
        sprite.color.set_alpha(t);
        transform.scale = Vec3::splat(0.75 + t * 0.45);
        if fx.ttl <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn mode_label(mode: GameMode) -> &'static str {
    match mode {
        GameMode::SoloCombat => "Solo Combat",
        GameMode::FitnessMode => "Fitness Mode",
        GameMode::MultiplayerDuel => "Duel Mode",
        GameMode::MultiplayerCoop => "Co-op Mode",
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

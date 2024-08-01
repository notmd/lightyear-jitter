use bevy::{
    color::palettes::tailwind,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use leafwing_input_manager::{
    prelude::{ActionState, DualAxis, InputMap},
    Actionlike,
};
use lightyear::{
    client::{config::ClientConfig, plugin::ClientPlugins},
    prelude::{
        client::{
            ClientCommands, ComponentSyncMode, LerpFn, VisualInterpolateStatus,
            VisualInterpolationPlugin,
        },
        server::{Replicate, ServerCommands, SyncTarget},
        AppComponentExt, ChannelDirection, ClientId, Deserialize, LeafwingInputPlugin, Mode,
        NetworkTarget, Serialize, SharedConfig,
    },
    server::{config::ServerConfig, plugin::ServerPlugins},
    utils::bevy::TransformLinearInterpolation,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(ServerPlugins {
            config: ServerConfig {
                shared: SharedConfig {
                    mode: Mode::HostServer,
                    ..default()
                },
                ..default()
            },
        })
        .add_plugins(ClientPlugins {
            config: ClientConfig {
                shared: SharedConfig {
                    mode: Mode::HostServer,
                    ..default()
                },
                ..default()
            },
        });

    app.add_plugins(LeafwingInputPlugin::<PlayerActions>::default());
    app.add_plugins(VisualInterpolationPlugin::<Transform>::default());

    app.register_component::<Player>(ChannelDirection::ClientToServer);
    app.register_component::<Transform>(ChannelDirection::ServerToClient)
        .add_prediction(ComponentSyncMode::Full)
        .add_interpolation_fn(TransformLinearInterpolation::lerp)
        .add_correction_fn(TransformLinearInterpolation::lerp);

    app.add_systems(Startup, set_up);
    app.add_systems(PostStartup, grab_cursor);

    app.add_systems(FixedUpdate, apply_actions);

    app.run();
}

fn set_up(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.start_server();
    commands.connect_client();

    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(50.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(2.0, 2., 2.)),
        material: materials.add(Color::Srgba(tailwind::YELLOW_400)),
        transform: Transform::from_translation(Vec3::new(0., 10., 0.)),
        ..default()
    });

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    let mut input_map = InputMap::new([
        (PlayerActions::Up, KeyCode::KeyW),
        (PlayerActions::Up, KeyCode::ArrowUp),
        (PlayerActions::Down, KeyCode::KeyS),
        (PlayerActions::Down, KeyCode::ArrowDown),
        (PlayerActions::Left, KeyCode::KeyA),
        (PlayerActions::Left, KeyCode::ArrowLeft),
        (PlayerActions::Right, KeyCode::KeyD),
        (PlayerActions::Right, KeyCode::ArrowRight),
    ]);
    input_map.insert(PlayerActions::Look, DualAxis::mouse_motion());

    commands
        .spawn((
            Player(ClientId::Local(0)),
            Replicate {
                sync: SyncTarget {
                    prediction: NetworkTarget::All,
                    ..default()
                },
                ..default()
            },
            input_map,
            TransformBundle::default(),
            VisualInterpolateStatus::<Transform>::default(),
        ))
        .with_children(|parent| {
            parent.spawn(Camera3dBundle {
                transform: Transform::from_translation(Vec3::new(0., 10., 0.)),
                ..default()
            });
        });
}

fn apply_actions(
    mut query: Query<(&ActionState<PlayerActions>, &mut Transform), With<Player>>,
    time: Res<Time>,
) {
    const MOVE_SPEED: f32 = 15.0;
    for (action, mut transform) in query.iter_mut() {
        let mut direction = Vec3::ZERO;
        if action.pressed(&PlayerActions::Up) {
            direction.z -= 1.0;
        }

        if action.pressed(&PlayerActions::Down) {
            direction.z += 1.0;
        }

        if action.pressed(&PlayerActions::Left) {
            direction.x -= 1.0;
        }

        if action.pressed(&PlayerActions::Right) {
            direction.x += 1.0;
        }

        let delta = (transform.rotation * direction.normalize_or_zero()).normalize_or_zero()
            * MOVE_SPEED
            * time.delta_seconds();

        transform.translation += delta;

        const SENSITIVE: f32 = 2.;
        let Some(event) = action.axis_pair(&PlayerActions::Look) else {
            continue;
        };

        if event.xy() != Vec2::ZERO {
            let event = event.xy();
            let yaw = (-event.x) * SENSITIVE * time.delta_seconds();
            // let pitch = event.y * SENSITIVE * time.delta_seconds();

            transform.rotation *= Quat::from_rotation_y(yaw.to_radians());
        }
    }
}

fn grab_cursor(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = windows.single_mut();
    window.cursor.grab_mode = CursorGrabMode::Locked;
    window.cursor.visible = false;
}

#[derive(Component, Serialize, Deserialize, PartialEq)]
struct Player(ClientId);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Hash, Reflect, Actionlike)]
enum PlayerActions {
    Up,
    Down,
    Left,
    Right,
    Look,
}

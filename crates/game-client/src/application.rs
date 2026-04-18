use crate::{
    camera::{self, ThirdPersonCamera},
    chat::{self, ChatInputState, ChatLog, OpenChat, SocialReceiver, SocialSender},
    configuration::Settings,
    input::{Chatting, Movement},
    movement::{self, PredictionHistory, RemoteInterpolation},
    network::{NetworkIdMapping, poll_connection, receive_server_events},
    tick_sync::{self, TickSync},
};
use avian3d::prelude::*;
use bevy::{gltf::GltfLoaderSettings, platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::*;
use bevy_renet::{
    RenetClient, RenetClientPlugin,
    netcode::{ClientAuthentication, ConnectToken, NetcodeClientPlugin, NetcodeClientTransport},
    renet::ConnectionConfig,
};
use game_core::{
    character_controller::CharacterVelocityY,
    collision::GameLayer,
    components::{LevelComponent, MovementSpeedComponent, NetworkId, Vitals},
    constants::BASE_MOVEMENT_SPEED,
};
use protocol::{models::Actor, server::EnterGameResponse};
use std::{net::UdpSocket, time::SystemTime};
use web_client::WebClient;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    CharacterSelect,
    Connecting,
    InGame,
}

#[derive(Resource)]
pub struct WebApi(pub WebClient);

#[derive(Event)]
pub struct EnterGame(pub EnterGameResponse);

#[derive(Component)]
pub struct PlayerComponent;

/// Shared capsule mesh handle used for debug placeholder rendering of all actors.
#[derive(Resource)]
pub struct DebugActorMesh(pub Handle<Mesh>);

/// Inserted once the world GLTF's `ColliderConstructorHierarchy` has finished
/// building trimesh colliders. Movement systems are gated on this resource so
/// characters don't fall through a not-yet-loaded floor.
#[derive(Resource)]
pub struct WorldReady;

#[derive(Component)]
pub struct NameComponent(pub String);

#[derive(Message)]
pub struct ActorSpawnMessage(pub Actor);

#[derive(Message)]
pub struct ActorDespawnMessage(pub NetworkId);

#[derive(Bundle)]
pub struct ActorBundle {
    name: NameComponent,
    transform: Transform,
    vitals: Vitals,
    movement_speed: MovementSpeedComponent,
    level: LevelComponent,
    body: RigidBody,
    collider: Collider,
    collision_layers: CollisionLayers,
}

impl ActorBundle {
    pub fn new(name: &str, transform: Transform, vitals: Vitals, level: i32) -> Self {
        Self {
            name: NameComponent(name.to_string()),
            transform,
            vitals: vitals.clone(),
            movement_speed: MovementSpeedComponent(BASE_MOVEMENT_SPEED),
            level: LevelComponent(level),
            body: RigidBody::Kinematic,
            collider: Collider::capsule(1., 2.),
            collision_layers: CollisionLayers::new(
                GameLayer::Player,
                [GameLayer::Default, GameLayer::Ground],
            ),
        }
    }
}

/// Function used for testing to skip the login page
pub fn create_authenticated_app(
    settings: Settings,
    api_client: WebClient,
    connect_token: ConnectToken,
) -> App {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins((RenetClientPlugin, NetcodeClientPlugin));
    app.add_plugins(
        PhysicsPlugins::new(FixedPostUpdate).set(PhysicsInterpolationPlugin::interpolate_all()),
    );
    app.add_plugins(EnhancedInputPlugin);

    app.add_input_context::<PlayerComponent>();
    app.add_input_context::<Chatting>();

    app.insert_resource(settings);
    app.insert_resource(WebApi(api_client));
    app.insert_resource(RenetClient::new(ConnectionConfig::default()));
    let transport = create_renet_transport(ClientAuthentication::Secure { connect_token });
    app.insert_resource(transport);
    app.insert_resource(ChatLog::default());
    app.insert_resource(ChatInputState::default());
    app.insert_resource(SocialSender(None));
    app.insert_resource(SocialReceiver(None));

    app.add_message::<ActorSpawnMessage>();
    app.add_message::<ActorDespawnMessage>();

    app.add_observer(on_enter_game);

    app.insert_state(AppState::Connecting);
    app.insert_resource(Time::<Fixed>::from_hz(game_core::constants::TICK_RATE_HZ));

    app.add_systems(
        Update,
        poll_connection.run_if(in_state(AppState::Connecting)),
    );

    // Input processing and tick sync run in FixedPreUpdate, before physics,
    // mirroring the server's schedule.
    // Tick sync always runs; movement prediction is gated on WorldReady so
    // characters don't fall through the floor before colliders are built.
    app.add_systems(
        FixedPreUpdate,
        (tick_sync::increment_tick, tick_sync::send_ping).run_if(in_state(AppState::InGame)),
    );
    app.add_systems(
        FixedPreUpdate,
        movement::predict_player_movement
            .after(tick_sync::increment_tick)
            .run_if(in_state(AppState::InGame)),
    );

    // After physics has stepped, send input to server.
    app.add_systems(
        FixedPostUpdate,
        movement::send_player_input
            .after(PhysicsSystems::Last)
            .run_if(in_state(AppState::InGame)),
    );

    // Reconciliation and tick rate adjustment run every render frame.
    // Remote entity interpolation also runs here for smooth visual movement.
    // Note: Local player visual interpolation is handled by avian3d's built-in
    // TransformInterpolation (enabled via interpolate_all above).
    app.add_systems(
        Update,
        (
            movement::reconcile_with_server,
            receive_server_events,
            tick_sync::adjust_tick_rate,
            movement::interpolate_remote_actors,
            handle_actor_spawn_messages,
            handle_actor_despawn_messages,
        )
            .run_if(in_state(AppState::InGame)),
    );
    app.add_systems(
        Update,
        (
            camera::camera_input,
            camera::manage_cursor_grab,
            camera::update_camera_transform,
        )
            .chain()
            .run_if(in_state(AppState::InGame)),
    );
    app.add_systems(
        Update,
        (
            chat::handle_open_chat,
            chat::handle_send_chat,
            chat::handle_cancel_chat,
            chat::handle_chat_text_input,
            chat::poll_social_events,
            chat::update_chat_ui,
        )
            .run_if(in_state(AppState::InGame)),
    );

    // app.add_systems(FixedPostUpdate, ().after(PhysicsSystems::Last));

    app.add_systems(Startup, setup_world);

    app
}

fn create_renet_transport(authentication: ClientAuthentication) -> NetcodeClientTransport {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    NetcodeClientTransport::new(current_time, authentication, socket).unwrap()
}

fn on_enter_game(
    event: On<EnterGame>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    tracing::info!("entering game");
    let response = &event.0;
    let player_actor = &response.player_actor;
    let transform = Transform::from_translation(player_actor.transform.position)
        .with_rotation(player_actor.transform.get_quat());

    commands.insert_resource(TickSync::new(response.server_tick));
    tracing::info!(
        server_tick = ?response.server_tick,
        "initial tick sync established"
    );

    // Debug placeholder mesh: capsule matching Collider::capsule(1., 2.)
    // Total height = 2 (cylinder) + 2*1 (hemispheres) = 4 units.
    // Mesh is centered at origin, so it aligns with the collider.
    let capsule_mesh = meshes.add(Capsule3d::new(1.0, 2.0));
    let player_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.7, 0.2),
        ..default()
    });

    let player_entity = commands.spawn((
        PlayerComponent,
        PredictionHistory::default(),
        CharacterVelocityY::default(),
        ActorBundle::new(
            &player_actor.name,
            transform,
            Vitals::from(player_actor.vitals.clone()),
            player_actor.level as i32,
        ),
        Mesh3d(capsule_mesh.clone()),
        MeshMaterial3d(player_material),
        actions!(PlayerComponent[
            (
                Action::<Movement>::new(),
                DeadZone::default(),
                DeltaScale::default(),
                Scale::splat(10.0),
                Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick())),
            ),
            (
                Action::<OpenChat>::new(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Enter))),
            ),
        ]),
    ));
    let player_entity_id = player_entity.id();

    // Store the capsule mesh handle as a resource so remote actors can reuse it.
    commands.insert_resource(DebugActorMesh(capsule_mesh));

    commands.spawn((
        Camera3d::default(),
        ThirdPersonCamera::default(),
        Transform::from_xyz(0.0, 10.0, 12.0).looking_at(transform.translation, Vec3::Y),
    ));

    let network_id = NetworkId(player_actor.id);
    let network_id_mapping = HashMap::from([(network_id, player_entity_id)]);
    commands.insert_resource(NetworkIdMapping(network_id_mapping));
    tracing::info!("spawned player");

    // Spawn the chat UI panel
    chat::spawn_chat_ui(&mut commands);
}

fn setup_world(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        SceneRoot(
            assets.load_with_settings("world.gltf#Scene0", |s: &mut GltfLoaderSettings| {
                s.load_cameras = false;
                s.load_lights = false;
                s.load_animations = false;
            }),
        ),
        // TODO: We are trying to match Godot here to make it work, but this is hacky
        Transform::from_xyz(0., -3., 0.),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
    ));

    // Directional sunlight.
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));
}

pub fn handle_actor_spawn_messages(
    mut reader: MessageReader<ActorSpawnMessage>,
    mut network_id_mapping: ResMut<NetworkIdMapping>,
    debug_mesh: Res<DebugActorMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for message in reader.read() {
        let actor = &message.0;
        let transform = Transform::from_translation(actor.transform.position)
            .with_rotation(actor.transform.get_quat());

        let remote_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.2, 0.2),
            ..default()
        });

        let entity = commands.spawn((
            RemoteInterpolation::default(),
            // Disable avian's built-in transform easing for remote entities.
            // Their visual position is driven by our RemoteInterpolation buffer
            // (server snapshot interpolation), not by physics simulation.
            NoTransformEasing,
            ActorBundle::new(
                &actor.name,
                transform,
                Vitals::from(actor.vitals.clone()),
                actor.level as i32,
            ),
            Mesh3d(debug_mesh.0.clone()),
            MeshMaterial3d(remote_material),
        ));
        network_id_mapping
            .0
            .insert(NetworkId(actor.id), entity.id());
    }
}

pub fn handle_actor_despawn_messages(
    mut reader: MessageReader<ActorDespawnMessage>,
    mut network_id_mapping: ResMut<NetworkIdMapping>,
    mut commands: Commands,
) {
    for message in reader.read() {
        let Some(entity) = network_id_mapping.0.get(&message.0) else {
            tracing::debug!(network_id = ?message.0, "tried to despawn actor, but it did not exist");
            continue;
        };

        commands.entity(*entity).despawn();
        network_id_mapping.0.remove(&message.0);
    }
}

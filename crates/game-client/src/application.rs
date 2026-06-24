use crate::{
    chat::{self, CancelChat, OpenChat, SendChat},
    combat,
    configuration::Settings,
    core::{ActorBundle, PlayerComponent},
    input::{Chatting, EscapePressed, Movement},
    materials,
    movement::{self, PredictionHistory},
    networking, party, theme, web,
    world::{self, DebugActorMesh, camera::ThirdPersonCamera},
};
use avian3d::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_enhanced_input::prelude::{Press, *};
use bevy_renet::{
    RenetClient, RenetClientPlugin,
    netcode::{ClientAuthentication, ConnectToken, NetcodeClientPlugin, NetcodeClientTransport},
    renet::ConnectionConfig,
};
use game_core::{
    character_controller::CharacterVelocityY,
    components::Vitals,
    networking::{NetworkId, NetworkIdMapping},
    spells::SpellLibrary,
};
use protocol::server::EnterGameResponse;
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

/// Function used for testing to skip the login page
pub fn create_authenticated_app(
    settings: Settings,
    api_client: WebClient,
    connect_token: ConnectToken,
) -> App {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins((RenetClientPlugin, NetcodeClientPlugin));
    app.add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default());
    app.add_plugins(
        PhysicsPlugins::new(FixedPostUpdate).set(PhysicsInterpolationPlugin::interpolate_all()),
    );
    app.add_plugins(EnhancedInputPlugin);
    app.add_plugins(RonAssetPlugin::<SpellLibrary>::new(&["spells.ron"]));

    app.add_plugins((
        networking::NetworkingPlugin,
        web::WebPlugin,
        theme::ThemePlugin,
        materials::MaterialsPlugin,
        movement::MovementPlugin,
        combat::CombatPlugin,
        world::WorldPlugin,
        chat::ChatPlugin,
        party::PartyPlugin,
    ));

    app.add_input_context::<PlayerComponent>();
    app.add_input_context::<Chatting>();

    app.insert_resource(settings);
    app.insert_resource(WebApi(api_client));
    app.insert_resource(RenetClient::new(ConnectionConfig::default()));
    let transport = create_renet_transport(ClientAuthentication::Secure { connect_token });
    app.insert_resource(transport);

    app.add_observer(on_enter_game);

    app.insert_state(AppState::Connecting);
    app.insert_resource(Time::<Fixed>::from_hz(game_core::constants::TICK_RATE_HZ));

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
                Press::default(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Enter))),
            ),
            (
                Action::<EscapePressed>::new(),
                Press::default(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Escape))),
            ),
        ]),
        Chatting,
        actions!(Chatting[
            (
                Action::<SendChat>::new(),
                Press::default(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Enter))),
            ),
            (
                Action::<CancelChat>::new(),
                Press::default(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Escape))),
            ),
        ]),
    ));
    let player_entity_id = player_entity.id();

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
}

use crate::{configuration::Settings, input::{Chatting, Movement}};
use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_enhanced_input::prelude::*;
use bevy_renet::{
    RenetClient, RenetClientPlugin,
    netcode::{ClientAuthentication, ConnectToken, NetcodeClientPlugin, NetcodeClientTransport},
    renet::{ConnectionConfig, DefaultChannel},
};
use game_core::{
    collision::GameLayer,
    components::{LevelComponent, MovementSpeedComponent, NetworkId, Vitals},
    constants::BASE_MOVEMENT_SPEED,
};
use protocol::{
    models::Actor,
    server::{ActorTransformUpdate, EnterGameResponse, ServerEvent},
};
use std::{collections::HashMap, net::UdpSocket, time::SystemTime};
use web_client::WebClient;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    CharacterSelect,
    Connecting,
    InGame,
}

#[derive(Resource)]
pub struct WebApi(pub WebClient);

#[derive(Resource)]
pub struct NetworkIdMapping(pub HashMap<NetworkId, Entity>);

#[derive(Event)]
pub struct EnterGame(pub EnterGameResponse);

#[derive(Component)]
pub struct PlayerComponent;

#[derive(Component)]
pub struct NameComponent(pub String);


#[derive(Message)]
pub struct ActorSpawnMessage(pub Actor);

#[derive(Message)]
pub struct ActorDespawnMessage(pub NetworkId);

#[derive(SystemParam)]
pub struct NetworkMessageWriters<'w> {
    pub spawns: MessageWriter<'w, ActorSpawnMessage>,
    pub despawns: MessageWriter<'w, ActorDespawnMessage>,
}

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
    shape_caster: ShapeCaster,
    locked_axes: LockedAxes,
}

impl ActorBundle {
    pub fn new(name: &str, transform: Transform, vitals: Vitals, level: i32) -> Self {
        Self {
            name: NameComponent(name.to_string()),
            transform,
            vitals: vitals.clone(),
            movement_speed: MovementSpeedComponent(BASE_MOVEMENT_SPEED),
            level: LevelComponent(level),
            body: RigidBody::Dynamic,
            locked_axes: LockedAxes::ROTATION_LOCKED,
            collider: Collider::capsule(1., 2.),
            collision_layers: CollisionLayers::new(
                GameLayer::Player,
                [GameLayer::Default, GameLayer::Ground],
            ),
            shape_caster: ShapeCaster::new(
                Collider::capsule(0.9, 0.1),
                Vec3::ZERO,
                Quat::IDENTITY,
                Dir3::NEG_Y,
            )
            .with_query_filter(SpatialQueryFilter::from_mask(LayerMask::ALL)),
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
    app.add_plugins(PhysicsPlugins::new(PostUpdate));
    app.add_plugins(EnhancedInputPlugin);

    app.add_input_context::<PlayerComponent>();
    app.add_input_context::<Chatting>();

    app.insert_resource(settings);
    app.insert_resource(WebApi(api_client));
    app.insert_resource(RenetClient::new(ConnectionConfig::default()));
    let transport = create_renet_transport(ClientAuthentication::Secure { connect_token });
    app.insert_resource(transport);

    app.add_message::<ActorSpawnMessage>();
    app.add_message::<ActorDespawnMessage>();

    app.add_observer(on_enter_game);

    app.insert_state(AppState::Connecting);

    app.add_systems(
        Update,
        poll_connection.run_if(in_state(AppState::Connecting)),
    );
    // app.add_systems(FixedPostUpdate, ().after(PhysicsSystems::Last));
    app.add_systems(OnEnter(AppState::InGame), setup_world);

    app
}

fn poll_connection(
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut client: ResMut<RenetClient>,
) {
    if let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        match bitcode::decode::<EnterGameResponse>(&message) {
            Ok(response) => {
                commands.trigger(EnterGame(response));
                next_state.set(AppState::InGame);
            }
            Err(e) => {
                tracing::error!("received invalid EnterGameResponse {}", e);
                next_state.set(AppState::CharacterSelect);
            }
        }
    }
}

fn create_renet_transport(authentication: ClientAuthentication) -> NetcodeClientTransport {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    NetcodeClientTransport::new(current_time, authentication, socket).unwrap()
}

fn on_enter_game(event: On<EnterGame>, mut commands: Commands) {
    let player_actor = &event.0.player_actor;
    let transform = Transform::from_translation(player_actor.transform.position)
        .with_rotation(player_actor.transform.get_quat());

    let network_id = NetworkId(player_actor.id);
    let player_entity = commands.spawn((
        PlayerComponent,
        ActorBundle::new(
            &player_actor.name,
            transform,
            Vitals::from(player_actor.vitals.clone()),
            player_actor.level as i32,
        ),
        actions!(Player[
            (
                Action::<Movement>::new(),
                DeadZone::default(),
                DeltaScale::default(),
                Scale::splat(10.0),
                Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick())),
            ),
        ]),
    ));

    let network_id_mapping = HashMap::from([(network_id, player_entity.id())]);
    commands.insert_resource(NetworkIdMapping(network_id_mapping));
}

fn setup_world() {}

fn receive_transform_updates(
    mut client: ResMut<RenetClient>,
    network_id_mapping: Res<NetworkIdMapping>,
) {
    while let Some(message) = client.receive_message(DefaultChannel::Unreliable) {
        match bitcode::decode::<ActorTransformUpdate>(&message) {
            Ok(update) => {}
            Err(e) => {
                tracing::error!("received invalid ActorTransformUpdate {}", e);
            }
        }
    }
}

fn receive_server_events(mut writers: NetworkMessageWriters, mut client: ResMut<RenetClient>) {
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        match bitcode::decode::<ServerEvent>(&message) {
            Ok(event) => match event {
                ServerEvent::ActorSpawn(actor) => {
                    writers.spawns.write(ActorSpawnMessage(*actor));
                }
                ServerEvent::ActorDespawn(id) => {
                    writers.despawns.write(ActorDespawnMessage(NetworkId(id)));
                }
                _ => todo!("Handle server event"),
            },
            Err(e) => {
                tracing::error!("received invalid ServerEvent {}", e);
            }
        }
    }
}

pub fn handle_actor_spawn_messages(
    mut reader: MessageReader<ActorSpawnMessage>,
    mut network_id_mapping: ResMut<NetworkIdMapping>,
    mut commands: Commands,
) {
    for message in reader.read() {
        let actor = &message.0;
        let transform = Transform::from_translation(actor.transform.position)
            .with_rotation(actor.transform.get_quat());
        let entity = commands.spawn(ActorBundle::new(
            &actor.name,
            transform,
            Vitals::from(actor.vitals.clone()),
            actor.level as i32,
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

pub fn send_player_movement(
    mut client: ResMut<RenetClient>,
)

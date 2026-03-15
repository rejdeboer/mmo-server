use crate::configuration::Settings;
use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_renet::{
    RenetClient, RenetClientPlugin,
    netcode::{ClientAuthentication, ConnectToken, NetcodeClientPlugin, NetcodeClientTransport},
    renet::{ConnectionConfig, DefaultChannel},
};
use game_core::components::{LevelComponent, MovementSpeedComponent, NetworkId, Vitals};
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

#[derive(Message)]
pub struct ActorSpawnMessage(pub Actor);

#[derive(Message)]
pub struct ActorDespawnMessage(pub NetworkId);

#[derive(SystemParam)]
pub struct NetworkMessageWriters<'w> {
    pub spawns: MessageWriter<'w, ActorSpawnMessage>,
    pub despawns: MessageWriter<'w, ActorDespawnMessage>,
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
        network_id,
        transform,
        Vitals::from(player_actor.vitals.clone()),
        LevelComponent(player_actor.level as i32),
        MovementSpeedComponent(player_actor.movement_speed.into()),
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
) {
    for message in reader.read() {}
}

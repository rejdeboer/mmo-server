use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};

use flatbuffers::{FlatBufferBuilder, InvalidFlatbuffer, WIPOffset, root};
use renet::{Bytes, ConnectionConfig, DefaultChannel, RenetClient};
use renet_netcode::{ClientAuthentication, ConnectToken, NetcodeClientTransport};

use crate::types::Character;
use crate::{PlayerAction, Transform, Vec3};

#[derive(Debug, Clone, PartialEq)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
    InGame,
}

#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    Connected,
    Disconnected,
    EnterGameSuccess { character: Character },
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    MoveEntity {
        entity_id: u64,
        transform: Transform,
    },
    SpawnEntity {
        entity_id: u64,
    },
    DespawnEntity {
        entity_id: u64,
    },
}

pub struct GameClient {
    client: RenetClient,
    transport: Option<NetcodeClientTransport>,
    state: ClientState,
}

impl Default for GameClient {
    fn default() -> Self {
        Self {
            client: RenetClient::new(ConnectionConfig::default()),
            transport: None,
            state: ClientState::Disconnected,
        }
    }
}

impl GameClient {
    pub fn connect(&mut self, connect_token: ConnectToken) {
        let authentication = ClientAuthentication::Secure { connect_token };
        self.setup_transport(authentication);
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn get_state(&self) -> &ClientState {
        &self.state
    }

    /// Poll events on a pending connection request
    pub fn poll_connection(&mut self, dt: Duration) -> Option<ConnectionEvent> {
        self.client.update(dt);

        if let Some(transport) = self.transport.as_mut() {
            transport.update(dt, &mut self.client).unwrap();
        }

        match self.state {
            ClientState::Connecting => {
                if self.client.is_connected() {
                    self.state = ClientState::Connected;
                    return Some(ConnectionEvent::Connected);
                } else if self.client.is_disconnected() {
                    // TODO: Handle reason
                    self.state = ClientState::Disconnected;
                    return Some(ConnectionEvent::Disconnected);
                }
            }
            ClientState::Connected => {
                if let Some(message) = self.client.receive_message(DefaultChannel::ReliableOrdered)
                {
                    match root::<schemas::mmo::EnterGameResponse>(&message) {
                        Ok(response) => {
                            self.state = ClientState::InGame;
                            return Some(ConnectionEvent::EnterGameSuccess {
                                character: response.into(),
                            });
                        }
                        Err(e) => {
                            tracing::error!("received invalid EnterGameResponse {}", e);
                            self.state = ClientState::Disconnected;
                            return Some(ConnectionEvent::Disconnected);
                        }
                    }
                }
            }
            ClientState::InGame => {
                if !self.is_connected() {
                    return Some(ConnectionEvent::Disconnected);
                }
            }
            _ => (),
        };
        None
    }

    pub fn update_game(&mut self, dt: Duration) -> Vec<GameEvent> {
        debug_assert!(matches!(self.state, ClientState::InGame));
        self.client.update(dt);
        self.transport
            .as_mut()
            .expect("this is only called when in game")
            .update(dt, &mut self.client)
            .unwrap();

        let mut events: Vec<GameEvent> = vec![];
        while let Some(message) = self.client.receive_message(DefaultChannel::ReliableOrdered) {
            if let Err(error) = read_event_batch(&mut events, message) {
                tracing::error!(?error, "unexpected reliable message received");
            };
        }
        while let Some(message) = self.client.receive_message(DefaultChannel::Unreliable) {
            if let Err(error) = read_event_batch(&mut events, message) {
                tracing::error!(?error, "unexpected unreliable message received");
            };
        }
        events
    }

    pub fn send_actions(&mut self, actions: Vec<PlayerAction>) {
        let mut builder = FlatBufferBuilder::new();
        let mut fb_actions = Vec::<WIPOffset<schemas::mmo::Action>>::with_capacity(actions.len());

        for action in actions {
            fb_actions.push(action.encode(&mut builder));
        }

        let actions_vec = builder.create_vector(fb_actions.as_slice());
        let fb_batch = schemas::mmo::BatchedActions::create(
            &mut builder,
            &schemas::mmo::BatchedActionsArgs {
                actions: Some(actions_vec),
            },
        );
        builder.finish_minimal(fb_batch);
        let data = builder.finished_data().to_vec();

        self.client.send_message(DefaultChannel::Unreliable, data);
        self.transport
            .as_mut()
            .unwrap()
            .send_packets(&mut self.client)
            .unwrap();
    }

    fn setup_transport(&mut self, authentication: ClientAuthentication) {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        self.transport = Some(transport);
        self.state = ClientState::Connecting;
    }

    /// Can be used for testing
    pub fn connect_unsecure(&mut self, host: String, port: u16, character_id: i32) {
        let ip_addr = IpAddr::V4(host.parse().expect("host should be IPV4 addr"));
        let server_addr: SocketAddr = SocketAddr::new(ip_addr, port);

        let mut builder = FlatBufferBuilder::new();
        let response_offset = schemas::mmo::NetcodeTokenUserData::create(
            &mut builder,
            &schemas::mmo::NetcodeTokenUserDataArgs { character_id },
        );
        builder.finish_minimal(response_offset);

        let mut user_data: [u8; 256] = [0; 256];
        let copy_data = builder.finished_data();
        user_data[0..copy_data.len()].copy_from_slice(copy_data);

        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id: 0,
            user_data: Some(user_data),
            protocol_id: 0,
        };

        self.setup_transport(authentication);
    }
}

fn read_event_batch(events: &mut Vec<GameEvent>, bytes: Bytes) -> Result<(), InvalidFlatbuffer> {
    let batch = root::<schemas::mmo::BatchedEvents>(&bytes)?;
    if let Some(fb_events) = batch.events() {
        for event in fb_events {
            match event.data_type() {
                schemas::mmo::EventData::EntityMoveEvent => {
                    let fb_event = event.data_as_entity_move_event().unwrap();
                    let transform = fb_event.transform().unwrap();
                    let pos = transform.position();
                    events.push(GameEvent::MoveEntity {
                        entity_id: fb_event.entity_id(),
                        transform: Transform {
                            position: Vec3::new(pos.x(), pos.y(), pos.z()),
                            yaw: transform.yaw(),
                        },
                    })
                }
                schemas::mmo::EventData::EntitySpawnEvent => events.push(GameEvent::SpawnEntity {
                    entity_id: event.data_as_entity_spawn_event().unwrap().entity_id(),
                }),
                schemas::mmo::EventData::EntityDespawnEvent => {
                    events.push(GameEvent::DespawnEntity {
                        entity_id: event.data_as_entity_despawn_event().unwrap().entity_id(),
                    })
                }
                event_type => {
                    tracing::warn!(?event_type, "unhandled event type");
                }
            }
        }
    }

    Ok(())
}

// Test that can be used to check if connection is successful with local server
// #[test]
// fn test_connection_manual() {
//     let mut client = GameClient::default();
//     client.connect_unsecure("127.0.0.1".to_string(), 8000);
//     let mut last_time = SystemTime::now();
//     loop {
//         let new_time = SystemTime::now();
//         let dt = new_time.duration_since(last_time).unwrap();
//         last_time = new_time;
//         let events = client.update(dt);
//         if !events.is_empty() {
//             break;
//         }
//     }
//     println!("WE PRINTIN");
// }

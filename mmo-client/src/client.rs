use protocol::client::{MoveAction, PlayerAction};
use protocol::models::Actor;
use protocol::server::{ActorTransformUpdate, EnterGameResponse, ServerEvent, TokenUserData};
use renet::{ConnectionConfig, DefaultChannel, RenetClient};
use renet_netcode::{ClientAuthentication, ConnectToken, NetcodeClientTransport};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};

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
    EnterGameSuccess { player_actor: Actor },
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
            transport
                .update(dt, &mut self.client)
                .expect("failed to update transport");
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
                    match bitcode::decode::<EnterGameResponse>(&message) {
                        Ok(response) => {
                            self.state = ClientState::InGame;
                            return Some(ConnectionEvent::EnterGameSuccess {
                                player_actor: response.player_actor,
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

    pub fn update_game(&mut self, dt: Duration) -> (Vec<ServerEvent>, Vec<ActorTransformUpdate>) {
        debug_assert!(matches!(self.state, ClientState::InGame));
        self.client.update(dt);
        self.transport
            .as_mut()
            .expect("this is only called when in game")
            .update(dt, &mut self.client)
            .expect("transport updated");

        let mut world_events: Vec<ServerEvent> = vec![];
        while let Some(message) = self.client.receive_message(DefaultChannel::ReliableOrdered) {
            match bitcode::decode::<ServerEvent>(&message) {
                Ok(event) => world_events.push(event),
                Err(err) => tracing::error!(?err, "unexpected reliable message received"),
            };
        }

        let mut movement_updates: Vec<ActorTransformUpdate> = vec![];
        while let Some(message) = self.client.receive_message(DefaultChannel::Unreliable) {
            match bitcode::decode::<ActorTransformUpdate>(&message) {
                Ok(update) => movement_updates.push(update),
                Err(err) => tracing::error!(?err, "unexpected unreliable message received"),
            };
        }

        (world_events, movement_updates)
    }

    pub fn send_actions(&mut self, movement: Option<MoveAction>, actions: Vec<PlayerAction>) {
        if movement.is_none() && actions.is_empty() {
            return;
        }

        if let Some(move_action) = movement {
            self.client
                .send_message(DefaultChannel::Unreliable, bitcode::encode(&move_action));
        }

        for action in actions.into_iter() {
            self.client
                .send_message(DefaultChannel::ReliableOrdered, bitcode::encode(&action));
        }

        self.transport
            .as_mut()
            .expect("actions are only sent while in game")
            .send_packets(&mut self.client)
            .expect("packet sent to server");
    }

    fn setup_transport(&mut self, authentication: ClientAuthentication) {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        let transport = NetcodeClientTransport::new(current_time, authentication, socket)
            .expect("failed to create transport");

        self.transport = Some(transport);
        self.state = ClientState::Connecting;
    }

    /// Can be used for testing
    pub fn connect_unsecure(&mut self, host: String, port: u16, character_id: i32) {
        let ip_addr = IpAddr::V4(host.parse().expect("host should be IPV4 addr"));
        let server_addr: SocketAddr = SocketAddr::new(ip_addr, port);

        let copy_data = bitcode::encode(&TokenUserData {
            character_id,
            traceparent: None,
        });

        let mut user_data: [u8; 256] = [0; 256];
        user_data[0..copy_data.len()].copy_from_slice(copy_data.as_slice());

        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id: character_id as u64,
            user_data: Some(user_data),
            protocol_id: 0,
        };

        self.setup_transport(authentication);
    }
}

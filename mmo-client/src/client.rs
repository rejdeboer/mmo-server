use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};

use flatbuffers::root;
use renet::{ConnectionConfig, DefaultChannel, RenetClient};
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

use crate::types::Character;

#[derive(Debug, Clone, PartialEq)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
    InGame,
}

#[derive(Debug, Clone)]
pub enum ClientEvent {
    Connected,
    Disconnected,
    EnterGameSuccess { character: Character },
}

pub struct GameClient {
    client: RenetClient,
    transport: NetcodeClientTransport,
    state: ClientState,
}

impl GameClient {
    pub fn connect(host: String, port: u16) -> Self {
        let client = RenetClient::new(ConnectionConfig::default());

        let ip_addr = IpAddr::V4(host.parse().expect("host should be IPV4 addr"));
        let server_addr: SocketAddr = SocketAddr::new(ip_addr, port);

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id: 0,
            user_data: None,
            protocol_id: 0,
        };

        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        Self {
            client,
            transport,
            state: ClientState::Connecting,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn update(&mut self, dt: Duration) -> Vec<ClientEvent> {
        self.client.update(dt);
        self.transport.update(dt, &mut self.client).unwrap();

        let mut events = Vec::new();
        match self.state {
            ClientState::Connecting => {
                if self.client.is_connected() {
                    self.state = ClientState::Connected;
                    events.push(ClientEvent::Connected);
                } else if self.client.is_disconnected() {
                    // TODO: Handle reason
                    self.state = ClientState::Disconnected;
                    events.push(ClientEvent::Disconnected);
                }
            }
            ClientState::Connected => {
                if let Some(message) = self.client.receive_message(DefaultChannel::ReliableOrdered)
                {
                    match root::<schemas::mmo::EnterGameResponse>(&message) {
                        Ok(response) => {
                            events.push(ClientEvent::EnterGameSuccess {
                                character: response.into(),
                            });
                            self.state = ClientState::InGame;
                        }
                        Err(e) => {
                            tracing::error!("received invalid EnterGameResponse {}", e);
                            events.push(ClientEvent::Disconnected);
                            self.state = ClientState::Disconnected;
                        }
                    }
                }
            }
            _ => (),
        }

        events
    }
}

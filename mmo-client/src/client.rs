use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};

use flatbuffers::{FlatBufferBuilder, root};
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
    pub fn connect(&mut self, host: String, port: u16) {
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

        self.transport = Some(transport);
        self.state = ClientState::Connecting;
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn get_state(&self) -> &ClientState {
        &self.state
    }

    pub fn update(&mut self, dt: Duration) -> Vec<ClientEvent> {
        self.client.update(dt);

        if let Some(transport) = self.transport.as_mut() {
            transport.update(dt, &mut self.client).unwrap();
        }

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

    pub fn send_enter_game_request(&mut self, character_id: i32, token: String) {
        assert!(matches!(self.state, ClientState::Connected));

        let mut builder = FlatBufferBuilder::new();
        let fb_token = builder.create_string(&token);
        // let request = schemas::mmo::EnterGameRequest::create(
        //     &mut builder,
        //     &schemas::mmo::EnterGameRequestArgs {
        //         token: Some(fb_token),
        //         character_id,
        //     },
        // );
        //
        // builder.finish_minimal(request);
        // let data = builder.finished_data();
        // self.client
        //     .send_message(DefaultChannel::ReliableOrdered, data.to_vec());
    }
}

// Test that can be used to check if connection is successful with local server
#[test]
fn test_connection_manual() {
    let mut client = GameClient::default();
    client.connect("127.0.0.1".to_string(), 8000);
    let mut last_time = SystemTime::now();
    loop {
        let new_time = SystemTime::now();
        let dt = new_time.duration_since(last_time).unwrap();
        last_time = new_time;
        let events = client.update(dt);
        if !events.is_empty() {
            break;
        }
    }
    println!("WE PRINTIN");
}

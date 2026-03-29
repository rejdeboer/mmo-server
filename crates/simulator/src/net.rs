use protocol::{client::MoveAction, server::EnterGameResponse};
use renet::{ConnectionConfig, DefaultChannel, RenetClient};
use renet_netcode::NetcodeClientTransport;
use renetcode::ClientAuthentication;
use std::{
    net::UdpSocket,
    time::{Duration, SystemTime},
};

pub use renetcode::ConnectToken;

/// Events returned by [`GameClient::poll_connection`] during the connecting phase.
pub enum ConnectionEvent {
    /// The server accepted us and sent the initial game state.
    EnterGameSuccess { player_name: String },
    /// The transport disconnected before we entered the game.
    Disconnected,
}

/// A minimal game-server client that handles the renet networking protocol.
///
/// This wraps `RenetClient` + `NetcodeClientTransport` and exposes a
/// poll-based API suitable for driving from a simple loop. No Bevy, no ECS,
/// no physics — just networking.
pub struct GameClient {
    client: RenetClient,
    transport: Option<NetcodeClientTransport>,
}

impl Default for GameClient {
    fn default() -> Self {
        Self {
            client: RenetClient::new(ConnectionConfig::default()),
            transport: None,
        }
    }
}

impl GameClient {
    /// Begin connecting to the game server using the given connect token.
    pub fn connect(&mut self, connect_token: ConnectToken) {
        let socket = UdpSocket::bind("0.0.0.0:0").expect("failed to bind UDP socket");
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time before unix epoch");

        let transport = NetcodeClientTransport::new(
            current_time,
            ClientAuthentication::Secure { connect_token },
            socket,
        )
        .expect("failed to create netcode transport");

        self.transport = Some(transport);
    }

    /// Drive the transport and poll for the initial `EnterGameResponse`.
    ///
    /// Call this repeatedly while in the `Connecting` state. Returns
    /// `Some(ConnectionEvent)` once the server responds or the connection drops.
    pub fn poll_connection(&mut self, dt: Duration) -> Option<ConnectionEvent> {
        self.update_transport(dt);

        if self.client.is_disconnected() {
            return Some(ConnectionEvent::Disconnected);
        }

        if let Some(message) = self.client.receive_message(DefaultChannel::ReliableOrdered) {
            match bitcode::decode::<EnterGameResponse>(&message) {
                Ok(response) => {
                    return Some(ConnectionEvent::EnterGameSuccess {
                        player_name: response.player_actor.name,
                    });
                }
                Err(e) => {
                    tracing::error!("invalid EnterGameResponse: {e}");
                }
            }
        }

        None
    }

    /// Drive the transport and drain all incoming messages.
    ///
    /// Call this every tick once connected. Server messages are consumed
    /// and discarded — the simulator doesn't need to act on them, it just
    /// needs to keep the receive buffer drained so renet doesn't back up.
    pub fn drain_messages(&mut self, dt: Duration) {
        self.update_transport(dt);

        while self
            .client
            .receive_message(DefaultChannel::ReliableOrdered)
            .is_some()
        {}
        while self
            .client
            .receive_message(DefaultChannel::Unreliable)
            .is_some()
        {}
    }

    /// Send a movement action to the server.
    pub fn send_movement(&mut self, action: MoveAction) {
        let encoded = bitcode::encode(&action);
        self.client
            .send_message(DefaultChannel::Unreliable, encoded);
        self.send_packets();
    }

    /// Returns true if the underlying renet client is disconnected.
    pub fn is_disconnected(&self) -> bool {
        self.client.is_disconnected()
    }

    fn update_transport(&mut self, dt: Duration) {
        let Some(transport) = self.transport.as_mut() else {
            return;
        };

        if let Err(e) = transport.update(dt, &mut self.client) {
            tracing::error!("transport error: {e}");
        }
    }

    fn send_packets(&mut self) {
        let Some(transport) = self.transport.as_mut() else {
            return;
        };

        if let Err(e) = transport.send_packets(&mut self.client) {
            tracing::error!("failed to send packets: {e}");
        }
    }
}

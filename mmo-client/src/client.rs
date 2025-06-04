use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};

use renet::{ConnectionConfig, RenetClient};
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

pub struct GameClient {
    client: RenetClient,
    transport: NetcodeClientTransport,
}

impl GameClient {
    pub fn connect(host: String, port: u16) -> Self {
        let client = RenetClient::new(ConnectionConfig::default());
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();

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

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        Self { client, transport }
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn update(&mut self, dt: Duration) {
        self.client.update(dt);
        self.transport.update(dt, &mut self.client).unwrap()
    }
}

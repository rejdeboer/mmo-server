use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime};

use bevy::prelude::*;
use bevy_renet::netcode::{ClientAuthentication, NetcodeClientTransport};
use bevy_renet::renet::{ConnectionConfig, DefaultChannel, RenetClient};
use bevy_tokio_tasks::TokioTasksRuntime;
use fake::Fake;
use fake::faker::internet::en::{Password, SafeEmail, Username};
use mmo_server::application::{self};
use mmo_server::configuration::{Settings, get_configuration};
use mmo_server::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber();
    init_subscriber(subscriber);
});

pub struct TestScenario {
    pub clients: Vec<TestClient>,
    app: App,
    tick_interval: Duration,
    timeout: Duration,
}

pub struct TestClient {
    pub character_id: i32,
    pub client: RenetClient,
    pub transport: NetcodeClientTransport,
}

#[derive(Resource)]
struct TestCharacterCount(pub usize);

#[derive(Resource)]
pub struct TestCharacterIds(pub Vec<i32>);

impl TestScenario {
    pub fn tick_interval(mut self, tick_interval: Duration) {
        self.tick_interval = tick_interval;
    }

    pub fn timeout(mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn world(&self) -> &World {
        self.app.world()
    }

    pub fn client_send_message<T: serde::Serialize>(
        &mut self,
        client_index: usize,
        channel: DefaultChannel,
        message: T,
    ) {
        let client = &mut self.clients[client_index];
        let encoded = bincode::serde::encode_to_vec(message, bincode::config::standard()).unwrap();
        client.client.send_message(channel, encoded);
        client.transport.send_packets(&mut client.client).unwrap();
    }

    pub fn client_receive_message<T: serde::de::DeserializeOwned>(
        &mut self,
        client_index: usize,
        channel: DefaultChannel,
    ) -> T {
        let start_time = Instant::now();
        let mut last_tick_time = Instant::now();
        let channel_id: u8 = channel.into();

        while start_time.elapsed() < self.timeout {
            if last_tick_time.elapsed() >= self.tick_interval {
                self.update(last_tick_time.elapsed());
                last_tick_time = Instant::now();
                if let Some(message) = self.clients[client_index]
                    .client
                    .receive_message(channel_id)
                {
                    return bincode::serde::decode_from_slice::<T, _>(
                        &message,
                        bincode::config::standard(),
                    )
                    .unwrap()
                    .0;
                }
            }
            std::thread::sleep(Duration::from_millis(1)); // Prevent test busy loop
        }
        panic!("Test timed out after {:?}", self.timeout);
    }

    pub fn run_until_condition_or_timeout(
        &mut self,
        mut condition_check: impl FnMut(&mut World) -> bool,
    ) -> Result<(), String> {
        let start_time = Instant::now();
        let mut last_tick_time = Instant::now();
        let mut condition_met = false;

        while start_time.elapsed() < self.timeout {
            if last_tick_time.elapsed() >= self.tick_interval {
                self.update(last_tick_time.elapsed());
                last_tick_time = Instant::now();
                if condition_check(self.app.world_mut()) {
                    condition_met = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(1)); // Prevent test busy loop
        }
        if condition_met {
            Ok(())
        } else {
            Err(format!("Test timed out after {:?}", self.timeout))
        }
    }

    fn update(&mut self, dt: Duration) {
        self.app.update();
        for client in self.clients.iter_mut() {
            client.update(dt);
        }
    }
}

impl TestClient {
    pub fn update(&mut self, dt: Duration) {
        self.client.update(dt);
        self.transport.update(dt, &mut self.client).unwrap();
    }
}

pub fn spawn_app(character_count: usize) -> TestScenario {
    // Only initialize tracer once instead of every test
    Lazy::force(&TRACING);

    let settings = {
        let mut c = get_configuration().expect("configuration fetched");
        c.database.db_name = Username().fake();
        c.server.port = 0;
        c
    };

    let (mut application, port) = application::build(settings.clone()).expect("application built");
    application.insert_resource(TestCharacterCount(character_count));
    application.add_systems(Startup, init_db);

    // NOTE: Run startup systems
    application.update();

    let test_character_ids = application
        .world()
        .get_resource::<TestCharacterIds>()
        .unwrap()
        .0
        .clone();

    let ip_addr = IpAddr::V4(
        settings
            .server
            .host
            .parse()
            .expect("host should be IPV4 addr"),
    );
    let server_addr: SocketAddr = SocketAddr::new(ip_addr, port);

    let mut clients: Vec<TestClient> = Vec::with_capacity(character_count);
    for id in test_character_ids {
        clients.push(create_client(id, server_addr));
    }

    let mut test_app = TestScenario {
        app: application,
        clients,
        tick_interval: Duration::from_millis(10),
        timeout: Duration::from_secs(1),
    };

    test_app
}

pub fn create_client(character_id: i32, server_addr: SocketAddr) -> TestClient {
    let client = RenetClient::new(ConnectionConfig::default());
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id: character_id as u64,
        user_data: None,
        protocol_id: 0,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

    let test_client = TestClient {
        character_id,
        client,
        transport,
    };
    test_client
}

fn init_db(
    mut commands: Commands,
    runtime: Res<TokioTasksRuntime>,
    settings: Res<Settings>,
    character_count: Res<TestCharacterCount>,
) {
    let character_ids = runtime.runtime().block_on(async move {
        let mut connection = PgConnection::connect_with(&settings.database.without_db())
            .await
            .expect("connected to postgres");
        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, settings.database.db_name).as_str())
            .await
            .expect("db created");

        let connection_pool = PgPool::connect_with(settings.database.with_db())
            .await
            .expect("failed to connect to postgres");
        sqlx::migrate!("../migrations")
            .run(&connection_pool)
            .await
            .expect("migration successful");

        let mut character_ids = Vec::with_capacity(character_count.0);
        for _ in 0..character_count.0 {
            let account_id = add_test_account(&connection_pool).await;
            let character_id = add_test_character(&connection_pool, account_id).await;
            character_ids.push(character_id);
        }
        character_ids
    });

    commands.insert_resource(TestCharacterIds(character_ids));
}

async fn add_test_account(pool: &PgPool) -> i32 {
    let username: String = Username().fake();
    let email: String = SafeEmail().fake();
    let password: String = Password(0..10).fake();

    let row = sqlx::query!(
        "INSERT INTO accounts (username, email, passhash)
        VALUES ($1, $2, $3)
        RETURNING id",
        &username,
        &email,
        &password,
    )
    .fetch_one(pool)
    .await
    .expect("test account created");

    row.id
}

async fn add_test_character(pool: &PgPool, account_id: i32) -> i32 {
    let username: String = Username().fake();

    let row = sqlx::query!(
        "INSERT INTO characters (name, account_id)
        VALUES ($1, $2)
        RETURNING id",
        &username,
        account_id,
    )
    .fetch_one(pool)
    .await
    .expect("test character created");

    row.id
}

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime};

use bevy::prelude::*;
use bevy_renet::netcode::{ClientAuthentication, NetcodeClientTransport};
use bevy_renet::renet::{ConnectionConfig, RenetClient};
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

pub struct TestApp {
    pub app: App,
    // TODO: This will probably become a vector
    pub test_character_id: i32,
    pub host: String,
    pub port: u16,
    tick_interval: Duration,
    timeout: Duration,
}

#[derive(Resource)]
pub struct TestCharacterId(pub i32);

impl TestApp {
    pub fn tick_interval(mut self, tick_interval: Duration) {
        self.tick_interval = tick_interval;
    }

    pub fn timeout(mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn create_client(&self) -> (RenetClient, NetcodeClientTransport) {
        let client = RenetClient::new(ConnectionConfig::default());
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let ip_addr = IpAddr::V4(self.host.parse().expect("host should be IPV4 addr"));
        let server_addr: SocketAddr = SocketAddr::new(ip_addr, self.port);
        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id: 0,
            user_data: None,
            protocol_id: 0,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        (client, transport)
    }

    pub fn run_until_condition_or_timeout(
        mut self,
        mut condition_check: impl FnMut(&mut World, Duration) -> bool,
    ) -> Result<(), String> {
        let start_time = Instant::now();
        let mut last_tick_time = Instant::now();
        let mut condition_met = false;

        while start_time.elapsed() < self.timeout {
            if last_tick_time.elapsed() >= self.tick_interval {
                self.app.update();
                last_tick_time = Instant::now();
                if condition_check(self.app.world_mut(), last_tick_time.elapsed()) {
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
}

pub fn spawn_app() -> TestApp {
    // Only initialize tracer once instead of every test
    Lazy::force(&TRACING);

    let settings = {
        let mut c = get_configuration().expect("configuration fetched");
        c.database.db_name = Username().fake();
        c.server.port = 0;
        c
    };

    let (mut application, port) = application::build(settings.clone()).expect("application built");
    application.add_systems(Startup, configure_database);

    // NOTE: Run startup systems
    application.update();

    let test_character_id = application
        .world()
        .get_resource::<TestCharacterId>()
        .unwrap()
        .0;

    let test_app = TestApp {
        app: application,
        test_character_id,
        host: settings.server.host,
        port,
        tick_interval: Duration::from_millis(10),
        timeout: Duration::from_secs(1),
    };

    test_app
}

fn configure_database(
    mut commands: Commands,
    runtime: Res<TokioTasksRuntime>,
    settings: Res<Settings>,
) {
    let character_id = runtime.runtime().block_on(async move {
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

        let account_id = add_test_account(&connection_pool).await;
        let character_id = add_test_character(&connection_pool, account_id).await;
        character_id
    });

    commands.insert_resource(TestCharacterId(character_id));
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

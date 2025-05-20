use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::prelude::*;
use bevy_renet::netcode::{ClientAuthentication, NetcodeClientTransport};
use bevy_renet::renet::{ConnectionConfig, RenetClient};
use fake::Fake;
use fake::faker::internet::en::{SafeEmail, Username};
use mmo_server::application::{self, get_connection_pool};
use mmo_server::configuration::{DatabaseSettings, get_configuration};
use mmo_server::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber();
    init_subscriber(subscriber);
});

pub struct TestApp {
    pub app: App,
    pub host: String,
    pub port: u16,
    pub db_pool: PgPool,
}

impl TestApp {
    pub async fn create_client(&self) -> (RenetClient, NetcodeClientTransport) {
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
}

pub async fn spawn_app() -> TestApp {
    // Only initialize tracer once instead of every test
    Lazy::force(&TRACING);

    let settings = {
        let mut c = get_configuration().expect("configuration fetched");
        c.database.db_name = Username().fake();
        c.server.port = 0;
        c
    };

    configure_database(&settings.database).await;
    let (application, port) = application::build(settings.clone()).expect("application built");

    let test_app = TestApp {
        app: application,
        db_pool: get_connection_pool(&settings),
        host: settings.server.host,
        port,
    };

    let test_account_id = add_test_account(&test_app.db_pool).await;
    add_test_character(&test_app.db_pool, test_account_id).await;
    test_app
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("connected to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.db_name).as_str())
        .await
        .expect("db created");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("failed to connect to postgres");
    sqlx::migrate!("../migrations")
        .run(&connection_pool)
        .await
        .expect("migration successful");

    connection_pool
}

async fn add_test_account(pool: &PgPool) -> i32 {
    let username: String = Username().fake();
    let email: String = SafeEmail().fake();

    let row = sqlx::query!(
        "INSERT INTO accounts (username, email)
        VALUES ($1, $2)
        RETURNING id",
        &username,
        &email,
    )
    .fetch_one(pool)
    .await
    .expect("test character created");

    row.id
}

async fn add_test_character(pool: &PgPool, account_id: i32) {
    let username: String = Username().fake();
    sqlx::query!(
        "INSERT INTO characters (name, account_id)
        VALUES ($1, $2)",
        &username,
        account_id,
    )
    .execute(pool)
    .await
    .expect("test character created");
}

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use rand::Rng;
use secrecy::{ExposeSecret, Secret};
use sqlx::types::Uuid;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use websocket::auth::Claims;
use websocket::configuration::{get_configuration, DatabaseSettings};
use websocket::server::{get_connection_pool, Application};
use websocket::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber();
    init_subscriber(subscriber);
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub signing_key: Secret<String>,
}

impl TestApp {
    pub async fn create_owner_client(
        &self,
    ) -> WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>> {
        let test_document = self.test_document().await;
        let owner_token = self.signed_jwt(test_document.1);
        let request = self.create_connection_request(owner_token, test_document.0);

        let (socket, _response) = tokio_tungstenite::connect_async(request)
            .await
            .expect("websocket connected");

        socket
    }

    pub async fn test_document(&self) -> (Uuid, Uuid) {
        let row = sqlx::query!("SELECT id, owner_id FROM documents LIMIT 1")
            .fetch_one(&self.db_pool)
            .await
            .expect("fetched document");
        (row.id, row.owner_id)
    }

    pub fn signed_jwt(&self, user_id: Uuid) -> String {
        let claims = Claims {
            user_id: user_id.to_string(),
            username: Uuid::new_v4().to_string(),
            exp: SystemTime::now()
                .duration_since(UNIX_EPOCH - Duration::from_secs(3600))
                .unwrap()
                .as_secs(),
        };

        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(self.signing_key.expose_secret().as_ref()),
        )
        .expect("token encoded")
        .to_string()
    }

    pub fn create_connection_request(&self, token: String, document_id: Uuid) -> Request {
        let url_str = &*format!(
            "{}/{}?token={}",
            self.address,
            document_id.to_string(),
            token
        );
        let url = url::Url::parse(url_str).unwrap();
        let host = url.host_str().expect("Host should be found in URL");

        Request::builder()
            .method("GET")
            .uri(url_str)
            .header("Host", host)
            .header("Upgrade", "websocket")
            .header("Connection", "upgrade")
            .header("Sec-Websocket-Key", generate_websocket_key())
            .header("Sec-Websocket-Version", "13")
            .body(())
            .unwrap()
    }
}

pub async fn spawn_app() -> TestApp {
    // Only initialize tracer once instead of every test
    Lazy::force(&TRACING);

    let settings = {
        let mut c = get_configuration().expect("configuration fetched");
        c.database.db_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c
    };

    configure_database(&settings.database).await;
    let application = Application::build(settings.clone())
        .await
        .expect("application built");
    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    let test_app = TestApp {
        address: format!("ws://localhost:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&settings.database),
        signing_key: settings.application.signing_key,
    };

    let test_owner_id = add_test_user(&test_app.db_pool).await;
    add_test_document(&test_app.db_pool, test_owner_id).await;
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
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("../db/migrations")
        .run(&connection_pool)
        .await
        .expect("migration successful");

    connection_pool
}

async fn add_test_document(pool: &PgPool, owner_id: Uuid) {
    sqlx::query!(
        "INSERT INTO documents (id, name, owner_id)
        VALUES ($1, $2, $3)",
        Uuid::new_v4(),
        Uuid::new_v4().to_string(),
        owner_id,
    )
    .execute(pool)
    .await
    .expect("test document created");
}

async fn add_test_user(pool: &PgPool) -> Uuid {
    let row = sqlx::query!(
        "INSERT INTO users (id, username, email, passhash)
        VALUES ($1, $2, $3, $4)
        RETURNING id",
        Uuid::new_v4(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
    )
    .fetch_one(pool)
    .await
    .expect("test user created");

    row.id
}

fn generate_websocket_key() -> String {
    let mut rng = rand::thread_rng();
    let mut random_bytes = [0u8; 16];
    rng.fill(&mut random_bytes);
    base64::encode(random_bytes)
}

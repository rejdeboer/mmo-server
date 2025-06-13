use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fake::faker::internet::en::{Password, SafeEmail, Username};
use fake::Fake;
use once_cell::sync::Lazy;
use secrecy::{ExposeSecret, Secret};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use web_server::auth::Claims;
use web_server::configuration::{get_configuration, DatabaseSettings};
use web_server::routes::AccountCreate;
use web_server::server::{get_connection_pool, Application};
use web_server::telemetry::{get_subscriber, init_subscriber};

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
    pub async fn create_account(&self, body: AccountCreate) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/account", &self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn signed_jwt(&self, account_id: i32) -> String {
        let claims = Claims {
            account_id,
            username: Username().fake(),
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
}

pub async fn spawn_app() -> TestApp {
    // Only initialize tracer once instead of every test
    Lazy::force(&TRACING);

    let settings = {
        let mut c = get_configuration().expect("configuration fetched");
        c.database.db_name = Username().fake();
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
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&settings.database),
        signing_key: settings.application.signing_key,
    };

    add_test_account(&test_app.db_pool).await;
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
    sqlx::migrate!("../migrations")
        .run(&connection_pool)
        .await
        .expect("migration successful");

    connection_pool
}

async fn add_test_account(pool: &PgPool) -> i32 {
    let username: String = Username().fake();
    let email: String = SafeEmail().fake();
    let password: String = Password(9..10).fake();

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

use fake::faker::internet::en::{Password, SafeEmail, Username};
use fake::Fake;
use once_cell::sync::Lazy;
use secrecy::{ExposeSecret, Secret};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use web_server::auth::encode_jwt;
use web_server::configuration::{get_configuration, DatabaseSettings};
use web_server::domain::SafePassword;
use web_server::server::{get_connection_pool, Application};
use web_server::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber();
    init_subscriber(subscriber);
});

#[derive(sqlx::FromRow)]
pub struct TestAccount {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl TestAccount {
    pub fn generate() -> Self {
        Self {
            username: Username().fake(),
            email: SafeEmail().fake(),
            password: Password(9..10).fake(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let password = SafePassword::parse(self.password.clone()).expect("password parsed");
        let passhash = password.hash().expect("password hashed");
        sqlx::query!(
            "INSERT INTO accounts (username, email, passhash)
            VALUES ($1, $2, $3)",
            &self.username,
            &self.email,
            passhash.as_str(),
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub jwt_signing_key: Secret<String>,
    pub account: TestAccount,
}

impl TestApp {
    pub fn signed_jwt(&self, account_id: i32) -> String {
        encode_jwt(
            account_id,
            Username().fake(),
            self.jwt_signing_key.expose_secret().as_ref(),
        )
        .expect("JWT encoded")
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
        jwt_signing_key: settings.application.signing_key,
        account: TestAccount::generate(),
    };

    test_app.account.store(&test_app.db_pool).await;
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

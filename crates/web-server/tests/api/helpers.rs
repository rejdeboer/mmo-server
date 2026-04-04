use fake::Fake;
use fake::faker::internet::en::{Password, SafeEmail, Username};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use web_client::{WebClient, WebClientError};
use web_server::configuration::{
    DatabaseSettings, TelemetrySettings, TracingFormat, get_configuration,
};
use web_server::domain::SafePassword;
use web_server::server::{Application, get_connection_pool};
use web_server::telemetry::init_subscriber;
use web_types::LoginBody;

static TRACING: Lazy<()> = Lazy::new(|| {
    init_subscriber(&TelemetrySettings {
        tracing_format: TracingFormat::Pretty,
        otel_exporter_endpoint: None,
    });
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

    async fn store(&self, pool: &PgPool) -> i32 {
        let password = SafePassword::parse(self.password.clone()).expect("password parsed");
        let passhash = password.hash().expect("password hashed");
        sqlx::query!(
            "INSERT INTO accounts (username, email, passhash)
            VALUES ($1, $2, $3) RETURNING id",
            &self.username,
            &self.email,
            passhash.as_str(),
        )
        .fetch_one(pool)
        .await
        .expect("Failed to store test user.")
        .id
    }
}

#[derive(sqlx::FromRow)]
pub struct TestCharacter {
    pub id: i32,
}

impl TestCharacter {
    async fn create(pool: &PgPool, account_id: i32) -> Self {
        let name: String = Username().fake();
        sqlx::query_as!(
            TestCharacter,
            "INSERT INTO characters (name, account_id)
            VALUES ($1, $2) RETURNING id",
            &name,
            account_id,
        )
        .fetch_one(pool)
        .await
        .expect("Failed to store test character.")
    }
}

pub struct TestApp {
    // pub db_pool: PgPool,
    pub client: WebClient,
    pub account: TestAccount,
    pub character: TestCharacter,
}

impl TestApp {
    pub async fn login_account(&mut self) -> Result<(), WebClientError> {
        self.client
            .login(&LoginBody {
                email: self.account.email.clone(),
                password: self.account.password.clone(),
            })
            .await
    }

    pub async fn login_character(&mut self) -> Result<(), WebClientError> {
        self.login_account().await?;
        self.client.select_character(self.character.id).await?;
        Ok(())
    }
}

pub async fn spawn_app() -> TestApp {
    // Only initialize tracer once instead of every test
    Lazy::force(&TRACING);

    let settings = {
        let mut c = get_configuration().expect("configuration fetched");
        c.database.name = Username().fake();
        c.application.port = 0;
        c
    };

    configure_database(&settings.database).await;
    let application = Application::build(settings.clone())
        .await
        .expect("application built");
    let application_port = application.port();

    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(application.run_until_stopped());

    let pool = get_connection_pool(&settings.database);
    let account = TestAccount::generate();
    let account_id = account.store(&pool).await;
    let character = TestCharacter::create(&pool, account_id).await;

    TestApp {
        // db_pool: pool,
        // jwt_signing_key: settings.application.jwt_signing_key,
        client: WebClient::new(format!("http://localhost:{application_port}")),
        account,
        character,
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("connected to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.name).as_str())
        .await
        .expect("db created");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("../../db/migrations")
        .run(&connection_pool)
        .await
        .expect("migration successful");

    connection_pool
}

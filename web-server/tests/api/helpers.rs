use axum::http::HeaderMap;
use fake::Fake;
use fake::faker::internet::en::{Password, SafeEmail, Username};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tracing_subscriber::EnvFilter;
use web_server::configuration::{DatabaseSettings, get_configuration};
use web_server::domain::SafePassword;
use web_server::routes::{
    CharacterCreate, GameEntryRequest, GameEntryResponse, LoginBody, TokenResponse,
};
use web_server::server::{Application, get_connection_pool};
use web_server::telemetry::{get_local_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    init_subscriber(get_local_subscriber(env_filter));
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
    pub address: String,
    // pub db_pool: PgPool,
    pub api_client: reqwest::Client,
    pub account: TestAccount,
    pub character: TestCharacter,
}

impl TestApp {
    // pub fn signed_jwt(&self, account_id: i32) -> String {
    //     encode_jwt(
    //         account_id,
    //         Username().fake(),
    //         self.jwt_signing_key.expose_secret().as_ref(),
    //     )
    //     .expect("JWT encoded")
    // }

    pub async fn login_account(&mut self) {
        let login_response = self
            .api_client
            .post(format!("{}/token", &self.address))
            .json(&LoginBody {
                email: self.account.email.clone(),
                password: self.account.password.clone(),
            })
            .send()
            .await
            .expect("Failed to execute request.")
            .json::<TokenResponse>()
            .await
            .unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", login_response.jwt).parse().unwrap(),
        );

        self.api_client = create_api_client(Some(headers));
    }

    pub async fn login_character(&mut self) {
        self.login_account().await;

        let login_response = self
            .api_client
            .post(format!("{}/game/request-entry", &self.address))
            .json(&GameEntryRequest {
                character_id: self.character.id,
            })
            .send()
            .await
            .expect("Failed to execute request.")
            .json::<GameEntryResponse>()
            .await
            .unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", login_response.jwt).parse().unwrap(),
        );

        self.api_client = create_api_client(Some(headers));
    }

    pub async fn create_character(&self, body: CharacterCreate) -> reqwest::Response {
        self.api_client
            .post(format!("{}/character", self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
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
        address: format!("http://localhost:{application_port}"),
        // db_pool: pool,
        // jwt_signing_key: settings.application.jwt_signing_key,
        api_client: create_api_client(None),
        account,
        character,
    }
}

fn create_api_client(default_headers: Option<HeaderMap>) -> reqwest::Client {
    let mut client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());

    if let Some(headers) = default_headers {
        client = client.default_headers(headers);
    }

    client.build().unwrap()
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
    sqlx::migrate!("../db/migrations")
        .run(&connection_pool)
        .await
        .expect("migration successful");

    connection_pool
}

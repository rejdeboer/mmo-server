use crate::{
    auth::auth_middleware,
    configuration::{DatabaseSettings, Settings},
    routes::{account_create, character_create, character_list, login},
};
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

pub struct Application {
    listener: TcpListener,
    router: Router,
    port: u16,
}

#[derive(Clone)]
pub struct NetcodePrivateKey([u8; 32]);

impl AsRef<[u8; 32]> for NetcodePrivateKey {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone)]
pub struct ApplicationState {
    pub pool: PgPool,
    pub jwt_signing_key: SecretString,
    pub netcode_private_key: NetcodePrivateKey,
}

#[derive(Deserialize)]
pub struct QueryParams {
    pub token: String,
}

impl Application {
    pub async fn build(settings: Settings) -> Result<Self, std::io::Error> {
        let address = format!(
            "{}:{}",
            settings.application.host, settings.application.port
        );

        let listener = TcpListener::bind(address).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let connection_pool = get_connection_pool(&settings.database);

        let mut netcode_private_key: [u8; 32] = [0; 32];
        base64::decode_config_slice(
            settings.application.netcode_private_key.expose_secret(),
            base64::STANDARD,
            &mut netcode_private_key,
        );

        let application_state = ApplicationState {
            pool: connection_pool,
            jwt_signing_key: settings.application.jwt_signing_key,
            netcode_private_key: NetcodePrivateKey(netcode_private_key),
        };

        let protected_routes = Router::new()
            .route("/character", get(character_list).post(character_create))
            .route_layer(middleware::from_fn_with_state(
                settings.application.jwt_signing_key,
                auth_middleware,
            ));

        let router = Router::new()
            .merge(protected_routes)
            .route("/account", post(account_create))
            .route("/token", post(login))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().include_headers(true)),
            )
            .with_state(application_state);

        Ok(Self {
            listener,
            router,
            port,
        })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        tracing::info!("listening on {}", self.listener.local_addr().unwrap());
        axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(settings.with_db())
}

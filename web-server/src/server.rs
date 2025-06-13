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
use secrecy::ExposeSecret;
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
pub struct ApplicationState {
    pub pool: PgPool,
    pub signing_key: String,
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

        let application_state = ApplicationState {
            pool: connection_pool,
            signing_key: settings.application.signing_key.expose_secret().to_string(),
        };

        let protected_routes = Router::new()
            .route("/character", get(character_list).post(character_create))
            .route_layer(middleware::from_fn_with_state(
                settings.application.signing_key,
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

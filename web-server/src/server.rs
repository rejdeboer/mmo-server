use crate::{
    auth::{account_auth_middleware, character_auth_middleware},
    configuration::{DatabaseSettings, NetcodePrivateKey, Settings},
    realm_resolution::{RealmResolver, create_realm_resolver},
    routes::{account_create, character_create, character_list, chat, game_entry, login},
};
use axum::{
    Router, middleware,
    routing::{get, post},
};
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{net::SocketAddr, sync::Arc};
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
    pub jwt_signing_key: SecretString,
    pub netcode_private_key: NetcodePrivateKey,
    pub realm_resolver: Arc<dyn RealmResolver>,
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
            jwt_signing_key: settings.application.jwt_signing_key.clone(),
            netcode_private_key: settings.application.netcode_private_key,
            realm_resolver: Arc::from(create_realm_resolver(&settings.realm_resolver).await),
        };

        let account_routes = Router::new()
            .route("/character", get(character_list).post(character_create))
            .route("/game/request-entry", post(game_entry))
            .route_layer(middleware::from_fn_with_state(
                settings.application.jwt_signing_key.clone(),
                account_auth_middleware,
            ));

        let character_routes =
            Router::new()
                .route("/chat", get(chat))
                .route_layer(middleware::from_fn_with_state(
                    settings.application.jwt_signing_key,
                    character_auth_middleware,
                ));

        let router = Router::new()
            .merge(character_routes)
            .merge(account_routes)
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

use crate::{
    auth::{account_auth_middleware, character_auth_middleware},
    configuration::{DatabaseSettings, NetcodePrivateKey, Settings},
    realm_resolution::{RealmResolver, create_realm_resolver},
    routes::{
        account_create, character_create, character_list, game_entry, login, metrics_get, social,
    },
    social::{Hub, HubMessage},
};
use axum::{
    Router, middleware,
    routing::{get, post},
};
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::mpsc::{Receiver, Sender, channel},
};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

pub struct Application {
    listener: TcpListener,
    router: Router,
    port: u16,
    hub_rx: Receiver<HubMessage>,
    pool: PgPool,
}

#[derive(Clone)]
pub struct ApplicationState {
    pub pool: PgPool,
    pub jwt_signing_key: SecretString,
    pub netcode_private_key: NetcodePrivateKey,
    pub realm_resolver: Arc<dyn RealmResolver>,
    pub hub_tx: Sender<HubMessage>,
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
        let pool = get_connection_pool(&settings.database);

        let (hub_tx, hub_rx) = channel::<HubMessage>(128);

        let application_state = ApplicationState {
            pool: pool.clone(),
            jwt_signing_key: settings.application.jwt_signing_key.clone(),
            netcode_private_key: settings.application.netcode_private_key,
            realm_resolver: Arc::from(create_realm_resolver(&settings.realm_resolver).await),
            hub_tx,
        };

        let account_routes = Router::new()
            .route("/character", get(character_list).post(character_create))
            .route("/game/request-entry", post(game_entry))
            .route_layer(middleware::from_fn_with_state(
                settings.application.jwt_signing_key.clone(),
                account_auth_middleware,
            ));

        let character_routes = Router::new().route("/social", get(social)).route_layer(
            middleware::from_fn_with_state(
                settings.application.jwt_signing_key,
                character_auth_middleware,
            ),
        );

        let router = Router::new()
            .merge(character_routes)
            .merge(account_routes)
            .route("/account", post(account_create))
            .route("/token", post(login))
            // TODO: Should probably not be a public route...
            .route("/metrics", get(metrics_get))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().include_headers(true)),
            )
            .with_state(application_state);

        Ok(Self {
            listener,
            router,
            port,
            hub_rx,
            pool,
        })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        tracing::info!("listening on {}", self.listener.local_addr().unwrap());

        start_social_hub(self.pool, self.hub_rx);

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

fn start_social_hub(db_pool: PgPool, receiver: Receiver<HubMessage>) {
    tokio::spawn(async move {
        tracing::info!("starting hub");
        let hub = Hub::new(db_pool, receiver);
        hub.run().await;
    });
}

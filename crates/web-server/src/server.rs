use crate::{
    auth::{account_auth_middleware, character_auth_middleware},
    configuration::{DatabaseSettings, NetcodePrivateKey, Settings},
    realm_resolution::{RealmResolver, create_realm_resolver},
    routes::{
        account_create, character_create, character_list, game_entry, health, login,
        social,
    },
    social::{Hub, HubMessage, NatsBridge},
    telemetry::init_metrics,
};
use axum::{
    Router, middleware,
    extract::{MatchedPath, Request},
    response::Response,
    routing::{get, post},
};
use metrics::{counter, histogram};
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tokio::{
    net::TcpListener,
    sync::mpsc::{Receiver, Sender, channel},
};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

pub struct Server {
    pub listener: TcpListener,
    pub router: Router,
}

pub struct Application {
    app_server: Server,
    port: u16,
    hub_rx: Receiver<HubMessage>,
    pool: PgPool,
    nats: Option<NatsBridge>,
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
        if let Some(metrics_settings) = &settings.metrics {
            init_metrics(metrics_settings);
        }

        let app_adderess = format!(
            "{}:{}",
            settings.application.host, settings.application.port
        );

        let app_listener = TcpListener::bind(app_adderess).await.unwrap();
        let port = app_listener.local_addr().unwrap().port();
        let pool = get_connection_pool(&settings.database);

        let nats = if let Some(nats_settings) = &settings.nats {
            Some(
                NatsBridge::connect(&nats_settings.url)
                    .await
                    .expect("failed to connect to NATS"),
            )
        } else {
            tracing::warn!("NATS not configured, cross-instance messaging disabled");
            None
        };

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

        let app_router = Router::new()
            .merge(character_routes)
            .merge(account_routes)
            .route("/account", post(account_create))
            .route("/token", post(login))
            .route("/health", get(health))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(
                        DefaultMakeSpan::default()
                            .level(Level::INFO)
                            .include_headers(true),
                    )
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(tower_http::LatencyUnit::Millis),
                    ),
            )
            .layer(middleware::from_fn(http_metrics_middleware))
            .with_state(application_state);
        let app_server = Server {
            listener: app_listener,
            router: app_router,
        };

        Ok(Self {
            app_server,
            port,
            hub_rx,
            pool,
            nats,
        })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        tracing::info!(
            "listening on {}",
            self.app_server.listener.local_addr().unwrap()
        );

        start_social_hub(self.pool, self.hub_rx, self.nats);

        let listener = self.app_server.listener;
        let app = self.app_server.router;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(settings.with_db())
}

fn start_social_hub(db_pool: PgPool, receiver: Receiver<HubMessage>, nats: Option<NatsBridge>) {
    tokio::spawn(async move {
        tracing::info!("starting hub");
        let hub = Hub::new(db_pool, receiver, nats);
        hub.run().await;
    });
}

async fn http_metrics_middleware(
    request: Request,
    next: middleware::Next,
) -> Response {
    let method = request.method().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());

    let start = Instant::now();
    let response = next.run(request).await;
    let elapsed = start.elapsed().as_secs_f64();

    let status = response.status().as_u16().to_string();

    counter!("http_requests_total", "method" => method.clone(), "route" => route.clone(), "status" => status).increment(1);
    histogram!("http_request_duration_seconds", "method" => method, "route" => route).record(elapsed);

    response
}

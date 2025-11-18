use crate::{ServerSettings, routes::provision_route};
use axum::{Router, routing::post};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use web_server::configuration::NetcodePrivateKey;

pub struct Application {
    listener: TcpListener,
    router: Router,
}

#[derive(Clone)]
pub struct ApplicationState {
    pub pool: PgPool,
    pub netcode_private_key: NetcodePrivateKey,
}

impl Application {
    pub async fn build(settings: ServerSettings, pool: PgPool) -> anyhow::Result<Self> {
        let address = format!("{}:{}", settings.host, settings.port);
        let listener = TcpListener::bind(address).await.unwrap();

        let application_state = ApplicationState {
            pool,
            netcode_private_key: settings.netcode_private_key,
        };

        let router = Router::new()
            .route("/provision", post(provision_route))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().include_headers(true)),
            )
            .with_state(application_state);

        Ok(Self { listener, router })
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
}

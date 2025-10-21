use axum::{Router, routing::post};
use sqlx::{
    PgConnection, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

pub struct Application {
    listener: TcpListener,
    router: Router,
    port: u16,
    pool: PgPool,
}

#[derive(Clone)]
pub struct ApplicationState {
    pub pool: PgPool,
}

impl Application {
    pub async fn build(host: &str, port: i32, db_url: &str) -> anyhow::Result<()> {
        let address = format!("{host}:{port}");

        let listener = TcpListener::bind(address).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let pool = get_connection_pool(db_url);

        let application_state = ApplicationState { pool: pool.clone() };

        let router = Router::new()
            .route("/seed", post(seed_route))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().include_headers(true)),
            )
            .with_state(application_state);

        Ok(Self {
            listener,
            router,
            port,
            pool,
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

pub fn get_connection_pool(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().connect_with(url.parse()?)
}

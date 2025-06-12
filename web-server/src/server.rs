use axum::{
    extract::{ConnectInfo, Path, State },
    middleware,
    response::Response,
    routing::get,
    Extension, Router,
};
use axum_extra::TypedHeader;
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    sync::{Arc, Mutex},
};
use tokio::{
    net::TcpListener,
    sync::mpsc::{channel, Sender},
};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use uuid::Uuid;

use crate::{
    auth::{auth_middleware, User},
    configuration::{DatabaseSettings, Settings},
    document::Document,
    error::ApiError,
    websocket::{handle_socket, Message, Syncer},
};

pub struct Application {
    listener: TcpListener,
    router: Router,
    port: u16,
}

pub struct ApplicationState {
    pub pool: PgPool,
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
        });

        let router = Router::new()
            .route("/character", post(character_post))
            .route_layer(middleware::from_fn_with_state(
                settings.application.signing_key,
                auth_middleware,
            ))
            .route("/", get(|| async { "Hello from web server" }))
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

// TODO: Add connection context for tracing
async fn character_post(
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    State(state): State<ApplicationState>,
    Extension(user): Extension<User>,
) -> Result<Response, ApiError> {
    let _user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown client")
    };

}


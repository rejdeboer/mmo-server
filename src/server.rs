use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::SocketAddr;
use std::{error::Error, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::instrument;

use crate::configuration::{DatabaseSettings, Settings};

pub struct Application {
    listener: TcpListener,
    port: u16,
    state: Arc<ApplicationState>,
}

pub struct ApplicationState {
    pub pool: PgPool,
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

        let state = Arc::new(ApplicationState {
            pool: connection_pool,
        });

        Ok(Self {
            listener,
            port,
            state,
        })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        tracing::info!("listening on {}", self.listener.local_addr().unwrap());
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::info!("accepted new connection from: {}", addr);

                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, addr).await {
                            tracing::error!("error handling client {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("failed to accept connection: {}", e);
                }
            }
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(settings.with_db())
}

#[instrument(name="client", skip_all, fields(addr = ?addr))]
async fn handle_client(mut stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    tracing::info!("connected");

    let mut buffer = [0u8; 1024];

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                tracing::info!("connection closed by client");
                return Ok(());
            }
            Ok(n) => {
                let received_data = &buffer[..n];
                let received_string = String::from_utf8_lossy(received_data);

                tracing::info!("received: {}", received_string.trim_end());

                if let Err(e) = stream.write_all(received_data).await {
                    tracing::error!("failed to write to stream: {}", e);
                    return Err(e.into());
                }
                tracing::info!("echoed back: {}", received_string.trim_end());
            }
            Err(e) => {
                tracing::error!("failed to read from stream: {}", e);
                return Err(e.into());
            }
        }
    }
}

use bevy::prelude::*;
use prometheus::{Encoder, IntGauge, Registry, TextEncoder};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, interval};

#[derive(Resource, Clone)]
pub struct Metrics {
    pub registry: Arc<Mutex<Registry>>,
    pub connected_players: Arc<IntGauge>,
}

impl Default for Metrics {
    fn default() -> Self {
        let registry = Registry::new();

        let connected_players = IntGauge::new(
            "connected_players_count",
            "Current number of connected players.",
        )
        .unwrap();
        registry
            .register(Box::new(connected_players.clone()))
            .unwrap();

        Self {
            registry: Arc::new(Mutex::new(registry)),
            connected_players: Arc::new(connected_players),
        }
    }
}

pub async fn run_metrics_exporter(metrics: Metrics, path: String) {
    let mut interval = interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let registry = metrics.registry.lock().await;
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();

        if let Err(err) = encoder.encode(&registry.gather(), &mut buffer) {
            error!(?err, "failed to encode metrics");
            continue;
        }

        match File::create(&path) {
            Ok(mut file) => {
                if let Err(err) = file.write_all(&buffer) {
                    error!(?err, "failed to write metrics to file");
                }
            }
            Err(err) => error!(?err, "failed to create metrics file"),
        }
    }
}

use bevy::prelude::*;
use bevy_tokio_tasks::TaskContext;
use prometheus::{Encoder, Gauge, IntGauge, Registry, TextEncoder};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, interval};

const EXPORT_INTERVAL_SECS: f32 = 5.;

#[derive(Resource, Clone)]
pub struct Metrics {
    pub registry: Arc<Mutex<Registry>>,
    pub connected_players: Arc<IntGauge>,
    pub tick_rate: Arc<Gauge>,
}

impl Default for Metrics {
    fn default() -> Self {
        let registry = Registry::new();

        let connected_players = IntGauge::new(
            "connected_players_count",
            "Current number of connected players",
        )
        .unwrap();
        registry
            .register(Box::new(connected_players.clone()))
            .unwrap();

        let tick_rate = Gauge::new(
            "server_tick_rate_hz",
            "The server's current tick rate in Hz",
        )
        .unwrap();
        registry.register(Box::new(tick_rate.clone())).unwrap();

        Self {
            registry: Arc::new(Mutex::new(registry)),
            connected_players: Arc::new(connected_players),
            tick_rate: Arc::new(tick_rate),
        }
    }
}

pub async fn run_metrics_exporter(ctx: TaskContext, metrics: Metrics, path: String) {
    let mut interval = interval(Duration::from_secs_f32(EXPORT_INTERVAL_SECS));
    let mut last_tick = ctx.current_tick();
    let mut last_instant = Instant::now();

    loop {
        let now = interval.tick().await;
        let dt = now.duration_since(last_instant).as_secs_f64();
        last_instant = now;

        let current_tick = ctx.current_tick();
        let elapsed_ticks = (current_tick - last_tick) as f64;
        last_tick = current_tick;

        metrics.tick_rate.set(elapsed_ticks / dt);

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

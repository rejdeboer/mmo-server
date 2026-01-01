use crate::configuration::{TracingFormat, TracingSettings};
use bevy::prelude::*;
use bevy_tokio_tasks::TaskContext;
use opentelemetry::global;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use prometheus::{
    Encoder, Gauge, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
    register_histogram_vec,
};
use prometheus::{Histogram, HistogramOpts, IntGauge};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, interval};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt};

const EXPORT_INTERVAL_SECS: f32 = 5.;

#[derive(Resource, Clone)]
pub struct Metrics {
    pub registry: Arc<Mutex<Registry>>,
    pub connected_players: IntGauge,
    pub tick_rate: Gauge,
    pub network_packets_total: IntCounterVec,
    pub network_bytes_total: IntCounterVec,
    pub network_packet_size_bytes: HistogramVec,
    pub server_rtt: Histogram,
}

impl Default for Metrics {
    fn default() -> Self {
        let registry = Registry::new_custom(Some("game".to_string()), None).unwrap();

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

        let network_packets_total = IntCounterVec::new(
            Opts::new(
                "network_packets_total",
                "The total number of packets sent / received",
            ),
            &["direction", "channel"],
        )
        .unwrap();
        registry
            .register(Box::new(network_packets_total.clone()))
            .unwrap();

        let network_bytes_total = IntCounterVec::new(
            Opts::new(
                "network_bytes_total",
                "The total number of bytes sent / received",
            ),
            &["direction", "channel"],
        )
        .unwrap();
        registry
            .register(Box::new(network_bytes_total.clone()))
            .unwrap();

        // TODO: Maybe create separate histograms for incoming / outgoing?
        let network_packet_size_bytes = register_histogram_vec!(
            "network_packet_size_bytes",
            "Size of packets sent / received in bytes",
            &["direction"],
            vec![32.0, 64.0, 128.0, 256.0, 512.0, 1024.0, 1400.0, 2048.0],
        )
        .unwrap();
        registry
            .register(Box::new(network_packet_size_bytes.clone()))
            .unwrap();

        let server_rtt =
            Histogram::with_opts(HistogramOpts::new("server_rtt", "Packet round trip time"))
                .unwrap();
        registry.register(Box::new(server_rtt.clone())).unwrap();

        #[cfg(target_os = "linux")]
        {
            use prometheus::process_collector::ProcessCollector;

            let process_collector = ProcessCollector::for_self();
            registry.register(Box::new(process_collector)).unwrap();
        }

        Self {
            registry: Arc::new(Mutex::new(registry)),
            connected_players,
            tick_rate,
            network_packets_total,
            network_bytes_total,
            network_packet_size_bytes,
            server_rtt,
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

        let task_path = path.clone();
        tokio::task::spawn_blocking(move || match File::create(&task_path) {
            Ok(mut file) => {
                if let Err(err) = file.write_all(&buffer) {
                    error!(?err, "failed to write metrics to file");
                }
            }
            Err(err) => error!(?err, "failed to create metrics file"),
        });
    }
}

pub fn init_subscriber(settings: &TracingSettings) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let (pretty, json) = match settings.format {
        TracingFormat::Pretty => (Some(fmt::layer().compact().with_ansi(true)), None),
        TracingFormat::Json => (
            None,
            Some(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true),
            ),
        ),
    };

    let otel = if let Some(endpoint) = &settings.otel_exporter_endpoint {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_protocol(Protocol::Grpc)
            .with_endpoint(endpoint)
            .build()
            .expect("tracing exporter built");

        let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(
                opentelemetry_sdk::Resource::builder()
                    .with_service_name("mmo-server")
                    .build(),
            )
            .build();
        global::set_tracer_provider(tracer_provider);
        global::set_text_map_propagator(TraceContextPropagator::new());

        let otel_layer = tracing_opentelemetry::layer().with_tracer(global::tracer(""));
        Some(otel_layer)
    } else {
        None
    };

    #[cfg(feature = "profiling")]
    let tracy_layer = tracing_tracy::TracyLayer::default();

    let subscriber = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(pretty)
        .with(json)
        .with(otel);

    #[cfg(feature = "profiling")]
    let subscriber = subscriber.with(tracy_layer);

    LogTracer::init().expect("logger init succeeded");
    tracing::subscriber::set_global_default(subscriber).expect("set subscriber succeeded");
}

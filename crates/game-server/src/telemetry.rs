use crate::configuration::{TracingFormat, TracingSettings};
use metrics::{Unit, describe_counter, describe_gauge, describe_histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use metrics_util::layers::{Layer, PrefixLayer};
use opentelemetry::global;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tokio::time::Duration;
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt};

pub const SERVER_TICK_METRIC: &str = "server_tick";
pub const CONNECTED_PLAYERS_METRIC: &str = "connected_players_count";
pub const NETWORK_PACKETS_TOTAL_METRIC: &str = "network_packets_total";
pub const NETWORK_BYTES_TOTAL_METRIC: &str = "network_bytes_total";
pub const SERVER_RTT_METRIC: &str = "server_rtt";
pub const CHAT_MESSAGES_TOTAL_METRIC: &str = "chat_messages_total";
pub const SPELL_CASTS_TOTAL_METRIC: &str = "spell_casts_total";
pub const AUTO_ATTACKS_TOTAL_METRIC: &str = "auto_attacks_total";
pub const PLAYER_DEATHS_TOTAL_METRIC: &str = "player_deaths_total";
pub const MOB_KILLS_TOTAL_METRIC: &str = "mob_kills_total";
pub const LOOT_ITEMS_GENERATED_TOTAL_METRIC: &str = "loot_items_generated_total";
pub const AI_STATE_TRANSITIONS_TOTAL_METRIC: &str = "ai_state_transitions_total";
pub const AI_EVADES_TOTAL_METRIC: &str = "ai_evades_total";

pub async fn init_prometheus_exporter() {
    tracing::info!("starting prometheus exporter");
    let (recorder, exporter) = PrometheusBuilder::new()
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::HISTOGRAM,
            Some(Duration::from_secs(10)),
        )
        .build()
        .expect("failed to build prometheus exporter");
    let prefixed_recorder = PrefixLayer::new("game").layer(recorder);
    metrics::set_global_recorder(prefixed_recorder).expect("failed to set global recorder");

    describe_histogram!(SERVER_RTT_METRIC, Unit::Seconds, "Packet round trip time");
    describe_counter!(
        NETWORK_BYTES_TOTAL_METRIC,
        Unit::Bytes,
        "The total number of bytes sent / received"
    );
    describe_counter!(
        NETWORK_PACKETS_TOTAL_METRIC,
        "The total number of packets sent / received"
    );
    describe_gauge!(
        CONNECTED_PLAYERS_METRIC,
        "Current number of connected players"
    );
    describe_counter!(SERVER_TICK_METRIC, "The current server tick");
    describe_counter!(
        CHAT_MESSAGES_TOTAL_METRIC,
        "Total chat messages sent by players"
    );
    describe_counter!(
        SPELL_CASTS_TOTAL_METRIC,
        "Total spell cast attempts by players"
    );
    describe_counter!(
        AUTO_ATTACKS_TOTAL_METRIC,
        "Total auto-attack swings that landed"
    );
    describe_counter!(
        PLAYER_DEATHS_TOTAL_METRIC,
        "Total player deaths"
    );
    describe_counter!(
        MOB_KILLS_TOTAL_METRIC,
        "Total mobs killed by players"
    );
    describe_counter!(
        LOOT_ITEMS_GENERATED_TOTAL_METRIC,
        "Total loot items generated from kills"
    );
    describe_counter!(
        AI_STATE_TRANSITIONS_TOTAL_METRIC,
        "Total AI state machine transitions"
    );
    describe_counter!(
        AI_EVADES_TOTAL_METRIC,
        "Total mob evade/leash events"
    );

    tracing::info!("serving metrics on port 9000");
    exporter.await.expect("failed to start exporter");
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
                    .with_service_name("game-server")
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

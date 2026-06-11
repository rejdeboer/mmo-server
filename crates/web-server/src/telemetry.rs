use crate::configuration::{MetricsSettings, TelemetrySettings, TracingFormat};
use metrics::{Unit, describe_counter, describe_gauge, describe_histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use metrics_util::layers::{Layer, PrefixLayer};
use opentelemetry::global;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{Resource, propagation::TraceContextPropagator};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::time::Duration;
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt};

pub fn init_metrics(settings: &MetricsSettings) {
    let addr = SocketAddr::from(([0, 0, 0, 0], settings.port));

    let (recorder, exporter) = PrometheusBuilder::new()
        .with_http_listener(addr)
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::HISTOGRAM,
            Some(Duration::from_secs(10)),
        )
        .build()
        .expect("failed to build prometheus exporter");

    let prefixed_recorder = PrefixLayer::new("web").layer(recorder);
    metrics::set_global_recorder(prefixed_recorder).expect("failed to set global recorder");

    // -- Connection metrics --
    describe_gauge!(
        "social_connections_active",
        "Current number of active WebSocket connections"
    );
    describe_histogram!(
        "social_connection_duration_seconds",
        Unit::Seconds,
        "WebSocket session lifetimes"
    );

    // -- Chat metrics --
    describe_counter!(
        "social_messages_total",
        "Total chat messages sent by channel type"
    );
    describe_counter!(
        "social_messages_delivered_total",
        "Messages delivered by channel and delivery method"
    );

    // -- Party metrics --
    describe_counter!(
        "social_party_actions_total",
        "Party actions performed by action type"
    );
    describe_gauge!("social_parties_active", "Current number of active parties");

    // -- Rate limiting --
    describe_counter!(
        "social_rate_limit_denied_total",
        "Messages denied by rate limiter"
    );

    // -- Errors --
    describe_counter!("social_errors_total", "Social hub errors by error type");

    // -- NATS metrics --
    describe_counter!(
        "nats_publishes_total",
        "NATS messages published by subject prefix"
    );
    describe_counter!(
        "nats_publish_failures_total",
        "Failed NATS publishes by subject prefix"
    );
    describe_counter!(
        "nats_messages_received_total",
        "NATS messages received by subject prefix"
    );

    // -- Hub internals --
    describe_gauge!(
        "social_guilds_active",
        "Guilds with at least one connected member"
    );

    // -- HTTP layer metrics --
    describe_counter!(
        "http_requests_total",
        "Total HTTP requests by method, route, and status"
    );
    describe_histogram!(
        "http_request_duration_seconds",
        Unit::Seconds,
        "HTTP request latency by method and route"
    );

    // Initialize gauges to 0
    metrics::gauge!("social_connections_active").set(0.0);
    metrics::gauge!("social_parties_active").set(0.0);
    metrics::gauge!("social_guilds_active").set(0.0);

    tokio::spawn(async move {
        tracing::info!("serving metrics on {}", addr);
        exporter.await.expect("failed to start metrics exporter");
    });
}

pub fn init_telemetry(settings: &TelemetrySettings) {
    init_subscriber(settings);
}

pub fn init_subscriber(settings: &TelemetrySettings) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let (pretty, json) = match settings.tracing_format {
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
            .with_resource(Resource::builder().with_service_name("web-server").build())
            .build();
        global::set_tracer_provider(tracer_provider);
        global::set_text_map_propagator(TraceContextPropagator::new());

        let otel_layer = tracing_opentelemetry::layer().with_tracer(global::tracer(""));
        Some(otel_layer)
    } else {
        None
    };

    let subscriber = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(pretty)
        .with(json)
        .with(otel);
    LogTracer::init().expect("logger init succeeded");
    set_global_default(subscriber).expect("set subscriber succeeded");
}

pub fn get_trace_parent() -> Option<String> {
    let context = tracing::Span::current().context();
    opentelemetry::global::get_text_map_propagator(|propagator| {
        let mut context_carrier = HashMap::<String, String>::new();
        propagator.inject_context(&context, &mut context_carrier);
        context_carrier.get("traceparent").cloned()
    })
}

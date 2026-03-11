use crate::configuration::{TelemetrySettings, TracingFormat};
use once_cell::sync::Lazy;
use opentelemetry::global;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{Resource, propagation::TraceContextPropagator};
use prometheus::IntGauge;
use std::collections::HashMap;
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt};

pub static REGISTRY: Lazy<prometheus::Registry> =
    Lazy::new(|| prometheus::Registry::new_custom(Some("web".to_string()), None).unwrap());

pub static ACTIVE_WS_CONNECTIONS: Lazy<IntGauge> = Lazy::new(|| {
    IntGauge::new(
        "social_active_ws_connections",
        "Current number of active WebSocket connections.",
    )
    .unwrap()
});

pub fn init_telemetry(settings: &TelemetrySettings) {
    register_metrics();
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

fn register_metrics() {
    REGISTRY
        .register(Box::new(ACTIVE_WS_CONNECTIONS.clone()))
        .unwrap();

    #[cfg(target_os = "linux")]
    {
        use prometheus::process_collector::ProcessCollector;

        let process_collector = ProcessCollector::for_self();
        REGISTRY.register(Box::new(process_collector)).unwrap();
    }
}

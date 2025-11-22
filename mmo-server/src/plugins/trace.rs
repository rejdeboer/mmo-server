use crate::configuration::{TracingFormat, TracingSettings};
use bevy::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

pub struct TracePlugin {
    settings: TracingSettings,
}

impl TracePlugin {
    pub fn new(settings: TracingSettings) -> Self {
        Self { settings }
    }
}

impl Plugin for TracePlugin {
    fn build(&self, app: &mut App) {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let (pretty, json) = match self.settings.format {
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

        let otel = if let Some(endpoint) = &self.settings.otel_exporter_endpoint {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_protocol(Protocol::Grpc)
                .with_endpoint(endpoint)
                .build()
                .expect("tracing exporter built");

            let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(Resource::builder().with_service_name("mmo-server").build())
                .build();
            global::set_tracer_provider(tracer_provider);

            let otel_layer = tracing_opentelemetry::layer().with_tracer(global::tracer(""));
            Some(otel_layer)
        } else {
            None
        };

        let mut tracy_layer = None;
        #[cfg(feature = "profiling")]
        {}

        let subscriber = tracing_subscriber::Registry::default()
            .with(env_filter)
            .with(pretty)
            .with(json)
            .with(otel)
            .with(tracy_layer);
        LogTracer::init().expect("logger init succeeded");
        set_global_default(subscriber).expect("set subscriber succeeded");
    }
}

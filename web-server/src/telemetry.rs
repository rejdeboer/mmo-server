use crate::configuration::Environment;
use once_cell::sync::Lazy;
use prometheus::{IntGauge, process_collector::ProcessCollector};
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_log::LogTracer;
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

pub fn init_telemetry() {
    register_metrics();
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let environment = Environment::read();
    if matches!(environment, Environment::Local) {
        init_subscriber(get_local_subscriber(env_filter));
    } else {
        init_subscriber(get_subscriber(env_filter));
    }
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("logger init succeeded");
    set_global_default(subscriber).expect("set subscriber succeeded");
}

pub fn get_local_subscriber(env_filter: EnvFilter) -> impl Subscriber + Send + Sync {
    let fmt_layer = fmt::layer().compact().with_ansi(true);

    tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(fmt_layer)
}

pub fn get_subscriber(env_filter: EnvFilter) -> impl Subscriber + Send + Sync {
    let fmt_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true);

    tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(fmt_layer)
}

fn register_metrics() {
    REGISTRY
        .register(Box::new(ACTIVE_WS_CONNECTIONS.clone()))
        .unwrap();

    let process_collector = ProcessCollector::for_self();
    REGISTRY.register(Box::new(process_collector)).unwrap();
}

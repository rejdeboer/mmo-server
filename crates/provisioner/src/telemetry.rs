use tracing::{Subscriber, subscriber::set_global_default};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt};

pub fn init_telemetry() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    init_subscriber(get_subscriber(env_filter));
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("logger init succeeded");
    set_global_default(subscriber).expect("set subscriber succeeded");
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

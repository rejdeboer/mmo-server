use tracing::{Subscriber, subscriber::set_global_default};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt};

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("logger init succeeded");
    set_global_default(subscriber).expect("set subscriber succeeded");
}

pub fn get_local_subscriber(env_filter: EnvFilter) -> impl Subscriber + Send + Sync {
    let fmt_layer = fmt::layer().compact().with_ansi(true);

    Registry::default().with(env_filter).with(fmt_layer)
}

pub fn get_subscriber(env_filter: EnvFilter) -> impl Subscriber + Send + Sync {
    let fmt_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true);

    Registry::default().with(env_filter).with(fmt_layer)
}

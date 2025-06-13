use tracing::{subscriber::set_global_default, Subscriber};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber() -> impl Subscriber + Send + Sync {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "web_server=debug,tower_http=debug".into());
    let formatting_layer = tracing_subscriber::fmt::layer();
    Registry::default().with(env_filter).with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("logger init succeeded");
    set_global_default(subscriber).expect("set subscriber succeeded");
}

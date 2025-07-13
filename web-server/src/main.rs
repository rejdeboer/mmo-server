use web_server::{
    configuration::{self, Environment},
    server::Application,
    telemetry::{get_local_subscriber, get_subscriber, init_subscriber},
};

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let environment = Environment::read();
    if matches!(environment, Environment::Local) {
        init_subscriber(get_local_subscriber(env_filter));
    } else {
        init_subscriber(get_subscriber(env_filter));
    }

    let settings = configuration::get_configuration().expect("config fetched");

    let application = Application::build(settings).await?;
    application.run_until_stopped().await?;
    Ok(())
}

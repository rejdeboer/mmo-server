use crate::{configuration, server::Application, telemetry::get_subscriber};

use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber();
    subscriber.init();

    let settings = configuration::get_configuration().expect("config fetched");

    let application = Application::build(settings).await?;
    application.run_until_stopped().await?;
    Ok(())
}

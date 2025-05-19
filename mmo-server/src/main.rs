use mmo_server::{application, configuration, telemetry::get_subscriber};

use tracing_subscriber::util::SubscriberInitExt;

fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber();
    subscriber.init();

    let settings = configuration::get_configuration().expect("config fetched");

    let (mut app, _) = application::build(settings)?;
    bevy::log::info!("starting application");
    app.run();
    Ok(())
}

use mmo_server::{application, configuration, telemetry::init_subscriber};

fn main() -> anyhow::Result<()> {
    let settings = configuration::get_configuration().expect("config fetched");

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        init_subscriber(&settings.tracing);
    });

    let (mut app, _) = application::build(settings)?;
    bevy::log::info!("starting application");
    app.run();
    Ok(())
}

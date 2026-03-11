use mmo_server::{agones::agones_connect, application, configuration, telemetry::init_subscriber};

fn main() -> anyhow::Result<()> {
    let mut settings = configuration::get_configuration().expect("config fetched");

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        init_subscriber(&settings.tracing);
    });

    if settings.server.enable_agones {
        let (public_host, public_port) = rt.block_on(async { agones_connect().await });
        settings.server.public_host = Some(public_host);
        settings.server.public_port = Some(public_port);
    }

    let (mut app, _) = application::build(settings)?;
    tracing::info!("starting application");
    app.run();
    Ok(())
}

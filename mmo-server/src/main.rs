use agones::Sdk;
use mmo_server::{
    agones::send_health_pings, application, configuration, telemetry::init_subscriber,
};

fn main() -> anyhow::Result<()> {
    let mut settings = configuration::get_configuration().expect("config fetched");

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        init_subscriber(&settings.tracing);
    });

    if settings.server.enable_agones {
        let (public_host, public_port) = rt.block_on(async {
            let sdk = Sdk::new(None, None)
                .await
                .expect("failed to connect to Agones server");

            match sdk.ready().await {
                Ok(()) => tracing::info!("server connected to Agones and marked as Ready"),
                Err(err) => panic!("failed to mark server as ready: {}", err),
            }

            let server_status = sdk
                .get_gameserver()
                .await
                .expect("game server retrieved")
                .status
                .expect("game server status retrieved");

            settings.server.public_host = Some(server_status.address);
            settings.server.public_port = Some(server_status.ports[0].port as u16);

            tokio::spawn(async move {
                send_health_pings(sdk);
            })
        });
    }

    let (mut app, _) = application::build(settings)?;
    bevy::log::info!("starting application");
    app.run();
    Ok(())
}

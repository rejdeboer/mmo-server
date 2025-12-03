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
            tracing::info!("connecting to Agones server");
            let mut sdk = Sdk::new(None, None)
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

            tokio::spawn(async move {
                send_health_pings(sdk).await;
            });

            (server_status.address, server_status.ports[0].port as u16)
        });

        settings.server.public_host = Some(public_host);
        settings.server.public_port = Some(public_port);
    }

    let (mut app, _) = application::build(settings)?;
    tracing::info!("starting application");
    app.run();
    Ok(())
}

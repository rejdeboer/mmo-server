use agones::Sdk;
use std::time::Duration;
use tokio::time::Instant;

const CHECK_DURATION: Duration = Duration::from_secs(5);

pub async fn agones_connect() -> (String, u16) {
    tracing::info!("connecting to Agones server");
    let mut sdk = Sdk::new(None, None)
        .await
        .expect("failed to connect to Agones server");

    let (host, port) = wait_for_public_address(&mut sdk).await;

    match sdk.ready().await {
        Ok(()) => tracing::info!("server connected to Agones and marked as Ready"),
        Err(err) => panic!("failed to mark server as ready: {}", err),
    }

    tokio::spawn(async move {
        send_health_pings(sdk).await;
    });

    (host, port)
}

async fn send_health_pings(sdk: Sdk) {
    let mut interval = tokio::time::interval(CHECK_DURATION);
    let mut last_tick = Instant::now();

    loop {
        let dt = interval.tick().await.duration_since(last_tick);
        last_tick += dt;

        let health = sdk.health_check();
        if let Err(err) = health.send(()).await {
            tracing::error!(?err, "health ping failed");
        }
    }
}

async fn wait_for_public_address(sdk: &mut Sdk) -> (String, u16) {
    tracing::info!("waiting for Agones to assign public IP/Port...");
    let mut watcher = sdk
        .watch_gameserver()
        .await
        .expect("failed to watch GameServer");

    while let Some(gameserver) = watcher
        .message()
        .await
        .expect("failed to fetch Agones watcher message")
    {
        if let Some(status) = &gameserver.status
            && !status.address.is_empty()
            && let Some(port_info) = status.ports.first()
        {
            let public_port = port_info.port;
            tracing::info!(
                "Agones Identity Assigned: {}:{}",
                status.address,
                public_port
            );
            return (status.address.clone(), public_port as u16);
        }
        tracing::info!("... still waiting for IP assignment ...");
    }

    panic!("Agones Watcher stream closed unexpectedly!");
}

use std::time::Duration;

use agones::Sdk;
use tokio::time::Instant;

const CHECK_DURATION: Duration = Duration::from_secs(5);

pub async fn send_health_pings(sdk: Sdk) {
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

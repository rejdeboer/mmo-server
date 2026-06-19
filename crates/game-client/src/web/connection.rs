use bevy::prelude::*;
use tokio::sync::mpsc;

use crate::application::WebApi;
use crate::configuration::Settings;

/// Sender half of the social WebSocket connection. `None` until connected.
#[derive(Resource)]
pub struct SocialSender(pub Option<mpsc::Sender<web_client::SocialAction>>);

/// Receiver half of the social WebSocket connection. `None` until connected.
#[derive(Resource)]
pub struct SocialReceiver(pub Option<mpsc::Receiver<web_client::SocialEvent>>);

/// Spawns the social WebSocket connection task on the tokio runtime.
pub fn connect_social(
    web_api: Res<WebApi>,
    settings: Res<Settings>,
    runtime: Res<bevy_tokio_tasks::TokioTasksRuntime>,
) {
    let Some(jwt) = web_api.0.jwt() else {
        tracing::warn!("cannot connect to social server: no JWT available");
        return;
    };

    let ws_url = format!(
        "{}/social",
        settings
            .web_server
            .endpoint
            .replace("http://", "ws://")
            .replace("https://", "wss://")
    );
    let jwt = jwt.to_owned();

    runtime.spawn_background_task(|mut ctx| async move {
        match web_client::connect(&ws_url, &jwt).await {
            Ok((sender, receiver)) => {
                ctx.run_on_main_thread(move |main_ctx| {
                    main_ctx.world.resource_mut::<SocialSender>().0 = Some(sender);
                    main_ctx.world.resource_mut::<SocialReceiver>().0 = Some(receiver);
                    tracing::info!("social WebSocket connected");
                })
                .await;
            }
            Err(e) => {
                tracing::error!("failed to connect to social WebSocket: {:?}", e);
            }
        }
    });

    tracing::info!("social WebSocket connection task spawned");
}

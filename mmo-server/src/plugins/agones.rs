use agones::Sdk;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use std::time::Duration;

#[derive(Resource)]
struct HealthCheckTimer(Timer);

#[derive(Resource)]
struct AgonesSdk(pub Sdk);

pub struct AgonesPlugin;

impl Plugin for AgonesPlugin {
    fn build(&self, app: &mut App) {
        info!("Agones SDK enabled, connecting...");
        app.insert_resource(HealthCheckTimer(Timer::new(
            Duration::from_secs(5),
            TimerMode::Repeating,
        )))
        .add_systems(Startup, init_sdk)
        .add_systems(PostStartup, mark_ready)
        .add_systems(Update, send_health_pings);
    }
}

fn init_sdk(runtime: Res<TokioTasksRuntime>, mut commands: Commands) {
    let sdk = runtime.runtime().block_on(async move {
        Sdk::new(None, None)
            .await
            .expect("failed to connect to Agones server")
    });
    commands.insert_resource(AgonesSdk(sdk));
}

fn mark_ready(runtime: Res<TokioTasksRuntime>, sdk: Res<AgonesSdk>) {
    let mut sdk = sdk.0.clone();
    runtime.runtime().block_on(async move {
        match sdk.ready().await {
            Ok(()) => info!("server connected to Agones and marked as Ready"),
            Err(err) => panic!("failed to mark server as ready: {}", err),
        }
    });
}

fn send_health_pings(
    sdk: Res<AgonesSdk>,
    runtime: Res<TokioTasksRuntime>,
    mut timer: ResMut<HealthCheckTimer>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        let sdk = sdk.0.clone();
        runtime.spawn_background_task(|_ctx| async move {
            let health = sdk.health_check();
            if let Err(err) = health.send(()).await {
                error!(?err, "health ping failed");
            }
        });
    }
}

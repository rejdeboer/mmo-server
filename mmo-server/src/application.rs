use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfLoaderSettings;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_renet::RenetServerPlugin;
use bevy_renet::netcode::{
    NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig,
};
use bevy_renet::renet::{ConnectionConfig, RenetServer};
use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use crate::configuration::Settings;
use crate::messages::{IncomingChatMessage, MoveActionMessage, OutgoingMessage};
use crate::plugins::{AgonesPlugin, AppPlugin};
use crate::telemetry::{Metrics, run_metrics_exporter};

#[derive(Resource, Clone)]
pub struct DatabasePool(pub PgPool);

#[derive(Debug, Resource, Default)]
pub struct SpatialGrid {
    pub cells: HashMap<IVec2, Vec<Entity>>,
}

pub fn build(settings: Settings) -> Result<(App, u16), std::io::Error> {
    let mut app = App::new();

    app.add_plugins(AppPlugin);
    app.add_plugins(RenetServerPlugin);
    app.add_plugins(NetcodeServerPlugin);
    app.add_plugins(TokioTasksPlugin::default());
    app.add_plugins(AgonesPlugin);
    app.add_plugins(PhysicsPlugins::new(PostUpdate));

    let ip_addr = IpAddr::V4(
        settings
            .server
            .host
            .parse()
            .expect("host should be IPV4 addr"),
    );
    let server_addr: SocketAddr = SocketAddr::new(ip_addr, settings.server.port);
    let socket = UdpSocket::bind(server_addr)?;
    let public_addresses: Vec<SocketAddr> = vec![server_addr];

    let port = socket.local_addr()?.port();

    let authentication = match settings.server.is_secure {
        true => ServerAuthentication::Secure {
            private_key: settings.server.netcode_private_key,
        },
        false => ServerAuthentication::Unsecure,
    };

    let netcode_server = RenetServer::new(ConnectionConfig::default());
    bevy::log::info!(
        "listening on {}; secure: {}",
        socket.local_addr()?,
        settings.server.is_secure
    );
    let netcode_transport = NetcodeServerTransport::new(
        ServerConfig {
            current_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
            max_clients: 100,
            protocol_id: 0,
            public_addresses,
            authentication,
        },
        socket,
    )?;

    app.insert_resource(Time::<Fixed>::from_hz(20.));
    app.insert_resource(netcode_server);
    app.insert_resource(netcode_transport);
    app.insert_resource(settings);
    app.insert_resource(SpatialGrid::default());
    app.insert_resource(Metrics::default());

    app.add_message::<IncomingChatMessage>();
    app.add_message::<OutgoingMessage>();
    app.add_message::<MoveActionMessage>();

    app.add_systems(
        Startup,
        (setup_database_pool, setup_world, setup_metrics_exporter),
    );
    app.add_systems(
        FixedPreUpdate,
        (
            crate::systems::process_client_actions,
            (
                crate::systems::process_incoming_chat,
                crate::systems::process_move_action_messages,
            ),
        )
            .chain(),
    );
    app.add_systems(Update, crate::systems::handle_connection_events);
    app.add_systems(FixedUpdate, crate::systems::update_spatial_grid);
    app.add_systems(
        FixedPostUpdate,
        (
            crate::systems::update_player_visibility,
            (
                crate::systems::send_transform_updates,
                crate::systems::sync_players,
            )
                .chain(),
        )
            .after(PhysicsSystems::Last),
    );

    Ok((app, port))
}

pub fn get_connection_pool(settings: &Settings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(settings.database.with_db())
}

fn setup_database_pool(
    mut commands: Commands,
    runtime: Res<TokioTasksRuntime>,
    settings: Res<Settings>,
) {
    let pool = runtime
        .runtime()
        .block_on(async move { get_connection_pool(&settings) });
    commands.insert_resource(DatabasePool(pool));
}

fn setup_world(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        SceneRoot(
            assets.load_with_settings("world.gltf#Scene0", |s: &mut GltfLoaderSettings| {
                s.load_materials = RenderAssetUsages::empty();
                s.load_cameras = false;
                s.load_lights = false;
                s.load_animations = false;
            }),
        ),
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexDecompositionFromMesh),
        Transform::from_xyz(0., 0., 0.),
        RigidBody::Static,
    ));
}

fn setup_metrics_exporter(
    runtime: Res<TokioTasksRuntime>,
    metrics: Res<Metrics>,
    settings: Res<Settings>,
) {
    info!("starting metrics exporter");
    let metrics_clone = metrics.clone();
    let path = settings.server.metrics_path.clone();

    runtime.spawn_background_task(async move |_ctx| {
        run_metrics_exporter(metrics_clone, path).await;
    });
}

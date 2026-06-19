use crate::ai::AiPlugin;
use crate::assets::ContentPlugin;
use crate::combat::{CombatPlugin, CombatSet};
use crate::configuration::Settings;
use crate::core::{NetworkIdCounter, ServerTick};
use crate::database::DatabasePlugin;
use crate::economy::EconomyPlugin;
use crate::networking::{NetworkingPlugin, NetworkingSet};
use crate::observability::ObservabilityPlugin;
use crate::social::{SocialPlugin, SocialSet};
use crate::world::{WorldPlugin, WorldSet};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksPlugin;
use game_core::networking::NetworkIdMapping;

pub fn build(settings: Settings) -> App {
    let mut app = App::new();

    // App runner (headless vs debug)
    #[cfg(feature = "debug")]
    {
        use avian3d::prelude::PhysicsDebugPlugin;
        use bevy::log::LogPlugin;
        use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

        app.add_plugins((
            DefaultPlugins.build().disable::<LogPlugin>(),
            PhysicsDebugPlugin,
            PanOrbitCameraPlugin,
        ));
        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn((
                PanOrbitCamera::default(),
                Transform::from_xyz(-10., 10., 15.).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        });
        info!("running in debug mode");
    }
    #[cfg(not(feature = "debug"))]
    {
        use bevy::app::ScheduleRunnerPlugin;
        use bevy::gltf::GltfPlugin;
        use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
        use bevy::mesh::MeshPlugin;
        use bevy::scene::ScenePlugin;

        let loop_interval =
            std::time::Duration::from_secs_f64(1.0 / game_core::constants::TICK_RATE_HZ);
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(loop_interval)));
        app.insert_resource(CompressedImageFormatSupport(CompressedImageFormats::NONE));
        app.add_plugins((
            AssetPlugin::default(),
            GltfPlugin::default(),
            MeshPlugin,
            ScenePlugin,
        ));
        app.init_asset::<StandardMaterial>();
        app.register_type::<MeshMaterial3d<StandardMaterial>>();
        info!("running in headless mode");
    }

    // Infrastructure plugins
    app.add_plugins((
        TokioTasksPlugin::default(),
        PhysicsPlugins::new(FixedPostUpdate),
    ));

    // Game plugins
    app.add_plugins((
        DatabasePlugin,
        ContentPlugin,
        NetworkingPlugin::new(&settings.server),
        CombatPlugin,
        WorldPlugin,
        SocialPlugin,
        EconomyPlugin,
        AiPlugin,
        ObservabilityPlugin,
    ));

    // Cross-plugin system ordering
    app.configure_sets(
        FixedPreUpdate,
        (
            SocialSet::ReceiveUpdates,
            WorldSet::Tick,
            NetworkingSet::ReceiveInput,
            WorldSet::PreProcess,
            (
                CombatSet::ProcessActions,
                SocialSet::ProcessChat,
                WorldSet::ProcessMovement,
            ),
        )
            .chain(),
    );

    app.configure_sets(
        FixedPostUpdate,
        (
            CombatSet::ApplyEffects,
            NetworkingSet::UpdateVisibility,
            NetworkingSet::Sync,
        )
            .chain()
            .after(PhysicsSystems::Last),
    );

    app.insert_resource(Time::<Fixed>::from_hz(game_core::constants::TICK_RATE_HZ));
    app.insert_resource(settings);
    app.insert_resource(ServerTick::default());
    app.insert_resource(NetworkIdCounter::default());
    app.insert_resource(NetworkIdMapping::default());

    app
}

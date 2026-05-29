use bevy::app::ScheduleRunnerPlugin;
use bevy::gltf::GltfPlugin;
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use game_core::constants::TICK_RATE_HZ;
use std::time::Duration;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Cap the main loop to the tick rate so we don't spin the CPU.
        // FixedUpdate still runs at exactly TICK_RATE_HZ regardless of this cap,
        // but this prevents Update from running thousands of times per second.
        let loop_interval = Duration::from_secs_f64(1.0 / TICK_RATE_HZ);
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(loop_interval)));

        // Asset plugins
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
}

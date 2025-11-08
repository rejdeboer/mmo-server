use bevy::gltf::GltfPlugin;
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
use bevy::log::LogPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MinimalPlugins);

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

        app.add_plugins(LogPlugin::default());
        info!("running in headless mode");
    }
}

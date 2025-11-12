use crate::configuration::Environment;
use bevy::gltf::GltfPlugin;
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
use bevy::log::LogPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use tracing_subscriber::{EnvFilter, Layer, fmt};

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

        let environment = Environment::read();
        if matches!(environment, Environment::Local) {
            app.add_plugins(LogPlugin::default());
        } else {
            app.add_plugins(LogPlugin {
                fmt_layer: move |_app: &mut App| {
                    let fmt_layer = fmt::layer()
                        .json()
                        .with_filter(EnvFilter::from_default_env());
                    Some(Box::new(fmt_layer) as Box<dyn Layer<_> + Send + Sync>)
                },
                ..Default::default()
            });
        }
        info!("running in headless mode");
    }
}

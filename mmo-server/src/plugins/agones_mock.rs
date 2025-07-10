use bevy::prelude::*;

pub struct AgonesPlugin;

impl Plugin for AgonesPlugin {
    fn build(&self, _app: &mut App) {
        bevy::log::info!("using agones mock");
    }
}

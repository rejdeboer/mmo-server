use avian3d::prelude::PhysicsDebugPlugin;
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.build().disable::<LogPlugin>(),
            PhysicsDebugPlugin,
            PanOrbitCameraPlugin,
        ));
        info!("running in debug mode");

        app.add_systems(Startup, setup_camera);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        PanOrbitCamera::default(),
        Transform::from_xyz(-10., 10., 15.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

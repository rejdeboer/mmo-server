use bevy::prelude::*;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins);
        info!("running in debug mode");

        app.add_systems(Startup, setup_camera);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Camera::default(),
        Transform::from_xyz(-10., 10., 15.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

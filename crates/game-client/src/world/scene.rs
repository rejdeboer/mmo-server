use bevy::{gltf::GltfLoaderSettings, prelude::*};
use avian3d::prelude::*;

pub fn setup_world(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        SceneRoot(
            assets.load_with_settings("world.gltf#Scene0", |s: &mut GltfLoaderSettings| {
                s.load_cameras = false;
                s.load_lights = false;
                s.load_animations = false;
            }),
        ),
        // TODO: We are trying to match Godot here to make it work, but this is hacky
        Transform::from_xyz(0., -3., 0.),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));
}

use crate::assets::{MonsterLibrary, MonsterLibraryHandle, SpellLibrary, SpellLibraryHandle};
use avian3d::prelude::*;
use bevy::{asset::RenderAssetUsages, gltf::GltfLoaderSettings, prelude::*};
use bevy_common_assets::ron::RonAssetPlugin;

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<MonsterLibrary>::new(&["monsters.ron"]),
            RonAssetPlugin::<SpellLibrary>::new(&["spells.ron"]),
        ));

        app.add_systems(PreStartup, setup_assets);
    }
}

fn setup_assets(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        SceneRoot(
            assets.load_with_settings("world.gltf#Scene0", |s: &mut GltfLoaderSettings| {
                s.load_materials = RenderAssetUsages::empty();
                s.load_cameras = false;
                s.load_lights = false;
                s.load_animations = false;
            }),
        ),
        // TODO: We are trying to match Godot here to make it work, but this is hacky
        Transform::from_xyz(0., -2., 0.),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
    ));

    let monsters_handle = assets.load::<MonsterLibrary>("monsters.ron");
    commands.insert_resource(MonsterLibraryHandle(monsters_handle));
    let spells_handle = assets.load::<SpellLibrary>("spells.ron");
    commands.insert_resource(SpellLibraryHandle(spells_handle));
}

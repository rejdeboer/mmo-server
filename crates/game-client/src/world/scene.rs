use avian3d::prelude::*;
use bevy::{gltf::GltfLoaderSettings, prelude::*};
use game_core::{
    props::{CollisionType, PropsConfig, PropsConfigHandle, model_name_from_asset_path},
    zone::{ZoneDef, ZoneDefHandle},
};

pub fn load_zone(mut commands: Commands, assets: Res<AssetServer>) {
    let handle = assets.load::<ZoneDef>("world/zones/meadow.zone.ron");
    commands.insert_resource(ZoneDefHandle(handle));

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));
}

/// Marker component to indicate the zone has been spawned.
#[derive(Component)]
pub struct ZoneTerrain;

/// Marker component for prop entities spawned from the zone definition.
#[derive(Component)]
pub struct ZoneProp;

pub fn spawn_zone_when_ready(
    mut commands: Commands,
    zone_handle: Option<Res<ZoneDefHandle>>,
    zone_assets: Res<Assets<ZoneDef>>,
    props_handle: Option<Res<PropsConfigHandle>>,
    props_assets: Res<Assets<PropsConfig>>,
    assets: Res<AssetServer>,
    terrain_query: Query<&ZoneTerrain>,
) {
    let Some(zone_h) = zone_handle else { return };
    let Some(props_h) = props_handle else { return };

    // Already spawned
    if !terrain_query.is_empty() {
        return;
    }

    let Some(zone) = zone_assets.get(&zone_h.0) else {
        return;
    };
    let Some(props_config) = props_assets.get(&props_h.0) else {
        return;
    };

    // Spawn terrain with full materials and trimesh collision
    if !zone.terrain.is_empty() {
        commands.spawn((
            ZoneTerrain,
            SceneRoot(
                assets.load_with_settings(
                    format!("{}#Scene0", zone.terrain),
                    |s: &mut GltfLoaderSettings| {
                        s.load_cameras = false;
                        s.load_lights = false;
                        s.load_animations = false;
                    },
                ),
            ),
            Transform::default(),
            RigidBody::Static,
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
        ));
    }

    // Spawn props
    for prop in &zone.props {
        let model_name = model_name_from_asset_path(&prop.asset);
        let collision = props_config
            .props
            .get(model_name)
            .map(|d| d.collision)
            .unwrap_or(CollisionType::None);

        let mut entity = commands.spawn((
            ZoneProp,
            SceneRoot(
                assets.load_with_settings(
                    format!("{}#Scene0", prop.asset),
                    |s: &mut GltfLoaderSettings| {
                        s.load_cameras = false;
                        s.load_lights = false;
                        s.load_animations = false;
                    },
                ),
            ),
            prop.transform(),
        ));

        match collision {
            CollisionType::ConvexHull => {
                entity.insert((
                    RigidBody::Static,
                    ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
                ));
            }
            CollisionType::TrimeshFromMesh => {
                entity.insert((
                    RigidBody::Static,
                    ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
                ));
            }
            CollisionType::None => {}
        }
    }

    tracing::info!(
        zone_id = %zone.id,
        prop_count = zone.props.len(),
        "zone loaded"
    );
}

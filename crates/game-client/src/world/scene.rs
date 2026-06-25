use avian3d::prelude::*;
use bevy::{gltf::GltfLoaderSettings, light::NotShadowCaster, prelude::*};
use game_core::{
    props::{CollisionType, PropsConfig, PropsConfigHandle, model_name_from_asset_path},
    zone::{ZoneDef, ZoneDefHandle},
};

use super::camera::ThirdPersonCamera;

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

/// Marker component for the skydome entity.
#[derive(Component)]
pub struct Skydome;

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
            SceneRoot(assets.load_with_settings(
                format!("{}#Scene0", zone.terrain),
                |s: &mut GltfLoaderSettings| {
                    s.load_cameras = false;
                    s.load_lights = false;
                    s.load_animations = false;
                },
            )),
            Transform::default(),
            RigidBody::Static,
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
        ));
    }

    // Spawn skydome (follows camera, no collision/shadows)
    if let Some(skydome_asset) = &zone.skydome {
        let scale = zone.skydome_scale.unwrap_or(1.0);
        commands.spawn((
            Skydome,
            SceneRoot(assets.load_with_settings(
                format!("{}#Scene0", skydome_asset),
                |s: &mut GltfLoaderSettings| {
                    s.load_cameras = false;
                    s.load_lights = false;
                    s.load_animations = false;
                },
            )),
            Transform::from_scale(Vec3::splat(scale)),
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
            SceneRoot(assets.load_with_settings(
                format!("{}#Scene0", prop.asset),
                |s: &mut GltfLoaderSettings| {
                    s.load_cameras = false;
                    s.load_lights = false;
                    s.load_animations = false;
                },
            )),
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

pub fn skydome_follow_camera(
    q_camera: Query<&Transform, With<ThirdPersonCamera>>,
    mut q_skydome: Query<&mut Transform, (With<Skydome>, Without<ThirdPersonCamera>)>,
) {
    let Ok(camera_transform) = q_camera.single() else {
        return;
    };
    let Ok(mut skydome_transform) = q_skydome.single_mut() else {
        return;
    };
    skydome_transform.translation = camera_transform.translation;
}

/// Marker indicating the skydome's mesh children have been configured.
#[derive(Component)]
pub(super) struct SkydomeConfigured;

/// Configures skydome mesh descendants: disables shadow casting and sets materials to unlit.
///
/// Runs every frame until the scene children are spawned, then marks the entity
/// as configured to stop re-processing.
pub fn configure_skydome(
    mut commands: Commands,
    skydome_query: Query<(Entity, &Children), (With<Skydome>, Without<SkydomeConfigured>)>,
    all_children: Query<&Children>,
    mesh_query: Query<Entity, With<Mesh3d>>,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (skydome_entity, children) in &skydome_query {
        let mut found_meshes = false;

        // BFS over all descendants
        let mut to_visit: Vec<Entity> = children.iter().collect();
        while let Some(entity) = to_visit.pop() {
            if let Ok(grandchildren) = all_children.get(entity) {
                to_visit.extend(grandchildren.iter());
            }
            if mesh_query.contains(entity) {
                found_meshes = true;
                commands.entity(entity).insert(NotShadowCaster);

                if let Ok(mat_handle) = material_query.get(entity) {
                    if let Some(mat) = materials.get_mut(&mat_handle.0) {
                        mat.unlit = true;
                    }
                }
            }
        }

        if found_meshes {
            commands.entity(skydome_entity).insert(SkydomeConfigured);
        }
    }
}

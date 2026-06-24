use avian3d::prelude::*;
use bevy::{asset::RenderAssetUsages, gltf::GltfLoaderSettings, prelude::*};
use game_core::lod;
use game_core::zone::{CollisionType, ZoneDef, ZoneDefHandle};

pub fn load_zone(commands: &mut Commands, assets: &AssetServer) {
    let handle = assets.load::<ZoneDef>("world/zones/meadow.zone.ron");
    commands.insert_resource(ZoneDefHandle(handle));
}

#[derive(Component)]
pub struct ZoneTerrain;

#[derive(Component)]
pub struct ZoneProp;

pub fn spawn_zone_when_ready(
    mut commands: Commands,
    zone_handle: Option<Res<ZoneDefHandle>>,
    zone_assets: Res<Assets<ZoneDef>>,
    assets: Res<AssetServer>,
    terrain_query: Query<&ZoneTerrain>,
) {
    let Some(handle) = zone_handle else { return };

    if !terrain_query.is_empty() {
        return;
    }

    let Some(zone) = zone_assets.get(&handle.0) else {
        return;
    };

    // Spawn terrain without materials, with trimesh collision
    if !zone.terrain.is_empty() {
        commands.spawn((
            ZoneTerrain,
            SceneRoot(
                assets.load_with_settings(
                    format!("{}#Scene0", zone.terrain),
                    |s: &mut GltfLoaderSettings| {
                        s.load_materials = RenderAssetUsages::empty();
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

    // Spawn props that have collision (skip decorative props entirely)
    for prop in &zone.props {
        let collider_constructor = match prop.collision {
            CollisionType::None => continue,
            CollisionType::ConvexHull => ColliderConstructor::ConvexHullFromMesh,
            CollisionType::TrimeshFromMesh => ColliderConstructor::TrimeshFromMesh,
        };

        commands.spawn((
            ZoneProp,
            SceneRoot(
                assets.load_with_settings(
                    format!("{}#Scene0", prop.asset),
                    |s: &mut GltfLoaderSettings| {
                        s.load_materials = RenderAssetUsages::empty();
                        s.load_cameras = false;
                        s.load_lights = false;
                        s.load_animations = false;
                    },
                ),
            ),
            prop.transform(),
            RigidBody::Static,
            ColliderConstructorHierarchy::new(collider_constructor),
        ));
    }

    tracing::info!(
        zone_id = %zone.id,
        prop_count = zone.props.iter().filter(|p| p.collision != CollisionType::None).count(),
        "zone collision loaded"
    );
}

/// Marker to avoid re-processing entities.
#[derive(Component)]
pub struct LodProcessed;

/// Despawns non-LOD0 node entities on the server.
///
/// The server only needs the highest-detail mesh (LOD0) for collision.
/// Lower LOD levels would create inaccurate colliders and waste memory.
/// Despawning the named node also removes its mesh children.
pub fn despawn_non_lod0(
    mut commands: Commands,
    query: Query<(Entity, &Name), Without<LodProcessed>>,
) {
    for (entity, name) in &query {
        let Some(level) = lod::parse_lod_level(name.as_str()) else {
            continue;
        };

        if level > 0 {
            commands.entity(entity).despawn();
        } else {
            commands.entity(entity).insert(LodProcessed);
        }
    }
}

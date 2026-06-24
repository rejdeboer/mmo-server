use super::components::*;
use avian3d::prelude::*;
use bevy::prelude::*;
use game_core::{
    character_controller::CharacterVelocityY,
    collision::GameLayer,
    components::{LevelComponent, MovementSpeedComponent, Vitals},
    constants::{ACTOR_COLLIDER_LENGTH, ACTOR_COLLIDER_RADIUS, BASE_MOVEMENT_SPEED},
    networking::NetworkId,
};
use std::sync::Arc;

#[derive(Bundle)]
/// Base components used by entities that interact with the world, like players, monsters, NPCs
pub struct ActorBundle {
    pub network_id: NetworkId,
    pub name: NameComponent,
    pub transform: Transform,
    pub vitals: Vitals,
    pub movement_speed: MovementSpeedComponent,
    pub level: LevelComponent,
    pub interested_clients: InterestedClients,
    pub body: RigidBody,
    pub collider: Collider,
    pub collision_layers: CollisionLayers,
    pub shape_caster: ShapeCaster,
    pub locked_axes: LockedAxes,
    pub velocity_y: CharacterVelocityY,
}

impl ActorBundle {
    pub fn new(
        network_id: NetworkId,
        name: &str,
        transform: Transform,
        vitals: Vitals,
        level: i32,
    ) -> Self {
        Self {
            network_id,
            name: NameComponent(Arc::from(name)),
            transform,
            vitals: vitals.clone(),
            movement_speed: MovementSpeedComponent(BASE_MOVEMENT_SPEED),
            level: LevelComponent(level),
            interested_clients: InterestedClients::default(),
            body: RigidBody::Kinematic,
            collider: Collider::capsule(ACTOR_COLLIDER_RADIUS, ACTOR_COLLIDER_LENGTH),
            collision_layers: CollisionLayers::new(
                GameLayer::Player,
                [GameLayer::Default, GameLayer::Ground],
            ),
            shape_caster: ShapeCaster::new(
                Collider::capsule(ACTOR_COLLIDER_RADIUS - 0.1, 0.1),
                Vec3::ZERO,
                Quat::IDENTITY,
                Dir3::NEG_Y,
            )
            .with_query_filter(SpatialQueryFilter::from_mask(LayerMask::ALL)),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            velocity_y: CharacterVelocityY::default(),
        }
    }
}

#[derive(Bundle)]
pub struct CharacterBundle {
    pub base: ActorBundle,
    pub id: CharacterIdComponent,
    pub client_id: ClientIdComponent,
    pub visible_entities: VisibleEntities,
    pub last_client_tick: LastClientTick,
}

impl CharacterBundle {
    pub fn new(base: ActorBundle, id: i32, client_id: u64) -> Self {
        Self {
            base,
            id: CharacterIdComponent(id),
            client_id: ClientIdComponent(client_id),
            visible_entities: VisibleEntities::default(),
            last_client_tick: LastClientTick::default(),
        }
    }
}

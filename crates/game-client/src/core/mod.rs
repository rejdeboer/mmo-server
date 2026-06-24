use bevy::prelude::*;
use game_core::{
    collision::GameLayer,
    components::{LevelComponent, MovementSpeedComponent, Vitals},
    constants::{ACTOR_COLLIDER_LENGTH, ACTOR_COLLIDER_RADIUS, BASE_MOVEMENT_SPEED},
};
use avian3d::prelude::*;

#[derive(Component)]
pub struct PlayerComponent;

#[derive(Component)]
pub struct NameComponent(pub String);

#[derive(Bundle)]
pub struct ActorBundle {
    name: NameComponent,
    transform: Transform,
    vitals: Vitals,
    movement_speed: MovementSpeedComponent,
    level: LevelComponent,
    body: RigidBody,
    collider: Collider,
    collision_layers: CollisionLayers,
}

impl ActorBundle {
    pub fn new(name: &str, transform: Transform, vitals: Vitals, level: i32) -> Self {
        Self {
            name: NameComponent(name.to_string()),
            transform,
            vitals,
            movement_speed: MovementSpeedComponent(BASE_MOVEMENT_SPEED),
            level: LevelComponent(level),
            body: RigidBody::Kinematic,
            collider: Collider::capsule(ACTOR_COLLIDER_RADIUS, ACTOR_COLLIDER_LENGTH),
            collision_layers: CollisionLayers::new(
                GameLayer::Player,
                [GameLayer::Default, GameLayer::Ground],
            ),
        }
    }
}

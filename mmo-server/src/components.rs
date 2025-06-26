use bevy::prelude::*;
use bevy_renet::renet::ClientId;

#[derive(Debug, Component)]
pub struct ClientIdComponent(pub ClientId);

#[derive(Debug, Component)]
pub struct CharacterIdComponent(pub i32);

#[derive(Debug, Component)]
pub struct EntityId(pub u32);

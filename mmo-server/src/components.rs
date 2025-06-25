use bevy::prelude::*;

#[derive(Debug, Component)]
pub struct ClientIdComponent(pub ClientId);

#[derive(Debug, Component)]
pub struct CharacterIdComponent(pub i32);

#[derive(Debug, Component)]
pub struct EntityIdComponent(pub u32);

use bevy::prelude::*;
use bevy_enhanced_input::prelude::InputAction;

#[derive(Component)]
pub struct Chatting;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Movement;

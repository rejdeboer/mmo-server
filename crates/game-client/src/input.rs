use bevy::prelude::*;
use bevy_enhanced_input::prelude::InputAction;

/// Input context marker for the chatting state.
/// When active on the player entity, it replaces the `PlayerComponent` context
/// so that WASD no longer triggers movement. The `SendChat` and `CancelChat`
/// actions are bound here via the `actions!` macro at spawn time.
#[derive(Component)]
pub struct Chatting;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Movement;

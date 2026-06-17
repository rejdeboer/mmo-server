use bevy::prelude::*;
use protocol::client::MoveAction;

#[derive(Message, Debug)]
pub struct MoveActionMessage {
    pub entity: Entity,
    pub action: MoveAction,
}

#[derive(Message, Debug)]
pub struct JumpActionMessage {
    pub entity: Entity,
}

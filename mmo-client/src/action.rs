use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::Vec3;

pub enum PlayerAction {
    Move(Vec3, f32),
}

impl PlayerAction {
    pub fn encode<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<schemas::mmo::Action<'a>> {
        match self {
            Self::Move(pos, yaw) => {
                let transform = schemas::mmo::Transform::new(
                    &schemas::mmo::Vec3::new(pos.x, pos.y, pos.z),
                    *yaw,
                );
                let action_data = schemas::mmo::PlayerMoveAction::create(
                    builder,
                    &schemas::mmo::PlayerMoveActionArgs {
                        transform: Some(&transform),
                    },
                );
                schemas::mmo::Action::create(
                    builder,
                    &schemas::mmo::ActionArgs {
                        data_type: schemas::mmo::ActionData::PlayerMoveAction,
                        data: Some(action_data.as_union_value()),
                    },
                )
            }
        }
    }
}

use flatbuffers::{FlatBufferBuilder, WIPOffset};
use schemas::mmo::ChannelType;

use crate::Vec3;

// NOTE: We handle move actions separately, since they can be sent unreliably
pub struct MoveAction {
    pub pos: Vec3,
    pub yaw: f32,
}

impl MoveAction {
    pub fn encode<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<schemas::mmo::Action<'a>> {
        let transform = schemas::mmo::Transform::new(
            &schemas::mmo::Vec3::new(self.pos.x, self.pos.y, self.pos.z),
            self.yaw,
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

#[derive(Debug)]
pub enum PlayerAction {
    Chat(ChannelType, String),
    Jump,
}

impl PlayerAction {
    pub fn encode<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<schemas::mmo::Action<'a>> {
        let data_type;
        let data = match self {
            Self::Chat(channel, msg) => {
                data_type = schemas::mmo::ActionData::mmo_ClientChatMessage;
                let fb_msg = builder.create_string(msg);
                schemas::mmo::ClientChatMessage::create(
                    builder,
                    &schemas::mmo::ClientChatMessageArgs {
                        channel: *channel,
                        recipient_id: 0,
                        text: Some(fb_msg),
                    },
                )
                .as_union_value()
            }
            Self::Jump => {
                data_type = schemas::mmo::ActionData::PlayerJumpAction;
                schemas::mmo::PlayerJumpAction::create(
                    builder,
                    &schemas::mmo::PlayerJumpActionArgs {},
                )
                .as_union_value()
            }
        };

        schemas::mmo::Action::create(
            builder,
            &schemas::mmo::ActionArgs {
                data_type,
                data: Some(data),
            },
        )
    }
}

use flatbuffers::{FlatBufferBuilder, WIPOffset};
use schema::ChannelType;
use schemas::game as schema;

use crate::Vec3;

// NOTE: We handle move actions separately, since they can be sent unreliably
pub struct MoveAction {
    pub pos: Vec3,
    pub yaw: f32,
}

impl MoveAction {
    pub fn encode<'a>(&self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Action<'a>> {
        let transform = schema::Transform::new(
            &schema::Vec3::new(self.pos.x, self.pos.y, self.pos.z),
            self.yaw,
        );
        let action_data = schema::PlayerMoveAction::create(
            builder,
            &schema::PlayerMoveActionArgs {
                transform: Some(&transform),
            },
        );
        schema::Action::create(
            builder,
            &schema::ActionArgs {
                data_type: schema::ActionData::PlayerMoveAction,
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
    pub fn encode<'a>(&self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Action<'a>> {
        let data_type;
        let data = match self {
            Self::Chat(channel, msg) => {
                data_type = schema::ActionData::game_ClientChatMessage;
                let fb_msg = builder.create_string(msg);
                schema::ClientChatMessage::create(
                    builder,
                    &schema::ClientChatMessageArgs {
                        channel: *channel,
                        text: Some(fb_msg),
                    },
                )
                .as_union_value()
            }
            Self::Jump => {
                data_type = schema::ActionData::PlayerJumpAction;
                schema::PlayerJumpAction::create(builder, &schema::PlayerJumpActionArgs {})
                    .as_union_value()
            }
        };

        schema::Action::create(
            builder,
            &schema::ActionArgs {
                data_type,
                data: Some(data),
            },
        )
    }
}

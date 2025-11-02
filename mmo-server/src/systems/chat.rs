use bevy::prelude::*;
use schemas::game::ChannelType;

use crate::{
    components::{ClientIdComponent, NameComponent, VisibleEntities},
    messages::{IncomingChatMessage, OutgoingMessage, OutgoingMessageData},
};

const MAX_SAY_DISTANCE: f32 = 32.0;

pub fn process_incoming_chat(
    mut chat_reader: MessageReader<IncomingChatMessage>,
    mut writer: MessageWriter<OutgoingMessage>,
    q_authors: Query<(
        &ClientIdComponent,
        &NameComponent,
        &Transform,
        &VisibleEntities,
    )>,
    q_recipients: Query<(&ClientIdComponent, &Transform)>,
) {
    for msg in chat_reader.read() {
        let Ok((author_id, name, author_transform, visible)) = q_authors.get(msg.author) else {
            error!(entity=?msg.author, "chat author does not exist");
            continue;
        };

        writer.write(OutgoingMessage::new(
            author_id.0,
            OutgoingMessageData::ChatMessage(msg.channel, name.clone(), msg.text.clone()),
        ));

        for entity in visible.entities.iter() {
            let Ok((recipient_id, recipient_transform)) = q_recipients.get(*entity) else {
                continue;
            };

            if msg.channel == ChannelType::Say
                && author_transform
                    .translation
                    .distance(recipient_transform.translation)
                    > MAX_SAY_DISTANCE
            {
                continue;
            }

            // TODO: Do we have to clone text here? Probably should use Arc
            writer.write(OutgoingMessage::new(
                recipient_id.0,
                OutgoingMessageData::ChatMessage(msg.channel, name.clone(), msg.text.clone()),
            ));
        }
    }
}

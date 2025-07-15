use bevy::prelude::*;
use schemas::mmo::ChannelType;

use crate::{
    components::{ClientIdComponent, NameComponent, VisibleEntities},
    events::{IncomingChatMessage, OutgoingMessage, OutgoingMessageData},
};

const MAX_SAY_DISTANCE: f32 = 32.0;

pub fn process_incoming_chat(
    mut chat_reader: EventReader<IncomingChatMessage>,
    mut writer: EventWriter<OutgoingMessage>,
    q_authors: Query<(&NameComponent, &Transform, &VisibleEntities)>,
    q_recipients: Query<(&ClientIdComponent, &Transform)>,
) {
    for event in chat_reader.read() {
        let Ok((name, author_transform, visible)) = q_authors.get(event.author) else {
            error!(entity=?event.author, "chat author does not exist");
            continue;
        };

        for entity in visible.entities.iter() {
            let Ok((recipient_id, recipient_transform)) = q_recipients.get(*entity) else {
                continue;
            };

            if event.channel == ChannelType::Say
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
                OutgoingMessageData::ChatMessage(event.channel, name.clone(), event.text.clone()),
            ));
        }
    }
}

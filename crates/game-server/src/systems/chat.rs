use crate::{
    components::{ClientIdComponent, NameComponent, VisibleEntities},
    messages::{IncomingChatMessage, OutgoingMessage, OutgoingMessageData},
};
use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use protocol::models::ChatChannel;

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

        let mut recipients: Vec<ClientId> = vec![author_id.0];
        for entity in visible.entities.iter() {
            let Ok((recipient_id, recipient_transform)) = q_recipients.get(*entity) else {
                continue;
            };

            if msg.channel == ChatChannel::Say
                && author_transform
                    .translation
                    .distance(recipient_transform.translation)
                    > MAX_SAY_DISTANCE
            {
                continue;
            }

            recipients.push(recipient_id.0);
        }

        writer.write(OutgoingMessage::new(
            recipients,
            OutgoingMessageData::ChatMessage {
                channel: msg.channel.clone(),
                sender_name: name.0.to_string(),
                text: msg.text.clone(),
            },
        ));
    }
}

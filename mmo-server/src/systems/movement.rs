use bevy::prelude::*;

use crate::{
    components::InterestedClients,
    events::{OutgoingMessage, OutgoingMessageData},
};

pub fn send_transform_updates(
    mut writer: EventWriter<OutgoingMessage>,
    q_moved: Query<(Entity, &Transform, &InterestedClients), Changed<Transform>>,
) {
    q_moved.iter().for_each(|(entity, transform, interested)| {
        for client_id in interested.clients.iter() {
            writer.write(OutgoingMessage::new(
                *client_id,
                OutgoingMessageData::Movement(entity, transform.clone()),
            ));
        }
    })
}

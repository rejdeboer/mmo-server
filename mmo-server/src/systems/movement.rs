use bevy::prelude::*;

use crate::{
    components::{ClientIdComponent, InterestedClients},
    events::{OutgoingMessage, OutgoingMessageData},
};

pub fn send_transform_updates(
    mut writer: EventWriter<OutgoingMessage>,
    q_moved: Query<
        (
            Entity,
            &Transform,
            &InterestedClients,
            Option<&ClientIdComponent>,
        ),
        Changed<Transform>,
    >,
) {
    q_moved
        .iter()
        .for_each(|(entity, transform, interested, client_id_option)| {
            if let Some(client_id) = client_id_option {
                writer.write(OutgoingMessage::new(
                    client_id.0,
                    OutgoingMessageData::Movement(entity, *transform),
                ));
            }

            for client_id in interested.clients.iter() {
                writer.write(OutgoingMessage::new(
                    *client_id,
                    OutgoingMessageData::Movement(entity, *transform),
                ));
            }
        })
}

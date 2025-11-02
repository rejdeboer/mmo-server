use crate::{
    components::{ClientIdComponent, InterestedClients, MovementSpeedComponent},
    messages::{MoveActionMessage, OutgoingMessage, OutgoingMessageData},
};
use bevy::prelude::*;
use std::f32::consts::TAU;

const YAW_QUANTIZATION_FACTOR: f32 = 65535.0;
const MOVEMENT_QUANTIZATION_FACTOR: f32 = 127.0;

// TODO: Validate movement
// TODO: Parallelism?
pub fn process_move_action_messages(
    mut reader: MessageReader<MoveActionMessage>,
    mut q_transform: Query<(&mut Transform, &MovementSpeedComponent)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    reader.read().for_each(|msg| {
        let Ok((mut transform, movement_speed)) = q_transform.get_mut(msg.entity) else {
            error!(entity = ?msg.entity, "could not find transform");
            return;
        };

        let yaw = msg.yaw as f32 / YAW_QUANTIZATION_FACTOR * TAU;
        transform.rotation = Quat::from_rotation_y(yaw);

        let forward = transform.forward();
        let right = transform.right();

        // TODO: dt is caclulated incorrectly for movement, need to design another system
        let forward_movement =
            forward * (msg.forward as f32 / MOVEMENT_QUANTIZATION_FACTOR) * movement_speed.0 * dt;
        let sideways_movement =
            right * (msg.sideways as f32 / MOVEMENT_QUANTIZATION_FACTOR) * movement_speed.0 * dt;

        transform.translation += forward_movement + sideways_movement;
    })
}

pub fn send_transform_updates(
    mut writer: MessageWriter<OutgoingMessage>,
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

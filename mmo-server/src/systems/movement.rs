use crate::{
    components::{ClientIdComponent, InterestedClients, MovementSpeedComponent},
    messages::{MoveActionMessage, OutgoingMessage, OutgoingMessageData},
};
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use std::f32::consts::TAU;

const YAW_QUANTIZATION_FACTOR: f32 = 65535.0;
const MOVEMENT_QUANTIZATION_FACTOR: f32 = 127.0;

// TODO: Validate movement
// TODO: Parallelism?
pub fn process_move_action_messages(
    mut reader: MessageReader<MoveActionMessage>,
    mut q_transform: Query<(&mut Transform, &mut LinearVelocity, &MovementSpeedComponent)>,
) {
    reader.read().for_each(|msg| {
        let Ok((mut transform, mut velocity, movement_speed)) = q_transform.get_mut(msg.entity)
        else {
            error!(entity = ?msg.entity, "could not find entity");
            return;
        };

        let yaw = msg.yaw as f32 / YAW_QUANTIZATION_FACTOR * TAU;
        transform.rotation = Quat::from_rotation_y(yaw);

        let forward = transform.forward();
        let right = transform.right();

        let forward_input = msg.forward as f32 / MOVEMENT_QUANTIZATION_FACTOR;
        let sideways_input = msg.sideways as f32 / MOVEMENT_QUANTIZATION_FACTOR;

        let direction = (forward * forward_input) + (right * sideways_input);
        let target_velocity = direction.normalize_or_zero() * movement_speed.0;
        velocity.x = target_velocity.x;
        velocity.z = target_velocity.z;
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

use flatbuffers::{FlatBufferBuilder, WIPOffset};
use std::f32::consts::TAU;

// NOTE: We handle move actions separately, since they can be sent unreliably
pub struct MoveAction {
    pub yaw: f32,
    pub forward: f32,
    pub sideways: f32,
}

impl MoveAction {
    // TODO: These constants should be defined once in a separate crate
    const YAW_QUANTIZATION_FACTOR: f32 = 65535.0;
    const MOVEMENT_QUANTIZATION_FACTOR: f32 = 127.0;

    pub fn encode<'a>(&self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Action<'a>> {
        let forward =
            (self.forward.clamp(-1.0, 1.0) * Self::MOVEMENT_QUANTIZATION_FACTOR).round() as i8;
        let sideways =
            (self.sideways.clamp(-1.0, 1.0) * Self::MOVEMENT_QUANTIZATION_FACTOR).round() as i8;

        let normalized_yaw = self.yaw.rem_euclid(TAU) / TAU;
        let yaw = (normalized_yaw * Self::YAW_QUANTIZATION_FACTOR).round() as u16;

        let action_data = schema::PlayerMoveAction::create(
            builder,
            &schema::PlayerMoveActionArgs {
                yaw,
                forward,
                sideways,
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

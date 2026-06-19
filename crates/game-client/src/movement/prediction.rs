use crate::core::PlayerComponent;
use crate::world::camera::ThirdPersonCamera;
use crate::input::Movement;
use crate::networking::TickSync;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use game_core::character_controller::{self, CharacterVelocityY, FIXED_DT_SECS};
use game_core::components::{GroundedComponent, MovementSpeedComponent};
use game_core::movement::MoveInput;
use protocol::client::MoveAction;
use std::collections::VecDeque;

const VISUAL_ROTATION_SPEED: f32 = 12.0;

/// Snapshot of a predicted position + rotation at a given tick.
#[derive(Debug, Clone)]
pub struct PredictedState {
    pub position: Vec3,
    pub yaw: f32,
    pub velocity_y: f32,
    pub grounded: bool,
}

/// Input and state history for client-side prediction and server reconciliation.
#[derive(Component, Debug)]
pub struct PredictionHistory {
    pub(crate) input_buffer: VecDeque<MoveAction>,
    state_buffer: VecDeque<PredictedState>,
    max_buffer_size: usize,
}

impl Default for PredictionHistory {
    fn default() -> Self {
        Self {
            input_buffer: VecDeque::new(),
            state_buffer: VecDeque::new(),
            max_buffer_size: 256,
        }
    }
}

impl PredictionHistory {
    pub fn push_input(&mut self, input: MoveAction) {
        self.input_buffer.push_back(input);

        while self.input_buffer.len() > self.max_buffer_size {
            self.input_buffer.pop_front();
            if !self.state_buffer.is_empty() {
                self.state_buffer.pop_front();
            }
        }
    }

    pub fn push_state(&mut self, state: PredictedState) {
        self.state_buffer.push_back(state);
    }

    pub fn acknowledge_up_to(&mut self, tick: u32) -> Option<PredictedState> {
        let mut confirmed_state = None;

        while let Some(front) = self.input_buffer.front() {
            if front.tick <= tick {
                self.input_buffer.pop_front();
                confirmed_state = self.state_buffer.pop_front();
            } else {
                break;
            }
        }

        confirmed_state
    }

    pub fn unacknowledged_inputs(&self) -> impl Iterator<Item = &MoveAction> {
        self.input_buffer.iter()
    }

    pub fn clear_states(&mut self) {
        self.state_buffer.clear();
    }

    pub(crate) fn latest_has_movement(&self) -> bool {
        self.input_buffer.back().is_some_and(|a| a.has_movement())
    }

    pub(crate) fn previous_had_movement(&self) -> bool {
        self.input_buffer
            .iter()
            .rev()
            .nth(1)
            .is_some_and(|a| a.has_movement())
    }
}

pub fn predict_player_movement(
    tick_sync: Res<TickSync>,
    movement_action: Single<(&Action<Movement>, &TriggerState), With<Action<Movement>>>,
    spatial_query: SpatialQuery,
    q_camera: Query<&ThirdPersonCamera>,
    mut q_player: Query<
        (
            Entity,
            &mut Transform,
            &Collider,
            &MovementSpeedComponent,
            &mut CharacterVelocityY,
            &mut PredictionHistory,
        ),
        With<PlayerComponent>,
    >,
    mut commands: Commands,
) {
    let current_tick = tick_sync.tick;

    let Ok((entity, mut transform, collider, speed, mut vel_y, mut history)) =
        q_player.single_mut()
    else {
        return;
    };

    let (action_value, trigger_state) = *movement_action;
    let mut movement_value = match trigger_state {
        TriggerState::Fired => **action_value,
        _ => Vec2::ZERO,
    };

    let cam = q_camera.single().ok();
    if cam.is_some_and(|c| c.both_buttons_move) {
        movement_value.y = movement_value.y.max(1.0);
    }

    let yaw_rad = cam
        .map(|c| c.yaw)
        .unwrap_or_else(|| transform.rotation.to_euler(EulerRot::YXZ).1);
    let action = MoveAction::from_f32(yaw_rad, movement_value.y, movement_value.x, current_tick);

    let move_input = MoveInput::from(action.clone());

    let result = character_controller::character_move_step(
        transform.translation,
        vel_y.0,
        &move_input,
        speed.0,
        collider,
        entity,
        &spatial_query,
    );

    transform.translation = result.position;

    let turn_character = cam.is_some_and(|c| c.turn_character);
    let is_moving = move_input.forward.abs() > 0.001 || move_input.sideways.abs() > 0.001;

    if turn_character {
        transform.rotation = Quat::from_rotation_y(yaw_rad);
    } else if is_moving {
        let target_rot = Quat::from_rotation_y(result.yaw);
        let t = (VISUAL_ROTATION_SPEED * FIXED_DT_SECS).min(1.0);
        transform.rotation = transform.rotation.slerp(target_rot, t);
    }

    vel_y.0 = result.velocity_y;

    if result.grounded {
        commands.entity(entity).insert(GroundedComponent);
    } else {
        commands.entity(entity).remove::<GroundedComponent>();
    }

    history.push_input(action);
    history.push_state(PredictedState {
        position: result.position,
        yaw: result.yaw,
        velocity_y: result.velocity_y,
        grounded: result.grounded,
    });
}

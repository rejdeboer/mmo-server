use super::components::{AiBrain, AiMovement, AiState, LeashAnchor, Wander, WanderState};
use bevy::prelude::*;
use rand::Rng;

/// Drives wander behavior for idle mobs. When pausing, ticks the timer and picks
/// a new random target when it expires. When walking, checks arrival and starts
/// a new pause.
pub fn wander(
    time: Res<Time>,
    mut q_mobs: Query<(&AiBrain, &LeashAnchor, &mut Wander, &mut AiMovement, &Transform)>,
) {
    let mut rng = rand::thread_rng();

    for (brain, leash, mut wander, mut movement, transform) in q_mobs.iter_mut() {
        // Only wander when idle
        if brain.state != AiState::Idle {
            // Reset wander state when not idle so it restarts cleanly
            if matches!(wander.state, WanderState::Walking) {
                wander.state = WanderState::Pausing {
                    timer: Timer::from_seconds(wander.pause_duration, TimerMode::Once),
                };
            }
            continue;
        }

        match &mut wander.state {
            WanderState::Pausing { timer } => {
                timer.tick(time.delta());
                if timer.is_finished() {
                    // Pick a random point within wander radius of the spawn anchor
                    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                    let dist = rng.gen_range(2.0..wander.radius);
                    let offset = Vec3::new(angle.cos() * dist, 0.0, angle.sin() * dist);
                    let target = leash.position + offset;

                    movement.target_position = Some(target);
                    movement.stop_distance = 1.0;
                    wander.state = WanderState::Walking;
                }
            }
            WanderState::Walking => {
                // Check if we've arrived (or close enough)
                let Some(target) = movement.target_position else {
                    // Movement was cleared externally, start pausing
                    wander.state = WanderState::Pausing {
                        timer: Timer::from_seconds(wander.pause_duration, TimerMode::Once),
                    };
                    continue;
                };

                let horizontal_dist = Vec3::new(
                    target.x - transform.translation.x,
                    0.0,
                    target.z - transform.translation.z,
                )
                .length();

                if horizontal_dist <= movement.stop_distance {
                    // Arrived — pause before next wander
                    movement.target_position = None;
                    let pause_time = rng.gen_range(2.0..wander.pause_duration * 2.0);
                    wander.state = WanderState::Pausing {
                        timer: Timer::from_seconds(pause_time, TimerMode::Once),
                    };
                }
            }
        }
    }
}

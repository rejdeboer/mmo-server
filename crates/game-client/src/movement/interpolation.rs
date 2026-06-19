use crate::core::PlayerComponent;
use bevy::prelude::*;
use game_core::character_controller::FIXED_DT_SECS;
use std::collections::VecDeque;
use std::f32::consts::TAU;

const REMOTE_SNAPSHOT_BUFFER_SIZE: usize = 8;

#[derive(Debug, Clone)]
struct RemoteSnapshot {
    position: Vec3,
    yaw: f32,
    timestamp: f64,
}

/// Interpolation buffer for remote (non-local) actors.
#[derive(Component, Debug)]
pub struct RemoteInterpolation {
    snapshots: VecDeque<RemoteSnapshot>,
}

impl Default for RemoteInterpolation {
    fn default() -> Self {
        Self {
            snapshots: VecDeque::with_capacity(REMOTE_SNAPSHOT_BUFFER_SIZE),
        }
    }
}

impl RemoteInterpolation {
    pub fn push(&mut self, position: Vec3, yaw: f32, timestamp: f64) {
        self.snapshots.push_back(RemoteSnapshot {
            position,
            yaw,
            timestamp,
        });

        while self.snapshots.len() > REMOTE_SNAPSHOT_BUFFER_SIZE {
            self.snapshots.pop_front();
        }
    }

    pub fn sample(&self, render_time: f64) -> Option<(Vec3, f32)> {
        if self.snapshots.len() < 2 {
            return self.snapshots.back().map(|s| (s.position, s.yaw));
        }

        for i in 0..self.snapshots.len() - 1 {
            let from = &self.snapshots[i];
            let to = &self.snapshots[i + 1];

            if render_time >= from.timestamp && render_time <= to.timestamp {
                let duration = to.timestamp - from.timestamp;
                if duration < f64::EPSILON {
                    return Some((to.position, to.yaw));
                }
                let t = ((render_time - from.timestamp) / duration) as f32;
                let t = t.clamp(0.0, 1.0);
                let pos = from.position.lerp(to.position, t);
                let yaw = lerp_angle(from.yaw, to.yaw, t);
                return Some((pos, yaw));
            }
        }

        let from = &self.snapshots[self.snapshots.len() - 2];
        let to = &self.snapshots[self.snapshots.len() - 1];
        let duration = to.timestamp - from.timestamp;
        if duration < f64::EPSILON {
            return Some((to.position, to.yaw));
        }
        let t = ((render_time - from.timestamp) / duration) as f32;
        let t = t.clamp(0.0, 2.0);
        let pos = from.position.lerp(to.position, t);
        let yaw = lerp_angle(from.yaw, to.yaw, t);
        Some((pos, yaw))
    }
}

fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let mut diff = (to - from) % TAU;
    if diff > std::f32::consts::PI {
        diff -= TAU;
    } else if diff < -std::f32::consts::PI {
        diff += TAU;
    }
    from + diff * t
}

pub fn interpolate_remote_actors(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &RemoteInterpolation), Without<PlayerComponent>>,
) {
    let render_time = time.elapsed_secs_f64() - FIXED_DT_SECS as f64;

    for (mut transform, remote_interp) in query.iter_mut() {
        if let Some((pos, yaw)) = remote_interp.sample(render_time) {
            transform.translation = pos;
            transform.rotation = Quat::from_rotation_y(yaw);
        }
    }
}

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AiBehavior {
    /// Only fights back when damaged
    Neutral,
    /// Attacks players on proximity
    #[default]
    Aggressive,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum AiState {
    #[default]
    Idle,
    Chase {
        target: Entity,
    },
    Combat {
        target: Entity,
    },
    Returning,
    Evading,
}

#[derive(Component, Debug, Default)]
pub struct AiBrain {
    pub state: AiState,
    pub behavior: AiBehavior,
}

#[derive(Debug, Clone)]
pub struct ThreatEntry {
    pub entity: Entity,
    pub threat: f32,
}

#[derive(Component, Debug, Default)]
pub struct ThreatTable {
    pub entries: Vec<ThreatEntry>,
}

impl ThreatTable {
    pub fn add_threat(&mut self, entity: Entity, amount: f32) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.entity == entity) {
            entry.threat += amount;
        } else {
            self.entries.push(ThreatEntry {
                entity,
                threat: amount,
            });
        }
    }

    pub fn highest_threat(&self) -> Option<&ThreatEntry> {
        self.entries.iter().max_by(|a, b| {
            a.threat
                .partial_cmp(&b.threat)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        self.entries.retain(|e| e.entity != entity);
    }
}

#[derive(Component, Debug)]
pub struct AggroRadius(pub f32);

#[derive(Component, Debug)]
pub struct LeashAnchor {
    pub position: Vec3,
    pub max_range: f32,
}

/// Maps spell_id -> priority for AI ability selection.
/// Higher priority abilities are preferred when available.
#[derive(Component, Debug, Default)]
pub struct AiAbilityConfig {
    pub priorities: HashMap<u32, u8>,
}

/// Tracks the AI's desired movement target and stop distance.
#[derive(Component, Debug)]
pub struct AiMovement {
    pub target_position: Option<Vec3>,
    pub stop_distance: f32,
}

impl Default for AiMovement {
    fn default() -> Self {
        Self {
            target_position: None,
            stop_distance: 2.5,
        }
    }
}

/// Wander behavior for idle mobs. Picks random points near the spawn
/// location, walks to them, pauses, and repeats.
#[derive(Component, Debug)]
pub struct Wander {
    pub radius: f32,
    pub pause_duration: f32,
    pub state: WanderState,
}

#[derive(Debug)]
pub enum WanderState {
    /// Standing still, waiting for the pause timer to expire
    Pausing { timer: Timer },
    /// Walking toward a chosen point
    Walking,
}

impl Wander {
    pub fn new(radius: f32, pause_duration: f32) -> Self {
        Self {
            radius,
            pause_duration,
            state: WanderState::Pausing {
                timer: Timer::from_seconds(pause_duration, TimerMode::Once),
            },
        }
    }
}

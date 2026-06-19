use avian3d::prelude::*;
use bevy::prelude::*;
use game_core::components::Vitals;
use game_core::networking::{NetworkId, NetworkIdMapping};

use super::DebugActorMesh;
use crate::core::ActorBundle;
use crate::movement::RemoteInterpolation;
use crate::networking::{ActorDespawnMessage, ActorSpawnMessage};

pub fn handle_actor_spawn_messages(
    mut reader: MessageReader<ActorSpawnMessage>,
    mut network_id_mapping: ResMut<NetworkIdMapping>,
    debug_mesh: Res<DebugActorMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for message in reader.read() {
        let actor = &message.0;
        let transform = Transform::from_translation(actor.transform.position)
            .with_rotation(actor.transform.get_quat());

        let base_color = match &actor.attributes {
            protocol::models::ActorAttributes::Player { .. } => Color::srgb(0.2, 0.4, 0.8),
            protocol::models::ActorAttributes::Npc { .. } => Color::srgb(0.7, 0.2, 0.2),
        };
        let remote_material = materials.add(StandardMaterial {
            base_color,
            ..default()
        });

        let entity = commands.spawn((
            RemoteInterpolation::default(),
            NoTransformEasing,
            NetworkId(actor.id),
            ActorBundle::new(
                &actor.name,
                transform,
                Vitals::from(actor.vitals.clone()),
                actor.level as i32,
            ),
            Mesh3d(debug_mesh.0.clone()),
            MeshMaterial3d(remote_material),
        ));
        network_id_mapping
            .0
            .insert(NetworkId(actor.id), entity.id());
    }
}

pub fn handle_actor_despawn_messages(
    mut reader: MessageReader<ActorDespawnMessage>,
    mut network_id_mapping: ResMut<NetworkIdMapping>,
    mut commands: Commands,
) {
    for message in reader.read() {
        let Some(entity) = network_id_mapping.0.get(&message.0) else {
            tracing::debug!(network_id = ?message.0, "tried to despawn actor, but it did not exist");
            continue;
        };

        commands.entity(*entity).despawn();
        network_id_mapping.0.remove(&message.0);
    }
}

use crate::{
    application::{ActorDespawnMessage, ActorSpawnMessage, AppState, EnterGame, PlayerComponent},
    combat_feedback::CombatHitMessage,
    social::{ChatLog, ChatMessage, ChatMessageChannel},
    tick_sync::TickSync,
    ui::cast_bar::ActiveCast,
};
use bevy::{ecs::system::SystemParam, platform::collections::HashMap, prelude::*};
use bevy_renet::{RenetClient, renet::DefaultChannel};
use game_core::{
    components::{NetworkId, Vitals},
    spells::{SpellLibrary, SpellLibraryHandle},
};
use protocol::server::{EnterGameResponse, ServerEvent};

#[derive(Resource)]
pub struct NetworkIdMapping(pub HashMap<NetworkId, Entity>);

#[derive(SystemParam)]
pub struct NetworkMessageWriters<'w> {
    pub spawns: MessageWriter<'w, ActorSpawnMessage>,
    pub despawns: MessageWriter<'w, ActorDespawnMessage>,
    pub combat_hits: MessageWriter<'w, CombatHitMessage>,
}

pub fn poll_connection(
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut client: ResMut<RenetClient>,
) {
    if let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        match bitcode::decode::<EnterGameResponse>(&message) {
            Ok(response) => {
                commands.trigger(EnterGame(response));
                next_state.set(AppState::InGame);
            }
            Err(e) => {
                tracing::error!("received invalid EnterGameResponse {}", e);
                next_state.set(AppState::CharacterSelect);
            }
        }
    }
}

pub fn receive_server_events(
    mut writers: NetworkMessageWriters,
    mut client: ResMut<RenetClient>,
    mut tick_sync: ResMut<TickSync>,
    mut chat_log: ResMut<ChatLog>,
    network_id_mapping: Res<NetworkIdMapping>,
    mut q_vitals: Query<&mut Vitals>,
    q_player: Query<&PlayerComponent>,
    spell_library_handle: Res<SpellLibraryHandle>,
    spell_libraries: Res<Assets<SpellLibrary>>,
    mut commands: Commands,
) {
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        match bitcode::decode::<ServerEvent>(&message) {
            Ok(event) => match event {
                ServerEvent::ActorSpawn(actor) => {
                    writers.spawns.write(ActorSpawnMessage(*actor));
                }
                ServerEvent::ActorDespawn(id) => {
                    writers.despawns.write(ActorDespawnMessage(NetworkId(id)));
                }
                ServerEvent::SpellImpact {
                    target_id,
                    spell_id: _,
                    impact_amount,
                } => {
                    if let Some(&entity) = network_id_mapping.0.get(&NetworkId(target_id))
                        && let Ok(mut vitals) = q_vitals.get_mut(entity)
                    {
                        vitals.hp -= impact_amount;
                        writers.combat_hits.write(CombatHitMessage {
                            target_entity: entity,
                            amount: impact_amount,
                        });
                    }
                }
                ServerEvent::ActorDeath(id) => {
                    if let Some(&entity) = network_id_mapping.0.get(&NetworkId(id))
                        && let Ok(mut vitals) = q_vitals.get_mut(entity)
                    {
                        vitals.hp = 0;
                    }
                }
                ServerEvent::StartCasting {
                    actor_id,
                    spell_id,
                } => {
                    if let Some(&entity) = network_id_mapping.0.get(&NetworkId(actor_id))
                        && q_player.get(entity).is_ok()
                        && let Some(library) = spell_libraries.get(&spell_library_handle.0)
                        && let Some(spell_def) = library.spells.get(&spell_id)
                        && spell_def.casting_duration > 0.0
                    {
                        commands.insert_resource(ActiveCast {
                            spell_id,
                            spell_name: spell_def.name.clone(),
                            timer: Timer::from_seconds(
                                spell_def.casting_duration,
                                TimerMode::Once,
                            ),
                        });
                    }
                }
                ServerEvent::KillReward {
                    victim_id: _,
                    loot: _,
                } => {
                    // TODO: Show loot notification
                }
                ServerEvent::Pong {
                    client_tick,
                    server_tick,
                } => {
                    tick_sync.observe_pong(server_tick, client_tick);
                    tracing::debug!(
                        server_tick,
                        client_tick,
                        current_tick = tick_sync.tick,
                        "PONG"
                    );
                }
                ServerEvent::Chat {
                    channel,
                    sender_name,
                    text,
                } => {
                    let ch = match channel {
                        protocol::models::ChatChannel::Say => ChatMessageChannel::Say,
                        protocol::models::ChatChannel::Yell => ChatMessageChannel::Yell,
                        protocol::models::ChatChannel::Zone => ChatMessageChannel::Zone,
                    };
                    chat_log.push(ChatMessage {
                        channel: ch,
                        sender: sender_name,
                        text,
                    });
                }
            },
            Err(e) => {
                tracing::error!("received invalid ServerEvent {}", e);
            }
        }
    }
}

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

#[derive(Message)]
pub struct SpellImpactMessage {
    pub target_id: u64,
    pub spell_id: u32,
    pub impact_amount: i32,
}

#[derive(Message)]
pub struct ActorDeathMessage(pub u64);

#[derive(Message)]
pub struct StartCastingMessage {
    pub actor_id: u64,
    pub spell_id: u32,
}

#[derive(Message)]
pub struct KillRewardMessage {
    pub victim_id: u64,
    pub loot: Vec<protocol::models::ItemDrop>,
}

#[derive(Message)]
pub struct ServerChatMessage {
    pub channel: protocol::models::ChatChannel,
    pub sender_name: String,
    pub text: String,
}

#[derive(SystemParam)]
pub struct NetworkMessageWriters<'w> {
    pub spawns: MessageWriter<'w, ActorSpawnMessage>,
    pub despawns: MessageWriter<'w, ActorDespawnMessage>,
    pub spell_impacts: MessageWriter<'w, SpellImpactMessage>,
    pub deaths: MessageWriter<'w, ActorDeathMessage>,
    pub casts: MessageWriter<'w, StartCastingMessage>,
    pub kill_rewards: MessageWriter<'w, KillRewardMessage>,
    pub chats: MessageWriter<'w, ServerChatMessage>,
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
                    spell_id,
                    impact_amount,
                } => {
                    writers.spell_impacts.write(SpellImpactMessage {
                        target_id,
                        spell_id,
                        impact_amount,
                    });
                }
                ServerEvent::ActorDeath(id) => {
                    writers.deaths.write(ActorDeathMessage(id));
                }
                ServerEvent::StartCasting { actor_id, spell_id } => {
                    writers
                        .casts
                        .write(StartCastingMessage { actor_id, spell_id });
                }
                ServerEvent::KillReward { victim_id, loot } => {
                    writers
                        .kill_rewards
                        .write(KillRewardMessage { victim_id, loot });
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
                    writers.chats.write(ServerChatMessage {
                        channel,
                        sender_name,
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

pub fn handle_spell_impacts(
    mut reader: MessageReader<SpellImpactMessage>,
    mut combat_hits: MessageWriter<CombatHitMessage>,
    network_id_mapping: Res<NetworkIdMapping>,
    mut q_vitals: Query<&mut Vitals>,
) {
    for msg in reader.read() {
        if let Some(&entity) = network_id_mapping.0.get(&NetworkId(msg.target_id))
            && let Ok(mut vitals) = q_vitals.get_mut(entity)
        {
            vitals.hp -= msg.impact_amount;
            combat_hits.write(CombatHitMessage {
                target_entity: entity,
                amount: msg.impact_amount,
            });
        }
    }
}

pub fn handle_actor_deaths(
    mut reader: MessageReader<ActorDeathMessage>,
    network_id_mapping: Res<NetworkIdMapping>,
    mut q_vitals: Query<&mut Vitals>,
) {
    for msg in reader.read() {
        if let Some(&entity) = network_id_mapping.0.get(&NetworkId(msg.0))
            && let Ok(mut vitals) = q_vitals.get_mut(entity)
        {
            vitals.hp = 0;
        }
    }
}

pub fn handle_start_casting(
    mut reader: MessageReader<StartCastingMessage>,
    network_id_mapping: Res<NetworkIdMapping>,
    q_player: Query<&PlayerComponent>,
    spell_library_handle: Res<SpellLibraryHandle>,
    spell_libraries: Res<Assets<SpellLibrary>>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        if let Some(&entity) = network_id_mapping.0.get(&NetworkId(msg.actor_id))
            && q_player.get(entity).is_ok()
            && let Some(library) = spell_libraries.get(&spell_library_handle.0)
            && let Some(spell_def) = library.spells.get(&msg.spell_id)
            && spell_def.casting_duration > 0.0
        {
            commands.insert_resource(ActiveCast {
                spell_id: msg.spell_id,
                spell_name: spell_def.name.clone(),
                timer: Timer::from_seconds(spell_def.casting_duration, TimerMode::Once),
            });
        }
    }
}

pub fn handle_kill_rewards(mut reader: MessageReader<KillRewardMessage>) {
    for _msg in reader.read() {
        // TODO: Show loot notification
    }
}

pub fn handle_server_chat(
    mut reader: MessageReader<ServerChatMessage>,
    mut chat_log: ResMut<ChatLog>,
) {
    for msg in reader.read() {
        let channel = match msg.channel {
            protocol::models::ChatChannel::Say => ChatMessageChannel::Say,
            protocol::models::ChatChannel::Yell => ChatMessageChannel::Yell,
            protocol::models::ChatChannel::Zone => ChatMessageChannel::Zone,
        };
        chat_log.push(ChatMessage {
            channel,
            sender: msg.sender_name.clone(),
            text: msg.text.clone(),
        });
    }
}

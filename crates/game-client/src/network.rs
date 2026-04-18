use crate::{
    application::{ActorDespawnMessage, ActorSpawnMessage, AppState, EnterGame},
    chat::{ChatLog, ChatMessage, ChatMessageChannel},
    tick_sync::TickSync,
};
use bevy::{ecs::system::SystemParam, platform::collections::HashMap, prelude::*};
use bevy_renet::{RenetClient, renet::DefaultChannel};
use game_core::components::NetworkId;
use protocol::server::{EnterGameResponse, ServerEvent};

#[derive(Resource)]
pub struct NetworkIdMapping(pub HashMap<NetworkId, Entity>);

#[derive(SystemParam)]
pub struct NetworkMessageWriters<'w> {
    pub spawns: MessageWriter<'w, ActorSpawnMessage>,
    pub despawns: MessageWriter<'w, ActorDespawnMessage>,
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
                _ => {
                    tracing::warn!("unhandled server event");
                }
            },
            Err(e) => {
                tracing::error!("received invalid ServerEvent {}", e);
            }
        }
    }
}

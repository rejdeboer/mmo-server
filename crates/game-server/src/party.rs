use bevy::prelude::*;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::components::CharacterIdComponent;

/// Party membership update received from the web-server via NATS.
#[derive(Deserialize, Debug)]
pub struct PartyUpdate {
    pub party_id: Option<i32>,
    pub members: Vec<i32>,
}

/// Component attached to character entities that are in a party.
#[derive(Component, Debug)]
pub struct PartyId(pub i32);

/// Resource that receives party updates from NATS subscription tasks.
#[derive(Resource)]
pub struct PartyUpdateReceiver {
    rx: mpsc::UnboundedReceiver<(i32, PartyUpdate)>,
}

/// Sender half, held by the NATS subscription tasks.
#[derive(Resource, Clone)]
pub struct PartyUpdateSender(pub mpsc::UnboundedSender<(i32, PartyUpdate)>);

pub fn new_party_channel() -> (PartyUpdateReceiver, PartyUpdateSender) {
    let (tx, rx) = mpsc::unbounded_channel();
    (PartyUpdateReceiver { rx }, PartyUpdateSender(tx))
}

/// System that drains incoming party updates from NATS and applies them as components.
pub fn process_party_updates(
    mut receiver: ResMut<PartyUpdateReceiver>,
    mut commands: Commands,
    characters: Query<(Entity, &CharacterIdComponent)>,
) {
    while let Ok((character_id, update)) = receiver.rx.try_recv() {
        let Some((entity, _)) = characters.iter().find(|(_, id)| id.0 == character_id) else {
            continue;
        };

        match update.party_id {
            Some(party_id) => {
                commands.entity(entity).insert(PartyId(party_id));
            }
            None => {
                commands.entity(entity).remove::<PartyId>();
            }
        }
    }
}

/// Spawns a NATS subscription for a specific character's party updates.
/// Call this when a character connects to the game server.
pub fn subscribe_character_party(
    character_id: i32,
    nats_client: &async_nats::Client,
    sender: &PartyUpdateSender,
) {
    let client = nats_client.clone();
    let tx = sender.0.clone();
    let subject = format!("party.update.{character_id}");

    tokio::spawn(async move {
        use futures_util::StreamExt;

        let mut sub = match client.subscribe(subject.clone()).await {
            Ok(s) => s,
            Err(err) => {
                tracing::error!(?err, %subject, "failed to subscribe to party updates");
                return;
            }
        };

        while let Some(msg) = sub.next().await {
            match serde_json::from_slice::<PartyUpdate>(&msg.payload) {
                Ok(update) => {
                    if tx.send((character_id, update)).is_err() {
                        break; // Receiver dropped
                    }
                }
                Err(err) => {
                    tracing::warn!(?err, "invalid party update payload");
                }
            }
        }
    });
}

use bevy::prelude::*;
use futures_util::{FutureExt, StreamExt};
use serde::Deserialize;

use crate::core::CharacterIdComponent;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct PartyUpdate {
    pub party_id: Option<i32>,
    pub members: Vec<i32>,
}

#[derive(Component, Debug)]
#[allow(dead_code)]
pub struct PartyId(pub i32);

#[derive(Resource)]
pub struct PartySubscription(pub async_nats::Subscriber);

pub fn process_party_updates(
    subscription: Option<ResMut<PartySubscription>>,
    mut commands: Commands,
    characters: Query<(Entity, &CharacterIdComponent)>,
) {
    let Some(mut subscription) = subscription else {
        return;
    };

    while let Some(msg) = subscription.0.next().now_or_never().flatten() {
        let subject = msg.subject.as_str();
        let Some(character_id_str) = subject.strip_prefix("party.update.") else {
            tracing::warn!(%subject, "unexpected party update subject");
            continue;
        };
        let Ok(character_id) = character_id_str.parse::<i32>() else {
            tracing::warn!(%character_id_str, "invalid character_id in party update subject");
            continue;
        };

        let update = match serde_json::from_slice::<PartyUpdate>(&msg.payload) {
            Ok(u) => u,
            Err(err) => {
                tracing::warn!(?err, "invalid party update payload");
                continue;
            }
        };

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

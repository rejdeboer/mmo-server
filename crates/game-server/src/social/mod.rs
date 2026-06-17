mod chat;
mod party;

pub use party::PartySubscription;

use crate::configuration::Settings;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SocialSet {
    /// Poll external updates (NATS party subscription).
    ReceiveUpdates,
    /// Process chat messages.
    ProcessChat,
}

#[derive(Message, Debug)]
pub struct IncomingChatMessage {
    pub author: Entity,
    pub channel: protocol::models::ChatChannel,
    pub text: String,
}

/// NATS client for receiving cross-service messages (party updates, etc.)
#[derive(Resource, Clone)]
pub struct NatsClient(pub async_nats::Client);

pub struct SocialPlugin;

impl Plugin for SocialPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<IncomingChatMessage>();

        app.add_systems(
            FixedPreUpdate,
            party::process_party_updates.in_set(SocialSet::ReceiveUpdates),
        );
        app.add_systems(
            FixedPreUpdate,
            chat::process_incoming_chat.in_set(SocialSet::ProcessChat),
        );

        app.add_systems(Startup, setup_nats);
    }
}

fn setup_nats(mut commands: Commands, runtime: Res<TokioTasksRuntime>, settings: Res<Settings>) {
    let Some(url) = &settings.nats_url else {
        info!("NATS URL not configured, party updates disabled");
        return;
    };

    let url = url.clone();
    match runtime
        .runtime()
        .block_on(async { async_nats::connect(&url).await })
    {
        Ok(client) => {
            info!(%url, "connected to NATS");
            match runtime
                .runtime()
                .block_on(async { client.subscribe("party.update.*").await })
            {
                Ok(subscriber) => {
                    commands.insert_resource(PartySubscription(subscriber));
                }
                Err(err) => {
                    error!(?err, "failed to subscribe to party updates");
                }
            }
            commands.insert_resource(NatsClient(client));
        }
        Err(err) => {
            error!(?err, "failed to connect to NATS, party updates disabled");
        }
    }
}

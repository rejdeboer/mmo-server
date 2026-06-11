use futures_util::StreamExt;
use metrics::{counter, gauge};
use protocol::social::{ChannelType, SocialEvent};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::social::{
    command::{HubMessage, Recipient},
    error::HubError,
    nats::{
        NatsBridge, NatsEnvelope, PartyUpdate, guild_subject, party_chat_subject,
        party_update_subject, whisper_subject,
    },
};

use super::command::HubCommand;
use super::rate_limit::TokenBucket;

/// Max burst of messages a client can send at once.
const RATE_LIMIT_BURST: f64 = 5.0;
/// Messages allowed per second (sustained rate).
const RATE_LIMIT_PER_SECOND: f64 = 2.0;

struct ConnectedClient {
    pub character_name: String,
    pub guild_id: Option<i32>,
    pub party_id: Option<i32>,
    pub tx: Sender<Arc<[u8]>>,
    pub rate_limiter: TokenBucket,
}

/// A pending party invitation.
struct PartyInvite {
    pub from_id: i32,
    pub party_id: Option<i32>, // None if inviter has no party yet (will create on accept)
}

struct Party {
    leader_id: i32,
    members: Vec<i32>,
}

/// Internal messages from NATS subscription tasks to the Hub.
enum NatsEvent {
    Guild {
        guild_id: i32,
        envelope: NatsEnvelope,
    },
    Whisper {
        character_id: i32,
        envelope: NatsEnvelope,
    },
    Party {
        party_id: i32,
        envelope: NatsEnvelope,
    },
}

pub struct Hub {
    clients: HashMap<i32, ConnectedClient>,
    rx: Receiver<HubMessage>,
    nats_rx: Receiver<NatsEvent>,
    nats_tx: Sender<NatsEvent>,
    guilds: HashMap<i32, Vec<i32>>,
    /// party_id → party state
    parties: HashMap<i32, Party>,
    /// character_id → pending invite
    pending_invites: HashMap<i32, PartyInvite>,
    /// Counter for generating party IDs
    next_party_id: i32,
    db_pool: PgPool,
    nats: Option<NatsBridge>,
    /// Track active guild subscriptions so we can unsubscribe
    guild_sub_handles: HashMap<i32, tokio::task::JoinHandle<()>>,
    /// Track active whisper subscriptions
    whisper_sub_handles: HashMap<i32, tokio::task::JoinHandle<()>>,
    /// Track active party subscriptions
    party_sub_handles: HashMap<i32, tokio::task::JoinHandle<()>>,
}

impl Hub {
    pub fn new(db_pool: PgPool, rx: Receiver<HubMessage>, nats: Option<NatsBridge>) -> Self {
        let (nats_tx, nats_rx) = channel::<NatsEvent>(256);
        Self {
            clients: HashMap::new(),
            rx,
            nats_rx,
            nats_tx,
            guilds: HashMap::new(),
            parties: HashMap::new(),
            pending_invites: HashMap::new(),
            next_party_id: 1,
            db_pool,
            nats,
            guild_sub_handles: HashMap::new(),
            whisper_sub_handles: HashMap::new(),
            party_sub_handles: HashMap::new(),
        }
    }

    #[tracing::instrument(name = "social_hub", skip_all)]
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                biased;

                Some(message) = self.rx.recv() => {
                    self.process_message(message).await;
                }

                Some(event) = self.nats_rx.recv() => {
                    match event {
                        NatsEvent::Guild { guild_id, envelope } => {
                            counter!("nats_messages_received_total", "subject_prefix" => "social.guild").increment(1);
                            self.handle_remote_guild(guild_id, envelope).await;
                        }
                        NatsEvent::Whisper { character_id, envelope } => {
                            counter!("nats_messages_received_total", "subject_prefix" => "social.whisper").increment(1);
                            self.handle_remote_whisper(character_id, envelope).await;
                        }
                        NatsEvent::Party { party_id, envelope } => {
                            counter!("nats_messages_received_total", "subject_prefix" => "social.party").increment(1);
                            self.handle_remote_party(party_id, envelope).await;
                        }
                    }
                }

                else => break,
            }
        }
    }

    /// Handle a guild message arriving from NATS.
    async fn handle_remote_guild(&self, guild_id: i32, envelope: NatsEnvelope) {
        let Some(members) = self.guilds.get(&guild_id) else {
            return;
        };

        let msg: Arc<[u8]> = Arc::from(envelope.payload);
        for &member_id in members {
            if let Some(client) = self.clients.get(&member_id)
                && let Err(err) = client.tx.send(msg.clone()).await
            {
                tracing::error!(?err, member_id, "failed to deliver guild message");
            }
        }
    }

    /// Handle a whisper arriving from NATS.
    async fn handle_remote_whisper(&self, character_id: i32, envelope: NatsEnvelope) {
        if let Some(client) = self.clients.get(&character_id) {
            let msg: Arc<[u8]> = Arc::from(envelope.payload);
            if let Err(err) = client.tx.send(msg).await {
                tracing::error!(?err, character_id, "failed to deliver whisper");
            }
        }
    }

    async fn process_message(&mut self, msg: HubMessage) {
        match msg.command {
            HubCommand::Connect {
                character_name,
                guild_id,
                tx,
            } => {
                // Spawn a whisper subscription task for this character
                self.spawn_whisper_sub(msg.sender_id);

                self.clients.insert(
                    msg.sender_id,
                    ConnectedClient {
                        character_name,
                        guild_id,
                        party_id: None,
                        tx,
                        rate_limiter: TokenBucket::new(RATE_LIMIT_BURST, RATE_LIMIT_PER_SECOND),
                    },
                );

                if let Some(guild_id) = guild_id {
                    let members = self.guilds.entry(guild_id).or_default();
                    let is_first = members.is_empty();
                    members.push(msg.sender_id);

                    if is_first {
                        gauge!("social_guilds_active").increment(1.0);
                        self.spawn_guild_sub(guild_id);
                    }
                }
            }
            HubCommand::Disconnect => {
                // Cancel whisper subscription
                if let Some(handle) = self.whisper_sub_handles.remove(&msg.sender_id) {
                    handle.abort();
                }

                // Remove pending invites for this player
                self.pending_invites.remove(&msg.sender_id);

                // Leave party on disconnect
                let party_id = self.clients.get(&msg.sender_id).and_then(|c| c.party_id);
                if let Some(pid) = party_id {
                    self.remove_from_party(msg.sender_id, pid).await;
                }

                if let Some(client) = self.clients.remove(&msg.sender_id)
                    && let Some(gid) = client.guild_id
                    && let Some(members) = self.guilds.get_mut(&gid)
                {
                    members.retain(|&id| id != msg.sender_id);
                    if members.is_empty() {
                        self.guilds.remove(&gid);
                        gauge!("social_guilds_active").decrement(1.0);
                        if let Some(handle) = self.guild_sub_handles.remove(&gid) {
                            handle.abort();
                        }
                    }
                }
            }
            HubCommand::ChatMessage { channel, text } => {
                if !self.check_rate_limit(msg.sender_id).await {
                    return;
                }
                let channel_label = match channel {
                    ChannelType::Guild => "guild",
                    ChannelType::Party => "party",
                    _ => "unknown",
                };
                counter!("social_messages_total", "channel" => channel_label).increment(1);
                self.handle_chat(msg.sender_id, channel, &text).await;
            }
            HubCommand::Whisper { recipient, text } => {
                if !self.check_rate_limit(msg.sender_id).await {
                    return;
                }
                counter!("social_messages_total", "channel" => "whisper").increment(1);
                self.handle_whisper(msg.sender_id, recipient, &text).await;
            }
            HubCommand::PartyInvite { target } => {
                counter!("social_party_actions_total", "action" => "invite").increment(1);
                self.handle_party_invite(msg.sender_id, target).await;
            }
            HubCommand::PartyAccept => {
                counter!("social_party_actions_total", "action" => "accept").increment(1);
                self.handle_party_accept(msg.sender_id).await;
            }
            HubCommand::PartyDecline => {
                counter!("social_party_actions_total", "action" => "decline").increment(1);
                self.handle_party_decline(msg.sender_id).await;
            }
            HubCommand::PartyLeave => {
                counter!("social_party_actions_total", "action" => "leave").increment(1);
                self.handle_party_leave(msg.sender_id).await;
            }
            HubCommand::PartyKick { target_id } => {
                counter!("social_party_actions_total", "action" => "kick").increment(1);
                self.handle_party_kick(msg.sender_id, target_id).await;
            }
        };
    }

    fn spawn_whisper_sub(&mut self, character_id: i32) {
        let Some(nats) = self.nats.clone() else {
            return;
        };
        let nats_tx = self.nats_tx.clone();
        let subject = whisper_subject(character_id);

        let handle = tokio::spawn(async move {
            let mut sub = match nats.subscribe(&subject).await {
                Ok(s) => s,
                Err(err) => {
                    tracing::error!(?err, %subject, "failed to subscribe");
                    return;
                }
            };

            while let Some(msg) = sub.next().await
                && let Some(envelope) = NatsBridge::deserialize_envelope(&msg.payload)
            {
                if nats_tx
                    .send(NatsEvent::Whisper {
                        character_id,
                        envelope,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        self.whisper_sub_handles.insert(character_id, handle);
    }

    fn spawn_guild_sub(&mut self, guild_id: i32) {
        let Some(nats) = self.nats.clone() else {
            return;
        };
        let nats_tx = self.nats_tx.clone();
        let subject = guild_subject(guild_id);

        let handle = tokio::spawn(async move {
            let mut sub = match nats.subscribe(&subject).await {
                Ok(s) => s,
                Err(err) => {
                    tracing::error!(?err, %subject, "failed to subscribe");
                    return;
                }
            };

            while let Some(msg) = sub.next().await {
                if let Some(envelope) = NatsBridge::deserialize_envelope(&msg.payload)
                    && nats_tx
                        .send(NatsEvent::Guild { guild_id, envelope })
                        .await
                        .is_err()
                {
                    break;
                }
            }
        });

        self.guild_sub_handles.insert(guild_id, handle);
    }

    async fn handle_chat(&self, sender_id: i32, channel: ChannelType, text: &str) {
        let sender_client = self.get_client_unchecked(&sender_id);
        match channel {
            ChannelType::Guild => {
                self.handle_guild_message(sender_id, sender_client, text)
                    .await
            }
            ChannelType::Party => {
                self.handle_party_message(sender_id, sender_client, text)
                    .await
            }
            unsupported => {
                tracing::error!(?unsupported, "channel not supported");
                self.write_error(HubError::Unexpected, sender_client.tx.clone())
                    .await;
            }
        };
    }

    async fn handle_guild_message(
        &self,
        sender_id: i32,
        sender_client: &ConnectedClient,
        text: &str,
    ) {
        let Some(gid) = sender_client.guild_id else {
            return self
                .write_error(HubError::SenderNotInGuild, sender_client.tx.clone())
                .await;
        };

        let event = SocialEvent::Chat {
            channel: ChannelType::Guild,
            sender_id,
            sender_name: sender_client.character_name.clone(),
            text: text.to_string(),
        };
        let msg: Arc<[u8]> = Arc::from(bitcode::encode(&event));

        // Publish to NATS — delivery happens when we receive our own message back
        if let Some(nats) = &self.nats {
            let envelope = NatsEnvelope {
                payload: msg.to_vec(),
            };
            nats.publish(&guild_subject(gid), &envelope).await;
            counter!("social_messages_delivered_total", "channel" => "guild", "delivery" => "nats")
                .increment(1);
        }
    }

    async fn handle_party_message(
        &self,
        sender_id: i32,
        sender_client: &ConnectedClient,
        text: &str,
    ) {
        let Some(party_id) = sender_client.party_id else {
            return self
                .write_error(HubError::NotInParty, sender_client.tx.clone())
                .await;
        };

        let event = SocialEvent::Chat {
            channel: ChannelType::Party,
            sender_id,
            sender_name: sender_client.character_name.clone(),
            text: text.to_string(),
        };
        let msg: Arc<[u8]> = Arc::from(bitcode::encode(&event));

        // Publish to NATS — delivery happens when we receive our own message back
        if let Some(nats) = &self.nats {
            let envelope = NatsEnvelope {
                payload: msg.to_vec(),
            };
            nats.publish(&party_chat_subject(party_id), &envelope).await;
            counter!("social_messages_delivered_total", "channel" => "party", "delivery" => "nats")
                .increment(1);
        }
    }

    async fn handle_whisper(&self, sender_id: i32, recipient: Recipient, text: &str) {
        let sender_client = self.get_client_unchecked(&sender_id);
        let recipient_id = match self.resolve_recipient_id(recipient).await {
            Ok(id) => id,
            Err(err) => return self.write_error(err, sender_client.tx.clone()).await,
        };

        // Build the whisper message
        let event = SocialEvent::Whisper {
            sender_id,
            sender_name: sender_client.character_name.clone(),
            text: text.to_string(),
        };
        let whisper: Arc<[u8]> = Arc::from(bitcode::encode(&event));

        // Try local delivery first
        if let Some(recipient_client) = self.clients.get(&recipient_id) {
            if let Err(err) = recipient_client.tx.send(whisper.clone()).await {
                tracing::error!(?err, "failed to send whisper to recipient");
                return self
                    .write_error(HubError::Unexpected, sender_client.tx.clone())
                    .await;
            }
            counter!("social_messages_delivered_total", "channel" => "whisper", "delivery" => "local").increment(1);
        } else {
            // Recipient not on this instance - publish via NATS if available
            if let Some(nats) = &self.nats {
                let envelope = NatsEnvelope {
                    payload: whisper.to_vec(),
                };
                nats.publish(&whisper_subject(recipient_id), &envelope)
                    .await;
                counter!("social_messages_delivered_total", "channel" => "whisper", "delivery" => "nats").increment(1);
            } else {
                tracing::warn!(recipient_id, "recipient not connected and NATS unavailable");
            }
        }

        // Send receipt to sender (always local since sender is on this instance)
        let recipient_name = if let Some(rc) = self.clients.get(&recipient_id) {
            rc.character_name.clone()
        } else {
            match sqlx::query!("SELECT name FROM characters WHERE id = $1", recipient_id)
                .fetch_one(&self.db_pool)
                .await
            {
                Ok(row) => row.name,
                Err(_) => String::from("Unknown"),
            }
        };

        let receipt_event = SocialEvent::WhisperReceipt {
            recipient_id,
            recipient_name,
            text: text.to_string(),
        };
        let receipt: Arc<[u8]> = Arc::from(bitcode::encode(&receipt_event));

        if let Err(err) = sender_client.tx.send(receipt).await {
            tracing::error!(?err, "failed to send receipt to sender");
        }
    }

    async fn write_error(&self, error: HubError, tx: Sender<Arc<[u8]>>) {
        let error_label = match &error {
            HubError::SenderNotInGuild => "not_in_guild",
            HubError::RecipientNotFound => "recipient_not_found",
            HubError::RateLimited => "rate_limited",
            HubError::TargetAlreadyInParty => "target_already_in_party",
            HubError::NoPendingInvite => "no_pending_invite",
            HubError::NotInParty => "not_in_party",
            HubError::NotPartyLeader => "not_party_leader",
            HubError::Unexpected => "unexpected",
        };
        counter!("social_errors_total", "error" => error_label).increment(1);

        let text = match error {
            HubError::SenderNotInGuild => "You are not in a guild",
            HubError::RecipientNotFound => "Player not found",
            HubError::RateLimited => "You are sending messages too fast",
            HubError::TargetAlreadyInParty => "That player is already in a party",
            HubError::NoPendingInvite => "You have no pending party invite",
            HubError::NotInParty => "You are not in a party",
            HubError::NotPartyLeader => "Only the party leader can do that",
            HubError::Unexpected => "An unexpected error occured, please try re-logging",
        };

        let event = SocialEvent::Error {
            message: text.to_string(),
        };
        let msg: Arc<[u8]> = Arc::from(bitcode::encode(&event));

        if let Err(err) = tx.send(msg).await {
            tracing::error!(?err, "failed to send error");
        }
    }

    async fn resolve_recipient_id(&self, recipient: Recipient) -> Result<i32, HubError> {
        match recipient {
            Recipient::Id(id) => Ok(id),
            Recipient::Name(name) => {
                let id = sqlx::query!("SELECT id from characters WHERE name = $1 LIMIT 1", &name)
                    .fetch_one(&self.db_pool)
                    .await
                    .map_err(|err| match err {
                        sqlx::Error::Database(_) => HubError::RecipientNotFound,
                        _ => HubError::Unexpected,
                    })?
                    .id;

                Ok(id)
            }
        }
    }

    // ─── Party handling ───────────────────────────────────────────────

    /// Handle a party chat message arriving from NATS.
    async fn handle_remote_party(&self, party_id: i32, envelope: NatsEnvelope) {
        let Some(party) = self.parties.get(&party_id) else {
            return;
        };

        let msg: Arc<[u8]> = Arc::from(envelope.payload);
        for &member_id in &party.members {
            if let Some(client) = self.clients.get(&member_id)
                && let Err(err) = client.tx.send(msg.clone()).await
            {
                tracing::error!(?err, member_id, "failed to deliver party message");
            }
        }
    }

    async fn handle_party_invite(&mut self, sender_id: i32, target: Recipient) {
        let sender_client = self.get_client_unchecked(&sender_id);
        let sender_name = sender_client.character_name.clone();
        let sender_party_id = sender_client.party_id;

        let target_id = match self.resolve_recipient_id(target).await {
            Ok(id) => id,
            Err(err) => {
                let tx = self.clients.get(&sender_id).unwrap().tx.clone();
                return self.write_error(err, tx).await;
            }
        };

        // Check target is online on this instance
        if !self.clients.contains_key(&target_id) {
            // TODO: Publish invite via NATS for cross-instance invites
            let tx = self.clients.get(&sender_id).unwrap().tx.clone();
            return self.write_error(HubError::RecipientNotFound, tx).await;
        }

        // Check target isn't already in a party
        if let Some(target_client) = self.clients.get(&target_id)
            && target_client.party_id.is_some()
        {
            let tx = self.clients.get(&sender_id).unwrap().tx.clone();
            return self.write_error(HubError::TargetAlreadyInParty, tx).await;
        }

        // Store pending invite
        self.pending_invites.insert(
            target_id,
            PartyInvite {
                from_id: sender_id,
                party_id: sender_party_id,
            },
        );

        // Notify target of invite
        self.write_system_message(
            target_id,
            &format!("{sender_name} has invited you to a party"),
        )
        .await;
    }

    async fn handle_party_accept(&mut self, character_id: i32) {
        let Some(invite) = self.pending_invites.remove(&character_id) else {
            let tx = self.clients.get(&character_id).unwrap().tx.clone();
            return self.write_error(HubError::NoPendingInvite, tx).await;
        };

        let party_id = if let Some(pid) = invite.party_id {
            // Join existing party
            pid
        } else {
            // Create new party with inviter as leader
            let pid = self.next_party_id;
            self.next_party_id += 1;

            if let Some(inviter) = self.clients.get_mut(&invite.from_id) {
                inviter.party_id = Some(pid);
            }
            self.parties.insert(
                pid,
                Party {
                    leader_id: invite.from_id,
                    members: vec![invite.from_id],
                },
            );
            gauge!("social_parties_active").increment(1.0);
            self.spawn_party_sub(pid);

            pid
        };

        // Update local state
        if let Some(client) = self.clients.get_mut(&character_id) {
            client.party_id = Some(party_id);
        }
        if let Some(party) = self.parties.get_mut(&party_id) {
            party.members.push(character_id);
        }

        // Publish membership update to NATS for game server
        self.publish_party_update(party_id).await;

        // Notify party members
        let character_name = self
            .clients
            .get(&character_id)
            .map(|c| c.character_name.clone())
            .unwrap_or_default();
        self.broadcast_party_system_message(
            party_id,
            &format!("{character_name} has joined the party"),
        )
        .await;
    }

    async fn handle_party_decline(&mut self, character_id: i32) {
        let Some(invite) = self.pending_invites.remove(&character_id) else {
            let tx = self.clients.get(&character_id).unwrap().tx.clone();
            return self.write_error(HubError::NoPendingInvite, tx).await;
        };

        // Notify inviter
        let character_name = self
            .clients
            .get(&character_id)
            .map(|c| c.character_name.clone())
            .unwrap_or_default();
        self.write_system_message(
            invite.from_id,
            &format!("{character_name} declined your party invite"),
        )
        .await;
    }

    async fn handle_party_leave(&mut self, character_id: i32) {
        let Some(party_id) = self.clients.get(&character_id).and_then(|c| c.party_id) else {
            let tx = self.clients.get(&character_id).unwrap().tx.clone();
            return self.write_error(HubError::NotInParty, tx).await;
        };

        self.remove_from_party(character_id, party_id).await;
    }

    async fn handle_party_kick(&mut self, sender_id: i32, target_id: i32) {
        let Some(party_id) = self.clients.get(&sender_id).and_then(|c| c.party_id) else {
            let tx = self.clients.get(&sender_id).unwrap().tx.clone();
            return self.write_error(HubError::NotInParty, tx).await;
        };

        // Verify sender is party leader
        let is_leader = self
            .parties
            .get(&party_id)
            .map(|p| p.leader_id == sender_id)
            .unwrap_or(false);

        if !is_leader {
            let tx = self.clients.get(&sender_id).unwrap().tx.clone();
            return self.write_error(HubError::NotPartyLeader, tx).await;
        }

        self.remove_from_party(target_id, party_id).await;
        self.write_system_message(target_id, "You have been kicked from the party")
            .await;
    }

    async fn remove_from_party(&mut self, character_id: i32, party_id: i32) {
        // Update local state
        if let Some(client) = self.clients.get_mut(&character_id) {
            client.party_id = None;
        }

        if let Some(party) = self.parties.get_mut(&party_id) {
            party.members.retain(|&id| id != character_id);

            if party.members.is_empty() {
                self.parties.remove(&party_id);
                gauge!("social_parties_active").decrement(1.0);
                if let Some(handle) = self.party_sub_handles.remove(&party_id) {
                    handle.abort();
                }
            }
        }

        // Publish membership update
        self.publish_party_update(party_id).await;

        // Notify character they're no longer in a party
        self.publish_party_update_for_character(character_id, None)
            .await;

        let character_name = self
            .clients
            .get(&character_id)
            .map(|c| c.character_name.clone())
            .unwrap_or_default();
        self.broadcast_party_system_message(
            party_id,
            &format!("{character_name} has left the party"),
        )
        .await;
    }

    async fn publish_party_update(&self, party_id: i32) {
        let Some(nats) = &self.nats else { return };

        let Some(party) = self.parties.get(&party_id) else {
            return;
        };

        let update = PartyUpdate {
            party_id: Some(party_id),
            members: party.members.clone(),
        };

        // Notify each member's game server
        for &member_id in &party.members {
            nats.publish_json(&party_update_subject(member_id), &update)
                .await;
        }
    }

    async fn publish_party_update_for_character(&self, character_id: i32, party_id: Option<i32>) {
        let Some(nats) = &self.nats else { return };
        let update = PartyUpdate {
            party_id,
            members: vec![],
        };
        nats.publish_json(&party_update_subject(character_id), &update)
            .await;
    }

    async fn broadcast_party_system_message(&self, party_id: i32, text: &str) {
        let Some(party) = self.parties.get(&party_id) else {
            return;
        };
        for &member_id in &party.members {
            self.write_system_message(member_id, text).await;
        }
    }

    async fn write_system_message(&self, character_id: i32, text: &str) {
        let Some(client) = self.clients.get(&character_id) else {
            return;
        };

        let event = SocialEvent::SystemMessage {
            text: text.to_string(),
        };
        let msg: Arc<[u8]> = Arc::from(bitcode::encode(&event));

        if let Err(err) = client.tx.send(msg).await {
            tracing::error!(?err, character_id, "failed to send system message");
        }
    }

    fn spawn_party_sub(&mut self, party_id: i32) {
        let Some(nats) = self.nats.clone() else {
            return;
        };
        let nats_tx = self.nats_tx.clone();
        let subject = party_chat_subject(party_id);

        let handle = tokio::spawn(async move {
            let mut sub = match nats.subscribe(&subject).await {
                Ok(s) => s,
                Err(err) => {
                    tracing::error!(?err, %subject, "failed to subscribe");
                    return;
                }
            };

            while let Some(msg) = sub.next().await {
                if let Some(envelope) = NatsBridge::deserialize_envelope(&msg.payload)
                    && nats_tx
                        .send(NatsEvent::Party { party_id, envelope })
                        .await
                        .is_err()
                {
                    break;
                }
            }
        });

        self.party_sub_handles.insert(party_id, handle);
    }

    // ─── Helpers ─────────────────────────────────────────────────────

    #[inline]
    fn get_client_unchecked(&self, client_id: &i32) -> &ConnectedClient {
        self.clients.get(client_id).expect("failed to fetch client")
    }

    /// Check rate limit for a client. Returns `true` if allowed, `false` if rate limited.
    /// Sends a system error message to the client if denied.
    async fn check_rate_limit(&mut self, sender_id: i32) -> bool {
        let Some(client) = self.clients.get_mut(&sender_id) else {
            return false;
        };

        if client.rate_limiter.allow() {
            return true;
        }

        counter!("social_rate_limit_denied_total").increment(1);
        tracing::warn!(sender_id, "rate limit denied");
        let tx = client.tx.clone();
        self.write_error(HubError::RateLimited, tx).await;
        false
    }
}

use bevy::prelude::*;
use bevy_renet::{RenetClient, renet::DefaultChannel};
use protocol::models::ChatChannel;
use std::collections::VecDeque;
use tokio::sync::mpsc;
use web_client::{ChannelType, SocialAction};

/// A unified chat message from any source (game-server or social hub).
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub channel: ChatMessageChannel,
    pub sender: String,
    pub text: String,
}

/// All possible chat channels, unifying game-server and social hub channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatMessageChannel {
    // Location-based (game-server)
    Say,
    Yell,
    Zone,
    // Social (web-server)
    Guild,
    Party,
    Trade,
    Whisper,
    WhisperSent,
    // System
    System,
}

impl ChatMessageChannel {
    /// Display name shown in the chat log, e.g. `[Say]`.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Say => "[Say]",
            Self::Yell => "[Yell]",
            Self::Zone => "[Zone]",
            Self::Guild => "[Guild]",
            Self::Party => "[Party]",
            Self::Trade => "[Trade]",
            Self::Whisper => "[Whisper]",
            Self::WhisperSent => "[To]",
            Self::System => "[System]",
        }
    }

    /// Color used for this channel in the chat log.
    pub fn color(&self) -> Color {
        match self {
            Self::Say => Color::WHITE,
            Self::Yell => Color::srgb(1.0, 0.2, 0.2),
            Self::Zone => Color::srgb(1.0, 0.75, 0.8),
            Self::Guild => Color::srgb(0.2, 1.0, 0.2),
            Self::Party => Color::srgb(0.4, 0.6, 1.0),
            Self::Trade => Color::srgb(1.0, 0.8, 0.2),
            Self::Whisper | Self::WhisperSent => Color::srgb(0.8, 0.4, 1.0),
            Self::System => Color::srgb(1.0, 1.0, 0.4),
        }
    }
}

/// Maximum number of messages kept in the scrollback buffer.
const MAX_CHAT_HISTORY: usize = 200;

/// Scrollback buffer of chat messages.
#[derive(Resource)]
pub struct ChatLog {
    pub messages: VecDeque<ChatMessage>,
}

impl Default for ChatLog {
    fn default() -> Self {
        Self {
            messages: VecDeque::with_capacity(MAX_CHAT_HISTORY),
        }
    }
}

impl ChatLog {
    pub fn push(&mut self, message: ChatMessage) {
        if self.messages.len() >= MAX_CHAT_HISTORY {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
    }
}

// ---------------------------------------------------------------------------
// Outgoing chat message (bridge from UI to network)
// ---------------------------------------------------------------------------

/// The target channel for an outgoing chat message.
#[derive(Debug, Clone)]
pub enum OutgoingChannel {
    Say,
    Yell,
    Zone,
    Guild,
    Party,
    Trade,
    Whisper { target_name: String },
}

/// Fired by the chat UI when the player submits a message.
/// The `route_outgoing_chat` observer picks this up and sends it
/// to the appropriate backend (renet for location, WebSocket for social).
#[derive(Event)]
pub struct OutgoingChatMessage {
    pub channel: OutgoingChannel,
    pub text: String,
}

/// Sender half of the social WebSocket connection. `None` until connected.
#[derive(Resource)]
pub struct SocialSender(pub Option<mpsc::Sender<SocialAction>>);

/// Receiver half of the social WebSocket connection. `None` until connected.
#[derive(Resource)]
pub struct SocialReceiver(pub Option<mpsc::Receiver<web_client::SocialEvent>>);

/// Spawns the social WebSocket connection task on the tokio runtime.
/// On completion, it calls back to the main thread to populate
/// `SocialSender` and `SocialReceiver`. Runs once at startup.
pub fn connect_social(
    web_api: Res<crate::application::WebApi>,
    settings: Res<crate::configuration::Settings>,
    runtime: Res<bevy_tokio_tasks::TokioTasksRuntime>,
) {
    let Some(jwt) = web_api.0.jwt() else {
        tracing::warn!("cannot connect to social server: no JWT available");
        return;
    };

    let ws_url = format!(
        "{}/social",
        settings
            .web_server
            .endpoint
            .replace("http://", "ws://")
            .replace("https://", "wss://")
    );
    let jwt = jwt.to_owned();

    runtime.spawn_background_task(|mut ctx| async move {
        match web_client::connect(&ws_url, &jwt).await {
            Ok((sender, receiver)) => {
                ctx.run_on_main_thread(move |main_ctx| {
                    main_ctx.world.resource_mut::<SocialSender>().0 = Some(sender);
                    main_ctx.world.resource_mut::<SocialReceiver>().0 = Some(receiver);
                    tracing::info!("social WebSocket connected");
                })
                .await;
            }
            Err(e) => {
                tracing::error!("failed to connect to social WebSocket: {:?}", e);
            }
        }
    });

    tracing::info!("social WebSocket connection task spawned");
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Routes an outgoing chat message to the appropriate backend.
pub fn route_outgoing_chat(
    event: On<OutgoingChatMessage>,
    mut renet_client: ResMut<RenetClient>,
    social_sender: Res<SocialSender>,
) {
    match &event.channel {
        OutgoingChannel::Say | OutgoingChannel::Yell | OutgoingChannel::Zone => {
            let chat_channel = match &event.channel {
                OutgoingChannel::Say => ChatChannel::Say,
                OutgoingChannel::Yell => ChatChannel::Yell,
                OutgoingChannel::Zone => ChatChannel::Zone,
                _ => unreachable!(),
            };
            let action = protocol::client::PlayerAction::Chat {
                channel: chat_channel,
                text: event.text.clone(),
            };
            let encoded = bitcode::encode(&action);
            renet_client.send_message(DefaultChannel::ReliableOrdered, encoded);
        }
        OutgoingChannel::Guild | OutgoingChannel::Party | OutgoingChannel::Trade => {
            let channel_type = match &event.channel {
                OutgoingChannel::Guild => ChannelType::Guild,
                OutgoingChannel::Party => ChannelType::Party,
                OutgoingChannel::Trade => ChannelType::Trade,
                _ => unreachable!(),
            };
            if let Some(ref sender) = social_sender.0 {
                let _ = sender.try_send(SocialAction::Chat {
                    channel: channel_type,
                    text: event.text.clone(),
                });
            }
        }
        OutgoingChannel::Whisper { target_name } => {
            if let Some(ref sender) = social_sender.0 {
                let _ = sender.try_send(SocialAction::WhisperByName {
                    recipient_name: target_name.clone(),
                    text: event.text.clone(),
                });
            }
        }
    }
}

/// Polls the social WebSocket receiver and pushes incoming messages into the chat log.
pub fn poll_social_events(
    mut social_receiver: ResMut<SocialReceiver>,
    mut chat_log: ResMut<ChatLog>,
) {
    let Some(ref mut rx) = social_receiver.0 else {
        return;
    };

    while let Ok(event) = rx.try_recv() {
        match event {
            web_client::SocialEvent::Chat {
                channel,
                text,
                sender_name,
                ..
            } => {
                let ch = match channel {
                    ChannelType::Guild => ChatMessageChannel::Guild,
                    ChannelType::Party => ChatMessageChannel::Party,
                    ChannelType::Trade => ChatMessageChannel::Trade,
                };
                chat_log.push(ChatMessage {
                    channel: ch,
                    sender: sender_name,
                    text,
                });
            }
            web_client::SocialEvent::Whisper {
                text, sender_name, ..
            } => {
                chat_log.push(ChatMessage {
                    channel: ChatMessageChannel::Whisper,
                    sender: sender_name,
                    text,
                });
            }
            web_client::SocialEvent::WhisperReceipt {
                text,
                recipient_name,
                ..
            } => {
                chat_log.push(ChatMessage {
                    channel: ChatMessageChannel::WhisperSent,
                    sender: recipient_name,
                    text,
                });
            }
            web_client::SocialEvent::SystemMessage { text } => {
                chat_log.push(ChatMessage {
                    channel: ChatMessageChannel::System,
                    sender: String::new(),
                    text,
                });
            }
            web_client::SocialEvent::Error { message } => {
                chat_log.push(ChatMessage {
                    channel: ChatMessageChannel::System,
                    sender: String::new(),
                    text: message,
                });
            }
            web_client::SocialEvent::PartyInvite { from_name, .. } => {
                chat_log.push(ChatMessage {
                    channel: ChatMessageChannel::System,
                    sender: String::new(),
                    text: format!("{from_name} has invited you to a party"),
                });
            }
            web_client::SocialEvent::PartyUpdate { .. } => {
                // TODO: Update party UI
            }
            web_client::SocialEvent::PartyDisbanded => {
                chat_log.push(ChatMessage {
                    channel: ChatMessageChannel::System,
                    sender: String::new(),
                    text: "Your party has been disbanded".to_string(),
                });
            }
        }
    }
}

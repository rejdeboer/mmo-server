//! # Chat System
//!
//! Implements a WoW-style chatbox in the bottom-left corner of the screen.
//!
//! ## Design Overview
//!
//! ### Two Chat Backends
//!
//! 1. **Location chat** (`Say`, `Yell`, `Zone`) — sent as `PlayerAction::Chat` over
//!    the UDP game-server connection (bevy_renet). The game-server broadcasts these
//!    based on player proximity (Say = 32 units, Yell/Zone = visible range).
//!
//! 2. **Social chat** (`Guild`, `Party`, `Trade`, whispers) — sent as `SocialAction`
//!    over the WebSocket connection to the web-server social hub. These are routed
//!    by group membership, not position.
//!
//! ### Input Flow
//!
//! The chat system uses `bevy_enhanced_input` input contexts to toggle between
//! gameplay and text entry:
//!
//! - **Normal gameplay**: The player entity has the `PlayerComponent` input context
//!   which maps WASD to movement. The `Chatting` context is not active.
//!
//! - **Press Enter**: The `OpenChat` action (bound in `PlayerComponent` context)
//!   fires. This removes the `PlayerComponent` context from the player entity and
//!   adds the `Chatting` context, which consumes all keyboard input to prevent
//!   movement. The chat input field becomes visible and focused.
//!
//! - **Press Enter again**: Sends the typed message to the appropriate backend
//!   based on the active channel, clears the input, and swaps back to the
//!   `PlayerComponent` context.
//!
//! - **Press Escape**: Cancels text entry without sending, swaps back to gameplay.
//!
//! ### Chat Channels
//!
//! The active channel is selected by typing a prefix command:
//!
//! - `/s` or `/say` → Say (default)
//! - `/y` or `/yell` → Yell
//! - `/z` or `/zone` → Zone
//! - `/g` or `/guild` → Guild (social)
//! - `/p` or `/party` → Party (social)
//! - `/t` or `/trade` → Trade (social)
//! - `/w <name>` or `/whisper <name>` → Whisper (social)
//!
//! The default channel persists between messages (e.g. if you last typed in
//! guild chat, pressing Enter again defaults to guild).

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_renet::{RenetClient, renet::DefaultChannel};
use protocol::models::ChatChannel;
use std::collections::VecDeque;
use tokio::sync::mpsc;
use web_client::{ChannelType, SocialAction};

use crate::application::PlayerComponent;
use crate::input::Chatting;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum number of messages kept in the scrollback buffer.
const MAX_CHAT_HISTORY: usize = 200;

/// Maximum messages rendered in the visible chat panel.
const VISIBLE_MESSAGE_COUNT: usize = 12;

/// Chat panel width in pixels.
const CHAT_PANEL_WIDTH: f32 = 500.0;

// ---------------------------------------------------------------------------
// Chat message model
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// The active input channel that determines where the next message is sent.
#[derive(Debug, Clone)]
pub enum ActiveChannel {
    Say,
    Yell,
    Zone,
    Guild,
    Party,
    Trade,
    Whisper { target_name: String },
}

impl Default for ActiveChannel {
    fn default() -> Self {
        Self::Say
    }
}

impl ActiveChannel {
    pub fn prefix(&self) -> String {
        match self {
            Self::Say => "/s ".to_string(),
            Self::Yell => "/y ".to_string(),
            Self::Zone => "/z ".to_string(),
            Self::Guild => "/g ".to_string(),
            Self::Party => "/p ".to_string(),
            Self::Trade => "/t ".to_string(),
            Self::Whisper { target_name } => format!("/w {target_name} "),
        }
    }
}

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

/// Whether the chatbox input field is currently active.
#[derive(Resource, Default)]
pub struct ChatInputState {
    pub active: bool,
    pub text: String,
    pub channel: ActiveChannel,
}

/// Sender half of the social WebSocket connection. `None` until connected.
#[derive(Resource)]
pub struct SocialSender(pub Option<mpsc::Sender<SocialAction>>);

/// Receiver half of the social WebSocket connection. `None` until connected.
#[derive(Resource)]
pub struct SocialReceiver(pub Option<mpsc::Receiver<web_client::SocialEvent>>);

// ---------------------------------------------------------------------------
// UI marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct ChatPanel;

#[derive(Component)]
pub struct ChatMessageList;

#[derive(Component)]
pub struct ChatInputField;

#[derive(Component)]
pub struct ChatInputContainer;

// ---------------------------------------------------------------------------
// Input actions
// ---------------------------------------------------------------------------

/// Bound in the `PlayerComponent` context — pressing Enter opens chat.
#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenChat;

/// Bound in the `Chatting` context — pressing Enter sends the message.
#[derive(InputAction)]
#[action_output(bool)]
pub struct SendChat;

/// Bound in the `Chatting` context — pressing Escape cancels input.
#[derive(InputAction)]
#[action_output(bool)]
pub struct CancelChat;

/// Spawns the chat UI. Called from the `on_enter_game` observer.
pub fn spawn_chat_ui(commands: &mut Commands) {
    // Root panel — bottom-left corner
    let panel = commands
        .spawn((
            ChatPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                width: Val::Px(CHAT_PANEL_WIDTH),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();

    // Message list container
    let _message_list = commands
        .spawn((
            ChatMessageList,
            Node {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                max_height: Val::Px(300.0),
                padding: UiRect::all(Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
            ChildOf(panel),
        ))
        .id();

    // Input row (hidden until chat is opened)
    let input_container = commands
        .spawn((
            ChatInputContainer,
            Node {
                margin: UiRect::top(Val::Px(4.0)),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Visibility::Hidden,
            ChildOf(panel),
        ))
        .id();

    // Input text
    commands.spawn((
        ChatInputField,
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        ChildOf(input_container),
    ));
}

/// Handles the Enter key while in gameplay mode — opens the chat input.
pub fn handle_open_chat(
    open_action: Single<&TriggerState, With<Action<OpenChat>>>,
    mut chat_input: ResMut<ChatInputState>,
    mut q_input_container: Query<&mut Visibility, With<ChatInputContainer>>,
    mut q_input_text: Query<&mut Text, With<ChatInputField>>,
    mut q_player: Query<Entity, With<PlayerComponent>>,
    mut commands: Commands,
) {
    if *open_action != &TriggerState::Fired {
        return;
    }

    let Ok(player_entity) = q_player.single_mut() else {
        return;
    };

    chat_input.active = true;
    chat_input.text.clear();

    // Show the input container
    if let Ok(mut vis) = q_input_container.single_mut() {
        *vis = Visibility::Inherited;
    }

    // Show channel prefix
    if let Ok(mut text) = q_input_text.single_mut() {
        **text = chat_input.channel.prefix();
    }

    // Swap input contexts: remove gameplay, add chatting
    commands
        .entity(player_entity)
        .remove::<Actions<PlayerComponent>>()
        .insert(actions!(Chatting[
            (
                Action::<SendChat>::new(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Enter))),
            ),
            (
                Action::<CancelChat>::new(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Escape))),
            ),
        ]));
}

/// Handles keyboard text input while the chat input field is active.
///
/// We read raw `ReceivedCharacter` events and `KeyboardInput` for special keys
/// since the `Chatting` input context consumes normal key bindings.
pub fn handle_chat_text_input(
    mut char_events: MessageReader<KeyboardInput>,
    mut chat_input: ResMut<ChatInputState>,
    mut q_input_text: Query<&mut Text, With<ChatInputField>>,
) {
    if !chat_input.active {
        return;
    }

    for event in char_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match event.key_code {
            KeyCode::Backspace => {
                // Don't delete past the channel prefix
                let prefix_len = chat_input.channel.prefix().len();
                if chat_input.text.len() > prefix_len {
                    chat_input.text.pop();
                }
            }
            KeyCode::Space => {
                chat_input.text.push(' ');
            }
            _ => {
                // Convert key code to character
                if let Key::Character(ref c) = event.logical_key {
                    chat_input.text.push_str(c.as_str());
                }
            }
        }
    }

    // Update the displayed text
    if let Ok(mut text) = q_input_text.single_mut() {
        **text = chat_input.text.clone();
    }
}

/// Handles Enter while in chat mode — sends the message and returns to gameplay.
pub fn handle_send_chat(
    send_action: Single<&TriggerState, With<Action<SendChat>>>,
    mut chat_input: ResMut<ChatInputState>,
    _chat_log: ResMut<ChatLog>,
    mut renet_client: ResMut<RenetClient>,
    social_sender: Res<SocialSender>,
    mut q_input_container: Query<&mut Visibility, With<ChatInputContainer>>,
    mut q_player: Query<Entity, With<PlayerComponent>>,
    mut commands: Commands,
) {
    if *send_action != &TriggerState::Fired {
        return;
    }

    let Ok(player_entity) = q_player.single_mut() else {
        return;
    };

    // Extract the message text (everything after the channel prefix)
    let prefix_len = chat_input.channel.prefix().len();
    let _raw_text = chat_input.text[prefix_len..].trim().to_string();

    // Parse channel prefix changes — the user might have typed a new prefix
    let (channel, message_text) = parse_chat_input(&chat_input.text);

    if !message_text.is_empty() {
        // Send to the appropriate backend
        match &channel {
            ActiveChannel::Say | ActiveChannel::Yell | ActiveChannel::Zone => {
                let chat_channel = match &channel {
                    ActiveChannel::Say => ChatChannel::Say,
                    ActiveChannel::Yell => ChatChannel::Yell,
                    ActiveChannel::Zone => ChatChannel::Zone,
                    _ => unreachable!(),
                };
                let action = protocol::client::PlayerAction::Chat {
                    channel: chat_channel,
                    text: message_text.clone(),
                };
                let encoded = bitcode::encode(&action);
                renet_client.send_message(DefaultChannel::ReliableOrdered, encoded);
            }
            ActiveChannel::Guild | ActiveChannel::Party | ActiveChannel::Trade => {
                let channel_type = match &channel {
                    ActiveChannel::Guild => ChannelType::Guild,
                    ActiveChannel::Party => ChannelType::Party,
                    ActiveChannel::Trade => ChannelType::Trade,
                    _ => unreachable!(),
                };
                if let Some(ref sender) = social_sender.0 {
                    let _ = sender.try_send(SocialAction::Chat {
                        channel: channel_type,
                        text: message_text.clone(),
                    });
                }
            }
            ActiveChannel::Whisper { target_name } => {
                if let Some(ref sender) = social_sender.0 {
                    let _ = sender.try_send(SocialAction::WhisperByName {
                        recipient_name: target_name.clone(),
                        text: message_text.clone(),
                    });
                }
            }
        }

        // Update the persisted active channel
        chat_input.channel = channel;
    }

    // Close the input
    close_chat_input(&mut chat_input, &mut q_input_container);

    // Swap back to gameplay input context
    commands
        .entity(player_entity)
        .remove::<Actions<Chatting>>()
        .insert(actions!(PlayerComponent[
            (
                Action::<crate::input::Movement>::new(),
                DeadZone::default(),
                DeltaScale::default(),
                Scale::splat(10.0),
                Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick())),
            ),
            (
                Action::<OpenChat>::new(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Enter))),
            ),
        ]));
}

/// Handles Escape while in chat mode — cancels input and returns to gameplay.
pub fn handle_cancel_chat(
    cancel_action: Single<&TriggerState, With<Action<CancelChat>>>,
    mut chat_input: ResMut<ChatInputState>,
    mut q_input_container: Query<&mut Visibility, With<ChatInputContainer>>,
    mut q_player: Query<Entity, With<PlayerComponent>>,
    mut commands: Commands,
) {
    if *cancel_action != &TriggerState::Fired {
        return;
    }

    let Ok(player_entity) = q_player.single_mut() else {
        return;
    };

    close_chat_input(&mut chat_input, &mut q_input_container);

    // Swap back to gameplay input context
    commands
        .entity(player_entity)
        .remove::<Actions<Chatting>>()
        .insert(actions!(PlayerComponent[
            (
                Action::<crate::input::Movement>::new(),
                DeadZone::default(),
                DeltaScale::default(),
                Scale::splat(10.0),
                Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick())),
            ),
            (
                Action::<OpenChat>::new(),
                Bindings::spawn(Spawn(Binding::from(KeyCode::Enter))),
            ),
        ]));
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
                    _ => ChatMessageChannel::System,
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
        }
    }
}

/// Refreshes the chat message list UI from the `ChatLog` resource.
pub fn update_chat_ui(
    chat_log: Res<ChatLog>,
    q_message_list: Query<Entity, With<ChatMessageList>>,
    mut commands: Commands,
) {
    if !chat_log.is_changed() {
        return;
    }

    let Ok(list_entity) = q_message_list.single() else {
        return;
    };

    // Despawn old message children and rebuild.
    // This is simple and fine for a chat log that updates infrequently.
    commands.entity(list_entity).despawn_related::<Children>();

    let skip = chat_log
        .messages
        .len()
        .saturating_sub(VISIBLE_MESSAGE_COUNT);

    commands.entity(list_entity).with_children(|parent| {
        for msg in chat_log.messages.iter().skip(skip) {
            let label = msg.channel.label();
            let display = if msg.sender.is_empty() {
                format!("{label} {}", msg.text)
            } else {
                format!("{label} {}: {}", msg.sender, msg.text)
            };

            parent.spawn((
                Text::new(display),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(msg.channel.color()),
            ));
        }
    });
}

fn close_chat_input(
    chat_input: &mut ResMut<ChatInputState>,
    q_input_container: &mut Query<&mut Visibility, With<ChatInputContainer>>,
) {
    chat_input.active = false;
    chat_input.text.clear();

    if let Ok(mut vis) = q_input_container.single_mut() {
        *vis = Visibility::Hidden;
    }
}

/// Parses the raw input text to determine the channel and message body.
///
/// If the text starts with a known prefix command, that channel is used.
/// Otherwise, the current default channel is assumed.
fn parse_chat_input(input: &str) -> (ActiveChannel, String) {
    let trimmed = input.trim();

    // Try to match channel prefix commands
    if let Some(rest) = strip_prefix_cmd(trimmed, &["/s ", "/say "]) {
        return (ActiveChannel::Say, rest.to_string());
    }
    if let Some(rest) = strip_prefix_cmd(trimmed, &["/y ", "/yell "]) {
        return (ActiveChannel::Yell, rest.to_string());
    }
    if let Some(rest) = strip_prefix_cmd(trimmed, &["/z ", "/zone "]) {
        return (ActiveChannel::Zone, rest.to_string());
    }
    if let Some(rest) = strip_prefix_cmd(trimmed, &["/g ", "/guild "]) {
        return (ActiveChannel::Guild, rest.to_string());
    }
    if let Some(rest) = strip_prefix_cmd(trimmed, &["/p ", "/party "]) {
        return (ActiveChannel::Party, rest.to_string());
    }
    if let Some(rest) = strip_prefix_cmd(trimmed, &["/t ", "/trade "]) {
        return (ActiveChannel::Trade, rest.to_string());
    }
    if let Some(rest) = strip_prefix_cmd(trimmed, &["/w ", "/whisper "]) {
        // First word after /w is the target name, rest is the message
        let mut parts = rest.splitn(2, ' ');
        let target = parts.next().unwrap_or_default().to_string();
        let msg = parts.next().unwrap_or_default().to_string();
        return (
            ActiveChannel::Whisper {
                target_name: target,
            },
            msg,
        );
    }

    // No prefix — treat the whole text as the message on Say
    (ActiveChannel::Say, trimmed.to_string())
}

fn strip_prefix_cmd<'a>(input: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for prefix in prefixes {
        if let Some(rest) = input.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

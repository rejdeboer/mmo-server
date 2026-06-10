//! # Chat UI
//!
//! Implements a WoW-style chatbox in the bottom-left corner of the screen.
//!
//! ## Input Flow
//!
//! The chat system uses `bevy_enhanced_input` input contexts to toggle between
//! gameplay and text entry:
//!
//! - **Normal gameplay**: The player entity has the `PlayerComponent` input context
//!   which maps WASD to movement. The `Chatting` context is not active.
//!
//! - **Press Enter**: The `OpenChat` action fires. The `Chatting` context is
//!   activated and `PlayerComponent` context is deactivated. The chat input field
//!   becomes visible.
//!
//! - **Press Enter again**: Fires an `OutgoingChatMessage` with the parsed channel
//!   and text, clears the input, and swaps back to the `PlayerComponent` context.
//!
//! - **Press Escape**: Cancels text entry without sending, swaps back to gameplay.
//!
//! ## Chat Channels
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

use crate::application::PlayerComponent;
use crate::input::Chatting;
use crate::social::{ChatLog, OutgoingChannel, OutgoingChatMessage};

/// Chat panel width in pixels.
const CHAT_PANEL_WIDTH: f32 = 500.0;

/// Maximum messages rendered in the visible chat panel.
const VISIBLE_MESSAGE_COUNT: usize = 12;

/// Whether the chatbox input field is currently active.
#[derive(Resource, Default)]
pub struct ChatInputState {
    pub active: bool,
    pub text: String,
    pub channel: ActiveChannel,
}

/// The active input channel that determines where the next message is sent.
#[derive(Debug, Clone, Default)]
pub enum ActiveChannel {
    #[default]
    Say,
    Yell,
    Zone,
    Guild,
    Party,
    Trade,
    Whisper {
        target_name: String,
    },
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

    /// Convert to an outgoing channel for network routing.
    fn to_outgoing(&self) -> OutgoingChannel {
        match self {
            Self::Say => OutgoingChannel::Say,
            Self::Yell => OutgoingChannel::Yell,
            Self::Zone => OutgoingChannel::Zone,
            Self::Guild => OutgoingChannel::Guild,
            Self::Party => OutgoingChannel::Party,
            Self::Trade => OutgoingChannel::Trade,
            Self::Whisper { target_name } => OutgoingChannel::Whisper {
                target_name: target_name.clone(),
            },
        }
    }
}

#[derive(Component)]
pub struct ChatPanel;

#[derive(Component)]
pub struct ChatMessageList;

#[derive(Component)]
pub struct ChatInputField;

#[derive(Component)]
pub struct ChatInputContainer;

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
    commands.spawn((
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
    ));

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

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

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

/// Handles keyboard text input while the chat input field is active.
pub fn handle_chat_text_input(
    mut char_events: MessageReader<KeyboardInput>,
    mut chat_input: ResMut<ChatInputState>,
    mut q_input_text: Query<&mut Text, With<ChatInputField>>,
) {
    if !chat_input.active {
        return;
    }

    let mut changed = false;

    for event in char_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match event.key_code {
            KeyCode::Backspace => {
                if !chat_input.text.is_empty() {
                    chat_input.text.pop();
                    changed = true;
                }
            }
            KeyCode::Space => {
                chat_input.text.push(' ');
                changed = true;
            }
            _ => {
                if let Key::Character(ref c) = event.logical_key {
                    chat_input.text.push_str(c.as_str());
                    changed = true;
                }
            }
        }
    }

    if !changed {
        return;
    }

    // Check if the user typed a channel switch command.
    // Once detected, switch the active channel and clear the command from the text.
    try_switch_channel(&mut chat_input);

    // Display: prefix + message text
    if let Ok(mut text) = q_input_text.single_mut() {
        **text = format!("{}{}", chat_input.channel.prefix(), chat_input.text);
    }
}

/// Handles the Enter key while in gameplay mode — opens the chat input.
pub fn on_open_chat(
    open: On<Start<OpenChat>>,
    mut chat_input: ResMut<ChatInputState>,
    mut q_input_container: Query<&mut Visibility, With<ChatInputContainer>>,
    mut q_input_text: Query<&mut Text, With<ChatInputField>>,
    mut commands: Commands,
) {
    commands.entity(open.context).insert((
        ContextActivity::<PlayerComponent>::INACTIVE,
        ContextActivity::<Chatting>::ACTIVE,
    ));

    chat_input.active = true;
    chat_input.text.clear();

    if let Ok(mut vis) = q_input_container.single_mut() {
        *vis = Visibility::Inherited;
    }

    // Display the channel prefix as the initial prompt
    if let Ok(mut text) = q_input_text.single_mut() {
        **text = chat_input.channel.prefix();
    }
}

/// Handles Enter while in chat mode — sends the message and returns to gameplay.
pub fn on_send_chat(
    send: On<Start<SendChat>>,
    mut chat_input: ResMut<ChatInputState>,
    mut q_input_container: Query<&mut Visibility, With<ChatInputContainer>>,
    mut commands: Commands,
) {
    let message_text = chat_input.text.trim().to_string();

    if !message_text.is_empty() {
        commands.trigger(OutgoingChatMessage {
            channel: chat_input.channel.to_outgoing(),
            text: message_text,
        });
    }

    // Close the input
    close_chat_input(&mut chat_input, &mut q_input_container);

    commands.entity(send.context).insert((
        ContextActivity::<PlayerComponent>::ACTIVE,
        ContextActivity::<Chatting>::INACTIVE,
    ));
}

/// Handles Escape while in chat mode — cancels input and returns to gameplay.
pub fn on_cancel_chat(
    cancel: On<Start<CancelChat>>,
    mut chat_input: ResMut<ChatInputState>,
    mut q_input_container: Query<&mut Visibility, With<ChatInputContainer>>,
    mut commands: Commands,
) {
    close_chat_input(&mut chat_input, &mut q_input_container);

    commands.entity(cancel.context).insert((
        ContextActivity::<PlayerComponent>::ACTIVE,
        ContextActivity::<Chatting>::INACTIVE,
    ));
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

/// Checks if the current text starts with a channel-switch command.
/// If a complete command is detected (e.g. "/g "), switches the active channel
/// and removes the command from the text buffer.
fn try_switch_channel(chat_input: &mut ResMut<ChatInputState>) {
    let text = &chat_input.text;

    // Simple channel commands: "/cmd " switches and clears
    let channel_commands: &[(&[&str], ActiveChannel)] = &[
        (&["/s ", "/say "], ActiveChannel::Say),
        (&["/y ", "/yell "], ActiveChannel::Yell),
        (&["/z ", "/zone "], ActiveChannel::Zone),
        (&["/g ", "/guild "], ActiveChannel::Guild),
        (&["/p ", "/party "], ActiveChannel::Party),
        (&["/t ", "/trade "], ActiveChannel::Trade),
    ];

    for (prefixes, channel) in channel_commands {
        for prefix in *prefixes {
            if let Some(text) = text.strip_prefix(prefix) {
                chat_input.text = text.to_string();
                chat_input.channel = channel.clone();
                return;
            }
        }
    }

    // Whisper: "/w name " or "/whisper name " — needs a target name followed by a space
    for prefix in &["/w ", "/whisper "] {
        if let Some(after_cmd) = text.strip_prefix(prefix) {
            // Wait until the user has typed the target name and a space after it
            if let Some(space_pos) = after_cmd.find(' ') {
                let target_name = after_cmd[..space_pos].to_string();
                let rest = after_cmd[space_pos + 1..].to_string();
                chat_input.channel = ActiveChannel::Whisper { target_name };
                chat_input.text = rest;
                return;
            }
        }
    }
}

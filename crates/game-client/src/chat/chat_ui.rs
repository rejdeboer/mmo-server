use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::core::PlayerComponent;
use crate::input::Chatting;
use crate::theme::palette;
use super::routing::{OutgoingChannel, OutgoingChatMessage};
use super::channels::ChatLog;

const CHAT_PANEL_WIDTH: f32 = 500.0;
const VISIBLE_MESSAGE_COUNT: usize = 12;

#[derive(Resource, Default)]
pub struct ChatInputState {
    pub active: bool,
    pub text: String,
    pub channel: ActiveChannel,
}

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

#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenChat;

#[derive(InputAction)]
#[action_output(bool)]
pub struct SendChat;

#[derive(InputAction)]
#[action_output(bool)]
pub struct CancelChat;

#[derive(Component)]
pub struct ChatPanel;

#[derive(Component)]
pub struct ChatMessageList;

#[derive(Component)]
pub struct ChatInputField;

#[derive(Component)]
pub struct ChatInputContainer;

pub fn spawn_chat_ui(commands: &mut Commands) {
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
        BackgroundColor(palette::PANEL_BG),
        ChildOf(panel),
    ));

    let input_container = commands
        .spawn((
            ChatInputContainer,
            Node {
                margin: UiRect::top(Val::Px(4.0)),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL_BG_DARK),
            Visibility::Hidden,
            ChildOf(panel),
        ))
        .id();

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

    try_switch_channel(&mut chat_input);

    if let Ok(mut text) = q_input_text.single_mut() {
        **text = format!("{}{}", chat_input.channel.prefix(), chat_input.text);
    }
}

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

    if let Ok(mut text) = q_input_text.single_mut() {
        **text = chat_input.channel.prefix();
    }
}

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

    close_chat_input(&mut chat_input, &mut q_input_container);

    commands.entity(send.context).insert((
        ContextActivity::<PlayerComponent>::ACTIVE,
        ContextActivity::<Chatting>::INACTIVE,
    ));
}

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

fn try_switch_channel(chat_input: &mut ResMut<ChatInputState>) {
    let text = &chat_input.text;

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

    for prefix in &["/w ", "/whisper "] {
        if let Some(after_cmd) = text.strip_prefix(prefix) {
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

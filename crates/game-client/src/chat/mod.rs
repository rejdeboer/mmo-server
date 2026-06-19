mod channels;
mod routing;
mod incoming;
mod chat_ui;

pub use channels::{ChatLog, ChatMessage, ChatMessageChannel};
pub use chat_ui::{OpenChat, SendChat, CancelChat};

use bevy::prelude::*;

use crate::application::{AppState, EnterGame};

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChatLog::default());
        app.insert_resource(chat_ui::ChatInputState::default());

        app.add_observer(chat_ui::on_send_chat);
        app.add_observer(chat_ui::on_cancel_chat);
        app.add_observer(chat_ui::on_open_chat);
        app.add_observer(routing::route_outgoing_chat);
        app.add_observer(on_enter_game);

        app.add_systems(
            Update,
            (
                chat_ui::handle_chat_text_input,
                chat_ui::update_chat_ui,
                incoming::handle_social_chat,
                incoming::handle_whisper_received,
                incoming::handle_whisper_sent,
                incoming::handle_system_notification,
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn on_enter_game(_event: On<EnterGame>, mut commands: Commands) {
    chat_ui::spawn_chat_ui(&mut commands);
}

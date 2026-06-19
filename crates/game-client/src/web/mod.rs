mod connection;
pub mod messages;
mod receive;

pub use connection::{SocialReceiver, SocialSender};
pub use messages::*;

use bevy::prelude::*;

use crate::application::AppState;

pub struct WebPlugin;

impl Plugin for WebPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SocialSender(None));
        app.insert_resource(SocialReceiver(None));

        app.add_message::<SocialChatMessage>();
        app.add_message::<WhisperReceivedMessage>();
        app.add_message::<WhisperSentMessage>();
        app.add_message::<SystemNotificationMessage>();
        app.add_message::<PartyInviteMessage>();
        app.add_message::<PartyUpdateMessage>();
        app.add_message::<PartyDisbandedMessage>();

        app.add_systems(Startup, connection::connect_social);
        app.add_systems(
            Update,
            receive::receive_social_events.run_if(in_state(AppState::InGame)),
        );
    }
}

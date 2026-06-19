use bevy::prelude::*;
use protocol::social::PartyMember;

use crate::application::AppState;
use crate::chat::{ChatLog, ChatMessage, ChatMessageChannel};
use crate::web::{PartyDisbandedMessage, PartyInviteMessage, PartyUpdateMessage};

/// Current party state, `None` when not in a party.
#[derive(Resource, Default)]
pub struct PartyState(pub Option<ActiveParty>);

pub struct ActiveParty {
    pub party_id: i32,
    pub leader_id: i32,
    pub members: Vec<PartyMember>,
}

pub struct PartyPlugin;

impl Plugin for PartyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PartyState>();

        app.add_systems(
            Update,
            (handle_party_invite, handle_party_update, handle_party_disbanded)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn handle_party_invite(
    mut reader: MessageReader<PartyInviteMessage>,
    mut chat_log: ResMut<ChatLog>,
) {
    for msg in reader.read() {
        chat_log.push(ChatMessage {
            channel: ChatMessageChannel::System,
            sender: String::new(),
            text: format!("{} has invited you to a party", msg.from_name),
        });
        // TODO: Show accept/decline UI
    }
}

fn handle_party_update(
    mut reader: MessageReader<PartyUpdateMessage>,
    mut party_state: ResMut<PartyState>,
) {
    for msg in reader.read() {
        party_state.0 = Some(ActiveParty {
            party_id: msg.party_id,
            leader_id: msg.leader_id,
            members: msg.members.clone(),
        });
    }
}

fn handle_party_disbanded(
    mut reader: MessageReader<PartyDisbandedMessage>,
    mut party_state: ResMut<PartyState>,
    mut chat_log: ResMut<ChatLog>,
) {
    for _msg in reader.read() {
        party_state.0 = None;
        chat_log.push(ChatMessage {
            channel: ChatMessageChannel::System,
            sender: String::new(),
            text: "Your party has been disbanded".to_string(),
        });
    }
}

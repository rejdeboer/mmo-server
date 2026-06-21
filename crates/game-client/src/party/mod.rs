use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use protocol::social::PartyMember;

use crate::application::AppState;
use crate::chat::{ChatLog, ChatMessage, ChatMessageChannel};
use crate::theme::{palette, widgets};
use crate::web::{PartyDisbandedMessage, PartyInviteMessage, PartyUpdateMessage, SocialSender};

const INVITE_TIMEOUT_SECS: f64 = 60.0;

/// Current party state, `None` when not in a party.
#[derive(Resource, Default)]
pub struct PartyState(pub Option<ActiveParty>);

pub struct ActiveParty {
    pub party_id: i32,
    pub leader_id: i32,
    pub members: Vec<PartyMember>,
}

/// Exists while a party invite is pending acceptance/decline.
#[derive(Resource)]
pub struct PendingPartyInvite {
    pub from_id: i32,
    pub from_name: String,
    pub received_at: f64,
}

/// Marker for the party invite dialog entity.
#[derive(Component)]
struct PartyInviteDialog;

pub struct PartyPlugin;

impl Plugin for PartyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PartyState>();

        app.add_systems(
            Update,
            (
                handle_party_invite,
                handle_party_update,
                handle_party_disbanded,
                dismiss_expired_invite.run_if(resource_exists::<PendingPartyInvite>),
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn handle_party_invite(
    mut reader: MessageReader<PartyInviteMessage>,
    mut chat_log: ResMut<ChatLog>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time<Real>>,
    existing_dialog: Query<Entity, With<PartyInviteDialog>>,
) {
    for msg in reader.read() {
        chat_log.push(ChatMessage {
            channel: ChatMessageChannel::System,
            sender: String::new(),
            text: format!("{} has invited you to a party", msg.from_name),
        });

        // Dismiss any existing invite dialog
        widgets::despawn_dialog(&mut commands, &existing_dialog);

        // Track the pending invite for timeout logic
        commands.insert_resource(PendingPartyInvite {
            from_id: msg.from_id,
            from_name: msg.from_name.clone(),
            received_at: time.elapsed_secs_f64(),
        });

        // Spawn the dialog inline
        spawn_invite_dialog(&mut commands, &msg.from_name);

        // Play notification sound
        commands.spawn(AudioPlayer::new(
            asset_server.load("sounds/party-invite.ogg"),
        ));
    }
}

fn spawn_invite_dialog(commands: &mut Commands, from_name: &str) {
    let message = format!("{} has invited you to a party", from_name);
    let dialog = widgets::spawn_dialog(commands, &message);
    commands.entity(dialog).insert(PartyInviteDialog);

    // Button row
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                ..default()
            },
            ChildOf(dialog),
        ))
        .id();

    let accept =
        widgets::spawn_dialog_button(commands, row, "Accept", palette::DIALOG_BUTTON_ACCEPT);
    let decline =
        widgets::spawn_dialog_button(commands, row, "Decline", palette::DIALOG_BUTTON_DECLINE);

    commands.entity(accept).observe(on_accept_click);
    commands.entity(decline).observe(on_decline_click);
}

fn on_accept_click(
    _event: On<Pointer<Click>>,
    social_sender: Res<SocialSender>,
    dialog_q: Query<Entity, With<PartyInviteDialog>>,
    mut commands: Commands,
) {
    if let Some(ref sender) = social_sender.0
        && let Err(e) = sender.try_send(web_client::SocialAction::PartyAccept)
    {
        tracing::error!("failed to send party accept: {}", e);
    }
    commands.remove_resource::<PendingPartyInvite>();
    widgets::despawn_dialog(&mut commands, &dialog_q);
}

fn on_decline_click(
    _event: On<Pointer<Click>>,
    social_sender: Res<SocialSender>,
    dialog_q: Query<Entity, With<PartyInviteDialog>>,
    mut commands: Commands,
) {
    if let Some(ref sender) = social_sender.0
        && let Err(e) = sender.try_send(web_client::SocialAction::PartyDecline)
    {
        tracing::error!("failed to send party decline: {}", e);
    }
    commands.remove_resource::<PendingPartyInvite>();
    widgets::despawn_dialog(&mut commands, &dialog_q);
}

fn dismiss_expired_invite(
    invite: Res<PendingPartyInvite>,
    time: Res<Time<Real>>,
    social_sender: Res<SocialSender>,
    dialog_q: Query<Entity, With<PartyInviteDialog>>,
    mut commands: Commands,
    mut chat_log: ResMut<ChatLog>,
) {
    let elapsed = time.elapsed_secs_f64() - invite.received_at;
    if elapsed < INVITE_TIMEOUT_SECS {
        return;
    }

    if let Some(ref sender) = social_sender.0 {
        let _ = sender.try_send(web_client::SocialAction::PartyDecline);
    }
    chat_log.push(ChatMessage {
        channel: ChatMessageChannel::System,
        sender: String::new(),
        text: "Party invite expired".to_string(),
    });
    commands.remove_resource::<PendingPartyInvite>();
    widgets::despawn_dialog(&mut commands, &dialog_q);
}

fn handle_party_update(
    mut reader: MessageReader<PartyUpdateMessage>,
    mut party_state: ResMut<PartyState>,
    dialog_q: Query<Entity, With<PartyInviteDialog>>,
    pending: Option<Res<PendingPartyInvite>>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        party_state.0 = Some(ActiveParty {
            party_id: msg.party_id,
            leader_id: msg.leader_id,
            members: msg.members.clone(),
        });

        // Auto-dismiss pending invite dialog if we joined a party
        if pending.is_some() {
            commands.remove_resource::<PendingPartyInvite>();
            widgets::despawn_dialog(&mut commands, &dialog_q);
        }
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

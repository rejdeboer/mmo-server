use crate::helpers::spawn_app;
use protocol::social::{ChannelType, SocialEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::channel;
use tokio::time::timeout;
use web_server::social::{Hub, HubCommand, HubMessage, NatsBridge, Recipient};

const TIMEOUT_DURATION: Duration = Duration::from_secs(2);
const SETTLE_DURATION: Duration = Duration::from_millis(100);

/// Spawn a Hub connected to NATS and return its message sender.
async fn spawn_hub_with_nats(
    pool: sqlx::PgPool,
    url: &str,
) -> tokio::sync::mpsc::Sender<HubMessage> {
    let nats = NatsBridge::connect(url)
        .await
        .expect("failed to connect to NATS");

    let (hub_tx, hub_rx) = channel::<HubMessage>(128);
    let hub = Hub::new(pool, hub_rx, Some(nats));
    tokio::spawn(hub.run());
    hub_tx
}

/// Helper to connect a character to a hub and return the client rx channel.
async fn connect_character(
    hub_tx: &tokio::sync::mpsc::Sender<HubMessage>,
    character_id: i32,
    name: &str,
    guild_id: Option<i32>,
) -> tokio::sync::mpsc::Receiver<Arc<[u8]>> {
    let (tx, rx) = channel::<Arc<[u8]>>(32);
    hub_tx
        .send(HubMessage::new(
            character_id,
            HubCommand::Connect {
                character_name: name.to_string(),
                guild_id,
                tx,
            },
        ))
        .await
        .unwrap();
    rx
}

/// Decode a received message into a SocialEvent.
fn decode_event(msg: &[u8]) -> SocialEvent {
    bitcode::decode(msg).expect("failed to decode SocialEvent")
}

// ─── Whisper Tests ───────────────────────────────────────────────────────────

#[tokio::test]
async fn whisper_is_relayed_across_instances() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_a_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    let hub_b_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let sender_id: i32 = 1000;
    let recipient_id: i32 = 2000;

    let mut recipient_rx = connect_character(&hub_b_tx, recipient_id, "Recipient", None).await;
    let mut _sender_rx = connect_character(&hub_a_tx, sender_id, "Sender", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    hub_a_tx
        .send(HubMessage::new(
            sender_id,
            HubCommand::Whisper {
                recipient: Recipient::Id(recipient_id),
                text: "Hello from another instance!".to_string(),
            },
        ))
        .await
        .unwrap();

    let msg = timeout(TIMEOUT_DURATION, recipient_rx.recv())
        .await
        .expect("timed out waiting for whisper")
        .expect("channel closed");

    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::Whisper { .. }));
}

// ─── Guild Tests ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn guild_message_relayed_with_sender_confirmation() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_a_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    let hub_b_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let guild_id: i32 = 42;
    let sender_id: i32 = 3000;
    let member_id: i32 = 4000;

    let mut member_rx =
        connect_character(&hub_b_tx, member_id, "GuildMember", Some(guild_id)).await;
    let mut sender_rx =
        connect_character(&hub_a_tx, sender_id, "GuildSender", Some(guild_id)).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    hub_a_tx
        .send(HubMessage::new(
            sender_id,
            HubCommand::ChatMessage {
                channel: ChannelType::Guild,
                text: "Hello guild!".to_string(),
            },
        ))
        .await
        .unwrap();

    let msg = timeout(TIMEOUT_DURATION, member_rx.recv())
        .await
        .expect("timed out waiting for guild message")
        .expect("channel closed");

    let event = decode_event(&msg);
    assert!(matches!(
        event,
        SocialEvent::Chat {
            channel: ChannelType::Guild,
            ..
        }
    ));

    let confirmation = timeout(TIMEOUT_DURATION, sender_rx.recv())
        .await
        .expect("timed out waiting for sender confirmation")
        .expect("channel closed");

    let event = decode_event(&confirmation);
    assert!(matches!(
        event,
        SocialEvent::Chat {
            channel: ChannelType::Guild,
            ..
        }
    ));
}

// ─── Party Tests ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn party_invite_accept_and_chat() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let leader_id: i32 = 5000;
    let member_id: i32 = 5001;

    let mut leader_rx = connect_character(&hub_tx, leader_id, "Leader", None).await;
    let mut member_rx = connect_character(&hub_tx, member_id, "Member", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    // Leader invites member
    hub_tx
        .send(HubMessage::new(
            leader_id,
            HubCommand::PartyInvite {
                target: Recipient::Id(member_id),
            },
        ))
        .await
        .unwrap();

    // Member receives invite notification (system message)
    let msg = timeout(TIMEOUT_DURATION, member_rx.recv())
        .await
        .expect("timed out waiting for invite")
        .expect("channel closed");

    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::SystemMessage { .. }));

    // Member accepts
    hub_tx
        .send(HubMessage::new(member_id, HubCommand::PartyAccept))
        .await
        .unwrap();

    // Both should receive a system message about joining
    let msg = timeout(TIMEOUT_DURATION, leader_rx.recv())
        .await
        .expect("timed out waiting for party join notification")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::SystemMessage { .. }));

    let msg = timeout(TIMEOUT_DURATION, member_rx.recv())
        .await
        .expect("timed out waiting for party join notification")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::SystemMessage { .. }));

    // Give party sub time to establish
    tokio::time::sleep(SETTLE_DURATION).await;

    // Leader sends party chat
    hub_tx
        .send(HubMessage::new(
            leader_id,
            HubCommand::ChatMessage {
                channel: ChannelType::Party,
                text: "Party time!".to_string(),
            },
        ))
        .await
        .unwrap();

    // Both should receive the party chat via NATS
    let msg = timeout(TIMEOUT_DURATION, leader_rx.recv())
        .await
        .expect("timed out waiting for party chat (leader)")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(
        event,
        SocialEvent::Chat {
            channel: ChannelType::Party,
            ..
        }
    ));

    let msg = timeout(TIMEOUT_DURATION, member_rx.recv())
        .await
        .expect("timed out waiting for party chat (member)")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(
        event,
        SocialEvent::Chat {
            channel: ChannelType::Party,
            ..
        }
    ));
}

#[tokio::test]
async fn party_decline() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let leader_id: i32 = 6000;
    let target_id: i32 = 6001;

    let mut leader_rx = connect_character(&hub_tx, leader_id, "Leader", None).await;
    let mut target_rx = connect_character(&hub_tx, target_id, "Target", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    // Invite
    hub_tx
        .send(HubMessage::new(
            leader_id,
            HubCommand::PartyInvite {
                target: Recipient::Id(target_id),
            },
        ))
        .await
        .unwrap();

    // Consume invite notification
    let _ = timeout(TIMEOUT_DURATION, target_rx.recv()).await.unwrap();

    // Decline
    hub_tx
        .send(HubMessage::new(target_id, HubCommand::PartyDecline))
        .await
        .unwrap();

    // Leader gets decline notification
    let msg = timeout(TIMEOUT_DURATION, leader_rx.recv())
        .await
        .expect("timed out waiting for decline notification")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::SystemMessage { .. }));
}

#[tokio::test]
async fn party_leave() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let leader_id: i32 = 7000;
    let member_id: i32 = 7001;

    let mut leader_rx = connect_character(&hub_tx, leader_id, "Leader", None).await;
    let mut member_rx = connect_character(&hub_tx, member_id, "Member", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    // Form party
    hub_tx
        .send(HubMessage::new(
            leader_id,
            HubCommand::PartyInvite {
                target: Recipient::Id(member_id),
            },
        ))
        .await
        .unwrap();
    let _ = timeout(TIMEOUT_DURATION, member_rx.recv()).await.unwrap(); // invite notification
    hub_tx
        .send(HubMessage::new(member_id, HubCommand::PartyAccept))
        .await
        .unwrap();
    // Drain join notifications
    let _ = timeout(TIMEOUT_DURATION, leader_rx.recv()).await.unwrap();
    let _ = timeout(TIMEOUT_DURATION, member_rx.recv()).await.unwrap();

    // Member leaves
    hub_tx
        .send(HubMessage::new(member_id, HubCommand::PartyLeave))
        .await
        .unwrap();

    // Leader gets leave notification
    let msg = timeout(TIMEOUT_DURATION, leader_rx.recv())
        .await
        .expect("timed out waiting for leave notification")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::SystemMessage { .. }));
}

#[tokio::test]
async fn party_kick_requires_leader() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let leader_id: i32 = 8000;
    let member_id: i32 = 8001;

    let mut leader_rx = connect_character(&hub_tx, leader_id, "Leader", None).await;
    let mut member_rx = connect_character(&hub_tx, member_id, "Member", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    // Form party
    hub_tx
        .send(HubMessage::new(
            leader_id,
            HubCommand::PartyInvite {
                target: Recipient::Id(member_id),
            },
        ))
        .await
        .unwrap();
    let _ = timeout(TIMEOUT_DURATION, member_rx.recv()).await.unwrap();
    hub_tx
        .send(HubMessage::new(member_id, HubCommand::PartyAccept))
        .await
        .unwrap();
    let _ = timeout(TIMEOUT_DURATION, leader_rx.recv()).await.unwrap();
    let _ = timeout(TIMEOUT_DURATION, member_rx.recv()).await.unwrap();

    // Member tries to kick leader — should get error
    hub_tx
        .send(HubMessage::new(
            member_id,
            HubCommand::PartyKick {
                target_id: leader_id,
            },
        ))
        .await
        .unwrap();

    let msg = timeout(TIMEOUT_DURATION, member_rx.recv())
        .await
        .expect("timed out waiting for kick error")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::Error { .. }));

    // Leader kicks member — should succeed
    hub_tx
        .send(HubMessage::new(
            leader_id,
            HubCommand::PartyKick {
                target_id: member_id,
            },
        ))
        .await
        .unwrap();

    // Member gets kicked notification
    let msg = timeout(TIMEOUT_DURATION, member_rx.recv())
        .await
        .expect("timed out waiting for kick notification")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::SystemMessage { .. }));
}

#[tokio::test]
async fn party_invite_target_already_in_party() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let leader_id: i32 = 9000;
    let member_id: i32 = 9001;
    let outsider_id: i32 = 9002;

    let mut leader_rx = connect_character(&hub_tx, leader_id, "Leader", None).await;
    let mut member_rx = connect_character(&hub_tx, member_id, "Member", None).await;
    let mut outsider_rx = connect_character(&hub_tx, outsider_id, "Outsider", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    // Form party between leader and member
    hub_tx
        .send(HubMessage::new(
            leader_id,
            HubCommand::PartyInvite {
                target: Recipient::Id(member_id),
            },
        ))
        .await
        .unwrap();
    let _ = timeout(TIMEOUT_DURATION, member_rx.recv()).await.unwrap();
    hub_tx
        .send(HubMessage::new(member_id, HubCommand::PartyAccept))
        .await
        .unwrap();
    let _ = timeout(TIMEOUT_DURATION, leader_rx.recv()).await.unwrap();
    let _ = timeout(TIMEOUT_DURATION, member_rx.recv()).await.unwrap();

    // Outsider tries to invite member who is already in a party
    hub_tx
        .send(HubMessage::new(
            outsider_id,
            HubCommand::PartyInvite {
                target: Recipient::Id(member_id),
            },
        ))
        .await
        .unwrap();

    let msg = timeout(TIMEOUT_DURATION, outsider_rx.recv())
        .await
        .expect("timed out waiting for error")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::Error { .. }));
}

// ─── Rate Limiting Tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn rate_limiting_kicks_in_after_burst() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let sender_id: i32 = 10000;
    let recipient_id: i32 = 10001;

    let mut sender_rx = connect_character(&hub_tx, sender_id, "Spammer", None).await;
    let mut _recipient_rx = connect_character(&hub_tx, recipient_id, "Target", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    // Send messages rapidly — burst is 5, so the 6th+ should be rate limited
    for i in 0..8 {
        hub_tx
            .send(HubMessage::new(
                sender_id,
                HubCommand::Whisper {
                    recipient: Recipient::Id(recipient_id),
                    text: format!("msg {i}"),
                },
            ))
            .await
            .unwrap();
    }

    // Give hub time to process all messages
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Drain all messages from sender_rx
    let mut events = Vec::new();
    while let Ok(msg) = sender_rx.try_recv() {
        events.push(decode_event(&msg));
    }

    // Should have some WhisperReceipts (successful sends) and at least one Error (rate limited)
    let receipts = events
        .iter()
        .filter(|e| matches!(e, SocialEvent::WhisperReceipt { .. }))
        .count();
    let errors = events
        .iter()
        .filter(|e| matches!(e, SocialEvent::Error { .. }))
        .count();

    assert!(receipts > 0, "should have some successful sends");
    assert!(errors > 0, "should have been rate limited");
    assert_eq!(receipts + errors, 8, "all messages should be accounted for");
}

// ─── Disconnect Tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn disconnect_removes_from_party() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let leader_id: i32 = 11000;
    let member_id: i32 = 11001;

    let mut leader_rx = connect_character(&hub_tx, leader_id, "Leader", None).await;
    let mut member_rx = connect_character(&hub_tx, member_id, "Member", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    // Form party
    hub_tx
        .send(HubMessage::new(
            leader_id,
            HubCommand::PartyInvite {
                target: Recipient::Id(member_id),
            },
        ))
        .await
        .unwrap();
    let _ = timeout(TIMEOUT_DURATION, member_rx.recv()).await.unwrap();
    hub_tx
        .send(HubMessage::new(member_id, HubCommand::PartyAccept))
        .await
        .unwrap();
    let _ = timeout(TIMEOUT_DURATION, leader_rx.recv()).await.unwrap();
    let _ = timeout(TIMEOUT_DURATION, member_rx.recv()).await.unwrap();

    // Member disconnects
    hub_tx
        .send(HubMessage::new(member_id, HubCommand::Disconnect))
        .await
        .unwrap();

    // Leader should get a leave notification
    let msg = timeout(TIMEOUT_DURATION, leader_rx.recv())
        .await
        .expect("timed out waiting for disconnect leave notification")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::SystemMessage { .. }));
}

// ─── Error Case Tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn party_chat_without_party_returns_error() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let solo_id: i32 = 12000;
    let mut solo_rx = connect_character(&hub_tx, solo_id, "Solo", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    hub_tx
        .send(HubMessage::new(
            solo_id,
            HubCommand::ChatMessage {
                channel: ChannelType::Party,
                text: "Hello?".to_string(),
            },
        ))
        .await
        .unwrap();

    let msg = timeout(TIMEOUT_DURATION, solo_rx.recv())
        .await
        .expect("timed out waiting for error")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::Error { .. }));
}

#[tokio::test]
async fn accept_without_invite_returns_error() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let char_id: i32 = 13000;
    let mut char_rx = connect_character(&hub_tx, char_id, "Nobody", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    hub_tx
        .send(HubMessage::new(char_id, HubCommand::PartyAccept))
        .await
        .unwrap();

    let msg = timeout(TIMEOUT_DURATION, char_rx.recv())
        .await
        .expect("timed out waiting for error")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::Error { .. }));
}

#[tokio::test]
async fn leave_without_party_returns_error() {
    let app = spawn_app().await;
    let pool = web_server::server::get_connection_pool(
        &web_server::configuration::get_configuration()
            .unwrap()
            .database,
    );

    let hub_tx = spawn_hub_with_nats(pool.clone(), &app.nats_url).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let char_id: i32 = 14000;
    let mut char_rx = connect_character(&hub_tx, char_id, "Loner", None).await;

    tokio::time::sleep(SETTLE_DURATION).await;

    hub_tx
        .send(HubMessage::new(char_id, HubCommand::PartyLeave))
        .await
        .unwrap();

    let msg = timeout(TIMEOUT_DURATION, char_rx.recv())
        .await
        .expect("timed out waiting for error")
        .expect("channel closed");
    let event = decode_event(&msg);
    assert!(matches!(event, SocialEvent::Error { .. }));
}

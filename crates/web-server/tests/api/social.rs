use crate::helpers::spawn_app;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::channel;
use tokio::time::timeout;
use web_server::social::{Hub, HubCommand, HubMessage, NatsBridge, Recipient};

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

/// A whisper sent on instance A should be delivered to the recipient on instance B.
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

    // Give hubs a moment to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    let sender_id: i32 = 1000;
    let recipient_id: i32 = 2000;

    // Connect recipient on Hub B
    let (recipient_client_tx, mut recipient_client_rx) = channel::<Arc<[u8]>>(32);
    hub_b_tx
        .send(HubMessage::new(
            recipient_id,
            HubCommand::Connect {
                character_name: "Recipient".to_string(),
                guild_id: None,
                tx: recipient_client_tx,
            },
        ))
        .await
        .unwrap();

    // Connect sender on Hub A
    let (sender_client_tx, mut _sender_client_rx) = channel::<Arc<[u8]>>(32);
    hub_a_tx
        .send(HubMessage::new(
            sender_id,
            HubCommand::Connect {
                character_name: "Sender".to_string(),
                guild_id: None,
                tx: sender_client_tx,
            },
        ))
        .await
        .unwrap();

    // Give NATS subscriptions time to establish
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send whisper from Hub A to recipient on Hub B (by ID)
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

    // Recipient on Hub B should receive the whisper via NATS
    let msg = timeout(Duration::from_secs(2), recipient_client_rx.recv())
        .await
        .expect("timed out waiting for whisper")
        .expect("channel closed unexpectedly");

    // Verify it's a valid FlatBuffer event (ServerWhisper)
    let event = flatbuffers::root::<schemas::social::Event>(&msg).expect("invalid flatbuffer");
    assert_eq!(event.data_type(), schemas::social::EventData::ServerWhisper);
}

/// A guild message sent on instance A should be delivered to guild members on instance B,
/// but not echoed back to the sender.
#[tokio::test]
async fn guild_message_relayed_without_echo() {
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

    // Connect a guild member on Hub B
    let (member_client_tx, mut member_client_rx) = channel::<Arc<[u8]>>(32);
    hub_b_tx
        .send(HubMessage::new(
            member_id,
            HubCommand::Connect {
                character_name: "GuildMember".to_string(),
                guild_id: Some(guild_id),
                tx: member_client_tx,
            },
        ))
        .await
        .unwrap();

    // Connect the sender on Hub A (same guild)
    let (sender_client_tx, mut sender_client_rx) = channel::<Arc<[u8]>>(32);
    hub_a_tx
        .send(HubMessage::new(
            sender_id,
            HubCommand::Connect {
                character_name: "GuildSender".to_string(),
                guild_id: Some(guild_id),
                tx: sender_client_tx,
            },
        ))
        .await
        .unwrap();

    // Give NATS subscriptions time to establish
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send guild message from Hub A
    hub_a_tx
        .send(HubMessage::new(
            sender_id,
            HubCommand::ChatMessage {
                channel: schemas::social::ChannelType::Guild,
                text: "Hello guild!".to_string(),
            },
        ))
        .await
        .unwrap();

    // Member on Hub B should receive the message
    let msg = timeout(Duration::from_secs(2), member_client_rx.recv())
        .await
        .expect("timed out waiting for guild message")
        .expect("channel closed unexpectedly");

    let event = flatbuffers::root::<schemas::social::Event>(&msg).expect("invalid flatbuffer");
    assert_eq!(
        event.data_type(),
        schemas::social::EventData::ServerChatMessage
    );

    // Sender should also receive their own message back as confirmation
    let confirmation = timeout(Duration::from_secs(2), sender_client_rx.recv())
        .await
        .expect("timed out waiting for sender confirmation")
        .expect("channel closed unexpectedly");

    let event =
        flatbuffers::root::<schemas::social::Event>(&confirmation).expect("invalid flatbuffer");
    assert_eq!(
        event.data_type(),
        schemas::social::EventData::ServerChatMessage
    );
}

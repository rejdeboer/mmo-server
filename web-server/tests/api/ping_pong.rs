use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite;

use crate::helpers::spawn_app;

#[tokio::test]
async fn ping_pong_works() {
    let app = spawn_app().await;
    let mut owner_client = app.create_owner_client().await;

    let ping_payload: Vec<u8> = vec![123];

    owner_client
        .send(tungstenite::Message::Ping(ping_payload.clone()))
        .await
        .unwrap();

    let pong_payload = match owner_client.next().await.unwrap().unwrap() {
        tungstenite::Message::Pong(payload) => payload,
        other => panic!("expected a pong message but got {other:?}"),
    };

    assert_eq!(pong_payload, ping_payload);
}
